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

use pact_janus_kernel::hooks::{HookFailure, HookInvoker, InvokeResult, Outcome};
use serde_json::{Value, json};
use std::time::Duration;

/// How much of a non-2xx response body is quoted back in the failure (spec §8.4: "a bounded
/// excerpt").
const BODY_EXCERPT: usize = 1024;

#[derive(Debug, Default)]
pub struct HttpHooks;

impl HttpHooks {
  pub fn new() -> Self {
    HttpHooks
  }
}

impl HookInvoker for HttpHooks {
  fn invoke(&self, run: &Value, context: &Value, deadline_ms: u64) -> Result<InvokeResult, HookFailure> {
    let Some(url) = run.get("url").and_then(Value::as_str) else {
      return Err(HookFailure::errored("an http hook must name a url"));
    };
    let method = run
      .get("method")
      .and_then(Value::as_str)
      .unwrap_or("POST")
      .to_ascii_uppercase();
    // A configuration written before a rename names the context format by its earlier name, and
    // still gets it (staged plan S0.5).
    let context_format = pact_janus_kernel::persisted::HOOK_BODY_FORMAT;
    let state_change = run.get("format").and_then(Value::as_str) == Some("pact-state-change");
    let body = match run
      .get("format")
      .and_then(Value::as_str)
      .unwrap_or(context_format.writes())
    {
      "pact-state-change" => state_change_body(context),
      format if context_format.reads(format) => context.clone(),
      other => {
        return Err(HookFailure::errored(format!(
          "'{other}' is not a body format this engine knows ('{}', 'pact-state-change')",
          context_format.writes()
        )));
      }
    };

    let agent: ureq::Agent = ureq::Agent::config_builder()
      .http_status_as_error(false)
      .timeout_global(Some(Duration::from_millis(deadline_ms)))
      .build()
      .into();

    let mut request = ureq::http::Request::builder()
      .method(method.as_str())
      .uri(url)
      .header("content-type", "application/json");
    if let Some(Value::Object(headers)) = run.get("headers") {
      for (name, value) in headers {
        if let Some(value) = value.as_str() {
          request = request.header(name.as_str(), value);
        }
      }
    }
    let payload = serde_json::to_vec(&body).expect("a HookContext always serializes");
    let request = request
      .body(payload.as_slice())
      .map_err(|err| HookFailure::errored(format!("could not build the request to {url}: {err}")))?;

    let mut response = agent.run(request).map_err(|err| {
      // A timeout and a refused connection are different facts about the endpoint, and a reader
      // chasing one should not be sent to the other.
      if matches!(err, ureq::Error::Timeout(_)) {
        HookFailure::timed_out(deadline_ms)
      } else {
        HookFailure::errored(format!("{url}: {err}"))
      }
    })?;

    let status = response.status().as_u16();
    let text = response.body_mut().read_to_string().unwrap_or_default();

    if !(200..300).contains(&status) {
      return Ok(InvokeResult {
        outcome: Some(Outcome::Failed),
        error: Some(json!({
          "code": "hook-http-failed",
          "message": format!("{url} answered {status}"),
          "details": { "status": status, "body": excerpt(&text) },
        })),
        ..InvokeResult::default()
      });
    }

    if text.trim().is_empty() {
      // A v3 state endpoint that returns nothing is `ok` — which is exactly what makes today's
      // providers verifiable unchanged.
      return Ok(InvokeResult::ok());
    }

    if state_change {
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
          "message": format!("{url} answered 2xx with something that is not JSON: {err}"),
          "details": { "body": excerpt(&text) },
        })),
        ..InvokeResult::default()
      }),
    }
  }
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
  fn any_other_state_change_reply_is_ok_with_no_values() {
    for body in ["OK", "[1, 2]", "42", "{not json"] {
      let result = state_change_result(body).unwrap();
      assert_eq!(result.outcome(), Outcome::Ok, "{body}");
      assert_eq!(result.values, None, "{body}");
    }
  }
}
