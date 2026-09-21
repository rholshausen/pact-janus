//! Plan tasks 4.1 and 4.3: the engine protocol's session lifecycle (`create`/`add-interaction`/
//! `finalise`, engine-protocol spec §8.2) and variant machinery (`variants`/`serve-variant`,
//! variant-semantics spec §3.9/§4.1) — frame dispatch, structured errors, per-interaction
//! results. Scenarios are drawn from the spec's own worked examples
//! (`examples/consumer-http-session.md`, `examples/error-and-negotiation.md`). `start-transport`
//! is not dispatched yet — no transport is bound to a consumer session (that gap is 4.2's
//! component crate waiting on its own wiring, tracked for whoever picks it up next).

use pact_janus_kernel::protocol::Engine;
use serde_json::{Value, json};

/// Send one request frame, return the decoded response frame.
fn send(engine: &mut Engine, id: &str, op: &str, body: Value) -> Value {
  let request = json!({ "type": "request", "id": id, "op": op, "body": body });
  let bytes = engine.dispatch(&serde_json::to_vec(&request).expect("a json! literal always serializes"));
  serde_json::from_slice(&bytes).expect("Engine::dispatch always returns valid JSON")
}

fn hello(engine: &mut Engine) -> Value {
  send(
    engine,
    "r-1",
    "engine/hello",
    json!({ "protocol-versions": [1], "host": { "name": "janus-test", "version": "0.0.0" }, "capabilities": {} }),
  )
}

// The RFC order interaction, per shape-language spec examples/order-payload.md §8 and
// tests/interaction_spec.rs's own fixture for it — the document consumer-session/add-interaction
// actually carries (`parts`, and shapes authored `{"shape": "<operator>", ...}`; the
// engine-protocol spec's own worked examples are illustrative sketches, not this literal shape,
// per their header notes).
fn order_interaction() -> Value {
  json!({
    "description": "a request for an order",
    "transport": { "kind": "http", "mode": "passive" },
    "parts": {
      "request": { "method": { "shape": "equality", "example": "GET" },
                   "path": { "shape": "equality", "example": "/orders/66" } },
      "response": { "status": { "shape": "equality", "example": 200 },
                    "body": { "shape": "object", "members": {} } } }
  })
}

#[test]
fn happy_path_create_add_interaction_finalise() {
  let mut engine = Engine::new();

  let ok = hello(&mut engine);
  assert_eq!(ok["ok"]["protocol-version"], 1);

  let create = send(
    &mut engine,
    "r-2",
    "consumer-session/create",
    json!({ "config": { "consumer": { "name": "web-app" }, "provider": { "name": "order-api" } } }),
  );
  let session = create["ok"]["session"].as_str().unwrap().to_string();
  assert!(!session.is_empty());

  let added = send(
    &mut engine,
    "r-3",
    "consumer-session/add-interaction",
    json!({ "session": session, "interaction": order_interaction() }),
  );
  let handle = added["ok"]["handle"].as_str().unwrap().to_string();
  assert_eq!(handle, "i-1");

  let finalised = send(
    &mut engine,
    "r-4",
    "consumer-session/finalise",
    json!({ "session": session }),
  );
  assert_eq!(
    finalised["ok"],
    json!({ "results": [ { "handle": "i-1", "status": "not-exercised" } ] }),
    "nothing was ever served (no serve-variant yet), so the result is honestly not-exercised \
     and no contract is present"
  );
  assert!(finalised["ok"].get("contract").is_none());
}

#[test]
fn finalise_reports_results_in_submission_order() {
  let mut engine = Engine::new();
  hello(&mut engine);
  let create = send(
    &mut engine,
    "r-2",
    "consumer-session/create",
    json!({ "config": { "consumer": { "name": "web-app" }, "provider": { "name": "order-api" } } }),
  );
  let session = create["ok"]["session"].as_str().unwrap().to_string();

  let mut first = order_interaction();
  first["description"] = json!("first");
  let mut second = order_interaction();
  second["description"] = json!("second");

  let h1 = send(
    &mut engine,
    "r-3",
    "consumer-session/add-interaction",
    json!({ "session": session, "interaction": first }),
  );
  let h2 = send(
    &mut engine,
    "r-4",
    "consumer-session/add-interaction",
    json!({ "session": session, "interaction": second }),
  );
  assert_eq!(h1["ok"]["handle"], "i-1");
  assert_eq!(h2["ok"]["handle"], "i-2");

  let finalised = send(
    &mut engine,
    "r-5",
    "consumer-session/finalise",
    json!({ "session": session }),
  );
  assert_eq!(
    finalised["ok"]["results"],
    json!([
      { "handle": "i-1", "status": "not-exercised" },
      { "handle": "i-2", "status": "not-exercised" }
    ])
  );
}

