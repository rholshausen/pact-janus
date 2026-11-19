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

/// The CSV component as a contract and a verification summary record it (ADR 0097): its handshake's
/// name and version, and the digest of the file it was loaded from, never the file's path.
fn recorded() -> Value {
  let bytes = std::fs::read(csv_wasm()).expect("the component was built");
  json!([ { "name": "csv", "version": "1.0.0",
            "source": { "kind": "file", "digest": pact_janus_component_host::oci::sha256(&bytes) } } ])
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
  engine.register_background(Arc::new(pact_janus_kernel::protocol::Threads));
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
  consumer_run_with(engine, orders_csv_interaction())
}

fn consumer_run_with(engine: &mut Engine, interaction: Value) -> Value {
  // The columns as the interaction declares them, in its order (ADR 0037).
  let declared: Vec<String> = interaction
    .pointer("/parts/response/body/items/members")
    .and_then(Value::as_object)
    .map(|members| members.keys().cloned().collect())
    .unwrap_or_default();
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
    json!({ "session": session, "interaction": interaction }),
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
    // In the order the interaction declares them, not by name: a consumer may read by position.
    let in_declared_order: Vec<&str> = declared
      .iter()
      .map(String::as_str)
      .filter(|column| header.contains(column))
      .collect();
    assert_eq!(header, in_declared_order, "the columns as declared, in {body:?}");
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
    let row: Vec<&String> = body["content"][0].as_object().unwrap().keys().collect();
    assert_eq!(
      row,
      ["id", "status", "items"],
      "each row in the order declared: {body}"
    );
  }
  let declared: Vec<&String> = interaction["parts"]["response"]["body"]["items"]["members"]
    .as_object()
    .unwrap()
    .keys()
    .collect();
  assert_eq!(
    declared,
    ["id", "status", "items"],
    "the contract records the columns in the order the consumer declared them"
  );
  // Which bytes decoded the body (ADR 0097, S5.1a's exit).
  assert_eq!(
    contract["metadata"]["components"],
    recorded(),
    "{}",
    contract["metadata"]
  );
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
  assert_eq!(summary["components"], recorded(), "{summary}");
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
  let hex: String = Sha256::digest(&bytes)
    .iter()
    .map(|b| format!("{b:02x}"))
    .collect();
  let pinned = format!("sha256:{hex}");
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

// ---------------------------------------------------------------------------------------------
// Substitutions (staged plan S5.4, ADR 0100)
// ---------------------------------------------------------------------------------------------

/// Orders as CSV, with `items` and `channel` as given and up to `max` rows.
fn orders_with(items: Value, channel: Value, max: u64) -> Value {
  let mut interaction = orders_csv_interaction();
  interaction["parts"]["response"]["body"] = json!({ "shape": "each-like", "min": 1, "max": max,
    "items": { "shape": "object", "members": {
      "id": { "shape": "string", "example": "66" },
      "status": { "shape": "string", "example": "PENDING" },
      "items": items,
      "channel": channel } } });
  interaction
}

/// Verify `contract` against the sample provider — one order, so one CSV row — asking for every
/// executed plan, and return (summary, every event's text).
fn verify_against_sample(engine: &mut Engine, contract: &Value) -> (Value, String) {
  let provider = order_service::start(order_service::Config {
    token: None,
    ..Default::default()
  })
  .unwrap();
  let started = send(
    engine,
    "verification/verify",
    json!({
      "source": { "kind": "inline", "contracts": [contract] },
      "target": { "transports": [ { "transport": "http", "options": { "base-url": provider.base_url() } } ],
                  "components": [declaration()] },
      "options": { "executed-plan": "always" },
    }),
  );
  let stream = started["ok"]["stream"]
    .as_str()
    .unwrap_or_else(|| panic!("verify: {started}"))
    .to_string();
  let mut text = String::new();
  for _ in 0..100 {
    let polled = send(
      engine,
      "events/poll",
      json!({ "streams": [stream], "wait-ms": 5_000 }),
    );
    for event in polled["ok"]["events"].as_array().unwrap() {
      text.push_str(&event.to_string());
      if event["last"] == json!(true) {
        return (event["payload"].clone(), text);
      }
    }
  }
  panic!("the run never finished");
}

/// What 8.1 could not write: `items: integer` of a CSV column. The engine's plan says
/// `match:integer`, and CSV's `"1"` is a string; the component's handshake says that under `text/csv`,
/// `integer` is `csv:integer`, text that spells an integer, and the contract verifies against a real
/// provider. So does `equality` of a number, which a CSV field holds as the text that spells it.
#[test]
fn a_consumer_can_say_integer_of_a_csv_column_because_the_component_says_what_it_means() {
  let mut engine = engine();
  let mut interaction = orders_with(
    json!({ "shape": "integer", "example": 1 }),
    json!({ "shape": "string", "example": "web" }),
    1,
  );
  interaction["parts"]["response"]["body"]["items"]["members"]["id"] =
    json!({ "shape": "equality", "example": 66 });
  let contract = consumer_run_with(&mut engine, interaction);
  let (summary, events) = verify_against_sample(&mut engine, &contract);
  assert_eq!(summary["status"], "verified", "{summary}\n{events}");
  assert!(
    events.contains("csv:integer") && events.contains("csv:equality"),
    "the component's actions check the operators whose meaning CSV changes: {events}"
  );
  assert!(!events.contains("match:integer"), "{events}");
  assert!(
    events.contains("match:string"),
    "an operator CSV does not change is the engine's: {events}"
  );
  assert!(
    events.contains("decoded as text/csv by csv"),
    "the plan says why: {events}"
  );
}

