//! The sample provider's own tests (plan task 5.6). They are not contract tests — the contract
//! tests are the verification run's (`engine/kernel/tests/verification.rs`). What these pin down is
//! the behaviour *those* runs depend on: that variance really is driven by provider state, that
//! auth really is enforced, and that the state endpoint really does distinguish "I cannot reach
//! that state" from "my handler is broken" (variant-semantics spec §6.7).

use pact_janus_sample_order_service::{Config, DEFAULT_TOKEN, start};
use serde_json::{Value, json};

fn provider() -> pact_janus_sample_order_service::Provider {
  start(Config::default()).expect("an OS-assigned port always binds")
}

fn get(base_url: &str, path: &str, token: Option<&str>) -> (u16, Value) {
  let agent: ureq::Agent = ureq::Agent::config_builder()
    .http_status_as_error(false)
    .build()
    .into();
  let mut request = agent.get(format!("{base_url}{path}"));
  if let Some(token) = token {
    request = request.header("authorization", &format!("Bearer {token}"));
  }
  let mut response = request.call().expect("the provider answers");
  let status = response.status().as_u16();
  let body = response.body_mut().read_to_string().expect("a body");
  (status, serde_json::from_str(&body).unwrap_or(Value::Null))
}

fn set_state(base_url: &str, state: Value) -> (u16, Value) {
  let agent: ureq::Agent = ureq::Agent::config_builder()
    .http_status_as_error(false)
    .build()
    .into();
  let mut response = agent
    .post(format!("{base_url}/_pact/provider-states"))
    .header("content-type", "application/json")
    .send(serde_json::to_string(&state).unwrap())
    .expect("the provider answers");
  let status = response.status().as_u16();
  let body = response.body_mut().read_to_string().expect("a body");
  (status, serde_json::from_str(&body).unwrap_or(Value::Null))
}

#[test]
fn health_needs_no_credential() {
  let provider = provider();
  let (status, body) = get(provider.base_url(), "/health", None);
  assert_eq!(status, 200);
  assert_eq!(body, json!({ "status": "up" }));
}

#[test]
fn orders_need_a_bearer_token() {
  let provider = provider();
  let (status, body) = get(provider.base_url(), "/orders/66", None);
  assert_eq!(status, 401);
  assert_eq!(body["error"], json!("unauthorized"));

  let (status, _) = get(provider.base_url(), "/orders/66", Some("not-the-token"));
  assert_eq!(status, 401, "a wrong token is as unauthorized as none");

  let (status, _) = get(provider.base_url(), "/orders/66", Some(DEFAULT_TOKEN));
  assert_eq!(status, 200);
}

#[test]
fn auth_can_be_turned_off_for_a_demo() {
  let provider = start(Config {
    token: None,
    ..Config::default()
  })
  .expect("binds");
  let (status, _) = get(provider.base_url(), "/orders/66", None);
  assert_eq!(status, 200);
}

#[test]
fn a_freshly_started_provider_has_one_pending_order() {
  let provider = provider();
  let (status, order) = get(provider.base_url(), "/orders/66", Some(DEFAULT_TOKEN));
  assert_eq!(status, 200);
  assert_eq!(order["id"], json!("66"));
  assert_eq!(order["status"], json!("PENDING"));
  assert_eq!(
    order.get("shippedAt"),
    None,
    "absent, never null — absence is the variance an `optional` shape describes"
  );
}

#[test]
fn state_setup_drives_the_variance_a_verification_run_replays() {
  let provider = provider();

  let (status, _) = set_state(
    provider.base_url(),
    json!({ "state": "an order exists", "action": "setup",
            "params": { "id": "66", "shipped": true, "items": 2 } }),
  );
  assert_eq!(status, 200);
  let (_, order) = get(provider.base_url(), "/orders/66", Some(DEFAULT_TOKEN));
  assert_eq!(order["status"], json!("SHIPPED"));
  assert!(order["shippedAt"].is_string(), "shipped orders carry a date");
  assert_eq!(order["items"].as_array().map(Vec::len), Some(2));

  // The same interaction, the other variant: setup is what makes the difference, which is the
  // whole mechanism variant-bound provider state exists for.
  let (status, _) = set_state(
    provider.base_url(),
    json!({ "state": "an order exists", "action": "setup",
            "params": { "id": "66", "shipped": false } }),
  );
  assert_eq!(status, 200);
  let (_, order) = get(provider.base_url(), "/orders/66", Some(DEFAULT_TOKEN));
  assert_eq!(order["status"], json!("PENDING"));
  assert_eq!(order.get("shippedAt"), None);
}

