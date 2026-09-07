//! A structured position, shared by every document model the kernel validates.
//!
//! Deliberately not `anyhow`: CLAUDE.md's "errors are values" rule applies to everything that will
//! eventually cross the engine boundary, and a document-validation error is exactly that kind of
//! value — later protocol wiring lifts these straight into an `EngineError` frame's `problems`
//! (engine protocol spec §10.2) without re-deriving the mapping.

use serde::Serialize;

/// One position where a document disagreed with what it claimed to be.
///
/// `pointer` is an RFC 6901 JSON pointer, so a reader can jump straight to the offending member
/// instead of bisecting the file (contract-file spec §11: "an error that names a position is the
/// difference between a fix and a bisect"; shape-language spec §5 states the same requirement for
/// shape well-formedness).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Problem {
  pub pointer: String,
  pub message: String,
}

/// Render a `serde_path_to_error::Path` as an RFC 6901 JSON pointer.
pub fn json_pointer(path: &serde_path_to_error::Path) -> String {
  use serde_path_to_error::Segment;
  let mut pointer = String::new();
  for segment in path {
    pointer.push('/');
    let raw = match segment {
      Segment::Seq { index } => index.to_string(),
      Segment::Map { key } => key.clone(),
      Segment::Enum { variant } => variant.clone(),
      Segment::Unknown => "?".to_string(),
    };
    pointer.push_str(&raw.replace('~', "~0").replace('/', "~1"));
  }
  pointer
}

/// Append `segment` to a JSON pointer, escaping it per RFC 6901 §4.
pub fn push_pointer_segment(pointer: &mut String, segment: &str) {
  pointer.push('/');
  pointer.push_str(&segment.replace('~', "~0").replace('/', "~1"));
}