/// Phase 9 finding 21, closed. The engine compiles a CSV body as it compiles any other, pinning each
/// variant's cardinality to the size it demonstrated (`expect:count 3`), and only the value
/// operators' checks are the component's. Against a provider with one order, the variant that
/// demonstrated three rows fails, as it does where nothing substitutes an operator. Under ADR 0022's
/// fragments it passed, because the fragment checked every variant against the unpinned range.
#[test]
fn a_csv_body_is_pinned_to_its_variant() {
  let items = json!({ "shape": "integer", "example": 1 });
  let mut engine = engine();
  let contract = consumer_run_with(
    &mut engine,
    orders_with(items, json!({ "shape": "string", "example": "web" }), 3),
  );
  let variants = contract["interactions"][0]["selection"]["variants"]
    .as_array()
    .unwrap()
    .len();
  assert!(variants >= 2, "a cardinality range has more than one variant");

  let (summary, events) = verify_against_sample(&mut engine, &contract);
  assert_eq!(summary["status"], "failed", "{summary}");
  assert!(events.contains("expect:count"), "{events}");
  assert!(events.contains("csv:integer"), "{events}");
}

// ---------------------------------------------------------------------------------------------
// Content that is not JSON (staged plan S5.3, ADR 0101)
// ---------------------------------------------------------------------------------------------

/// Serve every variant of `interaction` from a consumer session and return each one's body as the
/// mock served it, by variant id, then finalise.
fn served_bodies(engine: &mut Engine, interaction: Value) -> Vec<(String, String)> {
  let created = send(
    engine,
    "consumer-session/create",
    json!({ "config": { "consumer": { "name": "reporting" }, "provider": { "name": "order-service" },
                        "components": [declaration()] } }),
  );
  let session = created["ok"]["session"].clone();
  let added = send(
    engine,
    "consumer-session/add-interaction",
    json!({ "session": session, "interaction": interaction }),
  );
  let handle = added["ok"]["handle"].clone();
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
  let mut bodies = Vec::new();
  for variant in variants["ok"]["variants"].as_array().unwrap() {
    send(
      engine,
      "consumer-session/serve-variant",
      json!({ "session": session, "handle": handle, "variant": variant["id"] }),
    );
    let (status, _, body) = http_get(&addr, "/orders.csv");
    assert_eq!(status, 200, "{body}");
    bodies.push((variant["id"].as_str().unwrap().to_string(), body));
  }
  send(engine, "consumer-session/finalise", json!({ "session": session }));
  bodies
}

/// Phase 9 finding 12, closed. A list that may be empty has an empty variant (ADR 0079), whose
/// document has no rows and so names no columns. `content/encode` is handed the slot's shape, and the
/// component writes the header from it: the mock serves a body the component's own `decode` reads back
/// as no rows, where it served an empty body that `decode` refuses.
#[test]
fn an_empty_csv_variant_is_served_as_the_header_its_shape_declares() {
  let mut engine = engine();
  let mut interaction = orders_csv_interaction();
  interaction["parts"]["response"]["body"]["min"] = json!(0);
  let bodies = served_bodies(&mut engine, interaction);
  let empty: Vec<&String> = bodies
    .iter()
    .filter(|(_, body)| body.lines().count() == 1)
    .map(|(_, body)| body)
    .collect();
  assert_eq!(
    empty,
    [&"id,status,items\r\n".to_string()],
    "one variant has no rows, served as the header alone: {bodies:?}"
  );
  assert!(
    bodies.iter().any(|(_, body)| body.lines().count() == 2),
    "and one has a row: {bodies:?}"
  );
}

/// A provider that answers every request with `body`, labelled `content_type`, until the test ends.
fn fixed_provider(content_type: &'static str, body: &'static str) -> String {
  let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
  let addr = listener.local_addr().unwrap();
  std::thread::spawn(move || {
    for stream in listener.incoming() {
      let Ok(mut stream) = stream else { continue };
      let mut request = [0u8; 4096];
      let _ = stream.read(&mut request);
      let _ = write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
      );
    }
  });
  format!("http://{addr}")
}

