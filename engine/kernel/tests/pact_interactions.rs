//! ADR 0066: `consumer-session/add-pact-interaction` adds the interaction a consumer library's
//! `pact_ffi` calls build, converted as `upgrade/pact` converts a pact's. Driven through the protocol
//! against the real HTTP transport: a facade test, written as pact-js's http example is, serves its
//! base variant, and the contract it records holds what `janus upgrade` makes of pact-js's pact.

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
  let request = json!({ "type": "request", "id": "r", "op": op, "body": body });
  let bytes = engine.dispatch(&serde_json::to_vec(&request).unwrap());
  serde_json::from_slice(&bytes).unwrap()
}

fn engine() -> (Engine, String) {
  let mut transports: HashMap<String, Arc<dyn TransportComponent>> = HashMap::new();
  transports.insert("http".to_string(), Arc::new(HttpTransport::new()));
  let mut engine = Engine::with_components(transports, Some(Arc::new(JsonContent::new())));
  engine.register_background(Arc::new(pact_janus_kernel::protocol::Threads));
  send(
    &mut engine,
    "engine/hello",
    json!({ "protocol-versions": [1], "host": { "name": "janus-test", "version": "0.0.0" },
            "capabilities": { "test-verdicts": {} } }),
  );
  let created = send(
    &mut engine,
    "consumer-session/create",
    json!({ "config": { "consumer": { "name": "UserConsumer" }, "provider": { "name": "UserProvider" } } }),
  );
  let session = created["ok"]["session"].as_str().unwrap().to_string();
  (engine, session)
}

fn like(value: Value) -> Value {
  json!({ "pact:matcher:type": "type", "value": value })
}

/// pact-js's http example's first test, as its V4 builder calls the FFI.
fn get_user() -> Value {
  json!({ "specification": "V4", "steps": [
    { "call": "given", "description": "a user with ID 1 exists" },
    { "call": "upon_receiving", "description": "a GET request for user 1" },
    { "call": "with_request", "method": "GET", "path": "/users/1" },
    { "call": "with_header_v2", "part": "request", "name": "Accept", "index": 0, "value": "application/json" },
    { "call": "response_status_v2", "status": "200" },
    { "call": "with_header_v2", "part": "response", "name": "Content-Type", "index": 0, "value": "application/json" },
    { "call": "with_body", "part": "response", "content_type": "application/json", "body": like(json!({
        "id": { "pact:matcher:type": "integer", "value": 1 },
        "name": like(json!("Alice")),
        "email": like(json!("alice@example.com"))
    })) }
  ] })
}

/// One GET with an `Accept` header, answering the status and the body.
fn get(addr: &str, path: &str) -> (u16, Value) {
  let mut stream = TcpStream::connect(addr).unwrap();
  stream.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
  let request =
    format!("GET {path} HTTP/1.1\r\nHost: {addr}\r\nAccept: application/json\r\nConnection: close\r\n\r\n");
  stream.write_all(request.as_bytes()).unwrap();
  let mut response = Vec::new();
  let _ = stream.read_to_end(&mut response);
  let text = String::from_utf8_lossy(&response).to_string();
  let status = text
    .split_whitespace()
    .nth(1)
    .and_then(|s| s.parse().ok())
    .unwrap();
  let body = text.split("\r\n\r\n").nth(1).unwrap_or_default();
  (status, serde_json::from_str(body).unwrap_or(Value::Null))
}

#[test]
fn a_facade_test_records_what_upgrade_makes_of_the_pact() {
  let (mut engine, session) = engine();
  let added = send(
    &mut engine,
    "consumer-session/add-pact-interaction",
    json!({ "session": session, "pact": get_user() }),
  );
  let handle = added["ok"]["handle"]
    .as_str()
    .unwrap_or_else(|| panic!("{added}"))
    .to_string();

  // One run, against the example, as pact-js runs the test, whatever policy is asked for: the base
  // variant, pinning nothing, as upgrade records it (ADR 0066 decision 4).
  let selected = send(
    &mut engine,
    "consumer-session/variants",
    json!({ "session": session, "handle": handle, "policy": { "strategy": "exhaustive" } }),
  );
  let variants = selected["ok"]["variants"].as_array().unwrap();
  assert_eq!(variants.len(), 1, "{selected}");
  assert_eq!(variants[0]["assignment"], json!([]), "{selected}");
  let base = variants[0]["id"].as_str().unwrap().to_string();

  let started = send(
    &mut engine,
    "consumer-session/start-transport",
    json!({ "session": session, "transport": "http" }),
  );
  let endpoint = &started["ok"]["endpoint"];
  let addr = format!(
    "{}:{}",
    endpoint["host"].as_str().unwrap(),
    endpoint["port"].as_u64().unwrap()
  );
  send(
    &mut engine,
    "consumer-session/serve-variant",
    json!({ "session": session, "handle": handle, "variant": base }),
  );
  assert_eq!(
    get(&addr, "/users/1"),
    (
      200,
      json!({ "id": 1, "name": "Alice", "email": "alice@example.com" })
    )
  );
  let reported = send(
    &mut engine,
    "consumer-session/report-variant",
    json!({ "session": session, "handle": handle, "variant": base, "status": "passed" }),
  );
  assert_eq!(reported["ok"]["result"]["status"], "verified", "{reported}");

  let finalised = send(
    &mut engine,
    "consumer-session/finalise",
    json!({ "session": session }),
  );
  let recorded = &finalised["ok"]["contract"]["interactions"][0];

  let path = format!(
    "{}/../../corpora/verifier/examples/pact-js/http/pact.json",
    env!("CARGO_MANIFEST_DIR")
  );
  let upgraded = send(
    &mut engine,
    "upgrade/pact",
    json!({ "pact": serde_json::from_str::<Value>(&std::fs::read_to_string(path).unwrap()).unwrap() }),
  );
  let expected = upgraded["ok"]["contract"]["interactions"]
    .as_array()
    .unwrap()
    .iter()
    .find(|interaction| interaction["description"] == "a GET request for user 1")
    .unwrap()
    .clone();
  // Everything, the recorded variant too: its values are the interaction's own example, as upgrade
  // records it (ADR 0066 decision 4), so a verifier replays the request today's verifier sends. A
  // JSON body's text is compared as its document: pact_ffi wrote pact-js's pact with its members
  // sorted, and the engine keeps them in the order the test wrote them (ADR 0037).
  assert_eq!(documents(recorded.clone()), documents(expected));
}

