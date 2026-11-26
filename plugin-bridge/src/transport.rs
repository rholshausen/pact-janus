//! A plugin's transport, driven as a Janus transport kind (ADR 0111): `transport/start` in the drive role
//! names where the provider is, and one `transport/send` is the whole exchange, as today's verifier has the
//! plugin run it (`pact_verifier`'s `verify_interaction_using_transport`).
//!
//! The `request` part carries what the plugin builds the request from: `pact`, the v4 pact holding the one
//! interaction, `key`, its key there, and `provider-state`, the exchange's provider-state values. The bridge
//! asks the plugin to prepare the request and then to verify the interaction, each with the context today's
//! verifier builds, and replies with the plugin's verdict as the `response` part's `verdict` slot, which
//! the bridge's `<component>:verdict` action reads ([`verdict_result`]).

use crate::error::ComponentError;
use pact_plugin_driver::verification::{InteractionVerificationDetails, InteractionVerificationResult};
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;

/// The part a plugin transport's exchange starts with, and the one its reply is (spec §5.7).
pub const INITIATING: &str = "request";
pub const ANSWERING: &str = "response";
/// The reply's one slot: the plugin's verdict.
pub const VERDICT: &str = "verdict";

/// Where a drive instance sends: the provider's host and port, and the headers today's `--header`s add to
/// the request's metadata.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Endpoint {
  pub host: String,
  pub port: Option<u16>,
  pub headers: BTreeMap<String, String>,
}

impl Endpoint {
  /// Reads a drive instance's options: `host` and `port`, else the host and port of `base-url`, the HTTP
  /// transport's spelling a host binds the provider with, which is where today's verifier reaches a plugin
  /// transport with no `--transports` entry of its own; and `headers`, names to values.
  pub fn read(options: Option<&Value>) -> Result<Endpoint, ComponentError> {
    let options = match options {
      None | Some(Value::Null) => &Value::Object(Map::new()),
      Some(options @ Value::Object(_)) => options,
      Some(_) => {
        return Err(ComponentError::invalid_config(
          "a transport's options must be an object",
        ));
      }
    };
    let (mut host, mut port) = (None, None);
    if let Some(url) = options.get("base-url") {
      let url = url
        .as_str()
        .ok_or_else(|| ComponentError::invalid_config("'base-url' must be a string"))?;
      let uri: http::Uri = url
        .parse()
        .map_err(|err| ComponentError::invalid_config(format!("'base-url' {url} is not a URL: {err}")))?;
      host = uri.host().map(|host| host.trim_matches(['[', ']']).to_string());
      port = uri.port_u16();
    }
    if let Some(named) = options.get("host") {
      host = Some(
        named
          .as_str()
          .ok_or_else(|| ComponentError::invalid_config("'host' must be a string"))?
          .to_string(),
      );
    }
    if let Some(named) = options.get("port") {
      port = Some(
        named
          .as_u64()
          .and_then(|port| u16::try_from(port).ok())
          .ok_or_else(|| ComponentError::invalid_config("'port' must be a port number"))?,
      );
    }
    let mut headers = BTreeMap::new();
    match options.get("headers") {
      None | Some(Value::Null) => {}
      Some(Value::Object(named)) => {
        for (name, value) in named {
          let value = value.as_str().ok_or_else(|| {
            ComponentError::invalid_config(format!("the header '{name}' must have a string value"))
          })?;
          headers.insert(name.clone(), value.to_string());
        }
      }
      Some(_) => return Err(ComponentError::invalid_config("'headers' must be an object")),
    }
    Ok(Endpoint {
      // Today's verifier's `--hostname` defaults to localhost.
      host: host.unwrap_or_else(|| "localhost".to_string()),
      port,
      headers,
    })
  }

  /// The descriptor `transport/start` answers with (spec §4).
  pub fn descriptor(&self) -> Value {
    let mut descriptor = json!({ "host": self.host });
    if let Some(port) = self.port {
      descriptor["port"] = json!(port);
    }
    descriptor
  }

  /// The context today's verifier hands the plugin: `host`, `port` when it has one, and `providerState`.
  pub fn context(&self, provider_state: Map<String, Value>) -> std::collections::HashMap<String, Value> {
    let mut context = std::collections::HashMap::new();
    context.insert("host".to_string(), json!(self.host));
    if let Some(port) = self.port {
      context.insert("port".to_string(), json!(port));
    }
    context.insert("providerState".to_string(), Value::Object(provider_state));
    context
  }
}

/// What one `send` hands the plugin, read from its `request` part.
#[derive(Debug, Clone, PartialEq)]
pub struct Exchange {
  pub pact: Value,
  pub key: String,
  pub provider_state: Map<String, Value>,
}

