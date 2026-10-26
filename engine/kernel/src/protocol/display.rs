//! Display text on the wire (spec §2.7, ADR 0054): a member's text, and, for a host that declared
//! the `segments` capability, `<member>-segments` beside it.

use crate::display::Marked;
use serde_json::Value;

/// `text` as `member` in `document`, and its segments as `<member>-segments` when `segments` says
/// the host asked for them.
pub(crate) fn put(document: &mut Value, member: &str, text: &Marked, segments: bool) {
  document[member] = Value::String(text.text());
  if segments {
    document[format!("{member}-segments")] =
      serde_json::to_value(text.segments()).expect("segments always serialize");
  }
}
