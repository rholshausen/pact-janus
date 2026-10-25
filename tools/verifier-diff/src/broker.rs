//! The Pact Broker a broker fixture runs against (S1.3): seeding it, reading back what a run
//! published, and recording what each verifier asked of it.
//!
//! The broker is a real one, the image `tools/verifier-diff/pact-broker.json` pins, started outside
//! the harness (`.github/scripts/pact-broker.sh`) and named with `--broker`. It is shared by every
//! case, so each verifier's pacts are published under names of their own — a prefix on every
//! pacticipant — and nothing one verifier publishes changes what the other fetches.
//!
//! Each verifier talks to the broker through a [`RecordingProxy`], which forwards every request and
//! keeps its method, path, body and status. The broker writes absolute links from the host it was
//! asked on; the proxy says it is that host (`X-Forwarded-Host`), so a verifier that follows the
//! links stays on the proxy.

use crate::fixture::{BrokerSpec, Publication, Published};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

const HAL: &str = "application/hal+json";

pub struct Broker {
  base_url: String,
  agent: ureq::Agent,
}

/// A pact as one verifier sees it: its name in the fixture, and the consumer and provider it was
/// published under.
#[derive(Debug, Clone)]
pub struct PublishedPact {
  pub file: String,
  pub consumer: String,
  pub provider: String,
  pub version: String,
}

impl Broker {
  pub fn new(base_url: &str) -> Broker {
    Broker {
      base_url: base_url.trim_end_matches('/').to_string(),
      agent: ureq::Agent::config_builder()
        .http_status_as_error(false)
        .timeout_global(Some(Duration::from_secs(30)))
        .build()
        .into(),
    }
  }

  pub fn base_url(&self) -> &str {
    &self.base_url
  }

  /// The broker's version, from the header it puts on every response.
  pub fn version(&self) -> Result<String, String> {
    let response = self.send("GET", "/", None)?;
    response.0.clone().ok_or_else(|| {
      format!(
        "{} did not say its version (X-Pact-Broker-Version)",
        self.base_url
      )
    })
  }

  /// Publish every pact as `spec` says, and every result it says was there before the run.
  /// `pacts` are the documents with their names already prefixed.
  pub fn seed(&self, spec: &BrokerSpec, pacts: &[(String, Value)]) -> Result<Vec<PublishedPact>, String> {
    let mut published = Vec::new();
    for (file, document) in pacts {
      let publication = spec.publish.get(file).cloned().unwrap_or_default();
      let pact = PublishedPact {
        file: file.clone(),
        consumer: name_of(document, "consumer")?,
        provider: name_of(document, "provider")?,
        version: publication.version.clone(),
      };
      self.publish_contract(&pact, &publication, document)?;
      published.push(pact);
    }
    for result in &spec.results {
      let pact = published
        .iter()
        .find(|pact| pact.file == result.pact)
        .ok_or_else(|| {
          format!(
            "a prior result names '{}', which the fixture does not publish",
            result.pact
          )
        })?;
      let encoded = |text: &str| encode(text);
      if let Some(branch) = &result.provider_branch {
        self.expect_ok(
          "PUT",
          &format!(
            "/pacticipants/{}/branches/{}/versions/{}",
            encoded(&pact.provider),
            encoded(branch),
            encoded(&result.provider_version)
          ),
          Some(json!({})),
        )?;
      }
      let link = self.publish_link(pact)?;
      let (_, status, body) = self.send_url(
        "POST",
        &link,
        Some(json!({ "success": result.success, "providerApplicationVersion": result.provider_version })),
      )?;
      if !(200..300).contains(&status) {
        return Err(format!(
          "publishing a prior result for {}: {status} {body}",
          pact.file
        ));
      }
    }
    Ok(published)
  }

  fn publish_contract(
    &self,
    pact: &PublishedPact,
    publication: &Publication,
    document: &Value,
  ) -> Result<(), String> {
    let content = serde_json::to_vec(document).map_err(|err| err.to_string())?;
    let mut body = json!({
      "pacticipantName": pact.consumer,
      "pacticipantVersionNumber": publication.version,
      "tags": publication.tags,
      "contracts": [{
        "consumerName": pact.consumer,
        "providerName": pact.provider,
        "specification": "pact",
        "contentType": "application/json",
        "content": BASE64.encode(content),
      }],
    });
    if let Some(branch) = &publication.branch {
      body["branch"] = json!(branch);
    }
    self
      .expect_ok("POST", "/contracts/publish", Some(body))
      .map(|_| ())
  }

  /// The pact version's `pb:publish-verification-results` link.
  fn publish_link(&self, pact: &PublishedPact) -> Result<String, String> {
    let document = self.expect_ok("GET", &pact_path(pact), None)?;
    document["_links"]["pb:publish-verification-results"]["href"]
      .as_str()
      .map(str::to_string)
      .ok_or_else(|| format!("the broker gave {} no link to publish results to", pact.file))
  }

