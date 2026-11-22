//! The built-in multipart component (ADR 0040): what a body's document is, and that `encode` writes
//! back what `decode` reads. A part's body is decoded through the registry (ADR 0103), so the
//! component is called through one holding the built-in components every embedding registers, in
//! their order.

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use pact_janus_component_form::FormContent;
use pact_janus_component_json::JsonContent;
use pact_janus_component_multipart::{MultipartContent, RawPart, split};
use pact_janus_component_text::TextContent;
use pact_janus_component_xml::XmlContent;
use pact_janus_kernel::component::{
  Compile, CompileResult, ComponentError, ContentComponent, ContentRegistry, Decode, DecodeResult, Detect,
  DetectResult, Encode, EncodeResult, Label, SlotValue,
};
use pact_janus_kernel::plan::RuntimeValue;
use pretty_assertions::assert_eq;
use rstest::rstest;
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};

/// The built-in content components, registered as every embedding registers them.
fn built_ins() -> ContentRegistry {
  ContentRegistry::new()
    .with(Arc::new(JsonContent::new()))
    .with(Arc::new(XmlContent::new()))
    .with(Arc::new(TextContent::new()))
    .with(Arc::new(FormContent::new()))
    .with(Arc::new(MultipartContent::new()))
}

/// `body` as a multipart document, through [`built_ins`].
fn parse(body: &[u8], content_type: &str) -> Result<RuntimeValue, String> {
  built_ins()
    .decode(Decode::of(content_type, octets(body, content_type)))
    .map(|result| result.document)
    .map_err(|err| err.message)
}

fn document(value: Value) -> RuntimeValue {
  RuntimeValue::from_json(&value)
}

fn octets(body: &[u8], content_type: &str) -> SlotValue {
  SlotValue {
    content: Value::String(BASE64.encode(body)),
    encoded: Some("base64".to_string()),
    content_type: Some(content_type.to_string()),
  }
}

fn decode(body: &[u8], content_type: &str) -> RuntimeValue {
  built_ins()
    .decode(Decode {
      content_type: content_type.to_string(),
      value: octets(body, content_type),
      options: None,
      embedded: None,
    })
    .expect("decodes")
    .document
}

/// The compatibility suite's body: unnamed parts, the second base64-encoded.
const MIXED: &str = "--gc0p4Jq0M2Yt08jU534c0p\r\nContent-Type: text/plain\r\n\r\nThis is the body of the message.\r\n\
--gc0p4Jq0M2Yt08jU534c0p\r\nContent-Type: application/octet-stream\r\nContent-Transfer-Encoding: base64\r\n\r\n\
PGh0bWw+PC9odG1sPg==\r\n--gc0p4Jq0M2Yt08jU534c0p--";
const MIXED_TYPE: &str = "multipart/mixed; boundary=gc0p4Jq0M2Yt08jU534c0p";

#[test]
fn unnamed_parts_are_under_the_empty_name_in_order_and_base64_is_undone() {
  assert_eq!(
    parse(MIXED.as_bytes(), MIXED_TYPE),
    Ok(RuntimeValue::Object(
      [(
        String::new(),
        RuntimeValue::Array(vec![
          document(json!({
            "headers": { "content-type": "text/plain" },
            "body": "This is the body of the message."
          })),
          RuntimeValue::Object(
            [
              (
                "headers".to_string(),
                document(json!({
                  "content-type": "application/octet-stream",
                  "content-transfer-encoding": "base64"
                })),
              ),
              ("body".to_string(), RuntimeValue::Bytes(b"<html></html>".to_vec())),
            ]
            .into_iter()
            .collect()
          ),
        ]),
      )]
      .into_iter()
      .collect()
    ))
  );
}

const FORM_TYPE: &str = "multipart/form-data; boundary=XyZ";