#[test]
fn handshake_required_before_hello() {
  let mut engine = Engine::new();
  let response = send(
    &mut engine,
    "r-1",
    "consumer-session/create",
    json!({ "config": { "consumer": { "name": "web-app" }, "provider": { "name": "order-api" } } }),
  );
  assert_eq!(response["error"]["code"], "handshake-required");
  assert_eq!(response["error"]["category"], "protocol");
}

#[test]
fn protocol_version_unsupported_leaves_pipe_usable() {
  let mut engine = Engine::new();
  let rejected = send(
    &mut engine,
    "r-1",
    "engine/hello",
    json!({ "protocol-versions": [99], "host": { "name": "janus-ts", "version": "9.0.0" } }),
  );
  assert_eq!(rejected["error"]["code"], "protocol-version-unsupported");
  assert_eq!(rejected["error"]["category"], "protocol");
  assert_eq!(rejected["error"]["details"]["supported"], json!([1]));

  // The pipe stays usable: a subsequent, correct hello still succeeds.
  let ok = hello(&mut engine);
  assert_eq!(ok["ok"]["protocol-version"], 1);
}

#[test]
fn unknown_operation_is_named() {
  let mut engine = Engine::new();
  hello(&mut engine);
  let response = send(
    &mut engine,
    "r-2",
    "verification/replay",
    json!({ "session": "vs-1" }),
  );
  assert_eq!(response["error"]["code"], "operation-unsupported");
  assert_eq!(response["error"]["details"]["op"], "verification/replay");
}

#[test]
fn malformed_frame_gets_empty_id() {
  let mut engine = Engine::new();
  let bytes = engine.dispatch(b"{ this is not valid json");
  let response: Value = serde_json::from_slice(&bytes).unwrap();
  assert_eq!(response["error"]["code"], "malformed-frame");
  assert_eq!(response["id"], "");
}

#[test]
fn invalid_interaction_document_reports_pointer_problems() {
  let mut engine = Engine::new();
  hello(&mut engine);
  let create = send(
    &mut engine,
    "r-2",
    "consumer-session/create",
    json!({ "config": { "consumer": { "name": "web-app" }, "provider": { "name": "order-api" } } }),
  );
  let session = create["ok"]["session"].as_str().unwrap().to_string();

  let response = send(
    &mut engine,
    "r-3",
    "consumer-session/add-interaction",
    json!({
      "session": session,
      "interaction": {
        "description": "a request for an order",
        "parts": { "response": { "body": { "tpye": "integer" } } }
      }
    }),
  );
  assert_eq!(response["error"]["code"], "interaction-invalid");
  assert_eq!(response["error"]["category"], "document");
  let problems = response["error"]["details"]["problems"].as_array().unwrap();
  assert!(!problems.is_empty());
  assert!(problems[0]["pointer"].as_str().unwrap().starts_with('/'));
}

#[test]
fn session_not_found_on_unknown_session() {
  let mut engine = Engine::new();
  hello(&mut engine);

  let added = send(
    &mut engine,
    "r-2",
    "consumer-session/add-interaction",
    json!({ "session": "cs-does-not-exist", "interaction": order_interaction() }),
  );
  assert_eq!(added["error"]["code"], "session-not-found");
  assert_eq!(added["error"]["category"], "session");

  let finalised = send(
    &mut engine,
    "r-3",
    "consumer-session/finalise",
    json!({ "session": "cs-does-not-exist" }),
  );
  assert_eq!(finalised["error"]["code"], "session-not-found");
}

