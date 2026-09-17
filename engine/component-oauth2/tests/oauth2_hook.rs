//! The oauth2 hook component (plan task 5.3) against the sample provider's own token endpoint: the
//! component fetches a credential once and presents it on every request, which is the shape the
//! `before-verification` + `before-request` pair exists for.
//!
//! These are the component's own tests — invoked directly through the hook interface, not through a
//! verification run (that is `engine/kernel/tests/hooks.rs`'s job). What they pin down is the part
//! only this component decides: what it does per point, what it puts in a header, what it says when
//! the credentials are wrong, and what it refuses to say about the token it holds.

use pact_janus_component_oauth2::Oauth2Hook;
use pact_janus_kernel::component::{HookComponent, Invoke};
use pact_janus_sample_order_service::{
  Config, DEFAULT_CLIENT_ID, DEFAULT_CLIENT_SECRET, DEFAULT_TOKEN, start,
};
use serde_json::{Value, json};

fn invoke(hook: &Oauth2Hook, point: &str, context: Value, config: Value) -> Value {
  hook
    .invoke(Invoke {
      point: point.to_string(),
      context,
      config: Some(config),
      deadline_ms: 5_000,
    })
    .expect("the component answers rather than erroring")
}

fn credentials(base_url: &str) -> Value {
  json!({
    "token-url": format!("{base_url}/oauth/token"),
    "client-id": DEFAULT_CLIENT_ID,
    "client-secret": DEFAULT_CLIENT_SECRET,
  })
}

fn request_context(headers: Value) -> Value {
  json!({
    "point": "before-request",
    "role": "provider",
    "parts": { "request": {
      "method": { "content": "GET" },
      "path": { "content": "/orders/66" },
      "headers": { "content": headers }
    } },
    "mutable": ["parts.request.headers"],
  })
}

#[test]
fn a_token_is_fetched_once_at_before_verification_and_presented_at_every_request() {
  let provider = start(Config::default()).expect("the sample provider binds");
  let hook = Oauth2Hook::new();

  let acquired = invoke(
    &hook,
    "before-verification",
    json!({ "point": "before-verification", "role": "provider" }),
    credentials(provider.base_url()),
  );
  assert_eq!(acquired["outcome"], json!("ok"));
  assert_eq!(acquired["data"]["token-type"], json!("Bearer"));
  assert!(
    acquired["data"]["expires-in-ms"].as_u64().unwrap_or(0) > 0,
    "the lifetime is reported so an expiry is debuggable: {acquired}"
  );

  let applied = invoke(
    &hook,
    "before-request",
    request_context(json!({})),
    credentials(provider.base_url()),
  );
  assert_eq!(applied["outcome"], json!("ok"));
  assert_eq!(
    applied["changes"]["parts.request.headers"]["content"]["authorization"],
    json!([format!("Bearer {DEFAULT_TOKEN}")])
  );

  // One fetch, however many requests: the run-scope point is what keeps it that way.
  let store_hits = provider.store().lock().unwrap().state_log.len();
  assert_eq!(store_hits, 0, "no provider state was touched by any of this");
}

#[test]
fn the_credential_is_added_to_the_headers_that_were_already_there() {
  let provider = start(Config::default()).expect("binds");
  let hook = Oauth2Hook::new();
  let applied = invoke(
    &hook,
    "before-request",
    request_context(json!({ "content-type": ["application/json"] })),
    credentials(provider.base_url()),
  );
  let headers = &applied["changes"]["parts.request.headers"]["content"];
  assert_eq!(
    headers["content-type"],
    json!(["application/json"]),
    "merged, not replaced"
  );
  assert!(headers["authorization"].is_array());
}

#[test]
fn a_pre_issued_token_needs_no_endpoint() {
  let hook = Oauth2Hook::new();
  let applied = invoke(
    &hook,
    "before-request",
    request_context(json!({})),
    json!({ "token": "already-have-one", "scheme": "Token" }),
  );
  assert_eq!(
    applied["changes"]["parts.request.headers"]["content"]["authorization"],
    json!(["Token already-have-one"]),
    "a project whose CI already has a credential should not have to stand up an endpoint"
  );
}

#[test]
fn wrong_credentials_fail_the_hook_with_what_the_endpoint_said() {
  let provider = start(Config::default()).expect("binds");
  let hook = Oauth2Hook::new();
  let failed = invoke(
    &hook,
    "before-verification",
    json!({ "point": "before-verification", "role": "provider" }),
    json!({
      "token-url": format!("{}/oauth/token", provider.base_url()),
      "client-id": "someone-else",
      "client-secret": "nope",
    }),
  );
  assert_eq!(failed["outcome"], json!("failed"));
  assert_eq!(failed["error"]["code"], json!("token-request-failed"));
  assert!(
    failed["error"]["message"]
      .as_str()
      .is_some_and(|m| m.contains("invalid_client")),
    "the endpoint's own reason, which is not a secret: {failed}"
  );
}

#[test]
fn an_unreachable_token_endpoint_is_named_rather_than_retried_forever() {
  let hook = Oauth2Hook::new();
  let failed = invoke(
    &hook,
    "before-verification",
    json!({ "point": "before-verification", "role": "provider" }),
    json!({ "token-url": "http://127.0.0.1:1/oauth/token", "client-id": "x", "client-secret": "y" }),
  );
  assert_eq!(failed["outcome"], json!("failed"));
  assert_eq!(failed["error"]["code"], json!("token-endpoint-unreachable"));
}

#[test]
fn a_point_this_component_has_nothing_to_do_at_is_skipped_not_guessed_at() {
  let hook = Oauth2Hook::new();
  for point in ["state-setup", "after-response", "a-point-invented-next-year"] {
    let answered = invoke(
      &hook,
      point,
      json!({ "point": point, "role": "provider" }),
      json!({ "token": "t" }),
    );
    assert_eq!(
      answered["outcome"],
      json!("skipped"),
      "the open-discriminator rule at the hook boundary: {point}"
    );
  }
}

#[test]
fn nothing_the_component_says_contains_the_token() {
  let hook = Oauth2Hook::new();
  let acquired = invoke(
    &hook,
    "before-verification",
    json!({ "point": "before-verification", "role": "provider" }),
    json!({ "token": "s3cr3t-value" }),
  );
  let said = serde_json::to_string(&acquired).unwrap();
  assert!(
    !said.contains("s3cr3t-value"),
    "`data` reaches later hooks and, with report-data, the summary: {said}"
  );

  // The one place it does appear is the change itself, which is the header going on the wire — and
  // the report records that by path, never by value.
  let applied = invoke(
    &hook,
    "before-request",
    request_context(json!({})),
    json!({ "token": "s3cr3t-value" }),
  );
  assert!(
    !serde_json::to_string(&applied["data"])
      .unwrap()
      .contains("s3cr3t-value"),
    "even beside the change that carries it, the data stays clean"
  );
}
