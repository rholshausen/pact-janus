//! Structured errors for the contract model (contract-file spec §11).
//!
//! Deliberately not `anyhow`: CLAUDE.md's "errors are values" rule applies to everything that will
//! eventually cross the engine boundary, and a contract-model error is exactly that kind of value —
//! later protocol wiring lifts a `code()` straight into an `EngineError` frame (engine-protocol spec
//! §10) without re-deriving the mapping.

use serde::Serialize;

/// One position where a document disagreed with what it claimed to be.
///
/// `pointer` is an RFC 6901 JSON pointer, so a reader can jump straight to the offending member
/// instead of bisecting the file (contract-file spec §11: "an error that names a position is the
/// difference between a fix and a bisect").
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Problem {
  pub pointer: String,
  pub message: String,
}

/// A structured failure reading or writing a contract-shaped document — a Janus contract or, via
/// [`crate::legacy_pact`], a v1–v4 pact (engine protocol spec's `ContractSource` covers both).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContractError {
  /// Identification (contract-file spec §2.3) found no `$format` member: this is not a Janus
  /// contract, in either the strict or the tolerant sense.
  NotAContract,
  /// `$format` names a major version this engine does not implement.
  VersionUnsupported { format: String },
  /// Structurally invalid: schema violation, duplicate interaction identity, or (for a wrapped
  /// v1–v4 pact) a parse failure `pact_models` reported.
  Invalid { problems: Vec<Problem> },
}

impl ContractError {
  /// The engine-error code (engine-protocol spec §10.2, contract-file spec §11).
  pub fn code(&self) -> &'static str {
    match self {
      ContractError::NotAContract => "contract-invalid",
      ContractError::VersionUnsupported { .. } => "contract-version-unsupported",
      ContractError::Invalid { .. } => "contract-invalid",
    }
  }
}

impl std::fmt::Display for ContractError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      ContractError::NotAContract => write!(f, "not a Janus contract, or unreadable as one"),
      ContractError::VersionUnsupported { format } => {
        write!(f, "unsupported contract format: {format}")
      }
      ContractError::Invalid { problems } => {
        write!(f, "invalid contract ({} problem(s))", problems.len())
      }
    }
  }
}

impl std::error::Error for ContractError {}