fn form_data(parts: &[(&str, &[u8])]) -> Vec<u8> {
  let mut body = Vec::new();
  for (headers, data) in parts {
    body.extend_from_slice(b"--XyZ\r\n");
    body.extend_from_slice(headers.as_bytes());
    body.extend_from_slice(b"\r\n\r\n");
    body.extend_from_slice(data);
    body.extend_from_slice(b"\r\n");
  }
  body.extend_from_slice(b"--XyZ--\r\n");
  body
}

#[test]
fn each_part_is_decoded_by_its_own_type_through_the_registry() {
  let body = form_data(&[
    ("Content-Disposition: form-data; name=\"name\"", b"Mary"),
    (
      "Content-Disposition: form-data; name=\"order\"; filename=\"order.json\"\r\nContent-Type: application/json",
      br#"{"id": 1}"#,
    ),
    (
      "Content-Disposition: form-data; name=\"note\"\r\nContent-Type: application/xml",
      b"<note>hi</note>",
    ),
    (
      "Content-Disposition: form-data; name=\"query\"\r\nContent-Type: application/x-www-form-urlencoded",
      b"a=1&a=2",
    ),
    ("Content-Disposition: form-data; name=\"raw\"", &[0xff, 0xfe]),
    (
      "Content-Disposition: form-data; name=\"logo\"; filename=\"logo.png\"\r\nContent-Type: image/png",
      b"PNG",
    ),
    (
      "Content-Disposition: form-data; name=\"inner\"\r\nContent-Type: multipart/mixed; boundary=in",
      b"--in--",
    ),
  ]);
  let decoded = parse(&body, FORM_TYPE).expect("parses");
  let RuntimeValue::Object(names) = &decoded else {
    panic!("an object")
  };
  assert_eq!(
    names.keys().collect::<Vec<_>>(),
    vec!["name", "order", "note", "query", "raw", "logo", "inner"]
  );
  let body_of = |name: &str| {
    let RuntimeValue::Array(parts) = &names[name] else {
      panic!("a list of parts")
    };
    let RuntimeValue::Object(part) = &parts[0] else {
      panic!("a part")
    };
    part.get("body").cloned()
  };
  assert_eq!(body_of("name"), Some(RuntimeValue::String("Mary".to_string())));
  assert_eq!(body_of("order"), Some(document(json!({ "id": 1 }))));
  assert_eq!(
    body_of("note"),
    Some(document(json!({ "note": { "#text": "hi", "#trimmed": "hi" } })))
  );
  assert_eq!(body_of("query"), Some(document(json!({ "a": ["1", "2"] }))));
  assert_eq!(body_of("raw"), Some(RuntimeValue::Bytes(vec![0xff, 0xfe])));
  assert_eq!(body_of("logo"), Some(RuntimeValue::Bytes(b"PNG".to_vec())));
  assert_eq!(body_of("inner"), Some(RuntimeValue::Bytes(b"--in--".to_vec())));
}

#[test]
fn a_part_that_is_not_what_its_type_says_has_no_body() {
  let body = form_data(&[(
    "Content-Disposition: form-data; name=\"order\"\r\nContent-Type: application/json",
    b"{not json",
  )]);
  assert_eq!(
    parse(&body, FORM_TYPE),
    Ok(document(json!({
      "order": [{ "headers": {
        "content-disposition": "form-data; name=\"order\"",
        "content-type": "application/json"
      } }]
    })))
  );
}

#[test]
fn parts_with_one_name_are_together_in_body_order_and_the_file_name_is_kept() {
  let body = form_data(&[
    (
      "Content-Disposition: form-data; name=\"files\"; filename=\"a.txt\"",
      b"a",
    ),
    ("Content-Disposition: form-data; name=\"note\"", b"n"),
    (
      "Content-Disposition: form-data; name=\"files\"; filename=\"b.txt\"",
      b"b",
    ),
  ]);
  assert_eq!(
    split(&body, "XyZ")
      .expect("splits")
      .iter()
      .map(|part| part.name.as_str())
      .collect::<Vec<_>>(),
    vec!["files", "note", "files"]
  );
  let decoded = parse(&body, FORM_TYPE).expect("parses");
  let RuntimeValue::Object(names) = &decoded else {
    panic!("an object")
  };
  assert_eq!(names.keys().collect::<Vec<_>>(), vec!["files", "note"]);
  assert_eq!(
    names["files"],
    document(json!([
      { "headers": { "content-disposition": "form-data; name=\"files\"; filename=\"a.txt\"" }, "filename": "a.txt", "body": "a" },
      { "headers": { "content-disposition": "form-data; name=\"files\"; filename=\"b.txt\"" }, "filename": "b.txt", "body": "b" }
    ]))
  );
}

