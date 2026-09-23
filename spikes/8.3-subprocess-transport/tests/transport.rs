//! The escape hatch, end to end: a `tcp` transport written in Node, declared by a project as a
//! `subprocess` component, driven by the real engine as a consumer mock and then as a verifier —
//! the same process kind, both roles, over the same pipe.

use pact_janus_component_json::JsonContent;
use pact_janus_kernel::protocol::Engine;
use pretty_assertions::assert_eq;
use serde_json::{Value, json};
use spike_subprocess_transport::SubprocessLoader;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;

fn component(script: &str) -> String {
  let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    .join("component")
    .join(script);
  format!("node {}", path.display())
}

fn declaration() -> Value {
  json!({ "name": "tcp", "source": { "kind": "subprocess", "reference": component("tcp-transport.mjs") } })
}

fn send(engine: &mut Engine, op: &str, body: Value) -> Value {
  let request = json!({ "type": "request", "id": "1", "op": op, "body": body });
  serde_json::from_slice(&engine.dispatch(&serde_json::to_vec(&request).unwrap())).unwrap()
}

/// An engine with no transport of its own at all: whatever speaks `tcp` has to be declared.
fn engine() -> Engine {
  let _ = tracing_subscriber::fmt()
    .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
    .with_test_writer()
    .try_init();
  let mut engine = Engine::with_components(HashMap::new(), Some(Arc::new(JsonContent::new())));
  engine.declare_in_tree("content", "json", "1.0.0");
  engine.register_component_loader(Arc::new(SubprocessLoader));
  let hello = send(
    &mut engine,
    "engine/hello",
    json!({ "protocol-versions": [1], "host": { "name": "spike-8.3", "version": "0.0.0" }, "capabilities": {} }),
  );
  assert_eq!(
    hello["ok"]["capabilities"]["components"],
    json!({ "loaders": ["in-tree", "subprocess"] })
  );
  engine
}

/// "Look up an order" over JSON lines: one line in, one line out.
fn order_lookup() -> Value {
  json!({
    "description": "an order, by id, over a JSON-lines socket",
    "transport": { "kind": "tcp", "mode": "passive" },
    "requires": [ { "component": "transport/tcp", "min-version": 0 } ],
    "content-types": { "request": { "body": "application/json" }, "response": { "body": "application/json" } },
    "parts": {
      "request": { "body": { "shape": "object", "members": {
        "op": { "shape": "equality", "example": "get-order" },
        "id": { "shape": "string", "example": "66" } } } },
      "response": { "body": { "shape": "object", "members": {
        "id": { "shape": "string", "example": "66" },
        "status": { "shape": "equality", "example": "PENDING" } } } } }
  })
}

fn line_exchange(addr: &str, line: &str) -> String {
  let mut stream = TcpStream::connect(addr).unwrap();
  writeln!(stream, "{line}").unwrap();
  let mut reply = String::new();
  BufReader::new(stream).read_line(&mut reply).unwrap();
  reply.trim_end().to_string()
}

