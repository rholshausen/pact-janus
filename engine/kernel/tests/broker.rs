//! Staged plan S1.3: `verification/verify` from a Pact Broker, with a broker client that is not a
//! real one (engine-protocol spec §8.3, ADR 0032). The client is the kernel's seam: what it
//! fetched, the run verifies; what the run concluded, it publishes. Faking it here pins the run's
//! side of that — pending, publishing, a fetch that fails, credentials kept out of every event —
//! without a broker; the native client is tested against a real one by the differential harness.

use pact_janus_component_http::HttpTransport;
use pact_janus_component_json::JsonContent;
use pact_janus_kernel::broker::{BrokerClient, FetchRequest, FetchedPact, PublishRequest};
use pact_janus_kernel::component::{ComponentError, TransportComponent};
use pact_janus_kernel::protocol::Engine;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread;

fn send(engine: &mut Engine, id: &str, op: &str, body: Value) -> Value {
  let request = json!({ "type": "request", "id": id, "op": op, "body": body });
  let bytes = engine.dispatch(&serde_json::to_vec(&request).unwrap());
  serde_json::from_slice(&bytes).unwrap()
}

/// What the fake broker serves, and what it was asked.
#[derive(Default)]
struct FakeBroker {
  pacts: Vec<FetchedPact>,
  fail_fetch: bool,
  fetched: Mutex<Vec<FetchRequest>>,
  published: Mutex<Vec<Value>>,
}

impl BrokerClient for FakeBroker {
  fn fetch_for_verification(&self, request: &FetchRequest) -> Result<Vec<FetchedPact>, ComponentError> {
    self.fetched.lock().unwrap().push(request.clone());
    if self.fail_fetch {
      return Err(ComponentError {
        code: "broker-refused".to_string(),
        category: "component".to_string(),
        message: "the broker answered 401".to_string(),
        source: None,
        details: None,
      });
    }
    Ok(self.pacts.clone())
  }

  fn publish_results(&self, request: &PublishRequest<'_>) -> Result<(), ComponentError> {
    self.published.lock().unwrap().push(json!({
      "handle": request.handle,
      "provider-version": request.provider_version,
      "branch": request.branch,
      "success": request.success,
      "results": request.results.iter().map(|result| json!({
        "id": result.id, "description": result.description, "success": result.success,
      })).collect::<Vec<_>>(),
    }));
    Ok(())
  }
}

fn engine(broker: Option<Arc<FakeBroker>>) -> Engine {
  let mut transports: HashMap<String, Arc<dyn TransportComponent>> = HashMap::new();
  transports.insert("http".to_string(), Arc::new(HttpTransport::new()));
  let mut engine = Engine::with_components(transports, Some(Arc::new(JsonContent::new())));
  if let Some(broker) = broker {
    engine.register_broker_client(broker);
  }
  let hello = send(
    &mut engine,
    "r-1",
    "engine/hello",
    json!({ "protocol-versions": [1], "host": { "name": "janus-test", "version": "0.0.0" }, "capabilities": {} }),
  );
  assert!(hello.get("ok").is_some(), "{hello}");
  engine
}

/// A provider answering every request with `{"id": 1}`.
fn provider() -> String {
  let listener = TcpListener::bind("127.0.0.1:0").unwrap();
  let base_url = format!("http://{}", listener.local_addr().unwrap());
  thread::spawn(move || {
    for stream in listener.incoming() {
      let Ok(mut stream) = stream else { return };
      let mut reader = BufReader::new(stream.try_clone().unwrap());
      let mut length = 0;
      loop {
        let mut line = String::new();
        if reader.read_line(&mut line).unwrap_or(0) == 0 || line == "\r\n" {
          break;
        }
        if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
          length = value.trim().parse().unwrap_or(0);
        }
      }
      let mut body = vec![0; length];
      let _ = reader.read_exact(&mut body);
      let reply = r#"{"id":1}"#;
      let _ = write!(
        stream,
        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{reply}",
        reply.len()
      );
    }
  });
  base_url
}

/// A v3 pact as a broker serves it: every interaction carries the broker's `_id`. `expects` is the
/// body the consumer expects back.
fn pact(consumer: &str, expects: Value) -> Value {
  json!({
    "consumer": { "name": consumer }, "provider": { "name": "p" },
    "interactions": [{
      "_id": format!("{consumer}-1"),
      "description": format!("{consumer} asks for widget 1"),
      "request": { "method": "GET", "path": "/widgets/1" },
      "response": { "status": 200, "headers": { "Content-Type": "application/json" }, "body": expects }
    }],
    "metadata": { "pactSpecification": { "version": "3.0.0" } }
  })
}

fn fetched(document: Value, pending: bool) -> FetchedPact {
  let consumer = document["consumer"]["name"].as_str().unwrap().to_string();
  FetchedPact {
    document,
    pending,
    wip: false,
    notices: vec![format!("{consumer} was selected")],
    handle: json!({ "pact": consumer }),
  }
}

