//! Colour for text the engine wrote: the plans `explain` and `verify --explain-failures` print
//! (plan-grammar spec §3.3), and the page `janus check` prints (engine-protocol spec §8.6). The text
//! itself is never changed, only wrapped, and [`crate::output`] strips the styling again wherever
//! colour does not belong.

use crate::style::{self, paint};
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

/// `subsumption/decide`'s `text`, the `can-i-deploy` page, with its markers coloured. They are the
/// specification's, not prose read for meaning: a pair's header line opens with `✗`, `?` or `✓`
/// (subsumption-check spec §6.4); a finding is unprefixed, `? ` for a review or `! ` for an advisory
/// (§6.5), and `~ ` when an exemption silenced it; a reason is `✗`, `!` or `-`; and the decision is
/// `=> BLOCK`, `=> WARN` or `=> PASS` (engine-protocol spec §8.6). A line this does not recognise is
/// printed as it came.
pub fn page(text: &str) -> String {
  let mut exempt = false;
  text
    .split('\n')
    .map(|line| {
      // An exempted finding's lines, under its header, are quiet with it.
      if exempt && line.starts_with("    ") {
        return paint(style::DIM, line);
      }
      exempt = false;
      page_line(line, &mut exempt)
    })
    .collect::<Vec<_>>()
    .join("\n")
}

fn page_line(line: &str, exempt: &mut bool) -> String {
  // A pair's header line.
  for (marker, look) in [
    ("✗ ", style::FAIL),
    ("? ", style::WARN.bold()),
    ("✓ ", style::PASS.bold()),
  ] {
    if let Some(rest) = line.strip_prefix(marker) {
      return format!("{}{}", paint(look, marker), paint(style::HEADING, rest));
    }
  }
  // The page's verdict, and each pair's decision.
  if let Some((word, rest)) = verdict(line) {
    return match rest.split_once(" — ") {
      Some((counts, policy)) => format!(
        "{}{counts}{}",
        paint(style::status(word), word),
        paint(style::DIM, format!(" — {policy}"))
      ),
      None => format!("{}{rest}", paint(style::status(word), word)),
    };
  }
  if let Some(decision) = line.strip_prefix("  => ") {
    let word = decision.split(':').next().unwrap_or_default();
    return format!("  {}", paint(style::status(word), format!("=> {decision}")));
  }
  if let Some(rest) = line.strip_prefix("  verification: ") {
    let end = rest
      .find([' ', ','])
      .filter(|_| !rest.starts_with("no "))
      .unwrap_or(rest.len());
    let (status, tail) = rest.split_at(end);
    return format!("  verification: {}{tail}", paint(style::status(status), status));
  }
  // A finding's header line: its marker, and where it is.
  if let Some(body) = line.strip_prefix("  ") {
    let (marker, look, rest) = match body.split_at_checked(2) {
      Some(("~ ", _)) => {
        *exempt = true;
        return paint(style::DIM, line);
      }
      Some(("? ", rest)) => ("? ", style::WARN.bold(), rest),
      Some(("! ", rest)) => ("! ", style::WARN.bold(), rest),
      _ => ("", Style::new(), body),
    };
    if rest.starts_with("interaction '") && rest.ends_with(':') {
      let rest = match rest.rfind(" $") {
        Some(at) => format!(
          "{} {}:",
          &rest[..at],
          paint(style::PATH, &rest[at + 1..rest.len() - 1])
        ),
        None => rest.to_string(),
      };
      return format!("  {}{rest}", paint(look, marker));
    }
    // A reason.
    match body.split_at_checked(2) {
      Some(("✗ ", rest)) => return format!("  {}{rest}", paint(style::FAIL, "✗ ")),
      Some(("! ", rest)) => return format!("  {}{rest}", paint(style::WARN.bold(), "! ")),
      Some(("- ", _)) => return paint(style::DIM, line),
      _ => {}
    }
  }
  line.to_string()
}

/// `WARN: 0 pairs blocked, …` -> (`WARN`, `: 0 pairs blocked, …`).
fn verdict(line: &str) -> Option<(&str, &str)> {
  ["BLOCK", "WARN", "PASS"]
    .into_iter()
    .find(|word| line.strip_prefix(word).is_some_and(|rest| rest.starts_with(':')))
    .map(|word| line.split_at(word.len()))
}

#[cfg(test)]
mod tests {
  use super::*;
  use pretty_assertions::assert_eq;

  const PAGE: &str = "✗ web-app is not compatible with order-service
  interaction 'a request for an order', response body $.items:
    provider may produce an empty list, or up to 3 items
    consumer has only tested at least one item
  ? interaction 'a request for an order', response body $.status (shipped):
    provider may produce: 'CANCELLED' | 'PENDING'
    consumer has only tested exactly 'PENDING'
  ~ interaction 'a request for an order', response body $.note:
    provider may produce any string
    consumer has only tested exactly 'x'
    exempted until 2026-12-01: the note is advisory
  verification: verified (2 of 2 variants)
  ✗ the provider failed verification
  ! no verification result was supplied for this pair
  - 1 of 2 interactions has no published provider shape and was not checked
  => BLOCK: web-app -> order-service

BLOCK: 1 pair blocked, 0 warned, 0 passed — policy on-finding block, on-review warn";

  #[test]
  fn the_page_is_only_ever_wrapped() {
    assert_eq!(anstream::adapter::strip_str(&page(PAGE)).to_string(), PAGE);
  }

  #[test]
  fn the_pages_markers_are_coloured_by_what_they_say() {
    let styled = page(PAGE);
    let lines: Vec<&str> = styled.lines().collect();
    assert!(lines[0].starts_with(&paint(style::FAIL, "✗ ")), "{:?}", lines[0]);
    assert!(
      lines[1].ends_with(&format!("{}:", paint(style::PATH, "$.items"))),
      "{:?}",
      lines[1]
    );
    assert!(
      lines[4].starts_with(&format!("  {}", paint(style::WARN.bold(), "? "))),
      "{:?}",
      lines[4]
    );
    assert!(
      lines[4].contains(&paint(style::PATH, "$.status (shipped)")),
      "{:?}",
      lines[4]
    );
    for exempted in &lines[7..11] {
      assert!(exempted.starts_with(&style::DIM.to_string()), "{exempted:?}");
    }
    assert!(
      lines[11].contains(&paint(style::PASS.bold(), "verified")),
      "{:?}",
      lines[11]
    );
    assert_eq!(
      lines[15],
      format!("  {}", paint(style::FAIL, "=> BLOCK: web-app -> order-service"))
    );
    assert!(
      lines[17].starts_with(&paint(style::FAIL, "BLOCK")),
      "{:?}",
      lines[17]
    );
    assert!(lines[17].ends_with(&paint(style::DIM, " — policy on-finding block, on-review warn")));
  }

  #[test]
  fn a_missing_verification_is_yellow_as_a_whole_phrase() {
    let styled = page("  verification: no result supplied");
    assert_eq!(
      styled,
      format!(
        "  verification: {}",
        paint(style::WARN.bold(), "no result supplied")
      )
    );
  }

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
