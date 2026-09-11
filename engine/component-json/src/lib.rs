//! The built-in JSON content component (component-interfaces spec §6, plan task 4.2): the native
//! binding, in-tree but through the same interface a third-party component would use (ADR 0012).
//! Resolves kernel-boundary-review.md finding 1 — `plan::interpret`'s `match:content-type` no
//! longer guesses; it asks a real content component, and this is one.

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use pact_janus_kernel::component::{
  Compile, CompileResult, ComponentError, ContentComponent, Decode, DecodeResult, Detect, DetectResult,
  Encode, EncodeResult, SlotValue,
};
use pact_janus_kernel::plan::{ContentDetector, RuntimeValue};
use serde_json::Value;

/// `application/json` — and, per `ContentType::is_json()`, its `+json` suffix and `-json`-suffixed
/// subtype relatives (`application/hal+json`, `application/json-rpc`, ...).
pub struct JsonContent;

impl JsonContent {
  pub fn new() -> Self {
    JsonContent
  }
}

impl Default for JsonContent {
  fn default() -> Self {
    Self::new()
  }
}

/// A `SlotValue`'s tagged `content` as raw text — decoding `base64` (spec §2.4/§2.5) or taking
/// `text`/absent-tag content at face value. `None` means the tag names something this component
/// does not know how to turn into text (an open vocabulary — spec §2.2 rule 3: surfaced by name,
/// not guessed).
fn slot_text(value: &SlotValue) -> Result<Option<String>, ComponentError> {
  match value.encoded.as_deref() {
    Some("base64") => {
      let Some(text) = value.content.as_str() else {
        return Err(ComponentError::decode_failed(
          "application/json",
          "a base64-tagged slot's content must be a string",
        ));
      };
      let bytes = BASE64
        .decode(text)
        .map_err(|err| ComponentError::decode_failed("application/json", format!("invalid base64: {err}")))?;
      String::from_utf8(bytes)
        .map(Some)
        .map_err(|err| ComponentError::decode_failed("application/json", format!("not valid UTF-8: {err}")))
    }
    Some("text") => match value.content.as_str() {
      Some(text) => Ok(Some(text.to_string())),
      None => Err(ComponentError::decode_failed(
        "application/json",
        "a text-tagged slot's content must be a string",
      )),
    },
    None | Some("json") => Ok(None),
    Some(other) => Err(ComponentError::decode_failed(
      "application/json",
      format!("unrecognised content encoding '{other}'"),
    )),
  }
}

/// This component only speaks for JSON and its `+json`/`-json` relatives (`ContentType::is_json`).
fn require_json(content_type: &str) -> Result<(), ComponentError> {
  match pact_models::content_types::ContentType::parse(content_type) {
    Ok(ct) if ct.is_json() => Ok(()),
    _ => Err(ComponentError::unsupported_content_type(content_type)),
  }
}

impl ContentComponent for JsonContent {
  fn decode(&self, req: Decode) -> Result<DecodeResult, ComponentError> {
    require_json(&req.content_type)?;
    let document = match slot_text(&req.value)? {
      // The slot already carries its natural JSON value (no tag, or explicitly "json") — decode
      // is then just a representation change, not a parse.
      None => RuntimeValue::from_json(&req.value.content),
      Some(text) => {
        let value: Value = serde_json::from_str(&text)
          .map_err(|err| ComponentError::decode_failed(&req.content_type, err.to_string()))?;
        RuntimeValue::from_json(&value)
      }
    };
    Ok(DecodeResult {
      document,
      degradations: Vec::new(),
    })
  }

  fn encode(&self, req: Encode) -> Result<EncodeResult, ComponentError> {
    require_json(&req.content_type)?;
    let text = serde_json::to_string(&req.document.to_json())
      .map_err(|err| ComponentError::internal(format!("JSON serialises to itself: {err}")))?;
    Ok(EncodeResult {
      value: SlotValue {
        content: Value::String(BASE64.encode(text.as_bytes())),
        encoded: Some("base64".to_string()),
        content_type: Some("application/json".to_string()),
      },
    })
  }

  fn compile(&self, _req: Compile) -> Result<CompileResult, ComponentError> {
    // Legitimate per spec §6.3: the kernel compiles the slot generically and calls `decode` at
    // execution time. Nothing in this task needs a plan-fragment contribution.
    Ok(CompileResult {
      fragment: None,
      grammar_version: None,
    })
  }

  fn detect(&self, req: Detect) -> Result<DetectResult, ComponentError> {
    let media_type = match slot_text(&req.value)? {
      Some(text) => pact_models::content_types::detect_content_type_from_string(&text)
        .filter(|content_type| content_type.is_json())
        .map(|content_type| content_type.to_string()),
      // Already a structured, non-string JSON value: unambiguously JSON, nothing to sniff.
      None if !req.value.content.is_string() => Some("application/json".to_string()),
      None => None,
    };
    Ok(DetectResult {
      confidence: media_type.as_ref().map(|_| 1.0),
      media_type,
    })
  }
}

impl ContentDetector for JsonContent {
  fn detect(&self, value: &RuntimeValue) -> Option<String> {
    let slot_value = match value {
      RuntimeValue::Bytes(bytes) => SlotValue {
        content: Value::String(BASE64.encode(bytes)),
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
    ContentComponent::detect(
      self,
      Detect {
        value: slot_value,
        hint: None,
      },
    )
    .ok()
    .and_then(|result| result.media_type)
  }
}
