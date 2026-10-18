//! `janus-engine`'s stdio framing (engine-protocol spec §3): `Content-Length: N\r\n\r\n` and then
//! exactly N bytes of JSON, in both directions. Spike 1.3's proven approach, LSP-style.

use std::io::{Read, Write};

/// Reads one frame. `Ok(None)` on a clean EOF at a frame boundary; unknown headers are ignored.
///
/// The body is read as it arrives rather than allocated up front from the header, so a corrupt
/// `Content-Length` costs what the host actually sends, not what the header claims.
pub fn read_frame(reader: &mut impl Read) -> std::io::Result<Option<Vec<u8>>> {
  let mut content_length: Option<u64> = None;
  let mut line = Vec::new();
  loop {
    line.clear();
    let mut byte = [0u8; 1];
    loop {
      if reader.read(&mut byte)? == 0 {
        return if line.is_empty() && content_length.is_none() {
          Ok(None)
        } else {
          Err(invalid(std::io::ErrorKind::UnexpectedEof, "EOF mid-header"))
        };
      }
      match byte[0] {
        b'\n' => break,
        b'\r' => {}
        other => line.push(other),
      }
    }
    if line.is_empty() {
      break; // the blank line: headers done
    }
    let header = String::from_utf8_lossy(&line);
    if let Some(value) = header.strip_prefix("Content-Length:") {
      content_length = Some(value.trim().parse().map_err(|err| {
        invalid(
          std::io::ErrorKind::InvalidData,
          &format!("bad Content-Length: {err}"),
        )
      })?);
    }
  }
  let length = content_length
    .ok_or_else(|| invalid(std::io::ErrorKind::InvalidData, "missing Content-Length header"))?;
  let mut body = Vec::new();
  reader.take(length).read_to_end(&mut body)?;
  if body.len() as u64 != length {
    return Err(invalid(std::io::ErrorKind::UnexpectedEof, "EOF mid-body"));
  }
  Ok(Some(body))
}

/// Writes one frame and flushes it, so the host sees it now rather than at the next frame.
pub fn write_frame(writer: &mut impl Write, frame: &[u8]) -> std::io::Result<()> {
  write!(writer, "Content-Length: {}\r\n\r\n", frame.len())?;
  writer.write_all(frame)?;
  writer.flush()
}

fn invalid(kind: std::io::ErrorKind, message: &str) -> std::io::Error {
  std::io::Error::new(kind, message.to_string())
}

#[cfg(test)]
mod tests {
  use super::*;
  use pretty_assertions::assert_eq;

  fn read(bytes: &[u8]) -> std::io::Result<Option<Vec<u8>>> {
    read_frame(&mut &bytes[..])
  }

  #[test]
  fn a_frame_written_is_the_frame_read() {
    let mut wire = Vec::new();
    write_frame(&mut wire, br#"{"type":"request"}"#).unwrap();
    assert_eq!(wire, b"Content-Length: 18\r\n\r\n{\"type\":\"request\"}".to_vec());
    assert_eq!(read(&wire).unwrap(), Some(br#"{"type":"request"}"#.to_vec()));
  }

  #[test]
  fn other_headers_are_ignored_and_bare_newlines_accepted() {
    let frame = read(b"X-Other: 1\nContent-Length: 2\n\n{}").unwrap();
    assert_eq!(frame, Some(b"{}".to_vec()));
  }

  #[test]
  fn eof_between_frames_is_the_end_of_the_stream() {
    assert_eq!(read(b"").unwrap(), None);
  }

  #[test]
  fn eof_inside_a_frame_is_an_error() {
    assert_eq!(
      read(b"Content-Length: 2\r\n").unwrap_err().kind(),
      std::io::ErrorKind::UnexpectedEof
    );
    assert_eq!(
      read(b"Content-Length: 4\r\n\r\n{}").unwrap_err().kind(),
      std::io::ErrorKind::UnexpectedEof
    );
  }

  #[test]
  fn a_huge_length_with_a_short_body_fails_without_allocating_the_length() {
    let header = format!("Content-Length: {}\r\n\r\n{{}}", u64::MAX);
    assert_eq!(
      read(header.as_bytes()).unwrap_err().kind(),
      std::io::ErrorKind::UnexpectedEof
    );
  }

  #[test]
  fn a_missing_or_malformed_length_is_invalid_data() {
    assert_eq!(
      read(b"X-Other: 1\r\n\r\n{}").unwrap_err().kind(),
      std::io::ErrorKind::InvalidData
    );
    assert_eq!(
      read(b"Content-Length: two\r\n\r\n{}").unwrap_err().kind(),
      std::io::ErrorKind::InvalidData
    );
  }
}
