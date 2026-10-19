//! Staged plan S1.3's second test layer: the client against responses recorded from the pinned
//! Pact Broker (`tools/verifier-diff/pact-broker.json`), offline and on every system.
//!
//! One scenario runs the client through what a consumer and a provider do between them: two
//! consumers publish (a v3 pact on `main`, a Janus contract on a feature branch), the provider
//! fetches with pending and work-in-progress pacts asked for, publishes a result for each, and
//! fetches again, now that one pact is no longer pending; then a consumer republishes changed
//! content for a version the broker already has, which it refuses.
//!
//! By default the scenario is replayed against `tests/recorded/pact-broker.json`: every request the
//! client makes must be the recorded one, in order, and what it concluded at each step must be what
//! it concluded against the broker. `RECORDINGS=overwrite`, with `JANUS_PACT_BROKER` naming the
//! pinned broker, records it again instead: CI does so in the job that starts the broker and fails
//! on any difference, so the recording cannot drift from the broker it claims to be. The weekly run
//! against the latest broker records against that one with `JANUS_PACT_BROKER_ANY_VERSION=1`, which
//! waives the check that a recording is of the pinned version, and fails on any other difference.
//!
//! A recording is normalised so the same broker gives the same file: the broker's URL is written as
//! `http://pact-broker.invalid` (the client fetches every link from the broker it was given, so a
//! replay is served from its own), timestamps as one fixed instant, and the engine's version in a
//! published result as `{engine-version}`. The broker's database ids, which no deletion resets (a
//! verification result's number, and the consumer version id inside a pact link's base64
//! `metadata` segment), are numbered from 1 in the order they first appear, in requests and
//! responses alike, so every link the client follows is still one the recording served. The
//! scenario's pacticipants are deleted before it is recorded, so it starts from nothing however
//! often it runs.

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD as BASE64_URL;
use pact_janus_broker_client::HalBrokerClient;
use pact_janus_kernel::broker::{
  BrokerClient, BrokerDescriptor, ContractToPublish, ContractsToPublish, FetchRequest, FetchedPact,
  InteractionResult, PublishRequest,
};
use pact_janus_kernel::component::ComponentError;
use pretty_assertions::assert_eq;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

const WEB: &str = "recorded-web";
const MOBILE: &str = "recorded-mobile";
const PROVIDER: &str = "recorded-order-api";

/// How the broker is written in a recording.
const RECORDED_BROKER: &str = "http://pact-broker.invalid";
const RECORDED_INSTANT: &str = "2000-01-01T00:00:00+00:00";

fn recording_path() -> PathBuf {
  PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/recorded/pact-broker.json")
}

// ---------------------------------------------------------------------------------------------
// The scenario
// ---------------------------------------------------------------------------------------------

fn web_pact(status: &str) -> Value {
  json!({
    "consumer": { "name": WEB },
    "provider": { "name": PROVIDER },
    "interactions": [{
      "description": "a request for order 1",
      "providerState": "order 1 exists",
      "request": { "method": "GET", "path": "/orders/1" },
      "response": {
        "status": 200,
        "headers": { "Content-Type": "application/json" },
        "body": { "id": 1, "status": status }
      }
    }],
    "metadata": { "pactSpecification": { "version": "3.0.0" } }
  })
}

fn mobile_contract() -> Value {
  json!({
    "$format": "janus-contract/1",
    "consumer": { "name": MOBILE },
    "provider": { "name": PROVIDER },
    "interactions": [{
      "description": "a request for order 1's tracking",
      "transport": { "kind": "http", "mode": "passive" },
      "parts": {
        "request": { "method": { "shape": "equality", "example": "GET" },
                     "path": { "shape": "equality", "example": "/orders/1/tracking" } },
        "response": { "status": { "shape": "equality", "example": 200 } }
      },
      "selection": {
        "variants": [{
          "id": "base", "origin": "base", "assignment": [],
          "parts": {
            "request": { "method": { "content": "GET" }, "path": { "content": "/orders/1/tracking" } },
            "response": { "status": { "content": 200 } }
          }
        }],
        "report": { "space": { "size": 1, "dimensions": 0 } }
      }
    }]
  })
}

