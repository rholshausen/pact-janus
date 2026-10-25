//! The Pact Broker client (staged plan S1.3, ADR 0032): [`BrokerClient`] over the broker's HAL
//! API, registered by the native embeddings, `janus` and `janus-engine`.
//!
//! It does what today's verifier (`pact_verifier`) does, in the same order, so that a provider
//! moving to Janus sees the same broker traffic it saw before:
//!
//! - **fetch**: the broker's index, its `pb:provider-pacts-for-verification` link for the
//!   provider, a `POST` of the selectors and the pending and work-in-progress options, then each
//!   selected pact by its `self` link;
//! - **publish**: the provider version's branch and tags through the pact's `pb:provider`, then
//!   the result to the pact's own `pb:publish-verification-results` link, one entry per
//!   interaction, keyed by the broker's interaction ids;
//! - **publish contracts**: the index's `pb:publish-contracts` link, with every contract in one
//!   `POST`, each as the broker's `pact` specification (ADR 0011 decision 6), as Pact-JVM and the
//!   Pact Broker client publish today.
//!
//! A publication is compressed only when the broker says it can read one so (RFC 7694: an
//! `Accept-Encoding` naming `gzip` on the index's response), and sent again uncompressed if it then
//! answers 415. No broker says so today: Pact Broker 2.121.2 reads a gzipped body as JSON and
//! refuses it (staged plan S1.3).
//!
//! Every failure is a value, the client's own error document (engine-protocol spec §10.2):
//! `broker-unreachable` when nothing answered, `broker-refused` when the broker said no, and
//! `broker-response-invalid` when it answered something this client cannot read. The broker is
//! named in them by its URL with user information removed, and no credential appears in any.
//!
//! Every link the broker writes is fetched from the broker the user named, as today's verifier
//! does: an absolute link keeps only its path and query ([`absolute`]).
//!
//! Every request goes through the engine's one HTTP client (ADR 0035), configured by the broker
//! descriptor's `client` member, the same HTTP client options the HTTP transport's drive role takes
//! (component-interfaces spec §5.6): TLS, proxies, timeouts and how many times to retry. With none,
//! the broker is reached as today's verifier reaches it: the platform's trust store and the
//! environment's proxies.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use pact_janus_http_client::{Defaults, HttpClient};
use pact_janus_kernel::broker::{
  BrokerClient, BrokerDescriptor, ContractsPublished, ContractsToPublish, FetchRequest, FetchedPact,
  PublishRequest,
};
use pact_janus_kernel::component::ComponentError;
use serde_json::{Map, Value, json};
use std::time::Duration;

const HAL: &str = "application/hal+json, application/json";

/// How many times a request is tried again, when nothing answered it, the broker answered a 5xx,
/// or it answered 429 and said when to try again, unless the descriptor's `client.retries` says.
/// Today's verifier retries more (eight, with doubling delays); three attempts keep a run that
/// cannot reach its broker from waiting minutes to say so.
const DEFAULT_RETRIES: u32 = 2;

/// How long one request to the broker may take, unless the descriptor's
/// `client.timeouts.request-ms` says.
const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

/// The longest `Retry-After` the client waits for. A broker that asks for longer is reported at
/// once, with what it asked, rather than holding a run for it.
const LONGEST_RETRY_AFTER: Duration = Duration::from_secs(10);

/// The client holds nothing between calls: every operation builds an HTTP client from its own
/// descriptor's options, since each names its broker and may configure it differently.
pub struct HalBrokerClient;

impl Default for HalBrokerClient {
  fn default() -> Self {
    Self::new()
  }
}

impl HalBrokerClient {
  pub fn new() -> Self {
    HalBrokerClient
  }
}

/// The HTTP client a descriptor's `client` options describe. Options it cannot act on are the
/// host's mistake, named by member: `invalid-config`, never a request that went wrong.
fn connect(broker: &BrokerDescriptor) -> Result<HttpClient, ComponentError> {
  HttpClient::from_value(
    broker.client.as_ref(),
    Defaults {
      request_timeout: DEFAULT_REQUEST_TIMEOUT,
      retries: DEFAULT_RETRIES,
      user_agent: format!("pact-janus/{}", pact_janus_kernel::ENGINE_VERSION),
    },
  )
  .map_err(|err| {
    let member = err
      .pointer
      .map(|pointer| format!(" /client{pointer}"))
      .unwrap_or_default();
    ComponentError::invalid_config(
      "broker",
      format!(
        "the broker descriptor's{member} {} (broker {})",
        err.message,
        broker.display_url()
      ),
    )
  })
}

