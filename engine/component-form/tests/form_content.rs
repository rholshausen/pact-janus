//! The built-in form content component (ADR 0038): which types it claims, decode, encode, their
//! round trip (component-interfaces spec §6.2), and what it refuses.

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use pact_janus_component_form::FormContent;
use pact_janus_kernel::component::{ContentComponent, Decode, Encode, SlotValue};
use pact_janus_kernel::plan::RuntimeValue;
use pretty_assertions::assert_eq;
use rstest::rstest;
use serde_json::{Value, json};

const FORM: &str = "application/x-www-form-urlencoded";

fn octets(body: &str) -> SlotValue {
  SlotValue {
    content: json!(BASE64.encode(body)),
    encoded: Some("base64".to_string()),
    content_type: Some(FORM.to_string()),
  }
}

fn decode(value: SlotValue) -> Result<Value, String> {
  FormContent::new()
    .decode(Decode {
      content_type: FORM.to_string(),
      value,
      options: None,
    })
    .map(|result| result.document.to_json())
    .map_err(|error| error.code)
}

fn encode(document: Value) -> Result<String, String> {
  FormContent::new()
    .encode(Encode {
      content_type: FORM.to_string(),
      document: RuntimeValue::from_json(&document),
      options: None,
    })
    .map(|result| {
      let bytes = BASE64.decode(result.value.content.as_str().unwrap()).unwrap();
      String::from_utf8(bytes).unwrap()
    })
    .map_err(|error| error.code)
}

#[test]
fn it_claims_the_form_type_and_nothing_else() {
  let form = FormContent::new();
  for claimed in [FORM, "application/x-www-form-urlencoded; charset=utf-8"] {
    assert!(form.handles(claimed), "{claimed}");
  }
  for refused in ["application/json", "text/plain", "multipart/form-data"] {
    assert!(!form.handles(refused), "{refused}");
  }
}

#[test]
fn a_body_decodes_to_each_name_with_its_values_in_order() {
  let document = decode(octets("b=2&a=1&b=3")).unwrap();
  assert_eq!(document, json!({ "b": ["2", "3"], "a": ["1"] }));
  assert_eq!(
    document.to_string(),
    r#"{"b":["2","3"],"a":["1"]}"#,
    "names in the order they first appear (ADR 0037)"
  );
}

#[rstest]
#[case::percent_escapes("name=J%C3%BCrgen%20M", json!({ "name": ["Jürgen M"] }))]
#[case::plus_is_a_space("q=a+b", json!({ "q": ["a b"] }))]
#[case::no_value("flag=&other", json!({ "flag": [""], "other": [""] }))]
#[case::empty("", json!({}))]
fn a_body_decodes_as_todays_verifier_reads_it(#[case] body: &str, #[case] expected: Value) {
  assert_eq!(decode(octets(body)), Ok(expected));
}

#[test]
fn a_recorded_document_decodes_to_itself() {
  let value = SlotValue {
    content: json!({ "a": ["1"] }),
    encoded: None,
    content_type: Some(FORM.to_string()),
  };
  assert_eq!(decode(value), Ok(json!({ "a": ["1"] })));
}

#[test]
fn a_document_encodes_its_members_in_order_and_a_string_as_one_value() {
  assert_eq!(
    encode(json!({ "b": ["2", "3"], "a": "1", "name": ["Jürgen M"] })),
    Ok("b=2&b=3&a=1&name=J%C3%BCrgen+M".to_string())
  );
}

#[test]
fn what_decode_produces_encodes_back_to_itself() {
  let document = decode(octets("b=2&a=1&b=3&c=x+y")).unwrap();
  let body = encode(document.clone()).unwrap();
  assert_eq!(decode(octets(&body)), Ok(document));
}

/// A number or a boolean is written as its text, as the XML component writes one (ADR 0048): a
/// generator in place of a form value may answer with a number.
#[test]
fn a_number_or_a_boolean_is_written_as_its_text() {
  assert_eq!(
    encode(json!({ "a": 1, "b": [true, "x", 2.5] })),
    Ok("a=1&b=true&b=x&b=2.5".to_string())
  );
}

#[rstest]
#[case::not_an_object(json!(["a"]))]
#[case::a_null(json!({ "a": null }))]
#[case::a_nested_object(json!({ "a": [{ "b": "c" }] }))]
fn a_document_that_is_not_names_and_strings_does_not_encode(#[case] document: Value) {
  assert_eq!(encode(document), Err("encode-failed".to_string()));
}

#[test]
fn content_that_is_neither_octets_nor_a_document_does_not_decode() {
  let value = SlotValue {
    content: json!(42),
    encoded: None,
    content_type: Some(FORM.to_string()),
  };
  assert_eq!(decode(value), Err("decode-failed".to_string()));
}
