//! Routing content by media type (component-interfaces spec §6, contract-file spec §5.5): the
//! registry that replaced the engine's single content slot when plan task 8.1 put a second content
//! component behind it.
//!
//! A [`ContentRegistry`] is itself a [`ContentComponent`], so everything downstream of it — the
//! exchange loop, the verifier, the wire helpers — still holds one `dyn ContentComponent` and never
//! learns how many there are. It asks each registered component whether it [`handles`] a media type,
//! in registration order, and hands the call to the first that does. The kernel still knows no media
//! type: it compares strings the components declared.
//!
//! The registry is also where embedded content is resolved (component-interfaces spec §6.6, ADR 0103).
//! A component that finds content of another type inside its own, a multipart part's body, answers
//! with the embedded slots it needs instead of a result; the registry decodes or encodes each through
//! itself, so it reaches every component in the scope, declared ones included, and asks the component
//! again with the answers. The component is never called back while it is working, so the engine stays
//! its only caller (spec §3.1) and no binding changes.
//!
//! [`handles`]: ContentComponent::handles

use super::content::{
  Compile, CompileResult, ContentComponent, Decode, DecodeResult, Detect, DetectResult, Embedded,
  EmbeddedDecode, EmbeddedEncode, Encode, EncodeResult, Label, LabelResult,
};
use super::error::ComponentError;
use super::parts::SlotValue;
use crate::plan::RuntimeValue;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde_json::Value;
use std::collections::BTreeSet;
use std::sync::Arc;

/// How deeply content may be embedded in content (spec §6.6): a slot is depth 0, a part's body 1, a
/// part of a part's body 2. Deeper content is answered with `embedding-too-deep`, so a component that
/// asks for its own type again and again ends.
pub const EMBEDDING_DEPTH: usize = 8;

/// Content components in precedence order. Earlier wins: a session's declared components are
/// registered before the embedding's in-tree ones, so a project that declares its own handler for
/// a type the engine also handles gets the one it declared.
#[derive(Clone, Default)]
pub struct ContentRegistry {
  components: Vec<Arc<dyn ContentComponent>>,
}

impl ContentRegistry {
  pub fn new() -> Self {
    Self::default()
  }

  /// Register `component` after every component already here.
  pub fn with(mut self, component: Arc<dyn ContentComponent>) -> Self {
    self.components.push(component);
    self
  }

  /// This registry's components followed by `fallback`'s: a session's own components ahead of the
  /// engine's in-tree ones.
  pub fn then(mut self, fallback: &ContentRegistry) -> Self {
    self.components.extend(fallback.components.iter().cloned());
    self
  }

  pub fn is_empty(&self) -> bool {
    self.components.is_empty()
  }

  /// The component that handles `content_type`, if any does.
  pub fn route(&self, content_type: &str) -> Option<&Arc<dyn ContentComponent>> {
    self
      .components
      .iter()
      .find(|component| component.handles(content_type))
  }

  fn routed(&self, content_type: &str) -> Result<&Arc<dyn ContentComponent>, ComponentError> {
    self
      .route(content_type)
      .ok_or_else(|| ComponentError::unsupported_content_type(content_type))
  }
}

impl ContentComponent for ContentRegistry {
  fn handles(&self, content_type: &str) -> bool {
    self.route(content_type).is_some()
  }

  fn decode(&self, req: Decode) -> Result<DecodeResult, ComponentError> {
    self.decode_at(req, 0)
  }

  fn encode(&self, req: Encode) -> Result<EncodeResult, ComponentError> {
    self.encode_at(req, 0)
  }

  fn compile(&self, req: Compile) -> Result<CompileResult, ComponentError> {
    self.routed(&req.content_type)?.compile(req)
  }

  /// A member's label is the decoding component's to give: the call goes where `decode` would.
  fn label(&self, req: Label) -> Result<LabelResult, ComponentError> {
    self.routed(&req.content_type)?.label(req)
  }

