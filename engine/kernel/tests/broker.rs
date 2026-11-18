//! Staged plan S1.3: `verification/verify` from a Pact Broker, with a broker client that is not a
//! real one (engine-protocol spec §8.3, ADR 0032). The client is the kernel's seam: what it
//! fetched, the run verifies; what the run concluded, it publishes. Faking it here pins the run's
//! side of that — pending, publishing, a fetch that fails, credentials kept out of every event —
//! without a broker; the native client is tested against a real one by the differential harness.
//! The same fake takes a consumer's contracts (`broker/publish-contracts`, spec §8.7), for the
//! engine's side of that: what it checks before anything is published, and what it reports. And it
//! takes and serves provider shapes and takes subsumption reports (`broker/publish-provider-shape`
//! and `broker/check-contracts`, ADR 0093): which shape is fetched for which contract, what a check
//! reports, and what is published with a report.

use pact_janus_component_http::HttpTransport;
use pact_janus_component_json::JsonContent;
use pact_janus_kernel::broker::{
  BrokerClient, BrokerDescriptor, ContractsPublished, ContractsToPublish, FetchRequest, FetchedPact,
  FetchedShape, PublishRequest, ReportToPublish, ShapeAt, ShapeRequest, ShapeToPublish,
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
  /// Each pact fetched by its URL: the URL, the origin it was reached at, and the credential's kind.
  urls: Mutex<Vec<Value>>,
  published: Mutex<Vec<Value>>,
  /// Each publication of contracts, with every contract's content parsed back.
  publications: Mutex<Vec<Value>>,
  /// The shape it serves for every provider, unless it serves none.
  shape: Option<FetchedShape>,
  fail_shapes: bool,
  fail_reports: bool,
  /// Each shape fetched: the provider and which of its shapes.
  shapes_fetched: Mutex<Vec<(String, ShapeAt)>>,
  /// Each shape published, its content parsed back.
  shape_publications: Mutex<Vec<Value>>,
  /// Each report published: the handle, the contract parsed back, and the report.
  reports: Mutex<Vec<Value>>,
}