#[test]
fn text_after_the_closing_boundary_is_not_a_part() {
  let body = format!("{MIXED}!");
  assert_eq!(
    parse(body.as_bytes(), MIXED_TYPE),
    parse(MIXED.as_bytes(), MIXED_TYPE)
  );
}

#[test]
fn a_part_keeps_the_value_multer_keeps_of_a_repeated_header_its_last() {
  let body = form_data(&[(
    "Content-Disposition: form-data; name=\"a\"\r\nX-Tag: one\r\nX-Tag: two",
    b"x",
  )]);
  let parts = split(&body, "XyZ").expect("splits");
  assert_eq!(
    parts,
    vec![RawPart {
      name: "a".to_string(),
      filename: None,
      headers: [
        (
          "content-disposition".to_string(),
          "form-data; name=\"a\"".to_string()
        ),
        ("x-tag".to_string(), "two".to_string()),
      ]
      .into_iter()
      .collect(),
      data: b"x".to_vec(),
    }]
  );
}

#[rstest]
#[case("multipart/form-data")]
#[case("multipart/mixed; charset=utf-8")]
fn a_type_with_no_boundary_does_not_decode(#[case] content_type: &str) {
  let result = built_ins().decode(Decode {
    content_type: content_type.to_string(),
    value: octets(MIXED.as_bytes(), content_type),
    options: None,
    embedded: None,
  });
  assert!(result.is_err(), "{result:?}");
}

/// A declared type with no boundary decodes octets labelled with one at that boundary (ADR 0047): a
/// contract declares the type, and a reply names its own boundary.
#[test]
fn a_declared_type_with_no_boundary_reads_the_labels() {
  let decoded = |declared: &str, label: &str| {
    built_ins().decode(Decode {
      content_type: declared.to_string(),
      value: octets(MIXED.as_bytes(), label),
      options: None,
      embedded: None,
    })
  };
  assert_eq!(
    decoded("multipart/mixed", MIXED_TYPE).expect("decodes").document,
    decode(MIXED.as_bytes(), MIXED_TYPE)
  );
  // A declared boundary is the one used, and a label with none gives none.
  assert!(decoded("multipart/mixed; boundary=other", MIXED_TYPE).is_err());
  assert!(decoded("multipart/mixed", "multipart/mixed").is_err());
}

#[rstest]
#[case("multipart/form-data", true)]
#[case("multipart/mixed; boundary=x", true)]
#[case("Multipart/Related", true)]
#[case("application/json", false)]
#[case("text/plain", false)]
fn every_multipart_type_is_handled(#[case] content_type: &str, #[case] handled: bool) {
  assert_eq!(MultipartContent::new().handles(content_type), handled);
}

fn encode(document: &RuntimeValue, content_type: &str) -> SlotValue {
  built_ins()
    .encode(Encode {
      content_type: content_type.to_string(),
      document: document.clone(),
      options: None,
      shape: None,
      embedded: None,
    })
    .expect("encodes")
    .value
}