impl BrokerClient for HalBrokerClient {
  fn fetch_for_verification(&self, request: &FetchRequest) -> Result<Vec<FetchedPact>, ComponentError> {
    let broker = &request.broker;
    let client = connect(broker)?;
    let index = self.send(&client, broker, "GET", &broker.url, None)?;
    let link = href(&index, "pb:provider-pacts-for-verification").ok_or_else(|| {
      invalid(
        broker,
        "the broker's index has no 'pb:provider-pacts-for-verification' link: it is older than the pacts-for-verification API",
      )
    })?;
    let url = expand(broker, &link, &[("provider", &request.provider)]);

    let mut body = Map::new();
    body.insert("consumerVersionSelectors".to_string(), json!(request.selectors));
    body.insert("includePendingStatus".to_string(), json!(request.include_pending));
    if !request.provider_tags.is_empty() {
      body.insert("providerVersionTags".to_string(), json!(request.provider_tags));
    }
    if let Some(branch) = &request.provider_branch {
      body.insert("providerVersionBranch".to_string(), json!(branch));
    }
    if let Some(since) = &request.include_wip_since {
      body.insert("includeWipPactsSince".to_string(), json!(since));
    }
    let selected = self.send(&client, broker, "POST", &url, Some(&Value::Object(body)))?;
    let pacts = selected["_embedded"]["pacts"].as_array().ok_or_else(|| {
      invalid(
        broker,
        "the pacts-for-verification answer has no '_embedded.pacts'",
      )
    })?;

    let mut fetched = Vec::with_capacity(pacts.len());
    for pact in pacts {
      let link = pact["_links"]["self"]["href"]
        .as_str()
        .ok_or_else(|| invalid(broker, "a selected pact has no 'self' link"))?;
      let document = self.send(&client, broker, "GET", &absolute(broker, link), None)?;
      let properties = &pact["verificationProperties"];
      fetched.push(FetchedPact {
        handle: document["_links"].clone(),
        publishable: true,
        document,
        pending: properties["pending"].as_bool().unwrap_or(false),
        wip: properties["wip"].as_bool().unwrap_or(false),
        notices: properties["notices"]
          .as_array()
          .into_iter()
          .flatten()
          .filter_map(|notice| notice["text"].as_str().map(str::to_string))
          .collect(),
      });
    }
    Ok(fetched)
  }

  /// Today's `--url` and `--webhook-callback-url` (ADR 0053): the pact at `url`, fetched with the
  /// run's credentials. A pact a broker served carries its links, and its result can be published
  /// to them; one from anywhere else cannot.
  fn fetch_pact(&self, broker: &BrokerDescriptor, url: &str) -> Result<FetchedPact, ComponentError> {
    let client = connect(broker)?;
    let document = self.send(&client, broker, "GET", url, None)?;
    let handle = document["_links"].clone();
    Ok(FetchedPact {
      publishable: href(&document, "pb:publish-verification-results").is_some(),
      handle,
      document,
      pending: false,
      wip: false,
      notices: vec![],
    })
  }

  fn publish_results(&self, request: &PublishRequest<'_>) -> Result<(), ComponentError> {
    let broker = request.broker;
    let client = connect(broker)?;
    let links = request.handle;
    if request.branch.is_some() || !request.tags.is_empty() {
      let provider_link = links["pb:provider"]["href"].as_str().ok_or_else(|| {
        invalid(
          broker,
          "the pact has no 'pb:provider' link to record the provider version on",
        )
      })?;
      let provider = self.send(&client, broker, "GET", &absolute(broker, provider_link), None)?;
      if let Some(branch) = request.branch {
        let link = href(&provider, "pb:branch-version").ok_or_else(|| {
          invalid(
            broker,
            "the provider has no 'pb:branch-version' link: branches need Pact Broker 2.86.0 or later",
          )
        })?;
        let url = expand(
          broker,
          &link,
          &[("branch", branch), ("version", request.provider_version)],
        );
        self.send(&client, broker, "PUT", &url, Some(&json!({})))?;
      }
      for tag in request.tags {
        let link = href(&provider, "pb:version-tag").ok_or_else(|| {
          invalid(
            broker,
            "the provider has no 'pb:version-tag' link to tag the provider version with",
          )
        })?;
        let url = expand(
          broker,
          &link,
          &[("version", request.provider_version), ("tag", tag)],
        );
        self.send(&client, broker, "PUT", &url, Some(&json!({})))?;
      }
    }
    let link = links["pb:publish-verification-results"]["href"]
      .as_str()
      .ok_or_else(|| invalid(broker, "the pact has no 'pb:publish-verification-results' link"))?;
    self.send(
      &client,
      broker,
      "POST",
      &absolute(broker, link),
      Some(&payload(request)),
    )?;
    Ok(())
  }