#[test]
fn sessions_are_the_only_resource_finalise_ends_it() {
  let mut engine = Engine::new();
  hello(&mut engine);
  let create = send(
    &mut engine,
    "r-2",
    "consumer-session/create",
    json!({ "config": { "consumer": { "name": "web-app" }, "provider": { "name": "order-api" } } }),
  );
  let session = create["ok"]["session"].as_str().unwrap().to_string();

  let first = send(
    &mut engine,
    "r-3",
    "consumer-session/finalise",
    json!({ "session": session }),
  );
  assert!(first["ok"].is_object());

  // No cleanup call exists or is needed — finalise already released everything. A second
  // finalise on the same id is simply session-not-found, never undefined behaviour.
  let second = send(
    &mut engine,
    "r-4",
    "consumer-session/finalise",
    json!({ "session": session }),
  );
  assert_eq!(second["error"]["code"], "session-not-found");
}

fn create_session(engine: &mut Engine) -> String {
  hello(engine);
  let create = send(
    engine,
    "r-2",
    "consumer-session/create",
    json!({ "config": { "consumer": { "name": "web-app" }, "provider": { "name": "order-api" } } }),
  );
  create["ok"]["session"].as_str().unwrap().to_string()
}

// A widened order interaction — one `optional` member — so its variant space has more than the
// single degenerate `base` variant `order_interaction()` alone would give.
fn order_interaction_with_optional_field() -> Value {
  let mut interaction = order_interaction();
  interaction["parts"]["response"]["body"] = json!({
    "shape": "object",
    "members": { "shippedAt": { "shape": "optional", "of": { "shape": "string", "example": "2026-07-30" } } }
  });
  interaction
}

#[test]
fn variants_returns_the_selection_and_report_for_a_degenerate_space() {
  let mut engine = Engine::new();
  let session = create_session(&mut engine);
  let added = send(
    &mut engine,
    "r-3",
    "consumer-session/add-interaction",
    json!({ "session": session, "interaction": order_interaction() }),
  );
  let handle = added["ok"]["handle"].as_str().unwrap().to_string();

  let variants = send(
    &mut engine,
    "r-4",
    "consumer-session/variants",
    json!({ "session": session, "handle": handle }),
  );
  assert_eq!(
    variants["ok"]["variants"],
    json!([ { "id": "base", "label": "base", "origin": "base", "assignment": [] } ])
  );
  assert_eq!(variants["ok"]["report"]["space"]["size"], 1);
  assert_eq!(variants["ok"]["report"]["strategy"], "exhaustive");
  assert_eq!(variants["ok"]["report"]["selected"], 1);

  let finalised = send(
    &mut engine,
    "r-5",
    "consumer-session/finalise",
    json!({ "session": session }),
  );
  assert_eq!(
    finalised["ok"]["results"],
    json!([ { "handle": "i-1", "status": "not-exercised",
              "variants": [ { "variant": "base", "status": "not-exercised" } ] } ])
  );
}

#[test]
fn variants_selects_more_than_one_variant_when_the_shape_has_width() {
  let mut engine = Engine::new();
  let session = create_session(&mut engine);
  let added = send(
    &mut engine,
    "r-3",
    "consumer-session/add-interaction",
    json!({ "session": session, "interaction": order_interaction_with_optional_field() }),
  );
  let handle = added["ok"]["handle"].as_str().unwrap().to_string();

  let variants = send(
    &mut engine,
    "r-4",
    "consumer-session/variants",
    json!({ "session": session, "handle": handle }),
  );
  let ids: Vec<&str> = variants["ok"]["variants"]
    .as_array()
    .unwrap()
    .iter()
    .map(|v| v["id"].as_str().unwrap())
    .collect();
  assert_eq!(ids, vec!["base", "response.body.shippedAt#presence=absent"]);
  assert_eq!(variants["ok"]["report"]["space"]["size"], 2);
}

#[test]
fn serve_variant_arms_a_selected_variant() {
  let mut engine = Engine::new();
  let session = create_session(&mut engine);
  let added = send(
    &mut engine,
    "r-3",
    "consumer-session/add-interaction",
    json!({ "session": session, "interaction": order_interaction() }),
  );
  let handle = added["ok"]["handle"].as_str().unwrap().to_string();
  send(
    &mut engine,
    "r-4",
    "consumer-session/variants",
    json!({ "session": session, "handle": handle }),
  );

  let served = send(
    &mut engine,
    "r-5",
    "consumer-session/serve-variant",
    json!({ "session": session, "handle": handle, "variant": "base" }),
  );
  assert_eq!(served["ok"], json!({}));
}

