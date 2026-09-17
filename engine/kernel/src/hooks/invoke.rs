//! The two documents every invocation is made of (lifecycle-hooks spec §2.3) and the boundary the
//! implementations sit behind: a `HookContext` out, an `InvokeResult` back.
//!
//! That sameness is the property that makes a hook portable — a signing hook prototyped as a
//! script and later moved into a component is the same hook handling the same document — so the
//! conversion lives here once and every implementation shares it.
//!
//! Changes get the treatment spec §4.3 specifies and nothing looser: **declared, permitted,
//! applied, recorded**, and nothing partially applied. A result whose changes include one refused
//! key has *none* of its changes applied, because a hook that half-ran is a state no author
//! tested.

use super::config::HookEntry;
use super::points::Point;
use crate::component::{Parts, SlotValue};
use serde_json::{Map, Value};

/// What a hook answered (spec §5.1), or what the engine concluded on its behalf when it did not
/// answer at all. The last two are the engine's conclusions and are kept distinct because they
/// send a reader to different places: a deadline that passed is a slow hook, a process that died
/// is a broken one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
  Ok,
  Skipped,
  /// State points only: the provider cannot reach the state (variant-semantics spec §6.7).
  Unsupported,
  Failed,
  TimedOut,
  Errored,
}

impl Outcome {
  pub fn parse(text: &str) -> Option<Outcome> {
    match text {
      "ok" => Some(Outcome::Ok),
      "skipped" => Some(Outcome::Skipped),
      "unsupported" => Some(Outcome::Unsupported),
      "failed" => Some(Outcome::Failed),
      "timed-out" => Some(Outcome::TimedOut),
      "errored" => Some(Outcome::Errored),
      _ => None,
    }
  }

  pub fn as_str(self) -> &'static str {
    match self {
      Outcome::Ok => "ok",
      Outcome::Skipped => "skipped",
      Outcome::Unsupported => "unsupported",
      Outcome::Failed => "failed",
      Outcome::TimedOut => "timed-out",
      Outcome::Errored => "errored",
    }
  }

  /// Whether the point's `on-failure` policy applies to this outcome.
  pub fn is_failure(self) -> bool {
    matches!(self, Outcome::Failed | Outcome::TimedOut | Outcome::Errored)
  }
}

/// What a hook answered (design 2.6's `InvokeResult`, carried unchanged).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct InvokeResult {
  pub outcome: Option<Outcome>,
  pub changes: Map<String, Value>,
  pub data: Option<Value>,
  pub error: Option<Value>,
}

impl InvokeResult {
  /// An empty answer is `ok` with no changes — the common case for an observing hook, and what a
  /// command that succeeded and had nothing to say means (spec §8.3, §9.2).
  pub fn ok() -> InvokeResult {
    InvokeResult {
      outcome: Some(Outcome::Ok),
      ..InvokeResult::default()
    }
  }

  pub fn outcome(&self) -> Outcome {
    self.outcome.unwrap_or(Outcome::Ok)
  }

  /// Read a result document. An unknown `outcome` value is *not* silently treated as failure or as
  /// success: it is `errored` naming what arrived, because guessing at an open discriminator is
  /// exactly what protocol §2.2 rule 3 forbids.
  pub fn parse(document: &Value) -> Result<InvokeResult, String> {
    if document.is_null() {
      return Ok(InvokeResult::ok());
    }
    let Some(object) = document.as_object() else {
      return Err("a hook result must be an object".to_string());
    };
    let outcome = match object.get("outcome") {
      None => Some(Outcome::Ok),
      Some(Value::String(text)) => match Outcome::parse(text) {
        Some(outcome) => Some(outcome),
        None => return Err(format!("'{text}' is not a hook outcome")),
      },
      Some(other) => return Err(format!("'outcome' must be a string, got {other}")),
    };
    let changes = match object.get("changes") {
      None | Some(Value::Null) => Map::new(),
      Some(Value::Object(changes)) => changes.clone(),
      Some(_) => return Err("'changes' must be an object of context path -> value".to_string()),
    };
    Ok(InvokeResult {
      outcome,
      changes,
      data: object.get("data").cloned(),
      error: object.get("error").cloned(),
    })
  }
}

/// What the engine concludes when a hook does not answer at all (spec §5.1).
#[derive(Debug, Clone)]
pub struct HookFailure {
  pub outcome: Outcome,
  /// The implementation's own error document, passed through verbatim — the kernel does not
  /// translate error interiors it does not own (protocol §10.2).
  pub error: Value,
}