  fn publish_contracts(
    &self,
    request: &ContractsToPublish<'_>,
  ) -> Result<ContractsPublished, ComponentError> {
    let broker = request.broker;
    let client = connect(broker)?;
    let index = self.exchange(&client, broker, "GET", &broker.url, None, false)?;
    let link = href(&index.document, "pb:publish-contracts").ok_or_else(|| {
      invalid(
        broker,
        "the broker's index has no 'pb:publish-contracts' link: publishing contracts needs Pact Broker 2.86.0 or later",
      )
    })?;
    let url = absolute(broker, &link);

    let mut body = Map::new();
    body.insert("pacticipantName".to_string(), json!(request.consumer));
    body.insert(
      "pacticipantVersionNumber".to_string(),
      json!(request.consumer_version),
    );
    if let Some(branch) = request.branch {
      body.insert("branch".to_string(), json!(branch));
    }
    if !request.tags.is_empty() {
      body.insert("tags".to_string(), json!(request.tags));
    }
    if let Some(build_url) = request.build_url {
      body.insert("buildUrl".to_string(), json!(build_url));
    }
    let contracts: Vec<Value> = request
      .contracts
      .iter()
      .map(|contract| {
        json!({
          "consumerName": request.consumer,
          "providerName": contract.provider,
          "specification": "pact",
          "contentType": "application/json",
          "content": BASE64.encode(&contract.content),
        })
      })
      .collect();
    body.insert("contracts".to_string(), Value::Array(contracts));
    let body = Value::Object(body);

    let answer = match self.exchange(&client, broker, "POST", &url, Some(&body), index.accepts_gzip) {
      Err(error) if index.accepts_gzip && error.details.as_ref().is_some_and(|d| d["status"] == 415) => {
        tracing::debug!("the broker refused a compressed publication; sending it again uncompressed");
        self.exchange(&client, broker, "POST", &url, Some(&body), false)?
      }
      answer => answer?,
    };
    Ok(ContractsPublished {
      notices: answer.document["notices"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|notice| notice["text"].as_str().map(str::to_string))
        .collect(),
    })
  }
}

/// A broker's answer: the document, and whether it said it reads gzipped request bodies.
struct Answer {
  document: Value,
  accepts_gzip: bool,
}

/// A `Retry-After` in delta-seconds (RFC 9110 §10.2.3). The HTTP-date form is not read: the
/// client's own backoff stands in for it.
fn parse_retry_after(value: &str) -> Option<Duration> {
  value.trim().parse::<u64>().ok().map(Duration::from_secs)
}

/// Whether an `Accept-Encoding` value names gzip with a weight above zero (RFC 7694 §3).
fn names_gzip(value: &str) -> bool {
  value.split(',').any(|coding| {
    let mut parts = coding.split(';');
    let name = parts.next().unwrap_or_default().trim();
    let refused = parts.any(|parameter| {
      parameter
        .trim()
        .strip_prefix("q=")
        .and_then(|weight| weight.trim().parse::<f32>().ok())
        .is_some_and(|weight| weight == 0.0)
    });
    (name.eq_ignore_ascii_case("gzip") || name.eq_ignore_ascii_case("x-gzip")) && !refused
  })
}

fn gzip(bytes: &[u8]) -> Vec<u8> {
  use flate2::Compression;
  use flate2::write::GzEncoder;
  use std::io::Write;
  let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
  // Writing to a Vec cannot fail.
  let _ = encoder.write_all(bytes);
  encoder.finish().unwrap_or_default()
}