#[test]
fn a_bound_parameter_may_arrive_as_its_point_name() {
  let provider = provider();
  let (status, _) = set_state(
    provider.base_url(),
    json!({ "state": "an order exists", "action": "setup",
            "params": { "id": "66", "shipped": "present" } }),
  );
  assert_eq!(status, 200);
  let (_, order) = get(provider.base_url(), "/orders/66", Some(DEFAULT_TOKEN));
  assert!(order["shippedAt"].is_string());
}

#[test]
fn a_state_the_provider_cannot_reach_is_unsupported_not_an_error() {
  let provider = provider();
  let (status, body) = set_state(
    provider.base_url(),
    json!({ "state": "an order exists", "action": "setup",
            "params": { "id": "66", "status": "SHIPPED", "shipped": false } }),
  );
  assert_eq!(
    status, 200,
    "'cannot reach that state' is an answer, not an error — a non-2xx would read as a broken \
     handler (lifecycle-hooks spec §8.4)"
  );
  assert_eq!(body["outcome"], json!("unsupported"));
  assert!(
    body["error"]["message"].is_string(),
    "with a reason (variant-semantics spec §6.7): {body}"
  );
}

#[test]
fn an_unknown_state_is_a_broken_handler_not_an_unreachable_one() {
  let provider = provider();
  let (status, body) = set_state(
    provider.base_url(),
    json!({ "state": "the moon is in the seventh house", "action": "setup" }),
  );
  assert_eq!(status, 500);
  assert_eq!(body["error"], json!("unknown-state"));
}

#[test]
fn every_state_request_is_logged_so_a_demo_can_show_setup_ran_per_variant() {
  let provider = provider();
  for shipped in [true, false, true] {
    set_state(
      provider.base_url(),
      json!({ "state": "an order exists", "action": "setup",
              "params": { "id": "66", "shipped": shipped } }),
    );
  }
  let store = provider.store().lock().unwrap();
  assert_eq!(
    store.state_log.len(),
    3,
    "three setups, not one — a verifier MUST NOT collapse equal consecutive states"
  );
}

#[test]
fn the_provider_produces_more_than_any_consumer_declares() {
  let provider = provider();
  let (_, order) = get(provider.base_url(), "/orders/66", Some(DEFAULT_TOKEN));
  assert!(
    order["channel"].is_string(),
    "undeclared variance on purpose: this is what Phase 7's subsumption check is meant to find"
  );
}

#[test]
fn teardown_is_accepted_and_changes_nothing() {
  let provider = provider();
  set_state(
    provider.base_url(),
    json!({ "state": "an order exists", "action": "setup", "params": { "id": "66", "shipped": true } }),
  );
  let (status, _) = set_state(
    provider.base_url(),
    json!({ "state": "an order exists", "action": "teardown", "params": { "id": "66" } }),
  );
  assert_eq!(status, 200);
  let (status, order) = get(provider.base_url(), "/orders/66", Some(DEFAULT_TOKEN));
  assert_eq!(status, 200);
  assert_eq!(
    order["status"],
    json!("SHIPPED"),
    "teardown is a no-op here, by design"
  );
}

#[test]
fn no_orders_exist_empties_the_store() {
  let provider = provider();
  let (status, _) = set_state(
    provider.base_url(),
    json!({ "state": "no orders exist", "action": "setup" }),
  );
  assert_eq!(status, 200);
  let (status, _) = get(provider.base_url(), "/orders/66", Some(DEFAULT_TOKEN));
  assert_eq!(status, 404);
}

#[test]
fn dropping_the_provider_stops_its_thread() {
  let base_url = {
    let provider = provider();
    provider.base_url().to_string()
  };
  let agent: ureq::Agent = ureq::Agent::config_builder()
    .timeout_global(Some(std::time::Duration::from_secs(2)))
    .build()
    .into();
  assert!(
    agent.get(format!("{base_url}/health")).call().is_err(),
    "nothing is listening once the provider is dropped"
  );
}
