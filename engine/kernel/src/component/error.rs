//! `ComponentError` (component-interfaces spec §11): the structured error a component returns.
//! "Deliberately the same shape as the protocol's `EngineError`, because the kernel passes it
//! through verbatim... and a second error shape at that boundary would only need translating."

use crate::chain::{Layer, joined};
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
  /// Its layers, outermost first, when it has causes (spec §11.1, ADR 0054): see [`crate::chain`].
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub chain: Option<Vec<Layer>>,
}

fn internal_category() -> String {
  "internal".to_string()
}

impl ComponentError {
  /// This error, caused by `causes`: its chain is its own layer and then theirs, and its message
  /// says them all (ADR 0054). No causes leaves it as it is.
  pub fn caused_by(mut self, causes: Vec<Layer>) -> Self {
    if causes.is_empty() {
      return self;
    }
    let mut chain = self.layers();
    chain.extend(causes);
    self.message = joined(&chain);
    self.chain = Some(chain);
    self
  }

  /// This error inside an outer layer: the layer, then its chain, with the message saying them all.
  /// Its code and category are left to the caller.
  pub fn within(mut self, outer: Layer) -> Self {
    let mut chain = vec![outer];
    chain.extend(self.layers());
    self.message = joined(&chain);
    self.chain = Some(chain);
    self
  }

  /// Its chain, or, when it has none, itself as the one layer.
  pub fn layers(&self) -> Vec<Layer> {
    self
      .chain
      .clone()
      .unwrap_or_else(|| vec![Layer::coded(&self.code, &self.message)])
  }

  /// `operation-unsupported` (spec §11.1): an op or a role/kind this component declared it does
  /// not implement (e.g. the HTTP transport's `drive` role, or `transport/send`, in this task).
  pub fn operation_unsupported(op: &str) -> Self {
    ComponentError {
      code: "operation-unsupported".to_string(),
      category: "protocol".to_string(),
      message: format!("unsupported operation '{op}'"),
      source: None,
      details: Some(Box::new(serde_json::json!({ "op": op }))),
      chain: None,
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
      chain: None,
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
      chain: None,
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
      chain: None,
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
      chain: None,
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
      chain: None,
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
      chain: None,
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
      chain: None,
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
      chain: None,
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
      chain: None,
    }
  }
}
