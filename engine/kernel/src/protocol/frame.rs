//! The protocol envelope (engine-protocol spec §4): frames, and the structured error value
//! carried in a `ResponseFrame` (spec §10). Project-owned, hand-written types (spec §2.3) — not
//! generated, since there is no typify pipeline in this repo yet, and the envelope itself is
//! normatively project-owned regardless: a generated Rust type drops unknown members, which would
//! make the engine a lossy intermediary (spike 1.1, finding 14).

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// An incoming `RequestFrame` (spec §4.1). `extra` preserves any member this kernel does not
/// know about, per the open-world rule that unknown members are ignored and preserved where
/// practical (spec §2.2 rule 2).
#[derive(Debug, Clone, Deserialize)]
pub struct RequestFrame {
  pub id: String,
  pub op: String,
  pub body: Value,
  #[serde(flatten)]
  pub extra: Map<String, Value>,
}

/// An outgoing `ResponseFrame` (spec §4.2): exactly one of `ok`/`error`.
#[derive(Debug, Clone, Serialize)]
pub struct ResponseFrame {
  #[serde(rename = "type")]
  pub frame_type: &'static str,
  pub id: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub ok: Option<Value>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub error: Option<EngineError>,
}

impl ResponseFrame {
  pub fn ok(id: impl Into<String>, ok: Value) -> Self {
    ResponseFrame {
      frame_type: "response",
      id: id.into(),
      ok: Some(ok),
      error: None,
    }
  }

  pub fn err(id: impl Into<String>, error: EngineError) -> Self {
    ResponseFrame {
      frame_type: "response",
      id: id.into(),
      ok: None,
      error: Some(error),
    }
  }
}

/// The structured error value carried in a `ResponseFrame::error` (spec §10.1). Every failure
/// crosses the pipe as one of these — never a panic, trap, exception or broken pipe.
#[derive(Debug, Clone, Serialize)]
pub struct EngineError {
  pub code: String,
  pub category: String,
  pub message: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub details: Option<Value>,
}

impl EngineError {
  /// `handshake-required` (spec §5.1): any operation before a successful `engine/hello`.
  pub fn handshake_required() -> Self {
    EngineError {
      code: "handshake-required".to_string(),
      category: "protocol".to_string(),
      message: "the first request on a pipe must be engine/hello".to_string(),
      details: None,
    }
  }

  /// `operation-unsupported` (spec §4.1, §10.2): an unknown or unimplemented `op`, named.
  pub fn operation_unsupported(op: &str) -> Self {
    EngineError {
      code: "operation-unsupported".to_string(),
      category: "protocol".to_string(),
      message: format!("unknown operation '{op}'"),
      details: Some(serde_json::json!({ "op": op })),
    }
  }

  /// `protocol-version-unsupported` (spec §5.1): the host's offered versions, none supported.
  pub fn protocol_version_unsupported(supported: &[u32]) -> Self {
    EngineError {
      code: "protocol-version-unsupported".to_string(),
      category: "protocol".to_string(),
      message: format!("this engine speaks protocol versions {supported:?}"),
      details: Some(serde_json::json!({ "supported": supported })),
    }
  }

  /// `malformed-frame` (spec §4.4): the frame envelope or an operation's request body did not
  /// match its schema. `message` should carry enough to locate the problem.
  pub fn malformed_frame(message: impl Into<String>) -> Self {
    EngineError {
      code: "malformed-frame".to_string(),
      category: "protocol".to_string(),
      message: message.into(),
      details: None,
    }
  }

  /// `session-not-found` (spec §7.1): a stale, unknown or already-ended session id.
  pub fn session_not_found(session: &str) -> Self {
    EngineError {
      code: "session-not-found".to_string(),
      category: "session".to_string(),
      message: format!("session '{session}' does not exist or has ended"),
      details: Some(serde_json::json!({ "session": session })),
    }
  }