/// What the client concluded at one step, in a form a recording can hold.
fn concluded<T>(result: Result<T, ComponentError>, ok: impl FnOnce(T) -> Value) -> Value {
  match result {
    Ok(value) => ok(value),
    Err(error) => json!({
      "error": error.code,
      "status": error.details.as_ref().map(|details| details["status"].clone()),
      "message": error.message,
    }),
  }
}

fn fetched(pacts: Vec<FetchedPact>) -> Value {
  let mut pacts: Vec<Value> = pacts
    .into_iter()
    .map(|pact| {
      json!({
        "consumer": pact.document["consumer"]["name"],
        "pending": pact.pending,
        "wip": pact.wip,
        "notices": pact.notices,
        "interaction-ids": pact.document["interactions"].as_array().into_iter().flatten()
          .map(|interaction| interaction["_id"].is_string()).collect::<Vec<_>>(),
      })
    })
    .collect();
  pacts.sort_by_key(|pact| pact["consumer"].as_str().unwrap_or_default().to_string());
  json!({ "fetched": pacts })
}

fn publish(
  client: &HalBrokerClient,
  broker: &BrokerDescriptor,
  consumer: &str,
  version: &str,
  branch: &str,
  document: Value,
) -> Value {
  let contracts = [ContractToPublish {
    provider: PROVIDER.to_string(),
    content: serde_json::to_vec(&document).unwrap(),
  }];
  concluded(
    client.publish_contracts(&ContractsToPublish {
      broker,
      consumer,
      consumer_version: version,
      branch: Some(branch),
      tags: &[],
      build_url: Some("https://ci.example/1"),
      contracts: &contracts,
    }),
    |published| json!({ "notices": published.notices }),
  )
}

fn fetch(client: &HalBrokerClient, broker: &BrokerDescriptor) -> Result<Vec<FetchedPact>, ComponentError> {
  client.fetch_for_verification(&FetchRequest {
    broker: broker.clone(),
    provider: PROVIDER.to_string(),
    selectors: vec![json!({ "mainBranch": true })],
    provider_branch: Some("main".to_string()),
    provider_tags: vec![],
    include_pending: true,
    include_wip_since: Some("2020-01-01".to_string()),
  })
}

/// The scenario, against the broker at `url`: each step's name and what the client concluded.
fn scenario(url: &str, step: &mut dyn FnMut(&str, Value)) {
  let client = HalBrokerClient::new();
  let broker: BrokerDescriptor = serde_json::from_value(json!({ "url": url })).unwrap();

  step(
    "web publishes a v3 pact on main",
    publish(&client, &broker, WEB, "1.0.0", "main", web_pact("SHIPPED")),
  );
  step(
    "mobile publishes a Janus contract on a feature branch",
    publish(
      &client,
      &broker,
      MOBILE,
      "2.0.0",
      "feat/tracking",
      mobile_contract(),
    ),
  );

  let pacts = fetch(&client, &broker);
  let handles: Vec<(String, Value)> = pacts
    .as_ref()
    .map(|pacts| {
      pacts
        .iter()
        .map(|pact| {
          (
            pact.document["consumer"]["name"]
              .as_str()
              .unwrap_or_default()
              .to_string(),
            pact.handle.clone(),
          )
        })
        .collect()
    })
    .unwrap_or_default();
  step(
    "the provider fetches from main: web's pact is pending, mobile's is work in progress",
    concluded(pacts, fetched),
  );

  let tags = vec!["prod".to_string()];
  for (consumer, handle) in &handles {
    let success = consumer == WEB;
    let result = client.publish_results(&PublishRequest {
      broker: &broker,
      handle,
      provider_version: "3.0.0",
      branch: Some("main"),
      tags: &tags,
      build_url: Some("https://ci.example/2"),
      success,
      results: vec![InteractionResult {
        id: None,
        description: "the only interaction".to_string(),
        success,
        mismatches: if success {
          vec![]
        } else {
          vec![json!({ "path": "$.response.status", "message": "expected 200 but was 404" })]
        },
        error: None,
      }],
    });
    step(
      &format!("the provider publishes {consumer}'s result"),
      concluded(result, |()| json!("published")),
    );
  }

  step(
    "the provider fetches again: web's pact is no longer pending",
    concluded(fetch(&client, &broker), fetched),
  );
  step(
    "web republishes changed content for the version the broker has",
    publish(&client, &broker, WEB, "1.0.0", "main", web_pact("CANCELLED")),
  );
}

