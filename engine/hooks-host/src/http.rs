//! The `http` implementation (lifecycle-hooks spec §8.4): POST the context to an endpoint, read
//! the result from the response.
//!
//! The member that matters is `format`. `janus` sends this specification's context document;
//! **`pact-state-change` sends the v3/v4 provider-state body** — `{ state, params, action }` — so
//! an existing state-change endpoint keeps working unchanged. That is B5's actual migration test:
//! a provider that has a state endpoint today should be verifiable by naming it in a config file,
//! not by rewriting it. The response is read the same way either way, which means a v3 endpoint
//! that returns nothing is `ok`, and one that returns `{"outcome": "unsupported", …}` reaches the
//! state-unavailable path without knowing what Janus is.
//!
//! This is also how an SDK can offer callback-shaped ergonomics without the protocol growing a call
//! it does not have (ADR 0014): the test process serves a loopback endpoint and the configuration
//! names it. From here that is an ordinary `http` hook, which is the point.

//! **Reached as a provider is** (staged plan S1.9, ADR 0044): through `engine/http-client`, the one
//! client the HTTP transport and the broker client send through, configured by the entry's `client`
//! member, the HTTP client options (ADR 0035). A state endpoint behind the provider's mutual TLS or
//! private CA is reached the way the provider is.
//!
//! **A state call is tried three times** (ADR 0044): an answer of 5xx, 408 or 429 is tried again
//! after 100 ms, then after 1 s, as today's verifier was designed to (pact-reference #197), so an
//! endpoint that is still warming up is not a failed interaction. `client.retries` changes the
//! count; at any other point a request is tried once. `timeout-ms` bounds each attempt.

use pact_janus_http_client::{ClientOptions, Defaults, HttpClient, SendError};
use pact_janus_kernel::broker::without_userinfo;
use pact_janus_kernel::hooks::{HookFailure, HookInvoker, InvokeResult, Outcome};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// How much of a non-2xx response body is quoted back in the failure (spec §8.4: "a bounded
/// excerpt").
const BODY_EXCERPT: usize = 1024;

/// How many times a state call is tried again by default (ADR 0044): today's verifier's design,
/// three attempts in all.
const STATE_RETRIES: u32 = 2;

/// How long a request may take when nothing narrower bounds it. The hook's own deadline always
/// does, per attempt; this only keeps a client's default from being the narrower one.
const NO_CLIENT_TIMEOUT: Duration = Duration::from_secs(3600);

/// The `http` implementation. Each distinct `client` document gets one client, built once and
/// kept for the run, since a client holds its connections and its TLS configuration.
#[derive(Default)]
pub struct HttpHooks {
  clients: Mutex<HashMap<String, Arc<HttpClient>>>,
}

impl std::fmt::Debug for HttpHooks {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("HttpHooks").finish_non_exhaustive()
  }
}

impl HttpHooks {
  pub fn new() -> Self {
    HttpHooks::default()
  }

  /// The client for an entry's `client` document. Its errors name the member, never a value.
  fn client(&self, options: Option<&Value>) -> Result<Arc<HttpClient>, HookFailure> {
    let key = options.map(Value::to_string).unwrap_or_default();
    let mut clients = self
      .clients
      .lock()
      .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(client) = clients.get(&key) {
      return Ok(Arc::clone(client));
    }
    let defaults = Defaults {
      request_timeout: NO_CLIENT_TIMEOUT,
      retries: 0,
      user_agent: format!("pact-janus/{}", pact_janus_kernel::ENGINE_VERSION),
    };
    let client = HttpClient::from_value(options, defaults).map_err(|err| {
      let at = err.pointer.as_deref().unwrap_or("");
      HookFailure::errored(format!("run.client{at}: {}", err.message))
    })?;
    let client = Arc::new(client);
    clients.insert(key, Arc::clone(&client));
    Ok(client)
  }
}

/// What a hook is sent as: today's `{ state, params, action }` body, or the same as query
/// parameters (today's `--state-change-as-query`), or today's request for a message (`pact-message`),
/// or this specification's context document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Format {
  Context,
  StateChange,
  StateChangeQuery,
  Message,
}

