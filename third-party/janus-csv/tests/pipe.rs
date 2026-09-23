//! The component driven through its pipe, frame in and frame out, exactly as an engine would drive it.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use janus_csv::Component;
use serde_json::{Value, json};

fn call(component: &mut Component, op: &str, body: Value) -> Value {
  let frame = json!({ "type": "request", "id": "1", "op": op, "body": body });
  serde_json::from_slice(&component.call(&serde_json::to_vec(&frame).unwrap())).unwrap()
}

fn greeted() -> Component {
  let mut component = Component::default();
  let hello = call(
    &mut component,
    "component/hello",
    json!({ "component-protocol-versions": [1] }),
  );
  assert!(hello.get("ok").is_some(), "{hello}");
  component
}

fn slot(text: &str) -> Value {
  json!({ "content": BASE64.encode(text), "encoded": "base64", "content-type": "text/csv" })
}

#[test]
fn hello_declares_the_content_type_and_its_degradations() {
  let mut component = Component::default();
  let hello = call(
    &mut component,
    "component/hello",
    json!({ "component-protocol-versions": [2, 1] }),
  );
  let ok = &hello["ok"];
  assert_eq!(ok["component-protocol-version"], 1);
  assert_eq!(ok["component"]["name"], "csv");
  assert_eq!(ok["interfaces"], json!(["content", "matcher"]));
  assert_eq!(
    ok["contributes"]["actions"],
    json!([ { "name": "csv:integer" }, { "name": "csv:number" }, { "name": "csv:boolean" } ])
  );
  let content_types = ok["contributes"]["content-types"].as_array().unwrap();
  assert_eq!(content_types[0]["media-type"], "text/csv");
  let codes: Vec<&str> = content_types[0]["degradations"]
    .as_array()
    .unwrap()
    .iter()
    .map(|d| d["code"].as_str().unwrap())
    .collect();
  assert_eq!(codes, vec!["string-only", "no-null"]);
}

#[test]
fn hello_refuses_a_protocol_it_does_not_speak() {
  let mut component = Component::default();
  let hello = call(
    &mut component,
    "component/hello",
    json!({ "component-protocol-versions": [2] }),
  );
  assert_eq!(hello["error"]["code"], "protocol-version-unsupported");
  assert_eq!(hello["error"]["details"]["supported"], json!([1]));
}

#[test]
fn every_operation_before_hello_is_refused() {
  let mut component = Component::default();
  let decode = call(
    &mut component,
    "content/decode",
    json!({ "content-type": "text/csv", "value": slot("a\n1\n") }),
  );
  assert_eq!(decode["error"]["code"], "handshake-required");
}

#[test]
fn an_unknown_operation_is_named() {
  let mut component = greeted();
  // An op of an interface it declared (matcher) but does not implement: spec §3.1 says named.
  let result = call(&mut component, "matcher/compare", json!({}));
  assert_eq!(result["error"]["code"], "operation-unsupported");
  assert_eq!(result["error"]["details"]["op"], "matcher/compare");
  let detect = call(
    &mut component,
    "content/detect",
    json!({ "value": slot("a,b\n") }),
  );
  assert_eq!(detect["error"]["code"], "operation-unsupported");
}

#[test]
fn a_malformed_frame_is_answered_in_band() {
  let mut component = greeted();
  let result: Value = serde_json::from_slice(&component.call(b"not json")).unwrap();
  assert_eq!(result["id"], "");
  assert_eq!(result["error"]["code"], "malformed-frame");
}

#[test]
fn decode_reads_a_header_row_into_named_columns_of_strings() {
  let mut component = greeted();
  let result = call(
    &mut component,
    "content/decode",
    json!({ "content-type": "text/csv", "value": slot("id,status,total\r\no-1,shipped,12.50\r\no-2,new,0.00\r\n") }),
  );
  let ok = &result["ok"];
  assert_eq!(
    ok["document"],
    json!([ { "id": "o-1", "status": "shipped", "total": "12.50" }, { "id": "o-2", "status": "new", "total": "0.00" } ])
  );
  let degradations = ok["degradations"].as_array().unwrap();
  assert_eq!(degradations.len(), 1);
  assert_eq!(degradations[0]["code"], "numeric-lexical");
  assert_eq!(degradations[0]["path"], "$[*].total");
}

#[test]
fn decode_keeps_the_header_order() {
  let mut component = greeted();
  let result = call(
    &mut component,
    "content/decode",
    json!({ "content-type": "text/csv", "value": slot("z,a\n1,2\n") }),
  );
  let keys: Vec<&String> = result["ok"]["document"][0].as_object().unwrap().keys().collect();
  assert_eq!(keys, vec!["z", "a"]);
}

