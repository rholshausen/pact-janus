//! Drives the component frame by frame, as the engine does, through the same `call` the WASM binding
//! exports.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use janus_ndjson::Instance;
use pretty_assertions::assert_eq;
use serde_json::{Value, json};

const HELLO: &str = r#"{"type":"request","id":"1","op":"component/hello","body":{"component-protocol-versions":[1],"engine":{"name":"janus-engine","version":"0.8.0"},"grants":{"env":[],"fs":[],"network":false},"capabilities":{"embedded-content":{}}}}"#;

/// An instance that has been greeted.
fn greeted() -> Instance {
  let mut instance = Instance::new();
  let response = send_raw(&mut instance, HELLO.as_bytes());
  assert!(response.get("ok").is_some(), "{response}");
  instance
}

fn send_raw(instance: &mut Instance, request: &[u8]) -> Value {
  let response = instance.call(request);
  serde_json::from_slice(&response).expect("every response is a JSON frame")
}

fn send(instance: &mut Instance, id: &str, op: &str, body: Value) -> Value {
  let request = json!({ "type": "request", "id": id, "op": op, "body": body });
  let response = send_raw(instance, request.to_string().as_bytes());
  assert_eq!(response["type"], "response");
  assert_eq!(response["id"], id, "the response echoes the request's id");
  assert!(
    response.get("ok").is_some() != response.get("error").is_some(),
    "a response carries ok or error, never both: {response}"
  );
  response
}

fn ok(response: Value) -> Value {
  response
    .get("ok")
    .cloned()
    .unwrap_or_else(|| panic!("expected ok, got {response}"))
}

fn err(response: Value) -> Value {
  response
    .get("error")
    .cloned()
    .unwrap_or_else(|| panic!("expected an error, got {response}"))
}

fn decode_octets(instance: &mut Instance, octets: &[u8]) -> Value {
  send(
    instance,
    "d",
    "content/decode",
    json!({ "content-type": "application/x-ndjson",
            "value": { "content": BASE64.encode(octets), "encoded": "base64" } }),
  )
}

fn encode_document(instance: &mut Instance, document: Value) -> Value {
  send(
    instance,
    "e",
    "content/encode",
    json!({ "content-type": "application/x-ndjson", "document": document }),
  )
}

fn encoded_octets(result: &Value) -> Vec<u8> {
  assert_eq!(result["value"]["encoded"], "base64");
  BASE64
    .decode(result["value"]["content"].as_str().unwrap())
    .unwrap()
}

// ---------------------------------------------------------------------------------------------
// The handshake and the pipe

#[test]
fn the_handshake_says_who_the_component_is_and_what_it_handles() {
  let mut instance = Instance::new();
  let response = send_raw(&mut instance, HELLO.as_bytes());
  assert_eq!(
    response,
    json!({ "type": "response", "id": "1", "ok": {
      "component-protocol-version": 1,
      "component": { "name": "ndjson", "version": "1.0.0" },
      "interfaces": ["content"],
      "contributes": { "content-types": [ { "media-type": "application/x-ndjson" } ] },
      "capabilities": {} } })
  );
}

#[test]
fn anything_before_the_handshake_is_handshake_required() {
  let mut instance = Instance::new();
  let error = err(send(
    &mut instance,
    "7",
    "content/decode",
    json!({ "content-type": "application/x-ndjson", "value": { "content": "", "encoded": "base64" } }),
  ));
  assert_eq!(error["code"], "handshake-required");
  assert_eq!(error["category"], "protocol");
  // And the free function, which answers each frame as a fresh instance's first, says the same.
  let response: Value = serde_json::from_slice(&janus_ndjson::call(
    br#"{"type":"request","id":"2","op":"content/compile","body":{}}"#,
  ))
  .unwrap();
  assert_eq!(response["error"]["code"], "handshake-required");
}

#[test]
fn an_engine_offering_no_version_it_speaks_is_refused() {
  let mut instance = Instance::new();
  let error = err(send(
    &mut instance,
    "1",
    "component/hello",
    json!({ "component-protocol-versions": [2, 3] }),
  ));
  assert_eq!(error["code"], "protocol-version-unsupported");
  assert_eq!(error["details"]["supported"], json!([1]));
  // The refused handshake did not greet the instance.
  let error = err(send(&mut instance, "2", "content/compile", json!({})));
  assert_eq!(error["code"], "handshake-required");
}

#[test]
fn a_frame_that_cannot_be_parsed_is_malformed_with_an_empty_id() {
  let mut instance = greeted();
  for request in [
    &b"not json"[..],
    b"[1,2]",
    br#"{"type":"request","op":"content/decode","body":{}}"#,
  ] {
    let response = send_raw(&mut instance, request);
    assert_eq!(response["id"], "", "{response}");
    assert_eq!(response["error"]["code"], "malformed-frame");
  }
}