/// The verification result the broker reads (its `VerificationContract`): the outcome, who verified
/// it, and one entry per interaction.
fn payload(request: &PublishRequest<'_>) -> Value {
  let results: Vec<Value> = request
    .results
    .iter()
    .map(|result| {
      let mut entry = Map::new();
      if let Some(id) = &result.id {
        entry.insert("interactionId".to_string(), json!(id));
      }
      entry.insert("interactionDescription".to_string(), json!(result.description));
      entry.insert("success".to_string(), json!(result.success));
      if !result.mismatches.is_empty() {
        entry.insert(
          "mismatches".to_string(),
          Value::Array(result.mismatches.iter().map(broker_mismatch).collect()),
        );
      }
      if let Some(error) = &result.error {
        let message = error["message"]
          .as_str()
          .or(error["code"].as_str())
          .map(str::to_string)
          .unwrap_or_else(|| error.to_string());
        entry.insert("exceptions".to_string(), json!([{ "message": message }]));
      }
      Value::Object(entry)
    })
    .collect();
  let mut payload = json!({
    "success": request.success,
    "providerApplicationVersion": request.provider_version,
    "verifiedBy": { "implementation": "Pact Janus", "version": pact_janus_kernel::ENGINE_VERSION },
    "testResults": results,
  });
  if let Some(build_url) = request.build_url {
    payload["buildUrl"] = json!(build_url);
  }
  payload
}

/// An engine mismatch (`{ path, message }`) in the broker's terms: which part of the response
/// (`attribute`), where in it (`identifier`), and what was wrong (`description`).
fn broker_mismatch(mismatch: &Value) -> Value {
  let path = mismatch["path"].as_str().unwrap_or_default();
  let description = mismatch["message"].as_str().unwrap_or_default();
  let part = path.strip_prefix("$.response").unwrap_or(path);
  if part == ".status" {
    json!({ "attribute": "status", "description": description })
  } else if let Some(header) = part.strip_prefix(".headers") {
    let name = header
      .trim_start_matches('.')
      .trim_start_matches("['")
      .trim_end_matches("']");
    json!({ "attribute": "header", "identifier": name, "description": description })
  } else {
    let within = part.strip_prefix(".body").unwrap_or(part);
    json!({ "attribute": "body", "identifier": format!("${within}"), "description": description })
  }
}

impl HalBrokerClient {
  /// One request, retried while the broker is unavailable, answered as a JSON document or the
  /// client's own error.
  fn send(
    &self,
    client: &HttpClient,
    broker: &BrokerDescriptor,
    method: &str,
    url: &str,
    body: Option<&Value>,
  ) -> Result<Value, ComponentError> {
    self
      .exchange(client, broker, method, url, body, false)
      .map(|answer| answer.document)
  }