#[rstest]
#[case(MIXED.as_bytes().to_vec(), MIXED_TYPE)]
#[case(form_data(&[
  ("Content-Disposition: form-data; name=\"name\"", b"Mary"),
  ("Content-Disposition: form-data; name=\"order\"; filename=\"o.json\"\r\nContent-Type: application/json", br#"{"id":1,"lines":[{"sku":"a"}]}"#),
  ("Content-Disposition: form-data; name=\"note\"\r\nContent-Type: text/xml", b"<note a=\"1\">hi</note>"),
  ("Content-Disposition: form-data; name=\"q\"\r\nContent-Type: application/x-www-form-urlencoded", b"a=1"),
  ("Content-Disposition: form-data; name=\"logo\"; filename=\"l.png\"\r\nContent-Type: image/png\r\nContent-Transfer-Encoding: base64", b"iVBORw0KGgo="),
  ("Content-Disposition: form-data; name=\"name\"", &[0xff]),
]), FORM_TYPE)]
fn encode_writes_back_what_decode_reads(#[case] body: Vec<u8>, #[case] content_type: &str) {
  let decoded = decode(&body, content_type);
  let encoded = encode(&decoded, content_type);
  assert_eq!(encoded.content_type.as_deref(), Some(content_type));
  let bytes = BASE64
    .decode(encoded.content.as_str().expect("base64"))
    .expect("base64");
  assert_eq!(decode(&bytes, content_type), decoded);
}

#[test]
fn encode_names_a_boundary_when_the_type_has_none_and_writes_a_named_parts_disposition() {
  let document = document(json!({
    "name": [{ "body": "Mary" }],
    "order": [{ "headers": { "content-type": "application/json" }, "filename": "o.json", "body": { "id": 1 } }]
  }));
  let encoded = encode(&document, "multipart/form-data");
  let content_type = encoded.content_type.expect("labelled");
  assert_eq!(
    content_type,
    "multipart/form-data; boundary=janus-multipart-boundary"
  );
  let bytes = BASE64
    .decode(encoded.content.as_str().expect("base64"))
    .expect("base64");
  assert_eq!(
    String::from_utf8(bytes.clone()).expect("text"),
    "--janus-multipart-boundary\r\ncontent-disposition: form-data; name=\"name\"\r\n\r\nMary\r\n\
--janus-multipart-boundary\r\ncontent-disposition: form-data; name=\"order\"; filename=\"o.json\"\r\n\
content-type: application/json\r\n\r\n{\"id\":1}\r\n--janus-multipart-boundary--\r\n"
  );
  assert_eq!(
    parse(&bytes, &content_type),
    Ok(self::document(json!({
      "name": [{ "headers": { "content-disposition": "form-data; name=\"name\"" }, "body": "Mary" }],
      "order": [{
        "headers": {
          "content-disposition": "form-data; name=\"order\"; filename=\"o.json\"",
          "content-type": "application/json"
        },
        "filename": "o.json",
        "body": { "id": 1 }
      }]
    })))
  );
}

#[test]
fn a_boundary_a_part_holds_is_made_longer() {
  let document = document(json!({ "a": [{ "body": "--janus-multipart-boundary" }] }));
  let encoded = encode(&document, "multipart/mixed");
  assert_eq!(
    encoded.content_type.as_deref(),
    Some("multipart/mixed; boundary=janus-multipart-boundary-1")
  );
}

#[rstest]
#[case(json!({ "a": [{ "body": { "id": 1 } }] }), "a structured body needs a content type")]
#[case(json!({ "a": [{ "headers": { "content-type": "image/png" }, "body": { "id": 1 } }] }), "unsupported content type 'image/png'")]
#[case(json!([1]), "an object of part names")]
#[case(json!({ "a\"b": [{ "body": "x" }] }), "cannot be written in a Content-Disposition")]
fn a_document_that_is_not_a_multipart_body_does_not_encode(#[case] document: Value, #[case] message: &str) {
  let result = built_ins().encode(Encode {
    content_type: "multipart/form-data".to_string(),
    document: RuntimeValue::from_json(&document),
    options: None,
    shape: None,
    embedded: None,
  });
  let error = result.expect_err("refused");
  assert!(error.message.contains(message), "{}", error.message);
}

