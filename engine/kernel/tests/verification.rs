//! Plan task 5.1: `verification/verify` end to end — the same engine, driven the other way.
//! A contract's recorded variants are replayed at a real HTTP provider (a scripted stub on a real
//! socket, same hand-rolled style as `consumer_flow.rs`) and every outcome arrives as events on
//! the run's stream (engine-protocol spec §8.3, §9; variant-semantics spec §5).
//!
//! The first test is the one that matters: a contract *this engine wrote* from a consumer session
//! is verified by *this engine* against a provider, variant by variant, in recorded order. That
//! round trip is what "the verifier is the same engine" has to mean to be worth claiming, and it
//! is also M3's shape without the hooks 5.3 adds.

use pact_janus_component_http::HttpTransport;
use pact_janus_component_json::JsonContent;
use pact_janus_kernel::component::TransportComponent;
use pact_janus_kernel::protocol::Engine;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

// ---------------------------------------------------------------------------------------------
// Harness
// ---------------------------------------------------------------------------------------------

fn send(engine: &mut Engine, id: &str, op: &str, body: Value) -> Value {
  let request = json!({ "type": "request", "id": id, "op": op, "body": body });
  let bytes = engine.dispatch(&serde_json::to_vec(&request).expect("a json! literal always serializes"));
  serde_json::from_slice(&bytes).expect("Engine::dispatch always returns valid JSON")
}

fn engine_with_real_components() -> Engine {
  let mut transports: HashMap<String, Arc<dyn TransportComponent>> = HashMap::new();
  transports.insert("http".to_string(), Arc::new(HttpTransport::new()));
  let mut engine = Engine::with_components(transports, Some(Arc::new(JsonContent::new())));
  send(
    &mut engine,
    "r-1",
    "engine/hello",
    json!({ "protocol-versions": [1], "host": { "name": "janus-test", "version": "0.0.0" }, "capabilities": {} }),
  );
  engine
}

/// Long-poll the run's stream until its terminal event arrives (spec §9.2: termination is
/// structural, so this loop reads `last` and never a kind vocabulary), asserting the `seq`
/// contract as it goes.
fn drain_run(engine: &mut Engine, stream: &str) -> Vec<Value> {
  let mut events: Vec<Value> = Vec::new();
  for poll in 0..100 {
    let response = send(
      engine,
      &format!("p-{poll}"),
      "events/poll",
      json!({ "streams": [stream], "wait-ms": 5_000 }),
    );
    let polled = response["ok"]["events"]
      .as_array()
      .unwrap_or_else(|| panic!("events/poll failed: {response}"))
      .clone();
    events.extend(polled);
    if events.last().is_some_and(|event| event["last"] == json!(true)) {
      break;
    }
  }
  let last = events.last().expect("a run always emits at least one event");
  assert_eq!(last["last"], json!(true), "the run terminated structurally");
  for (i, event) in events.iter().enumerate() {
    assert_eq!(
      event["seq"],
      json!(i as u64 + 1),
      "seq starts at 1 and never gaps"
    );
    assert_eq!(event["stream"], json!(stream));
  }
  events
}

fn kinds(events: &[Value]) -> Vec<&str> {
  events.iter().map(|e| e["kind"].as_str().unwrap()).collect()
}

fn results(events: &[Value]) -> Vec<&Value> {
  events
    .iter()
    .filter(|e| e["kind"] == json!("verification/interaction-result"))
    .map(|e| &e["payload"])
    .collect()
}

fn summary(events: &[Value]) -> &Value {
  &events.last().expect("a terminal event")["payload"]
}

/// What one scripted response the stub provider will answer with.
struct Canned {
  status: u16,
  body: &'static str,
}

fn canned(status: u16, body: &'static str) -> Canned {
  Canned { status, body }
}

