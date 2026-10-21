//! The built-in text content component (ADR 0036 decision 1): which types it claims, decode, encode,
//! their round trip (component-interfaces spec §6.2), and what it refuses.

use pact_janus_component_text::TextContent;
use pact_janus_kernel::component::{
  Compile, ContentComponent, ContentRegistry, Decode, Detect, Encode, SlotValue,
};
use pact_janus_kernel::plan::RuntimeValue;
use pretty_assertions::assert_eq;
use rstest::rstest;
use serde_json::{Value, json};
use std::sync::Arc;

fn base64_slot(content: &str, content_type: &str) -> SlotValue {
  SlotValue {
    content: json!(content),
    encoded: Some("base64".to_string()),
    content_type: Some(content_type.to_string()),
  }
}

fn decode(content_type: &str, value: SlotValue) -> Result<RuntimeValue, String> {
  TextContent::new()
    .decode(Decode {
      content_type: content_type.to_string(),
      value,
      options: None,
    })
    .map(|result| result.document)
    .map_err(|error| error.code)
}

#[test]
fn it_claims_every_text_type_and_nothing_else() {
  let text = TextContent::new();
  for claimed in ["text/plain", "text/html; charset=utf-8", "TEXT/CSV"] {
    assert!(text.handles(claimed), "{claimed}");
  }
  for refused in [
    "application/json",
    "image/jpeg",
    "application/octet-stream",
    "text",
  ] {
    assert!(!text.handles(refused), "{refused}");
  }
}

#[test]
fn octets_decode_as_a_utf8_string() {
  // "Hello Wörld!" in UTF-8.
  let document = decode("text/plain", base64_slot("SGVsbG8gV8O2cmxkIQ==", "text/plain"));
  assert_eq!(document, Ok(RuntimeValue::String("Hello Wörld!".to_string())));
}

#[test]
fn a_charset_does_not_change_how_text_is_read() {
  let document = decode(
    "text/plain; charset=iso-8859-1",
    base64_slot("YWJj", "text/plain"),
  );
  assert_eq!(document, Ok(RuntimeValue::String("abc".to_string())));
}

#[test]
fn text_already_carried_as_a_string_is_taken_as_it_stands() {
  let slot = SlotValue {
    content: json!("plain"),
    encoded: Some("text".to_string()),
    content_type: None,
  };
  assert_eq!(
    decode("text/plain", slot),
    Ok(RuntimeValue::String("plain".to_string()))
  );
}

#[test]
fn octets_that_are_not_utf8_do_not_decode() {
  // 0xff 0xfe.
  assert_eq!(
    decode("text/plain", base64_slot("//4=", "text/plain")),
    Err("decode-failed".to_string())
  );
}

#[test]
fn a_type_it_does_not_claim_is_refused() {
  assert_eq!(
    decode("application/json", base64_slot("e30=", "application/json")),
    Err("unsupported-content-type".to_string())
  );
}

#[test]
fn a_string_encodes_as_utf8_octets_labelled_as_asked_and_decodes_back() {
  let text = TextContent::new();
  let encoded = text
    .encode(Encode {
      content_type: "text/plain; charset=utf-8".to_string(),
      document: RuntimeValue::String("Hello Wörld!".to_string()),
      options: None,
    })
    .expect("a string encodes")
    .value;
  assert_eq!(
    encoded,
    base64_slot("SGVsbG8gV8O2cmxkIQ==", "text/plain; charset=utf-8")
  );
  assert_eq!(
    decode("text/plain; charset=utf-8", encoded),
    Ok(RuntimeValue::String("Hello Wörld!".to_string()))
  );
}

#[test]
fn a_document_that_is_not_a_string_does_not_encode() {
  let error = TextContent::new()
    .encode(Encode {
      content_type: "text/plain".to_string(),
      document: RuntimeValue::from_json(&json!({ "a": 1 })),
      options: None,
    })
    .expect_err("an object is not text");
  assert_eq!(error.code, "encode-failed");
}