  /// What the run published for this pact: the latest result, if a provider at `provider_version`
  /// published it. A result from an earlier provider version was seeded, not published by the run.
  pub fn published(&self, pact: &PublishedPact, provider_version: &str) -> Result<Published, String> {
    let (_, status, body) = self.send(
      "GET",
      &format!("{}/verification-results/latest", pact_path(pact)),
      None,
    )?;
    if status == 404 {
      return Ok(Published::Absent);
    }
    if !(200..300).contains(&status) {
      return Err(format!("reading the result for {}: {status} {body}", pact.file));
    }
    if body["providerApplicationVersion"].as_str() != Some(provider_version) {
      return Ok(Published::Absent);
    }
    Ok(match body["success"].as_bool() {
      Some(true) => Published::Pass,
      _ => Published::Fail,
    })
  }

  fn expect_ok(&self, method: &str, path: &str, body: Option<Value>) -> Result<Value, String> {
    let (_, status, response) = self.send(method, path, body)?;
    if (200..300).contains(&status) {
      Ok(response)
    } else {
      Err(format!("{method} {path}: {status} {response}"))
    }
  }

  fn send(
    &self,
    method: &str,
    path: &str,
    body: Option<Value>,
  ) -> Result<(Option<String>, u16, Value), String> {
    self.send_url(method, &format!("{}{path}", self.base_url), body)
  }

  fn send_url(
    &self,
    method: &str,
    url: &str,
    body: Option<Value>,
  ) -> Result<(Option<String>, u16, Value), String> {
    let request = ureq::http::Request::builder()
      .method(method)
      .uri(url)
      .header("accept", HAL)
      .header("content-type", "application/json");
    let payload = body.map(|body| serde_json::to_vec(&body).unwrap_or_default());
    let sent = match &payload {
      Some(payload) => request
        .body(payload.as_slice())
        .map(|request| self.agent.run(request)),
      None => request.body(()).map(|request| self.agent.run(request)),
    };
    let mut response = sent
      .map_err(|err| format!("building {method} {url}: {err}"))?
      .map_err(|err| {
        format!("{method} {url}: {err} (is the broker running? .github/scripts/pact-broker.sh start)")
      })?;
    let version = response
      .headers()
      .get("x-pact-broker-version")
      .and_then(|value| value.to_str().ok())
      .map(str::to_string);
    let status = response.status().as_u16();
    let text = response.body_mut().read_to_string().unwrap_or_default();
    let value = serde_json::from_str(&text).unwrap_or(Value::String(text));
    Ok((version, status, value))
  }
}

pub fn pact_path(pact: &PublishedPact) -> String {
  format!(
    "/pacts/provider/{}/consumer/{}/version/{}",
    encode(&pact.provider),
    encode(&pact.consumer),
    encode(&pact.version)
  )
}

fn name_of(document: &Value, role: &str) -> Result<String, String> {
  document[role]["name"]
    .as_str()
    .map(str::to_string)
    .ok_or_else(|| format!("the pact names no {role}"))
}

/// A path segment, percent-encoded: everything but the unreserved characters.
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

/// One request a verifier made of the broker.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Exchange {
  pub method: String,
  pub path: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub body: Option<Value>,
  pub status: u16,
}

/// A reverse proxy in front of the broker that keeps every exchange, until dropped.
pub struct RecordingProxy {
  server: Arc<tiny_http::Server>,
  stop: Arc<AtomicBool>,
  thread: Option<JoinHandle<()>>,
  port: u16,
  exchanges: Arc<Mutex<Vec<Exchange>>>,
}

impl RecordingProxy {
  pub fn start(broker: &str) -> Result<RecordingProxy, String> {
    let server = Arc::new(tiny_http::Server::http("127.0.0.1:0").map_err(|err| err.to_string())?);
    let port = server
      .server_addr()
      .to_ip()
      .map(|addr| addr.port())
      .ok_or("the recording proxy has no IP address")?;
    let stop = Arc::new(AtomicBool::new(false));
    let exchanges = Arc::new(Mutex::new(Vec::new()));
    let thread = {
      let server = Arc::clone(&server);
      let stop = Arc::clone(&stop);
      let exchanges = Arc::clone(&exchanges);
      let broker = broker.trim_end_matches('/').to_string();
      let agent: ureq::Agent = ureq::Agent::config_builder()
        .http_status_as_error(false)
        .timeout_global(Some(Duration::from_secs(30)))
        .build()
        .into();
      std::thread::spawn(move || {
        for request in server.incoming_requests() {
          if stop.load(Ordering::Relaxed) {
            break;
          }
          forward(request, &agent, &broker, port, &exchanges);
        }
      })
    };
    Ok(RecordingProxy {
      server,
      stop,
      thread: Some(thread),
      port,
      exchanges,
    })
  }

  pub fn base_url(&self) -> String {
    format!("http://127.0.0.1:{}", self.port)
  }

  pub fn exchanges(&self) -> Vec<Exchange> {
    self.exchanges.lock().expect("exchanges").clone()
  }
}

