//! Display text marked with what each piece is (ADR 0054 decisions 4–8, engine-protocol spec §2.7).
//!
//! The engine writes text for people: a plan's text forms, the `can-i-deploy` page, a mismatch's
//! message. A host that wants to style it should not have to recover what the engine knew while it
//! wrote it. So a renderer writes through [`Marked`], which keeps the text and its segments
//! together: pieces whose texts, joined, are the text, each with the [`Kind`] of thing it is. The
//! text is what it always was. The segments are sent beside it, as `<member>-segments`, to a host
//! that declared the `segments` capability.
//!
//! A kind says what a piece *is*, never how it looks: the engine does not know where its text is
//! going (plan-grammar spec §3.3). The host maps each kind to its own look, and prints a kind it does
//! not know as plain text.

use serde::{Deserialize, Serialize};

/// What a piece of display text is. The protocol's vocabulary is open (`x-known-values`); these are
/// the kinds this engine writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
  /// Something passed: a verdict of `true`, a compatible pair, a pass decision.
  Pass,
  /// Something failed: a verdict of `false`, an error, an incompatible pair, a block decision.
  Fail,
  /// Worth a look, but not a failure: a warning, an advisory finding, a warn decision.
  Warn,
  /// Needs a person to decide: a finding that could not be compared.
  Review,
  /// Said for completeness, behind the rest: an exempted finding, a note, the policy behind a page.
  Quiet,
  /// Where something is: a resolve's path, a finding's address.
  Path,
  /// A code a reader can look up.
  Code,
  /// What a block of text is about.
  Heading,
  /// A quoted string value.
  String,
  /// A number value.
  Number,
  /// Any other value: `true`, `NULL`, `ABSENT`, bytes.
  Literal,
  /// A plan container's label.
  Label,
  /// A plan action's name.
  Action,
  /// A plan operator: `->`, `**`.
  Operator,
  /// A plan annotation.
  Annotation,
  /// The executed form's `=>`.
  Arrow,
}

/// One piece of display text, and what it is. No kind is plain text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Segment {
  pub text: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub kind: Option<Kind>,
}

/// Display text being written, with its segments: what a renderer writes through, so the text and
/// the segments cannot disagree.
///
/// Adjacent pieces of the same kind are kept as one, so two writers that say the same thing the same
/// way produce the same segments however they split their writes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Marked {
  segments: Vec<Segment>,
}

impl Marked {
  pub fn new() -> Self {
    Marked::default()
  }

  /// Text with no kind.
  pub fn plain(text: impl Into<String>) -> Self {
    let mut marked = Marked::new();
    marked.push(None, text);
    marked
  }

  /// Text of one kind.
  pub fn of(kind: Kind, text: impl Into<String>) -> Self {
    let mut marked = Marked::new();
    marked.push(Some(kind), text);
    marked
  }

  /// `text`, as `kind`, at the end.
  pub fn push(&mut self, kind: Option<Kind>, text: impl Into<String>) -> &mut Self {
    let text = text.into();
    if text.is_empty() {
      return self;
    }
    match self.segments.last_mut() {
      Some(last) if last.kind == kind => last.text.push_str(&text),
      _ => self.segments.push(Segment { text, kind }),
    }
    self
  }

  /// Plain `text` at the end.
  pub fn text_plain(&mut self, text: impl Into<String>) -> &mut Self {
    self.push(None, text)
  }

  /// `text` as `kind` at the end.
  pub fn text_as(&mut self, kind: Kind, text: impl Into<String>) -> &mut Self {
    self.push(Some(kind), text)
  }

  /// Another marked text at the end.
  pub fn append(&mut self, other: Marked) -> &mut Self {
    for segment in other.segments {
      self.push(segment.kind, segment.text);
    }
    self
  }

  /// The same text with every piece given `kind`: a block said as one thing, such as an exempted
  /// finding.
  pub fn all_as(self, kind: Kind) -> Marked {
    Marked::of(kind, self.text())
  }

  /// `parts`, with `separator` between each.
  pub fn join(parts: impl IntoIterator<Item = Marked>, separator: &str) -> Marked {
    let mut joined = Marked::new();
    for (at, part) in parts.into_iter().enumerate() {
      if at > 0 {
        joined.text_plain(separator);
      }
      joined.append(part);
    }
    joined
  }

  /// The text, as a host that reads only it receives it.
  pub fn text(&self) -> String {
    self
      .segments
      .iter()
      .map(|segment| segment.text.as_str())
      .collect()
  }

  pub fn segments(&self) -> &[Segment] {
    &self.segments
  }

  pub fn is_empty(&self) -> bool {
    self.segments.is_empty()
  }
}

impl From<String> for Marked {
  fn from(text: String) -> Self {
    Marked::plain(text)
  }
}

impl From<&str> for Marked {
  fn from(text: &str) -> Self {
    Marked::plain(text)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use pretty_assertions::assert_eq;

  #[test]
  fn the_segments_join_to_the_text_and_like_pieces_are_one() {
    let mut marked = Marked::new();
    marked
      .text_plain("Expected ")
      .text_as(Kind::Number, "401")
      .text_plain(" to equal ")
      .text_as(Kind::Number, "20")
      .text_as(Kind::Number, "0");
    assert_eq!(marked.text(), "Expected 401 to equal 200");
    assert_eq!(
      serde_json::to_value(marked.segments()).unwrap(),
      serde_json::json!([
        { "text": "Expected " },
        { "text": "401", "kind": "number" },
        { "text": " to equal " },
        { "text": "200", "kind": "number" },
      ])
    );
  }

  #[test]
  fn nothing_is_written_for_empty_text() {
    let mut marked = Marked::new();
    marked.text_as(Kind::Path, "").text_plain("");
    assert!(marked.is_empty());
    assert_eq!(
      Marked::join([Marked::plain("a"), Marked::of(Kind::Code, "b")], ", ").text(),
      "a, b"
    );
  }
}