// ---------------------------------------------------------------------------------------------
// Recording and replaying
// ---------------------------------------------------------------------------------------------

/// One request and the broker's answer, as a recording holds them.
type Exchanges = Arc<Mutex<Vec<Value>>>;

/// The response headers the client reads; the rest would only make a recording noisier.
const KEPT_HEADERS: [&str; 3] = ["content-type", "accept-encoding", "retry-after"];

/// A request body as the recording compares it: the engine's own version is not the broker's
/// business, and changes every release.
fn normalised_request(body: &str) -> Value {
  let mut body: Value = match serde_json::from_str(body) {
    Ok(body) => body,
    Err(_) if body.is_empty() => return Value::Null,
    Err(_) => return Value::String(body.to_string()),
  };
  if body["verifiedBy"]["version"].is_string() {
    body["verifiedBy"]["version"] = json!("{engine-version}");
  }
  body
}

/// What a recording writes in place of what differs from one recording to the next.
struct Normaliser {
  /// The broker's URL, and the proxy's, each written as [`RECORDED_BROKER`].
  brokers: Vec<String>,
  /// Each kind of database id, by the id the broker gave, to the number it is recorded as.
  ids: HashMap<&'static str, HashMap<u64, u64>>,
}

impl Normaliser {
  fn new(brokers: &[&str]) -> Self {
    Normaliser {
      brokers: brokers
        .iter()
        .map(|broker| broker.trim_end_matches('/').to_string())
        .collect(),
      ids: HashMap::new(),
    }
  }

  fn id(&mut self, kind: &'static str, id: &str) -> String {
    let table = self.ids.entry(kind).or_default();
    let next = table.len() as u64 + 1;
    let id: u64 = id.parse().unwrap_or_default();
    table.entry(id).or_insert(next).to_string()
  }

