//! ADR 0090 (staged plan S4.6): a provider shape of the messages a provider produces, recorded from
//! its own tests and checked against message contracts.
//!
//! A message has no request, so its operation is named by a selector over the message itself, its
//! metadata or a member of its body, and a synchronous message's by its request. The kind `message`
//! is read as `produce-message` hooks produce one, an unlabelled body by the type its metadata names,
//! and one session records a provider's HTTP answers and its messages into one shape. `janus check`
//! matches a message contract to an entry by selector, decoding a body a selector names, and decides
//! its `message` part as it decides a response.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use pact_janus_component_http::{HttpTransport, status};
use pact_janus_component_json::JsonContent;
use pact_janus_kernel::component::TransportComponent;
use pact_janus_kernel::protocol::Engine;
use pretty_assertions::assert_eq;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;

const ORDER_PLACED: &str = "../../corpora/verifier/messages/pact-js-order-placed/pact.json";
const REQUEST_REPLY: &str = "../../corpora/verifier/messages/pact-jvm-synchronous/pact.json";

// --- harness ------------------------------------------------------------------------------------

fn engine() -> Engine {
  let mut transports: HashMap<String, Arc<dyn TransportComponent>> = HashMap::new();
  transports.insert("http".to_string(), Arc::new(HttpTransport::new()));
  let mut engine = Engine::with_components(transports, Some(Arc::new(JsonContent::new())));
  engine.register_matcher(
    status::NAME,
    "1.0.0",
    status::ACTIONS,
    Arc::new(status::HttpMatcher::new()),
  );
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

fn create(engine: &mut Engine, provider: &str, kind: Option<&str>) -> String {
  let mut body = json!({ "provider": { "name": provider } });
  if let Some(kind) = kind {
    body["kind"] = json!(kind);
  }
  let created = send(engine, "provider-shape-session/create", body);
  created["ok"]["session"]
    .as_str()
    .unwrap_or_else(|| panic!("a session: {created}"))
    .to_string()
}

/// A message as a `produce-message` hook produces one: its metadata, and its body's octets, labelled
/// only when the producer gave a type.
fn message(metadata: Value, body: &Value, content_type: Option<&str>) -> Value {
  let mut slot = json!({ "content": BASE64.encode(body.to_string()), "encoded": "base64" });
  if let Some(content_type) = content_type {
    slot["content-type"] = json!(content_type);
  }
  json!({ "metadata": { "content": metadata }, "body": slot })
}

/// pact-js's message example's operation: what it publishes to `orders.placed`.
fn orders_placed() -> Value {
  json!({ "message": { "metadata": { "shape": "object", "members": {
    "queue": { "shape": "equality", "example": "orders.placed" } } } } })
}

fn order(id: &str, currency: &str, items: usize) -> Value {
  let items: Vec<Value> = (0..items)
    .map(|n| json!({ "productId": format!("prod-{n}"), "quantity": n + 1 }))
    .collect();
  json!({ "orderId": id, "customerId": "customer-001", "totalAmount": 149.95, "currency": currency,
          "items": items })
}

fn observe(engine: &mut Engine, body: Value) -> Value {
  send(engine, "provider-shape-session/observe", body)
}

fn observe_order(engine: &mut Engine, session: &str, order: &Value) -> Value {
  observe(
    engine,
    json!({ "session": session, "description": "publish orders.placed", "operation": orders_placed(),
            "parts": { "message": message(
              json!({ "queue": "orders.placed", "contentType": "application/json" }), order, None) } }),
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

fn pact(path: &str) -> Value {
  serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn check(engine: &mut Engine, contract: Value, shape: Value) -> Value {
  let checked = send(
    engine,
    "subsumption/check",
    json!({ "contract": contract, "provider-shape": shape }),
  );
  checked["ok"]["report"].clone()
}

fn finding_kinds(report: &Value) -> Vec<(String, String)> {
  report["interactions"]
    .as_array()
    .unwrap()
    .iter()
    .flat_map(|interaction| interaction["findings"].as_array().cloned().unwrap_or_default())
    .map(|finding| {
      (
        finding["path"].as_str().unwrap().to_string(),
        finding["kind"].as_str().unwrap().to_string(),
      )
    })
    .collect()
}

/// What the provider's own tests publish to `orders.placed`: orders in two currencies, one of them
/// with no items, which pact-js's consumer has only tested with at least one.
fn order_placed_shape(engine: &mut Engine) -> Value {
  let session = create(engine, "OrderProvider", Some("message"));
  for (n, (currency, items)) in [("GBP", 2), ("EUR", 0), ("GBP", 1), ("EUR", 3)]
    .into_iter()
    .enumerate()
  {
    let observed = observe_order(engine, &session, &order(&format!("order-{n}"), currency, items));
    assert!(observed["ok"].is_object(), "{observed}");
  }
  finalise(engine, &session)
}

// --- recording ----------------------------------------------------------------------------------

#[test]
fn a_message_is_recorded_by_the_operation_its_test_names() {
  let mut engine = engine();
  let shape = order_placed_shape(&mut engine);

  let entries = shape["interactions"].as_array().unwrap();
  assert_eq!(entries.len(), 1, "{shape:#}");
  let entry = &entries[0];
  assert_eq!(entry["description"], json!("publish orders.placed"));
  // Nothing tells a message's outcomes apart, so the selector is the operation as written.
  assert_eq!(entry["selector"], orders_placed());
  // Only the message: there is no request to an asynchronous one.
  let parts: Vec<&String> = entry["parts"].as_object().unwrap().keys().collect();
  assert_eq!(parts, vec!["message"]);
  // The body was unlabelled, and was decoded by the type its metadata names, as a verification
  // reads a produced message.
  let body = &entry["parts"]["message"]["body"];
  assert_eq!(body["shape"], json!("object"), "{body:#}");
  assert_eq!(body["members"]["items"]["shape"], json!("each-like"), "{body:#}");
  assert_eq!(body["members"]["items"]["min"], json!(0), "{body:#}");
  assert_eq!(
    entry["parts"]["message"]["metadata"]["members"]["queue"]["example"],
    json!("orders.placed")
  );
}

#[test]
fn a_message_its_operation_does_not_admit_is_refused() {
  let mut engine = engine();
  let session = create(&mut engine, "OrderProvider", Some("message"));
  let refused = observe(
    &mut engine,
    json!({ "session": session, "description": "publish orders.placed", "operation": orders_placed(),
            "parts": { "message": message(json!({ "queue": "orders.cancelled" }), &order("o", "GBP", 1),
                                          Some("application/json")) } }),
  );
  assert_eq!(
    refused["error"]["code"],
    json!("interaction-invalid"),
    "{refused}"
  );
  let problem = refused["error"]["details"]["problems"][0]["message"]
    .as_str()
    .unwrap();
  assert!(
    problem.starts_with("the operation does not admit the message the provider produced"),
    "{problem}"
  );
}

#[test]
fn one_session_records_answers_and_messages_each_by_its_own_kind() {
  let mut engine = engine();
  // The session's kind is HTTP's; the message says it is a message.
  let session = create(&mut engine, "OrderProvider", None);
  let answered = observe(
    &mut engine,
    json!({ "session": session, "description": "GET /orders/{id}",
            "operation": { "request": {
              "method": { "shape": "equality", "example": "GET" },
              "path": { "shape": "http:path", "template": "/orders/{id}", "example": "/orders/id" } } },
            "parts": { "response": {
              "status": { "content": 200 },
              "headers": { "content": { "content-type": ["application/json"],
                                        "set-cookie": ["session=secret"] } },
              "body": { "content": BASE64.encode(order("o", "GBP", 1).to_string()), "encoded": "base64",
                        "content-type": "application/json" } } } }),
  );
  assert!(answered["ok"].is_object(), "{answered}");
  let published = observe(
    &mut engine,
    json!({ "session": session, "kind": "message", "description": "publish orders.placed",
            "operation": orders_placed(),
            "parts": { "message": message(json!({ "queue": "orders.placed" }), &order("o", "GBP", 1),
                                          Some("application/json")) } }),
  );
  assert!(published["ok"].is_object(), "{published}");
  assert_eq!(published["ok"]["interactions"], json!(2));

  let shape = finalise(&mut engine, &session);
  let mut parts: Vec<String> = shape["interactions"]
    .as_array()
    .unwrap()
    .iter()
    .flat_map(|entry| entry["parts"].as_object().unwrap().keys().cloned())
    .collect();
  parts.sort();
  assert_eq!(parts, vec!["message", "response"], "{shape:#}");
  // HTTP's credentials are still withheld beside a kind that declares none.
  let response = shape["interactions"]
    .as_array()
    .unwrap()
    .iter()
    .find(|entry| entry["parts"].get("response").is_some())
    .unwrap();
  assert_eq!(
    response["parts"]["response"]["headers"]["members"]["set-cookie"]["example"],
    Value::Null,
    "{response:#}"
  );
}

#[test]
fn a_kind_nothing_serves_is_unavailable() {
  let mut engine = engine();
  let refused = send(
    &mut engine,
    "provider-shape-session/create",
    json!({ "provider": { "name": "OrderProvider" }, "kind": "kafka" }),
  );
  assert_eq!(
    refused["error"]["code"],
    json!("component-unavailable"),
    "{refused}"
  );

  let session = create(&mut engine, "OrderProvider", None);
  let refused = observe(
    &mut engine,
    json!({ "session": session, "kind": "kafka", "description": "publish",
            "parts": { "message": message(json!({}), &json!({}), Some("application/json")) } }),
  );
  assert_eq!(
    refused["error"]["code"],
    json!("component-unavailable"),
    "{refused}"
  );
}

// --- checking -----------------------------------------------------------------------------------

#[test]
fn a_message_contract_is_checked_against_the_shape_its_provider_recorded() {
  let mut engine = engine();
  let shape = order_placed_shape(&mut engine);

  // pact-js's consumer names the queue in its metadata, so its interaction selects the entry.
  let report = check(&mut engine, pact(ORDER_PLACED), shape);
  let interaction = &report["interactions"][0];
  assert_eq!(interaction["matched-by"], json!("selector"), "{report:#}");
  assert_eq!(interaction["description"], json!("an OrderPlaced event"));
  assert_eq!(
    finding_kinds(&report),
    vec![("message.body.items".to_string(), "wider-cardinality".to_string())],
    "{report:#}"
  );
}

#[test]
fn the_shape_every_sdk_records_is_matched_by_its_queue_and_finds_the_empty_list() {
  // S4.6's exit, across the boundary: `conformance/cases/live/recorded-message-shape.json` pins the
  // shape both SDKs write from tests that name only the queue. The conformance suite runs every SDK
  // as provider `orders-api`, so the pact is read as that provider's.
  let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
  let file = root.join("conformance/cases/live/recorded-message-shape.json");
  let case: Value = serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
  let shape = case["expect"]["shape"].clone();
  let mut contract = pact(ORDER_PLACED);
  contract["provider"]["name"] = json!("orders-api");

  let mut engine = engine();
  let report = check(&mut engine, contract, shape);
  assert_eq!(
    report["interactions"][0]["matched-by"],
    json!("selector"),
    "{report:#}"
  );
  assert_eq!(
    finding_kinds(&report),
    vec![("message.body.items".to_string(), "wider-cardinality".to_string())],
    "{report:#}"
  );
}

#[test]
fn a_selector_may_name_a_member_of_a_messages_body() {
  let mut engine = engine();
  let body_operation = json!({ "message": { "body": { "shape": "object", "members": {
    "orderId": { "shape": "regex", "pattern": "^order-", "example": "order-1" } } } } });
  let session = create(&mut engine, "OrderProvider", Some("message"));
  let observed = observe(
    &mut engine,
    json!({ "session": session, "description": "OrderPlaced", "operation": body_operation,
            "parts": { "message": message(json!({ "contentType": "application/json" }),
                                          &order("order-1", "GBP", 1), None) } }),
  );
  assert!(observed["ok"].is_object(), "{observed}");
  let shape = finalise(&mut engine, &session);

  // The contract records its body as JSON text: the selector is matched against the document it
  // decodes to, not the text.
  let report = check(&mut engine, pact(ORDER_PLACED), shape);
  assert_eq!(
    report["interactions"][0]["matched-by"],
    json!("selector"),
    "{report:#}"
  );
}

#[test]
fn a_synchronous_message_is_selected_by_its_request() {
  let mut engine = engine();
  let request_names = json!({ "request": { "body": { "shape": "object", "members": {
    "name": { "shape": "type", "example": "abcd123" } } } } });
  let session = create(&mut engine, "KafkaRequestReplyProvider", Some("message"));
  let observed = observe(
    &mut engine,
    json!({ "session": session, "description": "name lookup", "operation": request_names,
            "parts": {
              "request": message(json!({ "contentType": "application/json" }), &json!({ "name": "abcd123" }), None),
              "message": message(json!({ "contentType": "application/json" }), &json!({ "name": 321 }), None) } }),
  );
  assert!(observed["ok"].is_object(), "{observed}");
  let shape = finalise(&mut engine, &session);
  // A provider shape carries what the provider produced: the reply, not the request.
  let parts: Vec<&String> = shape["interactions"][0]["parts"]
    .as_object()
    .unwrap()
    .keys()
    .collect();
  assert_eq!(parts, vec!["message"], "{shape:#}");

  let report = check(&mut engine, pact(REQUEST_REPLY), shape);
  assert_eq!(
    report["interactions"][0]["matched-by"],
    json!("selector"),
    "{report:#}"
  );
  // The consumer has only tested a string name, and the provider answers a number.
  assert_eq!(
    finding_kinds(&report),
    vec![("message.body.name".to_string(), "broader-type".to_string())],
    "{report:#}"
  );
}
