//! The Janus contract model: read and write per design 2.5, including ADR 0011's canonical
//! writing rules and its strict/tolerant identification modes (plan task 3.1).

mod error;
mod identify;
mod model;
mod write;

pub use crate::common::{Requirement, State, Transport};
pub use error::{ContractError, Problem};
pub use identify::{IdentifyMode, identify, identify_strict, identify_tolerant};
pub use model::{
  Contract, FORMAT, Interaction, Metadata, Party, RecordedSelection, RecordedVariant, ResolvedState,
  ShapePart, SlotValue, ValuePart,
};
pub use write::write_canonical;

/// Read a Janus contract from bytes, identifying it first (contract-file spec §2.3) so a
/// non-contract degrades to a clear [`ContractError::NotAContract`] rather than a misparse.
pub fn read(bytes: &[u8], mode: IdentifyMode) -> Result<Contract, ContractError> {
  if !identify(bytes, mode) {
    return Err(ContractError::NotAContract);
  }
  let mut de = serde_json::Deserializer::from_slice(bytes);
  serde_path_to_error::deserialize(&mut de).map_err(|err| {
    let pointer = crate::error::json_pointer(err.path());
    ContractError::Invalid {
      problems: vec![Problem {
        pointer,
        message: err.inner().to_string(),
      }],
    }
  })
}
