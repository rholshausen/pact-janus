//! ADR 0036 at the protocol: the built-in `text` component claims every `text/` type, but only for
//! a body it can write as one string. A `text/csv` body of rows still needs a CSV component, and is
//! refused as `component-unavailable` when the interaction arrives, as it was before text existed.

use pact_janus_component_http::HttpTransport;
use pact_janus_component_json::JsonContent;
use pact_janus_component_text::TextContent;
use pact_janus_kernel::component::TransportComponent;
use pact_janus_kernel::protocol::Engine;
use pretty_assertions::assert_eq;
use rstest::rstest;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;

fn send(engine: &mut Engine, op: &str, body: Value) -> Value {
  let request = json!({ "type": "request", "id": "1", "op": op, "body": body });
  serde_json::from_slice(&engine.dispatch(&serde_json::to_vec(&request).unwrap())).unwrap()
}

fn engine() -> Engine {
  let mut transports: HashMap<String, Arc<dyn TransportComponent>> = HashMap::new();
  transports.insert("http".to_string(), Arc::new(HttpTransport::new()));
  let mut engine = Engine::with_components(transports, Some(Arc::new(JsonContent::new())));
  engine.register_content(Arc::new(TextContent::new()));
  engine.declare_in_tree("content", "text", "1.0.0");
  send(
    &mut engine,
    "engine/hello",
    json!({ "protocol-versions": [1], "host": { "name": "janus-test", "version": "0.0.0" }, "capabilities": {} }),
  );
  engine
}

/// `add-interaction` for a `GET /export` answered with `body` as `content_type`.
fn add(content_type: &str, body: Value) -> Value {
  let mut engine = engine();
  let created = send(
    &mut engine,
    "consumer-session/create",
    json!({ "config": { "consumer": { "name": "c" }, "provider": { "name": "p" } } }),
  );
  send(
    &mut engine,
    "consumer-session/add-interaction",
    json!({ "session": created["ok"]["session"], "interaction": {
      "description": "the export",
      "transport": { "kind": "http", "mode": "passive" },
      "content-types": { "response": { "body": content_type } },
      "parts": {
        "request": { "method": { "shape": "equality", "example": "GET" },
                     "path": { "shape": "equality", "example": "/export" } },
        "response": { "status": { "shape": "equality", "example": 200 }, "body": body } } } }),
  )
}

#[rstest]
#[case::a_string("text/csv", json!({ "shape": "string", "example": "id,status\r\n66,PENDING\r\n" }))]
#[case::a_regex("text/html", json!({ "shape": "regex", "pattern": "^<html>.*", "example": "<html></html>" }))]
#[case::a_choice_of_strings("text/plain", json!({ "shape": "any-of", "options": ["yes", "no"], "example": "yes" }))]
fn a_text_body_that_is_a_string_needs_no_declared_component(#[case] content_type: &str, #[case] body: Value) {
  let added = add(content_type, body);
  assert!(added["ok"].is_object(), "{added}");
}

#[rstest]
#[case::rows("text/csv", json!({ "shape": "each-like", "min": 1,
  "items": { "shape": "object", "members": { "id": { "shape": "string", "example": "66" } } } }), "each-like")]
#[case::a_number("text/plain", json!({ "shape": "integer", "example": 4 }), "integer")]
fn a_text_body_that_admits_no_string_is_refused_naming_its_type(
  #[case] content_type: &str,
  #[case] body: Value,
  #[case] kind: &str,
) {
  let added = add(content_type, body);
  let error = &added["error"];
  assert_eq!(error["code"], json!("component-unavailable"), "{added}");
  assert_eq!(
    error["details"]["component"],
    json!(format!("content/{content_type}"))
  );
  assert_eq!(error["details"]["reason"], json!("compile-failed"));
  let message = error["message"].as_str().unwrap_or_default();
  assert!(
    message.contains(&format!("('{kind}') admits no string")) && message.contains("declare a component"),
    "{message}"
  );
}