  /// [`HalBrokerClient::send`], with the body gzipped when `compress` says so, and what the
  /// broker's answer said about compression.
  fn exchange(
    &self,
    client: &HttpClient,
    broker: &BrokerDescriptor,
    method: &str,
    url: &str,
    body: Option<&Value>,
    compress: bool,
  ) -> Result<Answer, ComponentError> {
    let shown = pact_janus_kernel::broker::without_userinfo(url);
    let payload = body.map(|body| {
      let bytes = serde_json::to_vec(body).unwrap_or_default();
      if compress { gzip(&bytes) } else { bytes }
    });
    let mut last = None;
    let mut wait = None;
    for attempt in 1..=client.retries().saturating_add(1) {
      if let Some(wait) = wait.take() {
        std::thread::sleep(wait);
      }
      let backoff = Duration::from_millis(500 * u64::from(attempt));
      let mut request = http::Request::builder()
        .method(method)
        .uri(url)
        .header("accept", HAL)
        .header(
          "user-agent",
          format!("pact-janus/{}", pact_janus_kernel::ENGINE_VERSION),
        );
      if payload.is_some() {
        request = request.header("content-type", "application/json");
        if compress {
          request = request.header("content-encoding", "gzip");
        }
      }
      if let Some(authorization) = authorization(broker) {
        request = request.header("authorization", authorization);
      }
      // The HTTP library hides a URL's path below `trace`; the broker's are HAL links with no
      // secret in them, so they are logged in full, without user information.
      tracing::debug!(%method, url = %shown, attempt, "sending the request to the broker");
      let sent = request
        .body(payload.clone().unwrap_or_default())
        .map(|request| client.run(request, None));
      let response = match sent {
        Err(err) => return Err(unreachable(broker, &format!("building {method} {shown}: {err}"))),
        Ok(Err(err)) => {
          tracing::debug!(%method, url = %shown, attempt, error = %err, "the broker did not answer");
          last = Some(unreachable(
            broker,
            &format!("{method} {shown}: {}", redacted(broker, &err.to_string())),
          ));
          wait = Some(backoff);
          continue;
        }
        Ok(Ok(response)) => response,
      };
      let status = response.status().as_u16();
      tracing::debug!(%method, url = %shown, attempt, status, "the broker answered");
      let accepts_gzip = response
        .headers()
        .get_all("accept-encoding")
        .iter()
        .filter_map(|value| value.to_str().ok())
        .any(names_gzip);
      let retry_after = response
        .headers()
        .get("retry-after")
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
      let text = String::from_utf8_lossy(response.body()).to_string();
      // A 429 is retried only when the broker says when to: PactFlow answers 429 for a plan's limit
      // (a new integration past the maximum), which no wait changes. A 5xx is retried with the
      // client's own backoff, unless the broker asked for a delay of its own.
      if status >= 500 || (status == 429 && retry_after.is_some()) {
        let delay = match retry_after.as_deref().map(parse_retry_after) {
          None | Some(None) => backoff,
          Some(Some(delay)) if delay <= LONGEST_RETRY_AFTER => delay,
          Some(Some(delay)) => {
            let mut error = refused(broker, method, &shown, status, &text);
            error.message.push_str(&format!(
              " (it asked to be tried again in {}s, longer than this client waits)",
              delay.as_secs()
            ));
            if let Some(details) = error.details.as_mut() {
              details["retry-after"] = json!(delay.as_secs());
            }
            return Err(error);
          }
        };
        tracing::debug!(%method, url = %shown, attempt, status, ?delay, "the broker is unavailable");
        last = Some(refused(broker, method, &shown, status, &text));
        wait = Some(delay);
        continue;
      }
      if !(200..300).contains(&status) {
        return Err(refused(broker, method, &shown, status, &text));
      }
      if text.trim().is_empty() {
        return Ok(Answer {
          document: Value::Null,
          accepts_gzip,
        });
      }
      return serde_json::from_str(&text)
        .map(|document| Answer {
          document,
          accepts_gzip,
        })
        .map_err(|err| {
          invalid(
            broker,
            &format!("{method} {shown} answered something that is not JSON: {err}"),
          )
        });
    }
    Err(last.unwrap_or_else(|| unreachable(broker, &format!("{method} {shown}"))))
  }
}

fn authorization(broker: &BrokerDescriptor) -> Option<String> {
  let auth = broker.auth.as_ref()?;
  match auth.kind.as_str() {
    "bearer" => auth.token.as_ref().map(|token| format!("Bearer {token}")),
    "basic" => {
      let credentials = format!(
        "{}:{}",
        auth.username.as_deref().unwrap_or_default(),
        auth.password.as_deref().unwrap_or_default()
      );
      Some(format!("Basic {}", BASE64.encode(credentials)))
    }
    other => {
      tracing::warn!(kind = %other, "an unknown broker auth kind; the broker is asked without credentials");
      None
    }
  }
}

/// `text` with every credential the descriptor carries taken out, for an error that might repeat
/// what an HTTP library said about a URL.
fn redacted(broker: &BrokerDescriptor, text: &str) -> String {
  let mut text = text.replace(&broker.url, &broker.display_url());
  if let Some(auth) = &broker.auth {
    for secret in [&auth.token, &auth.password].into_iter().flatten() {
      if !secret.is_empty() {
        text = text.replace(secret.as_str(), "***");
      }
    }
  }
  text
}

/// A link's `href` in a HAL document.
fn href(document: &Value, relation: &str) -> Option<String> {
  document["_links"][relation]["href"].as_str().map(str::to_string)
}

/// A templated link with its values filled in, each a percent-encoded path segment.
fn expand(broker: &BrokerDescriptor, link: &str, values: &[(&str, &str)]) -> String {
  let mut url = link.to_string();
  for (name, value) in values {
    url = url.replace(&format!("{{{name}}}"), &encode(value));
  }
  absolute(broker, &url)
}

