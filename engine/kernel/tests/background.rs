//! ADR 0062: the kernel runs its background work through what its embedding registers, and an
//! engine with nothing registered says so in its hello and refuses, by name, the operations that
//! need it. `broker/publish-contracts`, the third, is in `tests/broker.rs`, beside its fake broker.

use pact_janus_component_http::HttpTransport;
use pact_janus_component_json::JsonContent;
use pact_janus_kernel::component::TransportComponent;
use pact_janus_kernel::protocol::{Engine, Threads};
use pretty_assertions::assert_eq;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;

fn send(engine: &mut Engine, id: &str, op: &str, body: Value) -> Value {
  let request = json!({ "type": "request", "id": id, "op": op, "body": body });
  let bytes = engine.dispatch(&serde_json::to_vec(&request).expect("a json! literal always serializes"));
  serde_json::from_slice(&bytes).expect("Engine::dispatch always returns valid JSON")
}

/// An engine with the real HTTP transport, and threads only when `threads` says so.
fn engine(threads: bool) -> (Engine, Value) {
  let mut transports: HashMap<String, Arc<dyn TransportComponent>> = HashMap::new();
  transports.insert("http".to_string(), Arc::new(HttpTransport::new()));
  let mut engine = Engine::with_components(transports, Some(Arc::new(JsonContent::new())));
  if threads {
    engine.register_background(Arc::new(Threads));
  }
  let hello = send(
    &mut engine,
    "h-1",
    "engine/hello",
    json!({ "protocol-versions": [1], "host": { "name": "janus-test", "version": "0.0.0" }, "capabilities": {} }),
  );
  (engine, hello)
}

/// The refusal ADR 0062 decision 3 specifies.
fn refused(response: &Value, what: &str) {
  assert_eq!(response["error"]["code"], "component-unavailable", "{response}");
  assert_eq!(
    response["error"]["details"],
    json!({ "component": "background" }),
    "{response}"
  );
  assert_eq!(
    response["error"]["message"],
    format!("this engine cannot run work in the background, so it cannot {what}")
  );
}

#[test]
fn hello_declares_background_only_when_the_embedding_registered_it() {
  let (_, with) = engine(true);
  assert_eq!(with["ok"]["capabilities"]["background"], json!({}), "{with}");
  let (_, without) = engine(false);
  assert!(
    without["ok"]["capabilities"].get("background").is_none(),
    "{without}"
  );
}

#[test]
fn start_transport_without_background_is_refused_and_starts_nothing() {
  let (mut engine, _) = engine(false);
  let created = send(
    &mut engine,
    "c-1",
    "consumer-session/create",
    json!({ "config": { "consumer": { "name": "web-app" }, "provider": { "name": "orders-api" } } }),
  );
  let session = created["ok"]["session"].as_str().expect("a session").to_string();
  let started = send(
    &mut engine,
    "c-2",
    "consumer-session/start-transport",
    json!({ "session": session, "transport": "http" }),
  );
  refused(&started, "serve a transport");
}

#[test]
fn start_transport_with_background_serves_and_finalise_stops_it() {
  let (mut engine, _) = engine(true);
  let created = send(
    &mut engine,
    "c-1",
    "consumer-session/create",
    json!({ "config": { "consumer": { "name": "web-app" }, "provider": { "name": "orders-api" } } }),
  );
  let session = created["ok"]["session"].as_str().expect("a session").to_string();
  let started = send(
    &mut engine,
    "c-2",
    "consumer-session/start-transport",
    json!({ "session": session, "transport": "http" }),
  );
  assert!(started["ok"]["endpoint"]["base-url"].is_string(), "{started}");
  let finalised = send(
    &mut engine,
    "c-3",
    "consumer-session/finalise",
    json!({ "session": session }),
  );
  assert!(finalised.get("ok").is_some(), "{finalised}");
}

#[test]
fn verify_without_background_is_refused_after_its_documents_are_checked() {
  let (mut engine, _) = engine(false);
  let target =
    json!({ "transports": [ { "transport": "http", "options": { "base-url": "http://127.0.0.1:9" } } ] });

  // A document that is not a contract is still the document's error, as on any engine.
  let invalid = send(
    &mut engine,
    "v-1",
    "verification/verify",
    json!({ "source": { "kind": "inline", "contracts": [ { "not": "a contract" } ] }, "target": target }),
  );
  assert_ne!(invalid["error"]["code"], "component-unavailable", "{invalid}");

  let contract = json!({
    "$format": "janus-contract/1",
    "consumer": { "name": "web-app" },
    "provider": { "name": "orders-api" },
    "interactions": [{
      "description": "get an order",
      "transport": { "kind": "http", "mode": "passive" },
      "parts": {
        "request": { "method": { "shape": "equality", "example": "GET" },
                     "path": { "shape": "equality", "example": "/orders/1" } },
        "response": { "status": { "shape": "equality", "example": 200 } }
      },
      "selection": {
        "variants": [{
          "id": "base", "origin": "base", "assignment": [],
          "parts": {
            "request": { "method": { "content": "GET" }, "path": { "content": "/orders/1" } },
            "response": { "status": { "content": 200 } }
          }
        }],
        "report": { "space": { "size": 1, "dimensions": 0 } }
      }
    }]
  });
  let verified = send(
    &mut engine,
    "v-2",
    "verification/verify",
    json!({ "source": { "kind": "inline", "contracts": [contract] }, "target": target }),
  );
  refused(&verified, "run a verification");
}