impl Exchange {
  /// Reads the `request` part of a `send`'s parts. Each slot is a slot value, its `content` plain JSON.
  pub fn read(parts: Option<&Value>) -> Result<Exchange, ComponentError> {
    let request = parts
      .and_then(|parts| parts.get(INITIATING))
      .and_then(Value::as_object)
      .ok_or_else(|| {
        ComponentError::invalid_config(format!("a plugin transport's send needs a '{INITIATING}' part"))
      })?;
    let slot = |name: &str| request.get(name).map(|slot| slot.get("content").unwrap_or(slot));
    let pact = slot("pact")
      .filter(|pact| pact.is_object())
      .cloned()
      .ok_or_else(|| ComponentError::invalid_config("the request's 'pact' slot must hold a v4 pact"))?;
    let key = slot("key")
      .and_then(Value::as_str)
      .ok_or_else(|| {
        ComponentError::invalid_config("the request's 'key' slot must hold the interaction's key")
      })?
      .to_string();
    let provider_state = match slot("provider-state") {
      None | Some(Value::Null) => Map::new(),
      Some(Value::Object(values)) => values.clone(),
      Some(_) => {
        return Err(ComponentError::invalid_config(
          "the request's 'provider-state' slot must hold an object",
        ));
      }
    };
    Ok(Exchange {
      pact,
      key,
      provider_state,
    })
  }
}

/// The plugin's verdict, as the reply's `verdict` slot carries it: `ok`, each mismatch with the plugin's
/// path as `at`, each error, and the plugin's output.
pub fn verdict(result: &InteractionVerificationResult) -> Value {
  let mut mismatches = Vec::new();
  let mut errors = Vec::new();
  for detail in &result.details {
    match detail {
      InteractionVerificationDetails::Mismatch {
        expected,
        actual,
        mismatch,
        path,
      } => {
        let mut described = Map::new();
        described.insert("message".to_string(), json!(mismatch));
        for (key, value) in [
          ("at", path.clone()),
          ("expected", String::from_utf8_lossy(expected).into_owned()),
          ("actual", String::from_utf8_lossy(actual).into_owned()),
        ] {
          if !value.is_empty() {
            described.insert(key.to_string(), json!(value));
          }
        }
        mismatches.push(Value::Object(described));
      }
      InteractionVerificationDetails::Error(error) => errors.push(json!(error)),
    }
  }
  json!({ "ok": result.ok, "mismatches": mismatches, "errors": errors, "output": result.output })
}

/// The reply to a `send`: the verdict as the `response` part's one slot.
pub fn reply(verdict: Value) -> Value {
  json!({ "reply": { ANSWERING: { VERDICT: { "content": verdict } } } })
}

/// What `<component>:verdict` makes of a verdict (ADR 0111 decision 3), as a plan result document
/// (plan-grammar spec §2.3): `ok`, or one `error` whose `mismatches` are the plugin's mismatches, located
/// `at` its path, and its errors, unlocated. A verdict that is not `ok` and names nothing still fails, as
/// today's verifier fails it.
pub fn verdict_result(value: &Value) -> Result<Value, ComponentError> {
  let verdict = value.get("content").unwrap_or(value);
  let ok = verdict
    .get("ok")
    .and_then(Value::as_bool)
    .ok_or_else(|| ComponentError::invalid_config("a verdict has a boolean 'ok'"))?;
  if ok {
    return Ok(json!({ "status": "ok" }));
  }
  let mut found: Vec<Value> = Vec::new();
  for mismatch in verdict
    .get("mismatches")
    .and_then(Value::as_array)
    .into_iter()
    .flatten()
  {
    let mut described = Map::new();
    let message = mismatch
      .get("message")
      .and_then(Value::as_str)
      .unwrap_or_default();
    described.insert("message".to_string(), json!(message));
    for key in ["at", "expected", "actual"] {
      if let Some(value) = mismatch
        .get(key)
        .and_then(Value::as_str)
        .filter(|v| !v.is_empty())
      {
        described.insert(key.to_string(), json!(value));
      }
    }
    found.push(Value::Object(described));
  }
  for error in verdict
    .get("errors")
    .and_then(Value::as_array)
    .into_iter()
    .flatten()
    .filter_map(Value::as_str)
  {
    found.push(json!({ "message": error }));
  }
  if found.is_empty() {
    found.push(json!({ "message": "the plugin failed the interaction and said nothing about why" }));
  }
  let message = found
    .iter()
    .map(|found| match found.get("at").and_then(Value::as_str) {
      Some(at) => format!("{at}: {}", found["message"].as_str().unwrap_or_default()),
      None => found["message"].as_str().unwrap_or_default().to_string(),
    })
    .collect::<Vec<_>>()
    .join("; ");
  Ok(json!({ "status": "error", "message": message, "mismatches": found }))
}

