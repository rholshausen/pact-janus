//! ADR 0038 on the consumer side: a Janus contract can declare a form body, shaped as the query slot
//! is (an object of string lists), and the mock serves it through the built-in form component, with
//! its names in the order the shape declares them (ADR 0037).

use pact_janus_component_form::FormContent;
use pact_janus_component_http::HttpTransport;
use pact_janus_component_json::JsonContent;
use pact_janus_kernel::component::TransportComponent;
use pact_janus_kernel::protocol::Engine;
use pretty_assertions::assert_eq;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::Arc;
use std::time::Duration;

fn send(engine: &mut Engine, op: &str, body: Value) -> Value {
  let request = json!({ "type": "request", "id": "1", "op": op, "body": body });
  serde_json::from_slice(&engine.dispatch(&serde_json::to_vec(&request).unwrap())).unwrap()
}

fn engine() -> Engine {
  let mut transports: HashMap<String, Arc<dyn TransportComponent>> = HashMap::new();
  transports.insert("http".to_string(), Arc::new(HttpTransport::new()));
  let mut engine = Engine::with_components(transports, Some(Arc::new(JsonContent::new())));
  engine.register_content(Arc::new(FormContent::new()));
  engine.declare_in_tree("content", "form", "1.0.0");
  send(
    &mut engine,
    "engine/hello",
    json!({ "protocol-versions": [1], "host": { "name": "janus-test", "version": "0.0.0" }, "capabilities": {} }),
  );
  engine
}

/// One GET to the mock: its status line, and its body as text.
fn get(addr: &str, path: &str) -> (String, String) {
  let mut stream = TcpStream::connect(addr).unwrap();
  stream.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
  stream
    .write_all(format!("GET {path} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n").as_bytes())
    .unwrap();
  let mut response = Vec::new();
  let _ = stream.read_to_end(&mut response);
  let response = String::from_utf8(response).unwrap();
  let (head, body) = response.split_once("\r\n\r\n").unwrap();
  (head.lines().next().unwrap().to_string(), body.to_string())
}

#[test]
fn the_mock_serves_a_form_body_with_its_names_as_declared() {
  let mut engine = engine();
  let created = send(
    &mut engine,
    "consumer-session/create",
    json!({ "config": { "consumer": { "name": "c" }, "provider": { "name": "p" } } }),
  );
  let session = created["ok"]["session"].clone();
  let added = send(
    &mut engine,
    "consumer-session/add-interaction",
    json!({ "session": session, "interaction": {
      "description": "a search form",
      "transport": { "kind": "http", "mode": "passive" },
      "content-types": { "response": { "body": "application/x-www-form-urlencoded" } },
      "parts": {
        "request": { "method": { "shape": "equality", "example": "GET" },
                     "path": { "shape": "equality", "example": "/search" } },
        "response": { "status": { "shape": "equality", "example": 200 },
                      "body": { "shape": "object", "members": {
                        "query": { "shape": "equality", "example": ["red shoes"] },
                        "tag": { "shape": "equality", "example": ["sale", "new"] } } } } } } }),
  );
  let handle = added["ok"]["handle"].clone();
  assert!(handle.is_string(), "{added}");
  send(
    &mut engine,
    "consumer-session/variants",
    json!({ "session": session, "handle": handle }),
  );
  let started = send(
    &mut engine,
    "consumer-session/start-transport",
    json!({ "session": session, "transport": "http" }),
  );
  let endpoint = &started["ok"]["endpoint"];
  let addr = format!("{}:{}", endpoint["host"].as_str().unwrap(), endpoint["port"]);
  send(
    &mut engine,
    "consumer-session/serve-variant",
    json!({ "session": session, "handle": handle, "variant": "base" }),
  );

  let (status, body) = get(&addr, "/search");
  assert_eq!(status, "HTTP/1.1 200 OK");
  assert_eq!(body, "query=red+shoes&tag=sale&tag=new");

  let finalised = send(
    &mut engine,
    "consumer-session/finalise",
    json!({ "session": session }),
  );
  let recorded =
    &finalised["ok"]["contract"]["interactions"][0]["selection"]["variants"][0]["parts"]["response"]["body"];
  assert_eq!(
    recorded,
    &json!({ "content": { "query": ["red shoes"], "tag": ["sale", "new"] },
             "content-type": "application/x-www-form-urlencoded" }),
    "the contract records the document, and its type"
  );
}
