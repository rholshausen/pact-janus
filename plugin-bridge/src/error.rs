//! A `ComponentError` (component-interfaces spec §11.1), the bridge's only way of saying no. The bridge
//! never panics, exits or hangs by design: a plugin that fails is a value, in the vocabulary spec §11.1
//! gives a component.

use serde_json::{Map, Value, json};

#[derive(Debug, Clone, PartialEq)]
pub struct ComponentError {
  pub code: &'static str,
  pub category: &'static str,
  pub message: String,
  /// Kept as pairs, not a map, so a `Result` carrying one stays small.
  pub details: Vec<(String, Value)>,
  /// Its causes, outermost first (ADR 0054). The message stays the whole sentence.
  pub chain: Vec<String>,
}

impl ComponentError {
  fn new(code: &'static str, category: &'static str, message: impl Into<String>) -> ComponentError {
    ComponentError {
      code,
      category,
      message: message.into(),
      details: Vec::new(),
      chain: Vec::new(),
    }
  }

  pub fn protocol(code: &'static str, message: impl Into<String>) -> ComponentError {
    ComponentError::new(code, "protocol", message)
  }

  pub fn document(code: &'static str, message: impl Into<String>) -> ComponentError {
    ComponentError::new(code, "document", message)
  }

  /// `unavailable`: what the component needs to answer is not there — here, the plugin.
  pub fn unavailable(message: impl Into<String>) -> ComponentError {
    ComponentError::new("unavailable", "component", message)
  }

  pub fn invalid_config(message: impl Into<String>) -> ComponentError {
    ComponentError::document("invalid-config", message)
  }

  pub fn with(mut self, key: &str, value: impl Into<Value>) -> ComponentError {
    self.details.push((key.to_string(), value.into()));
    self
  }

  /// Gives this error the causes of `err`, as `anyhow` chains them: the chain's first layer is this
  /// error in its own words, and the message becomes the whole sentence (ADR 0054).
  pub fn caused_by(mut self, err: &anyhow::Error) -> ComponentError {
    self.chain = std::iter::once(self.message.clone())
      .chain(err.chain().map(|cause| cause.to_string()))
      .collect();
    self.message = format!("{}: {err:#}", self.message);
    self
  }

  pub fn to_json(&self) -> Value {
    let mut error = json!({ "code": self.code, "category": self.category, "message": self.message });
    if !self.details.is_empty() {
      error["details"] = Value::Object(self.details.iter().cloned().collect::<Map<_, _>>());
    }
    if !self.chain.is_empty() {
      error["chain"] = self
        .chain
        .iter()
        .enumerate()
        .map(|(index, layer)| {
          if index == 0 {
            json!({ "code": self.code, "message": layer })
          } else {
            json!({ "message": layer })
          }
        })
        .collect();
    }
    error
  }
}