/// A contract interaction with each recorded JSON body's text parsed, so member order is not compared.
fn documents(mut interaction: Value) -> Value {
  for variant in interaction["selection"]["variants"].as_array_mut().unwrap() {
    for part in variant["parts"].as_object_mut().unwrap().values_mut() {
      if let Some(body) = part.get_mut("body")
        && body["encoded"] == "text"
        && let Some(document) = body["content"]
          .as_str()
          .and_then(|text| serde_json::from_str::<Value>(text).ok())
      {
        body["content"] = document;
      }
    }
  }
  interaction
}

#[test]
fn a_step_the_engine_cannot_make_is_refused_pointing_at_it() {
  let (mut engine, session) = engine();
  let refused = send(
    &mut engine,
    "consumer-session/add-pact-interaction",
    json!({ "session": session, "pact": { "steps": [
      { "call": "upon_receiving", "description": "a plugin interaction" },
      { "call": "with_plugin_contents", "part": "request" }
    ] } }),
  );
  assert_eq!(refused["error"]["code"], "interaction-invalid", "{refused}");
  assert_eq!(
    refused["error"]["details"]["problems"][0]["pointer"],
    "/pact/steps/1/call"
  );

  // A refused interaction withholds the contract, named by its description (ADR 0064 decision 4).
  let finalised = send(
    &mut engine,
    "consumer-session/finalise",
    json!({ "session": session }),
  );
  assert_eq!(
    finalised["ok"]["refused"][0]["description"], "a plugin interaction",
    "{finalised}"
  );
}

#[test]
fn what_the_conversion_lost_is_in_the_answer() {
  let (mut engine, session) = engine();
  // pact-js's eachKeyMatches: upgrade keeps the keys' rule and leaves the values open.
  let added = send(
    &mut engine,
    "consumer-session/add-pact-interaction",
    json!({ "session": session, "pact": { "steps": [
      { "call": "upon_receiving", "description": "the catalog" },
      { "call": "with_request", "method": "GET", "path": "/catalog" },
      { "call": "response_status_v2", "status": "200" },
      { "call": "with_body", "part": "response", "content_type": "application/json", "body": {
          "pact:matcher:type": "eachKey",
          "rules": [ { "pact:matcher:type": "regex", "regex": "^[A-Z]+-\\d{3}$", "value": "WIDGET-001" } ],
          "value": { "WIDGET-001": { "name": like(json!("Widget")) } }
      } }
    ] } }),
  );
  let findings = added["ok"]["findings"]
    .as_array()
    .unwrap_or_else(|| panic!("{added}"));
  let lossy = findings.iter().find(|f| f["code"] == "rule-unmapped").unwrap();
  assert_eq!(lossy["kind"], "lossy");
  assert_eq!(
    lossy["path"], "/response/body",
    "a pointer into the built interaction"
  );

  // The shapes cannot give the example back (an each-entry of open values, least size 0), so the
  // mock serves the interaction's own, as today's mock server does.
  let handle = added["ok"]["handle"].as_str().unwrap().to_string();
  let selected = send(
    &mut engine,
    "consumer-session/variants",
    json!({ "session": session, "handle": handle }),
  );
  let base = selected["ok"]["variants"][0]["id"].as_str().unwrap().to_string();
  let started = send(
    &mut engine,
    "consumer-session/start-transport",
    json!({ "session": session, "transport": "http" }),
  );
  let endpoint = &started["ok"]["endpoint"];
  let addr = format!(
    "{}:{}",
    endpoint["host"].as_str().unwrap(),
    endpoint["port"].as_u64().unwrap()
  );
  send(
    &mut engine,
    "consumer-session/serve-variant",
    json!({ "session": session, "handle": handle, "variant": base }),
  );
  assert_eq!(
    get(&addr, "/catalog"),
    (200, json!({ "WIDGET-001": { "name": "Widget" } }))
  );
}