#[test]
fn a_document_a_contract_recorded_decodes_as_it_is() {
  let recorded = json!({ "a": [{ "body": "x" }] });
  let decoded = MultipartContent::new()
    .decode(Decode {
      content_type: FORM_TYPE.to_string(),
      value: SlotValue {
        content: recorded.clone(),
        encoded: None,
        content_type: None,
      },
      options: None,
      embedded: None,
    })
    .expect("decodes");
  assert_eq!(decoded.document, document(recorded));
}

/// What `label` answers for the member at `at` of a body declared `multipart/mixed` with no boundary,
/// labelled with [`MIXED_TYPE`].
fn label(at: &str) -> Option<String> {
  MultipartContent::new()
    .label(Label {
      content_type: "multipart/mixed".to_string(),
      value: octets(MIXED.as_bytes(), MIXED_TYPE),
      at: at.to_string(),
    })
    .expect("labels")
    .media_type
}

#[rstest]
#[case("//0/body", Some("text/plain"))]
#[case("//1/body", Some("application/octet-stream"))]
#[case("//2/body", None)]
#[case("//0", None)]
#[case("//0/headers", None)]
#[case("/missing/0/body", None)]
fn a_parts_body_is_labelled_by_its_own_content_type_and_nothing_else_is(
  #[case] at: &str,
  #[case] expected: Option<&str>,
) {
  assert_eq!(label(at).as_deref(), expected);
}

#[test]
fn a_part_with_no_type_labels_nothing() {
  let body = form_data(&[("Content-Disposition: form-data; name=\"note\"", b"hello")]);
  let labelled = MultipartContent::new()
    .label(Label {
      content_type: "multipart/form-data; boundary=XyZ".to_string(),
      value: octets(&body, "multipart/form-data; boundary=XyZ"),
      at: "/note/0/body".to_string(),
    })
    .expect("labels");
  assert_eq!(labelled.media_type, None);
}

/// Decodes `text/csv` into rows of fields, split at commas, and records what it was asked to encode.
/// It reports a degradation for every decode, as a CSV component that cannot tell numbers from text
/// would.
#[derive(Default)]
struct Csv {
  encoded: Mutex<Vec<Encode>>,
}

impl ContentComponent for Csv {
  fn handles(&self, content_type: &str) -> bool {
    content_type.starts_with("text/csv")
  }
  fn decode(&self, req: Decode) -> Result<DecodeResult, ComponentError> {
    let bytes = BASE64
      .decode(req.value.content.as_str().unwrap_or_default())
      .map_err(|err| ComponentError::decode_failed(&req.content_type, err.to_string()))?;
    let text = String::from_utf8(bytes)
      .map_err(|err| ComponentError::decode_failed(&req.content_type, err.to_string()))?;
    let rows: Vec<Value> = text
      .lines()
      .map(|line| Value::Array(line.split(',').map(|field| json!(field)).collect()))
      .collect();
    Ok(DecodeResult {
      document: RuntimeValue::from_json(&Value::Array(rows)),
      degradations: vec![json!({ "code": "numeric-lexical", "path": "$[0]", "message": "fields are text" })],
      embedded: Vec::new(),
    })
  }
  fn encode(&self, req: Encode) -> Result<EncodeResult, ComponentError> {
    let RuntimeValue::Array(rows) = &req.document else {
      return Err(ComponentError::encode_failed(&req.content_type, "rows"));
    };
    let text: Vec<String> = rows
      .iter()
      .map(|row| match row {
        RuntimeValue::Array(fields) => fields
          .iter()
          .map(|field| match field {
            RuntimeValue::String(field) => field.clone(),
            other => other.to_json().to_string(),
          })
          .collect::<Vec<_>>()
          .join(","),
        _ => String::new(),
      })
      .collect();
    self.encoded.lock().unwrap().push(req.clone());
    Ok(EncodeResult::of(SlotValue {
      content: json!(BASE64.encode(text.join("\n"))),
      encoded: Some("base64".to_string()),
      content_type: Some(req.content_type),
    }))
  }
  fn compile(&self, _req: Compile) -> Result<CompileResult, ComponentError> {
    Ok(CompileResult::default())
  }
  fn detect(&self, _req: Detect) -> Result<DetectResult, ComponentError> {
    Err(ComponentError::operation_unsupported("content/detect"))
  }
}