#[test]
fn serve_variant_with_an_id_not_in_the_selection_is_variant_not_found() {
  let mut engine = Engine::new();
  let session = create_session(&mut engine);
  let added = send(
    &mut engine,
    "r-3",
    "consumer-session/add-interaction",
    json!({ "session": session, "interaction": order_interaction() }),
  );
  let handle = added["ok"]["handle"].as_str().unwrap().to_string();
  send(
    &mut engine,
    "r-4",
    "consumer-session/variants",
    json!({ "session": session, "handle": handle }),
  );

  let served = send(
    &mut engine,
    "r-5",
    "consumer-session/serve-variant",
    json!({ "session": session, "handle": handle, "variant": "does-not-exist" }),
  );
  assert_eq!(served["error"]["code"], "variant-not-found");
  assert_eq!(served["error"]["category"], "session");
  assert_eq!(served["error"]["details"]["selection"], json!(["base"]));
}

#[test]
fn variants_on_an_unknown_handle_is_handle_not_found() {
  let mut engine = Engine::new();
  let session = create_session(&mut engine);

  let variants = send(
    &mut engine,
    "r-3",
    "consumer-session/variants",
    json!({ "session": session, "handle": "i-does-not-exist" }),
  );
  assert_eq!(variants["error"]["code"], "handle-not-found");
  assert_eq!(variants["error"]["category"], "session");
}

#[test]
fn a_per_call_policy_override_can_force_base_only() {
  let mut engine = Engine::new();
  let session = create_session(&mut engine);
  let added = send(
    &mut engine,
    "r-3",
    "consumer-session/add-interaction",
    json!({ "session": session, "interaction": order_interaction_with_optional_field() }),
  );
  let handle = added["ok"]["handle"].as_str().unwrap().to_string();

  let variants = send(
    &mut engine,
    "r-4",
    "consumer-session/variants",
    json!({ "session": session, "handle": handle, "policy": { "strategy": "base-only" } }),
  );
  assert_eq!(
    variants["ok"]["variants"],
    json!([ { "id": "base", "label": "base", "origin": "base",
              "assignment": [ { "dimension": "response.body.shippedAt#presence", "point": "present" } ] } ])
  );
  assert_eq!(variants["ok"]["report"]["strategy"], "base-only");
}

// --- provider-shape sessions (plan task 7.2, spec §8.5) ----------------------------------------

fn record(engine: &mut Engine, session: &str, description: &str, states: Value, body: Value) -> Value {
  send(
    engine,
    "r-observe",
    "provider-shape-session/observe",
    json!({ "session": session, "description": description, "states": states,
            "parts": { "response": { "body": { "content": body } } } }),
  )
}

fn recording_session(engine: &mut Engine) -> String {
  let created = send(
    engine,
    "r-create",
    "provider-shape-session/create",
    json!({ "provider": { "name": "order-service" } }),
  );
  created["ok"]["session"]
    .as_str()
    .expect("a session id")
    .to_string()
}

#[test]
fn recording_is_declared_as_a_capability_rather_than_assumed() {
  // Spec §5.3: "a host MUST NOT rely on operations behind a capability the engine did not
  // declare", which only works if an engine that has them says so.
  let mut engine = Engine::new();
  let response = hello(&mut engine);
  assert_eq!(
    response["ok"]["capabilities"]["provider-shape-recording"],
    json!({}),
    "presence is the whole signal"
  );
}

#[test]
fn a_recording_session_turns_observed_responses_into_a_provider_shape() {
  let mut engine = Engine::new();
  hello(&mut engine);
  let session = recording_session(&mut engine);

  let states = json!(["an order exists"]);
  for status in ["PENDING", "SHIPPED", "PENDING", "CANCELLED"] {
    let progress = record(
      &mut engine,
      &session,
      "a request for an order",
      states.clone(),
      json!({ "id": "66", "status": status }),
    );
    assert_eq!(progress["ok"]["interactions"], json!(1));
  }

  let finalised = send(
    &mut engine,
    "r-finalise",
    "provider-shape-session/finalise",
    json!({ "session": session }),
  );
  let recorded = &finalised["ok"]["provider-shape"];
  assert_eq!(recorded["$format"], json!("janus-provider-shape/1"));
  assert_eq!(recorded["provenance"], json!("recorded"));
  assert_eq!(recorded["provider"]["name"], json!("order-service"));

  let interaction = &recorded["interactions"][0];
  assert_eq!(interaction["description"], json!("a request for an order"));
  assert_eq!(interaction["states"], json!([{ "name": "an order exists" }]));
  assert_eq!(interaction["source"]["observations"], json!(4));
  assert_eq!(
    interaction["parts"]["response"]["body"]["members"]["status"]["options"],
    json!(["CANCELLED", "PENDING", "SHIPPED"])
  );
}

