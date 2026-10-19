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
//! The HTTP client options in the broker descriptor (TLS, proxies, timeouts; staged plan S1.5) are
//! not read yet: the HTTP component's client half does not take them either.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use pact_janus_kernel::broker::{
  BrokerClient, BrokerDescriptor, ContractsPublished, ContractsToPublish, FetchRequest, FetchedPact,
  PublishRequest,
};
use pact_janus_kernel::component::ComponentError;
use serde_json::{Map, Value, json};
use std::time::Duration;

const HAL: &str = "application/hal+json, application/json";

/// How many times a request the broker answered with a 5xx, a 429 or nothing at all is tried in all.
/// Today's verifier retries more (eight, with doubling delays); three keeps a run that cannot reach
/// its broker from waiting minutes to say so. S1.5 makes it an option.
const ATTEMPTS: u32 = 3;

pub struct HalBrokerClient {
  agent: ureq::Agent,
}

impl Default for HalBrokerClient {
  fn default() -> Self {
    Self::new()
  }
}

impl HalBrokerClient {
  pub fn new() -> Self {
    HalBrokerClient {
      agent: ureq::Agent::config_builder()
        .http_status_as_error(false)
        .timeout_global(Some(Duration::from_secs(60)))
        .build()
        .into(),
    }
  }
}

impl BrokerClient for HalBrokerClient {
  fn fetch_for_verification(&self, request: &FetchRequest) -> Result<Vec<FetchedPact>, ComponentError> {
    let broker = &request.broker;
    let index = self.send(broker, "GET", &broker.url, None)?;
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
    let selected = self.send(broker, "POST", &url, Some(&Value::Object(body)))?;
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
      let document = self.send(broker, "GET", &absolute(broker, link), None)?;
      let properties = &pact["verificationProperties"];
      fetched.push(FetchedPact {
        handle: document["_links"].clone(),
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

  fn publish_results(&self, request: &PublishRequest<'_>) -> Result<(), ComponentError> {
    let broker = request.broker;
    let links = request.handle;
    if request.branch.is_some() || !request.tags.is_empty() {
      let provider_link = links["pb:provider"]["href"].as_str().ok_or_else(|| {
        invalid(
          broker,
          "the pact has no 'pb:provider' link to record the provider version on",
        )
      })?;
      let provider = self.send(broker, "GET", &absolute(broker, provider_link), None)?;
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
        self.send(broker, "PUT", &url, Some(&json!({})))?;
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
        self.send(broker, "PUT", &url, Some(&json!({})))?;
      }
    }
    let link = links["pb:publish-verification-results"]["href"]
      .as_str()
      .ok_or_else(|| invalid(broker, "the pact has no 'pb:publish-verification-results' link"))?;
    self.send(broker, "POST", &absolute(broker, link), Some(&payload(request)))?;
    Ok(())
  }

  fn publish_contracts(
    &self,
    request: &ContractsToPublish<'_>,
  ) -> Result<ContractsPublished, ComponentError> {
    let broker = request.broker;
    let index = self.exchange(broker, "GET", &broker.url, None, false)?;
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

    let answer = match self.exchange(broker, "POST", &url, Some(&body), index.accepts_gzip) {
      Err(error) if index.accepts_gzip && error.details.as_ref().is_some_and(|d| d["status"] == 415) => {
        tracing::debug!("the broker refused a compressed publication; sending it again uncompressed");
        self.exchange(broker, "POST", &url, Some(&body), false)?
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
    broker: &BrokerDescriptor,
    method: &str,
    url: &str,
    body: Option<&Value>,
  ) -> Result<Value, ComponentError> {
    self
      .exchange(broker, method, url, body, false)
      .map(|answer| answer.document)
  }

  /// [`HalBrokerClient::send`], with the body gzipped when `compress` says so, and what the
  /// broker's answer said about compression.
  fn exchange(
    &self,
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
    for attempt in 1..=ATTEMPTS {
      if attempt > 1 {
        std::thread::sleep(Duration::from_millis(500 * u64::from(attempt - 1)));
      }
      let mut request = ureq::http::Request::builder()
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
      let sent = match &payload {
        Some(payload) => request
          .body(payload.as_slice())
          .map(|request| self.agent.run(request)),
        None => request.body(()).map(|request| self.agent.run(request)),
      };
      let mut response = match sent {
        Err(err) => return Err(unreachable(broker, &format!("building {method} {shown}: {err}"))),
        Ok(Err(err)) => {
          tracing::debug!(%method, url = %shown, attempt, error = %err, "the broker did not answer");
          last = Some(unreachable(
            broker,
            &format!("{method} {shown}: {}", redacted(broker, &err.to_string())),
          ));
          continue;
        }
        Ok(Ok(response)) => response,
      };
      let status = response.status().as_u16();
      let accepts_gzip = response
        .headers()
        .get_all("accept-encoding")
        .iter()
        .filter_map(|value| value.to_str().ok())
        .any(names_gzip);
      let text = response.body_mut().read_to_string().unwrap_or_default();
      if status >= 500 || status == 429 {
        tracing::debug!(%method, url = %shown, attempt, status, "the broker is unavailable");
        last = Some(refused(broker, method, &shown, status, &text));
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

/// A link as an absolute URL: the broker writes absolute links, but a relative one is the broker's
/// own path.
fn absolute(broker: &BrokerDescriptor, link: &str) -> String {
  if link.contains("://") {
    link.to_string()
  } else {
    format!(
      "{}/{}",
      broker.url.trim_end_matches('/'),
      link.trim_start_matches('/')
    )
  }
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
    details: Some(details),
  }
}

fn unreachable(broker: &BrokerDescriptor, message: &str) -> ComponentError {
  error(
    broker,
    "broker-unreachable",
    format!("the Pact Broker could not be reached: {message}"),
    json!({}),
  )
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
