//! The component's end of the subprocess binding's framing (component-interfaces spec §9.3): the Engine
//! Protocol's stdio framing, a `Content-Length` header section and then exactly that many bytes of JSON,
//! in both directions. Unknown headers are ignored, and a header section with no length is skipped, so
//! the stream resynchronises on the next frame rather than dying.

use std::io::{self, BufRead, Write};

/// The next frame's body, or `None` when stdin has ended: the engine has gone, or has finished with this
/// component, and the bridge exits (spec §9.3).
pub fn read(reader: &mut impl BufRead) -> io::Result<Option<Vec<u8>>> {
  loop {
    let mut length: Option<usize> = None;
    let mut saw_header = false;
    loop {
      let mut line = Vec::new();
      if reader.read_until(b'\n', &mut line)? == 0 {
        return Ok(None);
      }
      let line = String::from_utf8_lossy(&line);
      let line = line.trim_end_matches(['\r', '\n']);
      if line.is_empty() {
        if saw_header {
          break;
        }
        continue;
      }
      if let Some((header, value)) = line.split_once(':') {
        saw_header = true;
        if header.trim().eq_ignore_ascii_case("content-length") {
          length = value.trim().parse().ok();
        }
      } else {
        tracing::warn!(line, "a header line with no ':' on stdin; skipped");
      }
    }
    let Some(length) = length else {
      tracing::warn!("a header section with no Content-Length on stdin; skipped");
      continue;
    };
    let mut body = vec![0; length];
    match reader.read_exact(&mut body) {
      Ok(()) => return Ok(Some(body)),
      Err(err) if err.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
      Err(err) => return Err(err),
    }
  }
}

/// Writes one frame, and flushes it: the engine is waiting on it.
pub fn write(writer: &mut impl Write, frame: &serde_json::Value) -> io::Result<()> {
  let body = serde_json::to_vec(frame)?;
  write!(writer, "Content-Length: {}\r\n\r\n", body.len())?;
  writer.write_all(&body)?;
  writer.flush()
}

#[cfg(test)]
mod tests {
  use super::*;
  use pretty_assertions::assert_eq;
  use serde_json::json;

  #[test]
  fn a_frame_is_read_back_as_it_was_written() {
    let mut bytes = Vec::new();
    write(&mut bytes, &json!({ "type": "request", "id": "1" })).unwrap();
    write(&mut bytes, &json!({ "type": "request", "id": "2" })).unwrap();
    let mut reader = bytes.as_slice();
    let first: serde_json::Value = serde_json::from_slice(&read(&mut reader).unwrap().unwrap()).unwrap();
    let second: serde_json::Value = serde_json::from_slice(&read(&mut reader).unwrap().unwrap()).unwrap();
    assert_eq!(first["id"], "1");
    assert_eq!(second["id"], "2");
    assert_eq!(read(&mut reader).unwrap(), None);
  }

  #[test]
  fn unknown_headers_are_ignored_and_a_section_with_no_length_is_skipped() {
    let bytes = b"X-Other: 1\r\n\r\ncontent-length: 2\r\nX-Other: 2\r\n\r\n{}";
    let mut reader = &bytes[..];
    assert_eq!(read(&mut reader).unwrap(), Some(b"{}".to_vec()));
  }

  #[test]
  fn a_body_cut_short_is_the_end_of_the_stream() {
    let bytes = b"Content-Length: 10\r\n\r\n{}";
    let mut reader = &bytes[..];
    assert_eq!(read(&mut reader).unwrap(), None);
  }
}
