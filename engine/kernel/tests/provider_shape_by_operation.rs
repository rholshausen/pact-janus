//! ADR 0086 (staged plan S4.1): a provider shape recorded from the provider's own tests, which name
//! the operation each answer belongs to and never a consumer's words.
//!
//! The protocol half: an observation is an arrival, its body octets decoded by the content registry
//! and its credentials withheld; the transport's outcome slots keep one operation's outcomes apart;
//! an operation that does not admit the request it answered is refused; and `finalise` writes the
//! shape from every source's evidence. The sample provider half drives the real provider (task 5.6)
//! the way its own tests would, and checks the recording against the pact a consumer has with it,
//! matched by operation.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use pact_janus_component_http::{HttpTransport, status};
use pact_janus_component_json::JsonContent;
use pact_janus_kernel::component::TransportComponent;
use pact_janus_kernel::contract::files::{ContractWriter, Next};
use pact_janus_kernel::protocol::{Engine, EngineError};
use pact_janus_sample_order_service::{Config, DEFAULT_TOKEN, start};
use pretty_assertions::assert_eq;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

const SHAPE: &str = "/shapes/order-service.provider-shape.json";
const EVIDENCE: &str = "/build/janus/evidence/order-service.json";

// --- harness ------------------------------------------------------------------------------------

/// Files in memory. The map's lock is not held while `next` runs, because finalising writes the shape
/// from inside the evidence file's update: the files are two, as the native writer's locks are.
#[derive(Default)]
struct Files(Mutex<HashMap<String, Vec<u8>>>);

impl Files {
  fn read(&self, path: &str) -> Option<Value> {
    let files = self.0.lock().unwrap();
    files
      .get(path)
      .map(|bytes| serde_json::from_slice(bytes).unwrap())
  }
}

struct Writer(Arc<Files>);

impl ContractWriter for Writer {
  fn update(&self, path: &str, next: &mut Next<'_>) -> Result<(), EngineError> {
    let existing = self.0.0.lock().unwrap().get(path).cloned().unwrap_or_default();
    let bytes = next(&existing)?;
    self.0.0.lock().unwrap().insert(path.to_string(), bytes);
    Ok(())
  }
}

fn engine(files: Option<&Arc<Files>>) -> Engine {
  let mut transports: HashMap<String, Arc<dyn TransportComponent>> = HashMap::new();
  transports.insert("http".to_string(), Arc::new(HttpTransport::new()));
  let mut engine = Engine::with_components(transports, Some(Arc::new(JsonContent::new())));
  engine.register_matcher(
    status::NAME,
    "1.0.0",
    status::ACTIONS,
    Arc::new(status::HttpMatcher::new()),
  );
  if let Some(files) = files {
    engine.register_contract_writer(Arc::new(Writer(Arc::clone(files))));
  }
  send(
    &mut engine,
    "engine/hello",
    json!({ "protocol-versions": [1], "host": { "name": "janus-test", "version": "0.0.0" } }),
  );
  engine
}

fn send(engine: &mut Engine, op: &str, body: Value) -> Value {
  let request = json!({ "type": "request", "id": "r", "op": op, "body": body });
  let bytes = engine.dispatch(&serde_json::to_vec(&request).unwrap());
  serde_json::from_slice(&bytes).unwrap()
}

fn create(engine: &mut Engine) -> String {
  let created = send(
    engine,
    "provider-shape-session/create",
    json!({ "provider": { "name": "order-service" } }),
  );
  created["ok"]["session"].as_str().expect("a session").to_string()
}

/// `GET /orders/{id}`, as both SDKs write it: a method, and the HTTP component's path template.
fn get_order() -> Value {
  json!({ "request": {
    "method": { "shape": "equality", "example": "GET" },
    "path": { "shape": "http:path", "template": "/orders/{id}", "example": "/orders/id" } } })
}

/// A response as the HTTP transport presents one (component-interfaces spec §5.9): a status, the
/// headers' field lines by lower-cased name, and the body's octets labelled with its type.
fn response(status: u16, headers: Value, body: &[u8], content_type: Option<&str>) -> Value {
  let mut body = json!({ "content": BASE64.encode(body), "encoded": "base64" });
  if let Some(content_type) = content_type {
    body["content-type"] = json!(content_type);
  }
  json!({ "status": { "content": status }, "headers": { "content": headers }, "body": body })
}

fn observe(engine: &mut Engine, session: &str, path: &str, answer: Value) -> Value {
  send(
    engine,
    "provider-shape-session/observe",
    json!({ "session": session, "description": "GET /orders/{id}", "operation": get_order(),
            "parts": { "request": { "method": { "content": "GET" }, "path": { "content": path } },
                       "response": answer } }),
  )
}

