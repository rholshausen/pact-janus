//! Plan task 4.6 — a report companion, not a test (it is *supposed* to panic; that is the whole
//! point). Findings: `Documentation/variant-ergonomics-report.md`.
//!
//! A consumer that loops over every selected variant correctly (no shortcut, no
//! `--exhaustive`-only-on-CI skip) but then reads the response the way most hand-written test
//! code does: assume the field it cares about is there. The RFC's own order example
//! (shape-language spec examples/order-payload.md) makes `shippedAt` an `optional` — legitimately
//! absent for a `PENDING` order — so the boundary variant that pins it absent is exactly the case
//! this consumer mishandles.
//!
//! Run it plain to see the panic a careless consumer actually gets:
//!   cargo run -p pact_janus_kernel --example careless_consumer
//! Run it with the trace-level frame logging plan task 4.5 wired up, to see whether the ambient
//! protocol trace helps more than the panic alone:
//!   RUST_LOG=trace cargo run -p pact_janus_kernel --example careless_consumer
//! And with a backtrace, since that's the other thing a real developer reaches for:
//!   RUST_BACKTRACE=1 cargo run -p pact_janus_kernel --example careless_consumer

use pact_janus_component_http::HttpTransport;
use pact_janus_component_json::JsonContent;
use pact_janus_kernel::component::TransportComponent;
use pact_janus_kernel::protocol::Engine;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::Arc;
use std::time::Duration;

fn send(engine: &mut Engine, id: &str, op: &str, body: Value) -> Value {
  let request = json!({ "type": "request", "id": id, "op": op, "body": body });
  let bytes = engine.dispatch(&serde_json::to_vec(&request).expect("a json! literal always serializes"));
  serde_json::from_slice(&bytes).expect("Engine::dispatch always returns valid JSON")
}

// The same widened order interaction engine/kernel/tests/consumer_flow.rs and tests/protocol.rs
// use: an `optional` member gives two variants, `base` and the boundary where it is absent.
fn order_interaction_with_optional_field() -> Value {
  json!({
    "description": "a request for an order",
    "transport": { "kind": "http", "mode": "passive" },
    "parts": {
      "request": { "method": { "shape": "equality", "example": "GET" },
                   "path": { "shape": "equality", "example": "/orders/66" } },
      "response": { "status": { "shape": "equality", "example": 200 },
                    "body": { "shape": "object",
                              "members": { "shippedAt": { "shape": "optional",
                                                           "of": { "shape": "string", "example": "2026-07-30" } } } } } }
  })
}

fn http_get(addr: &str, path: &str) -> (u16, Value) {
  let mut stream =
    TcpStream::connect(addr).unwrap_or_else(|err| panic!("connecting to the mock at {addr}: {err}"));
  stream
    .set_read_timeout(Some(Duration::from_secs(2)))
    .expect("setting a read timeout");
  let request = format!("GET {path} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n");
  stream.write_all(request.as_bytes()).expect("sending the request");
  let mut response = Vec::new();
  let _ = stream.read_to_end(&mut response);
  let split = response
    .windows(4)
    .position(|w| w == b"\r\n\r\n")
    .expect("a header/body separator");
  let head = std::str::from_utf8(&response[..split]).expect("headers are ASCII");
  let body = &response[split + 4..];
  let status: u16 = head
    .lines()
    .next()
    .and_then(|line| line.split_whitespace().nth(1))
    .and_then(|code| code.parse().ok())
    .expect("a status code");
  (status, serde_json::from_slice(body).unwrap_or(Value::Null))
}

fn main() {
  let mut transports: HashMap<String, Arc<dyn TransportComponent>> = HashMap::new();
  transports.insert("http".to_string(), Arc::new(HttpTransport::new()));
  let mut engine = Engine::with_components(transports, Some(Arc::new(JsonContent::new())));

  send(
    &mut engine,
    "r-1",
    "engine/hello",
    json!({ "protocol-versions": [1], "host": { "name": "careless-consumer", "version": "0.0.0" }, "capabilities": {} }),
  );
  let session = send(
    &mut engine,
    "r-2",
    "consumer-session/create",
    json!({ "config": { "consumer": { "name": "web-app" }, "provider": { "name": "order-api" } } }),
  )["ok"]["session"]
    .as_str()
    .unwrap()
    .to_string();
  let handle = send(
    &mut engine,
    "r-3",
    "consumer-session/add-interaction",
    json!({ "session": session, "interaction": order_interaction_with_optional_field() }),
  )["ok"]["handle"]
    .as_str()
    .unwrap()
    .to_string();

  let variants = send(
    &mut engine,
    "r-4",
    "consumer-session/variants",
    json!({ "session": session, "handle": handle }),
  );
  let variant_ids: Vec<String> = variants["ok"]["variants"]
    .as_array()
    .unwrap()
    .iter()
    .map(|v| v["id"].as_str().unwrap().to_string())
    .collect();
  println!("selected variants: {variant_ids:?}");

  let started = send(
    &mut engine,
    "r-5",
    "consumer-session/start-transport",
    json!({ "session": session, "transport": "http" }),
  );
  let endpoint = &started["ok"]["endpoint"];
  let addr = format!(
    "{}:{}",
    endpoint["host"].as_str().unwrap(),
    endpoint["port"].as_u64().unwrap()
  );

  // The careless loop: every variant IS exercised (the honesty rule is satisfied), but the
  // consumer's own assertion assumes a field the shape itself declares optional.
  for (i, variant_id) in variant_ids.iter().enumerate() {
    send(
      &mut engine,
      &format!("r-{}", 10 + i),
      "consumer-session/serve-variant",
      json!({ "session": session, "handle": handle, "variant": variant_id }),
    );
    let (status, body) = http_get(&addr, "/orders/66");
    println!("variant {variant_id}: status={status}");
    let shipped_at = body["shippedAt"]
      .as_str()
      .expect("shippedAt should always be present");
    println!("  shippedAt = {shipped_at}");
  }

  send(
    &mut engine,
    "r-99",
    "consumer-session/finalise",
    json!({ "session": session }),
  );
}
