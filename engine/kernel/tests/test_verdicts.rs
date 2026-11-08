//! ADR 0064: the consumer's test verdict reaches the engine. `consumer-session/report-variant`
//! records the test's verdict on a variant and answers with the variant's result, so a host learns
//! the engine's verdict in the test that ran the variant, and the engine withholds a contract the
//! consumer's own test failed on. A host that declared `test-verdicts` must report every variant,
//! and a refused interaction withholds its contract.
//!
//! Against the real HTTP transport: the race these tests care about, an outcome recorded after
//! its reply reached the client, only exists on a real wire.

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

/// A session over the real HTTP transport, the host declaring `capabilities`: its id, the
/// interaction's handle, its selected variant ids and the mock's address.
struct Session {
  engine: Engine,
  session: String,
  handle: String,
  variants: Vec<String>,
  addr: String,
}

impl Session {
  fn start(capabilities: Value) -> Session {
    let mut transports: HashMap<String, Arc<dyn TransportComponent>> = HashMap::new();
    transports.insert("http".to_string(), Arc::new(HttpTransport::new()));
    let mut engine = Engine::with_components(transports, Some(Arc::new(JsonContent::new())));
    engine.register_background(Arc::new(pact_janus_kernel::protocol::Threads));
    let hello = send(
      &mut engine,
      "engine/hello",
      json!({ "protocol-versions": [1], "host": { "name": "janus-test", "version": "0.0.0" }, "capabilities": capabilities }),
    );
    assert_eq!(
      hello["ok"]["capabilities"]["test-verdicts"],
      json!({}),
      "the engine declares that it has report-variant"
    );
    let created = send(
      &mut engine,
      "consumer-session/create",
      json!({ "config": { "consumer": { "name": "web-app" }, "provider": { "name": "order-api" } } }),
    );
    let session = created["ok"]["session"].as_str().unwrap().to_string();
    let added = send(
      &mut engine,
      "consumer-session/add-interaction",
      json!({ "session": session, "interaction": order_interaction() }),
    );
    let handle = added["ok"]["handle"].as_str().unwrap().to_string();
    let selected = send(
      &mut engine,
      "consumer-session/variants",
      json!({ "session": session, "handle": handle }),
    );
    let variants = selected["ok"]["variants"]
      .as_array()
      .unwrap()
      .iter()
      .map(|v| v["id"].as_str().unwrap().to_string())
      .collect::<Vec<_>>();
    assert_eq!(variants.len(), 2, "base, and shippedAt absent");
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
    Session {
      engine,
      session,
      handle,
      variants,
      addr,
    }
  }

  fn serve(&mut self, variant: &str) {
    let served = send(
      &mut self.engine,
      "consumer-session/serve-variant",
      json!({ "session": self.session, "handle": self.handle, "variant": variant }),
    );
    assert_eq!(served["ok"], json!({}), "{served}");
  }

  /// `report-variant`, answering the whole response frame.
  fn report(&mut self, variant: &str, status: &str, reason: Option<&str>) -> Value {
    let mut body =
      json!({ "session": self.session, "handle": self.handle, "variant": variant, "status": status });
    if let Some(reason) = reason {
      body["reason"] = Value::from(reason);
    }
    send(&mut self.engine, "consumer-session/report-variant", body)
  }

  fn finalise(&mut self) -> Value {
    let finalised = send(
      &mut self.engine,
      "consumer-session/finalise",
      json!({ "session": self.session }),
    );
    finalised["ok"].clone()
  }
}

fn order_interaction() -> Value {
  json!({
    "description": "a request for an order",
    "transport": { "kind": "http", "mode": "passive" },
    "parts": {
      "request": { "method": { "shape": "equality", "example": "GET" },
                   "path": { "shape": "equality", "example": "/orders/66" } },
      "response": { "status": { "shape": "equality", "example": 200 },
                    "body": { "shape": "object", "members": {
                      "shippedAt": { "shape": "optional", "of": { "shape": "string", "example": "2026-07-30" } } } } } }
  })
}