#[test]
fn an_operation_it_does_not_implement_is_unsupported_naming_it() {
  let mut instance = greeted();
  for op in [
    "content/label",
    "matcher/apply",
    "transport/start",
    "content/frobnicate",
  ] {
    let error = err(send(&mut instance, "9", op, json!({})));
    assert_eq!(error["code"], "operation-unsupported");
    assert_eq!(error["details"]["op"], op);
  }
}

#[test]
fn shutdown_is_answered() {
  let mut instance = greeted();
  assert_eq!(
    ok(send(&mut instance, "s", "component/shutdown", json!({}))),
    json!({})
  );
}

// ---------------------------------------------------------------------------------------------
// Decode

#[test]
fn a_body_decodes_to_an_array_of_its_records_in_order() {
  let mut instance = greeted();
  let result = ok(decode_octets(
    &mut instance,
    b"{\"id\":1,\"name\":\"a\"}\n{\"id\":2,\"name\":\"b\"}\n",
  ));
  assert_eq!(
    result,
    json!({ "document": [ { "id": 1, "name": "a" }, { "id": 2, "name": "b" } ] })
  );
}

#[test]
fn members_keep_the_order_the_octets_have_them() {
  let mut instance = greeted();
  let result = ok(decode_octets(&mut instance, b"{\"z\":1,\"a\":2,\"m\":3}\n"));
  let members: Vec<&String> = result["document"][0].as_object().unwrap().keys().collect();
  assert_eq!(members, ["z", "a", "m"]);
}

#[test]
fn a_record_may_be_any_json_value() {
  let mut instance = greeted();
  let result = ok(decode_octets(
    &mut instance,
    b"1\n\"two\"\n[3]\nnull\ntrue\n4.0\n",
  ));
  assert_eq!(result["document"], json!([1, "two", [3], null, true, 4.0]));
  // How a number was written is kept: 4.0 is a decimal, not the integer 4.
  assert!(result["document"][5].is_f64());
}

#[test]
fn the_trailing_newline_is_optional() {
  let mut instance = greeted();
  let with = ok(decode_octets(&mut instance, b"{\"a\":1}\n{\"a\":2}\n"));
  let without = ok(decode_octets(&mut instance, b"{\"a\":1}\n{\"a\":2}"));
  assert_eq!(with, without);
}

#[test]
fn crlf_line_endings_decode_as_lf_ones() {
  let mut instance = greeted();
  let crlf = ok(decode_octets(&mut instance, b"{\"a\":1}\r\n{\"a\":2}\r\n"));
  let mixed = ok(decode_octets(&mut instance, b"{\"a\":1}\r\n{\"a\":2}\n"));
  let lf = ok(decode_octets(&mut instance, b"{\"a\":1}\n{\"a\":2}\n"));
  assert_eq!(crlf, lf);
  assert_eq!(mixed, lf);
}

#[test]
fn blank_lines_are_ignored() {
  let mut instance = greeted();
  let result = ok(decode_octets(
    &mut instance,
    b"\n{\"a\":1}\n\n  \t\n\r\n{\"a\":2}\n\n",
  ));
  assert_eq!(result["document"], json!([ { "a": 1 }, { "a": 2 } ]));
}

#[test]
fn empty_octets_are_zero_records() {
  let mut instance = greeted();
  assert_eq!(ok(decode_octets(&mut instance, b"")), json!({ "document": [] }));
  assert_eq!(
    ok(decode_octets(&mut instance, b"\n\n")),
    json!({ "document": [] })
  );
  // An absent content is an empty slot.
  let result = ok(send(
    &mut instance,
    "d",
    "content/decode",
    json!({ "content-type": "application/x-ndjson", "value": { "encoded": "json" } }),
  ));
  assert_eq!(result, json!({ "document": [] }));
}

#[test]
fn a_text_slot_value_is_decoded_as_its_utf8_octets() {
  let mut instance = greeted();
  let result = ok(send(
    &mut instance,
    "d",
    "content/decode",
    json!({ "content-type": "application/x-ndjson; charset=utf-8",
            "value": { "content": "{\"name\":\"café\"}\n", "encoded": "text" } }),
  ));
  assert_eq!(result["document"], json!([ { "name": "café" } ]));
}

#[test]
fn a_json_slot_value_is_already_the_document() {
  let mut instance = greeted();
  for value in [
    json!({ "content": [ { "id": 1 } ], "encoded": "json", "content-type": "application/x-ndjson" }),
    json!({ "content": [ { "id": 1 } ] }),
  ] {
    let result = ok(send(
      &mut instance,
      "d",
      "content/decode",
      json!({ "content-type": "application/x-ndjson", "value": value }),
    ));
    assert_eq!(result, json!({ "document": [ { "id": 1 } ] }));
  }
}

