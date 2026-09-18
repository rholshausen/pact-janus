//! Plan task 5.4: **v1–v4 pacts verify, unchanged, through design 3.5's plan path.**
//!
//! This is B5's migration promise made concrete — "providers upgrade first at no cost". A pact
//! written years ago by some other SDK is handed to `verification/verify` exactly as it sits on
//! disk; the engine identifies it, compiles each interaction with the v1–v4 matching-rule compiler
//! (plan task 3.5), replays the pact's own recorded request, and scores the reply with the same
//! interpreter a Janus contract's variants are scored with. Nothing is upgraded on the way in,
//! because upgrading is lossy (contract-file spec §8) and verifying where the pact stands is not.
//!
//! The evidence is deliberately of two kinds:
//!
//! - **The sample provider (5.6), unchanged**, verified from a v3 pact with the very hooks its own
//!   `verifier.janus.yaml` declares — its existing v3 state endpoint for the states, the oauth2
//!   component for the credential. Nothing about that provider knows which kind of document is
//!   driving it, and the same run verifies a Janus contract and a v3 pact side by side.
//! - **Pacts this project did not write**: pact-jvm's own consumer-test output, and a pact_gem
//!   pact from pact-jvm's provider test resources — a Ruby SDK's v1-era file with a bare
//!   `provider_state` string. Those are verified against scripted stubs, because what is under
//!   test there is the *document*, not the provider.

use pact_janus_hooks_host::{ExecHooks, HttpHooks};
use pact_janus_kernel::component::TransportComponent;
use pact_janus_kernel::hooks::HookInvoker;
use pact_janus_kernel::protocol::Engine;
use pact_janus_sample_order_service::{
  Config as ProviderConfig, DEFAULT_CLIENT_ID, DEFAULT_CLIENT_SECRET, Provider, start as start_provider,
};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

// ---------------------------------------------------------------------------------------------
// Harness
// ---------------------------------------------------------------------------------------------

fn send(engine: &mut Engine, id: &str, op: &str, body: Value) -> Value {
  let request = json!({ "type": "request", "id": id, "op": op, "body": body });
  let bytes = engine.dispatch(&serde_json::to_vec(&request).expect("a json! literal serializes"));
  serde_json::from_slice(&bytes).expect("dispatch always returns valid JSON")
}

fn engine_with_everything() -> Engine {
  let mut transports: HashMap<String, Arc<dyn TransportComponent>> = HashMap::new();
  transports.insert(
    "http".to_string(),
    Arc::new(pact_janus_component_http::HttpTransport::new()),
  );
  let mut engine = Engine::with_components(
    transports,
    Some(Arc::new(pact_janus_component_json::JsonContent::new())),
  );
  engine.register_hook_invoker("exec", Arc::new(ExecHooks::new()) as Arc<dyn HookInvoker>);
  engine.register_hook_invoker("http", Arc::new(HttpHooks::new()) as Arc<dyn HookInvoker>);
  engine.register_hook_component("oauth2", Arc::new(pact_janus_component_oauth2::Oauth2Hook::new()));
  send(
    &mut engine,
    "r-1",
    "engine/hello",
    json!({ "protocol-versions": [1], "host": { "name": "janus-test", "version": "0.0.0" }, "capabilities": {} }),
  );
  engine
}

/// Long-poll the run's stream until its terminal event arrives (spec §9.2: termination is
/// structural, so this reads `last` and never a kind vocabulary).
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
  assert_eq!(
    events.last().expect("a run always emits events")["last"],
    json!(true),
    "the run terminated structurally"
  );
  events
}

fn verify(engine: &mut Engine, documents: Vec<Value>, base_url: &str, hooks: Option<Value>) -> Vec<Value> {
  let mut target = json!({
    "transports": [ { "transport": "http", "options": { "base-url": base_url } } ]
  });
  if let Some(hooks) = hooks {
    target["hooks"] = hooks;
  }
  let started = send(
    engine,
    "v-1",
    "verification/verify",
    json!({ "source": { "kind": "inline", "contracts": documents }, "target": target }),
  );
  let stream = started["ok"]["stream"]
    .as_str()
    .unwrap_or_else(|| panic!("verify failed: {started}"))
    .to_string();
  drain_run(engine, &stream)
}

fn payloads<'a>(events: &'a [Value], kind: &str) -> Vec<&'a Value> {
  events
    .iter()
    .filter(|e| e["kind"] == json!(kind))
    .map(|e| &e["payload"])
    .collect()
}

