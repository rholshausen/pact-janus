//! A built-in **hook component** (component-interfaces spec §8, lifecycle-hooks spec §8.1; plan
//! task 5.3): the auth hook, as the thing the RFC says hooks should mostly be — a *product*, not a
//! script each project rewrites.
//!
//! It answers two points and skips the rest:
//!
//! - **`before-verification`** — acquire a token, once, for the whole run. That is the point's
//!   reason for existing (spec §3.1): a token fetch at `before-request` would run once per
//!   exchange, and a hundred-variant run would fetch a hundred tokens.
//! - **`before-request`** — add the credential to the outbound headers, merging rather than
//!   replacing, because the recorded request may carry headers of its own and a hook that adds an
//!   `authorization` must not quietly drop a `content-type`.
//!
//! Anything else is `skipped`. That is the open-vocabulary rule at the hook boundary (spec §12.2):
//! a hook handed a point it does not know answers `skipped` rather than guessing what to do.
//!
//! **The token never leaves this component.** It is not returned as `data` (which reaches later
//! hooks and, when `report-data` is set, the run summary), and the `data` this component does
//! return says only what kind of credential it holds and how long it is good for — enough to debug
//! an expiry, useless to a log scraper.

use pact_janus_kernel::component::{ComponentError, HookComponent, Invoke};
use serde_json::{Map, Value, json};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// How long before a token's stated expiry this component refetches, so a run does not present a
/// credential that expires between the check and the request.
const REFRESH_MARGIN: Duration = Duration::from_secs(30);

struct Token {
  value: String,
  kind: String,
  expires_at: Option<Instant>,
}

impl Token {
  fn usable(&self) -> bool {
    match self.expires_at {
      None => true,
      Some(expiry) => Instant::now() + REFRESH_MARGIN < expiry,
    }
  }
}

/// The component. One instance answers every invocation (component-interfaces spec §3.4), so the
/// token it holds is shared by every exchange of the run — which is the whole point.
#[derive(Default)]
pub struct Oauth2Hook {
  token: Mutex<Option<Token>>,
}

impl Oauth2Hook {
  pub fn new() -> Self {
    Oauth2Hook::default()
  }

  /// The component's name in a configuration's `run: { kind: component, component: … }`.
  pub const NAME: &'static str = "oauth2";
}

impl HookComponent for Oauth2Hook {
  fn invoke(&self, req: Invoke) -> Result<Value, ComponentError> {
    let config = req.config.clone().unwrap_or_else(|| json!({}));
    match req.point.as_str() {
      "before-verification" => match self.acquire(&config, req.deadline_ms) {
        Ok(described) => Ok(json!({ "outcome": "ok", "data": described })),
        Err(error) => Ok(json!({ "outcome": "failed", "error": error })),
      },
      "before-request" => self.authorize(&req, &config),
      other => {
        tracing::debug!(point = other, "oauth2 has nothing to do at this point");
        Ok(json!({ "outcome": "skipped" }))
      }
    }
  }
}

impl Oauth2Hook {
  /// Get a usable token, fetching one if there is none or the one held is about to expire.
  /// Returns what may be *said* about it, never the token itself.
  fn acquire(&self, config: &Value, deadline_ms: u64) -> Result<Value, Value> {
    {
      let held = self.token.lock().expect("token lock poisoned");
      if let Some(token) = held.as_ref()
        && token.usable()
      {
        return Ok(describe(token));
      }
    }

    // A pre-issued token is the simple case and deliberately supported: a project whose CI already
    // has one should not have to stand up a token endpoint to use this component.
    if let Some(value) = config.get("token").and_then(Value::as_str) {
      let token = Token {
        value: value.to_string(),
        kind: scheme(config),
        expires_at: None,
      };
      let described = describe(&token);
      *self.token.lock().expect("token lock poisoned") = Some(token);
      return Ok(described);
    }

    let token = fetch(config, deadline_ms)?;
    let described = describe(&token);
    *self.token.lock().expect("token lock poisoned") = Some(token);
    Ok(described)
  }

