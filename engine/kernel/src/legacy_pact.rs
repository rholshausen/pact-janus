//! v1–v4 pact reading (plan task 3.1), via `pact_models` — the Phase 0.4 reuse decision
//! (`Documentation/reuse-inventory.md`).
//!
//! `pact_models::pact::load_pact_from_json` takes an already-parsed [`serde_json::Value`], not a
//! file path: no filesystem access, which is what the kernel needs on the WASM path (spike 1.2
//! finding: the engine is a guest with no ambient file system). Converting a v1–v4 pact's
//! matching rules into shapes is task 5.5's job, not this module's — this only proves the
//! document can be read.

use crate::contract::{ContractError, Problem};
use pact_models::pact::Pact;
use serde_json::Value;
use std::panic::RefUnwindSafe;

/// Parse a v1–v4 pact document already read into a [`Value`].
///
/// `source` is a diagnostic label (a file name, a URL, `"inline"`) that `pact_models` echoes back
/// into its own error messages; it is never used for I/O here.
pub fn read(
  source: &str,
  json: &Value,
) -> Result<Box<dyn Pact + Send + Sync + RefUnwindSafe>, ContractError> {
  pact_models::pact::load_pact_from_json(source, json).map_err(|err| ContractError::Invalid {
    problems: vec![Problem {
      pointer: String::new(),
      message: err.to_string(),
    }],
  })
}