impl HookInvoker for HttpHooks {
  fn invoke(&self, run: &Value, context: &Value, deadline_ms: u64) -> Result<InvokeResult, HookFailure> {
    let Some(url) = run.get("url").and_then(Value::as_str) else {
      return Err(HookFailure::errored("an http hook must name a url"));
    };
    let is_message = run.get("format").and_then(Value::as_str) == Some("pact-message");
    // Today's routing of a message's request (`pact-message`): to the transport the pact names, or
    // the message one, else `url`.
    let routed = if is_message {
      message_url(run, context)
    } else {
      None
    };
    let url = routed.as_deref().unwrap_or(url);
    // What a message may say about the endpoint: never its user information.
    let shown = without_userinfo(url);
    let method = run
      .get("method")
      .and_then(Value::as_str)
      .unwrap_or("POST")
      .to_ascii_uppercase();
    // A configuration written before a rename names the context format by its earlier name, and
    // still gets it (staged plan S0.5).
    let context_format = pact_janus_kernel::persisted::HOOK_BODY_FORMAT;
    let format = match run
      .get("format")
      .and_then(Value::as_str)
      .unwrap_or(context_format.writes())
    {
      "pact-state-change" => Format::StateChange,
      "pact-state-change-query" => Format::StateChangeQuery,
      "pact-message" => Format::Message,
      format if context_format.reads(format) => Format::Context,
      other => {
        return Err(HookFailure::errored(format!(
          "'{other}' is not a body format this engine knows ('{}', 'pact-state-change', \
           'pact-state-change-query', 'pact-message')",
          context_format.writes()
        )));
      }
    };

    let client_options = run.get("client");
    let retries = ClientOptions::from_value(client_options)
      .ok()
      .and_then(|options| options.retries)
      .unwrap_or_else(|| default_retries(context));
    let client = self.client(client_options)?;

    let target = match format {
      Format::StateChangeQuery => with_query(url, &state_change_query(context)),
      _ => Ok(url.to_string()),
    }
    .and_then(|target| pact_janus_http_client::uri(&target))
    .map_err(|err| HookFailure::errored(format!("the hook's url {err}")))?;
    let payload = match format {
      Format::Context => Some(serde_json::to_vec(context).expect("a HookContext always serializes")),
      Format::StateChange => {
        Some(serde_json::to_vec(&state_change_body(context)).expect("a json! value always serializes"))
      }
      Format::StateChangeQuery => None,
      Format::Message => {
        Some(serde_json::to_vec(&message_request(context)).expect("a json! value always serializes"))
      }
    };
    let request = || {
      let mut request = http::Request::builder()
        .method(method.as_str())
        .uri(target.clone());
      if payload.is_some() {
        request = request.header("content-type", "application/json");
      }
      if let Some(Value::Object(headers)) = run.get("headers") {
        for (name, value) in headers {
          if let Some(value) = value.as_str() {
            request = request.header(name.as_str(), value);
          }
        }
      }
      request
        .body(payload.clone().unwrap_or_default())
        .map_err(|err| HookFailure::errored(format!("could not build the request to {shown}: {err}")))
    };

    let deadline = Duration::from_millis(deadline_ms);
    let mut attempt = 0;
    let response = loop {
      attempt += 1;
      let response = client.run(request()?, Some(deadline)).map_err(|err| match err {
        // A timeout and a refused connection are different facts about the endpoint, and a reader
        // chasing one should not be sent to the other.
        SendError::Timeout(_) => HookFailure::timed_out(deadline_ms),
        other => HookFailure::errored(format!("{shown}: {other}")),
      })?;
      let status = response.status().as_u16();
      if attempt <= retries && retryable(status) {
        let wait = retry_delay(attempt);
        tracing::debug!(url = %shown, status, attempt, ?wait, "the hook endpoint is unavailable; trying again");
        std::thread::sleep(wait);
        continue;
      }
      break response;
    };

    let status = response.status().as_u16();
    let text = String::from_utf8_lossy(response.body()).to_string();

    if !(200..300).contains(&status) {
      return Ok(InvokeResult {
        outcome: Some(Outcome::Failed),
        error: Some(json!({
          "code": "hook-http-failed",
          "message": format!("{shown} answered {status}"),
          "details": { "status": status, "body": excerpt(&text) },
        })),
        ..InvokeResult::default()
      });
    }

    if format == Format::Message {
      // Read before the empty-body rule below: an empty reply is a message with no contents.
      return Ok(produced_message(&response));
    }

    if text.trim().is_empty() {
      // A v3 state endpoint that returns nothing is `ok` — which is exactly what makes today's
      // providers verifiable unchanged.
      return Ok(InvokeResult::ok());
    }

    if format != Format::Context {
      return state_change_result(&text);
    }

    match serde_json::from_str::<Value>(&text) {
      Ok(document) => InvokeResult::parse(&document).map_err(|message| HookFailure {
        outcome: Outcome::Errored,
        error: json!({
          "code": "hook-result-invalid",
          "message": message,
          "details": { "body": excerpt(&text) },
        }),
      }),
      // A 2xx with a body that is not JSON is a hook that answered something, so it is a result,
      // not the engine concluding on its behalf.
      Err(err) => Ok(InvokeResult {
        outcome: Some(Outcome::Failed),
        error: Some(json!({
          "code": "hook-result-invalid",
          "message": format!("{shown} answered 2xx with something that is not JSON: {err}"),
          "details": { "body": excerpt(&text) },
        })),
        ..InvokeResult::default()
      }),
    }
  }
}