fn json_response(status: u16, body: Value) -> Value {
  response(
    status,
    json!({ "content-type": ["application/json"] }),
    body.to_string().as_bytes(),
    Some("application/json"),
  )
}

fn finalise(engine: &mut Engine, session: &str) -> Value {
  let finalised = send(
    engine,
    "provider-shape-session/finalise",
    json!({ "session": session }),
  );
  finalised["ok"]["provider-shape"].clone()
}

fn entry(shape: &Value, status: u16) -> &Value {
  shape["interactions"]
    .as_array()
    .unwrap()
    .iter()
    .find(|entry| entry["selector"]["response"]["status"]["example"] == json!(status))
    .unwrap_or_else(|| panic!("no entry for {status} in {shape:#}"))
}

// --- an observation names its operation, and is read as an arrival -------------------------------

#[test]
fn an_observation_records_its_operation_as_the_entrys_selector_with_its_outcome() {
  let mut engine = engine(None);
  let session = create(&mut engine);
  let observed = observe(
    &mut engine,
    &session,
    "/orders/66",
    json_response(200, json!({ "id": "66", "status": "PENDING" })),
  );
  assert_eq!(observed["ok"], json!({ "observations": 1, "interactions": 1 }));

  let shape = finalise(&mut engine, &session);
  let entry = &shape["interactions"][0];
  assert_eq!(entry["description"], "GET /orders/{id}");
  assert_eq!(
    entry["selector"],
    json!({ "request": get_order()["request"],
            "response": { "status": { "shape": "equality", "example": 200 } } }),
    "the operation the test named, and the status it was answered with"
  );
  assert_eq!(
    entry["parts"]["response"]["body"]["members"]["status"],
    json!({ "shape": "equality", "example": "PENDING" }),
    "the body's octets were decoded as the JSON they were labelled as"
  );
  assert!(
    entry["parts"].get("request").is_none(),
    "a provider shape carries what the provider produced, never the request (design 2.8 §2.1)"
  );
}

#[test]
fn one_operations_outcomes_are_recorded_apart_and_neither_widens_the_other() {
  let mut engine = engine(None);
  let session = create(&mut engine);
  for status in ["PENDING", "SHIPPED", "CANCELLED"] {
    observe(
      &mut engine,
      &session,
      "/orders/66",
      json_response(200, json!({ "id": "66", "status": status })),
    );
  }
  let progress = observe(
    &mut engine,
    &session,
    "/orders/999",
    json_response(404, json!({ "error": "not found", "id": "999" })),
  );
  assert_eq!(progress["ok"]["interactions"], json!(2));

  let shape = finalise(&mut engine, &session);
  let found = &entry(&shape, 200)["parts"]["response"]["body"]["members"];
  assert_eq!(found["id"], json!({ "shape": "equality", "example": "66" }));
  assert!(
    found.get("error").is_none(),
    "the 404's members are not the 200's"
  );
  let missing = &entry(&shape, 404)["parts"]["response"]["body"]["members"];
  assert_eq!(
    missing["error"],
    json!({ "shape": "equality", "example": "not found" })
  );
}

#[test]
fn an_operation_that_does_not_admit_the_request_it_answered_is_refused() {
  // A test that named the wrong template would otherwise record an entry no consumer selects.
  let mut engine = engine(None);
  let session = create(&mut engine);
  let refused = observe(
    &mut engine,
    &session,
    "/customers/66",
    json_response(200, json!({ "id": "66" })),
  );
  assert_eq!(refused["error"]["code"], "interaction-invalid");
  let problems = &refused["error"]["details"]["problems"];
  assert_eq!(problems[0]["pointer"], "/operation");
  assert_eq!(
    problems[0]["message"],
    "the operation does not admit the request the provider answered: Expected a path matching the \
     template '/orders/{id}' but was '/customers/66'"
  );
}

#[test]
fn an_operation_that_is_not_a_shape_document_is_refused_where_it_is_wrong() {
  let mut engine = engine(None);
  let session = create(&mut engine);
  let refused = send(
    &mut engine,
    "provider-shape-session/observe",
    json!({ "session": session, "description": "GET /orders/{id}",
            "operation": { "request": { "method": { "shape": "no-such-operator" } } },
            "parts": { "response": json_response(200, json!({})) } }),
  );
  assert_eq!(refused["error"]["code"], "interaction-invalid");
  assert_eq!(
    refused["error"]["details"]["problems"][0]["pointer"],
    "/operation/request/method"
  );
}

