//! Plan task 8.1, end to end: the out-of-tree CSV component (`third-party/janus-csv`, which depends
//! on nothing in this workspace) built to `wasm32-wasip2`, declared by path, loaded by the WASM
//! loader, and used by the real engine in a consumer test *and* a verification — the same `.wasm`
//! both times, unmodified.
//!
//! The component is built by the test itself, once per test binary (`support::csv_wasm`), so these
//! tests never run against a stale `.wasm`.

mod support;

use pact_janus_component_host::WasmLoader;
use pact_janus_component_http::HttpTransport;
use pact_janus_component_json::JsonContent;
use pact_janus_kernel::component::TransportComponent;
use pact_janus_kernel::protocol::Engine;
use pact_janus_sample_order_service as order_service;
use pretty_assertions::assert_eq;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::Arc;
use std::time::Duration;
use support::csv_wasm;

fn declaration() -> Value {
  json!({ "name": "csv", "source": { "kind": "file", "reference": csv_wasm() } })
}

fn send(engine: &mut Engine, op: &str, body: Value) -> Value {
  let request = json!({ "type": "request", "id": "1", "op": op, "body": body });
  serde_json::from_slice(&engine.dispatch(&serde_json::to_vec(&request).unwrap())).unwrap()
}

/// What `janus-engine` builds: the in-tree HTTP transport and JSON content, and the WASM loader.
fn engine() -> Engine {
  let mut transports: HashMap<String, Arc<dyn TransportComponent>> = HashMap::new();
  transports.insert("http".to_string(), Arc::new(HttpTransport::new()));
  let mut engine = Engine::with_components(transports, Some(Arc::new(JsonContent::new())));
  engine.declare_in_tree("content", "json", "1.0.0");
  engine.register_component_loader(Arc::new(WasmLoader::new().expect("wasmtime starts")));
  let hello = send(
    &mut engine,
    "engine/hello",
    json!({ "protocol-versions": [1], "host": { "name": "janus-test", "version": "0.0.0" }, "capabilities": {} }),
  );
  assert_eq!(
    hello["ok"]["capabilities"]["components"],
    json!({ "loaders": ["in-tree", "wasm"] })
  );
  engine
}

/// Orders as CSV: the columns this consumer reads, as the CSV component decodes them — strings,
/// every one of them, because that is all CSV has (the component's declared `string-only`
/// degradation). `items` is a count, and a count in CSV is the text that spells one.
///
/// Exactly one row: the sample provider starts with one order, and nothing here sets up a state
/// that makes more (the CLI's end-to-end run does, with the sample's state hook). A wider
/// cardinality would add a variant this provider cannot produce without one.
fn orders_csv_interaction() -> Value {
  json!({
    "description": "every order, as CSV",
    "transport": { "kind": "http", "mode": "passive" },
    "requires": [ { "component": "content/csv", "min-version": 1 } ],
    "content-types": { "response": { "body": "text/csv" } },
    "parts": {
      "request": { "method": { "shape": "equality", "example": "GET" },
                   "path": { "shape": "equality", "example": "/orders.csv" } },
      "response": {
        "status": { "shape": "equality", "example": 200 },
        "body": { "shape": "each-like", "min": 1, "max": 1,
                  "items": { "shape": "object", "members": {
                    "id": { "shape": "string", "example": "66" },
                    "status": { "shape": "string", "example": "PENDING" },
                    "items": { "shape": "regex", "pattern": "^[0-9]+$", "example": "1" } } } } } }
  })
}

fn http_get(addr: &str, path: &str) -> (u16, Vec<(String, String)>, String) {
  let mut stream = TcpStream::connect(addr).unwrap();
  stream.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
  write!(
    stream,
    "GET {path} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n"
  )
  .unwrap();
  let mut response = Vec::new();
  let _ = stream.read_to_end(&mut response);
  let text = String::from_utf8(response).unwrap();
  let (head, body) = text.split_once("\r\n\r\n").unwrap();
  let status = head
    .lines()
    .next()
    .unwrap()
    .split_whitespace()
    .nth(1)
    .unwrap()
    .parse()
    .unwrap();
  let headers = head
    .lines()
    .skip(1)
    .filter_map(|line| line.split_once(':'))
    .map(|(name, value)| (name.trim().to_ascii_lowercase(), value.trim().to_string()))
    .collect();
  (status, headers, body.to_string())
}