/// A body with a CSV part and a JSON part.
fn with_csv() -> Vec<u8> {
  form_data(&[
    (
      "Content-Disposition: form-data; name=\"rows\"; filename=\"r.csv\"\r\nContent-Type: text/csv",
      b"a,b\n1,2",
    ),
    (
      "Content-Disposition: form-data; name=\"order\"\r\nContent-Type: application/json",
      br#"{"id":1}"#,
    ),
  ])
}

/// ADR 0103: the first request is answered with the parts' bodies the component needs, one per typed
/// part, by the part's index; the repeated one, carrying the registry's answers, with the document.
#[test]
fn the_component_asks_for_each_typed_parts_body_and_answers_when_given_them() {
  let body = with_csv();
  let asked = MultipartContent::new()
    .decode(Decode::of(FORM_TYPE, octets(&body, FORM_TYPE)))
    .expect("asks");
  assert_eq!(
    asked
      .embedded
      .iter()
      .map(|embedded| (embedded.id.as_str(), embedded.content_type.as_str()))
      .collect::<Vec<_>>(),
    vec![("0", "text/csv"), ("1", "application/json")]
  );
  assert_eq!(
    BASE64
      .decode(asked.embedded[0].value.content.as_str().unwrap())
      .unwrap(),
    b"a,b\n1,2"
  );

  let answers = [
    (
      "0".to_string(),
      Ok(DecodeResult::of(RuntimeValue::String("rows".to_string()))),
    ),
    (
      "1".to_string(),
      Err(ComponentError::decode_failed("application/json", "no")),
    ),
  ]
  .into_iter()
  .collect();
  let answered = MultipartContent::new()
    .decode(Decode {
      embedded: Some(answers),
      ..Decode::of(FORM_TYPE, octets(&body, FORM_TYPE))
    })
    .expect("answers");
  assert!(answered.embedded.is_empty());
  assert_eq!(
    answered.document,
    document(json!({
      "rows": [{
        "headers": { "content-disposition": "form-data; name=\"rows\"; filename=\"r.csv\"", "content-type": "text/csv" },
        "filename": "r.csv",
        "body": "rows"
      }],
      "order": [{
        "headers": { "content-disposition": "form-data; name=\"order\"", "content-type": "application/json" }
      }]
    }))
  );
}

#[test]
fn a_body_with_no_typed_part_needs_nothing_asked() {
  let asked = MultipartContent::new()
    .decode(Decode::of(
      FORM_TYPE,
      octets(
        &form_data(&[("Content-Disposition: form-data; name=\"a\"", b"plain")]),
        FORM_TYPE,
      ),
    ))
    .expect("decodes");
  assert!(asked.embedded.is_empty(), "{:?}", asked.embedded);
}