#[test]
fn a_credential_the_transport_declares_is_recorded_as_present_and_nothing_more() {
  let mut engine = engine(None);
  let session = create(&mut engine);
  observe(
    &mut engine,
    &session,
    "/orders/66",
    response(
      200,
      json!({ "content-type": ["application/json"], "set-cookie": ["session=s3cr3t; HttpOnly"] }),
      b"{}",
      Some("application/json"),
    ),
  );
  let shape = finalise(&mut engine, &session);
  let headers = &shape["interactions"][0]["parts"]["response"]["headers"]["members"];
  assert_eq!(headers["set-cookie"], json!({ "shape": "any" }));
  assert!(
    !shape.to_string().contains("s3cr3t"),
    "no value of a credential reaches the shape (ADR 0086 decision 6)"
  );
}

#[test]
fn a_body_that_does_not_decode_as_labelled_is_recorded_as_its_octets_with_a_warning() {
  let mut engine = engine(None);
  let session = create(&mut engine);
  let observed = observe(
    &mut engine,
    &session,
    "/orders/66",
    response(200, json!({}), b"{ not json", Some("application/json")),
  );
  let warnings = observed["ok"]["warnings"].as_array().expect("a warning");
  assert!(
    warnings[0].as_str().unwrap().starts_with(
      "response.body: the value labelled 'application/json' could not be decoded, so its octets were recorded"
    ),
    "{warnings:?}"
  );
  let shape = finalise(&mut engine, &session);
  assert_eq!(
    shape["interactions"][0]["parts"]["response"]["body"],
    json!({ "shape": "equality", "example": BASE64.encode(b"{ not json") })
  );
}

#[test]
fn a_kind_no_transport_serves_cannot_be_recorded() {
  let mut engine = engine(None);
  let refused = send(
    &mut engine,
    "provider-shape-session/create",
    json!({ "provider": { "name": "order-service" }, "kind": "grpc" }),
  );
  assert_eq!(refused["error"]["code"], "component-unavailable");
  assert_eq!(refused["error"]["details"]["component"], "grpc");
}

// --- the engine's host writes the shape, from every source's evidence ----------------------------

fn write(engine: &mut Engine, session: &str, source: &str) -> Value {
  send(
    engine,
    "provider-shape-session/finalise",
    json!({ "session": session,
            "write": { "path": SHAPE, "evidence": EVIDENCE, "source": source } }),
  )
}

/// One session recording `statuses` as answers to `GET /orders/66`.
fn record(engine: &mut Engine, statuses: &[&str]) -> String {
  let session = create(engine);
  for status in statuses {
    observe(
      engine,
      &session,
      "/orders/66",
      json_response(200, json!({ "id": "66", "status": status })),
    );
  }
  session
}

#[test]
fn sources_written_one_after_another_add_up_to_one_sessions_shape() {
  let files = Arc::new(Files::default());
  let mut engine = engine(Some(&files));
  let first = record(&mut engine, &["PENDING", "SHIPPED"]);
  let written = write(&mut engine, &first, "orders.test.ts");
  assert_eq!(written["ok"]["written"], SHAPE);
  assert_eq!(Some(written["ok"]["provider-shape"].clone()), files.read(SHAPE));
  let second = record(&mut engine, &["PENDING", "CANCELLED"]);
  write(&mut engine, &second, "cancellations.test.ts");

  let mut alone = self::engine(None);
  let session = record(&mut alone, &["PENDING", "SHIPPED", "PENDING", "CANCELLED"]);
  assert_eq!(
    files.read(SHAPE).expect("the shape is written"),
    finalise(&mut alone, &session),
    "two test files write the shape one session recording both would have"
  );
  let evidence = files.read(EVIDENCE).expect("the evidence is kept");
  assert_eq!(evidence["$format"], "janus-provider-shape-evidence/1");
  assert_eq!(
    evidence["sources"]
      .as_object()
      .unwrap()
      .keys()
      .collect::<Vec<_>>(),
    vec!["cancellations.test.ts", "orders.test.ts"]
  );
}

#[test]
fn a_source_recorded_again_replaces_its_evidence_rather_than_adding_to_it() {
  // Counts that added up on every run would change the recorder's judgements with how often a
  // suite had run: three distinct values in three observations is an open domain, in six a set.
  let files = Arc::new(Files::default());
  let mut engine = engine(Some(&files));
  for _ in 0..2 {
    let session = record(&mut engine, &["PENDING", "SHIPPED", "CANCELLED"]);
    write(&mut engine, &session, "orders.test.ts");
  }
  let shape = files.read(SHAPE).unwrap();
  assert_eq!(shape["interactions"][0]["source"]["observations"], json!(3));
}