impl HookFailure {
  pub fn errored(message: impl Into<String>) -> HookFailure {
    HookFailure {
      outcome: Outcome::Errored,
      error: serde_json::json!({ "code": "hook-errored", "message": message.into() }),
    }
  }

  pub fn timed_out(deadline_ms: u64) -> HookFailure {
    HookFailure {
      outcome: Outcome::TimedOut,
      error: serde_json::json!({
        "code": "hook-timed-out",
        "message": format!("the hook did not answer within {deadline_ms}ms"),
      }),
    }
  }
}

/// One implementation kind, provided by whatever can actually run it (spec §8.5).
///
/// The engine holds no callbacks into a host (ADR 0014) and this is not one: an invoker is a
/// *capability the embedding compiled in*, registered once, the same way a transport component is.
/// The distinction matters — a callback would need the protocol to grow an engine-to-host call,
/// and it never does.
pub trait HookInvoker: Send + Sync {
  /// Perform one invocation. `run` is the entry's `run` document, `context` the `HookContext`.
  /// Returning `Err` is the engine concluding on the hook's behalf; the implementation MUST bound
  /// itself by `deadline_ms` and report expiry rather than hanging (spec §5.4).
  fn invoke(&self, run: &Value, context: &Value, deadline_ms: u64) -> Result<InvokeResult, HookFailure>;
}

/// Applying a result's changes to the parts in flight (spec §4.3). `Err` names the first refused
/// path; on `Err` the caller applies nothing, which is the "nothing is partially applied" rule.
pub fn apply_changes(
  result: &InvokeResult,
  entry: &HookEntry,
  point: &Point,
  parts: &mut Parts,
) -> Result<Vec<String>, Value> {
  // Every key is checked before any is applied.
  for path in result.changes.keys() {
    if !point.permits(path) {
      return Err(refused(
        path,
        format!("'{}' does not permit changing it", point.name),
      ));
    }
    if !entry.changes.iter().any(|declared| declared == path) {
      return Err(refused(
        path,
        format!("hook '{}' did not declare it in its 'changes'", entry.name),
      ));
    }
  }

  let mut changed = Vec::new();
  for (path, value) in &result.changes {
    let segments: Vec<&str> = path.split('.').collect();
    match segments.as_slice() {
      ["parts"] => {
        let Some(replacement) = parse_parts(value) else {
          return Err(refused(path, "the replacement is not a parts document"));
        };
        *parts = replacement;
      }
      ["parts", part, slot] => {
        let Some(replacement) = parse_slot(value) else {
          return Err(refused(path, "the replacement is not a slot value"));
        };
        parts
          .entry((*part).to_string())
          .or_default()
          .insert((*slot).to_string(), replacement);
      }
      _ => return Err(refused(path, "it is not a context path this engine can replace")),
    }
    changed.push(path.clone());
  }
  Ok(changed)
}

fn refused(path: &str, why: impl Into<String>) -> Value {
  serde_json::json!({
    "code": "hook-change-refused",
    "message": format!("the change to '{path}' was refused: {}", why.into()),
    "details": { "path": path },
  })
}

fn parse_slot(value: &Value) -> Option<SlotValue> {
  serde_json::from_value(value.clone()).ok()
}

fn parse_parts(value: &Value) -> Option<Parts> {
  serde_json::from_value(value.clone()).ok()
}

/// The parts document as a hook sees it (design 2.6's `Parts`, unchanged).
pub fn parts_json(parts: &Parts) -> Value {
  serde_json::to_value(parts).expect("Parts always serializes")
}

#[cfg(test)]
mod tests {
  use super::super::config::{HookEntry, RunSpec};
  use super::super::points::point;
  use super::*;
  use serde_json::json;

  fn entry(changes: &[&str]) -> HookEntry {
    HookEntry {
      name: "sign".to_string(),
      run: RunSpec::from_json(json!({ "kind": "script", "source": "" })),
      config: None,
      when: None,
      changes: changes.iter().map(|c| c.to_string()).collect(),
      timeout_ms: None,
      on_failure: None,
      report_data: false,
    }
  }

  fn parts_with_headers() -> Parts {
    let mut parts = Parts::new();
    let mut request = std::collections::BTreeMap::new();
    request.insert(
      "path".to_string(),
      SlotValue {
        content: json!("/orders/66"),
        encoded: None,
        content_type: None,
      },
    );
    parts.insert("request".to_string(), request);
    parts
  }

