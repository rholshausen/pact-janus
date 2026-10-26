//! Colour for text the engine wrote, from the segments it sends beside it (engine-protocol spec
//! §2.7, ADR 0054): each piece styled by its `kind`. The CLI declares the `segments` capability
//! ([`crate::engine::start`]), so the plans `explain` and `--explain-failures` print, the page
//! `janus check` prints and each mismatch's message all arrive marked. Nothing here parses the text:
//! the engine said what each piece is.
//!
//! The text is never changed, only wrapped, and [`crate::output`] strips the styling again wherever
//! colour does not belong. Text whose segments are missing, or do not join to it, is printed as it
//! came.

use crate::style::{self, paint};
use anstyle::{AnsiColor, Style};
use serde_json::Value;

/// `text`, styled by `segments`. Each line of a piece is styled on its own, so a caller may indent
/// the lines it prints, and no style is left open at the end of a line.
pub fn styled(text: &str, segments: &Value) -> String {
  let Some(pieces) =
    pieces(segments).filter(|pieces| pieces.iter().map(|(_, piece)| *piece).collect::<String>() == text)
  else {
    return text.to_string();
  };
  let mut out = String::with_capacity(text.len() * 2);
  for (kind, piece) in pieces {
    let style = kind.map(look).unwrap_or_default();
    for (at, line) in piece.split('\n').enumerate() {
      if at > 0 {
        out.push('\n');
      }
      if line.is_empty() || style == Style::new() {
        out.push_str(line);
      } else {
        out.push_str(&paint(style, line));
      }
    }
  }
  out
}

/// `member` of `document`, styled by its `<member>-segments`.
pub fn member(document: &Value, member: &str) -> String {
  styled(
    document[member].as_str().unwrap_or_default(),
    &document[format!("{member}-segments")],
  )
}

fn pieces(segments: &Value) -> Option<Vec<(Option<&str>, &str)>> {
  segments
    .as_array()?
    .iter()
    .map(|segment| Some((segment["kind"].as_str(), segment["text"].as_str()?)))
    .collect()
}

/// What each kind looks like, from the CLI's palette ([`crate::style`]): outcomes as everywhere
/// else, a path blue, values in the colours most highlighters give them, the plan's structure bold
/// and its annotations quiet. A kind this CLI does not know is plain.
fn look(kind: &str) -> Style {
  let colour = |colour: AnsiColor| Style::new().fg_color(Some(colour.into()));
  match kind {
    "pass" => style::PASS,
    "fail" => style::FAIL,
    "warn" | "review" => style::WARN.bold(),
    "quiet" | "annotation" | "arrow" => style::DIM,
    "path" => style::PATH,
    "heading" | "code" | "label" | "operator" => style::HEADING,
    "string" => colour(AnsiColor::Green),
    "number" | "literal" => colour(AnsiColor::Yellow),
    "action" => colour(AnsiColor::Cyan).bold(),
    _ => Style::new(),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use pretty_assertions::assert_eq;
  use serde_json::json;

  const TEXT: &str = "(\n  %expect:size (\n    'two\nlines'\n  ) => ERROR(Expected 1 item)\n)";

  fn segments() -> Value {
    json!([
      { "text": "(\n  " },
      { "text": "%expect:size", "kind": "action" },
      { "text": " (\n    " },
      { "text": "'two\nlines'", "kind": "string" },
      { "text": "\n  ) " },
      { "text": "=>", "kind": "arrow" },
      { "text": " " },
      { "text": "ERROR(Expected 1 item)", "kind": "fail" },
      { "text": "\n)" },
      { "text": "", "kind": "a-kind-from-a-newer-engine" },
    ])
  }

  #[test]
  fn the_styling_is_only_ever_added() {
    assert_eq!(
      anstream::adapter::strip_str(&styled(TEXT, &segments())).to_string(),
      TEXT
    );
  }

  #[test]
  fn each_line_is_styled_on_its_own() {
    let out = styled(TEXT, &segments());
    assert!(
      out.contains("\u{1b}[32m'two\u{1b}[0m\n\u{1b}[32mlines'\u{1b}[0m"),
      "{out:?}"
    );
    assert!(
      out.contains(&paint(style::FAIL, "ERROR(Expected 1 item)")),
      "{out:?}"
    );
    for line in out.lines() {
      let closed = line
        .rfind("\u{1b}[")
        .is_none_or(|at| line[at..].starts_with("\u{1b}[0m"));
      assert!(closed, "a style is left open at the end of {line:?}");
    }
  }

  /// Segments that are not the text say nothing about it, so the text is printed as it came.
  #[test]
  fn segments_that_do_not_join_to_the_text_are_ignored() {
    assert_eq!(styled("other text", &segments()), "other text");
    assert_eq!(styled(TEXT, &Value::Null), TEXT);
  }

  #[test]
  fn a_member_is_styled_by_its_siblings_segments() {
    let mismatch = json!({
      "message": "Expected 401 to equal 200",
      "message-segments": [
        { "text": "Expected " }, { "text": "401", "kind": "number" },
        { "text": " to equal " }, { "text": "200", "kind": "number" },
      ],
    });
    let yellow = Style::new().fg_color(Some(AnsiColor::Yellow.into()));
    assert_eq!(
      member(&mismatch, "message"),
      format!(
        "Expected {} to equal {}",
        paint(yellow, "401"),
        paint(yellow, "200")
      )
    );
  }
}