/// A consumer test: the engine's mock answers over raw TCP through the Node process.
fn consumer_run(engine: &mut Engine) -> Value {
  let created = send(
    engine,
    "consumer-session/create",
    json!({ "config": { "consumer": { "name": "dispatch" }, "provider": { "name": "order-lines" },
                        "components": [declaration()] } }),
  );
  let session = created["ok"]["session"]
    .as_str()
    .unwrap_or_else(|| panic!("create: {created}"))
    .to_string();
  let added = send(
    engine,
    "consumer-session/add-interaction",
    json!({ "session": session, "interaction": order_lookup() }),
  );
  let handle = added["ok"]["handle"]
    .as_str()
    .unwrap_or_else(|| panic!("add: {added}"))
    .to_string();
  let variants = send(
    engine,
    "consumer-session/variants",
    json!({ "session": session, "handle": handle }),
  );
  let started = send(
    engine,
    "consumer-session/start-transport",
    json!({ "session": session, "transport": "tcp" }),
  );
  let endpoint = &started["ok"]["endpoint"];
  assert_eq!(endpoint["kind"], "tcp", "{started}");
  let addr = format!("{}:{}", endpoint["host"].as_str().unwrap(), endpoint["port"]);

  for variant in variants["ok"]["variants"].as_array().unwrap() {
    let served = send(
      engine,
      "consumer-session/serve-variant",
      json!({ "session": session, "handle": handle, "variant": variant["id"] }),
    );
    assert_eq!(served["ok"], json!({}), "{served}");
    let reply: Value =
      serde_json::from_str(&line_exchange(&addr, r#"{"op":"get-order","id":"66"}"#)).unwrap();
    assert_eq!(reply["status"], "PENDING", "{reply}");
  }

  let finalised = send(engine, "consumer-session/finalise", json!({ "session": session }));
  let results = &finalised["ok"]["results"];
  assert_eq!(
    results,
    &json!([ { "handle": "i-1", "status": "verified", "variants": [ { "status": "verified", "variant": "base" } ] } ]),
    "{finalised}"
  );
  finalised["ok"]["contract"].clone()
}

#[test]
fn a_consumer_test_is_served_over_raw_tcp_by_a_node_process() {
  let mut engine = engine();
  let contract = consumer_run(&mut engine);
  let interaction = &contract["interactions"][0];
  assert_eq!(interaction["transport"]["kind"], "tcp");
  assert!(
    !interaction["selection"]["variants"]
      .as_array()
      .unwrap()
      .is_empty(),
    "{contract}"
  );
}

#[test]
fn a_request_that_does_not_match_is_answered_not_hung() {
  let mut engine = engine();
  let session = send(
    &mut engine,
    "consumer-session/create",
    json!({ "config": { "consumer": { "name": "c" }, "provider": { "name": "p" }, "components": [declaration()] } }),
  )["ok"]["session"]
    .clone();
  let handle = send(
    &mut engine,
    "consumer-session/add-interaction",
    json!({ "session": session, "interaction": order_lookup() }),
  )["ok"]["handle"]
    .clone();
  let variants = send(
    &mut engine,
    "consumer-session/variants",
    json!({ "session": session, "handle": handle }),
  );
  let endpoint = send(
    &mut engine,
    "consumer-session/start-transport",
    json!({ "session": session, "transport": "tcp" }),
  )["ok"]["endpoint"]
    .clone();
  send(
    &mut engine,
    "consumer-session/serve-variant",
    json!({ "session": session, "handle": handle, "variant": variants["ok"]["variants"][0]["id"] }),
  );
  let addr = format!("{}:{}", endpoint["host"].as_str().unwrap(), endpoint["port"]);
  let reply = line_exchange(&addr, r#"{"op":"cancel-order","id":"66"}"#);
  // The kernel's mismatch reply is a list of mismatches in the body — and a `status: 500` slot this
  // transport has no use for, because the kernel wrote it for HTTP (finding 4).
  assert!(reply.contains("cancel-order"), "{reply}");
}

/// A provider that speaks JSON lines: one request line, one reply line, per connection.
fn provider(status: &'static str) -> String {
  let listener = TcpListener::bind("127.0.0.1:0").unwrap();
  let addr = listener.local_addr().unwrap().to_string();
  thread::spawn(move || {
    for stream in listener.incoming().flatten() {
      let mut line = String::new();
      let mut reader = BufReader::new(stream.try_clone().unwrap());
      if reader.read_line(&mut line).is_err() {
        continue;
      }
      let request: Value = serde_json::from_str(&line).unwrap_or(Value::Null);
      let reply = json!({ "id": request["id"], "status": status, "placed": "2026-09-23" });
      let _ = writeln!(&stream, "{reply}");
    }
  });
  addr
}

fn drain(engine: &mut Engine, stream: &str) -> Value {
  for _ in 0..100 {
    let polled = send(
      engine,
      "events/poll",
      json!({ "streams": [stream], "wait-ms": 5_000 }),
    );
    for event in polled["ok"]["events"].as_array().unwrap() {
      if event["last"] == json!(true) {
        return event["payload"].clone();
      }
    }
  }
  panic!("the run never finished");
}

fn verify(engine: &mut Engine, contract: &Value, provider: &str) -> Value {
  let (host, port) = provider.split_once(':').unwrap();
  let started = send(
    engine,
    "verification/verify",
    json!({
      "source": { "kind": "inline", "contracts": [contract] },
      "target": { "transports": [ { "transport": "tcp",
                                    "options": { "host": host, "port": port.parse::<u16>().unwrap() } } ],
                  "components": [declaration()] },
    }),
  );
  let stream = started["ok"]["stream"]
    .as_str()
    .unwrap_or_else(|| panic!("verify: {started}"))
    .to_string();
  drain(engine, &stream)
}

#[test]
fn the_contract_is_verified_against_a_tcp_provider_through_the_same_component() {
  let mut engine = engine();
  let contract = consumer_run(&mut engine);
  let summary = verify(&mut engine, &contract, &provider("PENDING"));
  assert_eq!(summary["status"], "verified", "{summary}");
  assert_eq!(summary["variants"]["verified"], 1, "{summary}");
}

#[test]
fn a_tcp_provider_that_answers_differently_fails_verification() {
  let mut engine = engine();
  let contract = consumer_run(&mut engine);
  let summary = verify(&mut engine, &contract, &provider("SHIPPED"));
  assert_eq!(summary["status"], "failed", "{summary}");
  assert_eq!(summary["variants"]["failed"], 1, "{summary}");
}
