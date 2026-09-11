//! `ComponentError` (component-interfaces spec §11): the structured error a component returns.
//! "Deliberately the same shape as the protocol's `EngineError`, because the kernel passes it
//! through verbatim... and a second error shape at that boundary would only need translating."

use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Serialize)]
pub struct ComponentError {
  pub code: String,
  pub category: String,
  pub message: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub details: Option<Value>,
}

impl ComponentError {
  /// `operation-unsupported` (spec §11.1): an op or a role/kind this component declared it does
  /// not implement (e.g. the HTTP transport's `drive` role, or `transport/send`, in this task).
  pub fn operation_unsupported(op: &str) -> Self {
    ComponentError {
      code: "operation-unsupported".to_string(),
      category: "protocol".to_string(),
      message: format!("unsupported operation '{op}'"),
      details: Some(serde_json::json!({ "op": op })),
    }
  }

  /// `decode-failed` (spec §11.1): the content component could not turn octets into a document.
  pub fn decode_failed(content_type: &str, message: impl Into<String>) -> Self {
    ComponentError {
      code: "decode-failed".to_string(),
      category: "component".to_string(),
      message: message.into(),
      details: Some(serde_json::json!({ "content-type": content_type })),
    }
  }

  /// `unsupported-content-type` (spec §11.1): asked to decode/encode a type this component does
  /// not handle.
  pub fn unsupported_content_type(content_type: &str) -> Self {
    ComponentError {
      code: "unsupported-content-type".to_string(),
      category: "component".to_string(),
      message: format!("unsupported content type '{content_type}'"),
      details: Some(serde_json::json!({ "content-type": content_type })),
    }
  }

  /// `transport-failed` (spec §11.1): the machinery failed — a bind error, a broken connection —
  /// as distinct from a mismatch, which is not an error at all (spec §11's own point).
  pub fn transport_failed(message: impl Into<String>) -> Self {
    ComponentError {
      code: "transport-failed".to_string(),
      category: "component".to_string(),
      message: message.into(),
      details: None,
    }
  }

  /// `internal` (spec §11.1): the component's own bug.
  pub fn internal(message: impl Into<String>) -> Self {
    ComponentError {
      code: "internal".to_string(),
      category: "internal".to_string(),
      message: message.into(),
      details: None,
    }
  }
}