/// A consumer test against the mock: every variant served as real CSV, a contract written.
fn consumer_run(engine: &mut Engine) -> Value {
  let created = send(
    engine,
    "consumer-session/create",
    json!({ "config": { "consumer": { "name": "reporting" }, "provider": { "name": "order-service" },
                        "components": [declaration()] } }),
  );
  let session = created["ok"]["session"]
    .as_str()
    .unwrap_or_else(|| panic!("create: {created}"))
    .to_string();
  let added = send(
    engine,
    "consumer-session/add-interaction",
    json!({ "session": session, "interaction": orders_csv_interaction() }),
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
    json!({ "session": session, "transport": "http" }),
  );
  let endpoint = &started["ok"]["endpoint"];
  let addr = format!("{}:{}", endpoint["host"].as_str().unwrap(), endpoint["port"]);

  for variant in variants["ok"]["variants"].as_array().unwrap() {
    let served = send(
      engine,
      "consumer-session/serve-variant",
      json!({ "session": session, "handle": handle, "variant": variant["id"] }),
    );
    assert_eq!(served["ok"], json!({}));
    let (status, headers, body) = http_get(&addr, "/orders.csv");
    assert_eq!(status, 200);
    assert!(
      headers
        .iter()
        .any(|(name, value)| name == "content-type" && value == "text/csv"),
      "the mock labels the body as the declared type: {headers:?}"
    );
    // The consumer's own reading of it, as a CSV client would: a header row, then rows.
    let mut lines = body.lines();
    let header: Vec<&str> = lines.next().unwrap().split(',').collect();
    for column in ["id", "status", "items"] {
      assert!(header.contains(&column), "column '{column}' in {header:?}");
    }
    assert!(lines.count() >= 1, "at least one row: {body:?}");
  }

  let finalised = send(engine, "consumer-session/finalise", json!({ "session": session }));
  finalised["ok"]["contract"].clone()
}

#[test]
fn the_component_loads_and_declares_what_it_handles() {
  let mut engine = engine();
  let created = send(
    &mut engine,
    "consumer-session/create",
    json!({ "config": { "consumer": { "name": "c" }, "provider": { "name": "p" }, "components": [declaration()] } }),
  );
  assert!(created["ok"]["session"].is_string(), "{created}");
}

#[test]
fn a_consumer_test_serves_csv_and_records_a_contract_that_says_so() {
  let mut engine = engine();
  let contract = consumer_run(&mut engine);
  assert!(contract.is_object(), "a contract was written");
  let interaction = &contract["interactions"][0];
  assert_eq!(
    interaction["content-types"],
    json!({ "response": { "body": "text/csv" } })
  );
  for variant in interaction["selection"]["variants"].as_array().unwrap() {
    let body = &variant["parts"]["response"]["body"];
    assert_eq!(
      body["content-type"], "text/csv",
      "the evidence repeats the declaration"
    );
    assert!(
      body["content"].is_array(),
      "and is recorded as a readable document: {body}"
    );
  }
}

#[test]
fn the_same_component_verifies_the_contract_against_a_real_provider() {
  let mut engine = engine();
  let contract = consumer_run(&mut engine);
  let provider = order_service::start(order_service::Config {
    token: None,
    ..Default::default()
  })
  .unwrap();

  let verify = send(
    &mut engine,
    "verification/verify",
    json!({
      "source": { "kind": "inline", "contracts": [contract] },
      "target": { "transports": [ { "transport": "http", "options": { "base-url": provider.base_url() } } ],
                  "components": [declaration()] },
    }),
  );
  let stream = verify["ok"]["stream"]
    .as_str()
    .unwrap_or_else(|| panic!("verify: {verify}"))
    .to_string();
  let summary = drain(&mut engine, &stream);
  assert_eq!(summary["status"], "verified", "{summary}");
  assert!(
    summary["variants"]["verified"].as_u64().unwrap() >= 1,
    "{summary}"
  );
}

#[test]
fn a_provider_answering_json_where_csv_was_agreed_fails() {
  let mut engine = engine();
  let mut contract = consumer_run(&mut engine);
  // `GET /orders` answers the same orders as JSON: an array of objects that a JSON decoder would
  // happily match against the consumer's shape. Decoded as the consumer declared it, it is not CSV.
  for variant in contract["interactions"][0]["selection"]["variants"]
    .as_array_mut()
    .unwrap()
  {
    variant["parts"]["request"]["path"]["content"] = json!("/orders");
  }
  let provider = order_service::start(order_service::Config {
    token: None,
    ..Default::default()
  })
  .unwrap();
  let verify = send(
    &mut engine,
    "verification/verify",
    json!({
      "source": { "kind": "inline", "contracts": [contract] },
      "target": { "transports": [ { "transport": "http", "options": { "base-url": provider.base_url() } } ],
                  "components": [declaration()] },
    }),
  );
  let summary = drain(&mut engine, verify["ok"]["stream"].as_str().unwrap());
  assert_eq!(summary["status"], "failed", "{summary}");
  assert_eq!(summary["variants"]["verified"], 0, "{summary}");
}

