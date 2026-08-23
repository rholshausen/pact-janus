//! `pact-engine-toy`: the 1.2 toy engine behind LSP-style framing over stdio.
//!
//! Protocol: `Content-Length: N\r\n\r\n<N bytes of JSON frame>`, both
//! directions. stdout carries protocol frames ONLY; all logging goes to
//! stderr. Exit paths:
//!   - `{"op":"shutdown"}` frame → ack, flush, exit 0
//!   - stdin EOF (test runner died or closed the pipe) → exit 0
//! Framing errors are answered with an `err` frame when possible, then the
//! stream is treated as poisoned and the engine exits non-zero.

// logic.rs is copied verbatim from spike 1.2 (engine-toy/logic/src/lib.rs) —
// spikes stay standalone. The `tests` module rides along harmlessly.
#[path = "logic.rs"]
mod logic;

use std::io::{BufReader, BufWriter, Read, Write};

fn main() {
    eprintln!(
        "pact-engine-toy starting (pid {}, protocol version {})",
        std::process::id(),
        logic::PROTOCOL_VERSION
    );
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut reader = BufReader::new(stdin.lock());
    let mut writer = BufWriter::new(stdout.lock());

    loop {
        match read_frame(&mut reader) {
            Ok(Some(request)) => {
                if is_shutdown(&request) {
                    write_frame(&mut writer, br#"{"ok":{"shutting-down":true}}"#);
                    eprintln!("pact-engine-toy: shutdown requested, exiting");
                    std::process::exit(0);
                }
                let response = logic::handle_frame(&request);
                write_frame(&mut writer, &response);
            }
            Ok(None) => {
                // EOF: our counterpart is gone (cleanly or not). Never linger.
                eprintln!("pact-engine-toy: stdin closed, exiting");
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("pact-engine-toy: framing error: {e}");
                write_frame(
                    &mut writer,
                    br#"{"err":{"code":"framing-error","detail":"malformed Content-Length framing"}}"#,
                );
                std::process::exit(1);
            }
        }
    }
}

fn is_shutdown(frame: &[u8]) -> bool {
    serde_json::from_slice::<serde_json::Value>(frame)
        .ok()
        .and_then(|f| f.get("op").and_then(|o| o.as_str().map(|s| s == "shutdown")))
        .unwrap_or(false)
}

/// Reads one `Content-Length`-framed message. `Ok(None)` on clean EOF at a
/// message boundary. Unknown headers are ignored (LSP behaviour).
fn read_frame(reader: &mut impl Read) -> std::io::Result<Option<Vec<u8>>> {
    let mut content_length: Option<usize> = None;
    let mut line = Vec::new();
    loop {
        line.clear();
        let mut byte = [0u8; 1];
        loop {
            match reader.read(&mut byte)? {
                0 => {
                    return if line.is_empty() && content_length.is_none() {
                        Ok(None) // EOF at a message boundary
                    } else {
                        Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "EOF mid-header"))
                    };
                }
                _ => {
                    if byte[0] == b'\n' {
                        break;
                    }
                    if byte[0] != b'\r' {
                        line.push(byte[0]);
                    }
                }
            }
        }
        if line.is_empty() {
            break; // blank line: headers done
        }
        let header = String::from_utf8_lossy(&line);
        if let Some(v) = header.strip_prefix("Content-Length:") {
            content_length = Some(
                v.trim()
                    .parse()
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, format!("bad Content-Length: {e}")))?,
            );
        }
    }
    let len = content_length
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "missing Content-Length header"))?;
    let mut body = vec![0u8; len];
    reader.read_exact(&mut body)?;
    Ok(Some(body))
}

fn write_frame(writer: &mut impl Write, frame: &[u8]) {
    // A write failure means the counterpart is gone; exit rather than spin.
    let result = write!(writer, "Content-Length: {}\r\n\r\n", frame.len())
        .and_then(|_| writer.write_all(frame))
        .and_then(|_| writer.flush());
    if let Err(e) = result {
        eprintln!("pact-engine-toy: stdout write failed ({e}), exiting");
        std::process::exit(1);
    }
}