/// Today's request for a message (`pact-message`, staged plan S1.7): what `pact_verifier` POSTs to
/// a message provider — its `description`, its `providerStates`, and a synchronous message's
/// `request` — which the engine hands a hook as `interaction.pact-message.request`. An interaction
/// with none is asked for by its description and states alone.
fn message_request(context: &Value) -> Value {
  let interaction = context.get("interaction");
  if let Some(request) = interaction.and_then(|i| i.pointer("/pact-message/request")) {
    return request.clone();
  }
  let mut request = json!({
    "description": interaction.and_then(|i| i.get("description")).cloned().unwrap_or(Value::Null),
  });
  if let Some(Value::Array(states)) = interaction.and_then(|i| i.get("states"))
    && !states.is_empty()
  {
    request["providerStates"] = Value::Array(states.clone());
  }
  request
}

/// Where today's verifier sends a message's request (staged plan S1.7): the entry of the hook's
/// `transports` (today's `--transports`) named by the transport the pact names for the interaction;
/// or, when it names none, the first named `message`, or `async-message` (`sync-message` for a
/// synchronous message). `None` sends it to the hook's `url`, which is also where a transport the
/// pact names but `transports` does not goes, as today.
fn message_url(run: &Value, context: &Value) -> Option<String> {
  let transports = run.get("transports")?.as_array()?;
  let pact_message = context.pointer("/interaction/pact-message");
  let named = pact_message
    .and_then(|m| m.get("transport"))
    .and_then(Value::as_str);
  let synchronous = pact_message
    .and_then(|m| m.get("synchronous"))
    .and_then(Value::as_bool)
    .unwrap_or(false);
  let kind = if synchronous {
    "sync-message"
  } else {
    "async-message"
  };
  transports
    .iter()
    .find(|entry| {
      let name = entry.get("name").and_then(Value::as_str);
      match named {
        Some(named) => name == Some(named),
        None => name == Some("message") || name == Some(kind),
      }
    })
    .and_then(|entry| entry.get("url"))
    .and_then(Value::as_str)
    .map(str::to_string)
}

