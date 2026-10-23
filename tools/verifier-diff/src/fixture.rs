//! The fixture corpus (`corpora/verifier/`): what a case is, and how the corpus becomes cases.
//!
//! A fixture is a directory holding `fixture.json` and the pacts it names. It says which provider
//! answers — the scripted one, with the routes the fixture lists, or the sample order-service — and
//! whether its interactions are also run mutated ([`crate::mutate`]). One fixture is one case as
//! written, plus a case per mutation.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use pact_models::bodies::OptionalBody;
use pact_models::content_types::ContentType;
use pact_models::v4::synch_http::SynchronousHttp;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const FIXTURE_FILE: &str = "fixture.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Fixture {
  pub description: String,
  /// Where the fixture came from, for a reader: free-form.
  #[serde(default, skip_serializing_if = "Value::is_null")]
  pub source: Value,
  /// Pact files, relative to the fixture's directory.
  pub pacts: Vec<String>,
  pub provider: ProviderSpec,
  /// The verdict the fixture's source says the whole run reaches, when it says one. Today's
  /// verifier is held to it — a fixture it disagrees with is a fixture translated wrongly.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub expected: Option<RunVerdict>,
  /// Also run every interaction against mutated responses.
  #[serde(default, skip_serializing_if = "std::ops::Not::not")]
  pub mutations: bool,
  /// Publish the pacts to a Pact Broker first, and have both verifiers fetch them from it, and
  /// publish their results back, instead of reading them from files (S1.3).
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub broker: Option<BrokerSpec>,
  /// How both verifiers reach the provider, given to each in its own spelling (S1.5).
  #[serde(default, skip_serializing_if = "ConnectionSpec::is_default")]
  pub connection: ConnectionSpec,
}

/// The provider-connection options both verifiers are given (S1.5), in today's verifier's terms:
/// `--base-path`, `--header`, `--request-timeout` and `--disable-ssl-verification`, which
/// `janus verify` spells the same way.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ConnectionSpec {
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub base_path: Option<String>,
  /// Header name to value, each given as `--header NAME=VALUE`.
  #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
  pub headers: BTreeMap<String, String>,
  /// Milliseconds.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub request_timeout: Option<u64>,
  /// Serve the provider over TLS, with a certificate from an authority nothing trusts.
  #[serde(default, skip_serializing_if = "std::ops::Not::not")]
  pub tls: bool,
  #[serde(default, skip_serializing_if = "std::ops::Not::not")]
  pub disable_ssl_verification: bool,
}

impl ConnectionSpec {
  pub fn is_default(&self) -> bool {
    self == &ConnectionSpec::default()
  }
}

/// A fixture verified from a Pact Broker: how its pacts are published, what was verified before the
/// run, and how the verifiers fetch and publish. Every pact the fixture names is published, under
/// names of its own for each verifier, so neither sees what the other did to the broker.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct BrokerSpec {
  /// How a pact is published, by its file name. One not listed is published as consumer
  /// version `1.0.0` on branch `main`.
  #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
  pub publish: BTreeMap<String, Publication>,
  /// Verification results published before the run, as an earlier provider build would have.
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub results: Vec<PriorResult>,
  /// How the verifiers fetch the pacts, and whether they publish.
  #[serde(default)]
  pub verify: BrokerVerify,
  /// What the source says the run publishes for a pact, by its file name. Today's verifier is
  /// held to it, as to `expected`.
  #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
  pub expected_published: BTreeMap<String, Published>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Publication {
  #[serde(default = "first_version")]
  pub version: String,
  /// `None` publishes it on no branch.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub branch: Option<String>,
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub tags: Vec<String>,
}

impl Default for Publication {
  fn default() -> Self {
    Publication {
      version: first_version(),
      branch: Some(MAIN_BRANCH.to_string()),
      tags: vec![],
    }
  }
}

pub const MAIN_BRANCH: &str = "main";

fn first_version() -> String {
  "1.0.0".to_string()
}

