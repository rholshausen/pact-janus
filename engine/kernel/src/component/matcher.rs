//! The matcher interface's execution half (component-interfaces spec §7.1, plan task 8.4):
//! `matcher/apply`, which runs one of a component's own namespaced actions when a plan reaches it.
//! `compile`, `variant-space`, `compare` and `generator/generate` have no caller yet — no shape
//! operator is contributed by a component anywhere in this engine — so they are not here.

use super::ComponentError;
use serde_json::Value;

/// `matcher/apply`'s request (`schemas/v1/matcher.schema.json`). One entry in `values` per
/// application: this engine never batches, so there is always exactly one.
#[derive(Debug, Clone)]
pub struct Apply {
  pub action: String,
  pub config: Option<Value>,
  pub values: Vec<Value>,
}

/// One plan result document (plan-grammar spec §2.3's result form) per value, in order.
#[derive(Debug, Clone)]
pub struct ApplyResult {
  pub results: Vec<Value>,
}

/// Native binding (spec §9.1) — `Send + Sync` for the same reason the other interfaces are: the
/// passive exchange loop executes plans on its own thread.
pub trait MatcherComponent: Send + Sync {
  fn apply(&self, req: Apply) -> Result<ApplyResult, ComponentError>;
}