#[test]
fn decode_without_a_header_reads_arrays() {
  let mut component = greeted();
  let by_parameter = call(
    &mut component,
    "content/decode",
    json!({ "content-type": "text/csv; header=absent", "value": slot("a,b\nc,d\n") }),
  );
  assert_eq!(by_parameter["ok"]["document"], json!([["a", "b"], ["c", "d"]]));
  let by_option = call(
    &mut component,
    "content/decode",
    json!({ "content-type": "text/csv", "value": slot("a,b\n"), "options": { "header": "absent" } }),
  );
  assert_eq!(by_option["ok"]["document"], json!([["a", "b"]]));
}

#[test]
fn decode_reports_empty_fields_where_they_are() {
  let mut component = greeted();
  let result = call(
    &mut component,
    "content/decode",
    json!({ "content-type": "text/csv", "value": slot("id,note\n1,\n2,hi\n") }),
  );
  let degradations = result["ok"]["degradations"].as_array().unwrap();
  assert!(
    degradations
      .iter()
      .any(|d| d["code"] == "no-null" && d["path"] == "$[*].note"),
    "{degradations:?}"
  );
}

#[test]
fn decode_accepts_text_slots() {
  let mut component = greeted();
  let result = call(
    &mut component,
    "content/decode",
    json!({ "content-type": "text/csv", "value": { "content": "a\nx\n", "encoded": "text" } }),
  );
  assert_eq!(result["ok"]["document"], json!([{ "a": "x" }]));
}

#[test]
fn decode_fails_rather_than_guessing() {
  let mut component = greeted();
  let ragged = call(
    &mut component,
    "content/decode",
    json!({ "content-type": "text/csv", "value": slot("a,b\n1,2\n3\n") }),
  );
  assert_eq!(ragged["error"]["code"], "decode-failed");
  assert_eq!(ragged["error"]["details"]["content-type"], "text/csv");
  assert_eq!(ragged["error"]["details"]["line"], 3);

  let duplicate = call(
    &mut component,
    "content/decode",
    json!({ "content-type": "text/csv", "value": slot("a,a\n1,2\n") }),
  );
  assert_eq!(duplicate["error"]["code"], "decode-failed");

  let empty = call(
    &mut component,
    "content/decode",
    json!({ "content-type": "text/csv", "value": slot("") }),
  );
  assert_eq!(empty["error"]["code"], "decode-failed");
}

#[test]
fn a_content_type_it_does_not_handle_is_refused() {
  let mut component = greeted();
  let json_body = call(
    &mut component,
    "content/decode",
    json!({ "content-type": "application/json", "value": slot("{}") }),
  );
  assert_eq!(json_body["error"]["code"], "unsupported-content-type");
  let latin1 = call(
    &mut component,
    "content/decode",
    json!({ "content-type": "text/csv; charset=iso-8859-1", "value": slot("a\n") }),
  );
  assert_eq!(latin1["error"]["code"], "unsupported-content-type");
  let utf8 = call(
    &mut component,
    "content/decode",
    json!({ "content-type": "text/csv; charset=utf-8", "value": slot("a\nx\n") }),
  );
  assert!(utf8.get("ok").is_some(), "{utf8}");
}

#[test]
fn encode_inverts_decode() {
  let mut component = greeted();
  let document = json!([ { "id": "o-1", "note": "a, \"quoted\" note", "total": "12.50" }, { "id": "o-2", "note": "", "total": "0.00" } ]);
  let encoded = call(
    &mut component,
    "content/encode",
    json!({ "content-type": "text/csv", "document": document }),
  );
  let value = &encoded["ok"]["value"];
  assert_eq!(value["encoded"], "base64");
  assert_eq!(value["content-type"], "text/csv");
  let text = String::from_utf8(BASE64.decode(value["content"].as_str().unwrap()).unwrap()).unwrap();
  assert_eq!(
    text,
    "id,note,total\r\no-1,\"a, \"\"quoted\"\" note\",12.50\r\no-2,,0.00\r\n"
  );

  let decoded = call(
    &mut component,
    "content/decode",
    json!({ "content-type": "text/csv", "value": value }),
  );
  assert_eq!(decoded["ok"]["document"], document);
}

#[test]
fn encode_spells_typed_values_as_text() {
  let mut component = greeted();
  let encoded = call(
    &mut component,
    "content/encode",
    json!({ "content-type": "text/csv", "document": [ { "n": 42, "b": true, "z": null } ] }),
  );
  let text = String::from_utf8(
    BASE64
      .decode(encoded["ok"]["value"]["content"].as_str().unwrap())
      .unwrap(),
  )
  .unwrap();
  assert_eq!(text, "n,b,z\r\n42,true,\r\n");
}