fn refused(message: &str) -> ComponentError {
  ComponentError {
    code: "broker-refused".to_string(),
    category: "component".to_string(),
    message: message.to_string(),
    source: None,
    details: None,
    chain: None,
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

  /// A pact by its URL: the served pact whose consumer the URL's last segment names, publishable
  /// unless the URL says it is not a broker's.
  fn fetch_pact(&self, broker: &BrokerDescriptor, url: &str) -> Result<FetchedPact, ComponentError> {
    self.urls.lock().unwrap().push(json!({
      "url": url, "origin": broker.url, "auth": broker.auth.as_ref().map(|auth| auth.kind.clone()),
    }));
    if self.fail_fetch {
      return Err(refused("the broker answered 401"));
    }
    let consumer = url.rsplit('/').next().unwrap_or_default();
    let mut pact = self
      .pacts
      .iter()
      .find(|pact| pact.document["consumer"]["name"] == consumer)
      .cloned()
      .ok_or_else(|| refused("the server answered 404"))?;
    pact.pending = false;
    pact.publishable = !url.contains("/files/");
    Ok(pact)
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
        "janus": contract.janus,
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

  fn publish_provider_shape(
    &self,
    request: &ShapeToPublish<'_>,
  ) -> Result<ContractsPublished, ComponentError> {
    self.shape_publications.lock().unwrap().push(json!({
      "provider": request.provider,
      "provider-version": request.provider_version,
      "branch": request.branch,
      "tags": request.tags,
      "build-url": request.build_url,
      "content": serde_json::from_slice::<Value>(request.content).unwrap(),
    }));
    if self.fail_shapes {
      return Err(refused("the broker answered 401"));
    }
    Ok(ContractsPublished {
      notices: vec![format!("published a shape for {}", request.provider)],
    })
  }

  fn fetch_provider_shape(&self, request: &ShapeRequest<'_>) -> Result<Option<FetchedShape>, ComponentError> {
    self
      .shapes_fetched
      .lock()
      .unwrap()
      .push((request.provider.to_string(), request.at.clone()));
    if self.fail_shapes {
      return Err(refused("the broker answered 401"));
    }
    Ok(self.shape.clone())
  }

  fn publish_report(&self, request: &ReportToPublish<'_>) -> Result<(), ComponentError> {
    self.reports.lock().unwrap().push(json!({
      "handle": request.handle,
      "contract": serde_json::from_slice::<Value>(request.contract).unwrap(),
      "contract-ends-in-a-line-break": request.contract.ends_with(b"\n"),
      "report": request.report,
    }));
    if self.fail_reports {
      return Err(refused("the broker answered 500"));
    }
    Ok(())
  }
}

fn engine(broker: Option<Arc<FakeBroker>>) -> Engine {
  let mut transports: HashMap<String, Arc<dyn TransportComponent>> = HashMap::new();
  transports.insert("http".to_string(), Arc::new(HttpTransport::new()));
  let mut engine = Engine::with_components(transports, Some(Arc::new(JsonContent::new())));
  engine.register_background(Arc::new(pact_janus_kernel::protocol::Threads));
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
    publishable: true,
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

/// ADR 0062: a publication runs in the background, so a broker client alone is not enough.
#[test]
fn publishing_without_background_is_refused_and_reaches_no_broker() {
  let broker = Arc::new(FakeBroker::default());
  let mut engine = Engine::new();
  engine.register_broker_client(Arc::clone(&broker) as Arc<dyn BrokerClient>);
  let hello =
    json!({ "protocol-versions": [1], "host": { "name": "t", "version": "0" }, "capabilities": {} });
  send(&mut engine, "r-1", "engine/hello", hello);
  let (started, events) = publish_contracts(&mut engine, vec![janus_contract()]);
  assert_eq!(started["error"]["code"], "component-unavailable", "{started}");
  assert_eq!(
    started["error"]["details"],
    json!({ "component": "background" }),
    "{started}"
  );
  assert!(events.is_empty());
  assert!(broker.publications.lock().unwrap().is_empty());
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

/// A Janus contract for provider `p` with two recorded variants of one interaction, `base` and
/// `again`, both answered by [`provider`]: a filter on one of them narrows it.
fn contract_with_two_variants(consumer: &str) -> Value {
  let variant = |id: &str| {
    json!({
      "id": id, "origin": "base", "assignment": [],
      "parts": {
        "request": { "method": { "content": "GET" }, "path": { "content": "/widgets/1" } },
        "response": { "status": { "content": 200 } }
      }
    })
  };
  json!({
    "$format": "janus-contract/1",
    "consumer": { "name": consumer },
    "provider": { "name": "p" },
    "interactions": [{
      "description": "a request for widget 1",
      "transport": { "kind": "http", "mode": "passive" },
      "parts": {
        "request": { "method": { "shape": "equality", "example": "GET" },
                     "path": { "shape": "equality", "example": "/widgets/1" } },
        "response": { "status": { "shape": "equality", "example": 200 } }
      },
      "selection": {
        "variants": [variant("base"), variant("again")],
        "report": { "space": { "size": 2, "dimensions": 1 } }
      }
    }]
  })
}

/// ADR 0041 decision 4: a pact the filter narrowed is never published, and says why; one it did not
/// narrow is published as usual. `base` is every variant a v1–v4 pact records, so `web`'s run is
/// whole, while `mobile`'s contract also records `again`.
#[test]
fn a_pact_the_filter_narrowed_is_not_published_and_the_rest_are() {
  let broker = Arc::new(FakeBroker {
    pacts: vec![
      fetched(pact("web", json!({ "id": 1 })), false),
      fetched(contract_with_two_variants("mobile"), false),
    ],
    ..FakeBroker::default()
  });
  let mut engine = engine(Some(Arc::clone(&broker)));
  let events = verify(
    &mut engine,
    json!({}),
    json!({ "variants": ["base"], "publish": { "provider-version": "2.0.0" } }),
  );

  let summary = &events.last().unwrap()["payload"];
  assert_eq!(summary["status"], "filtered", "{summary}");
  assert_eq!(summary["pairs"][0]["status"], "verified");
  assert_eq!(summary["pairs"][1]["status"], "filtered");
  assert_eq!(
    summary["publication"],
    json!([
      { "consumer": "web", "outcome": "published" },
      { "consumer": "mobile", "outcome": "skipped", "reason": "filtered" }
    ])
  );
  let published = of_kind(&events, "verification/published");
  assert_eq!(published[1]["outcome"], "skipped");
  assert_eq!(published[1]["provider-version"], "2.0.0");

  let sent = broker.published.lock().unwrap();
  assert_eq!(sent.len(), 1, "only web's result reached the broker: {sent:?}");
  assert_eq!(sent[0]["handle"], json!({ "pact": "web" }));
  assert_eq!(sent[0]["success"], true);
}

/// ADR 0053: today's `--filter-consumer`, as `options.filter.consumers`. The pact it leaves out is
/// narrowed to nothing, so it is not published, and says why, as ADR 0041 decision 4 has it.
#[test]
fn a_pact_the_consumer_filter_leaves_out_is_not_published() {
  let broker = Arc::new(FakeBroker {
    pacts: vec![
      fetched(pact("web", json!({ "id": 1 })), false),
      fetched(pact("mobile", json!({ "id": 1 })), false),
    ],
    ..FakeBroker::default()
  });
  let mut engine = engine(Some(Arc::clone(&broker)));
  let events = verify(
    &mut engine,
    json!({}),
    json!({ "filter": { "consumers": ["web"] }, "publish": { "provider-version": "2.0.0" } }),
  );

  let summary = &events.last().unwrap()["payload"];
  assert_eq!(summary["status"], "filtered", "{summary}");
  assert_eq!(
    summary["publication"],
    json!([
      { "consumer": "web", "outcome": "published" },
      { "consumer": "mobile", "outcome": "skipped", "reason": "filtered" }
    ])
  );
  assert_eq!(broker.published.lock().unwrap().len(), 1);
}

/// ADR 0041 decision 1, for a broker source: the documents are only known once fetched, so the run
/// ends there, as one that fetched nothing does. Nothing runs and nothing is published — before,
/// this published a success for every pact, since a pact with no results had none that failed.
#[test]
fn a_filter_naming_a_variant_no_fetched_pact_records_ends_the_run_before_anything_runs() {
  let broker = Arc::new(FakeBroker {
    pacts: vec![fetched(pact("web", json!({ "id": 1 })), false)],
    ..FakeBroker::default()
  });
  let mut engine = engine(Some(Arc::clone(&broker)));
  let events = verify(
    &mut engine,
    json!({}),
    json!({ "variant": "nothing-is-called-this", "publish": { "provider-version": "2.0.0" } }),
  );

  assert_eq!(
    events
      .iter()
      .map(|event| event["kind"].as_str().unwrap())
      .collect::<Vec<_>>(),
    vec!["verification/fetched", "verification/finished"]
  );
  let summary = &events.last().unwrap()["payload"];
  assert_eq!(summary["status"], "failed");
  assert_eq!(summary["error"]["code"], "variant-not-found");
  assert_eq!(summary["error"]["details"]["variant"], "nothing-is-called-this");
  assert_eq!(summary["error"]["details"]["recorded"], json!(["base"]));
  assert!(broker.publications.lock().unwrap().is_empty());
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

// ---------------------------------------------------------------------------------------------
// Pacts by URL (staged plan S1.13, ADR 0053): today's --url and --webhook-callback-url
// ---------------------------------------------------------------------------------------------

/// A `urls` source fetches each pact by its URL, at that URL's origin with the run's credential, and
/// verifies it; a pact a broker served is published to its own link, and one from anywhere else is
/// not, and says so. Neither is pending.
#[test]
fn a_pact_fetched_by_url_is_verified_and_published_when_a_broker_served_it() {
  let broker = Arc::new(FakeBroker {
    pacts: vec![
      fetched(pact("web", json!({ "id": 1 })), true),
      fetched(pact("mobile", json!({ "id": 1 })), false),
    ],
    ..FakeBroker::default()
  });
  let mut engine = engine(Some(Arc::clone(&broker)));
  let events = verify(
    &mut engine,
    json!({
      "kind": "urls",
      "urls": ["https://broker.example/pacts/web", "https://files.example/files/mobile"],
      "auth": { "kind": "bearer", "token": "s3cret-token" },
    }),
    json!({ "publish": { "provider-version": "2.0.0" } }),
  );

  assert_eq!(
    *broker.urls.lock().unwrap(),
    vec![
      json!({ "url": "https://broker.example/pacts/web", "origin": "https://broker.example", "auth": "bearer" }),
      json!({ "url": "https://files.example/files/mobile", "origin": "https://files.example", "auth": "bearer" }),
    ]
  );
  assert!(broker.fetched.lock().unwrap().is_empty(), "nothing was selected");
  let fetched = of_kind(&events, "verification/fetched");
  assert_eq!(
    fetched[0]["pacts"][0]["pending"], false,
    "a pact by URL is never pending"
  );

  let summary = &events.last().unwrap()["payload"];
  assert_eq!(summary["status"], "verified", "{summary}");
  assert_eq!(
    summary["publication"],
    json!([
      { "consumer": "web", "outcome": "published" },
      { "consumer": "mobile", "outcome": "skipped", "reason": "not-from-a-broker" }
    ])
  );
  assert_eq!(broker.published.lock().unwrap().len(), 1);
}

#[test]
fn a_urls_source_needs_a_url_and_a_pact_that_cannot_be_fetched_ends_the_run() {
  let broker = Arc::new(FakeBroker {
    fail_fetch: true,
    ..FakeBroker::default()
  });
  let mut engine = engine(Some(Arc::clone(&broker)));
  let refused = send(
    &mut engine,
    "v-1",
    "verification/verify",
    json!({
      "source": { "kind": "urls", "urls": [] },
      "target": { "transports": [{ "transport": "http", "options": { "base-url": "http://127.0.0.1:1" } }] },
    }),
  );
  assert!(
    refused["error"]["message"].as_str().unwrap().contains("'urls'"),
    "{refused}"
  );

  let events = verify(
    &mut engine,
    json!({ "kind": "urls", "urls": ["https://broker.example/pacts/web"] }),
    json!({}),
  );
  let summary = &events.last().unwrap()["payload"];
  assert_eq!(summary["status"], "failed");
  assert_eq!(summary["error"]["code"], "component-failed", "{summary}");
}

#[test]
fn a_contract_is_marked_as_one_and_its_text_is_published_as_the_document() {
  let broker = Arc::new(FakeBroker::default());
  let mut engine = engine(Some(Arc::clone(&broker)));
  let pact = json!({
    "consumer": { "name": "web-app" }, "provider": { "name": "order-api" },
    "interactions": [{ "description": "a request", "request": { "method": "GET", "path": "/" },
                       "response": { "status": 200 } }],
    "metadata": { "pactSpecification": { "version": "3.0.0" } }
  });
  let text = serde_json::to_string_pretty(&janus_contract()).unwrap();
  let (_, events) = publish_contracts(&mut engine, vec![pact, json!(text)]);
  assert_eq!(events.last().unwrap()["payload"]["status"], "published");
  let publications = broker.publications.lock().unwrap();
  let contracts = &publications[0]["contracts"];
  assert_eq!(contracts[0]["janus"], false);
  assert_eq!(contracts[1]["janus"], true);
  // The contract the text holds, not the text as a JSON string.
  assert_eq!(contracts[1]["content"], janus_contract());
  assert_eq!(contracts[1]["starts"], r#"{"$format":"#);
}

// ---------------------------------------------------------------------------------------------
// broker/publish-provider-shape and broker/check-contracts (ADR 0093)
// ---------------------------------------------------------------------------------------------

fn sample(path: &str) -> Value {
  let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../..")
    .join(path);
  serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap()
}

/// The sample provider's v3 pact, from `web-app`.
fn order_pact() -> Value {
  sample("samples/order-service/pacts/web-app-order-service.json")
}

/// The shape the sample provider's own tests recorded.
fn order_shape() -> Value {
  sample("samples/order-service/shapes/order-service.provider-shape.json")
}

/// The shape as a broker serves it: the document with its links added, as a pact is served.
fn served_shape(reportable: bool) -> FetchedShape {
  let mut document = order_shape();
  let links = json!({
    "pb:provider-version": { "name": "4.1.0", "href": "/pacticipants/order-service/versions/4.1.0" },
    "pb:publish-subsumption-report": { "href": "/provider-shapes/abc/subsumption-reports" },
  });
  document["_links"] = links.clone();
  FetchedShape {
    document,
    provider_version: Some("4.1.0".to_string()),
    handle: links,
    reportable,
  }
}

const BROKER: &str = "https://user:hunter2@broker.example";

/// Start `op` with `body` (its broker added) and drain its stream: the response, then its events.
fn broker_session(engine: &mut Engine, op: &str, mut body: Value) -> (Value, Vec<Value>) {
  body["broker"] = json!({ "url": BROKER, "auth": { "kind": "bearer", "token": "s3cret-token" } });
  let started = send(engine, "b-1", op, body);
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

fn no_credential_in(events: &[Value]) {
  let everything = serde_json::to_string(events).unwrap();
  assert!(
    !everything.contains("s3cret-token") && !everything.contains("hunter2"),
    "{everything}"
  );
}

#[test]
fn a_shape_is_published_for_the_provider_it_names_as_the_host_sent_it() {
  let broker = Arc::new(FakeBroker::default());
  let mut engine = engine(Some(Arc::clone(&broker)));
  let (started, events) = broker_session(
    &mut engine,
    "broker/publish-provider-shape",
    json!({
      "provider-version": "4.1.0", "branch": "main", "tags": ["prod"],
      "build-url": "https://ci.example/9", "provider-shape": order_shape().to_string(),
    }),
  );
  assert!(
    started["ok"]["session"].as_str().unwrap().starts_with("bs-"),
    "{started}"
  );
  let published = broker.shape_publications.lock().unwrap();
  assert_eq!(published.len(), 1);
  assert_eq!(published[0]["provider"], "order-service");
  assert_eq!(published[0]["provider-version"], "4.1.0");
  assert_eq!(published[0]["branch"], "main");
  assert_eq!(published[0]["tags"], json!(["prod"]));
  assert_eq!(published[0]["build-url"], "https://ci.example/9");
  assert_eq!(published[0]["content"], order_shape());

  assert_eq!(events.len(), 1, "{events:?}");
  assert_eq!(events[0]["kind"], "broker/finished");
  assert_eq!(events[0]["last"], true);
  assert_eq!(
    events[0]["payload"],
    json!({
      "status": "published", "broker": "https://broker.example", "provider": "order-service",
      "provider-version": "4.1.0", "notices": ["published a shape for order-service"],
    })
  );
}

#[test]
fn a_shape_the_broker_refuses_fails_the_session_naming_the_broker_and_never_a_credential() {
  let broker = Arc::new(FakeBroker {
    fail_shapes: true,
    ..FakeBroker::default()
  });
  let mut engine = engine(Some(broker));
  let (_, events) = broker_session(
    &mut engine,
    "broker/publish-provider-shape",
    json!({ "provider-version": "4.1.0", "provider-shape": order_shape() }),
  );
  let finished = &events.last().unwrap()["payload"];
  assert_eq!(finished["status"], "failed");
  assert_eq!(finished["error"]["code"], "component-failed");
  assert_eq!(finished["error"]["details"]["component"], "broker");
  no_credential_in(&events);
}

#[test]
fn a_document_that_is_not_a_shape_is_refused_before_anything_is_published() {
  let broker = Arc::new(FakeBroker::default());
  let mut engine = engine(Some(Arc::clone(&broker)));
  let (refused, events) = broker_session(
    &mut engine,
    "broker/publish-provider-shape",
    json!({ "provider-version": "4.1.0", "provider-shape": janus_contract() }),
  );
  assert_eq!(refused["error"]["code"], "contract-invalid", "{refused}");
  assert!(
    refused["error"]["message"]
      .as_str()
      .unwrap()
      .contains("provider-shape"),
    "{refused}"
  );
  assert!(events.is_empty());

  let (refused, _) = broker_session(
    &mut engine,
    "broker/publish-provider-shape",
    json!({ "provider-version": " ", "provider-shape": order_shape() }),
  );
  assert_eq!(refused["error"]["code"], "malformed-frame", "{refused}");
  assert!(broker.shape_publications.lock().unwrap().is_empty());
}

#[test]
fn an_engine_with_no_broker_client_names_it_for_each_new_operation() {
  let mut engine = engine(None);
  for (op, body) in [
    (
      "broker/publish-provider-shape",
      json!({ "provider-version": "1", "provider-shape": order_shape() }),
    ),
    (
      "broker/check-contracts",
      json!({ "provider-branch": "main", "contracts": [order_pact()] }),
    ),
  ] {
    let (refused, _) = broker_session(&mut engine, op, body);
    assert_eq!(
      refused["error"]["code"], "component-unavailable",
      "{op}: {refused}"
    );
    assert_eq!(
      refused["error"]["details"],
      json!({ "component": "broker" }),
      "{op}"
    );
  }
}

/// The sample provider's pact against the shape its tests recorded, fetched from the broker: the
/// report `subsumption/check` gives for the same two documents, and the version the shape belongs to.
#[test]
fn a_contract_is_checked_against_the_shape_its_provider_published_at_a_branch() {
  let broker = Arc::new(FakeBroker {
    shape: Some(served_shape(true)),
    ..FakeBroker::default()
  });
  let mut engine = engine(Some(Arc::clone(&broker)));
  let (started, events) = broker_session(
    &mut engine,
    "broker/check-contracts",
    json!({ "provider-branch": "main", "contracts": [order_pact()] }),
  );
  assert!(
    started["ok"]["session"].as_str().unwrap().starts_with("bs-"),
    "{started}"
  );
  assert_eq!(
    *broker.shapes_fetched.lock().unwrap(),
    vec![("order-service".to_string(), ShapeAt::Branch("main".to_string()))]
  );
  let direct = send(
    &mut engine,
    "c-1",
    "subsumption/check",
    json!({ "contract": order_pact(), "provider-shape": order_shape() }),
  );

  let kinds: Vec<&str> = events.iter().map(|e| e["kind"].as_str().unwrap()).collect();
  assert_eq!(kinds, vec!["broker/checked", "broker/finished"]);
  let checked = &events[0]["payload"];
  assert_eq!(checked["consumer"], "web-app");
  assert_eq!(checked["provider"], "order-service");
  assert_eq!(checked["outcome"], "checked");
  assert_eq!(checked["provider-version"], "4.1.0");
  assert_eq!(checked["format"], direct["ok"]["format"]);
  assert_eq!(checked["report"], direct["ok"]["report"]);
  assert!(checked.get("publication").is_none(), "{checked}");
  let finished = &events[1]["payload"];
  assert_eq!(finished["status"], "checked");
  assert_eq!(finished["broker"], "https://broker.example");
  assert_eq!(finished["results"], json!([checked]));
  assert!(
    broker.reports.lock().unwrap().is_empty(),
    "nothing published unasked"
  );
}

#[test]
fn a_report_is_published_to_the_shapes_link_with_the_contract_it_is_about() {
  let broker = Arc::new(FakeBroker {
    shape: Some(served_shape(true)),
    ..FakeBroker::default()
  });
  let mut engine = engine(Some(Arc::clone(&broker)));
  let text = serde_json::to_string_pretty(&order_pact()).unwrap();
  let (_, events) = broker_session(
    &mut engine,
    "broker/check-contracts",
    json!({ "provider-version": "4.1.0", "contracts": [text], "publish-reports": true }),
  );
  assert_eq!(
    broker.shapes_fetched.lock().unwrap()[0].1,
    ShapeAt::Version("4.1.0".to_string())
  );
  let checked = &events[0]["payload"];
  assert_eq!(checked["publication"], json!({ "outcome": "published" }));
  let reports = broker.reports.lock().unwrap();
  assert_eq!(reports.len(), 1);
  assert_eq!(reports[0]["handle"], served_shape(true).handle);
  assert_eq!(reports[0]["contract"], order_pact());
  assert_eq!(reports[0]["contract-ends-in-a-line-break"], true);
  assert_eq!(reports[0]["report"], checked["report"]);
}

#[test]
fn a_report_the_broker_cannot_take_is_skipped_or_failed_and_the_check_still_stands() {
  for (shape, fail_reports, publication) in [
    (
      served_shape(false),
      false,
      json!({ "outcome": "skipped", "reason": "not-supported" }),
    ),
    (served_shape(true), true, json!("failed")),
  ] {
    let broker = Arc::new(FakeBroker {
      shape: Some(shape),
      fail_reports,
      ..FakeBroker::default()
    });
    let mut engine = engine(Some(Arc::clone(&broker)));
    let (_, events) = broker_session(
      &mut engine,
      "broker/check-contracts",
      json!({ "provider-branch": "main", "contracts": [order_pact()], "publish-reports": true }),
    );
    let checked = &events[0]["payload"];
    assert_eq!(checked["outcome"], "checked");
    if publication.is_string() {
      assert_eq!(checked["publication"]["outcome"], publication);
      assert_eq!(checked["publication"]["error"]["details"]["component"], "broker");
    } else {
      assert_eq!(checked["publication"], publication);
      assert!(broker.reports.lock().unwrap().is_empty());
    }
    assert_eq!(events[1]["payload"]["status"], "checked");
    no_credential_in(&events);
  }
}

/// A provider with no shape there is `not-published`, which the host decides as replay-only; and
/// a provider's shape is fetched once however many of its consumers' contracts are checked.
#[test]
fn a_provider_that_published_no_shape_is_not_published_and_is_asked_once() {
  let broker = Arc::new(FakeBroker::default());
  let mut engine = engine(Some(Arc::clone(&broker)));
  let mut mobile = order_pact();
  mobile["consumer"]["name"] = json!("mobile-app");
  let (_, events) = broker_session(
    &mut engine,
    "broker/check-contracts",
    json!({ "provider-branch": "main", "contracts": [order_pact(), mobile] }),
  );
  assert_eq!(broker.shapes_fetched.lock().unwrap().len(), 1);
  let checked = of_kind(&events, "broker/checked");
  assert_eq!(
    checked,
    vec![
      &json!({ "consumer": "web-app", "provider": "order-service", "outcome": "not-published" }),
      &json!({ "consumer": "mobile-app", "provider": "order-service", "outcome": "not-published" }),
    ]
  );
  assert_eq!(events.last().unwrap()["payload"]["status"], "checked");
}

#[test]
fn a_fetch_that_fails_fails_each_contract_and_the_session_and_never_shows_a_credential() {
  let broker = Arc::new(FakeBroker {
    fail_shapes: true,
    ..FakeBroker::default()
  });
  let mut engine = engine(Some(broker));
  let (_, events) = broker_session(
    &mut engine,
    "broker/check-contracts",
    json!({ "provider-branch": "main", "contracts": [order_pact()] }),
  );
  let checked = &events[0]["payload"];
  assert_eq!(checked["outcome"], "failed");
  assert_eq!(checked["error"]["code"], "component-failed");
  assert_eq!(checked["error"]["details"]["error"]["code"], "broker-refused");
  assert_eq!(events[1]["payload"]["status"], "failed");
  no_credential_in(&events);
}

#[test]
fn a_served_shape_about_another_provider_fails_that_contract_as_check_would() {
  let mut shape = served_shape(true);
  shape.document["provider"]["name"] = json!("billing-service");
  let broker = Arc::new(FakeBroker {
    shape: Some(shape),
    ..FakeBroker::default()
  });
  let mut engine = engine(Some(broker));
  let (_, events) = broker_session(
    &mut engine,
    "broker/check-contracts",
    json!({ "provider-branch": "main", "contracts": [order_pact()], "publish-reports": true }),
  );
  let checked = &events[0]["payload"];
  assert_eq!(checked["outcome"], "failed");
  assert_eq!(checked["error"]["code"], "document-mismatched", "{checked}");
  assert!(checked.get("publication").is_none());
  assert_eq!(events[1]["payload"]["status"], "failed");
}

#[test]
fn which_shape_is_said_once_and_a_version_belongs_to_one_provider() {
  let broker = Arc::new(FakeBroker::default());
  let mut engine = engine(Some(Arc::clone(&broker)));
  for body in [
    json!({ "contracts": [order_pact()] }),
    json!({ "provider-version": "1", "provider-branch": "main", "contracts": [order_pact()] }),
    json!({ "provider-branch": "main", "contracts": [] }),
    json!({ "provider-version": "1", "contracts": [order_pact(), janus_contract()] }),
  ] {
    let (refused, events) = broker_session(&mut engine, "broker/check-contracts", body.clone());
    assert_eq!(refused["error"]["code"], "malformed-frame", "{body}: {refused}");
    assert!(events.is_empty());
  }
  let (refused, _) = broker_session(
    &mut engine,
    "broker/check-contracts",
    json!({ "provider-branch": "main", "contracts": [order_pact(), { "hello": "world" }] }),
  );
  assert_eq!(
    refused["error"]["code"], "contract-version-unsupported",
    "{refused}"
  );
  assert!(broker.shapes_fetched.lock().unwrap().is_empty());
}
