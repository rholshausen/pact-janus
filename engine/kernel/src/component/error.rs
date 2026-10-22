//! `ComponentError` (component-interfaces spec §11): the structured error a component returns.
//! "Deliberately the same shape as the protocol's `EngineError`, because the kernel passes it
//! through verbatim... and a second error shape at that boundary would only need translating."

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentError {
  pub code: String,
  /// Absent or unknown is treated as `internal` (spec §11.1), which is also what a component that
  /// sent none gets here.
  #[serde(default = "internal_category")]
  pub category: String,
  pub message: String,
  /// Who produced it (spec §11.2): absent means the component did; `engine` marks an error the
  /// binding synthesised because the component trapped, timed out or exited — "the component said
  /// no" and "the component died" send a reader to different places.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub source: Option<Box<str>>,
  /// Boxed: an object `Value` keeps its members in order (ADR 0037), which makes it large, and an
  /// error is carried by every `Result` that can fail.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub details: Option<Box<Value>>,
}

fn internal_category() -> String {
  "internal".to_string()
}

impl ComponentError {
  /// `operation-unsupported` (spec §11.1): an op or a role/kind this component declared it does
  /// not implement (e.g. the HTTP transport's `drive` role, or `transport/send`, in this task).
  pub fn operation_unsupported(op: &str) -> Self {
    ComponentError {
      code: "operation-unsupported".to_string(),
      category: "protocol".to_string(),
      message: format!("unsupported operation '{op}'"),
      source: None,
      details: Some(Box::new(serde_json::json!({ "op": op }))),
    }
  }

  /// `decode-failed` (spec §11.1): the content component could not turn octets into a document.
  pub fn decode_failed(content_type: &str, message: impl Into<String>) -> Self {
    ComponentError {
      code: "decode-failed".to_string(),
      category: "component".to_string(),
      message: message.into(),
      source: None,
      details: Some(Box::new(serde_json::json!({ "content-type": content_type }))),
    }
  }

  /// `encode-failed` (spec §11.1): the content component could not turn a document into octets of
  /// this type, such as a document that is not a string for a text type.
  pub fn encode_failed(content_type: &str, message: impl Into<String>) -> Self {
    ComponentError {
      code: "encode-failed".to_string(),
      category: "document".to_string(),
      message: message.into(),
      source: None,
      details: Some(Box::new(serde_json::json!({ "content-type": content_type }))),
    }
  }

  /// `unsupported-content-type` (spec §11.1): asked to decode/encode a type this component does
  /// not handle.
  pub fn unsupported_content_type(content_type: &str) -> Self {
    ComponentError {
      code: "unsupported-content-type".to_string(),
      category: "component".to_string(),
      message: format!("unsupported content type '{content_type}'"),
      source: None,
      details: Some(Box::new(serde_json::json!({ "content-type": content_type }))),
    }
  }

  /// `transport-failed` (spec §11.1): the machinery failed — a bind error, a broken connection —
  /// as distinct from a mismatch, which is not an error at all (spec §11's own point).
  pub fn transport_failed(message: impl Into<String>) -> Self {
    ComponentError {
      code: "transport-failed".to_string(),
      category: "component".to_string(),
      message: message.into(),
      source: None,
      details: None,
    }
  }

  /// `unknown-generator` (spec §11.1): a generator this component does not provide.
  pub fn unknown_generator(name: &str) -> Self {
    ComponentError {
      code: "unknown-generator".to_string(),
      category: "document".to_string(),
      message: format!("no generator '{name}'"),
      source: None,
      details: Some(Box::new(serde_json::json!({ "generator": name }))),
    }
  }

  /// `invalid-config` (spec §11.1): the configuration this call carried is not one the component
  /// can act on.
  pub fn invalid_config(name: &str, message: impl Into<String>) -> Self {
    ComponentError {
      code: "invalid-config".to_string(),
      category: "document".to_string(),
      message: message.into(),
      source: None,
      details: Some(Box::new(serde_json::json!({ "name": name }))),
    }
  }

  /// `generate-failed` (spec §11.1): the generator could not produce a value from what it was given.
  pub fn generate_failed(generator: &str, message: impl Into<String>) -> Self {
    ComponentError {
      code: "generate-failed".to_string(),
      category: "document".to_string(),
      message: message.into(),
      source: None,
      details: Some(Box::new(serde_json::json!({ "generator": generator }))),
    }
  }

  /// `internal` (spec §11.1): the component's own bug.
  pub fn internal(message: impl Into<String>) -> Self {
    ComponentError {
      code: "internal".to_string(),
      category: "internal".to_string(),
      message: message.into(),
      source: None,
      details: None,
    }
  }

  /// An error the binding synthesised on a component's behalf (spec §11.2): `code` is one of
  /// `component-trapped`, `component-timeout`, `component-exited`, and `source` is `engine`.
  pub fn synthesised(code: &str, message: impl Into<String>) -> Self {
    ComponentError {
      code: code.to_string(),
      category: "component".to_string(),
      message: message.into(),
      source: Some("engine".into()),
      details: None,
    }
  }
}