#[test]
fn a_consumer_that_declares_csv_without_the_component_is_told_at_add_interaction() {
  let mut engine = engine();
  let created = send(
    &mut engine,
    "consumer-session/create",
    json!({ "config": { "consumer": { "name": "c" }, "provider": { "name": "p" } } }),
  );
  let added = send(
    &mut engine,
    "consumer-session/add-interaction",
    json!({ "session": created["ok"]["session"], "interaction": orders_csv_interaction() }),
  );
  assert_eq!(added["error"]["code"], "component-unavailable");
  assert_eq!(added["error"]["details"]["component"], "content/csv");
  assert_eq!(added["error"]["details"]["min-version"], 1);
  assert_eq!(added["error"]["details"]["loaders"], json!(["in-tree", "wasm"]));
}

#[test]
fn a_verification_that_needs_csv_without_the_component_fails_before_it_starts() {
  let mut engine = engine();
  let contract = consumer_run(&mut engine);
  let verify = send(
    &mut engine,
    "verification/verify",
    json!({
      "source": { "kind": "inline", "contracts": [contract] },
      "target": { "transports": [ { "transport": "http", "options": { "base-url": "http://127.0.0.1:1" } } ] },
    }),
  );
  assert_eq!(verify["error"]["code"], "component-unavailable", "{verify}");
  assert_eq!(verify["error"]["details"]["component"], "content/csv");
}

#[test]
fn a_digest_is_checked_before_anything_is_instantiated() {
  let mut engine = engine();
  let bytes = std::fs::read(csv_wasm()).unwrap();
  let pinned = format!("sha256:{:x}", Sha256::digest(&bytes));
  let mut good = declaration();
  good["source"]["digest"] = json!(pinned);
  let created = send(
    &mut engine,
    "consumer-session/create",
    json!({ "config": { "consumer": { "name": "c" }, "provider": { "name": "p" }, "components": [good] } }),
  );
  assert!(created["ok"]["session"].is_string(), "{created}");

  let mut bad = declaration();
  bad["source"]["digest"] = json!(format!("sha256:{}", "0".repeat(64)));
  let refused = send(
    &mut engine,
    "consumer-session/create",
    json!({ "config": { "consumer": { "name": "c" }, "provider": { "name": "p" }, "components": [bad] } }),
  );
  assert_eq!(refused["error"]["code"], "component-unavailable");
  assert_eq!(refused["error"]["details"]["error"]["code"], "digest-mismatch");
}

#[test]
fn a_component_that_answers_to_another_name_is_invalid() {
  let mut engine = engine();
  let mut renamed = declaration();
  renamed["name"] = json!("tsv");
  let refused = send(
    &mut engine,
    "consumer-session/create",
    json!({ "config": { "consumer": { "name": "c" }, "provider": { "name": "p" }, "components": [renamed] } }),
  );
  assert_eq!(refused["error"]["code"], "component-unavailable");
  assert_eq!(refused["error"]["details"]["reason"], "component-invalid");
}

#[test]
fn a_name_already_taken_is_a_conflict_found_at_resolution() {
  let mut engine = engine();
  let mut json_named = declaration();
  json_named["name"] = json!("json");
  let twice = json!([declaration(), declaration()]);
  for components in [json!([json_named]), twice] {
    let refused = send(
      &mut engine,
      "consumer-session/create",
      json!({ "config": { "consumer": { "name": "c" }, "provider": { "name": "p" }, "components": components } }),
    );
    assert_eq!(refused["error"]["code"], "component-unavailable", "{refused}");
    assert_eq!(refused["error"]["details"]["code"], "component-conflict");
  }
}

#[test]
fn a_source_no_loader_handles_names_the_loaders_there_are() {
  let mut engine = engine();
  let refused = send(
    &mut engine,
    "consumer-session/create",
    json!({ "config": { "consumer": { "name": "c" }, "provider": { "name": "p" },
                        "components": [ { "name": "csv", "source": { "kind": "subprocess", "reference": "janus-csv" } } ] } }),
  );
  assert_eq!(refused["error"]["code"], "component-unavailable");
  assert_eq!(refused["error"]["details"]["loaders"], json!(["in-tree", "wasm"]));
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
      if event["kind"] == "verification/interaction-result" && event["payload"]["status"] != "verified" {
        eprintln!("{}", event["payload"]);
      }
    }
  }
  panic!("the run never finished");
}