/// A message provider's 2xx reply, read as today's verifier reads it (`messages.rs`): its body is
/// the message's contents, labelled with the reply's `Content-Type`; its metadata is `contentType`,
/// the reply's `Content-Type` or empty, with every member of the base64 JSON in a
/// `Pact-Message-Metadata` header over it. Produced as the `message` part, which the engine scores.
fn produced_message(response: &http::Response<Vec<u8>>) -> InvokeResult {
  let header = |name: &str| {
    response
      .headers()
      .get(name)
      .and_then(|value| value.to_str().ok())
      .map(str::to_string)
  };
  let content_type = header("content-type");
  let mut metadata = serde_json::Map::new();
  metadata.insert(
    "contentType".to_string(),
    json!(content_type.clone().unwrap_or_default()),
  );
  if let Some(encoded) = header("pact-message-metadata") {
    use base64::Engine as _;
    let decoded = base64::engine::general_purpose::STANDARD
      .decode(encoded.trim())
      .ok()
      .and_then(|bytes| serde_json::from_slice::<serde_json::Map<String, Value>>(&bytes).ok());
    match decoded {
      Some(members) => metadata.extend(members),
      // Today's verifier reads one it cannot decode as no metadata, and so does this.
      None => tracing::debug!("the Pact-Message-Metadata header is not base64 JSON; ignored"),
    }
  }
  let mut part = serde_json::Map::new();
  part.insert("metadata".to_string(), json!({ "content": metadata }));
  if !response.body().is_empty() {
    use base64::Engine as _;
    let mut body = json!({
      "content": base64::engine::general_purpose::STANDARD.encode(response.body()),
      "encoded": "base64",
    });
    if let Some(content_type) = content_type {
      body["content-type"] = json!(content_type);
    }
    part.insert("body".to_string(), body);
  }
  let mut changes = serde_json::Map::new();
  let mut parts = serde_json::Map::new();
  parts.insert(
    pact_janus_kernel::legacy::pact::MESSAGE_PART.to_string(),
    Value::Object(part),
  );
  changes.insert("parts".to_string(), Value::Object(parts));
  InvokeResult {
    changes,
    ..InvokeResult::ok()
  }
}

/// How many times a request is tried again when `client.retries` does not say (ADR 0044): a state
/// call twice, anything else never.
fn default_retries(context: &Value) -> u32 {
  match context.get("point").and_then(Value::as_str) {
    Some("state-setup" | "state-teardown") => STATE_RETRIES,
    _ => 0,
  }
}

/// The answers today's verifier tries again: a server error, a request timeout, or too many
/// requests. A refused connection is not one: nothing is there to warm up.
fn retryable(status: u16) -> bool {
  status >= 500 || status == 408 || status == 429
}

/// The wait before the `attempt`th retry: 100 ms, then 1 s, ten times longer each time after, as
/// today's state calls were designed to wait (pact-reference #197).
fn retry_delay(attempt: u32) -> Duration {
  Duration::from_millis(10u64.saturating_pow(attempt.saturating_add(1)))
}

/// `url` with `pairs` added to its query, each encoded.
fn with_query(url: &str, pairs: &[(String, String)]) -> Result<String, String> {
  let mut url = url::Url::parse(url).map_err(|err| format!("is not a URL: {err}"))?;
  {
    let mut query = url.query_pairs_mut();
    for (name, value) in pairs {
      query.append_pair(name, value);
    }
  }
  Ok(url.to_string())
}

/// Today's `--state-change-as-query` request: the state's name, the action and each parameter as a
/// query parameter, a string as it is and anything else as its JSON text, with no body.
fn state_change_query(context: &Value) -> Vec<(String, String)> {
  let body = state_change_body(context);
  let mut pairs = vec![
    (
      "state".to_string(),
      body["state"].as_str().unwrap_or_default().to_string(),
    ),
    (
      "action".to_string(),
      body["action"].as_str().unwrap_or_default().to_string(),
    ),
  ];
  if let Some(params) = body["params"].as_object() {
    for (name, value) in params {
      let value = match value {
        Value::String(text) => text.clone(),
        other => other.to_string(),
      };
      pairs.push((name.clone(), value));
    }
  }
  pairs
}

/// A state-change endpoint's 2xx reply, read as today's verifiers read it (spec §8.4, ADR 0034): a
/// JSON object is the provider-state values a `ProviderState` generator reads, unless it has an
/// `outcome`, which makes it a hook result (so an endpoint can still answer `unsupported`). Any
/// other body is `ok` with nothing to add, which is what today's verifiers make of it.
fn state_change_result(text: &str) -> Result<InvokeResult, HookFailure> {
  match serde_json::from_str::<Value>(text) {
    Ok(document @ Value::Object(_)) if document.get("outcome").is_some() => InvokeResult::parse(&document)
      .map_err(|message| HookFailure {
        outcome: Outcome::Errored,
        error: json!({
          "code": "hook-result-invalid",
          "message": message,
          "details": { "body": excerpt(text) },
        }),
      }),
    Ok(Value::Object(values)) => Ok(InvokeResult {
      values: Some(values),
      ..InvokeResult::ok()
    }),
    _ => Ok(InvokeResult::ok()),
  }
}

