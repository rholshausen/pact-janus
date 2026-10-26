//! The CLI's palette: what a pass, a failure, a warning or a path looks like on a terminal.
//!
//! Text is styled where it is built and printed through [`crate::output`], whose streams strip the
//! styling again wherever colour does not belong: not a terminal, `NO_COLOR`, `TERM=dumb`,
//! `--no-colour`, unless `CLICOLOR_FORCE` asks for it. So styling only ever wraps text, never
//! changes it, and what a pipe or a file receives is exactly what it received before colour.
//!
//! One palette for every command, so a reader learns it once: green is a pass, red a failure,
//! yellow something to look at that did not fail, blue a location — the colour the plans give a
//! `resolve` ([`crate::highlight`]), since a mismatch's path is one — and dim what explains the
//! line above it.

use anstyle::{AnsiColor, Style};

const fn colour(colour: AnsiColor) -> Style {
  Style::new().fg_color(Some(anstyle::Color::Ansi(colour)))
}

pub const PASS: Style = colour(AnsiColor::Green);
pub const FAIL: Style = colour(AnsiColor::Red).bold();
pub const WARN: Style = colour(AnsiColor::Yellow);
pub const NOTE: Style = colour(AnsiColor::Cyan);
pub const PATH: Style = colour(AnsiColor::BrightBlue);
pub const HEADING: Style = Style::new().bold();
pub const DIM: Style = Style::new().dimmed();

/// `--help` and usage errors in cargo's colours, which a Rust user already reads: headings in green,
/// and what to type in cyan. The `[env: NAME]` notes stay plain: clap wraps a styled note with a
/// space left at the end of the line, which colour or no colour changes the help's text.
pub const HELP: clap::builder::Styles = clap::builder::Styles::styled()
  .header(colour(AnsiColor::Green).bold())
  .usage(colour(AnsiColor::Green).bold())
  .literal(colour(AnsiColor::Cyan).bold())
  .placeholder(colour(AnsiColor::Cyan))
  .error(FAIL)
  .valid(colour(AnsiColor::Cyan).bold())
  .invalid(colour(AnsiColor::Yellow).bold());

/// `text` in `style`, the style closed after it so nothing bleeds into what follows.
pub fn paint(style: Style, text: impl std::fmt::Display) -> String {
  format!("{style}{text}{style:#}")
}

/// A run's or a decision's status word, by what it means: `verified`, `published` and `pass` are
/// green, `failed` and `block` red, and anything else — `filtered`, `warn`, a word a newer engine
/// added — yellow, since it is neither.
pub fn status(word: &str) -> Style {
  match word.to_ascii_lowercase().as_str() {
    "verified" | "published" | "pass" | "passed" => PASS.bold(),
    "failed" | "block" | "blocked" => FAIL,
    _ => WARN.bold(),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use pretty_assertions::assert_eq;

  #[test]
  fn painting_only_wraps_the_text() {
    let painted = paint(FAIL, "FAILED");
    assert_eq!(painted, "\u{1b}[1m\u{1b}[31mFAILED\u{1b}[0m");
    assert_eq!(anstream::adapter::strip_str(&painted).to_string(), "FAILED");
  }

  #[test]
  fn a_status_is_coloured_by_what_it_means() {
    assert_eq!(status("VERIFIED"), PASS.bold());
    assert_eq!(status("failed"), FAIL);
    assert_eq!(status("BLOCK"), FAIL);
    assert_eq!(status("filtered"), WARN.bold());
  }
}