  /// Detection resolves an *absent* type (spec §6.5), so there is nothing to route on: the first
  /// component that recognises the octets answers. A component that does not implement detection
  /// answers `operation-unsupported`, which here means "not me".
  fn detect(&self, req: Detect) -> Result<DetectResult, ComponentError> {
    for component in &self.components {
      if let Ok(result) = component.detect(req.clone())
        && result.media_type.is_some()
      {
        return Ok(result);
      }
    }
    Ok(DetectResult {
      media_type: None,
      confidence: None,
    })
  }
}

impl ContentRegistry {
  /// `req` decoded by the component that handles its type, resolving what it asks for (spec §6.6).
  /// `depth` is how deeply `req`'s octets are embedded.
  fn decode_at(&self, req: Decode, depth: usize) -> Result<DecodeResult, ComponentError> {
    let component = self.routed(&req.content_type)?;
    let asked = component.decode(req.clone())?;
    if asked.embedded.is_empty() {
      return Ok(asked);
    }
    unique_ids(
      asked.embedded.iter().map(|embedded| embedded.id.as_str()),
      &req.content_type,
    )?;
    let answers: Embedded<DecodeResult> = asked
      .embedded
      .into_iter()
      .map(|embedded| (embedded.id.clone(), self.embedded_decode(embedded, depth + 1)))
      .collect();
    let answered = component.decode(Decode {
      embedded: Some(answers),
      ..req
    })?;
    if !answered.embedded.is_empty() {
      return Err(asked_twice("content/decode", &answered_type(&answered.embedded)));
    }
    Ok(answered)
  }

  /// One embedded slot, decoded as a slot is: by the component that handles its type, else as its
  /// octets, which is what content no component claims is (spec §6.1).
  fn embedded_decode(&self, embedded: EmbeddedDecode, depth: usize) -> Result<DecodeResult, ComponentError> {
    if depth > EMBEDDING_DEPTH {
      return Err(too_deep(&embedded.content_type));
    }
    if self.route(&embedded.content_type).is_none() {
      return octets(&embedded.value).map(|bytes| DecodeResult::of(RuntimeValue::Bytes(bytes)));
    }
    self.decode_at(Decode::of(embedded.content_type, embedded.value), depth)
  }

  /// `req` encoded by the component that handles its type, resolving what it asks for (spec §6.6).
  fn encode_at(&self, req: Encode, depth: usize) -> Result<EncodeResult, ComponentError> {
    let component = self.routed(&req.content_type)?;
    let asked = component.encode(req.clone())?;
    if asked.embedded.is_empty() {
      return Ok(asked);
    }
    unique_ids(
      asked.embedded.iter().map(|embedded| embedded.id.as_str()),
      &req.content_type,
    )?;
    let answers: Embedded<EncodeResult> = asked
      .embedded
      .into_iter()
      .map(|embedded| {
        let shape = req
          .shape
          .as_ref()
          .zip(embedded.at.as_deref())
          .and_then(|(shape, at)| shape_at(shape, at));
        (
          embedded.id.clone(),
          self.embedded_encode(embedded, shape, depth + 1),
        )
      })
      .collect();
    let answered = component.encode(Encode {
      embedded: Some(answers),
      ..req
    })?;
    if !answered.embedded.is_empty() {
      let types: Vec<&str> = answered
        .embedded
        .iter()
        .map(|e| e.content_type.as_str())
        .collect();
      return Err(asked_twice("content/encode", &types.join(", ")));
    }
    Ok(answered)
  }

  /// One embedded document, encoded as a slot is: by the component that handles its type. Octets
  /// need no component, and are written as they are whatever their type.
  fn embedded_encode(
    &self,
    embedded: EmbeddedEncode,
    shape: Option<Value>,
    depth: usize,
  ) -> Result<EncodeResult, ComponentError> {
    if depth > EMBEDDING_DEPTH {
      return Err(too_deep(&embedded.content_type));
    }
    if let RuntimeValue::Bytes(bytes) = &embedded.document
      && self.route(&embedded.content_type).is_none()
    {
      return Ok(EncodeResult::of(SlotValue {
        content: Value::String(BASE64.encode(bytes)),
        encoded: Some("base64".to_string()),
        content_type: Some(embedded.content_type),
      }));
    }
    self.encode_at(
      Encode {
        shape,
        ..Encode::of(embedded.content_type, embedded.document)
      },
      depth,
    )
  }
}