  /// Add the credential to the outbound headers. The headers slot is read from the context and
  /// written back whole — design 2.6's slot wrapper is unconditional, so a change replaces a slot
  /// rather than reaching inside one.
  fn authorize(&self, req: &Invoke, config: &Value) -> Result<Value, ComponentError> {
    let described = match self.acquire(config, req.deadline_ms) {
      Ok(described) => described,
      Err(error) => return Ok(json!({ "outcome": "failed", "error": error })),
    };
    let held = self.token.lock().expect("token lock poisoned");
    let Some(token) = held.as_ref() else {
      return Ok(json!({ "outcome": "failed", "error": {
        "code": "no-token", "message": "no credential was acquired" } }));
    };

    let header = config
      .get("header")
      .and_then(Value::as_str)
      .unwrap_or("authorization")
      .to_ascii_lowercase();
    let mut headers = req
      .context
      .get("parts")
      .and_then(|parts| parts.get("request"))
      .and_then(|request| request.get(HEADERS_SLOT))
      .and_then(|slot| slot.get("content"))
      .and_then(Value::as_object)
      .cloned()
      .unwrap_or_default();
    headers.insert(header, json!([format!("{} {}", token.kind, token.value)]));

    Ok(json!({
      "outcome": "ok",
      "changes": { "parts.request.headers": { "content": Value::Object(headers) } },
      "data": described,
    }))
  }
}

/// The slot the HTTP transport calls its headers (component-interfaces spec §4: slot names are the
/// transport's vocabulary, not the kernel's). A constant here rather than a guess at the call site.
const HEADERS_SLOT: &str = "headers";

fn scheme(config: &Value) -> String {
  config
    .get("scheme")
    .and_then(Value::as_str)
    .unwrap_or("Bearer")
    .to_string()
}

/// What may be said about a token: its scheme and its lifetime, never its value.
fn describe(token: &Token) -> Value {
  json!({
    "token-type": token.kind,
    "expires-in-ms": token.expires_at.map(|expiry| {
      expiry.saturating_duration_since(Instant::now()).as_millis() as u64
    }),
  })
}

/// The client-credentials grant: form-encoded in, JSON out.
fn fetch(config: &Value, deadline_ms: u64) -> Result<Token, Value> {
  let url = config
    .get("token-url")
    .and_then(Value::as_str)
    .ok_or_else(|| error("config-invalid", "oauth2 needs a 'token-url' or a 'token'"))?;
  let client_id = config
    .get("client-id")
    .and_then(Value::as_str)
    .unwrap_or_default();
  let client_secret = config
    .get("client-secret")
    .and_then(Value::as_str)
    .unwrap_or_default();

  let mut form = format!("grant_type=client_credentials&client_id={client_id}&client_secret={client_secret}");
  if let Some(scope) = config.get("scope").and_then(Value::as_str) {
    form.push_str(&format!("&scope={scope}"));
  }

  let agent: ureq::Agent = ureq::Agent::config_builder()
    .http_status_as_error(false)
    .timeout_global(Some(Duration::from_millis(deadline_ms)))
    .build()
    .into();
  let mut response = agent
    .post(url)
    .header("content-type", "application/x-www-form-urlencoded")
    .send(form.as_str())
    .map_err(|err| error("token-endpoint-unreachable", format!("{url}: {err}")))?;

  let status = response.status().as_u16();
  let body = response.body_mut().read_to_string().unwrap_or_default();
  if !(200..300).contains(&status) {
    // The body of a failed token request routinely carries the reason (`invalid_client`), and it
    // is not a secret — the credential that failed is the one the configuration already shows.
    return Err(error(
      "token-request-failed",
      format!("{url} answered {status}: {body}"),
    ));
  }

  let document: Value =
    serde_json::from_str(&body).map_err(|err| error("token-response-invalid", format!("{url}: {err}")))?;
  let value = document
    .get("access_token")
    .and_then(Value::as_str)
    .ok_or_else(|| {
      error(
        "token-response-invalid",
        format!("{url} returned no access_token"),
      )
    })?;
  let kind = document
    .get("token_type")
    .and_then(Value::as_str)
    .map(capitalise)
    .unwrap_or_else(|| scheme(config));
  let expires_at = document
    .get("expires_in")
    .and_then(Value::as_u64)
    .map(|seconds| Instant::now() + Duration::from_secs(seconds));

  Ok(Token {
    value: value.to_string(),
    kind,
    expires_at,
  })
}

/// `bearer` from a token endpoint is `Bearer` on the wire — servers are not required to be kind
/// about the case, and the scheme is the one part of the header this component chooses.
fn capitalise(text: &str) -> String {
  let mut chars = text.chars();
  match chars.next() {
    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    None => String::new(),
  }
}

fn error(code: &str, message: impl Into<String>) -> Value {
  let mut error = Map::new();
  error.insert("code".to_string(), json!(code));
  error.insert("message".to_string(), json!(message.into()));
  Value::Object(error)
}
