//! What a run records about its hooks (lifecycle-hooks spec §10, `hook-report.schema.json`): one
//! entry per invocation, in the order they ran, plus the abort reason when a hook ended the run.
//!
//! The document exists to answer one question — *which hook rewrote this header* — and the shape
//! of the answer is the whole design: it carries the paths a hook changed and never the values it
//! wrote. The path is what a reader needs; the value is what an attacker needs, and a summary
//! travels through every CI log.
//!
//! Two absences are also load-bearing. A hook's `data` appears only when its entry set
//! `report-data`, because the fetch-a-token hook is precisely the one whose output must not be in
//! the logs (§7.3). And hooks that never ran are absent while hooks that ran and changed nothing
//! are present with an empty `changed` — only one of those two means a selector is wrong.

use super::invoke::Outcome;
use serde_json::{Map, Value, json};

/// What the outcome did to the run, after `on-failure` was applied (spec §10.2's `effect`). Present
/// when the outcome was not `ok`, so a warning that was *configured* to be a warning is
/// distinguishable from one that was silently treated as such.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
  None,
  Warned,
  FailedExchange,
  StateUnavailable,
  AbortedRun,
}

impl Effect {
  pub fn as_str(self) -> &'static str {
    match self {
      Effect::None => "none",
      Effect::Warned => "warned",
      Effect::FailedExchange => "failed-exchange",
      Effect::StateUnavailable => "state-unavailable",
      Effect::AbortedRun => "aborted-run",
    }
  }
}

/// One invocation, as the report records it and as the `verification/hook` event carries it — the
/// same shape in both places, deliberately: a host rendering hook activity inline with results
/// should not have to reconcile two documents describing the same call.
#[derive(Debug, Clone)]
pub struct Invocation {
  pub point: String,
  pub hook: String,
  pub implementation: String,
  pub outcome: Outcome,
  pub effect: Effect,
  pub exchange: Option<String>,
  pub interaction: Option<String>,
  pub variant: Option<String>,
  pub state: Option<String>,
  pub changed: Vec<String>,
  pub duration_ms: u64,
  /// Only ever `Some` when the entry set `report-data` (spec §7.3).
  pub data: Option<Value>,
  pub error: Option<Value>,
}

impl Invocation {
  pub fn to_json(&self) -> Value {
    let mut record = Map::new();
    record.insert("point".to_string(), json!(self.point));
    record.insert("hook".to_string(), json!(self.hook));
    record.insert("implementation".to_string(), json!(self.implementation));
    record.insert("outcome".to_string(), json!(self.outcome.as_str()));
    if self.outcome != Outcome::Ok {
      record.insert("effect".to_string(), json!(self.effect.as_str()));
    }
    for (key, value) in [
      ("exchange", &self.exchange),
      ("interaction", &self.interaction),
      ("variant", &self.variant),
      ("state", &self.state),
    ] {
      if let Some(value) = value {
        record.insert(key.to_string(), json!(value));
      }
    }
    record.insert("changed".to_string(), json!(self.changed));
    record.insert("duration-ms".to_string(), json!(self.duration_ms));
    if let Some(data) = &self.data {
      record.insert("data".to_string(), data.clone());
    }
    if let Some(error) = &self.error {
      record.insert("error".to_string(), error.clone());
    }
    Value::Object(record)
  }
}

/// Why a run stopped early (spec §10.2's `HookAbort`). A verification that stopped early and a
/// verification that failed are different results, and a host that cannot tell them apart will
/// publish one as the other.
#[derive(Debug, Clone)]
pub struct Abort {
  pub point: String,
  pub hook: String,
  pub error: Option<Value>,
  /// Counted as things not done, never folded into the failed tally.
  pub exchanges_not_run: usize,
}

#[derive(Debug, Clone, Default)]
pub struct HookReport {
  pub invocations: Vec<Invocation>,
  pub aborted: Option<Abort>,
}

impl HookReport {
  pub fn record(&mut self, invocation: Invocation) {
    self.invocations.push(invocation);
  }

  pub fn is_empty(&self) -> bool {
    self.invocations.is_empty() && self.aborted.is_none()
  }

  pub fn to_json(&self) -> Value {
    let mut report = Map::new();
    report.insert(
      "invocations".to_string(),
      Value::Array(self.invocations.iter().map(Invocation::to_json).collect()),
    );
    if let Some(abort) = &self.aborted {
      let mut aborted = Map::new();
      aborted.insert("point".to_string(), json!(abort.point));
      aborted.insert("hook".to_string(), json!(abort.hook));
      if let Some(error) = &abort.error {
        aborted.insert("error".to_string(), error.clone());
      }
      aborted.insert("exchanges-not-run".to_string(), json!(abort.exchanges_not_run));
      report.insert("aborted".to_string(), Value::Object(aborted));
    }
    Value::Object(report)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn invocation() -> Invocation {
    Invocation {
      point: "before-request".to_string(),
      hook: "sign-requests".to_string(),
      implementation: "script".to_string(),
      outcome: Outcome::Ok,
      effect: Effect::None,
      exchange: Some("x-1".to_string()),
      interaction: Some("a request for an order".to_string()),
      variant: Some("base".to_string()),
      state: None,
      changed: vec!["parts.request.headers".to_string()],
      duration_ms: 3,
      data: None,
      error: None,
    }
  }

  #[test]
  fn an_invocation_records_the_path_it_changed_and_never_the_value() {
    let record = invocation().to_json();
    assert_eq!(record["changed"], json!(["parts.request.headers"]));
    let rendered = serde_json::to_string(&record).unwrap();
    assert!(!rendered.contains("Bearer"), "no values ride along: {rendered}");
    assert!(record.get("data").is_none(), "data is opt-in (§7.3)");
    assert!(
      record.get("effect").is_none(),
      "an ok invocation had no effect to report"
    );
  }

  #[test]
  fn a_hook_that_ran_and_changed_nothing_is_present_with_an_empty_changed() {
    let mut nothing = invocation();
    nothing.changed = Vec::new();
    let record = nothing.to_json();
    assert_eq!(
      record["changed"],
      json!([]),
      "absent would mean 'never ran', and only one of those means the selector is wrong"
    );
  }

  #[test]
  fn a_failure_records_what_the_policy_made_of_it() {
    let mut failed = invocation();
    failed.outcome = Outcome::TimedOut;
    failed.effect = Effect::FailedExchange;
    let record = failed.to_json();
    assert_eq!(record["outcome"], json!("timed-out"));
    assert_eq!(record["effect"], json!("failed-exchange"));
  }

  #[test]
  fn an_abort_counts_what_never_ran_instead_of_calling_it_failed() {
    let report = HookReport {
      invocations: vec![invocation()],
      aborted: Some(Abort {
        point: "before-verification".to_string(),
        hook: "auth-token".to_string(),
        error: Some(json!({ "code": "hook-errored", "message": "no token" })),
        exchanges_not_run: 6,
      }),
    };
    let json = report.to_json();
    assert_eq!(json["aborted"]["exchanges-not-run"], json!(6));
    assert_eq!(json["aborted"]["hook"], json!("auth-token"));
    assert_eq!(json["invocations"].as_array().map(Vec::len), Some(1));
  }
}