/// A verification result that was there before the run.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct PriorResult {
  /// The pact it is for, by its file name.
  pub pact: String,
  pub provider_version: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub provider_branch: Option<String>,
  pub success: bool,
}

/// The broker options both verifiers are given, in today's verifier's terms.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct BrokerVerify {
  /// Consumer version selectors, as the broker reads them. None: the broker's default, the latest
  /// pact from each consumer.
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub selectors: Vec<Value>,
  #[serde(default, skip_serializing_if = "std::ops::Not::not")]
  pub enable_pending: bool,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub include_wip_pacts_since: Option<String>,
  #[serde(default = "first_version")]
  pub provider_version: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub provider_branch: Option<String>,
  /// Publish the results.
  #[serde(default, skip_serializing_if = "std::ops::Not::not")]
  pub publish: bool,
}

impl Default for BrokerVerify {
  fn default() -> Self {
    BrokerVerify {
      selectors: vec![],
      enable_pending: false,
      include_wip_pacts_since: None,
      provider_version: first_version(),
      provider_branch: None,
      publish: false,
    }
  }
}

/// What a run published for a pact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Published {
  /// A successful result.
  Pass,
  /// A failed result.
  Fail,
  /// No result.
  Absent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RunVerdict {
  Pass,
  Fail,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ProviderSpec {
  /// [`crate::provider::ScriptedProvider`], serving these routes.
  Scripted {
    #[serde(rename = "state-callback")]
    state_callback: StateCallback,
    routes: Vec<Route>,
  },
  /// The sample provider (`samples/order-service`), started in this process with auth off. Its
  /// provider-state endpoint is the state callback.
  OrderService,
}

/// Whether the verifiers are given a provider-state callback, and how it answers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StateCallback {
  /// None configured.
  None,
  /// Configured, and every call succeeds.
  Ok,
  /// Configured, and every setup call fails with a 500.
  Fail,
  /// Configured, and the first call fails with a 503, as a state endpoint still warming up would;
  /// every call after it succeeds. A verifier that retries passes; one that does not fails.
  FailOnce,
  /// Configured, and every setup call answers with these provider-state values as a JSON object,
  /// which is what a `ProviderState` generator reads (`{"values": {"id": 1234}}`; ADR 0034).
  Values(serde_json::Map<String, Value>),
}

/// One request the scripted provider recognises, and its answer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Route {
  /// The provider states this answer belongs to: when several routes recognise a request, the one
  /// whose states were just set up wins.
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub states: Vec<String>,
  pub request: RequestExample,
  pub response: ResponseSpec,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequestExample {
  pub method: String,
  pub path: String,
  /// `None` and empty both mean the request must carry no query.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub query: Option<BTreeMap<String, Vec<String>>>,
  /// Headers the request must carry, with these values. Others are ignored.
  #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
  pub headers: BTreeMap<String, Vec<String>>,
  /// The body the request must carry; `None` accepts any.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub body: Option<Body>,
  /// Values a generator produces, recognised by a regular expression the whole value must match
  /// rather than by example: `path`, `query.<name>`, `headers.<name>`, `body<JSON pointer>`
  /// (`body/one`), or `form.<name>`, every value of a name in a form body. What the example has at
  /// that position is ignored.
  #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
  pub patterns: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ResponseSpec {
  pub status: u16,
  /// How long the provider takes to answer, in milliseconds: a slow provider, for the request
  /// timeout cases.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub delay_ms: Option<u64>,
  #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
  pub headers: BTreeMap<String, Vec<String>>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub body: Option<Body>,
}

/// A body, in whichever form a reader can review: JSON as JSON, text as text, anything else as
/// base64. The bytes sent are [`Body::bytes`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum Body {
  Json(Value),
  Text(String),
  Base64(String),
}

impl Body {
  pub fn bytes(&self) -> Vec<u8> {
    match self {
      Body::Json(value) => serde_json::to_vec(value).unwrap_or_default(),
      Body::Text(text) => text.as_bytes().to_vec(),
      Body::Base64(encoded) => BASE64.decode(encoded).unwrap_or_default(),
    }
  }