fn answered_type(embedded: &[EmbeddedDecode]) -> String {
  embedded
    .iter()
    .map(|embedded| embedded.content_type.as_str())
    .collect::<Vec<_>>()
    .join(", ")
}

/// A component asks once (spec §6.6): the request that carries its answers must be answered.
fn asked_twice(op: &str, types: &str) -> ComponentError {
  ComponentError::synthesised(
    "component-malformed",
    format!(
      "the component answered '{op}' with more embedded content ({types}) after it was given what it asked for"
    ),
  )
}

/// Every id a component gives the content it asks for names one of them (spec §6.6).
fn unique_ids<'a>(ids: impl Iterator<Item = &'a str>, content_type: &str) -> Result<(), ComponentError> {
  let mut seen = BTreeSet::new();
  for id in ids {
    if !seen.insert(id) {
      return Err(ComponentError::synthesised(
        "component-malformed",
        format!(
          "the component decoding '{content_type}' asked for embedded content under the id '{id}' twice"
        ),
      ));
    }
  }
  Ok(())
}

fn too_deep(content_type: &str) -> ComponentError {
  let mut error = ComponentError::synthesised(
    "embedding-too-deep",
    format!("'{content_type}' is embedded more than {EMBEDDING_DEPTH} levels deep"),
  );
  error.details = Some(Box::new(serde_json::json!({ "content-type": content_type })));
  error
}

/// A slot value's octets: `base64` decoded, text as its UTF-8, and a JSON value as its text.
fn octets(value: &SlotValue) -> Result<Vec<u8>, ComponentError> {
  match (&value.content, value.encoded.as_deref()) {
    (Value::String(content), Some("base64")) => BASE64.decode(content).map_err(|err| {
      ComponentError::synthesised(
        "component-malformed",
        format!("embedded content is not valid base64: {err}"),
      )
    }),
    (Value::String(content), _) => Ok(content.as_bytes().to_vec()),
    (Value::Null, _) => Ok(Vec::new()),
    (other, _) => Ok(other.to_string().into_bytes()),
  }
}

/// The part of `shape` describing the value at `pointer` (RFC 6901) in a document it admits, when the
/// way there is unambiguous: through an `object`'s members, an `array`'s entries, an `each-like`'s
/// items and an `each-entry`'s values, and into what `optional` and `nullable` wrap. Anything else,
/// an alternative or a component's operator, has no one shape there.
fn shape_at(shape: &Value, pointer: &str) -> Option<Value> {
  let mut node = shape;
  let segments = pointer.strip_prefix('/').map(|rest| rest.split('/'));
  for segment in segments.into_iter().flatten() {
    let segment = segment.replace("~1", "/").replace("~0", "~");
    while matches!(operator(node), Some("optional" | "nullable")) {
      node = node.get("of")?;
    }
    node = match operator(node)? {
      "object" => node.get("members")?.get(&segment)?,
      "array" => node.get("entries")?.get(segment.parse::<usize>().ok()?)?,
      "each-like" => {
        segment.parse::<usize>().ok()?;
        node.get("items")?
      }
      "each-entry" => node.get("values")?,
      _ => return None,
    };
  }
  Some(node.clone())
}

fn operator(shape: &Value) -> Option<&str> {
  shape.get("shape").and_then(Value::as_str)
}

/// Whether `content_type` (a header value: `text/csv; charset=utf-8`) is one `pattern` declares
/// (a handshake's `media-type`: `text/csv`, `text/*` or `*/*`). Parameters are ignored and case is
/// not significant (RFC 9110 §8.3.1) — what a content type *is* is its essence.
pub fn media_type_matches(pattern: &str, content_type: &str) -> bool {
  let essence = |value: &str| {
    value
      .split(';')
      .next()
      .unwrap_or_default()
      .trim()
      .to_ascii_lowercase()
  };
  let (pattern, content_type) = (essence(pattern), essence(content_type));
  let (Some((pattern_type, pattern_subtype)), Some((main_type, subtype))) =
    (pattern.split_once('/'), content_type.split_once('/'))
  else {
    return false;
  };
  (pattern_type == "*" || pattern_type == main_type) && (pattern_subtype == "*" || pattern_subtype == subtype)
}

