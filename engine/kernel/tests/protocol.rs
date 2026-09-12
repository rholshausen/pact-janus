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
