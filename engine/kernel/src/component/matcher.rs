//! The matcher interface's execution half (component-interfaces spec §7.1, plan task 8.4):
//! `matcher/apply`, which runs one of a component's own namespaced actions when a plan reaches it, and
//! `matcher/compare`, which the subsumption walk asks about an operator the component declared
//! comparable (spec §7.5, ADR 0087), and `matcher/prefix`, which narrows the entries a check tries
//! (spec §7.6, ADR 0096). `compile`, `variant-space` and `generator/generate` have no caller
//! yet: a contributed operator compiles to the action of the same name, which `apply` runs.

use super::ComponentError;
use serde_json::Value;
use std::sync::Arc;

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

  /// The comparability class this component declares for one of its operators (spec §7.5): the
  /// in-tree form of an `operators` entry's `comparability`. Opaque unless a component says so.
  fn comparability(&self, operator: &str) -> Comparability {
    let _ = operator;
    Comparability::Opaque
  }

  /// `matcher/compare` (spec §7.5): whether `admits(provider) ⊆ admits(consumer)`, one of the two
  /// being this component's operator. A component answers `unknown` rather than guess.
  fn compare(&self, req: Compare) -> Result<CompareResult, ComponentError> {
    let _ = req;
    Err(ComponentError::operation_unsupported("matcher/compare"))
  }

  /// `matcher/prefix` (spec §7.6, ADR 0096): text every value the operator admits begins with, read
  /// from one of its shape documents, so a check can rule an entry out without running it. `None`
  /// when there is none; an operator that admits non-strings has none.
  fn prefix(&self, req: Prefix) -> Result<PrefixResult, ComponentError> {
    let _ = req;
    Err(ComponentError::operation_unsupported("matcher/prefix"))
  }
}

/// `matcher/prefix`'s request (`schemas/v1/matcher.schema.json`): the operator and its shape
/// document.
#[derive(Debug, Clone)]
pub struct Prefix {
  pub operator: String,
  pub node: Value,
}

/// `matcher/prefix`'s result: the text every admitted value begins with, if there is any.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrefixResult {
  pub prefix: Option<String>,
}

/// An operator's comparability class (shape spec §8, component-interfaces spec §7.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Comparability {
  /// Decided `yes` or `no` by `compare`, which the component implements.
  Exact,
  /// `yes` on identity and exactly-wider containers, a finite provider decided by the operator's own
  /// matcher, `unknown` otherwise; `compare` optional.
  Conservative,
  /// `yes` on identity, `unknown` otherwise.
  Opaque,
}

/// `matcher/compare`'s request (`schemas/v1/matcher.schema.json`): the two shape documents.
#[derive(Debug, Clone)]
pub struct Compare {
  pub operator: String,
  pub provider: Value,
  pub consumer: Value,
}

/// `matcher/compare`'s result: `yes`, `no` or `unknown`, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompareResult {
  pub verdict: String,
  pub reason: Option<String>,
}

/// A matcher compiled into the embedding (ADR 0057): the built-in HTTP component's
/// `http:status-class`. Its actions are run as a declared component's are, through `matcher/apply`'s
/// native binding, and its name is declared in-tree, so a requirement naming it is satisfied.
#[derive(Clone)]
pub struct InTreeMatcher {
  pub name: String,
  /// The namespaced actions it runs, each `<name>:<action>` (component-interfaces spec §2.4).
  pub actions: Vec<String>,
  pub matcher: Arc<dyn MatcherComponent>,
}