#[test]
fn evidence_about_another_provider_is_refused_and_left_as_it_was() {
  let files = Arc::new(Files::default());
  let other = json!({ "$format": "janus-provider-shape-evidence/1",
                      "provider": { "name": "billing" }, "sources": {} });
  files
    .0
    .lock()
    .unwrap()
    .insert(EVIDENCE.to_string(), other.to_string().into_bytes());
  let mut engine = engine(Some(&files));
  let session = record(&mut engine, &["PENDING"]);
  let refused = write(&mut engine, &session, "orders.test.ts");
  assert_eq!(refused["error"]["code"], "contract-conflict");
  assert_eq!(files.read(EVIDENCE), Some(other));
  assert_eq!(files.read(SHAPE), None);
}

#[test]
fn a_write_with_no_writer_to_do_it_is_refused() {
  let mut engine = engine(None);
  let session = record(&mut engine, &["PENDING"]);
  let refused = write(&mut engine, &session, "orders.test.ts");
  assert_eq!(refused["error"]["code"], "component-unavailable");
  assert_eq!(refused["error"]["details"]["component"], "contract-writer");
}

// --- the sample provider, recorded by operation and checked against its consumer's pact -----------

/// What the sample provider answers to `GET <path>`, as an observation's response part.
fn call(base_url: &str, path: &str) -> Value {
  let agent: ureq::Agent = ureq::Agent::config_builder()
    .http_status_as_error(false)
    .build()
    .into();
  let mut answer = agent
    .get(format!("{base_url}{path}"))
    .header("authorization", &format!("Bearer {DEFAULT_TOKEN}"))
    .call()
    .expect("the provider answers");
  let mut headers: serde_json::Map<String, Value> = serde_json::Map::new();
  for (name, value) in answer.headers() {
    let lines = headers
      .entry(name.as_str().to_ascii_lowercase())
      .or_insert_with(|| json!([]));
    lines.as_array_mut().unwrap().push(json!(value.to_str().unwrap()));
  }
  let content_type = answer
    .headers()
    .get("content-type")
    .map(|value| value.to_str().unwrap().to_string());
  let status = answer.status().as_u16();
  let body = answer.body_mut().read_to_vec().expect("a body");
  response(status, Value::Object(headers), &body, content_type.as_deref())
}

fn set_state(base_url: &str, state: Value) {
  let agent: ureq::Agent = ureq::Agent::config_builder()
    .http_status_as_error(false)
    .build()
    .into();
  agent
    .post(format!("{base_url}/_pact/provider-states"))
    .header("content-type", "application/json")
    .send(serde_json::to_string(&state).unwrap())
    .expect("the provider answers");
}

/// The provider's own tests: set up an order, fetch it, and once fetch one that does not exist. No
/// test names a consumer's interaction, or a provider state.
fn record_the_sample_provider(engine: &mut Engine) -> Value {
  let provider = start(Config::default()).expect("an OS-assigned port always binds");
  let base_url = provider.base_url();
  let session = create(engine);
  for (status, shipped, items) in [
    ("PENDING", false, 1),
    ("SHIPPED", true, 2),
    ("PENDING", false, 3),
    ("CANCELLED", false, 0),
    ("SHIPPED", true, 1),
    ("PENDING", false, 2),
  ] {
    set_state(
      base_url,
      json!({ "action": "setup", "state": "an order exists",
              "params": { "id": "66", "status": status, "shipped": shipped, "items": items } }),
    );
    let observed = observe(engine, &session, "/orders/66", call(base_url, "/orders/66"));
    assert!(observed.get("ok").is_some(), "{observed:#}");
  }
  set_state(base_url, json!({ "action": "setup", "state": "no orders exist" }));
  observe(engine, &session, "/orders/999", call(base_url, "/orders/999"));
  finalise(engine, &session)
}

