//! The Janus contract model: read and write per design 2.5, including ADR 0011's canonical
//! writing rules and its strict/tolerant identification modes (plan task 3.1).

mod error;
mod identify;
mod model;
mod write;

pub use error::{ContractError, Problem};
pub use identify::{IdentifyMode, identify, identify_strict, identify_tolerant};
pub use model::{
  Contract, FORMAT, Interaction, Metadata, Party, RecordedSelection, RecordedVariant, Requirement,
  ResolvedState, ShapePart, SlotValue, State, Transport, ValuePart,
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
    let pointer = json_pointer(err.path());
    ContractError::Invalid {
      problems: vec![Problem {
        pointer,
        message: err.inner().to_string(),
      }],
    }
  })
}

/// Render a `serde_path_to_error::Path` as an RFC 6901 JSON pointer (contract-file spec §11).
fn json_pointer(path: &serde_path_to_error::Path) -> String {
  use serde_path_to_error::Segment;
  let mut pointer = String::new();
  for segment in path {
    pointer.push('/');
    let raw = match segment {
      Segment::Seq { index } => index.to_string(),
      Segment::Map { key } => key.clone(),
      Segment::Enum { variant } => variant.clone(),
      Segment::Unknown => "?".to_string(),
    };
    pointer.push_str(&raw.replace('~', "~0").replace('/', "~1"));
  }
  pointer
}