  /// The form a reader can review: JSON when the content type says so and the bytes parse, text
  /// when they are UTF-8, base64 otherwise.
  pub fn from_bytes(bytes: &[u8], json: bool) -> Body {
    if json && let Ok(value) = serde_json::from_slice(bytes) {
      return Body::Json(value);
    }
    match std::str::from_utf8(bytes) {
      Ok(text) => Body::Text(text.to_string()),
      Err(_) => Body::Base64(BASE64.encode(bytes)),
    }
  }
}

impl Route {
  /// The route a provider honouring this interaction would answer with: its request as the
  /// example to recognise, its response as the answer, and a `Content-Type` header when the body
  /// has a content type the headers do not already name.
  pub fn from_interaction(interaction: &SynchronousHttp) -> Route {
    let request = &interaction.request;
    let response = &interaction.response;
    let request_body =
      present(&request.body, request.content_type()).map(|(bytes, json)| Body::from_bytes(&bytes, json));
    let response_type = response.content_type();
    let mut headers = headers_of(&response.headers);
    if let (Some(content_type), Some(_)) = (&response_type, present(&response.body, None))
      && !headers
        .keys()
        .any(|name| name.eq_ignore_ascii_case("content-type"))
    {
      headers.insert("Content-Type".to_string(), vec![content_type.to_string()]);
    }
    Route {
      states: interaction
        .provider_states
        .iter()
        .map(|state| state.name.clone())
        .collect(),
      request: RequestExample {
        method: request.method.to_uppercase(),
        path: request.path.clone(),
        query: request.query.as_ref().map(|query| {
          query
            .iter()
            .map(|(name, values)| {
              let values = values
                .iter()
                .map(|value| value.clone().unwrap_or_default())
                .collect();
              (name.clone(), values)
            })
            .collect()
        }),
        headers: headers_of(&request.headers),
        body: request_body,
        patterns: BTreeMap::new(),
      },
      response: ResponseSpec {
        status: response.status,
        delay_ms: None,
        headers,
        body: present(&response.body, response_type).map(|(bytes, json)| Body::from_bytes(&bytes, json)),
      },
    }
  }
}

fn headers_of(
  headers: &Option<std::collections::HashMap<String, Vec<String>>>,
) -> BTreeMap<String, Vec<String>> {
  headers
    .iter()
    .flatten()
    .map(|(name, values)| (name.clone(), values.clone()))
    .collect()
}

/// A present, non-empty body's bytes, and whether it is JSON: by its part's content type, which
/// `pact_models` takes from the headers or detects from the body, as today's verifier does.
fn present(body: &OptionalBody, content_type: Option<ContentType>) -> Option<(Vec<u8>, bool)> {
  match body {
    OptionalBody::Present(bytes, _, _) if !bytes.is_empty() => {
      let json = content_type.as_ref().is_some_and(ContentType::is_json);
      Some((bytes.to_vec(), json))
    }
    _ => None,
  }
}

/// Every fixture under `corpus`, by its directory relative to it (with `/` separators: an id, not
/// a path), in name order.
pub fn load_corpus(corpus: &Path) -> Result<Vec<(String, PathBuf, Fixture)>, String> {
  let mut found = Vec::new();
  walk(corpus, corpus, &mut found)?;
  found.sort_by(|a, b| a.0.cmp(&b.0));
  Ok(found)
}

fn walk(root: &Path, dir: &Path, found: &mut Vec<(String, PathBuf, Fixture)>) -> Result<(), String> {
  let entries = std::fs::read_dir(dir).map_err(|err| format!("{}: {err}", dir.display()))?;
  for entry in entries.filter_map(Result::ok) {
    let path = entry.path();
    if path.is_dir() {
      walk(root, &path, found)?;
    } else if path.file_name().and_then(|name| name.to_str()) == Some(FIXTURE_FILE) {
      let dir = path.parent().unwrap_or(root).to_path_buf();
      let text = std::fs::read_to_string(&path).map_err(|err| format!("{}: {err}", path.display()))?;
      let fixture: Fixture =
        serde_json::from_str(&text).map_err(|err| format!("{}: {err}", path.display()))?;
      found.push((case_id(root, &dir), dir, fixture));
    }
  }
  Ok(())
}

