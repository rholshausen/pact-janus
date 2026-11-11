//! Staged plan S3.5: a consumer session whose host asked for events tells each arrival's outcome on a
//! stream (engine-protocol spec §9.6), as a `consumer-session/exchange` event, while the test is still
//! running; `finalise` ends the stream. Driven through the protocol against the real HTTP transport.

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

fn engine() -> Engine {
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
  engine
}

/// One GET, answering its status.
fn get(addr: &str, path: &str) -> u16 {
  let mut stream = TcpStream::connect(addr).unwrap();
  stream.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
  let request = format!("GET {path} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n");
  stream.write_all(request.as_bytes()).unwrap();
  let mut response = Vec::new();
  let _ = stream.read_to_end(&mut response);
  String::from_utf8_lossy(&response)
    .split_whitespace()
    .nth(1)
    .and_then(|s| s.parse().ok())
    .unwrap()
}

/// Polls `stream` until it has given `count` events, or two seconds pass.
fn events(engine: &mut Engine, stream: &str, count: usize) -> Vec<Value> {
  let mut events = Vec::new();
  for _ in 0..20 {
    let polled = send(
      engine,
      "events/poll",
      json!({ "streams": [stream], "wait-ms": 100 }),
    );
    events.extend(
      polled["ok"]["events"]
        .as_array()
        .unwrap_or_else(|| panic!("{polled}"))
        .clone(),
    );
    if events.len() >= count {
      break;
    }
  }
  events
}

fn session(engine: &mut Engine, events: bool) -> (String, Option<String>, String, String) {
  let mut create =
    json!({ "config": { "consumer": { "name": "web-app" }, "provider": { "name": "orders" } } });
  if events {
    create["events"] = json!(true);
  }
  let created = send(engine, "consumer-session/create", create);
  let session = created["ok"]["session"].as_str().unwrap().to_string();
  let stream = created["ok"]["stream"].as_str().map(str::to_string);
  let added = send(
    engine,
    "consumer-session/add-interaction",
    json!({ "session": session, "interaction": {
      "description": "get an order",
      "transport": { "kind": "http", "mode": "passive" },
      "parts": {
        "request": { "method": { "shape": "equality", "example": "GET" }, "path": { "shape": "equality", "example": "/orders/42" } },
        "response": { "status": { "shape": "equality", "example": 200 } }
      }
    } }),
  );
  let handle = added["ok"]["handle"]
    .as_str()
    .unwrap_or_else(|| panic!("{added}"))
    .to_string();
  let selected = send(
    engine,
    "consumer-session/variants",
    json!({ "session": session, "handle": handle }),
  );
  assert!(selected["ok"]["variants"].is_array(), "{selected}");
  let started = send(
    engine,
    "consumer-session/start-transport",
    json!({ "session": session, "transport": "http" }),
  );
  let endpoint = &started["ok"]["endpoint"];
  let addr = format!(
    "{}:{}",
    endpoint["host"].as_str().unwrap(),
    endpoint["port"].as_u64().unwrap()
  );
  (session, stream, handle, addr)
}

#[test]
fn each_arrival_is_told_on_the_sessions_stream_while_the_test_runs_and_finalise_ends_it() {
  let mut engine = engine();
  let (session, stream, handle, addr) = session(&mut engine, true);
  let stream = stream.expect("create answers with a stream when asked");

  // Nothing armed yet: refused, and told.
  assert_eq!(get(&addr, "/orders/42"), 500);
  let served = send(
    &mut engine,
    "consumer-session/serve-variant",
    json!({ "session": session, "handle": handle, "variant": "base" }),
  );
  assert!(served["ok"].is_object(), "{served}");
  assert_eq!(get(&addr, "/orders/7"), 500);
  assert_eq!(get(&addr, "/orders/42"), 200);

  // Before the test is reported: the events are there to surface while it runs.
  let told = events(&mut engine, &stream, 3);
  let kinds: Vec<&str> = told.iter().map(|e| e["kind"].as_str().unwrap()).collect();
  assert_eq!(kinds, vec!["consumer-session/exchange"; 3], "{told:?}");
  assert_eq!(told[0]["payload"]["status"], "unmatched");
  assert_eq!(told[1]["payload"]["status"], "mismatched");
  assert_eq!(told[1]["payload"]["handle"], json!(handle));
  assert_eq!(told[1]["payload"]["variant"], "base");
  assert_eq!(told[1]["payload"]["mismatches"][0]["path"], "$.request.path");
  assert_eq!(told[2]["payload"]["status"], "matched");
  assert!(told[2]["payload"].get("mismatches").is_none(), "{told:?}");
  assert!(told.iter().all(|e| e["last"] == false));

  send(
    &mut engine,
    "consumer-session/report-variant",
    json!({ "session": session, "handle": handle, "variant": "base", "status": "passed" }),
  );
  send(
    &mut engine,
    "consumer-session/finalise",
    json!({ "session": session }),
  );
  let finished = events(&mut engine, &stream, 1);
  assert_eq!(finished.len(), 1, "{finished:?}");
  assert_eq!(finished[0]["kind"], "consumer-session/finished");
  assert_eq!(finished[0]["last"], true);
  let after = send(&mut engine, "events/poll", json!({ "streams": [stream] }));
  assert_eq!(after["error"]["code"], "stream-not-found", "{after}");
}

#[test]
fn a_session_that_did_not_ask_has_no_stream() {
  let mut engine = engine();
  let (session, stream, _, addr) = session(&mut engine, false);
  assert_eq!(stream, None);
  assert_eq!(get(&addr, "/orders/42"), 500);
  let finalised = send(
    &mut engine,
    "consumer-session/finalise",
    json!({ "session": session }),
  );
  assert!(finalised["ok"].is_object(), "{finalised}");
}

/// A host that asked for events and never drains them cannot hold up `finalise`: the exchange waiting
/// for room is released.
#[test]
fn finalise_ends_a_full_stream_without_waiting_for_the_host() {
  let mut engine = engine();
  let (session, stream, _, addr) = session(&mut engine, true);
  let stream = stream.expect("a stream");
  // More arrivals than the stream holds; the last waits for room until finalise releases it.
  let client = std::thread::spawn(move || {
    for _ in 0..1030 {
      let _ = std::panic::catch_unwind(|| get(&addr, "/orders/42"));
    }
  });
  std::thread::sleep(Duration::from_secs(2));
  let finalised = send(
    &mut engine,
    "consumer-session/finalise",
    json!({ "session": session }),
  );
  assert!(finalised["ok"].is_object(), "{finalised}");
  let _ = client.join();
  let mut last = Value::Null;
  loop {
    let polled = send(
      &mut engine,
      "events/poll",
      json!({ "streams": [stream], "max": 1024 }),
    );
    let Some(events) = polled["ok"]["events"].as_array() else {
      break;
    };
    if let Some(event) = events.last() {
      last = event.clone();
    }
    if last["last"] == true {
      break;
    }
  }
  assert_eq!(last["kind"], "consumer-session/finished", "{last}");
}
