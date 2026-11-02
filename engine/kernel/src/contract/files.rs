//! Writing a contract file, merging into what it holds (engine-protocol spec §8.2, ADR 0065).
//!
//! A file and a lock that holds across processes are the embedding's to give, as the configuration
//! loader is (ADR 0055): a kernel that builds for `wasm32-wasip2` has neither. So the file arrives as a
//! [`ContractWriter`] the embedding registers. The writer only opens, locks, reads and writes; what to
//! write is decided here, from what the file held, by [`next_bytes`]. With no writer, a `write` is
//! `component-unavailable` naming [`WRITER`], and `engine/hello` declares no `contract-writing`.

use super::merge::{self, Conflict};
use super::{Contract, ContractError, IdentifyMode, write_canonical};
use crate::component::ComponentError;
use crate::protocol::EngineError;
use serde_json::{Value, json};

/// What the writer is called in `component-unavailable` and `component-failed`.
pub const WRITER: &str = "contract-writer";

/// What a writer calls with the file's contents: the bytes to replace them with, or why not.
pub type Next<'a> = dyn FnMut(&[u8]) -> Result<Vec<u8>, EngineError> + 'a;

/// One file, read and rewritten under one exclusive lock.
pub trait ContractWriter: Send + Sync {
  /// Opens the file at `path`, creating it and its directory, takes an exclusive lock on it, waiting
  /// as long as that takes, and calls `next` with what the file holds (nothing, when it is new). The
  /// file's contents become the bytes `next` returns, rewritten in place, before the lock is released.
  /// An error `next` returns is returned as it is, and the file is left as it was. Whether `path` is
  /// absolute is the writer's to judge, a relative one being `malformed-frame`.
  fn update(&self, path: &str, next: &mut Next<'_>) -> Result<(), EngineError>;
}

/// `finalise`'s `write.mode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
  Merge,
  Overwrite,
}

impl Mode {
  /// `merge` when absent; an unknown mode is `malformed-frame`.
  pub fn parse(mode: Option<&str>) -> Result<Mode, EngineError> {
    match mode {
      None | Some("merge") => Ok(Mode::Merge),
      Some("overwrite") => Ok(Mode::Overwrite),
      Some(other) => Err(EngineError::malformed_frame(format!(
        "write.mode: '{other}' is neither 'merge' nor 'overwrite'"
      ))),
    }
  }
}

/// The bytes to write over `existing`, the file's contents: `contract`, merged into what the file
/// holds unless overwriting, in canonical form.
pub fn next_bytes(
  path: &str,
  existing: &[u8],
  contract: &Contract,
  mode: Mode,
) -> Result<Vec<u8>, EngineError> {
  let blank = existing.iter().all(u8::is_ascii_whitespace);
  let written = if mode == Mode::Overwrite || blank {
    contract.clone()
  } else {
    let held = super::read(existing, IdentifyMode::Tolerant).map_err(|err| unreadable(path, err))?;
    merge::merge(held, contract.clone()).map_err(|conflict| conflicted(path, conflict))?
  };
  write_canonical(&written).map_err(|err| unreadable(path, err))
}

/// `component-failed` naming the writer, for a file it could not create, read or write.
pub fn failed(path: &str, err: &std::io::Error) -> EngineError {
  let error = ComponentError {
    code: "contract-unwritable".to_string(),
    category: "component".to_string(),
    message: format!("could not write the contract to '{path}'"),
    source: None,
    details: Some(Box::new(json!({ "path": path }))),
    chain: None,
  }
  .caused_by(crate::chain::layers_of(err, Some));
  EngineError::component_failed(WRITER, &error)
}

/// A file that holds something other than a contract, as reading one fails, with its path.
fn unreadable(path: &str, err: ContractError) -> EngineError {
  let mut error = match err {
    ContractError::Invalid { problems } => EngineError::document_invalid(&problems),
    ContractError::NotAContract => EngineError::document_invalid(&[super::Problem {
      pointer: String::new(),
      message: "no '$format' member: not a Janus contract".to_string(),
    }]),
    ContractError::VersionUnsupported { format } => {
      let mut error = EngineError::not_a_contract(0, Some(&format));
      error.details = Some(Box::new(json!({ "found": format, "expected": super::FORMAT })));
      error
    }
  };
  error.message = format!(
    "the file at '{path}' holds no contract this engine can merge into: {}",
    error.message
  );
  let details = error.details.get_or_insert_with(|| Box::new(json!({})));
  if let Value::Object(members) = details.as_mut() {
    members.insert("path".to_string(), Value::from(path));
  }
  error
}

/// `contract-conflict` (spec §10.2): the file cannot take the session's contract.
fn conflicted(path: &str, conflict: Conflict) -> EngineError {
  let (message, details) = match conflict {
    Conflict::Parties { consumer, provider } => (
      format!("the file at '{path}' holds the contract between '{consumer}' and '{provider}'"),
      json!({ "path": path, "consumer": consumer, "provider": provider }),
    ),
    Conflict::Interactions(interactions) => {
      let named: Vec<String> = interactions
        .iter()
        .map(|i| format!("'{}'", i.description))
        .collect();
      let listed: Vec<Value> = interactions
        .iter()
        .map(|i| match &i.states {
          Some(states) => json!({ "description": i.description, "states": states }),
          None => json!({ "description": i.description }),
        })
        .collect();
      (
        format!(
          "the file at '{path}' records {} differently from this session: {}",
          if named.len() == 1 {
            "an interaction"
          } else {
            "interactions"
          },
          named.join(", ")
        ),
        json!({ "path": path, "interactions": listed }),
      )
    }
  };
  EngineError {
    code: "contract-conflict".to_string(),
    category: "document".to_string(),
    message,
    details: Some(Box::new(details)),
    chain: None,
  }
}