fn the_consumers_pact() -> Value {
  let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    .join("../../samples/order-service/pacts/web-app-order-service.json");
  serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

#[test]
fn a_shape_recorded_by_operation_is_matched_to_the_consumers_pact_and_finds_its_variance() {
  // S4.1's exit, in the engine: tests that never name a consumer's interaction record a shape, and
  // the check matches it to the consumer's pact by operation and finds the `CANCELLED` M5 found.
  let mut engine = engine(None);
  let shape = record_the_sample_provider(&mut engine);
  let checked = send(
    &mut engine,
    "subsumption/check",
    json!({ "contract": the_consumers_pact(), "provider-shape": shape }),
  );
  let interactions = checked["ok"]["report"]["interactions"]
    .as_array()
    .unwrap_or_else(|| panic!("{checked:#}"));

  let found = &interactions[0];
  assert_eq!(found["description"], "a request for an order");
  assert_eq!(found["matched-by"], "selector");
  let status = finding(found, "response.body.status").expect("a finding at the status");
  assert_eq!(status["kind"], "wider-values");
  assert_eq!(
    status["provider"]["summary"],
    "one of 'CANCELLED' | 'PENDING' | 'SHIPPED'"
  );
  // What the provider sent as its `Content-Type`, against the pact's `http:media-type`, is decided:
  // the HTTP component's matcher admits it (S4.7, ADR 0087), so there is nothing to report.
  assert_eq!(finding(found, "response.headers.content-type[0]"), None);

  let missing = &interactions[1];
  assert_eq!(
    missing["description"],
    "a request for an order that does not exist"
  );
  assert_eq!(
    missing["matched-by"], "selector",
    "the 404 the consumer expects selects the entry the provider's 404 was recorded in"
  );
  assert_eq!(
    missing["findings"],
    json!([]),
    "nothing the 404 consumer has not tested"
  );
}

fn finding<'a>(result: &'a Value, path: &str) -> Option<&'a Value> {
  result["findings"]
    .as_array()
    .unwrap()
    .iter()
    .find(|finding| finding["path"] == path)
}

#[test]
fn an_entry_about_the_interactions_states_is_used_over_one_about_any_state() {
  // ADR 0086 decision 5: two entries select the interaction, and the one naming its states wins.
  let mut engine = engine(None);
  let session = create(&mut engine);
  for (states, status) in [(json!([]), "PENDING"), (json!(["an order exists"]), "SHIPPED")] {
    send(
      &mut engine,
      "provider-shape-session/observe",
      json!({ "session": session, "description": "GET /orders/{id}", "states": states,
              "operation": get_order(),
              "parts": { "response": json_response(200, json!({ "id": "66", "status": status })) } }),
    );
  }
  let shape = finalise(&mut engine, &session);
  assert_eq!(shape["interactions"].as_array().unwrap().len(), 2);
  let checked = send(
    &mut engine,
    "subsumption/check",
    json!({ "contract": the_consumers_pact(), "provider-shape": shape }),
  );
  let found = &checked["ok"]["report"]["interactions"][0];
  assert_eq!(found["matched-by"], "selector", "{checked:#}");
  assert_eq!(
    finding(found, "response.body.status"),
    None,
    "the stateful entry only ever saw SHIPPED, which the consumer's pact records, so nothing about \
     the PENDING the stateless entry saw is reported"
  );
}

#[test]
fn the_shape_every_sdk_records_is_matched_by_operation_and_finds_its_variance() {
  // S4.1's exit, across the boundary: `conformance/cases/live/recorded-shape.json` pins the shape both
  // SDKs write from tests that name only the operation. Checked against the sample provider's consumer
  // pact, the pair the demo and M5 use, it is matched by the operation and finds the CANCELLED status
  // the consumer never tested. The conformance suite runs every SDK as provider `orders-api`, so the
  // pact is read as that provider's. (The demo's own contract is written when the demo runs, and is
  // not in the repository to read here.)
  let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
  let read = |path: &str| -> Value {
    let file = root.join(path);
    serde_json::from_slice(&std::fs::read(&file).unwrap_or_else(|err| panic!("{}: {err}", file.display())))
      .unwrap()
  };
  let shape = read("conformance/cases/live/recorded-shape.json")["expect"]["shape"].clone();
  let mut pact = read("samples/order-service/pacts/web-app-order-service.json");
  pact["provider"]["name"] = json!("orders-api");

  let mut engine = engine(None);
  let checked = send(
    &mut engine,
    "subsumption/check",
    json!({ "contract": pact, "provider-shape": shape }),
  );
  let found = &checked["ok"]["report"]["interactions"][0];
  assert_eq!(found["description"], "a request for an order", "{checked:#}");
  assert_eq!(found["matched-by"], "selector");
  let status = finding(found, "response.body.status").unwrap_or_else(|| panic!("{checked:#}"));
  assert_eq!(
    status["provider"]["summary"],
    "one of 'CANCELLED' | 'PENDING' | 'SHIPPED'"
  );
  assert_eq!(
    checked["ok"]["report"]["interactions"][1]["matched-by"],
    "selector"
  );
}