#[test]
fn a_line_that_is_not_json_fails_naming_the_line() {
  let mut instance = greeted();
  let error = err(decode_octets(
    &mut instance,
    b"{\"a\":1}\n\n{\"a\":2,}\n{\"a\":3}\n",
  ));
  assert_eq!(error["code"], "decode-failed");
  assert_eq!(error["category"], "document");
  let message = error["message"].as_str().unwrap();
  assert!(message.starts_with("line 3 is not a JSON value: "), "{message}");
  assert_eq!(error["details"]["content-type"], "application/x-ndjson");
  assert_eq!(error["details"]["line"], 3);
  // The record it would have been is the second.
  assert_eq!(error["details"]["path"], "$[1]");
}

#[test]
fn a_line_holding_two_values_fails() {
  let mut instance = greeted();
  let error = err(decode_octets(&mut instance, b"{\"a\":1} {\"a\":2}\n"));
  assert_eq!(error["code"], "decode-failed");
  assert!(
    error["message"].as_str().unwrap().starts_with("line 1 "),
    "{error}"
  );
}

#[test]
fn pretty_printed_json_is_not_ndjson() {
  let mut instance = greeted();
  let error = err(decode_octets(&mut instance, b"{\n  \"a\": 1\n}\n"));
  assert_eq!(error["code"], "decode-failed");
  assert_eq!(error["details"]["line"], 1);
}

#[test]
fn a_line_that_is_not_utf8_fails_naming_the_line() {
  let mut instance = greeted();
  let error = err(decode_octets(&mut instance, b"{\"a\":1}\n\"\xff\"\n"));
  assert_eq!(error["code"], "decode-failed");
  assert!(
    error["message"]
      .as_str()
      .unwrap()
      .starts_with("line 2 is not UTF-8"),
    "{error}"
  );
}

#[test]
fn a_slot_value_that_is_not_base64_fails() {
  let mut instance = greeted();
  let error = err(send(
    &mut instance,
    "d",
    "content/decode",
    json!({ "content-type": "application/x-ndjson", "value": { "content": "%%%", "encoded": "base64" } }),
  ));
  assert_eq!(error["code"], "decode-failed");
}

#[test]
fn another_content_type_is_unsupported() {
  let mut instance = greeted();
  let error = err(send(
    &mut instance,
    "d",
    "content/decode",
    json!({ "content-type": "application/json", "value": { "content": "e30=", "encoded": "base64" } }),
  ));
  assert_eq!(error["code"], "unsupported-content-type");
}

// ---------------------------------------------------------------------------------------------
// Encode

#[test]
fn a_document_encodes_one_compact_record_per_line() {
  let mut instance = greeted();
  let result = ok(encode_document(
    &mut instance,
    json!([ { "id": 1, "name": "a" }, { "id": 2, "name": "b" } ]),
  ));
  assert_eq!(
    String::from_utf8(encoded_octets(&result)).unwrap(),
    "{\"id\":1,\"name\":\"a\"}\n{\"id\":2,\"name\":\"b\"}\n"
  );
  assert_eq!(result["value"]["content-type"], "application/x-ndjson");
}

#[test]
fn encode_writes_members_in_the_order_it_is_handed_them() {
  let mut instance = greeted();
  let request = br#"{"type":"request","id":"e","op":"content/encode","body":{"content-type":"application/x-ndjson","document":[{"z":1,"a":2}]}}"#;
  let result = ok(send_raw(&mut instance, request));
  assert_eq!(encoded_octets(&result), b"{\"z\":1,\"a\":2}\n");
}

#[test]
fn the_empty_array_is_no_octets() {
  let mut instance = greeted();
  let result = ok(encode_document(&mut instance, json!([])));
  assert_eq!(encoded_octets(&result), b"");
}

#[test]
fn encode_works_with_a_shape_and_without_one() {
  let mut instance = greeted();
  let result = ok(send(
    &mut instance,
    "e",
    "content/encode",
    json!({ "content-type": "application/x-ndjson", "document": [ { "id": 1 } ],
            "shape": { "shape": "each-like", "min": 1,
                       "items": { "shape": "object", "members": { "id": { "shape": "integer", "example": 1 } } } } }),
  ));
  assert_eq!(encoded_octets(&result), b"{\"id\":1}\n");
}

