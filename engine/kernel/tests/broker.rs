//! Staged plan S1.3: `verification/verify` from a Pact Broker, with a broker client that is not a
//! real one (engine-protocol spec §8.3, ADR 0032). The client is the kernel's seam: what it
//! fetched, the run verifies; what the run concluded, it publishes. Faking it here pins the run's
//! side of that — pending, publishing, a fetch that fails, credentials kept out of every event —
//! without a broker; the native client is tested against a real one by the differential harness.
//! The same fake takes a consumer's contracts (`broker/publish-contracts`, spec §8.7), for the
//! engine's side of that: what it checks before anything is published, and what it reports.

use pact_janus_component_http::HttpTransport;
use pact_janus_component_json::JsonContent;
use pact_janus_kernel::broker::{
  BrokerClient, ContractsPublished, ContractsToPublish, FetchRequest, FetchedPact, PublishRequest,
};
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
  fail_publish: bool,
  fetched: Mutex<Vec<FetchRequest>>,
  published: Mutex<Vec<Value>>,
  /// Each publication of contracts, with every contract's content parsed back.
  publications: Mutex<Vec<Value>>,
}

fn refused(message: &str) -> ComponentError {
  ComponentError {
    code: "broker-refused".to_string(),
    category: "component".to_string(),
    message: message.to_string(),
    source: None,
    details: None,
  }
}