/// Verify `contract` against `base_url` and return (summary, every event).
fn verify_events(engine: &mut Engine, contract: &Value, base_url: &str) -> (Value, Vec<Value>) {
  let started = send(
    engine,
    "verification/verify",
    json!({
      "source": { "kind": "inline", "contracts": [contract] },
      "target": { "transports": [ { "transport": "http", "options": { "base-url": base_url } } ],
                  "components": [declaration()] },
      "options": { "executed-plan": "on-failure" },
    }),
  );
  let stream = started["ok"]["stream"]
    .as_str()
    .unwrap_or_else(|| panic!("verify: {started}"))
    .to_string();
  let mut events = Vec::new();
  for _ in 0..100 {
    let polled = send(
      engine,
      "events/poll",
      json!({ "streams": [stream], "wait-ms": 5_000 }),
    );
    for event in polled["ok"]["events"].as_array().unwrap() {
      events.push(event.clone());
      if event["last"] == json!(true) {
        return (event["payload"].clone(), events);
      }
    }
  }
  panic!("the run never finished");
}

fn of_kind<'a>(events: &'a [Value], kind: &str) -> Vec<&'a Value> {
  events
    .iter()
    .filter(|event| event["kind"] == kind)
    .map(|event| &event["payload"])
    .collect()
}

/// Phase 9 finding 14, closed. A body the component cannot decode — a ragged row — is reported at the
/// body with the component's own message, once, and no shape is applied to a guess: no "expected at
/// least 1 item" for a body that has no document.
#[test]
fn a_csv_reply_that_cannot_be_decoded_is_reported_at_the_body_in_the_components_words() {
  let mut engine = engine();
  let contract = consumer_run(&mut engine);
  let provider = fixed_provider("text/csv", "id,status,items\r\n66,PENDING\r\n");
  let (summary, events) = verify_events(&mut engine, &contract, &provider);
  assert_eq!(summary["status"], "failed", "{summary}");
  for result in of_kind(&events, "verification/interaction-result") {
    let mismatches = result["mismatches"]
      .as_array()
      .unwrap_or_else(|| panic!("{result}"));
    assert_eq!(mismatches.len(), 1, "one mismatch, at the body: {result}");
    assert_eq!(mismatches[0]["path"], "$.response.body");
    let message = mismatches[0]["message"].as_str().unwrap();
    assert!(
      message.starts_with("could not be decoded as 'text/csv': ") && message.contains("fields"),
      "the component says what is wrong with the octets: {message}"
    );
  }
}

/// Phase 9 finding 13. A provider answering JSON where CSV was declared is still decoded as declared
/// (contract-file spec §5.5), and a warning before the result says what it was labelled, so the
/// size mismatch that follows is read for what it is.
#[test]
fn a_reply_labelled_json_where_csv_was_declared_is_said_to_be() {
  let mut engine = engine();
  let contract = consumer_run(&mut engine);
  let provider = fixed_provider(
    "application/json",
    r#"[{"id":"66","status":"PENDING","items":"1"}]"#,
  );
  let (summary, events) = verify_events(&mut engine, &contract, &provider);
  assert_eq!(summary["status"], "failed", "{summary}");
  let warnings = of_kind(&events, "verification/warning");
  let differs: Vec<&&Value> = warnings
    .iter()
    .filter(|warning| warning["code"] == "content-type-differs")
    .collect();
  assert!(!differs.is_empty(), "{warnings:?}");
  assert_eq!(differs[0]["target"], "$.response.body");
  assert_eq!(differs[0]["declared"], "text/csv");
  assert_eq!(differs[0]["labelled"], "application/json");
  // Before the result it qualifies.
  let position = |code: &str| events.iter().position(|event| event["payload"]["code"] == code);
  let result = events
    .iter()
    .position(|event| event["kind"] == "verification/interaction-result")
    .unwrap();
  assert!(position("content-type-differs").unwrap() < result);
}

/// Spec §6.4: a loss a decode reports for this value is surfaced, as a `content-degraded` warning
/// located in the interaction, and changes no verdict. The sample provider's `items` column is a
/// number, which CSV can only carry as text.
#[test]
fn a_degradation_a_decode_reports_is_a_warning_located_in_the_reply() {
  let mut engine = engine();
  let contract = consumer_run(&mut engine);
  let provider = order_service::start(order_service::Config {
    token: None,
    ..Default::default()
  })
  .unwrap();
  let (summary, events) = verify_events(&mut engine, &contract, provider.base_url().as_str());
  assert_eq!(summary["status"], "verified", "{summary}");
  let warnings = of_kind(&events, "verification/warning");
  let degraded = warnings
    .iter()
    .find(|warning| warning["code"] == "content-degraded" && warning["target"] == "$.response.body[*].items")
    .unwrap_or_else(|| panic!("{warnings:?}"));
  assert_eq!(degraded["content-type"], "text/csv");
  assert_eq!(degraded["degradation"]["code"], "numeric-lexical");
  assert!(
    degraded["message"].as_str().unwrap().contains("numeric-lexical"),
    "{degraded}"
  );
}
