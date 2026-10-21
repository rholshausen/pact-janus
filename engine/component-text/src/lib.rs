//! The built-in plain-text content component (component-interfaces spec §6, ADR 0036 decision 1):
//! `text/*` octets to a string document and back. In-tree, but bound through the same interface a
//! third-party component uses (ADR 0012), as the JSON one is.
//!
//! Every embedding registers it after JSON, and a session's declared components come before both,
//! so a component for a particular text type (CSV, and S1.22's XML) is asked first and this one
//! answers for the rest.

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use pact_janus_kernel::component::{
  Compile, CompileResult, ComponentError, ContentComponent, Decode, DecodeResult, Detect, DetectResult,
  Encode, EncodeResult, SlotValue, media_type_matches,
};
use pact_janus_kernel::plan::RuntimeValue;
use serde_json::Value;

/// The media types this component declares.
pub const MEDIA_TYPE: &str = "text/*";

/// Text in any `text/` type, read as UTF-8 whatever its `charset` says, as today's verifier reads it.
pub struct TextContent;

impl TextContent {
  pub fn new() -> Self {
    TextContent
  }
}

impl Default for TextContent {
  fn default() -> Self {
    Self::new()
  }
}

fn require_text(content_type: &str) -> Result<(), ComponentError> {
  if media_type_matches(MEDIA_TYPE, content_type) {
    Ok(())
  } else {
    Err(ComponentError::unsupported_content_type(content_type))
  }
}

/// A slot's content as text: `base64` octets that are UTF-8, or content already carried as a string.
fn slot_text(content_type: &str, value: &SlotValue) -> Result<String, ComponentError> {
  let Some(content) = value.content.as_str() else {
    return Err(ComponentError::decode_failed(
      content_type,
      "a text slot's content must be a string",
    ));
  };
  match value.encoded.as_deref() {
    Some("base64") => {
      let bytes = BASE64
        .decode(content)
        .map_err(|err| ComponentError::decode_failed(content_type, format!("invalid base64: {err}")))?;
      String::from_utf8(bytes)
        .map_err(|err| ComponentError::decode_failed(content_type, format!("not valid UTF-8: {err}")))
    }
    None | Some("text") | Some("json") => Ok(content.to_string()),
    Some(other) => Err(ComponentError::decode_failed(
      content_type,
      format!("unrecognised content encoding '{other}'"),
    )),
  }
}

impl ContentComponent for TextContent {
  fn handles(&self, content_type: &str) -> bool {
    require_text(content_type).is_ok()
  }

  fn decode(&self, req: Decode) -> Result<DecodeResult, ComponentError> {
    require_text(&req.content_type)?;
    Ok(DecodeResult {
      document: RuntimeValue::String(slot_text(&req.content_type, &req.value)?),
      degradations: Vec::new(),
    })
  }

  fn encode(&self, req: Encode) -> Result<EncodeResult, ComponentError> {
    require_text(&req.content_type)?;
    let RuntimeValue::String(text) = &req.document else {
      return Err(ComponentError::encode_failed(
        &req.content_type,
        "only a string can be written as text",
      ));
    };
    Ok(EncodeResult {
      value: SlotValue {
        content: Value::String(BASE64.encode(text.as_bytes())),
        encoded: Some("base64".to_string()),
        content_type: Some(req.content_type),
      },
    })
  }

  /// Nothing to contribute (spec §6.3): a text body is one string, which the generic plan handles.
  /// A shape that admits no string at all, such as rows of a `text/csv` body, is refused: this
  /// component cannot write it, and the slot needs a component for its type (ADR 0036).
  fn compile(&self, req: Compile) -> Result<CompileResult, ComponentError> {
    require_text(&req.content_type)?;
    if !admits_a_string(&req.shape) {
      return Err(ComponentError::encode_failed(
        &req.content_type,
        format!(
          "the built-in text component writes a body as one string, and this shape ('{}') admits no string",
          req.shape.get("shape").and_then(Value::as_str).unwrap_or("?")
        ),
      ));
    }
    Ok(CompileResult {
      fragment: None,
      grammar_version: None,
    })
  }

  /// Any UTF-8 is plausibly text, so recognising it would claim every body: detection is left to
  /// components whose types have a signature (spec §6.5).
  fn detect(&self, _req: Detect) -> Result<DetectResult, ComponentError> {
    Err(ComponentError::operation_unsupported("content/detect"))
  }
}

/// Whether a shape (shape-language spec §4) admits some string. Conservative: only a shape that can
/// never be a string is refused, and an operator this component does not know is taken to admit one.
fn admits_a_string(shape: &Value) -> bool {
  let example_is_a_string = || shape.get("example").is_none_or(Value::is_string);
  match shape.get("shape").and_then(Value::as_str) {
    Some(
      "object" | "array" | "each-like" | "each-entry" | "contains" | "number" | "integer" | "decimal"
      | "boolean" | "null",
    ) => false,
    Some("equality" | "type") => example_is_a_string(),
    Some("optional" | "nullable") => shape.get("of").is_none_or(admits_a_string),
    Some("any-of") => shape
      .get("options")
      .and_then(Value::as_array)
      .is_none_or(|options| options.iter().any(Value::is_string)),
    Some("one-of") => shape
      .get("alternatives")
      .and_then(Value::as_object)
      .is_none_or(|alternatives| alternatives.values().any(admits_a_string)),
    _ => true,
  }
}