  /// `interaction-invalid` (spec §8.2, §10.2): the submitted interaction specification is not
  /// well-formed. `problems` carries RFC 6901 pointers (contract-file spec §11's convention).
  pub fn interaction_invalid(problems: &[crate::error::Problem]) -> Self {
    EngineError {
      code: "interaction-invalid".to_string(),
      category: "document".to_string(),
      message: "interaction specification is not valid".to_string(),
      details: Some(serde_json::json!({ "problems": problems })),
    }
  }

  /// `handle-not-found` (spec §10.2): a stale, unknown, or already-ended interaction handle.
  pub fn handle_not_found(handle: &str) -> Self {
    EngineError {
      code: "handle-not-found".to_string(),
      category: "session".to_string(),
      message: format!("interaction handle '{handle}' does not exist in this session"),
      details: Some(serde_json::json!({ "handle": handle })),
    }
  }

  /// `variant-not-found` (variant-semantics spec §8): `serve-variant` named an id that is not in
  /// this interaction's current selection.
  pub fn variant_not_found(variant: &str, selection: &[String]) -> Self {
    EngineError {
      code: "variant-not-found".to_string(),
      category: "session".to_string(),
      message: format!("'{variant}' is not in this interaction's selection"),
      details: Some(serde_json::json!({ "variant": variant, "selection": selection })),
    }
  }

  /// `variant-budget-exceeded` (variant-semantics spec §3.6, §8): the selection would exceed
  /// `max-variants`. Naming the space size, the selection size it would have been, the budget,
  /// and the dimensions contributing the most points is what turns this into a decision the
  /// author can act on rather than a bare rejection.
  pub fn variant_budget_exceeded(space: u64, selected: usize, budget: u64, dimensions: &[String]) -> Self {
    EngineError {
      code: "variant-budget-exceeded".to_string(),
      category: "document".to_string(),
      message: format!("the selection would need {selected} variants, above the budget of {budget}"),
      details: Some(serde_json::json!({
        "space": space, "selected": selected, "budget": budget, "dimensions": dimensions,
      })),
    }
  }

  /// `interaction-invalid` (spec §8.2, §10.2), for a malformed sampling policy or an
  /// unresolvable/ambiguous dimension reference within it (variant-semantics spec §8) — the same
  /// code `add-interaction` uses, since both report the same shape of problem.
  pub fn invalid_policy(problems: &[crate::error::Problem]) -> Self {
    EngineError {
      code: "interaction-invalid".to_string(),
      category: "document".to_string(),
      message: "sampling policy is not valid".to_string(),
      details: Some(serde_json::json!({ "problems": problems })),
    }
  }

  /// `internal` (spec §10.1): the dispatch boundary's own panic-catch. A panic reaching this
  /// constructor is itself a bug — it exists so a panic never crosses the pipe.
  pub fn internal(message: impl Into<String>) -> Self {
    EngineError {
      code: "internal".to_string(),
      category: "internal".to_string(),
      message: message.into(),
      details: None,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn ok_response_carries_no_error_member() {
    let response = ResponseFrame::ok("r-1", serde_json::json!({ "session": "cs-1" }));
    let value = serde_json::to_value(&response).unwrap();
    assert_eq!(value["type"], "response");
    assert_eq!(value["id"], "r-1");
    assert_eq!(value["ok"]["session"], "cs-1");
    assert!(value.get("error").is_none());
  }

  #[test]
  fn err_response_carries_no_ok_member() {
    let response = ResponseFrame::err("r-1", EngineError::handshake_required());
    let value = serde_json::to_value(&response).unwrap();
    assert_eq!(value["error"]["code"], "handshake-required");
    assert_eq!(value["error"]["category"], "protocol");
    assert!(value.get("ok").is_none());
  }

  #[test]
  fn interaction_invalid_serializes_pointer_and_message() {
    let problems = vec![crate::error::Problem {
      pointer: "/response/body/shape/id".to_string(),
      message: "unknown shape operator 'tpye'".to_string(),
    }];
    let error = EngineError::interaction_invalid(&problems);
    let value = serde_json::to_value(&error).unwrap();
    assert_eq!(
      value["details"]["problems"][0]["pointer"],
      "/response/body/shape/id"
    );
    assert_eq!(
      value["details"]["problems"][0]["message"],
      "unknown shape operator 'tpye'"
    );
  }
}