#[test]
fn a_host_may_set_the_recorders_judgements_per_session() {
  let mut engine = Engine::new();
  hello(&mut engine);
  // `min-evidence: 1` makes one non-repeating observation enough to call a position open, which
  // is a host saying "my suite hits each endpoint once, do not read enums into that".
  let created = send(
    &mut engine,
    "r-create",
    "provider-shape-session/create",
    json!({ "provider": { "name": "order-service" }, "policy": { "min-evidence": 1 } }),
  );
  let session = created["ok"]["session"]
    .as_str()
    .expect("a session id")
    .to_string();
  record(
    &mut engine,
    &session,
    "get an order",
    json!([]),
    json!({ "id": "66" }),
  );

  let finalised = send(
    &mut engine,
    "r-finalise",
    "provider-shape-session/finalise",
    json!({ "session": session }),
  );
  assert_eq!(
    finalised["ok"]["provider-shape"]["interactions"][0]["parts"]["response"]["body"]["members"]["id"]["shape"],
    json!("string"),
    "one value, seen once, never repeated: a sample of an open domain under this policy"
  );
}

#[test]
fn a_recording_session_ends_at_finalise_and_says_so_afterwards() {
  // Spec §7.1: a session ends in exactly one way per kind, and a stale id gets a named error.
  let mut engine = Engine::new();
  hello(&mut engine);
  let session = recording_session(&mut engine);
  record(
    &mut engine,
    &session,
    "get an order",
    json!([]),
    json!({ "id": "66" }),
  );
  send(
    &mut engine,
    "r-finalise",
    "provider-shape-session/finalise",
    json!({ "session": session }),
  );

  let stale = record(
    &mut engine,
    &session,
    "get an order",
    json!([]),
    json!({ "id": "67" }),
  );
  assert_eq!(stale["error"]["code"], json!("session-not-found"));
  let refinalise = send(
    &mut engine,
    "r-finalise",
    "provider-shape-session/finalise",
    json!({ "session": session }),
  );
  assert_eq!(refinalise["error"]["code"], json!("session-not-found"));
}

#[test]
fn recording_a_response_never_needs_the_engine_to_know_what_a_response_is() {
  // The value arrives decoded and wrapped as a contract wraps one (contract spec §5.3). A slot
  // that carried nothing is left out rather than given a special value, and a part the provider
  // did not produce simply is not there.
  let mut engine = Engine::new();
  hello(&mut engine);
  let session = recording_session(&mut engine);
  let observed = send(
    &mut engine,
    "r-observe",
    "provider-shape-session/observe",
    json!({ "session": session, "description": "get an order",
            "parts": { "response": { "status": { "content": 200 },
                                     "body": { "content": { "id": "66" } } } } }),
  );
  assert_eq!(observed["ok"]["observations"], json!(1));

  let finalised = send(
    &mut engine,
    "r-finalise",
    "provider-shape-session/finalise",
    json!({ "session": session }),
  );
  let slots = &finalised["ok"]["provider-shape"]["interactions"][0]["parts"]["response"];
  assert_eq!(slots["status"], json!({ "shape": "equality", "example": 200 }));
  assert!(slots["body"].is_object());
}

#[test]
fn a_malformed_observation_is_a_value_not_a_panic() {
  let mut engine = Engine::new();
  hello(&mut engine);
  let session = recording_session(&mut engine);
  let response = send(
    &mut engine,
    "r-observe",
    "provider-shape-session/observe",
    json!({ "session": session, "description": "get an order", "parts": "not an object" }),
  );
  // The same code every operation answers a body it cannot read with (spec §4.4) — a recording
  // session adds no error vocabulary of its own.
  assert_eq!(response["error"]["code"], json!("malformed-frame"));
}