/// A directory under the corpus as an id: its components joined with `/`, whatever the platform.
pub fn case_id(root: &Path, dir: &Path) -> String {
  dir
    .strip_prefix(root)
    .unwrap_or(dir)
    .components()
    .map(|component| component.as_os_str().to_string_lossy().into_owned())
    .collect::<Vec<_>>()
    .join("/")
}

/// One interaction of a v1-v4 pact, as the route a provider honouring it answers with.
pub struct InteractionRoute {
  pub route: Route,
  /// A message (staged plan S1.7): its status is not part of what it is, so no mutation changes it.
  pub message: bool,
}

/// Every interaction of a v1-v4 pact as the route a provider honouring it answers with, read the
/// way today's verifier reads them: an HTTP interaction's request and response
/// ([`Route::from_interaction`]), or a message's request and the message ([`Route::from_message`]).
pub fn routes_of(pact: &Value, source: &str) -> Result<Vec<InteractionRoute>, String> {
  let pact =
    pact_models::pact::load_pact_from_json(source, pact).map_err(|err| format!("{source}: {err}"))?;
  let pact = pact.as_v4_pact().map_err(|err| format!("{source}: {err}"))?;
  pact
    .interactions
    .iter()
    .map(|interaction| {
      if let Some(http) = interaction.as_v4_http() {
        return Ok(InteractionRoute {
          route: Route::from_interaction(&http),
          message: false,
        });
      }
      Route::from_message(interaction.as_ref())
        .map(|route| InteractionRoute { route, message: true })
        .ok_or_else(|| {
          format!(
            "{source}: '{}' is neither an HTTP interaction nor a message",
            interaction.description()
          )
        })
    })
    .collect()
}

/// Where a pact keeps its interactions: a v3 message pact under `messages`, any other under
/// `interactions`.
pub fn interactions_key(pact: &Value) -> &'static str {
  if pact.get("interactions").is_none() && pact.get("messages").is_some() {
    "messages"
  } else {
    "interactions"
  }
}