#[cfg(test)]
mod tests {
  use super::*;
  use pretty_assertions::assert_eq;
  use rstest::rstest;
  use serde_json::json;

  /// Content of `x/wrap`, whose octets are another type's: `<type>\n<payload>`. It asks for the
  /// payload as embedded content, and answers with `{ "inner": <its document> }`, or `{ "failed":
  /// <code> }`. `twice` asks again when given its answers; `duplicate` asks twice under one id.
  #[derive(Default)]
  struct Wrap {
    twice: bool,
    duplicate: bool,
  }

  fn text_of(value: &SlotValue) -> String {
    String::from_utf8(octets(value).unwrap()).unwrap()
  }

  fn text_slot(text: &str) -> SlotValue {
    SlotValue {
      content: json!(text),
      encoded: Some("text".to_string()),
      content_type: None,
    }
  }

  impl ContentComponent for Wrap {
    fn handles(&self, content_type: &str) -> bool {
      content_type == "x/wrap"
    }
    fn decode(&self, req: Decode) -> Result<DecodeResult, ComponentError> {
      let text = text_of(&req.value);
      let (content_type, payload) = text.split_once('\n').unwrap_or((&text, ""));
      let ask = EmbeddedDecode {
        id: "inner".to_string(),
        content_type: content_type.to_string(),
        value: text_slot(payload),
      };
      match req.embedded {
        Some(_) if self.twice => Ok(DecodeResult::asking(vec![ask])),
        Some(answers) => Ok(DecodeResult::of(RuntimeValue::from_json(
          &match &answers["inner"] {
            Ok(result) => json!({ "inner": result.document.to_json() }),
            Err(error) => json!({ "failed": error.code }),
          },
        ))),
        None if self.duplicate => Ok(DecodeResult::asking(vec![ask.clone(), ask])),
        None => Ok(DecodeResult::asking(vec![ask])),
      }
    }
    fn encode(&self, req: Encode) -> Result<EncodeResult, ComponentError> {
      let ask = EmbeddedEncode {
        id: "inner".to_string(),
        content_type: "x/text".to_string(),
        document: req.document.clone(),
        at: Some("/inner".to_string()),
      };
      match req.embedded {
        None => Ok(EncodeResult::asking(vec![ask])),
        Some(answers) => {
          let inner = answers["inner"].as_ref().map_err(Clone::clone)?;
          Ok(EncodeResult::of(text_slot(&format!(
            "x/text\n{}",
            text_of(&inner.value)
          ))))
        }
      }
    }
    fn compile(&self, _req: Compile) -> Result<CompileResult, ComponentError> {
      Ok(CompileResult::default())
    }
    fn detect(&self, _req: Detect) -> Result<DetectResult, ComponentError> {
      Err(ComponentError::operation_unsupported("content/detect"))
    }
  }

  /// `x/text`: its octets as a string, and a string as its octets, with the shape it was given
  /// written first.
  struct Text;

  impl ContentComponent for Text {
    fn handles(&self, content_type: &str) -> bool {
      content_type == "x/text"
    }
    fn decode(&self, req: Decode) -> Result<DecodeResult, ComponentError> {
      Ok(DecodeResult::of(RuntimeValue::String(text_of(&req.value))))
    }
    fn encode(&self, req: Encode) -> Result<EncodeResult, ComponentError> {
      let shape = req.shape.map(|shape| shape.to_string()).unwrap_or_default();
      Ok(EncodeResult::of(text_slot(&format!(
        "{shape}{}",
        req.document.to_json()
      ))))
    }
    fn compile(&self, _req: Compile) -> Result<CompileResult, ComponentError> {
      Ok(CompileResult::default())
    }
    fn detect(&self, _req: Detect) -> Result<DetectResult, ComponentError> {
      Err(ComponentError::operation_unsupported("content/detect"))
    }
  }

  fn registry(wrap: Wrap) -> ContentRegistry {
    ContentRegistry::new().with(Arc::new(wrap)).with(Arc::new(Text))
  }

  fn decoded(registry: &ContentRegistry, text: &str) -> Result<Value, ComponentError> {
    registry
      .decode(Decode::of("x/wrap", text_slot(text)))
      .map(|result| result.document.to_json())
  }