  #[test]
  fn an_empty_answer_is_ok_with_no_changes() {
    assert_eq!(InvokeResult::parse(&Value::Null).unwrap(), InvokeResult::ok());
    assert_eq!(InvokeResult::parse(&json!({})).unwrap(), InvokeResult::ok());
  }

  #[test]
  fn an_unknown_outcome_is_refused_rather_than_guessed_at() {
    let err = InvokeResult::parse(&json!({ "outcome": "probably-fine" })).expect_err("open != anything");
    assert!(err.contains("probably-fine"));
  }

  #[test]
  fn a_declared_and_permitted_change_is_applied_and_recorded_by_path() {
    let result = InvokeResult::parse(&json!({
      "outcome": "ok",
      "changes": { "parts.request.headers": { "content": { "authorization": ["Bearer t"] } } }
    }))
    .unwrap();
    let mut parts = parts_with_headers();
    let changed = apply_changes(
      &result,
      &entry(&["parts.request.headers"]),
      point("before-request").unwrap(),
      &mut parts,
    )
    .expect("declared and permitted");
    assert_eq!(changed, vec!["parts.request.headers".to_string()]);
    assert_eq!(
      parts["request"]["headers"].content,
      json!({ "authorization": ["Bearer t"] })
    );
    assert_eq!(parts["request"]["path"].content, json!("/orders/66"), "untouched");
  }

  #[test]
  fn an_undeclared_change_is_refused_even_where_the_point_permits_it() {
    let result = InvokeResult::parse(&json!({
      "changes": { "parts.request.path": { "content": "/somewhere-else" } }
    }))
    .unwrap();
    let mut parts = parts_with_headers();
    let refused = apply_changes(
      &result,
      &entry(&["parts.request.headers"]),
      point("before-request").unwrap(),
      &mut parts,
    )
    .expect_err("a hook that adds a header cannot quietly rewrite a path");
    assert_eq!(refused["code"], json!("hook-change-refused"));
    assert_eq!(refused["details"]["path"], json!("parts.request.path"));
    assert_eq!(
      parts["request"]["path"].content,
      json!("/orders/66"),
      "nothing applied"
    );
  }

  #[test]
  fn a_change_at_a_point_that_permits_none_is_refused() {
    let result = InvokeResult::parse(&json!({
      "changes": { "parts.response.headers": { "content": {} } }
    }))
    .unwrap();
    let mut parts = parts_with_headers();
    apply_changes(
      &result,
      &entry(&["parts.response.headers"]),
      point("after-response").unwrap(),
      &mut parts,
    )
    .expect_err("rewriting the response before matching would be editing the evidence");
  }

  #[test]
  fn one_refused_key_means_none_of_the_changes_are_applied() {
    let result = InvokeResult::parse(&json!({
      "changes": {
        "parts.request.headers": { "content": { "authorization": ["Bearer t"] } },
        "parts.request.path": { "content": "/elsewhere" }
      }
    }))
    .unwrap();
    let mut parts = parts_with_headers();
    apply_changes(
      &result,
      &entry(&["parts.request.headers"]),
      point("before-request").unwrap(),
      &mut parts,
    )
    .expect_err("one refused key refuses the result");
    assert!(
      !parts["request"].contains_key("headers"),
      "a hook that half-ran is a state no author tested: {parts:?}"
    );
  }

  #[test]
  fn produce_message_replaces_the_whole_parts_document() {
    let result = InvokeResult::parse(&json!({
      "changes": { "parts": { "message": { "body": { "content": { "id": "o-1" } } } } }
    }))
    .unwrap();
    let mut parts = parts_with_headers();
    apply_changes(
      &result,
      &entry(&["parts"]),
      point("produce-message").unwrap(),
      &mut parts,
    )
    .expect("produce-message owns parts");
    assert!(parts.contains_key("message"));
    assert!(!parts.contains_key("request"), "replaced, not merged");
  }

  #[test]
  fn outcomes_that_take_the_failure_policy_are_exactly_the_three() {
    assert!(Outcome::Failed.is_failure());
    assert!(Outcome::TimedOut.is_failure());
    assert!(Outcome::Errored.is_failure());
    assert!(!Outcome::Ok.is_failure());
    assert!(!Outcome::Skipped.is_failure());
    assert!(
      !Outcome::Unsupported.is_failure(),
      "state-unavailable is its own status, reached without any on-failure path (spec §5.5)"
    );
  }
}