fn verify(engine: &mut Engine, source_extra: Value, options: Value) -> Vec<Value> {
  let mut source = json!({
    "kind": "broker",
    "broker": { "url": "https://user:hunter2@broker.example", "auth": { "kind": "bearer", "token": "s3cret-token" } },
    "provider": "p",
    "consumer-version-selectors": [{ "mainBranch": true }],
    "provider-branch": "main",
    "include-pending": true,
  });
  for (key, value) in source_extra.as_object().unwrap() {
    source[key] = value.clone();
  }
  let started = send(
    engine,
    "v-1",
    "verification/verify",
    json!({
      "source": source,
      "target": { "transports": [{ "transport": "http", "options": { "base-url": provider() } }] },
      "options": options,
    }),
  );
  let stream = started["ok"]["stream"]
    .as_str()
    .unwrap_or_else(|| panic!("{started}"))
    .to_string();
  let mut events = Vec::new();
  for poll in 0..100 {
    let polled = send(
      engine,
      &format!("p-{poll}"),
      "events/poll",
      json!({ "streams": [stream], "wait-ms": 5_000 }),
    );
    events.extend(polled["ok"]["events"].as_array().cloned().unwrap_or_default());
    if events.last().is_some_and(|event| event["last"] == json!(true)) {
      break;
    }
  }
  events
}

fn of_kind<'a>(events: &'a [Value], kind: &str) -> Vec<&'a Value> {
  events
    .iter()
    .filter(|event| event["kind"] == json!(kind))
    .map(|event| &event["payload"])
    .collect()
}

#[test]
fn an_engine_with_a_broker_client_says_so_in_the_handshake() {
  let mut with = Engine::new();
  with.register_broker_client(Arc::new(FakeBroker::default()));
  let hello =
    json!({ "protocol-versions": [1], "host": { "name": "t", "version": "0" }, "capabilities": {} });
  let answered = send(&mut with, "r-1", "engine/hello", hello.clone());
  assert_eq!(answered["ok"]["capabilities"]["broker"], json!({}));
  let answered = send(&mut Engine::new(), "r-1", "engine/hello", hello);
  assert!(
    answered["ok"]["capabilities"].get("broker").is_none(),
    "{answered}"
  );
}

/// The run verifies what the broker selected, says what it fetched before anything else, and a
/// pending pact's failure is reported, published, and does not fail the run.
#[test]
fn a_pending_pacts_failure_is_reported_and_published_and_does_not_fail_the_run() {
  let broker = Arc::new(FakeBroker {
    pacts: vec![
      fetched(pact("web", json!({ "id": 1 })), false),
      fetched(pact("admin", json!({ "id": 1, "colour": "red" })), true),
    ],
    ..FakeBroker::default()
  });
  let mut engine = engine(Some(Arc::clone(&broker)));
  let events = verify(
    &mut engine,
    json!({}),
    json!({ "publish": { "provider-version": "2.0.0", "branch": "main" } }),
  );

  assert_eq!(events[0]["kind"], "verification/fetched");
  assert_eq!(events[0]["payload"]["pacts"][1]["consumer"], "admin");
  assert_eq!(events[0]["payload"]["pacts"][1]["pending"], true);

  let results = of_kind(&events, "verification/interaction-result");
  assert_eq!(results[0]["status"], "verified");
  assert_eq!(
    results[0]["interaction"]["id"], "web-1",
    "the broker's id rides in the reference"
  );
  assert_eq!(results[1]["status"], "failed");
  assert_eq!(results[1]["pending"], true);

  let summary = &events.last().unwrap()["payload"];
  assert_eq!(summary["status"], "verified", "{summary}");
  assert_eq!(summary["pending"], json!([false, true]));
  assert_eq!(
    summary["variants"]["failed"], 1,
    "a pending failure is still counted"
  );
  assert_eq!(
    summary["publication"],
    json!([{ "consumer": "web", "outcome": "published" }, { "consumer": "admin", "outcome": "published" }])
  );
  assert_eq!(of_kind(&events, "verification/published").len(), 2);

  let published = broker.published.lock().unwrap();
  assert_eq!(published[0]["handle"], json!({ "pact": "web" }));
  assert_eq!(published[0]["success"], true);
  assert_eq!(published[0]["branch"], "main");
  assert_eq!(
    published[1]["success"], false,
    "a pending pact's failure is published as one"
  );
  assert_eq!(published[1]["results"][0]["id"], "admin-1");

  let asked = broker.fetched.lock().unwrap();
  assert_eq!(asked[0].selectors, vec![json!({ "mainBranch": true })]);
  assert!(asked[0].include_pending);
  assert_eq!(asked[0].provider_branch.as_deref(), Some("main"));
}

