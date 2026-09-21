//! The provider-shape document (design 2.8 §2, `schemas/v1/provider-shape.schema.json`).
//!
//! Every struct here mirrors one of the schema's `$defs`, in the schema's own property order, for
//! the reason [`crate::contract::model`] gives: `serde_json` serializes a struct's fields in
//! declaration order, so a canonical member order falls out of the type rather than a hand-rolled
//! serializer. Shapes stay `Value` — design 2.2's document, opaque here (§2.4).

use crate::contract::Party;
use crate::error::{Problem, json_pointer};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// The format token this module reads and writes (spec §2.4).
pub const FORMAT: &str = "janus-provider-shape/1";

/// One part's shapes: slot name -> shape node, opaque here (spec §2.4).
pub type ShapePart = BTreeMap<String, Value>;

/// What a provider publishes about the shapes it may produce (spec §2.1). Not a contract: no
/// consumer name, no evidence, no variants.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderShape {
  #[serde(rename = "$format")]
  pub format: String,
  #[serde(rename = "$schema", skip_serializing_if = "Option::is_none")]
  pub schema: Option<String>,
  pub provider: Party,
  /// Document-level default, overridable per interaction (spec §2.3). Never an input to the walk:
  /// §2.3 forbids deciding a different verdict because a shape arrived by a different route.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub provenance: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub metadata: Option<Metadata>,
  pub interactions: Vec<ProviderInteraction>,
}

/// One operation's published shape, matched to a consumer interaction by `description` plus
/// `states` names (spec §2.2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderInteraction {
  pub description: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub states: Option<Vec<StateRef>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub provenance: Option<String>,
  /// Open provenance detail (spec §2.4), not interpreted by the walk.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub source: Option<BTreeMap<String, Value>>,
  /// Part name -> slot name -> shape.
  pub parts: BTreeMap<String, ShapePart>,
}

/// A provider state by name only — a provider shape does not know a given consumer's parameter
/// values (spec §2.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateRef {
  pub name: String,
}

/// How the file came to exist (spec §2.4), outside anything the walk reads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Metadata {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub writer: Option<BTreeMap<String, String>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub created: Option<String>,
}

impl ProviderShape {
  /// The provenance in force for one interaction (spec §2.3): the interaction's own, else the
  /// document's, else the schema's `authored` default.
  pub fn provenance_of<'a>(&'a self, interaction: &'a ProviderInteraction) -> &'a str {
    interaction
      .provenance
      .as_deref()
      .or(self.provenance.as_deref())
      .unwrap_or("authored")
  }

  /// The entry matching an interaction identified by `description` plus its state *names* (spec
  /// §2.2) — contract spec §4.2's identity, with the consumer's state `params` ignored because a
  /// provider shape never carries them.
  pub fn find(&self, description: &str, states: &[String]) -> Option<&ProviderInteraction> {
    self
      .interactions
      .iter()
      .find(|candidate| candidate.description == description && candidate.state_names() == states)
  }
}

impl ProviderInteraction {
  pub fn state_names(&self) -> Vec<String> {
    self
      .states
      .iter()
      .flatten()
      .map(|state| state.name.clone())
      .collect()
  }
}

/// Read a provider shape from bytes (spec §8's error table: `contract-invalid` for a document
/// that is not one, `contract-version-unsupported` for a major this checker does not implement).
pub fn read_provider_shape(bytes: &[u8]) -> Result<ProviderShape, crate::contract::ContractError> {
  use crate::contract::ContractError;

  let format = serde_json::from_slice::<Value>(bytes)
    .ok()
    .and_then(|value| value.get("$format").and_then(Value::as_str).map(str::to_string));
  match format.as_deref() {
    None => return Err(ContractError::NotAContract),
    Some(FORMAT) => {}
    Some(other) => {
      // The major is the whole of `janus-provider-shape/<major>`: a different name is not a
      // provider shape at all, a different major is one this checker cannot read (§8).
      let unsupported_major = other.starts_with("janus-provider-shape/");
      return Err(if unsupported_major {
        ContractError::VersionUnsupported {
          format: other.to_string(),
        }
      } else {
        ContractError::NotAContract
      });
    }
  }

  let mut de = serde_json::Deserializer::from_slice(bytes);
  serde_path_to_error::deserialize(&mut de).map_err(|err| ContractError::Invalid {
    problems: vec![Problem {
      pointer: json_pointer(err.path()),
      message: err.inner().to_string(),
    }],
  })
}
