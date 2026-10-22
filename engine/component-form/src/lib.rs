//! The built-in form-urlencoded content component (component-interfaces spec §6, ADR 0038):
//! `application/x-www-form-urlencoded` octets to a document and back. In-tree, but bound through the
//! same interface a third-party component uses (ADR 0012), as the JSON and text ones are.
//!
//! A form body's document is the query slot's model: an object whose members are arrays of strings,
//! one member per name in the order the name first appears, its values in order (ADR 0038 decision 2,
//! ADR 0037). It is parsed and written with `serde_urlencoded`, as today's verifier parses and writes
//! one.

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use indexmap::IndexMap;
use pact_janus_kernel::component::{
  Compile, CompileResult, ComponentError, ContentComponent, Decode, DecodeResult, Detect, DetectResult,
  Encode, EncodeResult, SlotValue, media_type_matches,
};
use pact_janus_kernel::plan::RuntimeValue;
use serde_json::Value;

/// The media type this component declares.
pub const MEDIA_TYPE: &str = "application/x-www-form-urlencoded";

/// Form bodies, `application/x-www-form-urlencoded`.
pub struct FormContent;

impl FormContent {
  pub fn new() -> Self {
    FormContent
  }
}

impl Default for FormContent {
  fn default() -> Self {
    Self::new()
  }
}

/// `body` as a form document: each name once, in the order it first appears, with every value it
/// was given, in order. What `serde_urlencoded` decodes — percent-escapes, `+` as a space — is decoded.
pub fn parse(body: &[u8]) -> Result<RuntimeValue, String> {
  let pairs: Vec<(String, String)> = serde_urlencoded::from_bytes(body).map_err(|err| err.to_string())?;
  let mut names: IndexMap<String, Vec<RuntimeValue>> = IndexMap::new();
  for (name, value) in pairs {
    names.entry(name).or_default().push(RuntimeValue::String(value));
  }
  Ok(RuntimeValue::Object(
    names
      .into_iter()
      .map(|(name, values)| (name, RuntimeValue::Array(values)))
      .collect(),
  ))
}

/// A form document as a body: each member's values in order, a member that is one string as one
/// value. Repeated names come out together, whatever order they were parsed in (ADR 0038 decision 2).
pub fn write(document: &RuntimeValue) -> Result<String, String> {
  let RuntimeValue::Object(members) = document else {
    return Err("a form body is an object of names and their values".to_string());
  };
  let mut pairs: Vec<(&str, &str)> = Vec::new();
  for (name, values) in members {
    match values {
      RuntimeValue::String(value) => pairs.push((name, value)),
      RuntimeValue::Array(values) => {
        for value in values {
          let RuntimeValue::String(value) = value else {
            return Err(format!("the values of '{name}' must be strings"));
          };
          pairs.push((name, value));
        }
      }
      _ => return Err(format!("'{name}' must be a string or a list of strings")),
    }
  }
  serde_urlencoded::to_string(pairs).map_err(|err| err.to_string())
}

fn require_form(content_type: &str) -> Result<(), ComponentError> {
  if media_type_matches(MEDIA_TYPE, content_type) {
    Ok(())
  } else {
    Err(ComponentError::unsupported_content_type(content_type))
  }
}

/// A slot's content as a document: `base64` octets or text are parsed; a document already decoded,
/// as a contract records one, is taken as it is.
fn slot_document(content_type: &str, value: &SlotValue) -> Result<RuntimeValue, ComponentError> {
  let failed = |message: String| ComponentError::decode_failed(content_type, message);
  match (&value.content, value.encoded.as_deref()) {
    (Value::String(content), Some("base64")) => {
      let bytes = BASE64
        .decode(content)
        .map_err(|err| failed(format!("invalid base64: {err}")))?;
      parse(&bytes).map_err(failed)
    }
    (Value::String(content), None | Some("text") | Some("json")) => parse(content.as_bytes()).map_err(failed),
    (Value::Object(_), None | Some("json")) => Ok(RuntimeValue::from_json(&value.content)),
    (_, Some(other)) if !matches!(other, "base64" | "text" | "json") => {
      Err(failed(format!("unrecognised content encoding '{other}'")))
    }
    _ => Err(failed(
      "a form slot's content must be its text or an object of names".to_string(),
    )),
  }
}

impl ContentComponent for FormContent {
  fn handles(&self, content_type: &str) -> bool {
    require_form(content_type).is_ok()
  }

  fn decode(&self, req: Decode) -> Result<DecodeResult, ComponentError> {
    require_form(&req.content_type)?;
    Ok(DecodeResult {
      document: slot_document(&req.content_type, &req.value)?,
      degradations: Vec::new(),
    })
  }

  fn encode(&self, req: Encode) -> Result<EncodeResult, ComponentError> {
    require_form(&req.content_type)?;
    let body =
      write(&req.document).map_err(|message| ComponentError::encode_failed(&req.content_type, message))?;
    Ok(EncodeResult {
      value: SlotValue {
        content: Value::String(BASE64.encode(body.as_bytes())),
        encoded: Some("base64".to_string()),
        content_type: Some(req.content_type),
      },
    })
  }

  /// Nothing to contribute (spec §6.3): a form document is an object of string lists, which the
  /// generic plan handles.
  fn compile(&self, req: Compile) -> Result<CompileResult, ComponentError> {
    require_form(&req.content_type)?;
    Ok(CompileResult {
      fragment: None,
      grammar_version: None,
    })
  }

  /// A form body has no signature to recognise (spec §6.5).
  fn detect(&self, _req: Detect) -> Result<DetectResult, ComponentError> {
    Err(ComponentError::operation_unsupported("content/detect"))
  }
}
