//! Request bodies for `consumer-session/*` (engine-protocol spec §8.2). Result bodies are built
//! directly as `Value` at the call site — they're small enough that a second set of typed
//! structs would only restate the schema, not earn it.

use crate::contract::Party;
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub struct Create {
  pub config: SessionConfig,
}

#[derive(Debug, Deserialize)]
pub struct SessionConfig {
  pub consumer: Party,
  pub provider: Party,
  /// The session-wide sampling policy layer (variant-semantics spec §3.8 layer 2).
  #[serde(default)]
  pub policy: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub struct AddInteraction {
  pub session: String,
  pub interaction: Value,
}

#[derive(Debug, Deserialize)]
pub struct Variants {
  pub session: String,
  pub handle: String,
  /// Per-call sampling policy override (variant-semantics spec §3.8), for a host offering an
  /// `--exhaustive`-style switch.
  #[serde(default)]
  pub policy: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub struct ServeVariant {
  pub session: String,
  pub handle: String,
  pub variant: String,
}

#[derive(Debug, Deserialize)]
pub struct Finalise {
  pub session: String,
}