/// One GET, answering its status.
fn get(addr: &str, path: &str) -> u16 {
  let mut stream = TcpStream::connect(addr).unwrap();
  stream.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
  let request = format!("GET {path} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n");
  stream.write_all(request.as_bytes()).unwrap();
  let mut response = Vec::new();
  let _ = stream.read_to_end(&mut response);
  let head = String::from_utf8_lossy(&response);
  head
    .split_whitespace()
    .nth(1)
    .and_then(|status| status.parse().ok())
    .unwrap_or_else(|| panic!("no status in: {head}"))
}

#[test]
fn a_report_answers_the_variants_result_as_soon_as_its_test_ends() {
  let mut session = Session::start(json!({ "test-verdicts": {} }));
  let base = session.variants[0].clone();
  session.serve(&base);
  assert_eq!(get(&session.addr, "/orders/66"), 200);

  // No wait between the response and the report: the engine waits out the exchange's recording.
  let reported = session.report(&base, "passed", None);
  assert_eq!(
    reported["ok"],
    json!({ "result": { "variant": base, "status": "verified", "test": { "status": "passed" } } })
  );
}

#[test]
fn a_request_the_mock_rejected_fails_its_variant_in_the_report() {
  let mut session = Session::start(json!({ "test-verdicts": {} }));
  let base = session.variants[0].clone();
  session.serve(&base);
  assert_eq!(get(&session.addr, "/orders/7"), 500);

  let reported = session.report(&base, "passed", None);
  let result = &reported["ok"]["result"];
  assert_eq!(result["status"], "failed", "{result}");
  assert!(
    result["mismatches"].as_array().is_some_and(|m| !m.is_empty()),
    "the engine's mismatches ride with the result: {result}"
  );
}

#[test]
fn a_variant_its_test_never_sent_is_not_exercised_in_the_report() {
  let mut session = Session::start(json!({ "test-verdicts": {} }));
  let base = session.variants[0].clone();
  session.serve(&base);

  let reported = session.report(&base, "passed", None);
  assert_eq!(reported["ok"]["result"]["status"], "not-exercised");
}

/// A failed report decides the status, and hides `not-exercised`; `exercised: false` says it still,
/// so a host can say nothing called the mock beside the test's own error (ADR 0076's facade cases).
#[test]
fn a_failed_test_that_sent_nothing_says_so_beside_its_failure() {
  let mut session = Session::start(json!({ "test-verdicts": {} }));
  let base = session.variants[0].clone();
  session.serve(&base);

  let reported = session.report(&base, "failed", Some("Error: the client broke"));
  assert_eq!(
    reported["ok"]["result"],
    json!({ "variant": base, "status": "failed", "exercised": false,
            "test": { "status": "failed", "reason": "Error: the client broke" } })
  );
  let finalised = session.finalise();
  assert_eq!(
    finalised["results"][0]["variants"][0]["exercised"], false,
    "{finalised}"
  );
}

#[test]
fn a_failed_test_withholds_the_contract_though_every_exchange_verified() {
  // Task 4.6's careless client: it crashes whenever shippedAt is absent, and every exchange matches.
  let mut session = Session::start(json!({ "test-verdicts": {}, "contract-text": {} }));
  let variants = session.variants.clone();
  for variant in &variants {
    session.serve(variant);
    assert_eq!(get(&session.addr, "/orders/66"), 200);
  }
  session.report(&variants[0], "passed", None);
  let reported = session.report(&variants[1], "failed", Some("TypeError: shippedAt is undefined"));
  assert_eq!(
    reported["ok"]["result"],
    json!({ "variant": variants[1], "status": "failed",
            "test": { "status": "failed", "reason": "TypeError: shippedAt is undefined" } })
  );

  let finalised = session.finalise();
  assert_eq!(
    finalised["contract-text"],
    Value::Null,
    "the engine withheld it: {finalised}"
  );
  assert_eq!(finalised["results"][0]["status"], "failed");
  assert_eq!(finalised["results"][0]["variants"][1]["status"], "failed");
  assert_eq!(finalised["refused"], json!([]));
}

#[test]
fn an_unknown_verdict_counts_as_a_failure() {
  let mut session = Session::start(json!({ "test-verdicts": {} }));
  let base = session.variants[0].clone();
  session.serve(&base);
  assert_eq!(get(&session.addr, "/orders/66"), 200);

  let reported = session.report(&base, "inconclusive", None);
  assert_eq!(reported["ok"]["result"]["status"], "failed");
}

