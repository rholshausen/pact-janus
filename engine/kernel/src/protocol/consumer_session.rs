//! Request bodies for `consumer-session/*` (engine-protocol spec §8.2). Result bodies are built
//! directly as `Value` at the call site — they're small enough that a second set of typed
//! structs would only restate the schema, not earn it.

use crate::contract::Party;
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub struct Create {
  pub config: SessionConfig,
  /// Tell each arrival's outcome on a stream the answer names (spec §9.6, staged plan S3.5).
  #[serde(default)]
  pub events: bool,
}

#[derive(Debug, Deserialize)]
pub struct SessionConfig {
  pub consumer: Party,
  pub provider: Party,
  /// The session-wide sampling policy layer (variant-semantics spec §3.8 layer 2).
  #[serde(default)]
  pub policy: Option<Value>,
  /// What a contract records redacted beyond the transport's credentials, and which of those it keeps
  /// (ADR 0074 decision 3): `{ add, keep }`, each a list of paths.
  #[serde(default)]
  pub redaction: Option<Value>,
  /// The project's declared components, as the host's loader resolved them (spec §8.2,
  /// component-interfaces spec §10.2). Loaded before `create` answers.
  #[serde(default)]
  pub components: Vec<Value>,
}

#[derive(Debug, Deserialize)]
pub struct AddInteraction {
  pub session: String,
  pub interaction: Value,
}

/// `add-pact-interaction` (ADR 0066): the builder calls a consumer library makes to `pact_ffi`,
/// which the engine applies and converts as `upgrade` does.
#[derive(Debug, Deserialize)]
pub struct AddPactInteraction {
  pub session: String,
  pub pact: Value,
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
pub struct StartTransport {
  pub session: String,
  /// Transport component name (component-interfaces spec, design 2.6): an open vocabulary,
  /// `"http"` today.
  pub transport: String,
  #[serde(default)]
  pub options: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub struct ServeVariant {
  pub session: String,
  pub handle: String,
  pub variant: String,
}

/// `consumer-session/report-variant` (ADR 0064): the test's verdict on one variant.
#[derive(Debug, Deserialize)]
pub struct ReportVariant {
  pub session: String,
  pub handle: String,
  pub variant: String,
  pub status: String,
  #[serde(default)]
  pub reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Finalise {
  pub session: String,
  /// Write the contract to a file through the engine's host (spec §8.2, ADR 0065).
  #[serde(default)]
  pub write: Option<ContractWrite>,
}

/// `finalise`'s `write`: the file, and whether to merge into it or overwrite it.
#[derive(Debug, Deserialize)]
pub struct ContractWrite {
  pub path: String,
  #[serde(default)]
  pub mode: Option<String>,
}