/// ADR 0103: a part reaches the component the scope has for its type, a declared one ahead of the
/// built-ins, and its degradations are reported where the part's body is.
#[test]
fn a_part_reaches_a_component_registered_for_its_type() {
  let registry = ContentRegistry::new()
    .with(Arc::new(Csv::default()))
    .then(&built_ins());
  let decoded = registry
    .decode(Decode::of(FORM_TYPE, octets(&with_csv(), FORM_TYPE)))
    .expect("decodes");
  let RuntimeValue::Object(names) = &decoded.document else {
    panic!("an object")
  };
  assert_eq!(
    names["rows"],
    document(json!([{
      "headers": { "content-disposition": "form-data; name=\"rows\"; filename=\"r.csv\"", "content-type": "text/csv" },
      "filename": "r.csv",
      "body": [["a", "b"], ["1", "2"]]
    }]))
  );
  assert_eq!(
    names["order"],
    document(json!([{
      "headers": { "content-disposition": "form-data; name=\"order\"", "content-type": "application/json" },
      "body": { "id": 1 }
    }]))
  );
  assert_eq!(
    decoded.degradations,
    vec![json!({ "code": "numeric-lexical", "path": "$.rows[0].body[0]", "message": "fields are text" })]
  );

  // With no CSV component, the text component reads the part, as before.
  assert_eq!(
    parse(&with_csv(), FORM_TYPE).unwrap(),
    document(json!({
      "rows": [{
        "headers": { "content-disposition": "form-data; name=\"rows\"; filename=\"r.csv\"", "content-type": "text/csv" },
        "filename": "r.csv",
        "body": "a,b\n1,2"
      }],
      "order": [{
        "headers": { "content-disposition": "form-data; name=\"order\"", "content-type": "application/json" },
        "body": { "id": 1 }
      }]
    }))
  );
}

/// ADR 0103: a part's body is encoded by the component for its type, given the part of the slot's shape
/// that describes it.
#[test]
fn a_parts_body_is_encoded_by_the_component_for_its_type_with_its_own_shape() {
  let csv = Arc::new(Csv::default());
  let registry = ContentRegistry::new().with(csv.clone()).then(&built_ins());
  let rows_shape = json!({ "shape": "each-like", "items": { "shape": "array", "entries": [] }, "min": 0 });
  let shape = json!({
    "shape": "object",
    "members": {
      "rows": { "shape": "each-like", "min": 1, "items": {
        "shape": "object",
        "members": { "body": { "shape": "optional", "of": rows_shape.clone() } }
      } }
    }
  });
  let document = document(json!({
    "rows": [{ "headers": { "content-type": "text/csv" }, "filename": "r.csv", "body": [["a", "b"], ["1", "2"]] }],
    "order": [{ "headers": { "content-type": "application/json" }, "body": { "id": 1 } }]
  }));
  let encoded = registry
    .encode(Encode {
      shape: Some(shape),
      ..Encode::of(FORM_TYPE, document.clone())
    })
    .expect("encodes")
    .value;
  let bytes = BASE64.decode(encoded.content.as_str().unwrap()).unwrap();
  let text = String::from_utf8(bytes.clone()).unwrap();
  assert!(text.contains("\r\n\r\na,b\n1,2\r\n"), "{text}");
  assert!(text.contains("\r\n\r\n{\"id\":1}\r\n"), "{text}");
  let asked = csv.encoded.lock().unwrap();
  assert_eq!(asked.len(), 1);
  assert_eq!(
    asked[0].shape,
    Some(json!({ "shape": "optional", "of": rows_shape }))
  );

  let decoded = registry
    .decode(Decode::of(FORM_TYPE, octets(&bytes, FORM_TYPE)))
    .unwrap();
  assert_eq!(decoded.document, document_with_dispositions());
}

fn document_with_dispositions() -> RuntimeValue {
  document(json!({
    "rows": [{
      "headers": { "content-disposition": "form-data; name=\"rows\"; filename=\"r.csv\"", "content-type": "text/csv" },
      "filename": "r.csv",
      "body": [["a", "b"], ["1", "2"]]
    }],
    "order": [{
      "headers": { "content-disposition": "form-data; name=\"order\"", "content-type": "application/json" },
      "body": { "id": 1 }
    }]
  }))
}

/// A part of a type no component writes is written as its text, as before (ADR 0040 decision 2).
#[test]
fn a_text_body_of_a_type_nothing_writes_is_its_text() {
  let encoded = encode(
    &document(json!({ "a": [{ "headers": { "content-type": "application/x-custom" }, "body": "raw" }] })),
    FORM_TYPE,
  );
  let text = String::from_utf8(BASE64.decode(encoded.content.as_str().unwrap()).unwrap()).unwrap();
  assert!(text.contains("\r\n\r\nraw\r\n"), "{text}");
}