fn results(events: &[Value]) -> Vec<&Value> {
  payloads(events, "verification/interaction-result")
}

fn summary(events: &[Value]) -> &Value {
  &events.last().expect("a terminal event")["payload"]
}

fn started(events: &[Value]) -> &Value {
  &events.first().expect("a first event")["payload"]
}

fn assert_all_verified(events: &[Value], expected: usize) {
  let results = results(events);
  assert_eq!(results.len(), expected, "one result per interaction × variant");
  for result in &results {
    assert_eq!(result["status"], json!("verified"), "unverified: {result}");
  }
  assert_eq!(summary(events)["status"], json!("verified"));
}

/// A pact this repo keeps under `tests/fixtures/legacy-pacts/` — files other SDKs wrote.
fn fixture(name: &str) -> Value {
  read_json(&format!(
    "{}/tests/fixtures/legacy-pacts/{name}",
    env!("CARGO_MANIFEST_DIR")
  ))
}

/// The v3 pact a consumer published for the sample provider.
fn order_service_pact() -> Value {
  read_json(&format!(
    "{}/../../samples/order-service/pacts/web-app-order-service.json",
    env!("CARGO_MANIFEST_DIR")
  ))
}

fn read_json(path: &str) -> Value {
  let text = std::fs::read_to_string(path).unwrap_or_else(|err| panic!("reading {path}: {err}"));
  serde_json::from_str(&text).unwrap_or_else(|err| panic!("parsing {path}: {err}"))
}

// ---------------------------------------------------------------------------------------------
// A scripted stub, for the pacts whose providers are not in this repo
// ---------------------------------------------------------------------------------------------