impl BrokerClient for FakeBroker {
  fn fetch_for_verification(&self, request: &FetchRequest) -> Result<Vec<FetchedPact>, ComponentError> {
    self.fetched.lock().unwrap().push(request.clone());
    if self.fail_fetch {
      return Err(refused("the broker answered 401"));
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

  fn publish_contracts(
    &self,
    request: &ContractsToPublish<'_>,
  ) -> Result<ContractsPublished, ComponentError> {
    self.publications.lock().unwrap().push(json!({
      "consumer": request.consumer,
      "consumer-version": request.consumer_version,
      "branch": request.branch,
      "tags": request.tags,
      "build-url": request.build_url,
      "contracts": request.contracts.iter().map(|contract| json!({
        "provider": contract.provider,
        "content": serde_json::from_slice::<Value>(&contract.content).unwrap(),
        "starts": String::from_utf8_lossy(&contract.content[..11.min(contract.content.len())]),
      })).collect::<Vec<_>>(),
    }));
    if self.fail_publish {
      return Err(refused("the broker answered 409"));
    }
    Ok(ContractsPublished {
      notices: vec!["published for web-app version 2.7.0".to_string()],
    })
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
  // ADR 0033: each source's own counts, which add up to the run's.
  assert_eq!(summary["pairs"][0]["consumer"], "web");
  assert_eq!(summary["pairs"][0]["status"], "verified");
  assert_eq!(summary["pairs"][0]["variants"]["verified"], 1);
  assert_eq!(
    summary["pairs"][1]["status"], "failed",
    "a pending pact that failed still failed"
  );
  assert_eq!(summary["pairs"][1]["variants"]["failed"], 1);
  let total: u64 = (0..2)
    .map(|i| summary["pairs"][i]["variants"]["total"].as_u64().unwrap())
    .sum();
  assert_eq!(json!(total), summary["variants"]["total"]);

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

// ---------------------------------------------------------------------------------------------
// broker/publish-contracts
// ---------------------------------------------------------------------------------------------

/// A Janus contract from `web-app` to `stock-api`.
fn janus_contract() -> Value {
  json!({
    "$format": "janus-contract/1",
    "consumer": { "name": "web-app" },
    "provider": { "name": "stock-api" },
    "interactions": [{
      "description": "a request for stock",
      "transport": { "kind": "http", "mode": "passive" },
      "parts": {
        "request": { "method": { "shape": "equality", "example": "GET" },
                     "path": { "shape": "equality", "example": "/stock/1" } },
        "response": { "status": { "shape": "equality", "example": 200 } }
      },
      "selection": {
        "variants": [{
          "id": "base", "origin": "base", "assignment": [],
          "parts": {
            "request": { "method": { "content": "GET" }, "path": { "content": "/stock/1" } },
            "response": { "status": { "content": 200 } }
          }
        }],
        "report": { "space": { "size": 1, "dimensions": 0 } }
      },
      "x-from-a-newer-writer": true
    }]
  })
}

/// Publish `contracts` and drain the broker session's stream; the response, then its events.
fn publish_contracts(engine: &mut Engine, contracts: Vec<Value>) -> (Value, Vec<Value>) {
  let started = send(
    engine,
    "b-1",
    "broker/publish-contracts",
    json!({
      "broker": { "url": "https://user:hunter2@broker.example", "auth": { "kind": "bearer", "token": "s3cret-token" } },
      "consumer-version": "2.7.0",
      "branch": "main",
      "tags": ["rc"],
      "build-url": "https://ci.example/377",
      "contracts": contracts,
    }),
  );
  let Some(stream) = started["ok"]["stream"].as_str().map(str::to_string) else {
    return (started, vec![]);
  };
  let mut events = Vec::new();
  for attempt in 0..100 {
    let polled = send(
      engine,
      &format!("p-{attempt}"),
      "events/poll",
      json!({ "streams": [stream], "wait-ms": 200 }),
    );
    let batch = polled["ok"]["events"].as_array().cloned().unwrap_or_default();
    let done = batch.iter().any(|event| event["last"] == json!(true));
    events.extend(batch);
    if done {
      break;
    }
  }
  (started, events)
}

#[test]
fn a_consumers_contracts_are_published_together_and_reported_one_by_one() {
  let broker = Arc::new(FakeBroker::default());
  let mut engine = engine(Some(Arc::clone(&broker)));
  let pact = json!({
    "consumer": { "name": "web-app" }, "provider": { "name": "order-api" },
    "interactions": [{ "description": "a request", "request": { "method": "GET", "path": "/" },
                       "response": { "status": 200 } }],
    "metadata": { "pactSpecification": { "version": "3.0.0" } }
  });
  let (started, events) = publish_contracts(&mut engine, vec![pact.clone(), janus_contract()]);
  assert!(
    started["ok"]["session"].as_str().unwrap().starts_with("bs-"),
    "{started}"
  );

  let publications = broker.publications.lock().unwrap();
  assert_eq!(publications.len(), 1, "one publication for both contracts");
  let publication = &publications[0];
  assert_eq!(publication["consumer"], "web-app");
  assert_eq!(publication["consumer-version"], "2.7.0");
  assert_eq!(publication["branch"], "main");
  assert_eq!(publication["tags"], json!(["rc"]));
  assert_eq!(publication["build-url"], "https://ci.example/377");
  assert_eq!(publication["contracts"][0]["provider"], "order-api");
  assert_eq!(publication["contracts"][0]["content"], pact);
  assert_eq!(publication["contracts"][1]["provider"], "stock-api");
  // The document as the host sent it, members this engine does not know included, and still
  // opening with `$format`.
  assert_eq!(publication["contracts"][1]["content"], janus_contract());
  assert_eq!(publication["contracts"][1]["starts"], r#"{"$format":"#);

  let kinds: Vec<&str> = events.iter().map(|e| e["kind"].as_str().unwrap()).collect();
  assert_eq!(
    kinds,
    vec!["broker/published", "broker/published", "broker/finished"]
  );
  assert_eq!(
    events[0]["payload"],
    json!({ "consumer": "web-app", "provider": "order-api", "outcome": "published" })
  );
  let finished = &events[2]["payload"];
  assert_eq!(finished["status"], "published");
  assert_eq!(finished["broker"], "https://broker.example");
  assert_eq!(finished["consumer-version"], "2.7.0");
  assert_eq!(
    finished["notices"],
    json!(["published for web-app version 2.7.0"])
  );
  assert_eq!(finished["contracts"][1]["provider"], "stock-api");
}

#[test]
fn a_publication_the_broker_refuses_fails_every_contract_and_never_shows_a_credential() {
  let broker = Arc::new(FakeBroker {
    fail_publish: true,
    ..FakeBroker::default()
  });
  let mut engine = engine(Some(broker));
  let (_, events) = publish_contracts(&mut engine, vec![janus_contract()]);
  let finished = &events.last().unwrap()["payload"];
  assert_eq!(finished["status"], "failed");
  assert_eq!(finished["error"]["code"], "component-failed");
  assert_eq!(finished["error"]["details"]["component"], "broker");
  assert_eq!(finished["error"]["details"]["error"]["code"], "broker-refused");
  assert_eq!(finished["contracts"][0]["outcome"], "failed");
  assert_eq!(events[0]["payload"]["outcome"], "failed");
  let everything = serde_json::to_string(&events).unwrap();
  assert!(
    !everything.contains("s3cret-token") && !everything.contains("hunter2"),
    "{everything}"
  );
}

#[test]
fn contracts_from_two_consumers_are_refused_before_anything_is_published() {
  let broker = Arc::new(FakeBroker::default());
  let mut engine = engine(Some(Arc::clone(&broker)));
  let mut other = janus_contract();
  other["consumer"]["name"] = json!("mobile-app");
  let (refused, events) = publish_contracts(&mut engine, vec![janus_contract(), other]);
  assert_eq!(refused["error"]["code"], "malformed-frame", "{refused}");
  let message = refused["error"]["message"].as_str().unwrap();
  assert!(
    message.contains("contracts/1") && message.contains("mobile-app"),
    "{message}"
  );
  assert!(events.is_empty());
  assert!(broker.publications.lock().unwrap().is_empty());
}

#[test]
fn a_document_that_cannot_be_verified_cannot_be_published_and_is_named_by_its_position() {
  let broker = Arc::new(FakeBroker::default());
  let mut engine = engine(Some(Arc::clone(&broker)));
  let (not_a_contract, _) =
    publish_contracts(&mut engine, vec![janus_contract(), json!({ "hello": "world" })]);
  assert_eq!(
    not_a_contract["error"]["code"], "contract-version-unsupported",
    "{not_a_contract}"
  );
  assert_eq!(not_a_contract["error"]["details"]["index"], 1);

  let mut invalid = janus_contract();
  invalid["interactions"][0]["description"] = json!(7);
  let (invalid, _) = publish_contracts(&mut engine, vec![invalid]);
  assert_eq!(invalid["error"]["code"], "contract-invalid", "{invalid}");
  let pointer = invalid["error"]["details"]["problems"][0]["pointer"]
    .as_str()
    .unwrap();
  assert!(pointer.starts_with("/contracts/0/interactions/0"), "{pointer}");
  assert!(broker.publications.lock().unwrap().is_empty());
}
