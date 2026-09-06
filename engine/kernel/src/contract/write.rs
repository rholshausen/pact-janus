//! The canonical writer (contract-file spec §2.4, ADR 0018).
//!
//! Canonical bytes are **compact**: no insignificant whitespace at all. `$format` is `Contract`'s
//! first field (never skipped: it is a plain `String`, not an `Option`), so a compact writer's
//! output starts `{"$format":...` with nothing in between — the eleven-byte identification prefix
//! (contract-file spec §2.3) falls out of ordinary serialization, with no special-cased splice for
//! one member. Readability is a display concern for whatever renders a contract to a human, never
//! a property of the file itself; `jq` or any JSON formatter reformats losslessly on demand.

use super::error::ContractError;
use super::model::Contract;

/// Serialize `contract` to its canonical bytes (contract-file spec §2.4): UTF-8, no BOM, LF-only,
/// compact, members in schema order, trailing newline.
pub fn write_canonical(contract: &Contract) -> Result<Vec<u8>, ContractError> {
  let mut bytes = serde_json::to_vec(contract).map_err(|err| ContractError::Invalid {
    problems: vec![super::error::Problem {
      pointer: String::new(),
      message: err.to_string(),
    }],
  })?;
  bytes.push(b'\n');
  Ok(bytes)
}