/// One scripted reply: a status, headers, and a body.
struct Canned {
  status: u16,
  headers: Vec<(&'static str, String)>,
  body: String,
}

fn canned(status: u16, body: &str) -> Canned {
  Canned {
    status,
    headers: vec![("content-type", "application/json".to_string())],
    body: body.to_string(),
  }
}

impl Canned {
  fn with_header(mut self, name: &'static str, value: &str) -> Canned {
    self.headers.push((name, value.to_string()));
    self
  }
}

/// A provider stub answering a scripted sequence, one reply per request, reporting each request
/// line it saw. Scripted rather than routed, for the same reason `verification.rs`'s is: what is
/// under test here is the document, and a router would be a second thing that could be wrong.
fn stub_provider(replies: Vec<Canned>) -> (String, Receiver<String>) {
  let listener = TcpListener::bind("127.0.0.1:0").expect("an OS-assigned port always binds");
  let base_url = format!("http://{}", listener.local_addr().unwrap());
  let (tx, rx) = mpsc::channel();
  thread::spawn(move || {
    for canned in replies {
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

      let mut head = format!(
        "HTTP/1.1 {} X\r\ncontent-length: {}\r\n",
        canned.status,
        canned.body.len()
      );
      for (name, value) in &canned.headers {
        head.push_str(&format!("{name}: {value}\r\n"));
      }
      head.push_str("connection: close\r\n\r\n");
      let stream = reader.get_mut();
      let _ = stream.write_all(head.as_bytes());
      let _ = stream.write_all(canned.body.as_bytes());
      let _ = stream.flush();
      let _ = tx.send(request_line.trim_end().to_string());
    }
  });
  (base_url, rx)
}

// ---------------------------------------------------------------------------------------------
// The sample provider, verified from a v3 pact — the headline claim
// ---------------------------------------------------------------------------------------------

/// This provider's own hook configuration, as `verifier.janus.yaml` declares it: its existing v3
/// state endpoint, and the oauth2 component for the credential. **Nothing in here mentions the
/// kind of document being verified**, which is the point — it is the same configuration the Janus
/// contract runs under.
fn verifier_hooks(provider: &Provider) -> Value {
  let credentials = json!({
    "token-url": format!("{}/oauth/token", provider.base_url()),
    "client-id": DEFAULT_CLIENT_ID,
    "client-secret": DEFAULT_CLIENT_SECRET,
  });
  json!({
    "version": 1,
    "hooks": {
      "before-verification": [
        { "name": "auth", "run": { "kind": "component", "component": "oauth2" },
          "config": credentials } ],
      "state-setup": [
        { "name": "fixtures",
          "run": { "kind": "http", "url": format!("{}/_pact/provider-states", provider.base_url()),
                   "format": "pact-state-change" } } ],
      "before-request": [
        { "name": "auth", "run": { "kind": "component", "component": "oauth2" },
          "config": credentials, "changes": ["parts.request.headers"] } ]
    }
  })
}

#[test]
fn a_v3_pact_verifies_against_the_sample_provider_with_no_change_to_either() {
  let provider = start_provider(ProviderConfig::default()).expect("the sample provider binds");
  let mut engine = engine_with_everything();
  let hooks = verifier_hooks(&provider);

  let events = verify(
    &mut engine,
    vec![order_service_pact()],
    provider.base_url(),
    Some(hooks),
  );

  assert_all_verified(&events, 2);

  // The run says what it verified, in its first event and in every result.
  assert_eq!(started(&events)["formats"], json!(["pact/3.0.0"]));
  assert_eq!(started(&events)["providers"], json!(["order-service"]));
  assert_eq!(started(&events)["variants"], json!(2), "one example, one variant");
  for result in results(&events) {
    assert_eq!(result["interaction"]["format"], json!("pact/3.0.0"));
    assert_eq!(
      result["variant"],
      json!("base"),
      "a single example is the base variant (contract-file spec §8.3)"
    );
  }

  // The provider's own state endpoint received exactly what it received before Janus existed:
  // the v3 `{state, params, action}` document, with the pact's own parameters.
  let store = provider.store();
  let log = store.lock().unwrap();
  let setups: Vec<&Value> = log
    .state_log
    .iter()
    .filter(|entry| entry["action"] == json!("setup"))
    .collect();
  assert_eq!(setups.len(), 2, "once per interaction: {setups:?}");
  assert_eq!(setups[0]["state"], json!("an order exists"));
  assert_eq!(setups[0]["params"], json!({ "id": "66", "shipped": true }));
  assert_eq!(setups[1]["state"], json!("no orders exist"));
}

/// The same provider, the same hooks, the same run — one Janus contract and one v3 pact together.
/// If the migration promise holds, a provider cannot tell which of its consumers has moved, and
/// the only difference visible anywhere is the `format` each result names.
#[test]
fn one_run_verifies_a_janus_contract_and_a_v3_pact_against_the_same_provider() {
  let provider = start_provider(ProviderConfig::default()).expect("the sample provider binds");
  let mut engine = engine_with_everything();
  let hooks = verifier_hooks(&provider);
  let contract = janus_contract_for_the_same_order();

  let events = verify(
    &mut engine,
    vec![contract, order_service_pact()],
    provider.base_url(),
    Some(hooks),
  );

  assert_all_verified(&events, 3);
  assert_eq!(
    started(&events)["formats"],
    json!(["janus-contract/1", "pact/3.0.0"])
  );
  assert_eq!(started(&events)["contracts"], json!(2));

  let formats: Vec<&Value> = results(&events)
    .iter()
    .map(|result| &result["interaction"]["format"])
    .collect();
  assert_eq!(
    formats,
    vec![
      &json!("janus-contract/1"),
      &json!("pact/3.0.0"),
      &json!("pact/3.0.0")
    ],
    "every result names the document it came from"
  );
}

/// A hand-written Janus contract for the order the v3 pact above also describes: one recorded
/// variant, the same request, a shape over the same response.
fn janus_contract_for_the_same_order() -> Value {
  json!({
    "$format": "janus-contract/1",
    "consumer": { "name": "mobile-app" },
    "provider": { "name": "order-service" },
    "interactions": [{
      "description": "a request for an order",
      "transport": { "kind": "http", "mode": "passive" },
      "states": [{ "name": "an order exists", "params": { "id": "66", "shipped": true } }],
      "parts": {
        "request": { "method": { "shape": "equality", "example": "GET" },
                     "path": { "shape": "equality", "example": "/orders/66" } },
        "response": { "status": { "shape": "equality", "example": 200 },
                      "body": { "shape": "object", "members": {
                        "id": { "shape": "equality", "example": "66" },
                        "status": { "shape": "string", "example": "SHIPPED" } } } }
      },
      "selection": {
        "strategy": "base-only",
        "report": { "space": { "size": 1, "dimensions": 0 }, "selected": 1 },
        "variants": [{
          "id": "base",
          "origin": "base",
          "assignment": [],
          "states": [{ "name": "an order exists", "params": { "id": "66", "shipped": true } }],
          "parts": {
            "request": { "method": { "content": "GET" }, "path": { "content": "/orders/66" } }
          }
        }]
      }
    }]
  })
}

/// The failure this run has without the credential hook, and the reason the sample provider has
/// auth at all: the pact records no `Authorization` header, because the consumer's mock never
/// wanted one. Configuration adds it; the pact stays exactly as its author wrote it.
#[test]
fn without_the_credential_hook_the_same_pact_fails_on_the_providers_auth() {
  let provider = start_provider(ProviderConfig::default()).expect("the sample provider binds");
  let mut engine = engine_with_everything();
  let states_only = json!({
    "version": 1,
    "hooks": {
      "state-setup": [
        { "name": "fixtures",
          "run": { "kind": "http", "url": format!("{}/_pact/provider-states", provider.base_url()),
                   "format": "pact-state-change" } } ]
    }
  });

  let events = verify(
    &mut engine,
    vec![order_service_pact()],
    provider.base_url(),
    Some(states_only),
  );

  let results = results(&events);
  assert_eq!(results.len(), 2);
  assert_eq!(results[0]["status"], json!("failed"));
  let mismatches = serde_json::to_string(&results[0]["mismatches"]).unwrap();
  assert!(
    mismatches.contains("401"),
    "the provider answered 401 and the plan said so: {mismatches}"
  );
  assert_eq!(summary(&events)["status"], json!("failed"));
}

// ---------------------------------------------------------------------------------------------
// Pacts this project did not write
// ---------------------------------------------------------------------------------------------

/// pact-jvm's own consumer-test output, v4, two interactions asserting nothing but a status.
#[test]
fn a_v4_pact_another_sdk_wrote_verifies_through_the_same_path() {
  let (base_url, _requests) = stub_provider(vec![canned(200, "{}"), canned(200, "{}")]);
  let mut engine = engine_with_everything();

  let events = verify(
    &mut engine,
    vec![fixture("consumer-provider-v4.json")],
    &base_url,
    None,
  );

  assert_all_verified(&events, 2);
  assert_eq!(started(&events)["formats"], json!(["pact/4.0"]));
}

/// A pact_gem pact from pact-jvm's provider test resources — v1-era, five interactions, a bare
/// `provider_state` string rather than v3's list, and a lower-case `get` method. Every one of
/// those is a thing the engine must normalise rather than refuse: the states arrive as one state
/// with no parameters, and `get` matches `GET` because v1–v4 compare methods case-insensitively.
#[test]
fn a_v1_pact_from_another_ecosystem_verifies_and_its_bare_state_string_reaches_the_hook() {
  let (base_url, requests) = stub_provider(vec![
    canned(200, r#"{"alligators":[{"name":"Bob"}]}"#),
    canned(200, r#"{"alligators":[{"name":"Bob"}]}"#),
    canned(201, r#"{"alligators":[{"name":"Mary"}]}"#),
    canned(500, r#"{"error":"Argh!!!"}"#),
    canned(404, ""),
  ]);
  let (states_url, states) = state_recorder(5);
  let mut engine = engine_with_everything();
  let hooks = json!({
    "version": 1,
    "hooks": {
      "state-setup": [
        { "name": "fixtures",
          "run": { "kind": "http", "url": states_url, "format": "pact-state-change" } } ]
    }
  });

  let events = verify(
    &mut engine,
    vec![fixture("zoo_app-animal_service.json")],
    &base_url,
    Some(hooks),
  );

  assert_all_verified(&events, 5);

  let lines: Vec<String> = (0..5).map(|_| requests.recv().expect("five requests")).collect();
  assert_eq!(
    lines[0], "GET /animals HTTP/1.1",
    "a lower-case `get` replays as GET"
  );

  let asked: Vec<String> = (0..5)
    .map(|_| states.recv().expect("five state requests"))
    .collect();
  assert!(
    asked[0].contains("\"there are alligators\"") && asked[0].contains("\"params\":{}"),
    "a v1 pact's bare state string arrives as one state with no parameters: {}",
    asked[0]
  );
}

/// pact-jvm's v3 provider-state pact: `$.userId` carries a `type` matcher and `userName` carries
/// none. That is the whole of design 3.5's claim, observed from the verification side — the plan
/// decides the verdict, not the recorded example, and it decides it per position.
#[test]
fn a_v3_pacts_matching_rules_decide_the_verdict_not_the_recorded_example() {
  let reply = |user_id: &str, name: &str| {
    canned(200, &format!(r#"{{"userId":{user_id},"userName":"{name}"}}"#))
      .with_header("LOCATION", "http://server/users/666")
  };

  // A different number under a `type` matcher passes; the un-ruled name must still be equal.
  let (base_url, _requests) = stub_provider(vec![reply("777", "Test")]);
  let mut engine = engine_with_everything();
  let events = verify(
    &mut engine,
    vec![fixture("V3Consumer-ProviderStateService.json")],
    &base_url,
    None,
  );
  assert_all_verified(&events, 1);

  // Same document, same position, a string where the matcher wants a number.
  let (base_url, _requests) = stub_provider(vec![reply("\"777\"", "Test")]);
  let mut engine = engine_with_everything();
  let events = verify(
    &mut engine,
    vec![fixture("V3Consumer-ProviderStateService.json")],
    &base_url,
    None,
  );
  let results = results(&events);
  assert_eq!(results[0]["status"], json!("failed"));
  let mismatches = serde_json::to_string(&results[0]["mismatches"]).unwrap();
  assert!(
    mismatches.contains("$.response.body.userId"),
    "the failure is located at the position whose rule it broke: {mismatches}"
  );
}

/// The one documented gap, reported as what it is. Design 3.5 compiles JSON bodies only, so an XML
/// response is a **missing component** — the identifier names the content component that would
/// supply it (spec §10.2) — and never a crash, a skip, or a pass.
#[test]
fn an_xml_body_is_a_named_component_gap_rather_than_a_crash() {
  let (base_url, _requests) = stub_provider(vec![canned(200, "<alligator/>")]);
  let mut engine = engine_with_everything();

  let events = verify(
    &mut engine,
    vec![fixture("XMLConsumer-XMLProvider.json")],
    &base_url,
    None,
  );

  let results = results(&events);
  assert_eq!(
    results.len(),
    1,
    "the gap is reported against the variant, not swallowed"
  );
  assert_eq!(results[0]["status"], json!("failed"));
  assert_eq!(results[0]["error"]["code"], json!("component-unavailable"));
  assert!(
    results[0]["error"]["component"]
      .as_str()
      .is_some_and(|component| component.starts_with("content/")),
    "the missing piece is named: {}",
    results[0]["error"]
  );
}

/// A pact whose interactions are all messages reads clean and verifies nothing — an honest empty
/// run, which is a different answer from a misparse and from a refusal.
#[test]
fn a_message_only_pact_verifies_nothing_and_says_so() {
  let (base_url, _requests) = stub_provider(vec![]);
  let mut engine = engine_with_everything();

  let events = verify(
    &mut engine,
    vec![fixture("test_consumer_v3-MessageProvider.json")],
    &base_url,
    None,
  );

  assert_eq!(started(&events)["interactions"], json!(0));
  assert_eq!(summary(&events)["variants"]["total"], json!(0));
  assert_eq!(summary(&events)["status"], json!("verified"));
}

/// A tiny HTTP endpoint that records the `pact-state-change` documents it is sent and answers
/// `200 {}` — for the pacts whose providers are not in this repo but whose states still have to
/// arrive somewhere.
fn state_recorder(count: usize) -> (String, Receiver<String>) {
  let listener = TcpListener::bind("127.0.0.1:0").expect("an OS-assigned port always binds");
  let url = format!("http://{}/states", listener.local_addr().unwrap());
  let (tx, rx) = mpsc::channel();
  thread::spawn(move || {
    // Setup and teardown both arrive here, so the recorder outlives twice the interaction count.
    for _ in 0..count * 2 {
      let Ok((stream, _)) = listener.accept() else {
        return;
      };
      stream.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
      let mut reader = BufReader::new(stream);
      let mut line = String::new();
      if reader.read_line(&mut line).is_err() {
        return;
      }
      let mut length = 0usize;
      loop {
        let mut header = String::new();
        if reader.read_line(&mut header).is_err() {
          return;
        }
        let header = header.trim_end();
        if header.is_empty() {
          break;
        }
        if let Some((name, value)) = header.split_once(':')
          && name.eq_ignore_ascii_case("content-length")
        {
          length = value.trim().parse().unwrap_or(0);
        }
      }
      let mut body = vec![0u8; length];
      let _ = reader.read_exact(&mut body);
      let body = String::from_utf8_lossy(&body).to_string();
      let response = "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: 2\r\nconnection: close\r\n\r\n{}";
      let stream = reader.get_mut();
      let _ = stream.write_all(response.as_bytes());
      let _ = stream.flush();
      if body.contains("\"setup\"") {
        let _ = tx.send(body);
      }
    }
  });
  (url, rx)
}
