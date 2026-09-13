//! `janus-engine`: the real kernel behind spike 1.3's proven Content-Length stdio framing (ADR
//! 0003's "one engine build produces three artifacts" — this is the subprocess one). Where the
//! spike wrapped a toy engine's own ad hoc `logic::handle_frame`, this wraps
//! `pact_janus_kernel::protocol::Engine::dispatch` directly, wired with the real HTTP transport
//! and JSON content components (plan task 4.5) — same proven framing loop, a real engine behind
//! it this time.
//!
//! Protocol: `Content-Length: N\r\n\r\n<N bytes of JSON frame>`, both directions; frames on
//! stdout only, logs on stderr. Exit path: stdin EOF (engine-protocol spec §6/§7.1's
//! orphan-prevention mechanism — no process groups or signal choreography needed, spike 1.3
//! finding 2). A framing violation (a malformed `Content-Length` header, EOF mid-header) poisons
//! the stream — there is no way to resynchronise a byte-counted protocol — so it is reported on
//! stderr and the process exits non-zero rather than guessing at recovery.

use pact_janus_component_http::HttpTransport;
use pact_janus_component_json::JsonContent;
use pact_janus_kernel::component::TransportComponent;
use pact_janus_kernel::protocol::Engine;
use std::collections::HashMap;
use std::io::{BufReader, BufWriter, Read, Write};
use std::sync::Arc;

fn main() {
  eprintln!(
    "janus-engine starting (pid {}, protocol version {})",
    std::process::id(),
    pact_janus_kernel::PROTOCOL_VERSION
  );

  let mut transports: HashMap<String, Arc<dyn TransportComponent>> = HashMap::new();
  transports.insert("http".to_string(), Arc::new(HttpTransport::new()));
  let mut engine = Engine::with_components(transports, Some(Arc::new(JsonContent::new())));

  let stdin = std::io::stdin();
  let stdout = std::io::stdout();
  let mut reader = BufReader::new(stdin.lock());
  let mut writer = BufWriter::new(stdout.lock());

  loop {
    match read_frame(&mut reader) {
      Ok(Some(request)) => {
        let response = engine.dispatch(&request);
        write_frame(&mut writer, &response);
      }
      Ok(None) => {
        eprintln!("janus-engine: stdin closed, exiting");
        std::process::exit(0);
      }
      Err(err) => {
        eprintln!("janus-engine: framing error: {err}");
        write_frame(
          &mut writer,
          br#"{"type":"response","id":"","error":{"code":"malformed-frame","category":"protocol","message":"malformed Content-Length framing"}}"#,
        );
        std::process::exit(1);
      }
    }
  }
}

/// Reads one `Content-Length`-framed message (LSP-style, spike 1.3's proven approach, carried
/// over verbatim). `Ok(None)` on clean EOF at a message boundary; unknown headers are ignored.
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
            Err(std::io::Error::new(
              std::io::ErrorKind::UnexpectedEof,
              "EOF mid-header",
            ))
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
      content_length = Some(v.trim().parse().map_err(|err| {
        std::io::Error::new(
          std::io::ErrorKind::InvalidData,
          format!("bad Content-Length: {err}"),
        )
      })?);
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
  if let Err(err) = result {
    eprintln!("janus-engine: stdout write failed ({err}), exiting");
    std::process::exit(1);
  }
}