/// A provider stub answering a scripted sequence of responses, one per request, and reporting each
/// request line it saw. Deliberately scripted rather than routed: two variants of one interaction
/// replay the *same* request and expect different responses, which is exactly the situation
/// provider states exist for — until 5.2/5.3 wire those up, the sequence stands in for them, and
/// it doubles as the assertion that variants are replayed in recorded order.
fn stub_provider(responses: Vec<Canned>) -> (String, Receiver<String>) {
  let listener = TcpListener::bind("127.0.0.1:0").expect("an OS-assigned port always binds");
  let base_url = format!("http://{}", listener.local_addr().unwrap());
  let (tx, rx) = mpsc::channel();
  thread::spawn(move || {
    for canned in responses {
      let Ok((stream, _)) = listener.accept() else {
        return;
      };
      stream.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
      let mut reader = BufReader::new(stream);
      let mut request_line = String::new();
      if reader.read_line(&mut request_line).is_err() {
        return;
      }
      let mut length = 0usize;
      loop {
        let mut line = String::new();
        if reader.read_line(&mut line).is_err() {
          return;
        }
        let line = line.trim_end();
        if line.is_empty() {
          break;
        }
        if let Some((name, value)) = line.split_once(':')
          && name.eq_ignore_ascii_case("content-length")
        {
          length = value.trim().parse().unwrap_or(0);
        }
      }
      let mut body = vec![0u8; length];
      let _ = reader.read_exact(&mut body);

      let reason = if canned.status == 200 { "OK" } else { "Error" };
      let response = format!(
        "HTTP/1.1 {} {reason}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
        canned.status,
        canned.body.len(),
        canned.body
      );
      let stream = reader.get_mut();
      let _ = stream.write_all(response.as_bytes());
      let _ = stream.flush();
      let _ = tx.send(request_line.trim_end().to_string());
    }
  });
  (base_url, rx)
}

fn verify(engine: &mut Engine, contract: Value, base_url: &str) -> Value {
  send(
    engine,
    "v-1",
    "verification/verify",
    json!({
      "source": { "kind": "inline", "contracts": [contract] },
      "target": { "transports": [ { "transport": "http", "options": { "base-url": base_url } } ] },
    }),
  )
}

// ---------------------------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------------------------

/// A hand-written Janus contract: one interaction, one recorded variant, a JSON body whose shape
/// is `{ id: string }`. Small enough to read, and every later fixture is a tweak of it.
fn contract_with_one_variant() -> Value {
  json!({
    "$format": "janus-contract/1",
    "consumer": { "name": "web-app" },
    "provider": { "name": "order-api" },
    "interactions": [{
      "description": "a request for an order",
      "transport": { "kind": "http", "mode": "passive" },
      "parts": {
        "request": { "method": { "shape": "equality", "example": "GET" },
                     "path": { "shape": "equality", "example": "/orders/66" } },
        "response": { "status": { "shape": "equality", "example": 200 },
                      "body": { "shape": "object",
                                "members": { "id": { "shape": "string", "example": "o-1" } } } }
      },
      "selection": {
        "variants": [{
          "id": "base",
          "origin": "base",
          "assignment": [],
          "parts": {
            "request": { "method": { "content": "GET" }, "path": { "content": "/orders/66" } },
            "response": { "status": { "content": 200 }, "body": { "content": { "id": "o-1" } } }
          }
        }],
        "report": { "space": { "size": 1, "dimensions": 0 } }
      }
    }]
  })
}

// ---------------------------------------------------------------------------------------------
// The round trip
// ---------------------------------------------------------------------------------------------

/// The consumer side of `consumer_flow.rs`, condensed: run the interaction through the engine's
/// own mock with a real HTTP client and return the contract the engine wrote. Two variants, since
/// the point of the round trip is that the verifier replays *each* of them.
fn contract_written_by_a_consumer_session(engine: &mut Engine) -> Value {
  let interaction = json!({
    "description": "a request for an order",
    "transport": { "kind": "http", "mode": "passive" },
    "parts": {
      "request": { "method": { "shape": "equality", "example": "GET" },
                   "path": { "shape": "equality", "example": "/orders/66" } },
      "response": { "status": { "shape": "equality", "example": 200 },
                    "body": { "shape": "object", "members": {
                      "id": { "shape": "string", "example": "o-1" },
                      "shippedAt": { "shape": "optional",
                                     "of": { "shape": "string", "example": "2026-07-30" } } } } }
    }
  });

  let create = send(
    engine,
    "c-1",
    "consumer-session/create",
    json!({ "config": { "consumer": { "name": "web-app" }, "provider": { "name": "order-api" } } }),
  );
  let session = create["ok"]["session"].as_str().unwrap().to_string();
  let added = send(
    engine,
    "c-2",
    "consumer-session/add-interaction",
    json!({ "session": session, "interaction": interaction }),
  );
  let handle = added["ok"]["handle"].as_str().unwrap().to_string();
  let variants = send(
    engine,
    "c-3",
    "consumer-session/variants",
    json!({ "session": session, "handle": handle }),
  );
  let variant_ids: Vec<String> = variants["ok"]["variants"]
    .as_array()
    .expect("a selection")
    .iter()
    .map(|v| v["id"].as_str().unwrap().to_string())
    .collect();
  assert_eq!(
    variant_ids.len(),
    2,
    "one optional member: base plus the boundary where it is absent ({variant_ids:?})"
  );

  let started = send(
    engine,
    "c-4",
    "consumer-session/start-transport",
    json!({ "session": session, "transport": "http" }),
  );
  let endpoint = &started["ok"]["endpoint"];
  let addr = format!(
    "{}:{}",
    endpoint["host"].as_str().expect("endpoint.host"),
    endpoint["port"].as_u64().expect("endpoint.port")
  );

  for (i, variant) in variant_ids.iter().enumerate() {
    send(
      engine,
      &format!("c-1{i}"),
      "consumer-session/serve-variant",
      json!({ "session": session, "handle": handle, "variant": variant }),
    );
    let (status, _body) = http_get(&addr, "/orders/66");
    assert_eq!(status, 200, "the mock served variant '{variant}'");
  }

  let finalised = send(
    engine,
    "c-99",
    "consumer-session/finalise",
    json!({ "session": session }),
  );
  let contract = finalised["ok"]["contract"].clone();
  assert!(contract.is_object(), "every variant verified: {finalised}");
  contract
}