  #[test]
  fn embedded_content_is_decoded_by_the_component_for_its_type_and_the_answer_given_back() {
    assert_eq!(
      decoded(&registry(Wrap::default()), "x/text\nhello").unwrap(),
      json!({ "inner": "hello" })
    );
  }

  /// Spec §6.1: octets no component claims are bytes.
  #[test]
  fn embedded_content_no_component_claims_is_its_octets() {
    let result = registry(Wrap::default())
      .decode(Decode::of("x/wrap", text_slot("image/png\nPNG")))
      .unwrap();
    assert_eq!(
      result.document,
      RuntimeValue::Object(
        [("inner".to_string(), RuntimeValue::String(BASE64.encode("PNG")))]
          .into_iter()
          .collect()
      ),
      "the answer was octets, which to_json writes as base64"
    );
  }

  #[test]
  fn content_embedded_in_itself_ends_at_the_depth_bound() {
    let mut text = "x/text\nend".to_string();
    for _ in 0..12 {
      text = format!("x/wrap\n{text}");
    }
    let mut document = decoded(&registry(Wrap::default()), &text).unwrap();
    let mut depth = 0;
    while let Some(inner) = document.get("inner").cloned() {
      document = inner;
      depth += 1;
    }
    assert_eq!(depth, EMBEDDING_DEPTH);
    assert_eq!(document, json!({ "failed": "embedding-too-deep" }));
  }

  #[rstest]
  #[case(Wrap { twice: true, duplicate: false }, "after it was given what it asked for")]
  #[case(Wrap { twice: false, duplicate: true }, "under the id 'inner' twice")]
  fn a_component_asks_once_and_names_each_thing_it_asks_for_once(#[case] wrap: Wrap, #[case] message: &str) {
    let error = decoded(&registry(wrap), "x/text\nhello").unwrap_err();
    assert_eq!(error.code, "component-malformed");
    assert_eq!(error.source.as_deref(), Some("engine"));
    assert!(error.message.contains(message), "{}", error.message);
  }

  #[test]
  fn an_embedded_document_is_encoded_with_the_part_of_the_shape_where_it_sits() {
    let shape = json!({ "shape": "object", "members": { "inner": { "shape": "string" } } });
    let encoded = registry(Wrap::default())
      .encode(Encode {
        shape: Some(shape),
        ..Encode::of("x/wrap", RuntimeValue::String("hi".to_string()))
      })
      .unwrap();
    assert_eq!(text_of(&encoded.value), "x/text\n{\"shape\":\"string\"}\"hi\"");
  }

  #[rstest]
  #[case("", Some(json!({ "shape": "object", "members": { "a": { "shape": "optional", "of": { "shape": "each-like", "min": 0, "items": { "shape": "string" } } } } })))]
  #[case("/a", Some(json!({ "shape": "optional", "of": { "shape": "each-like", "min": 0, "items": { "shape": "string" } } })))]
  #[case("/a/3", Some(json!({ "shape": "string" })))]
  #[case("/a/x", None)]
  #[case("/b", None)]
  fn a_shape_is_found_at_a_pointer_through_structures_and_modifiers(
    #[case] pointer: &str,
    #[case] expected: Option<Value>,
  ) {
    let shape = json!({ "shape": "object", "members": { "a": { "shape": "optional", "of": { "shape": "each-like", "min": 0, "items": { "shape": "string" } } } } });
    assert_eq!(shape_at(&shape, pointer), expected);
  }

  #[test]
  fn a_shape_is_found_through_arrays_and_entries_and_not_through_alternatives() {
    let array = json!({ "shape": "array", "entries": [{ "shape": "integer" }, { "shape": "string" }] });
    assert_eq!(shape_at(&array, "/1"), Some(json!({ "shape": "string" })));
    assert_eq!(shape_at(&array, "/2"), None);
    let entries = json!({ "shape": "each-entry", "values": { "shape": "boolean" } });
    assert_eq!(shape_at(&entries, "/a~1b"), Some(json!({ "shape": "boolean" })));
    let one_of = json!({ "shape": "one-of", "discriminator": "k", "alternatives": {} });
    assert_eq!(shape_at(&one_of, "/k"), None);
  }
}
