//! The structured error a malformed interaction specification produces (engine protocol spec
//! §10.2, shape-language spec §5).

pub use crate::error::Problem;

/// The document a caller submitted as an interaction specification was not well-formed. Carries
/// every violation found, not just the first (`add-interaction`'s stated goal: "errors good
/// enough to surface through the protocol to a DSL user", engine protocol spec §8.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InteractionSpecError {
  pub problems: Vec<Problem>,
}

impl InteractionSpecError {
  /// The engine-error code (engine protocol spec §10.2): every failure here is a document the
  /// user authored being invalid, never a missing component or an engine bug.
  pub fn code(&self) -> &'static str {
    "interaction-invalid"
  }
}

impl std::fmt::Display for InteractionSpecError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "invalid interaction specification ({} problem(s))",
      self.problems.len()
    )
  }
}

impl std::error::Error for InteractionSpecError {}