#[cfg(test)]
mod tests {
  use super::*;
  use bytes::Bytes;
  use pretty_assertions::assert_eq;

  #[test]
  fn an_endpoint_is_named_by_host_and_port_or_by_a_base_url() {
    assert_eq!(
      Endpoint::read(Some(&json!({ "host": "127.0.0.1", "port": 8222 }))).unwrap(),
      Endpoint {
        host: "127.0.0.1".into(),
        port: Some(8222),
        headers: BTreeMap::new()
      }
    );
    let endpoint = Endpoint::read(Some(&json!({
      "base-url": "http://127.0.0.1:37757/base",
      "headers": { "x-trace": "1" },
      "client": { "timeouts": { "request-ms": 100 } }
    })))
    .unwrap();
    assert_eq!(endpoint.host, "127.0.0.1");
    assert_eq!(endpoint.port, Some(37757));
    assert_eq!(endpoint.headers["x-trace"], "1");
    assert_eq!(
      endpoint.descriptor(),
      json!({ "host": "127.0.0.1", "port": 37757 })
    );
    let context = endpoint.context(Map::from_iter([("id".to_string(), json!(7))]));
    assert_eq!(context["port"], json!(37757));
    assert_eq!(context["providerState"], json!({ "id": 7 }));
    assert_eq!(
      Endpoint::read(None).unwrap().host,
      "localhost",
      "today's --hostname default"
    );
    for options in [
      json!("x"),
      json!({ "port": "x" }),
      json!({ "base-url": 1 }),
      json!({ "headers": [] }),
    ] {
      assert_eq!(
        Endpoint::read(Some(&options)).unwrap_err().code,
        "invalid-config",
        "{options}"
      );
    }
  }

  #[test]
  fn an_exchange_is_read_from_the_request_parts_slots() {
    let exchange = Exchange::read(Some(&json!({ "request": {
      "pact": { "content": { "interactions": [] } },
      "key": { "content": "1234" },
      "provider-state": { "content": { "id": 1 } }
    } })))
    .unwrap();
    assert_eq!(exchange.key, "1234");
    assert_eq!(exchange.provider_state["id"], json!(1));
    assert!(Exchange::read(Some(&json!({ "request": { "key": { "content": "1" } } }))).is_err());
    assert!(Exchange::read(None).is_err());
  }

  #[test]
  fn a_verdict_keeps_each_mismatch_and_error_and_is_read_as_one_result() {
    let result = InteractionVerificationResult {
      ok: false,
      details: vec![
        InteractionVerificationDetails::Mismatch {
          expected: Bytes::from_static(b"12.0"),
          actual: Bytes::from_static(b"0.0"),
          mismatch: "Expected 0.0 to be equal to 12.0".into(),
          path: "$.value[0]".into(),
        },
        InteractionVerificationDetails::Error("the call failed".into()),
      ],
      output: vec!["Verifying a gRPC call".into()],
    };
    let verdict = verdict(&result);
    assert_eq!(
      verdict,
      json!({
        "ok": false,
        "mismatches": [{ "message": "Expected 0.0 to be equal to 12.0", "at": "$.value[0]", "expected": "12.0", "actual": "0.0" }],
        "errors": ["the call failed"],
        "output": ["Verifying a gRPC call"]
      })
    );
    assert_eq!(
      reply(verdict.clone())["reply"]["response"]["verdict"]["content"]["ok"],
      false
    );
    assert_eq!(
      verdict_result(&json!({ "content": verdict })).unwrap(),
      json!({
        "status": "error",
        "message": "$.value[0]: Expected 0.0 to be equal to 12.0; the call failed",
        "mismatches": [
          { "message": "Expected 0.0 to be equal to 12.0", "at": "$.value[0]", "expected": "12.0", "actual": "0.0" },
          { "message": "the call failed" }
        ]
      })
    );
    assert_eq!(
      verdict_result(&json!({ "content": { "ok": true } })).unwrap(),
      json!({ "status": "ok" })
    );
    assert_eq!(
      verdict_result(&json!({ "content": { "ok": false } })).unwrap()["status"],
      "error",
      "a failure that says nothing still fails"
    );
    assert_eq!(
      verdict_result(&json!({ "content": "nonsense" }))
        .unwrap_err()
        .code,
      "invalid-config"
    );
  }
}
