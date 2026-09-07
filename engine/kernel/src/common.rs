//! Document fragments shared between the interaction spec (plan task 3.2) and the recorded
//! contract (contract-file spec, plan task 3.1): a transport descriptor, a provider-state
//! binding, and a component requirement. These are the same documents in both places — the
//! contract-file spec's §4.3, §6 and §7 describe exactly what an interaction spec carries before
//! it is recorded — so they live once here rather than being re-derived on each side.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// How the parts crossed the wire. An open descriptor: `kind`/`mode` are fixed here, everything
/// else belongs to the transport component (component-interfaces spec, design 2.6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transport {
  pub kind: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub mode: Option<String>,
}

/// A provider state. `params` holds literal parameters; `variant_params` holds bindings whose
/// value depends on the running variant (variant-semantics spec §6.2, opaque here). Bindings
/// appear once, on the interaction (contract-file spec §6) — this is that binding form, not the
/// per-variant resolved form (contract-file spec's `ResolvedState`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct State {
  pub name: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub params: Option<BTreeMap<String, Value>>,
  #[serde(rename = "variant-params", skip_serializing_if = "Option::is_none")]
  pub variant_params: Option<Vec<Value>>,
}

/// A component this interaction cannot be matched without, at a major version or above
/// (contract-file spec §7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Requirement {
  pub component: String,
  #[serde(rename = "min-version", skip_serializing_if = "Option::is_none")]
  pub min_version: Option<u64>,
}