/// Where a link the broker wrote is fetched from: always the broker the user named, as today's
/// verifier does. Of an absolute link only the path and query are kept, because a broker behind a
/// proxy often writes links with a host its clients cannot reach. When the broker's URL has a context
/// path (`https://example.com/broker`) and a link's path does not start with it, the context path is
/// put in front of it; a relative link is relative to the context path.
fn absolute(broker: &BrokerDescriptor, link: &str) -> String {
  let (origin, context) = split_origin(&broker.url);
  let context = context
    .split(['?', '#'])
    .next()
    .unwrap_or_default()
    .trim_end_matches('/');
  let path = match link.split_once("://") {
    Some(_) => {
      let (_, path) = split_origin(link);
      if path.is_empty() { "/" } else { path }
    }
    None => link,
  };
  if !path.starts_with('/') {
    return format!("{origin}{context}/{path}");
  }
  let within = path == context
    || path
      .strip_prefix(context)
      .is_some_and(|rest| rest.starts_with(['/', '?', '#']));
  if context.is_empty() || within {
    format!("{origin}{path}")
  } else {
    format!("{origin}{context}{path}")
  }
}

/// `scheme://authority` and the rest (`/path?query`, or empty) of an absolute URL. A URL with no
/// scheme is all rest.
fn split_origin(url: &str) -> (&str, &str) {
  let Some((scheme, rest)) = url.split_once("://") else {
    return ("", url);
  };
  let end = scheme.len() + 3 + rest.find(['/', '?', '#']).unwrap_or(rest.len());
  url.split_at(end)
}

fn encode(segment: &str) -> String {
  let mut out = String::new();
  for byte in segment.bytes() {
    if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
      out.push(byte as char);
    } else {
      out.push_str(&format!("%{byte:02X}"));
    }
  }
  out
}

fn error(broker: &BrokerDescriptor, code: &str, message: String, details: Value) -> ComponentError {
  let mut details = details;
  details["broker"] = json!(broker.display_url());
  ComponentError {
    code: code.to_string(),
    category: "component".to_string(),
    message,
    source: None,
    details: Some(Box::new(details)),
  }
}

/// The code says the broker could not be reached; the message says which request, and why. A pact
/// fetched by its URL comes through here too, from wherever it is served.
fn unreachable(broker: &BrokerDescriptor, message: &str) -> ComponentError {
  error(broker, "broker-unreachable", message.to_string(), json!({}))
}

fn refused(broker: &BrokerDescriptor, method: &str, url: &str, status: u16, body: &str) -> ComponentError {
  // The broker's own explanation, when it gave one, is the most useful thing to show.
  let said: Option<String> = serde_json::from_str::<Value>(body).ok().and_then(|value| {
    errors(&value["errors"])
      .or_else(|| value["error"]["message"].as_str().map(str::to_string))
      .or_else(|| value["error"].as_str().map(str::to_string))
      .or_else(|| value["title"].as_str().map(str::to_string))
  });
  let message = match &said {
    Some(said) => format!(
      "the Pact Broker answered {method} {url} with {status}: {}",
      redacted(broker, said)
    ),
    None => format!("the Pact Broker answered {method} {url} with {status}"),
  };
  error(broker, "broker-refused", message, json!({ "status": status }))
}

/// The broker's `errors`, as one line: a list of messages, or, from a request it would not accept,
/// messages by the member they are about (`{"contracts": ["..."]}`).
fn errors(errors: &Value) -> Option<String> {
  let text = |value: &Value| value.as_str().map_or_else(|| value.to_string(), str::to_string);
  let said: Vec<String> = match errors {
    Value::Array(messages) => messages.iter().map(text).collect(),
    Value::Object(members) => members
      .iter()
      .flat_map(|(member, messages)| match messages {
        Value::Array(messages) => messages
          .iter()
          .map(|message| format!("{member}: {}", text(message)))
          .collect::<Vec<_>>(),
        message => vec![format!("{member}: {}", text(message))],
      })
      .collect(),
    _ => return None,
  };
  (!said.is_empty()).then(|| said.join("; "))
}

fn invalid(broker: &BrokerDescriptor, message: &str) -> ComponentError {
  error(broker, "broker-response-invalid", message.to_string(), json!({}))
}

#[cfg(test)]
mod tests;