#[test]
fn an_unreported_variant_withholds_the_contract_from_a_host_that_reports() {
  let mut session = Session::start(json!({ "test-verdicts": {} }));
  let variants = session.variants.clone();
  for variant in &variants {
    session.serve(variant);
    assert_eq!(get(&session.addr, "/orders/66"), 200);
  }
  session.report(&variants[0], "passed", None);

  let finalised = session.finalise();
  assert_eq!(finalised["contract"], Value::Null, "{finalised}");
  assert_eq!(
    finalised["results"],
    json!([{ "handle": "i-1", "status": "not-reported", "variants": [
      { "variant": variants[0], "status": "verified", "test": { "status": "passed" } },
      { "variant": variants[1], "status": "not-reported" }
    ] }])
  );
}

#[test]
fn a_host_that_does_not_report_gets_protocol_version_ones_results() {
  let mut session = Session::start(json!({}));
  let variants = session.variants.clone();
  for variant in &variants {
    session.serve(variant);
    assert_eq!(get(&session.addr, "/orders/66"), 200);
  }

  let finalised = session.finalise();
  assert!(finalised["contract"].is_object(), "{finalised}");
  assert_eq!(finalised["results"][0]["status"], "verified");
  assert_eq!(
    finalised["refused"],
    Value::Null,
    "no refused member for this host"
  );
}

#[test]
fn a_reported_failure_withholds_the_contract_from_any_host() {
  let mut session = Session::start(json!({}));
  let variants = session.variants.clone();
  for variant in &variants {
    session.serve(variant);
    assert_eq!(get(&session.addr, "/orders/66"), 200);
  }
  session.report(&variants[1], "failed", None);

  let finalised = session.finalise();
  assert_eq!(finalised["contract"], Value::Null, "{finalised}");
}

#[test]
fn a_report_disarms_its_variant_so_a_later_request_matches_nothing() {
  let mut session = Session::start(json!({ "test-verdicts": {} }));
  let base = session.variants[0].clone();
  session.serve(&base);
  session.report(&base, "passed", None);

  assert_eq!(
    get(&session.addr, "/orders/66"),
    500,
    "nothing is armed once the variant's test has ended"
  );
}

#[test]
fn a_variant_outside_the_selection_is_variant_not_found() {
  let mut session = Session::start(json!({ "test-verdicts": {} }));
  let reported = session.report("status=NOPE", "passed", None);
  assert_eq!(reported["error"]["code"], "variant-not-found", "{reported}");
}

#[test]
fn a_refused_interaction_withholds_the_contract_and_is_listed() {
  let mut session = Session::start(json!({ "test-verdicts": {} }));
  let refused = send(
    &mut session.engine,
    "consumer-session/add-interaction",
    json!({ "session": session.session, "interaction": { "description": "a broken one", "parts": "no" } }),
  );
  assert_eq!(refused["error"]["code"], "interaction-invalid", "{refused}");
  let variants = session.variants.clone();
  for variant in &variants {
    session.serve(variant);
    assert_eq!(get(&session.addr, "/orders/66"), 200);
    session.report(variant, "passed", None);
  }

  let finalised = session.finalise();
  assert_eq!(finalised["results"][0]["status"], "verified", "{finalised}");
  assert_eq!(
    finalised["contract"],
    Value::Null,
    "the suite's contract would lack an interaction"
  );
  assert_eq!(finalised["refused"][0]["description"], "a broken one");
  assert_eq!(finalised["refused"][0]["code"], "interaction-invalid");
}

#[test]
fn every_variant_reported_passed_writes_the_contract() {
  let mut session = Session::start(json!({ "test-verdicts": {} }));
  let variants = session.variants.clone();
  for variant in &variants {
    session.serve(variant);
    assert_eq!(get(&session.addr, "/orders/66"), 200);
    session.report(variant, "passed", None);
  }

  let finalised = session.finalise();
  assert!(finalised["contract"].is_object(), "{finalised}");
  assert_eq!(finalised["refused"], json!([]));
}