fn compile(shape: Value) -> Result<(), String> {
  TextContent::new()
    .compile(Compile {
      content_type: "text/plain".to_string(),
      shape,
      path: "$.response.body".to_string(),
    })
    .map(|result| assert!(result.fragment.is_none()))
    .map_err(|error| error.code)
}

#[rstest]
#[case::string(json!({ "shape": "string" }))]
#[case::regex(json!({ "shape": "regex", "pattern": "a+", "example": "aa" }))]
#[case::equality_of_a_string(json!({ "shape": "equality", "example": "hi" }))]
#[case::nullable_string(json!({ "shape": "nullable", "of": { "shape": "string" } }))]
#[case::any_of_with_a_string(json!({ "shape": "any-of", "options": [1, "one"] }))]
#[case::one_of_with_a_string(json!({ "shape": "one-of", "alternatives": {
  "n": { "shape": "integer" }, "s": { "shape": "string" } } }))]
#[case::an_operator_it_does_not_know(json!({ "shape": "csv:row" }))]
fn a_shape_that_admits_a_string_compiles_to_the_generic_plan(#[case] shape: Value) {
  assert_eq!(compile(shape), Ok(()));
}

#[rstest]
#[case::object(json!({ "shape": "object", "members": {} }))]
#[case::each_like(json!({ "shape": "each-like", "items": { "shape": "string" }, "min": 1 }))]
#[case::integer(json!({ "shape": "integer", "example": 4 }))]
#[case::equality_of_a_number(json!({ "shape": "equality", "example": 4 }))]
#[case::nullable_object(json!({ "shape": "nullable", "of": { "shape": "object", "members": {} } }))]
#[case::any_of_numbers(json!({ "shape": "any-of", "options": [1, 2] }))]
fn a_shape_that_admits_no_string_is_refused(#[case] shape: Value) {
  assert_eq!(compile(shape), Err("encode-failed".to_string()));
}

#[test]
fn it_detects_nothing() {
  let error = TextContent::new()
    .detect(Detect {
      value: base64_slot("YWJj", "text/plain"),
      hint: None,
    })
    .expect_err("detection is not offered");
  assert_eq!(error.code, "operation-unsupported");
}

#[test]
fn a_component_registered_before_it_wins_its_type() {
  struct Csv;
  impl ContentComponent for Csv {
    fn handles(&self, content_type: &str) -> bool {
      content_type == "text/csv"
    }
    fn decode(
      &self,
      _req: Decode,
    ) -> Result<pact_janus_kernel::component::DecodeResult, pact_janus_kernel::component::ComponentError>
    {
      Ok(pact_janus_kernel::component::DecodeResult {
        document: RuntimeValue::String("csv".to_string()),
        degradations: Vec::new(),
      })
    }
    fn encode(
      &self,
      _req: Encode,
    ) -> Result<pact_janus_kernel::component::EncodeResult, pact_janus_kernel::component::ComponentError>
    {
      unreachable!()
    }
    fn compile(
      &self,
      _req: pact_janus_kernel::component::Compile,
    ) -> Result<pact_janus_kernel::component::CompileResult, pact_janus_kernel::component::ComponentError>
    {
      unreachable!()
    }
    fn detect(
      &self,
      _req: Detect,
    ) -> Result<pact_janus_kernel::component::DetectResult, pact_janus_kernel::component::ComponentError>
    {
      unreachable!()
    }
  }
  let registry = ContentRegistry::new()
    .with(Arc::new(Csv))
    .with(Arc::new(TextContent::new()));
  let decode = |content_type: &str| {
    registry
      .decode(Decode {
        content_type: content_type.to_string(),
        value: base64_slot("YWJj", content_type),
        options: None,
      })
      .map(|result| result.document)
      .ok()
  };
  assert_eq!(decode("text/csv"), Some(RuntimeValue::String("csv".to_string())));
  assert_eq!(
    decode("text/plain"),
    Some(RuntimeValue::String("abc".to_string()))
  );
}
