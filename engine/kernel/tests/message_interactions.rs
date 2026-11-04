//! ADR 0067: a message interaction in a Janus contract. A consumer session records an emissive
//! interaction by handing each variant's message to the host, whose test reports the verdict, and the
//! contract it writes is verified by asking a provider's producer for the message, as a pact's is.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use pact_janus_kernel::component::TransportComponent;
use pact_janus_kernel::protocol::Engine;
use pretty_assertions::assert_eq;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;

fn send(engine: &mut Engine, op: &str, body: Value) -> Value {
  let request = json!({ "type": "request", "id": "r", "op": op, "body": body });
  let bytes = engine.dispatch(&serde_json::to_vec(&request).unwrap());
  serde_json::from_slice(&bytes).unwrap()
}

fn ok<'a>(response: &'a Value, op: &str) -> &'a Value {
  response
    .get("ok")
    .unwrap_or_else(|| panic!("{op} failed: {response}"))
}

fn engine() -> Engine {
  let mut transports: HashMap<String, Arc<dyn TransportComponent>> = HashMap::new();
  transports.insert(
    "http".to_string(),
    Arc::new(pact_janus_component_http::HttpTransport::new()),
  );
  let mut engine = Engine::with_components(
    transports,
    Some(Arc::new(pact_janus_component_json::JsonContent::new())),
  );
  engine.register_background(Arc::new(pact_janus_kernel::protocol::Threads));
  ok(
    &send(
      &mut engine,
      "engine/hello",
      json!({ "protocol-versions": [1], "host": { "name": "janus-test", "version": "0.0.0" },
              "capabilities": { "test-verdicts": {} } }),
    ),
    "engine/hello",
  );
  engine
}

/// An order-placed event whose `note` may be absent: two variants.
fn order_placed() -> Value {
  json!({
    "description": "an order was placed",
    "transport": { "kind": "message", "mode": "emissive" },
    "states": [{ "name": "an order exists" }],
    "parts": { "message": {
      "metadata": { "shape": "object", "members": {
        "queue": { "shape": "equality", "example": "orders.placed" } } },
      "body": { "shape": "object", "members": {
        "id": { "shape": "type", "example": 42 },
        "note": { "shape": "optional", "of": { "shape": "type", "example": "fragile" } } } },
    } },
    "content-types": { "message": { "body": "application/json" } },
  })
}

fn session(engine: &mut Engine, interaction: Value) -> (String, String, Vec<Value>) {
  let created = send(
    engine,
    "consumer-session/create",
    json!({ "config": { "consumer": { "name": "shipping" }, "provider": { "name": "orders" } } }),
  );
  let session = ok(&created, "create")["session"].as_str().unwrap().to_string();
  let added = send(
    engine,
    "consumer-session/add-interaction",
    json!({ "session": session, "interaction": interaction }),
  );
  let handle = ok(&added, "add-interaction")["handle"]
    .as_str()
    .unwrap()
    .to_string();
  let selected = send(
    engine,
    "consumer-session/variants",
    json!({ "session": session, "handle": handle }),
  );
  let variants = ok(&selected, "variants")["variants"].as_array().unwrap().clone();
  (session, handle, variants)
}

fn absent(variant: &Value) -> bool {
  variant["assignment"]
    .as_array()
    .is_some_and(|assignment| assignment.iter().any(|entry| entry["point"] == json!("absent")))
}

/// A JSON body slot as a transport carries it: octets, base64, labelled.
fn body_document(slot: &Value) -> Value {
  assert_eq!(slot["encoded"], json!("base64"), "{slot}");
  assert_eq!(slot["content-type"], json!("application/json"), "{slot}");
  let octets = BASE64.decode(slot["content"].as_str().unwrap()).unwrap();
  serde_json::from_slice(&octets).unwrap()
}

/// ADR 0067 decision 5: with no transport started, `serve-variant` answers each variant's message as
/// a transport would carry it, the body encoded by its declared type and the metadata as it stands.
/// The test's reports decide the variants, and the contract records the interaction as emissive.
#[test]
fn a_consumer_session_hands_each_variant_its_message_and_records_the_contract() {
  let mut engine = engine();
  let (session, handle, variants) = session(&mut engine, order_placed());
  assert_eq!(variants.len(), 2, "note present, and absent");

  for variant in &variants {
    let served = send(
      &mut engine,
      "consumer-session/serve-variant",
      json!({ "session": session, "handle": handle, "variant": variant["id"] }),
    );
    let parts = &ok(&served, "serve-variant")["parts"];
    assert_eq!(
      parts["message"]["metadata"],
      json!({ "content": { "queue": "orders.placed" } })
    );
    let body = body_document(&parts["message"]["body"]);
    assert_eq!(body["id"], json!(42));
    assert_eq!(body.get("note").is_none(), absent(variant), "{body}");

    let reported = send(
      &mut engine,
      "consumer-session/report-variant",
      json!({ "session": session, "handle": handle, "variant": variant["id"], "status": "passed" }),
    );
    assert_eq!(
      ok(&reported, "report-variant")["result"]["status"],
      json!("verified")
    );
  }

  let finalised = send(
    &mut engine,
    "consumer-session/finalise",
    json!({ "session": session }),
  );
  let contract = &ok(&finalised, "finalise")["contract"];
  let interaction = &contract["interactions"][0];
  assert_eq!(
    interaction["transport"],
    json!({ "kind": "message", "mode": "emissive" })
  );
  assert_eq!(interaction["selection"]["variants"].as_array().unwrap().len(), 2);
}

