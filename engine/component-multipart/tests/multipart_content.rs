//! The built-in multipart component (ADR 0040): what a body's document is, and that `encode` writes
//! back what `decode` reads.

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use pact_janus_component_multipart::{MultipartContent, RawPart, parse, split};
use pact_janus_kernel::component::{ContentComponent, Decode, Encode, SlotValue};
use pact_janus_kernel::plan::RuntimeValue;
use pretty_assertions::assert_eq;
use rstest::rstest;
use serde_json::{Value, json};

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
  MultipartContent::new()
    .decode(Decode {
      content_type: content_type.to_string(),
      value: octets(body, content_type),
      options: None,
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
fn each_part_is_decoded_by_its_own_type_through_the_built_in_components() {
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
    Some(document(json!({ "note": { "#text": "hi" } })))
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
  let result = MultipartContent::new().decode(Decode {
    content_type: content_type.to_string(),
    value: octets(MIXED.as_bytes(), content_type),
    options: None,
  });
  assert!(result.is_err(), "{result:?}");
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
  MultipartContent::new()
    .encode(Encode {
      content_type: content_type.to_string(),
      document: document.clone(),
      options: None,
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
#[case(json!({ "a": [{ "headers": { "content-type": "image/png" }, "body": { "id": 1 } }] }), "a structured body needs a content type")]
#[case(json!([1]), "an object of part names")]
#[case(json!({ "a\"b": [{ "body": "x" }] }), "cannot be written in a Content-Disposition")]
fn a_document_that_is_not_a_multipart_body_does_not_encode(#[case] document: Value, #[case] message: &str) {
  let result = MultipartContent::new().encode(Encode {
    content_type: "multipart/form-data".to_string(),
    document: RuntimeValue::from_json(&document),
    options: None,
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
    })
    .expect("decodes");
  assert_eq!(decoded.document, document(recorded));
}