#[test]
fn a_document_that_is_not_an_array_is_refused() {
  let mut instance = greeted();
  for document in [json!({ "id": 1 }), json!("x"), json!(1), json!(null)] {
    let error = err(encode_document(&mut instance, document));
    assert_eq!(error["code"], "encode-failed");
    assert_eq!(error["category"], "document");
  }
  let error = err(send(
    &mut instance,
    "e",
    "content/encode",
    json!({ "content-type": "application/x-ndjson", "document": "AAEC", "encoded": "base64" }),
  ));
  assert_eq!(error["code"], "encode-failed");
}

#[test]
fn decode_of_encode_is_the_document() {
  let mut instance = greeted();
  let bodies: [&[u8]; 6] = [
    b"{\"id\":1,\"name\":\"a\"}\n{\"id\":2,\"name\":\"b\"}\n",
    b"\r\n{\"text\":\"line\\nbreak \\u2028 \\\"quoted\\\"\"}\r\n\r\n[1,2.5,-0.0,1e300,18446744073709551615,-9223372036854775808]",
    b"null\ntrue\nfalse\n\"\"\n{}\n[]\n",
    b"{\"nested\":{\"deep\":[{\"x\":[null,{\"y\":1.0}]}]},\"z\":\"\xc3\xa9\xf0\x9f\x98\x80\"}",
    b"",
    b"123456789012345678901234567890\n0.1\n100.0\n1E2\n",
  ];
  for body in bodies {
    let decoded = ok(decode_octets(&mut instance, body))["document"].clone();
    let encoded = encoded_octets(&ok(encode_document(&mut instance, decoded.clone())));
    let again = ok(decode_octets(&mut instance, &encoded))["document"].clone();
    assert_eq!(again, decoded, "body {:?}", String::from_utf8_lossy(body));
    // And byte for byte: a decode and an encode are idempotent from then on.
    let encoded_again = encoded_octets(&ok(encode_document(&mut instance, again)));
    assert_eq!(encoded_again, encoded);
  }
}

// ---------------------------------------------------------------------------------------------
// Compile

#[test]
fn compile_accepts_a_shape_that_admits_an_array() {
  let mut instance = greeted();
  for shape in [
    json!({ "shape": "each-like", "min": 1, "items": { "shape": "object", "members": {} } }),
    json!({ "shape": "array", "entries": [] }),
    json!({ "shape": "contains", "entries": [] }),
    json!({ "shape": "any" }),
    json!({ "shape": "not-empty" }),
    json!({ "shape": "type", "example": [] }),
    json!({ "shape": "equality", "example": [ { "id": 1 } ] }),
    json!({ "shape": "optional", "of": { "shape": "each-like", "items": { "shape": "integer" } } }),
    json!({ "shape": "forbidden" }),
    json!({ "shape": "acme:records" }),
  ] {
    let result = ok(send(
      &mut instance,
      "c",
      "content/compile",
      json!({ "content-type": "application/x-ndjson", "shape": shape, "path": "$.response.body" }),
    ));
    assert_eq!(result, json!({}), "{shape}");
  }
}

#[test]
fn compile_refuses_a_shape_that_admits_no_array() {
  let mut instance = greeted();
  for shape in [
    json!({ "shape": "object", "members": {} }),
    json!({ "shape": "string" }),
    json!({ "shape": "integer" }),
    json!({ "shape": "equality", "example": { "id": 1 } }),
    json!({ "shape": "nullable", "of": { "shape": "object", "members": {} } }),
    json!({ "shape": "any-of", "options": ["a", "b"], "example": "a" }),
  ] {
    let error = err(send(
      &mut instance,
      "c",
      "content/compile",
      json!({ "content-type": "application/x-ndjson", "shape": shape, "path": "$.response.body" }),
    ));
    assert_eq!(error["code"], "encode-failed", "{shape}");
    assert!(
      error["message"].as_str().unwrap().contains("$.response.body"),
      "{error}"
    );
  }
}

// ---------------------------------------------------------------------------------------------
// Detect

#[test]
fn detect_claims_several_records_and_nothing_else() {
  let mut instance = greeted();
  let detect = |instance: &mut Instance, octets: &[u8]| {
    ok(send(
      instance,
      "t",
      "content/detect",
      json!({ "value": { "content": BASE64.encode(octets), "encoded": "base64" } }),
    ))
  };
  assert_eq!(
    detect(&mut instance, b"{\"a\":1}\n{\"a\":2}\n"),
    json!({ "media-type": "application/x-ndjson", "confidence": 0.6 })
  );
  // One JSON document, even over several lines or alone on one, is JSON's to claim.
  assert_eq!(detect(&mut instance, b"{\"a\":1}\n"), json!({}));
  assert_eq!(detect(&mut instance, b"[1,\n2]\n"), json!({}));
  assert_eq!(detect(&mut instance, b"id,name\n1,a\n"), json!({}));
  assert_eq!(detect(&mut instance, b""), json!({}));
}