#[test]
fn encode_refuses_what_csv_cannot_hold() {
  let mut component = greeted();
  let nested = call(
    &mut component,
    "content/encode",
    json!({ "content-type": "text/csv", "document": [ { "a": { "b": 1 } } ] }),
  );
  assert_eq!(nested["error"]["code"], "encode-failed");
  assert_eq!(nested["error"]["details"]["path"], "$[0].a");
  let not_rows = call(
    &mut component,
    "content/encode",
    json!({ "content-type": "text/csv", "document": { "a": 1 } }),
  );
  assert_eq!(not_rows["error"]["code"], "encode-failed");
}

fn orders() -> Value {
  json!({ "shape": "each-like", "min": 1, "items": { "shape": "object", "members": {
    "id": { "shape": "string", "example": "66" },
    "items": { "shape": "integer", "example": 1 },
    "paid": { "shape": "boolean", "example": true } } } })
}

fn greeted_reading(grammars: Value) -> Component {
  let mut component = Component::default();
  call(
    &mut component,
    "component/hello",
    json!({ "component-protocol-versions": [1], "plan-grammar-versions": grammars }),
  );
  component
}

fn compile(component: &mut Component, shape: Value) -> Value {
  call(
    component,
    "content/compile",
    json!({ "content-type": "text/csv", "shape": shape, "path": "$.response.body" }),
  )
}

#[test]
fn compile_contributes_a_fragment_that_reads_typed_operators_as_text() {
  let mut component = greeted_reading(json!(["v0"]));
  let result = compile(&mut component, orders());
  assert_eq!(result["ok"]["grammar-version"], "v0");
  let text = result["ok"]["fragment"].to_string();
  for expected in [
    r#""name":"match:string""#,
    r#""name":"csv:integer""#,
    r#""name":"csv:boolean""#,
    r#""path":"~>.items""#,
    r#""label":"$.response.body[*].paid""#,
  ] {
    assert!(text.contains(expected), "{expected} in {text}");
  }
  assert!(!text.contains("match:integer"), "{text}");
}

#[test]
fn compile_declines_what_it_does_not_cover_and_what_the_engine_does_not_read() {
  let mut component = greeted_reading(json!(["v0"]));
  let optional_member = json!({ "shape": "each-like", "items": { "shape": "object", "members": {
    "note": { "shape": "optional", "of": { "shape": "string" } } } } });
  assert_eq!(compile(&mut component, optional_member)["ok"], json!({}));
  assert_eq!(
    compile(&mut component, json!({ "shape": "string" }))["ok"],
    json!({})
  );

  // An engine that says nothing about grammars, or reads only one this component does not write,
  // gets the generic plan: correct for every operator but the typed ones, and never a skew.
  for grammars in [Value::Null, json!(["v1"])] {
    let mut component = greeted_reading(grammars);
    assert_eq!(compile(&mut component, orders())["ok"], json!({}));
  }
  let mut silent = greeted();
  assert_eq!(compile(&mut silent, orders())["ok"], json!({}));
}

#[test]
fn apply_reads_text_as_the_type_it_spells() {
  let mut component = greeted();
  let apply = |component: &mut Component, action: &str, value: Value| {
    call(
      component,
      "matcher/apply",
      json!({ "action": action, "values": [ { "content": value } ] }),
    )["ok"]["results"][0]["status"]
      .clone()
  };
  assert_eq!(apply(&mut component, "csv:integer", json!("12")), "ok");
  assert_eq!(apply(&mut component, "csv:integer", json!("-3")), "ok");
  assert_eq!(apply(&mut component, "csv:integer", json!("1.5")), "error");
  assert_eq!(apply(&mut component, "csv:integer", json!("")), "error");
  assert_eq!(apply(&mut component, "csv:number", json!("12.50")), "ok");
  assert_eq!(apply(&mut component, "csv:number", json!("twelve")), "error");
  assert_eq!(apply(&mut component, "csv:number", json!("NaN")), "error");
  assert_eq!(apply(&mut component, "csv:boolean", json!("true")), "ok");
  assert_eq!(apply(&mut component, "csv:boolean", json!("yes")), "error");
  let unknown = call(
    &mut component,
    "matcher/apply",
    json!({ "action": "csv:date", "values": [ { "content": "x" } ] }),
  );
  assert_eq!(unknown["error"]["code"], "unknown-action");
}