/// The v3/v4 provider-state body (spec §8.4): `{ state, params, action }`, with `action` taken from
/// the point — which is the whole of what an existing endpoint needs to keep working.
fn state_change_body(context: &Value) -> Value {
  let state = context.get("state");
  let action = match context.get("point").and_then(Value::as_str) {
    Some("state-teardown") => "teardown",
    _ => "setup",
  };
  json!({
    "state": state.and_then(|s| s.get("name")).cloned().unwrap_or(Value::Null),
    "params": state.and_then(|s| s.get("params")).cloned().unwrap_or_else(|| json!({})),
    "action": action,
  })
}

fn excerpt(text: &str) -> String {
  let end = text
    .char_indices()
    .map(|(i, _)| i)
    .take_while(|i| *i <= BODY_EXCERPT)
    .last()
    .unwrap_or(0);
  text[..end].to_string()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn the_state_change_body_is_the_v3_document_and_nothing_else() {
    let context = json!({
      "point": "state-setup",
      "state": { "name": "an order exists", "params": { "id": "66", "shipped": true } },
      "config": { "secret": "hunter2" },
    });
    assert_eq!(
      state_change_body(&context),
      json!({
        "state": "an order exists",
        "params": { "id": "66", "shipped": true },
        "action": "setup"
      }),
      "a v3 endpoint receives exactly what it received before Janus existed — no config, no secrets"
    );
  }

  #[test]
  fn the_point_decides_the_action() {
    let context = json!({ "point": "state-teardown", "state": { "name": "an order exists" } });
    assert_eq!(state_change_body(&context)["action"], json!("teardown"));
  }

  #[test]
  fn a_state_with_no_parameters_still_sends_an_object() {
    let context = json!({ "point": "state-setup", "state": { "name": "no orders exist" } });
    assert_eq!(state_change_body(&context)["params"], json!({}));
  }

  #[test]
  fn a_state_change_reply_object_is_the_provider_state_values() {
    let result = state_change_result(r#"{"id": 1234, "name": "Mary"}"#).unwrap();
    assert_eq!(result.outcome(), Outcome::Ok);
    assert_eq!(
      result.values.map(Value::Object),
      Some(json!({ "id": 1234, "name": "Mary" })),
      "what today's verifiers hand a ProviderState generator"
    );
  }

  #[test]
  fn a_state_change_reply_with_an_outcome_is_a_hook_result() {
    let result = state_change_result(r#"{"outcome": "unsupported", "error": {"message": "no"}}"#).unwrap();
    assert_eq!(result.outcome(), Outcome::Unsupported);
    assert_eq!(result.values, None);
  }

  #[test]
  fn a_state_call_is_tried_three_times_and_nothing_else_is_tried_again() {
    assert_eq!(default_retries(&json!({ "point": "state-setup" })), 2);
    assert_eq!(default_retries(&json!({ "point": "state-teardown" })), 2);
    assert_eq!(default_retries(&json!({ "point": "before-request" })), 0);
    assert_eq!(retry_delay(1), Duration::from_millis(100));
    assert_eq!(retry_delay(2), Duration::from_secs(1));
    for status in [500, 503, 408, 429] {
      assert!(retryable(status), "{status}");
    }
    for status in [200, 400, 404] {
      assert!(!retryable(status), "{status}");
    }
  }

  #[test]
  fn the_query_form_is_todays_state_change_as_query() {
    let context = json!({
      "point": "state-teardown",
      "state": { "name": "an order exists", "params": { "id": "66", "shipped": true } },
    });
    let pairs = state_change_query(&context);
    assert_eq!(
      with_query("http://localhost:8080/state?x=1", &pairs).unwrap(),
      "http://localhost:8080/state?x=1&state=an+order+exists&action=teardown&id=66&shipped=true"
    );
  }

  #[test]
  fn any_other_state_change_reply_is_ok_with_no_values() {
    for body in ["OK", "[1, 2]", "42", "{not json"] {
      let result = state_change_result(body).unwrap();
      assert_eq!(result.outcome(), Outcome::Ok, "{body}");
      assert_eq!(result.values, None, "{body}");
    }
  }
}
