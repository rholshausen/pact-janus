//! Colour for the plans `explain` and `verify --explain-failures` print (plan-grammar spec §3.3):
//! the kernel's tokens of the text the engine rendered, each styled by what it is. The text itself
//! is never changed, only wrapped, and [`crate::output::out_styled`] strips the styling again
//! wherever colour does not belong: not a terminal, `NO_COLOR`, `TERM=dumb`, unless
//! `CLICOLOR_FORCE` asks for it.

use anstyle::{AnsiColor, Style};
use pact_janus_kernel::plan::{TokenKind, tokens};

/// `text`, a pretty or executed form, with each token styled. Each line is styled on its own, so a
/// caller may indent the lines it prints.
pub fn plan(text: &str) -> String {
  let mut out = String::with_capacity(text.len() * 2);
  for token in tokens(text) {
    let style = style(token.kind);
    for (at, piece) in token.text.split('\n').enumerate() {
      if at > 0 {
        out.push('\n');
      }
      if piece.is_empty() || style == Style::new() {
        out.push_str(piece);
      } else {
        out.push_str(&format!("{style}{piece}{style:#}"));
      }
    }
  }
  out
}

/// What each kind looks like: the structure in bold, data in the colours most highlighters give
/// it, the `=>` and annotations quiet, and every failure red so the failing branch stands out.
fn style(kind: TokenKind) -> Style {
  let colour = |colour: AnsiColor| Style::new().fg_color(Some(colour.into()));
  match kind {
    TokenKind::Container | TokenKind::Operator => Style::new().bold(),
    TokenKind::Action => colour(AnsiColor::Cyan).bold(),
    TokenKind::Resolve => colour(AnsiColor::BrightBlue),
    TokenKind::String => colour(AnsiColor::Green),
    TokenKind::Number | TokenKind::Literal => colour(AnsiColor::Yellow),
    TokenKind::Annotation | TokenKind::Arrow => Style::new().dimmed(),
    TokenKind::Pass => colour(AnsiColor::Green),
    TokenKind::Fail | TokenKind::Error => colour(AnsiColor::Red),
    TokenKind::Plain => Style::new(),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use pretty_assertions::assert_eq;

  const EXECUTED: &str = "(\n  :body (\n    %expect:size (\n      $.items => [],\n      'two\nlines'\n    ) \
                          => ERROR(Expected at least 1 item but got 0)\n  ) => BOOL(false)\n)";

  #[test]
  fn the_styling_is_only_ever_added() {
    assert_eq!(
      anstream::adapter::strip_str(&plan(EXECUTED)).to_string(),
      EXECUTED
    );
  }

  #[test]
  fn each_line_is_styled_on_its_own() {
    let styled = plan(EXECUTED);
    assert!(
      styled.contains("\u{1b}[32m'two\u{1b}[0m\n\u{1b}[32mlines'\u{1b}[0m"),
      "{styled:?}"
    );
    for line in styled.lines() {
      let closed = line
        .rfind("\u{1b}[")
        .is_none_or(|at| line[at..].starts_with("\u{1b}[0m"));
      assert!(closed, "a style is left open at the end of {line:?}");
    }
  }

  #[test]
  fn a_failure_is_red_and_a_pass_is_green() {
    let styled = plan("(\n  :a (\n    1\n  ) => BOOL(true),\n  :b (\n    2\n  ) => BOOL(false)\n)");
    assert!(styled.contains("\u{1b}[32mBOOL(true)\u{1b}[0m"), "{styled:?}");
    assert!(styled.contains("\u{1b}[31mBOOL(false)\u{1b}[0m"), "{styled:?}");
  }
}
