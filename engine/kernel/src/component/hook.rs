//! The hook interface's request/result documents and native-binding trait (component-interfaces
//! spec §8, `schemas/v1/hook.schema.json`): one operation, `hook/invoke`, carrying the same two
//! documents every other hook implementation carries (lifecycle-hooks spec §2.3).
//!
//! The *system* around it — which points exist, how they are ordered, what a failure means, how
//! configuration and secrets reach them — is design 2.7's and lives in [`crate::hooks`]. A hook
//! component is one implementation of a hook point, not the only one, and this module is
//! deliberately just the call.

use super::ComponentError;
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct Invoke {
  pub point: String,
  /// Design 2.7's `HookContext`, assembled by the engine for this point.
  pub context: Value,
  /// The hook entry's own configuration, already interpolated — a component receives values, never
  /// templates (ADR 0014).
  pub config: Option<Value>,
  /// How long this invocation has (lifecycle-hooks spec §5.4). Not in the schema's request body,
  /// which carries it inside `context` as `deadline-ms`; passed alongside here so a native binding
  /// does not have to dig it back out of a document to honour it.
  pub deadline_ms: u64,
}

/// Native binding (spec §9.1). The result document is design 2.7's [`crate::hooks::InvokeResult`],
/// which is where it is parsed and where its changes are governed — a component that answers with
/// a change it did not declare is refused there, by the same code that refuses a script's.
pub trait HookComponent: Send + Sync {
  fn invoke(&self, req: Invoke) -> Result<Value, ComponentError>;
}