/// A message variant served and never reported is not verified, as any variant is (ADR 0064), and
/// one the test failed withholds the contract: the engine handed the message, the test decides.
#[test]
fn a_message_the_test_failed_withholds_the_contract() {
  let mut engine = engine();
  let (session, handle, variants) = session(&mut engine, order_placed());
  let first = &variants[0]["id"];
  ok(
    &send(
      &mut engine,
      "consumer-session/serve-variant",
      json!({ "session": session, "handle": handle, "variant": first }),
    ),
    "serve-variant",
  );
  let reported = send(
    &mut engine,
    "consumer-session/report-variant",
    json!({ "session": session, "handle": handle, "variant": first, "status": "failed",
            "reason": "the handler threw" }),
  );
  assert_eq!(
    ok(&reported, "report-variant")["result"]["status"],
    json!("failed")
  );
  let finalised = send(
    &mut engine,
    "consumer-session/finalise",
    json!({ "session": session }),
  );
  assert!(ok(&finalised, "finalise").get("contract").is_none());
}

/// ADR 0067 decision 2: a synchronous message hands the test both halves, its request and the
/// message that answers it.
#[test]
fn a_synchronous_message_hands_the_test_its_request_and_its_answer() {
  let mut engine = engine();
  let message = |member: &str, example: Value| json!({ "body": { "shape": "object", "members": { member: { "shape": "equality", "example": example } } } });
  let (session, handle, variants) = session(
    &mut engine,
    json!({
      "description": "a square's area",
      "transport": { "kind": "message", "mode": "emissive" },
      "parts": { "request": message("side", json!(3)), "message": message("area", json!(9)) },
      "content-types": { "request": { "body": "application/json" }, "message": { "body": "application/json" } },
    }),
  );
  let served = send(
    &mut engine,
    "consumer-session/serve-variant",
    json!({ "session": session, "handle": handle, "variant": variants[0]["id"] }),
  );
  let parts = &ok(&served, "serve-variant")["parts"];
  assert_eq!(body_document(&parts["request"]["body"]), json!({ "side": 3 }));
  assert_eq!(body_document(&parts["message"]["body"]), json!({ "area": 9 }));
}

/// The staged plan's S2.12 exit: a consumer session records a message interaction, and a provider
/// then verifies the contract it wrote. The producer, a script here as a provider test's would be, is
/// asked once per variant with today's request for a message, and answers with the variant's message.
#[test]
fn a_recorded_message_contract_is_verified_through_its_producer() {
  let mut engine = engine();
  let (session, handle, variants) = session(&mut engine, order_placed());
  for variant in &variants {
    for (op, extra) in [
      ("consumer-session/serve-variant", json!({})),
      ("consumer-session/report-variant", json!({ "status": "passed" })),
    ] {
      let mut body = json!({ "session": session, "handle": handle, "variant": variant["id"] });
      body
        .as_object_mut()
        .unwrap()
        .extend(extra.as_object().unwrap().clone());
      ok(&send(&mut engine, op, body), op);
    }
  }
  let finalised = send(
    &mut engine,
    "consumer-session/finalise",
    json!({ "session": session }),
  );
  let contract = ok(&finalised, "finalise")["contract"].clone();

  // The producer leaves `note` out for the variant that recorded it absent, and reads what it was
  // asked for from today's request, as an SDK's message producer does.
  let producer = |note: bool| {
    let source = format!(
      "function hook(ctx) {{ \
         const asked = ctx.interaction['pact-message'].request; \
         if (asked.description !== 'an order was placed' || asked.providerStates[0].name !== 'an order exists') {{ \
           return {{ outcome: 'failed', error: {{ code: 'unexpected-request', message: JSON.stringify(asked) }} }}; \
         }} \
         const absent = ctx.variant.assignment.some(entry => entry.point === 'absent'); \
         const body = absent || !{note} ? {{ id: 7 }} : {{ id: 7, note: 'handle with care' }}; \
         return {{ outcome: 'ok', changes: {{ parts: {{ message: {{ \
           metadata: {{ content: {{ queue: 'orders.placed' }} }}, \
           body: {{ content: JSON.stringify(body), encoded: 'text', 'content-type': 'application/json' }} }} }} }} }}; \
       }}"
    );
    json!({ "version": 1, "hooks": { "produce-message": [{
      "name": "in-test", "changes": ["parts"], "run": { "kind": "script", "source": source },
    }] } })
  };
  let results = |engine: &mut Engine, hooks: Value| {
    let started = send(
      engine,
      "verification/verify",
      json!({ "source": { "kind": "inline", "contracts": [contract.clone()] },
              "target": { "transports": [], "hooks": hooks } }),
    );
    let stream = ok(&started, "verify")["stream"].as_str().unwrap().to_string();
    let mut events: Vec<Value> = Vec::new();
    while events.last().is_none_or(|event| event["last"] != json!(true)) {
      let polled = send(
        engine,
        "events/poll",
        json!({ "streams": [stream], "wait-ms": 5_000 }),
      );
      events.extend(ok(&polled, "events/poll")["events"].as_array().unwrap().clone());
    }
    events
      .into_iter()
      .filter(|event| event["kind"] == json!("verification/interaction-result"))
      .map(|event| event["payload"]["status"].clone())
      .collect::<Vec<_>>()
  };

  assert_eq!(
    results(&mut engine, producer(true)),
    vec![json!("verified"), json!("verified")]
  );
  // A producer that never writes `note` has not produced the variant that recorded it present.
  let without_note = results(&mut engine, producer(false));
  assert_eq!(
    without_note
      .iter()
      .filter(|status| **status == json!("failed"))
      .count(),
    1
  );
}