  /// Text from the broker (a body, a path, what the client concluded from them), normalised.
  fn text(&mut self, text: &str) -> String {
    let mut text = self.brokers.iter().fold(text.to_string(), |text, broker| {
      text.replace(broker.as_str(), RECORDED_BROKER)
    });
    let timestamps =
      regex::Regex::new(r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?(Z|[+-]\d{2}:?\d{2})").unwrap();
    text = timestamps.replace_all(&text, RECORDED_INSTANT).to_string();
    let results = regex::Regex::new(r"(verification-results/|Verification result )(\d+)").unwrap();
    text = results
      .replace_all(&text, |found: &regex::Captures| {
        format!("{}{}", &found[1], self.id("verification", &found[2]))
      })
      .to_string();
    // A pact link's `metadata` segment is base64 of a query string that names the consumer
    // version by its id (`s[][cv]=153`).
    let metadata = regex::Regex::new(r"/metadata/([A-Za-z0-9_-]+)").unwrap();
    let consumer_version = regex::Regex::new(r"(cv%5D=)(\d+)").unwrap();
    metadata
      .replace_all(&text, |found: &regex::Captures| {
        let Some(decoded) = BASE64_URL
          .decode(&found[1])
          .ok()
          .and_then(|bytes| String::from_utf8(bytes).ok())
        else {
          return found[0].to_string();
        };
        let renumbered = consumer_version
          .replace_all(&decoded, |id: &regex::Captures| {
            format!("{}{}", &id[1], self.id("consumer-version", &id[2]))
          })
          .to_string();
        format!("/metadata/{}", BASE64_URL.encode(renumbered))
      })
      .to_string()
  }

  /// A body as a recording holds it: JSON when it is JSON.
  fn body(&mut self, body: &str) -> Value {
    let body = self.text(body);
    serde_json::from_str(&body).unwrap_or(Value::String(body))
  }
}

/// A recording proxy: where it listens, what it recorded, the broker version it saw, and the
/// normaliser it wrote with.
struct Proxy {
  url: String,
  exchanges: Exchanges,
  version: Arc<Mutex<Option<String>>>,
  normaliser: Arc<Mutex<Normaliser>>,
}

/// A proxy in front of the broker at `broker` that records every exchange.
fn recording_proxy(broker: &str) -> Proxy {
  let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
  let url = format!("http://{}", server.server_addr().to_ip().unwrap());
  let exchanges: Exchanges = Arc::default();
  let version: Arc<Mutex<Option<String>>> = Arc::default();
  let normaliser = Arc::new(Mutex::new(Normaliser::new(&[broker, &url])));
  let (record, seen_version, broker) = (Arc::clone(&exchanges), Arc::clone(&version), broker.to_string());
  let normalise = Arc::clone(&normaliser);
  std::thread::spawn(move || {
    let agent: ureq::Agent = ureq::Agent::config_builder()
      .http_status_as_error(false)
      .build()
      .into();
    for mut request in server.incoming_requests() {
      let mut body = Vec::new();
      let _ = request.as_reader().read_to_end(&mut body);
      let path = request.url().to_string();
      let method = request.method().as_str().to_string();
      let mut forwarded = ureq::http::Request::builder()
        .method(method.as_str())
        .uri(format!("{}{path}", broker.trim_end_matches('/')));
      for header in request.headers() {
        let name = header.field.as_str().as_str().to_ascii_lowercase();
        if ["accept", "authorization", "content-type", "content-encoding"].contains(&name.as_str()) {
          forwarded = forwarded.header(name, header.value.as_str());
        }
      }
      let mut response = agent.run(forwarded.body(body.clone()).unwrap()).unwrap();
      let status = response.status().as_u16();
      if let Some(found) = response.headers().get("x-pact-broker-version") {
        *seen_version.lock().unwrap() = found.to_str().ok().map(str::to_string);
      }
      let headers: Vec<(String, String)> = KEPT_HEADERS
        .iter()
        .filter_map(|name| {
          response
            .headers()
            .get(*name)
            .and_then(|value| value.to_str().ok())
            .map(|value| (name.to_string(), value.to_string()))
        })
        .collect();
      let text = response.body_mut().read_to_string().unwrap_or_default();
      let mut normaliser = normalise.lock().unwrap();
      record.lock().unwrap().push(json!({
        "request": {
          "method": method,
          "path": normaliser.text(&path),
          "body": normalised_request(&String::from_utf8_lossy(&body)),
        },
        "response": {
          "status": status,
          "headers": headers.iter().map(|(name, value)| (name.clone(), json!(value)))
            .collect::<serde_json::Map<String, Value>>(),
          "body": normaliser.body(&text),
        },
      }));
      drop(normaliser);
      let mut answer = tiny_http::Response::from_string(text).with_status_code(status);
      for (name, value) in &headers {
        answer =
          answer.with_header(tiny_http::Header::from_bytes(name.as_bytes(), value.as_bytes()).unwrap());
      }
      let _ = request.respond(answer);
    }
  });
  Proxy {
    url,
    exchanges,
    version,
    normaliser,
  }
}

/// A broker that is the recording: it answers each request with the next recorded answer, and
/// notes every request that is not the recorded one.
fn replay(exchanges: Vec<Value>) -> (String, Arc<Mutex<Vec<String>>>) {
  let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
  let url = format!("http://{}", server.server_addr().to_ip().unwrap());
  let mismatches: Arc<Mutex<Vec<String>>> = Arc::default();
  let noted = Arc::clone(&mismatches);
  std::thread::spawn(move || {
    let mut recorded = exchanges.into_iter();
    for mut request in server.incoming_requests() {
      let mut body = String::new();
      let _ = request.as_reader().read_to_string(&mut body);
      let asked = json!({
        "method": request.method().as_str(),
        "path": request.url(),
        "body": normalised_request(&body),
      });
      let Some(exchange) = recorded.next() else {
        noted
          .lock()
          .unwrap()
          .push(format!("a request after the recording ended: {asked}"));
        let _ = request.respond(tiny_http::Response::from_string("{}").with_status_code(418));
        continue;
      };
      if asked != exchange["request"] {
        noted
          .lock()
          .unwrap()
          .push(format!("expected {}\n     got {asked}", exchange["request"]));
        // Not a status the client retries, so one difference is reported once.
        let _ = request.respond(tiny_http::Response::from_string("{}").with_status_code(418));
        continue;
      }
      let response = &exchange["response"];
      let body = match &response["body"] {
        Value::String(text) => text.clone(),
        other => other.to_string(),
      };
      let mut answer = tiny_http::Response::from_string(body)
        .with_status_code(response["status"].as_u64().unwrap_or(200) as u16);
      for (name, value) in response["headers"].as_object().into_iter().flatten() {
        answer = answer.with_header(
          tiny_http::Header::from_bytes(name.as_bytes(), value.as_str().unwrap_or_default().as_bytes())
            .unwrap(),
        );
      }
      let _ = request.respond(answer);
    }
  });
  (url, mismatches)
}

/// Delete the scenario's pacticipants, and everything of theirs, so a recording starts from
/// nothing. Only on the broker a recording is made against, which is a test's own.
fn start_afresh(broker: &str) {
  let agent: ureq::Agent = ureq::Agent::config_builder()
    .http_status_as_error(false)
    .build()
    .into();
  for name in [WEB, MOBILE, PROVIDER] {
    let url = format!("{}/pacticipants/{name}", broker.trim_end_matches('/'));
    let status = agent.delete(&url).call().map(|r| r.status().as_u16());
    assert!(
      matches!(status, Ok(200..=299 | 404)),
      "deleting {name} from {broker}: {status:?}"
    );
  }
}

fn record(broker: &str) {
  start_afresh(broker);
  let Proxy {
    url,
    exchanges,
    version,
    normaliser,
  } = recording_proxy(broker);
  let mut steps = Vec::new();
  scenario(&url, &mut |name, outcome| {
    let exchanges = std::mem::take(&mut *exchanges.lock().unwrap());
    let outcome = normaliser.lock().unwrap().body(&outcome.to_string());
    steps.push(json!({ "step": name, "exchanges": exchanges, "outcome": outcome }));
  });
  let recording = json!({
    "pact-broker-version": *version.lock().unwrap(),
    "steps": steps,
  });
  let path = recording_path();
  std::fs::create_dir_all(path.parent().unwrap()).unwrap();
  let mut text = serde_json::to_string_pretty(&recording).unwrap();
  text.push('\n');
  std::fs::write(&path, text).unwrap();
  eprintln!("recorded {} against {broker}", path.display());
}

#[test]
fn the_client_does_what_it_did_against_the_pinned_broker() {
  if std::env::var("RECORDINGS").as_deref() == Ok("overwrite") {
    let broker = std::env::var("JANUS_PACT_BROKER")
      .expect("RECORDINGS=overwrite records against a broker: name it with JANUS_PACT_BROKER");
    record(&broker);
  }

  let recording: Value =
    serde_json::from_str(&std::fs::read_to_string(recording_path()).expect("the recording")).expect("json");
  let pinned: Value = serde_json::from_str(
    &std::fs::read_to_string(
      PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tools/verifier-diff/pact-broker.json"),
    )
    .expect("the broker pin"),
  )
  .expect("json");
  // The weekly run against the latest broker records against it, to see what moved; nothing it
  // records is committed.
  if std::env::var("JANUS_PACT_BROKER_ANY_VERSION").as_deref() != Ok("1") {
    assert_eq!(
      recording["pact-broker-version"], pinned["broker-version"],
      "the recording is of another broker than the pinned one: record it again (RECORDINGS=overwrite)"
    );
  }

  let steps = recording["steps"].as_array().expect("steps");
  let exchanges: Vec<Value> = steps
    .iter()
    .flat_map(|step| step["exchanges"].as_array().cloned().unwrap_or_default())
    .collect();
  let (url, mismatches) = replay(exchanges);
  let mut concluded = Vec::new();
  scenario(&url, &mut |name, outcome| {
    // What was served is normalised already: only the replay's own URL is left to write as the
    // recording does.
    let outcome = outcome.to_string().replace(&url, RECORDED_BROKER);
    concluded.push(json!({ "step": name, "outcome": serde_json::from_str::<Value>(&outcome).unwrap() }));
  });

  let mismatches = mismatches.lock().unwrap();
  assert!(
    mismatches.is_empty(),
    "the client's requests differ from the recording:\n{}",
    mismatches.join("\n")
  );
  let recorded: Vec<Value> = steps
    .iter()
    .map(|step| json!({ "step": step["step"], "outcome": step["outcome"] }))
    .collect();
  assert_eq!(concluded, recorded);
}