#[test]
fn a_failure_in_a_pact_that_is_not_pending_fails_the_run() {
  let broker = Arc::new(FakeBroker {
    pacts: vec![fetched(pact("admin", json!({ "id": 1, "colour": "red" })), false)],
    ..FakeBroker::default()
  });
  let events = verify(&mut engine(Some(broker)), json!({}), json!({}));
  let summary = &events.last().unwrap()["payload"];
  assert_eq!(summary["status"], "failed", "{summary}");
  assert!(
    summary.get("publication").is_none(),
    "nothing was asked to publish"
  );
}

/// Spec §8.3: a fetch that fails ends the run as one that cannot complete does, with
/// `component-failed` naming `broker` and the client's own error. And spec §8.7: the credentials
/// are in the request and nowhere in what the run reports.
#[test]
fn a_fetch_that_fails_ends_the_run_naming_the_broker_and_never_its_credentials() {
  let broker = Arc::new(FakeBroker {
    fail_fetch: true,
    ..FakeBroker::default()
  });
  let events = verify(&mut engine(Some(broker)), json!({}), json!({}));
  assert_eq!(events.len(), 1, "nothing ran: {events:?}");
  let summary = &events[0]["payload"];
  assert_eq!(summary["status"], "failed");
  assert_eq!(summary["error"]["code"], "component-failed");
  assert_eq!(summary["error"]["details"]["component"], "broker");
  assert_eq!(summary["error"]["details"]["error"]["code"], "broker-refused");
  let everything = serde_json::to_string(&events).unwrap();
  assert!(
    !everything.contains("s3cret-token") && !everything.contains("hunter2"),
    "{everything}"
  );
}

#[test]
fn selectors_that_select_nothing_fail_the_run_unless_no_pacts_were_required() {
  let empty = || Arc::new(FakeBroker::default());
  let events = verify(&mut engine(Some(empty())), json!({}), json!({}));
  let summary = &events.last().unwrap()["payload"];
  assert_eq!(summary["error"]["code"], "pacts-not-found", "{summary}");

  let events = verify(
    &mut engine(Some(empty())),
    json!({ "require-pacts": false }),
    json!({}),
  );
  let summary = &events.last().unwrap()["payload"];
  assert_eq!(summary["status"], "verified", "{summary}");
  assert_eq!(summary["contracts"], 0);
}

#[test]
fn a_broker_event_names_the_broker_by_its_url_without_user_information() {
  let broker = Arc::new(FakeBroker {
    pacts: vec![fetched(pact("web", json!({ "id": 1 })), false)],
    ..FakeBroker::default()
  });
  let events = verify(&mut engine(Some(broker)), json!({}), json!({}));
  assert_eq!(events[0]["payload"]["broker"], "https://broker.example");
}

#[test]
fn publishing_needs_a_broker_source() {
  let mut engine = engine(Some(Arc::new(FakeBroker::default())));
  let refused = send(
    &mut engine,
    "v-1",
    "verification/verify",
    json!({
      "source": { "kind": "inline", "contracts": [] },
      "target": { "transports": [{ "transport": "http", "options": { "base-url": "http://127.0.0.1:1" } }] },
      "options": { "publish": { "provider-version": "1.0.0" } },
    }),
  );
  assert_eq!(refused["error"]["code"], "malformed-frame", "{refused}");
}

/// A v4 interaction marked pending (the compatibility suite's "a pending HTTP interaction"): its
/// failure is reported, and does not fail the run — from a file as from a broker.
#[test]
fn a_pending_v4_interactions_failure_does_not_fail_the_run() {
  let mut engine = engine(None);
  let document = json!({
    "consumer": { "name": "c" }, "provider": { "name": "p" },
    "interactions": [{
      "type": "Synchronous/HTTP", "key": "k1", "pending": true,
      "description": "a pending one",
      "request": { "method": "GET", "path": "/widgets/1" },
      "response": { "status": 200, "body": { "content": { "colour": "red" }, "contentType": "application/json" } }
    }],
    "metadata": { "pactSpecification": { "version": "4.0" } }
  });
  let started = send(
    &mut engine,
    "v-1",
    "verification/verify",
    json!({
      "source": { "kind": "inline", "contracts": [document] },
      "target": { "transports": [{ "transport": "http", "options": { "base-url": provider() } }] },
    }),
  );
  let stream = started["ok"]["stream"].as_str().unwrap().to_string();
  let mut events = Vec::new();
  while !events
    .last()
    .is_some_and(|event: &Value| event["last"] == json!(true))
  {
    let polled = send(
      &mut engine,
      "p",
      "events/poll",
      json!({ "streams": [stream], "wait-ms": 5_000 }),
    );
    events.extend(polled["ok"]["events"].as_array().cloned().unwrap_or_default());
  }
  let results = of_kind(&events, "verification/interaction-result");
  assert_eq!(results[0]["status"], "failed");
  assert_eq!(results[0]["pending"], true);
  assert_eq!(events.last().unwrap()["payload"]["status"], "verified");
}
