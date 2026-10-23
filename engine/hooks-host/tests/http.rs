//! The `http` implementation against a real endpoint (staged plan S1.9, ADR 0044): a state call
//! tried three times while its endpoint warms up, and once anywhere else; a state endpoint behind
//! mutual TLS, reached through the entry's `client` options; and today's query form.

use pact_janus_hooks_host::http::HttpHooks;
use pact_janus_kernel::hooks::{HookInvoker, Outcome};
use pact_janus_test_servers::{Authority, Response, TlsProvider};
use serde_json::{Value, json};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// An endpoint, trusted through a CA made for the test, that answers `answers` in turn and then
/// the last of them for ever.
fn endpoint(authority: &Authority, answers: Vec<Response>, clients: Option<&Authority>) -> TlsProvider {
  let count = Arc::new(AtomicUsize::new(0));
  TlsProvider::start(&authority.server(&["127.0.0.1"]), clients, move |_| {
    let n = count.fetch_add(1, Ordering::SeqCst);
    answers[n.min(answers.len() - 1)].clone()
  })
}

fn run(provider: &TlsProvider, authority: &Authority, extra: Value) -> Value {
  let mut run = json!({
    "kind": "http",
    "url": format!("{}/_pact/provider-states", provider.base_url),
    "format": "pact-state-change",
    "client": { "tls": { "ca-certificates": [authority.certificate()] } },
  });
  for (name, value) in extra.as_object().unwrap() {
    run[name] = value.clone();
  }
  run
}

fn at(point: &str) -> Value {
  json!({ "point": point, "state": { "name": "an order exists", "params": { "id": 66 } } })
}

#[test]
fn a_state_endpoint_that_is_warming_up_is_tried_again() {
  let authority = Authority::new("hooks");
  let provider = endpoint(
    &authority,
    vec![Response::json(503, "{}"), Response::json(200, r#"{"id": 7}"#)],
    None,
  );
  let result = HttpHooks::new()
    .invoke(&run(&provider, &authority, json!({})), &at("state-setup"), 5000)
    .expect("a result");
  assert_eq!(result.outcome(), Outcome::Ok);
  assert_eq!(result.values.map(Value::Object), Some(json!({ "id": 7 })));
  assert_eq!(provider.requests().len(), 2);
}

#[test]
fn a_state_endpoint_that_keeps_failing_fails_after_three_attempts_in_about_a_second() {
  let authority = Authority::new("hooks");
  let provider = endpoint(&authority, vec![Response::json(500, "{}")], None);
  let started = Instant::now();
  let result = HttpHooks::new()
    .invoke(
      &run(&provider, &authority, json!({})),
      &at("state-teardown"),
      5000,
    )
    .expect("a result");
  assert_eq!(result.outcome(), Outcome::Failed);
  assert_eq!(provider.requests().len(), 3);
  assert!(
    started.elapsed() < Duration::from_secs(5),
    "{:?}",
    started.elapsed()
  );
}

#[test]
fn a_request_at_any_other_point_or_with_no_retries_is_tried_once() {
  let authority = Authority::new("hooks");
  let provider = endpoint(&authority, vec![Response::json(503, "{}")], None);
  let hooks = HttpHooks::new();
  let other = hooks
    .invoke(
      &run(&provider, &authority, json!({ "format": "janus" })),
      &at("before-request"),
      5000,
    )
    .expect("a result");
  assert_eq!(other.outcome(), Outcome::Failed);
  assert_eq!(provider.requests().len(), 1);

  let mut none = run(&provider, &authority, json!({}));
  none["client"]["retries"] = json!(0);
  let result = hooks.invoke(&none, &at("state-setup"), 5000).expect("a result");
  assert_eq!(result.outcome(), Outcome::Failed);
  assert_eq!(provider.requests().len(), 2, "one more request, not three");
}

#[test]
fn a_state_endpoint_behind_mutual_tls_is_reached_with_the_entrys_client_options() {
  let authority = Authority::new("hooks");
  let provider = endpoint(&authority, vec![Response::json(200, "{}")], Some(&authority));
  let identity = authority.client("verifier");
  let mut with = run(&provider, &authority, json!({}));
  with["client"]["tls"]["client-certificate"] =
    json!({ "certificate": identity.certificate, "key": identity.key });
  let hooks = HttpHooks::new();
  let result = hooks.invoke(&with, &at("state-setup"), 5000).expect("a result");
  assert_eq!(result.outcome(), Outcome::Ok);
  assert!(
    provider
      .handshakes()
      .iter()
      .any(|handshake| handshake.client_certificate)
  );

  let mut without = run(&provider, &authority, json!({}));
  without["client"]["retries"] = json!(0);
  let refused = hooks
    .invoke(&without, &at("state-setup"), 5000)
    .expect_err("no certificate, no connection");
  let message = refused.error["message"].as_str().unwrap_or_default().to_string();
  assert!(!message.contains(&identity.key), "{message}");
}

#[test]
fn the_query_form_sends_the_state_as_query_parameters_and_no_body() {
  let authority = Authority::new("hooks");
  let provider = endpoint(&authority, vec![Response::json(200, "")], None);
  let result = HttpHooks::new()
    .invoke(
      &run(
        &provider,
        &authority,
        json!({ "format": "pact-state-change-query" }),
      ),
      &at("state-setup"),
      5000,
    )
    .expect("a result");
  assert_eq!(result.outcome(), Outcome::Ok);
  let requests = provider.requests();
  assert_eq!(requests[0].method, "POST");
  assert_eq!(
    requests[0].target,
    "/_pact/provider-states?state=an+order+exists&action=setup&id=66"
  );
  assert!(requests[0].body.is_empty());
}

#[test]
fn a_url_with_user_information_is_never_repeated_with_it() {
  let authority = Authority::new("hooks");
  let provider = endpoint(&authority, vec![Response::json(500, "{}")], None);
  let mut entry = run(&provider, &authority, json!({}));
  entry["url"] = json!(provider.base_url.replace("https://", "https://user:hunter2@"));
  entry["client"]["retries"] = json!(0);
  let result = HttpHooks::new()
    .invoke(&entry, &at("state-setup"), 5000)
    .expect("a result");
  let error = result.error.expect("a failure says why").to_string();
  assert!(!error.contains("hunter2"), "{error}");
}