/// One GET against the engine's mock, read to EOF — `consumer_flow.rs`'s helper, kept local
/// rather than shared through a test-support crate for one caller.
fn http_get(addr: &str, path: &str) -> (u16, Value) {
  let mut stream = TcpStream::connect(addr).expect("the mock is listening");
  stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
  let request = format!("GET {path} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n");
  stream.write_all(request.as_bytes()).unwrap();
  let mut response = Vec::new();
  let _ = stream.read_to_end(&mut response);
  let split = response
    .windows(4)
    .position(|w| w == b"\r\n\r\n")
    .expect("a separator");
  let head = std::str::from_utf8(&response[..split]).expect("ASCII headers");
  let status: u16 = head
    .lines()
    .next()
    .and_then(|line| line.split_whitespace().nth(1))
    .expect("a status code")
    .parse()
    .expect("a numeric status code");
  let body = &response[split + 4..];
  let json = if body.is_empty() {
    Value::Null
  } else {
    serde_json::from_slice(body).unwrap_or(Value::Null)
  };
  (status, json)
}

#[test]
fn a_contract_this_engine_wrote_verifies_against_a_provider_variant_by_variant() {
  let mut engine = engine_with_real_components();
  let contract = contract_written_by_a_consumer_session(&mut engine);

  let recorded: Vec<String> = contract["interactions"][0]["selection"]["variants"]
    .as_array()
    .expect("recorded variants")
    .iter()
    .map(|v| v["id"].as_str().unwrap().to_string())
    .collect();
  assert_eq!(recorded.len(), 2, "both variants were exercised and recorded");

  // The provider answers each replay in recorded order: the base variant's body carries
  // `shippedAt`, the boundary variant's does not. A provider that returned the base body to both
  // would fail the second variant, which is the whole point of pinning the shape per variant.
  let (base_url, requests) = stub_provider(vec![
    canned(200, r#"{"id":"o-1","shippedAt":"2026-07-30"}"#),
    canned(200, r#"{"id":"o-1"}"#),
  ]);

  let started = verify(&mut engine, contract, &base_url);
  let stream = started["ok"]["stream"].as_str().expect("a stream id").to_string();
  assert!(
    started["ok"]["session"]
      .as_str()
      .is_some_and(|s| s.starts_with("vs-")),
    "verify answers with its own session: {started}"
  );

  let events = drain_run(&mut engine, &stream);
  assert_eq!(
    kinds(&events),
    vec![
      "verification/started",
      "verification/interaction-started",
      "verification/interaction-result",
      "verification/interaction-started",
      "verification/interaction-result",
      "verification/finished",
    ]
  );

  for result in results(&events) {
    assert_eq!(
      result["status"],
      json!("verified"),
      "every variant verified: {result}"
    );
  }
  let replayed: Vec<String> = (0..2)
    .map(|_| {
      requests
        .recv_timeout(Duration::from_secs(10))
        .expect("a replayed request")
    })
    .collect();
  assert_eq!(
    replayed,
    vec!["GET /orders/66 HTTP/1.1", "GET /orders/66 HTTP/1.1"],
    "replay is by example: the provider saw the bytes the consumer sent"
  );

  let summary = summary(&events);
  assert_eq!(summary["status"], json!("verified"));
  assert_eq!(
    summary["variants"],
    json!({ "total": 2, "verified": 2, "failed": 0 })
  );
  assert_eq!(summary["failures"], json!([]));

  // Spec §7.1/§9.1: the run ended itself at the terminal event, and delivering it spent the
  // stream id — so the host cannot poll a finished run, and there is nothing left to release.
  let stale = send(&mut engine, "p-x", "events/poll", json!({ "streams": [stream] }));
  assert_eq!(stale["error"]["code"], "stream-not-found");
}

// ---------------------------------------------------------------------------------------------
// Failure reporting
// ---------------------------------------------------------------------------------------------

#[test]
fn a_provider_that_answers_differently_fails_that_variant_with_its_mismatches() {
  let mut engine = engine_with_real_components();
  let (base_url, _requests) = stub_provider(vec![canned(200, r#"{"id":42}"#)]);

  let started = verify(&mut engine, contract_with_one_variant(), &base_url);
  let stream = started["ok"]["stream"].as_str().expect("a stream id").to_string();
  let events = drain_run(&mut engine, &stream);

  let results = results(&events);
  assert_eq!(results.len(), 1);
  assert_eq!(results[0]["status"], json!("failed"));
  let mismatches = results[0]["mismatches"].as_array().expect("mismatches");
  assert!(
    !mismatches.is_empty(),
    "a failed variant names what did not match"
  );
  assert!(
    mismatches
      .iter()
      .any(|m| m["path"].as_str().is_some_and(|p| p.contains("id"))),
    "the mismatch is attributed to the member that differed: {mismatches:?}"
  );

  let summary = summary(&events);
  assert_eq!(summary["status"], json!("failed"));
  assert_eq!(
    summary["variants"],
    json!({ "total": 1, "verified": 0, "failed": 1 })
  );
  assert_eq!(
    summary["failures"].as_array().map(Vec::len),
    Some(1),
    "the terminal event stands alone: a host that read only it still knows what failed"
  );
}

#[test]
fn a_wrong_status_fails_the_variant_too() {
  let mut engine = engine_with_real_components();
  let (base_url, _requests) = stub_provider(vec![canned(500, r#"{"id":"o-1"}"#)]);

  let started = verify(&mut engine, contract_with_one_variant(), &base_url);
  let stream = started["ok"]["stream"].as_str().unwrap().to_string();
  let events = drain_run(&mut engine, &stream);

  let results = results(&events);
  assert_eq!(results[0]["status"], json!("failed"));
  assert!(
    results[0]["mismatches"]
      .as_array()
      .expect("mismatches")
      .iter()
      .any(|m| m["path"].as_str().is_some_and(|p| p.contains("status"))),
    "a 500 where 200 was recorded is a mismatch, not a transport error: {results:?}"
  );
}

#[test]
fn an_unreachable_provider_fails_the_variant_with_the_components_own_error() {
  let mut engine = engine_with_real_components();
  // Nothing is listening: port 1 on loopback is refused immediately on every platform CI runs on.
  let started = verify(&mut engine, contract_with_one_variant(), "http://127.0.0.1:1");
  let stream = started["ok"]["stream"].as_str().unwrap().to_string();
  let events = drain_run(&mut engine, &stream);

  let results = results(&events);
  assert_eq!(results[0]["status"], json!("failed"));
  assert_eq!(results[0]["error"]["code"], json!("component-failed"));
  assert_eq!(results[0]["error"]["component"], json!("transport/http"));
  assert!(
    results[0]["error"]["error"].is_object(),
    "the component's own error document is passed through, not translated: {results:?}"
  );
  assert_eq!(summary(&events)["status"], json!("failed"));
}

#[test]
fn an_interaction_whose_transport_the_run_cannot_speak_is_named_never_skipped() {
  let mut engine = engine_with_real_components();
  let mut contract = contract_with_one_variant();
  contract["interactions"][0]["transport"] = json!({ "kind": "amqp" });

  let started = verify(&mut engine, contract, "http://127.0.0.1:1");
  let stream = started["ok"]["stream"].as_str().unwrap().to_string();
  let events = drain_run(&mut engine, &stream);

  let results = results(&events);
  assert_eq!(results.len(), 1, "one result per interaction × variant, always");
  assert_eq!(results[0]["status"], json!("failed"));
  assert_eq!(results[0]["error"]["code"], json!("component-unavailable"));
  assert_eq!(results[0]["error"]["component"], json!("transport/amqp"));
}

// ---------------------------------------------------------------------------------------------
// Refusals before the run starts
// ---------------------------------------------------------------------------------------------

#[test]
fn a_v3_pact_is_refused_by_name_rather_than_misparsed() {
  let mut engine = engine_with_real_components();
  let pact = json!({
    "consumer": { "name": "web-app" },
    "provider": { "name": "order-api" },
    "interactions": [],
    "metadata": { "pactSpecification": { "version": "3.0.0" } }
  });
  let refused = verify(&mut engine, pact, "http://127.0.0.1:1");
  assert_eq!(refused["error"]["code"], "contract-version-unsupported");
  assert_eq!(refused["error"]["category"], "document");
  assert_eq!(refused["error"]["details"]["found"], "pactSpecification 3.0.0");
  assert_eq!(refused["error"]["details"]["expected"], "janus-contract/1");
}

#[test]
fn a_contract_with_a_broken_interaction_record_is_a_document_error_with_a_pointer() {
  let mut engine = engine_with_real_components();
  let mut contract = contract_with_one_variant();
  contract["interactions"][0]["selection"]["variants"][0]
    .as_object_mut()
    .unwrap()
    .remove("origin");
  let refused = verify(&mut engine, contract, "http://127.0.0.1:1");
  assert_eq!(refused["error"]["code"], "contract-invalid");
  let pointer = refused["error"]["details"]["problems"][0]["pointer"]
    .as_str()
    .expect("a pointer");
  assert!(
    pointer.starts_with("/source/contracts/0"),
    "the pointer locates the document in the request: {pointer}"
  );
}

#[test]
fn a_source_the_engine_cannot_read_is_refused_by_kind() {
  let mut engine = engine_with_real_components();
  let refused = send(
    &mut engine,
    "v-1",
    "verification/verify",
    json!({
      "source": { "kind": "broker", "url": "https://broker.example" },
      "target": { "transports": [ { "transport": "http", "options": { "base-url": "http://127.0.0.1:1" } } ] },
    }),
  );
  assert_eq!(refused["error"]["code"], "operation-unsupported");
  assert_eq!(
    refused["error"]["details"]["op"],
    "verification/verify (source kind: broker)"
  );
}

#[test]
fn a_target_naming_a_transport_the_engine_does_not_have_fails_the_call() {
  let mut engine = engine_with_real_components();
  let refused = send(
    &mut engine,
    "v-1",
    "verification/verify",
    json!({
      "source": { "kind": "inline", "contracts": [contract_with_one_variant()] },
      "target": { "transports": [ { "transport": "grpc", "options": {} } ] },
    }),
  );
  assert_eq!(refused["error"]["code"], "component-unavailable");
  assert_eq!(refused["error"]["details"]["component"], "transport/grpc");
}

#[test]
fn a_target_the_transport_cannot_start_fails_the_call_not_the_run() {
  let mut engine = engine_with_real_components();
  let refused = send(
    &mut engine,
    "v-1",
    "verification/verify",
    json!({
      "source": { "kind": "inline", "contracts": [contract_with_one_variant()] },
      // No base-url: the HTTP component has nowhere to send, and says so at `start`.
      "target": { "transports": [ { "transport": "http", "options": {} } ] },
    }),
  );
  assert_eq!(refused["error"]["code"], "component-failed");
  assert_eq!(refused["error"]["details"]["component"], "transport/http");
}

#[test]
fn verify_needs_the_handshake_like_every_other_operation() {
  let mut engine = Engine::new();
  let refused = send(
    &mut engine,
    "v-1",
    "verification/verify",
    json!({ "source": { "kind": "inline", "contracts": [] }, "target": { "transports": [] } }),
  );
  assert_eq!(refused["error"]["code"], "handshake-required");
}

#[test]
fn a_run_with_nothing_to_verify_still_reports_a_summary() {
  let mut engine = engine_with_real_components();
  let started = send(
    &mut engine,
    "v-1",
    "verification/verify",
    json!({
      "source": { "kind": "inline", "contracts": [] },
      "target": { "transports": [ { "transport": "http", "options": { "base-url": "http://127.0.0.1:1" } } ] },
    }),
  );
  let stream = started["ok"]["stream"].as_str().unwrap().to_string();
  let events = drain_run(&mut engine, &stream);
  assert_eq!(
    kinds(&events),
    vec!["verification/started", "verification/finished"]
  );
  let summary = summary(&events);
  assert_eq!(
    summary["status"],
    json!("verified"),
    "nothing failed, so nothing is failed"
  );
  assert_eq!(
    summary["variants"],
    json!({ "total": 0, "verified": 0, "failed": 0 })
  );
}