impl Drop for RecordingProxy {
  fn drop(&mut self) {
    self.stop.store(true, Ordering::Relaxed);
    self.server.unblock();
    if let Some(thread) = self.thread.take() {
      let _ = thread.join();
    }
  }
}

/// The request headers passed on to the broker. Everything else — the connection's own, the
/// client's `Host` — is the proxy's business.
const FORWARDED: &[&str] = &["accept", "content-type", "authorization", "user-agent"];

fn forward(
  mut request: tiny_http::Request,
  agent: &ureq::Agent,
  broker: &str,
  port: u16,
  exchanges: &Mutex<Vec<Exchange>>,
) {
  let mut body = Vec::new();
  let _ = request.as_reader().read_to_end(&mut body);
  let method = request.method().as_str().to_uppercase();
  let path = request.url().to_string();
  let mut outgoing = ureq::http::Request::builder()
    .method(method.as_str())
    .uri(format!("{broker}{path}"))
    .header("x-forwarded-host", format!("127.0.0.1:{port}"))
    .header("x-forwarded-proto", "http");
  for header in request.headers() {
    let name = header.field.as_str().as_str().to_ascii_lowercase();
    if FORWARDED.contains(&name.as_str()) {
      outgoing = outgoing.header(name, header.value.as_str());
    }
  }
  let answered = outgoing
    .body(body.as_slice())
    .map_err(|err| err.to_string())
    .and_then(|outgoing| agent.run(outgoing).map_err(|err| err.to_string()));
  let (status, content_type, bytes) = match answered {
    Ok(mut response) => {
      let content_type = response
        .headers()
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
      let bytes = response.body_mut().read_to_vec().unwrap_or_default();
      (response.status().as_u16(), content_type, bytes)
    }
    Err(err) => (
      502,
      Some("text/plain".to_string()),
      format!("the recording proxy could not reach the broker: {err}").into_bytes(),
    ),
  };
  exchanges.lock().expect("exchanges").push(Exchange {
    method,
    path,
    body: (!body.is_empty()).then(|| {
      serde_json::from_slice(&body)
        .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&body).into_owned()))
    }),
    status,
  });
  let mut response = tiny_http::Response::from_data(bytes).with_status_code(status);
  if let Some(content_type) = content_type
    && let Ok(header) = tiny_http::Header::from_bytes("Content-Type", content_type)
  {
    response.add_header(header);
  }
  let _ = request.respond(response);
}

/// Exchanges as a reader compares them: the verifier's own pacticipant prefix and the proxy's
/// address replaced by placeholders, so the two verifiers' recordings line up.
pub fn normalised(exchanges: Vec<Exchange>, prefix: &str, proxy: &str) -> Vec<Exchange> {
  let replace = |text: &str| text.replace(proxy, "{broker}").replace(prefix, "{prefix}");
  exchanges
    .into_iter()
    .map(|exchange| Exchange {
      path: replace(&exchange.path),
      body: exchange.body.map(|body| {
        let text = replace(&body.to_string());
        serde_json::from_str(&text).unwrap_or(Value::String(text))
      }),
      ..exchange
    })
    .collect()
}

/// What both verifiers asked of the broker in one case.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Traffic {
  pub today: Vec<Exchange>,
  pub janus: Vec<Exchange>,
}

/// The broker the harness was told about, checked against the pinned version.
pub fn pinned_version(repo: &std::path::Path) -> Result<String, String> {
  let path = repo.join("tools").join("verifier-diff").join("pact-broker.json");
  let text = std::fs::read_to_string(&path).map_err(|err| format!("{}: {err}", path.display()))?;
  let pin: Value = serde_json::from_str(&text).map_err(|err| format!("{}: {err}", path.display()))?;
  pin["broker-version"]
    .as_str()
    .map(str::to_string)
    .ok_or_else(|| format!("{}: no broker-version", path.display()))
}

#[cfg(test)]
mod tests {
  use super::*;
  use pretty_assertions::assert_eq;

  #[test]
  fn a_path_segment_is_percent_encoded() {
    assert_eq!(encode("Foo Consumer/1"), "Foo%20Consumer%2F1");
    assert_eq!(encode("c_1-v2.0~x"), "c_1-v2.0~x");
  }

  #[test]
  fn a_recording_is_normalised_so_two_verifiers_line_up() {
    let recorded = vec![Exchange {
      method: "POST".to_string(),
      path: "/pacts/provider/vd1-today-p/for-verification".to_string(),
      body: Some(json!({ "href": "http://127.0.0.1:4000/pacts/provider/vd1-today-p" })),
      status: 200,
    }];
    let normalised = normalised(recorded, "vd1-today-", "http://127.0.0.1:4000");
    assert_eq!(normalised[0].path, "/pacts/provider/{prefix}p/for-verification");
    assert_eq!(
      normalised[0].body,
      Some(json!({ "href": "{broker}/pacts/provider/{prefix}p" }))
    );
  }
}
