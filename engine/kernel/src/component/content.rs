//! The content interface's request/result documents and native-binding trait
//! (component-interfaces spec §6, `schemas/v1/content.schema.json`): octets to document and back.
//! "This is the entire content of the rule that the kernel knows nothing about JSON — a shape
//! applies to what a content component decoded, never to a guess."
//!
//! `decode`/`encode`'s document is `plan::RuntimeValue` directly: the wire form tags it
//! (`x-tagged-by: "encoded"`) because JSON text needs a marker to carry bytes losslessly, but the
//! native binding passes in-memory values, and `RuntimeValue` already *is* "the shape language's
//! value model — JSON values plus bytes" (spec §6.1) with no ambiguity to tag.

use super::ComponentError;
use super::parts::SlotValue;
use crate::plan::RuntimeValue;
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct Decode {
  pub content_type: String,
  pub value: SlotValue,
  pub options: Option<Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DecodeResult {
  pub document: RuntimeValue,
  /// Value-dependent losses this decode could not preserve (spec §6.4); empty when there are
  /// none, which is every case this task's components produce.
  pub degradations: Vec<Value>,
}

#[derive(Debug, Clone)]
pub struct Encode {
  pub content_type: String,
  pub document: RuntimeValue,
  pub options: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct EncodeResult {
  pub value: SlotValue,
}

#[derive(Debug, Clone)]
pub struct Compile {
  pub content_type: String,
  pub shape: Value,
  pub path: String,
}

#[derive(Debug, Clone)]
pub struct CompileResult {
  /// A plan fragment (design 2.4's grammar) spliced at `Compile::path`. `None` is legitimate —
  /// spec §6.3: "the kernel then compiles the slot generically and calls decode at execution
  /// time."
  pub fragment: Option<Value>,
  pub grammar_version: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Detect {
  pub value: SlotValue,
  pub hint: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DetectResult {
  pub media_type: Option<String>,
  pub confidence: Option<f64>,
}

/// Native binding (spec §9.1), same shape as [`super::transport::TransportComponent`] —
/// including its `Send + Sync` bound, for the same reason (plan task 4.5's live exchange loop
/// calls `decode`/`encode` from a background thread while the dispatch thread runs concurrently).
pub trait ContentComponent: Send + Sync {
  /// Whether this component handles `content_type` — the native projection of the `content-types`
  /// its handshake declares (spec §3.3), which is what a [`super::ContentRegistry`] routes on. Not an
  /// operation: it answers from the declaration, never from the octets.
  fn handles(&self, content_type: &str) -> bool;
  fn decode(&self, req: Decode) -> Result<DecodeResult, ComponentError>;
  fn encode(&self, req: Encode) -> Result<EncodeResult, ComponentError>;
  fn compile(&self, req: Compile) -> Result<CompileResult, ComponentError>;
  fn detect(&self, req: Detect) -> Result<DetectResult, ComponentError>;
}

/// `match:content-type`'s detector (shape-language spec §4.2, staged plan S1.10, ADR 0049), answered
/// by a content component: on the live paths, the session's registry, which asks each of its
/// components in turn (component-interfaces spec §6.5).
///
/// The operator inspects octets, not a decoded document, so a value resolved from a whole slot is
/// read from the arrival's own `parts`, where its octets and its label still are. On the contract
/// path (`by_label`), the slot's type is the one the interaction declares for it, else the one it
/// arrived labelled with, and detection answers only for a slot with neither: detection resolves an
/// absent type and never overrides one (§6.5). On the v1–v4 path a pact's `contentType` matcher
/// sniffs the octets whatever they are labelled, as today's verifier does, so only detection
/// answers. Any other value (a member of a document, text with no slot behind it) is detected as it
/// stands.
pub struct ContentDetection<'a> {
  pub content: &'a dyn ContentComponent,
  pub parts: Option<&'a super::Parts>,
  pub declared: Option<&'a crate::common::ContentTypes>,
  pub by_label: bool,
}

impl<'a> ContentDetection<'a> {
  /// Detection of values alone, with no arrival behind them: what a golden corpus case has.
  pub fn of(content: &'a dyn ContentComponent) -> Self {
    ContentDetection {
      content,
      parts: None,
      declared: None,
      by_label: false,
    }
  }

  fn detected(&self, value: SlotValue) -> Option<String> {
    self
      .content
      .detect(Detect { value, hint: None })
      .ok()
      .and_then(|result| result.media_type)
  }
}

impl crate::plan::ContentDetector for ContentDetection<'_> {
  fn detect(&self, value: &RuntimeValue, at: Option<&str>) -> Option<String> {
    let slot = at.and_then(|at| {
      let mut segments = at.strip_prefix("$.")?.split('.');
      match (segments.next(), segments.next(), segments.next()) {
        (Some(part), Some(slot), None) => self.parts?.get(part)?.get(slot).map(|raw| (part, slot, raw)),
        _ => None,
      }
    });
    if let Some((part, slot, raw)) = slot {
      if self.by_label
        && let Some(content_type) =
          crate::common::declared_type(self.declared, part, slot).or(raw.content_type.as_deref())
      {
        return Some(content_type.to_string());
      }
      return self.detected(raw.clone());
    }
    let value = match value {
      RuntimeValue::Bytes(bytes) => SlotValue {
        content: Value::String(base64::Engine::encode(
          &base64::engine::general_purpose::STANDARD,
          bytes,
        )),
        encoded: Some("base64".to_string()),
        content_type: None,
      },
      RuntimeValue::String(text) => SlotValue {
        content: Value::String(text.clone()),
        encoded: Some("text".to_string()),
        content_type: None,
      },
      _ => return None,
    };
    self.detected(value)
  }
}

#[cfg(test)]
mod detection_tests {
  use super::*;
  use crate::component::Parts;
  use crate::plan::ContentDetector;
  use serde_json::json;

  /// Detects JSON for a string or octets that start with `{`, and nothing else.
  struct Braces;

  impl ContentComponent for Braces {
    fn handles(&self, _content_type: &str) -> bool {
      false
    }
    fn decode(&self, _req: Decode) -> Result<DecodeResult, ComponentError> {
      Err(ComponentError::operation_unsupported("content/decode"))
    }
    fn encode(&self, _req: Encode) -> Result<EncodeResult, ComponentError> {
      Err(ComponentError::operation_unsupported("content/encode"))
    }
    fn compile(&self, _req: Compile) -> Result<CompileResult, ComponentError> {
      Err(ComponentError::operation_unsupported("content/compile"))
    }
    fn detect(&self, req: Detect) -> Result<DetectResult, ComponentError> {
      let text = match req.value.encoded.as_deref() {
        Some("base64") => String::from_utf8(
          base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            req.value.content.as_str().unwrap(),
          )
          .unwrap(),
        )
        .unwrap_or_default(),
        _ => req.value.content.as_str().unwrap_or_default().to_string(),
      };
      Ok(DetectResult {
        media_type: text.starts_with('{').then(|| "application/json".to_string()),
        confidence: None,
      })
    }
  }

  fn reply(label: Option<&str>, body: &str) -> Parts {
    let slot = SlotValue {
      content: json!(base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        body
      )),
      encoded: Some("base64".to_string()),
      content_type: label.map(str::to_string),
    };
    Parts::from([(
      "response".to_string(),
      BTreeMap::from([("body".to_string(), slot)]),
    )])
  }

  use std::collections::BTreeMap;

  #[test]
  fn a_whole_slot_is_its_declared_type_else_its_label_else_what_its_octets_are_detected_as() {
    let decoded = RuntimeValue::from_json(&json!({ "decoded": true }));
    let at = Some("$.response.body");
    let declared = BTreeMap::from([(
      "response".to_string(),
      BTreeMap::from([("body".to_string(), "text/csv".to_string())]),
    )]);
    let none = BTreeMap::new();
    let detect = |parts: &Parts, declared: &crate::common::ContentTypes, by_label: bool| {
      ContentDetection {
        content: &Braces,
        parts: Some(parts),
        declared: Some(declared),
        by_label,
      }
      .detect(&decoded, at)
    };
    let labelled = reply(Some("text/plain"), "{\"a\":1}");
    let unlabelled = reply(None, "{\"a\":1}");
    assert_eq!(detect(&labelled, &declared, true).as_deref(), Some("text/csv"));
    assert_eq!(detect(&labelled, &none, true).as_deref(), Some("text/plain"));
    assert_eq!(
      detect(&unlabelled, &none, true).as_deref(),
      Some("application/json")
    );
    // The v1–v4 path sniffs the octets, label or no label.
    assert_eq!(
      detect(&labelled, &declared, false).as_deref(),
      Some("application/json")
    );
    assert_eq!(detect(&reply(None, "plain"), &none, true), None);
  }

  #[test]
  fn a_value_with_no_slot_behind_it_is_detected_as_it_stands() {
    let detection = ContentDetection::of(&Braces);
    assert_eq!(
      detection
        .detect(
          &RuntimeValue::String("{}".to_string()),
          Some("$.response.body.inner")
        )
        .as_deref(),
      Some("application/json")
    );
    assert_eq!(detection.detect(&RuntimeValue::from_json(&json!(1)), None), None);
  }
}