impl Route {
  /// The route a message provider honouring this message would answer with (staged plan S1.7): the
  /// request today's verifier sends to ask for it — a POST of its description, its states and a
  /// synchronous message's request (`pact_verifier`'s `messages.rs`) — recognised by that body, and
  /// the message: its contents, labelled with its content type, and its metadata as base64 JSON in a
  /// `Pact-Message-Metadata` header. `None` for an interaction that is not a message.
  pub fn from_message(
    interaction: &(dyn pact_models::interaction::Interaction + Send + Sync + std::panic::RefUnwindSafe),
  ) -> Option<Route> {
    let states: Vec<Value> = interaction
      .provider_states()
      .iter()
      .map(|state| state.to_json())
      .collect();
    let mut request = serde_json::Map::new();
    request.insert(
      "description".to_string(),
      Value::String(interaction.description()),
    );
    let contents = if let Some(sync) = interaction.as_v4_sync_message() {
      request.insert("request".to_string(), sync.request.to_json());
      sync.response.first().cloned().unwrap_or_default()
    } else {
      interaction.as_v4_async_message()?.contents
    };
    if !states.is_empty() {
      request.insert("providerStates".to_string(), Value::Array(states));
    }

    let content_type = contents.message_content_type();
    let mut headers = BTreeMap::new();
    let body =
      present(&contents.contents, content_type.clone()).map(|(bytes, json)| Body::from_bytes(&bytes, json));
    if let (Some(content_type), Some(_)) = (&content_type, &body) {
      headers.insert("Content-Type".to_string(), vec![content_type.to_string()]);
    }
    if !contents.metadata.is_empty() {
      let metadata: serde_json::Map<String, Value> = contents
        .metadata
        .iter()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
      headers.insert(
        "Pact-Message-Metadata".to_string(),
        vec![BASE64.encode(Value::Object(metadata).to_string())],
      );
    }
    Some(Route {
      states: interaction
        .provider_states()
        .iter()
        .map(|state| state.name.clone())
        .collect(),
      request: RequestExample {
        method: "POST".to_string(),
        path: "/".to_string(),
        query: None,
        headers: BTreeMap::new(),
        body: Some(Body::Json(Value::Object(request))),
        patterns: BTreeMap::new(),
      },
      response: ResponseSpec {
        status: 200,
        delay_ms: None,
        headers,
        body,
      },
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use pretty_assertions::assert_eq;
  use serde_json::json;

  #[test]
  fn a_body_is_kept_in_the_form_a_reader_can_review() {
    assert_eq!(
      Body::from_bytes(br#"{"a":1}"#, true),
      Body::Json(json!({ "a": 1 }))
    );
    assert_eq!(
      Body::from_bytes(br#"{"a":1}"#, false),
      Body::Text(r#"{"a":1}"#.to_string())
    );
    assert_eq!(
      Body::from_bytes(&[0xff, 0x00], false),
      Body::Base64("/wA=".to_string())
    );
    assert_eq!(Body::Base64("/wA=".to_string()).bytes(), vec![0xff, 0x00]);
  }

  #[test]
  fn a_route_is_the_interaction_a_provider_would_honour() {
    let pact = json!({
      "consumer": { "name": "c" }, "provider": { "name": "p" },
      "interactions": [{
        "description": "d", "providerState": "s",
        "request": { "method": "get", "path": "/a", "query": "x=1&x=2" },
        "response": { "status": 200, "body": { "ok": true } }
      }],
      "metadata": { "pactSpecification": { "version": "2.0.0" } }
    });
    let routes = routes_of(&pact, "test").unwrap();
    assert!(!routes[0].message);
    let route = &routes[0].route;
    assert_eq!(route.states, vec!["s".to_string()]);
    assert_eq!(route.request.method, "GET");
    assert_eq!(
      route.request.query,
      Some(BTreeMap::from([(
        "x".to_string(),
        vec!["1".to_string(), "2".to_string()]
      )]))
    );
    assert_eq!(route.response.body, Some(Body::Json(json!({ "ok": true }))));
    assert_eq!(
      route.response.headers.get("Content-Type"),
      Some(&vec!["application/json".to_string()])
    );
  }

  #[test]
  fn a_message_route_is_todays_request_for_it_and_the_message() {
    let pact = json!({
      "consumer": { "name": "c" }, "provider": { "name": "p" },
      "messages": [{
        "description": "an order was shipped", "providerStates": [{ "name": "an order", "params": { "id": 1 } }],
        "contents": { "id": 1 },
        "metaData": { "contentType": "application/json", "topic": "orders" }
      }],
      "metadata": { "pactSpecification": { "version": "3.0.0" } }
    });
    let routes = routes_of(&pact, "test").unwrap();
    assert!(routes[0].message);
    let route = &routes[0].route;
    assert_eq!(
      (route.request.method.as_str(), route.request.path.as_str()),
      ("POST", "/")
    );
    assert_eq!(
      route.request.body,
      Some(Body::Json(json!({
        "description": "an order was shipped",
        "providerStates": [{ "name": "an order", "params": { "id": 1 } }]
      })))
    );
    assert_eq!(route.response.body, Some(Body::Json(json!({ "id": 1 }))));
    let metadata = &route.response.headers["Pact-Message-Metadata"][0];
    assert_eq!(
      serde_json::from_slice::<Value>(&BASE64.decode(metadata).unwrap()).unwrap(),
      json!({ "contentType": "application/json", "topic": "orders" })
    );
  }
}
