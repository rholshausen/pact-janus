//! A stand-in for a Pact Broker with the `janus` feature (ADR 0093 decision 3): what the broker PR
//! series' rows 2, 4 and 6 serve (staged plan S4.3c, S4.3d), as this client expects them, so the
//! client and the CLI can be tested before a broker built from the series can be run.
//!
//! It is a stand-in, not a broker. It stores what it is sent in memory and serves it back by the
//! links its index advertises, and it records every request. With the feature off it advertises
//! none of the series' links, serves none of its routes and refuses `specification: "janus"` as both
//! brokers do today, so a test can show a client asking a broker that has not caught up. What it
//! keeps of a report is what was sent; it computes no SHA, and decides nothing.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};

/// What the stand-in holds: everything published to it, and every request it was sent.
#[derive(Default)]
pub struct Held {
  /// Each request: its method and path, query included.
  pub requests: Vec<String>,
  /// Each contract a `POST /contracts/publish` carried: `consumer`, `provider`, `version`,
  /// `specification` and `content`, decoded.
  pub contracts: Vec<Value>,
  /// Each shape, in the order published: `provider`, `version`, `branch`, `tags`, `buildUrl` and
  /// `content`, decoded.
  pub shapes: Vec<Value>,
  /// Each report: `shape`, the index into [`Held::shapes`] of the shape its link was on, and the
  /// body as sent, its contract decoded into `contract.decoded`.
  pub reports: Vec<Value>,
}

/// A running stand-in. It serves until the test process ends.
pub struct BrokerStandIn {
  pub url: String,
  held: Arc<Mutex<Held>>,
}

impl BrokerStandIn {
  /// A stand-in with the `janus` feature on, or off.
  pub fn start(janus: bool) -> BrokerStandIn {
    let server = tiny_http::Server::http("127.0.0.1:0").expect("the stand-in binds");
    let url = format!("http://{}", server.server_addr().to_ip().expect("an IP address"));
    let held: Arc<Mutex<Held>> = Arc::default();
    let state = Arc::clone(&held);
    let me = url.clone();
    std::thread::spawn(move || {
      for mut request in server.incoming_requests() {
        let mut body = String::new();
        let _ = request.as_reader().read_to_string(&mut body);
        let method = request.method().as_str().to_string();
        let path = request.url().to_string();
        let (status, answer) = {
          let mut held = state.lock().expect("the stand-in's state");
          held.requests.push(format!("{method} {path}"));
          let body = serde_json::from_str(&body).unwrap_or(Value::Null);
          answer(&mut held, janus, &me, &method, &path, &body)
        };
        let _ = request.respond(
          tiny_http::Response::from_string(answer.to_string())
            .with_status_code(status)
            .with_header(
              tiny_http::Header::from_bytes("Content-Type", "application/hal+json").expect("a header"),
            ),
        );
      }
    });
    BrokerStandIn { url, held }
  }

  /// What it holds now.
  pub fn held(&self) -> std::sync::MutexGuard<'_, Held> {
    self.held.lock().expect("the stand-in's state")
  }

  /// Hold `shape` for `provider` at `version`, on `branch`, as if it had been published.
  pub fn holds_shape(&self, provider: &str, version: &str, branch: Option<&str>, shape: &Value) {
    self.held().shapes.push(json!({
      "provider": provider, "version": version, "branch": branch, "tags": [], "content": shape,
    }));
  }
}

fn answer(held: &mut Held, janus: bool, me: &str, method: &str, path: &str, body: &Value) -> (u16, Value) {
  let segments: Vec<String> = path
    .split('?')
    .next()
    .unwrap_or_default()
    .split('/')
    .skip(1)
    .map(decode)
    .collect();
  let segments: Vec<&str> = segments.iter().map(String::as_str).collect();
  match (method, segments.as_slice()) {
    ("GET", [""]) => (200, index(janus, me)),
    ("POST", ["contracts", "publish"]) => publish_contracts(held, janus, body),
    _ if !janus => not_found(),
    ("POST", ["provider-shapes", "publish"]) => {
      let Some(content) = decoded(&body["content"]) else {
        return (400, json!({ "errors": { "content": ["is not base64 JSON"] } }));
      };
      let provider = body["pacticipantName"].as_str().unwrap_or_default();
      let version = body["pacticipantVersionNumber"].as_str().unwrap_or_default();
      held.shapes.push(json!({
        "provider": provider, "version": version, "branch": body["branch"], "tags": body["tags"],
        "buildUrl": body["buildUrl"], "content": content,
      }));
      (
        200,
        json!({ "notices": [{ "type": "success", "text": format!("Provider shape published for {provider} version {version}.") }] }),
      )
    }
    ("GET", ["pacticipants", provider, "versions", version, "provider-shape"]) => {
      let found = held
        .shapes
        .iter()
        .rposition(|shape| shape["provider"] == *provider && shape["version"] == *version);
      served(held, me, found)
    }
    (
      "GET",
      [
        "pacticipants",
        provider,
        "branches",
        branch,
        "latest-version",
        "provider-shape",
      ],
    ) => {
      let found = held
        .shapes
        .iter()
        .rposition(|shape| shape["provider"] == *provider && shape["branch"] == *branch);
      served(held, me, found)
    }
    ("POST", ["provider-shapes", id, "subsumption-reports"]) => {
      let Some(shape) = id.parse::<usize>().ok().filter(|id| *id < held.shapes.len()) else {
        return not_found();
      };
      let mut report = body.clone();
      report["contract"]["decoded"] = decoded(&body["contract"]["content"]).unwrap_or(Value::Null);
      held.reports.push(json!({ "shape": shape, "body": report }));
      (200, json!({}))
    }
    _ => not_found(),
  }
}

fn index(janus: bool, me: &str) -> Value {
  let mut links = json!({
    "pb:publish-contracts": { "href": format!("{me}/contracts/publish") },
    "pb:pacticipant": { "href": format!("{me}/pacticipants/{{pacticipant}}"), "templated": true },
  });
  if janus {
    links["pb:publish-janus-contracts"] = json!({ "href": format!("{me}/contracts/publish") });
    links["pb:publish-provider-shape"] = json!({ "href": format!("{me}/provider-shapes/publish") });
    links["pb:provider-shape"] = json!({
      "href": format!("{me}/pacticipants/{{provider}}/versions/{{version}}/provider-shape"), "templated": true,
    });
    links["pb:latest-branch-provider-shape"] = json!({
      "href": format!("{me}/pacticipants/{{provider}}/branches/{{branch}}/latest-version/provider-shape"),
      "templated": true,
    });
  }
  json!({ "_links": links })
}

/// A contract publication: every contract held, unless a `janus` one arrives at a broker without the
/// feature, which refuses it as both brokers refuse an unknown specification today.
fn publish_contracts(held: &mut Held, janus: bool, body: &Value) -> (u16, Value) {
  let contracts = body["contracts"].as_array().cloned().unwrap_or_default();
  if !janus
    && contracts
      .iter()
      .any(|contract| contract["specification"] != json!("pact"))
  {
    return (
      400,
      json!({ "errors": { "contracts": ["specification must be one of: pact"] } }),
    );
  }
  let version = body["pacticipantVersionNumber"].as_str().unwrap_or_default();
  let mut notices = Vec::new();
  for contract in contracts {
    held.contracts.push(json!({
      "consumer": contract["consumerName"], "provider": contract["providerName"], "version": version,
      "specification": contract["specification"],
      "content": decoded(&contract["content"]).unwrap_or(Value::Null),
    }));
    notices.push(json!({ "type": "success", "text": format!(
      "Pact successfully published for {} version {version} and provider {}.",
      contract["consumerName"].as_str().unwrap_or_default(), contract["providerName"].as_str().unwrap_or_default()
    ) }));
  }
  (200, json!({ "notices": notices }))
}

/// A held shape as the brokers serve a pact: the document, with its links added.
fn served(held: &Held, me: &str, found: Option<usize>) -> (u16, Value) {
  let Some(index) = found else {
    return not_found();
  };
  let shape = &held.shapes[index];
  let provider = shape["provider"].as_str().unwrap_or_default();
  let version = shape["version"].as_str().unwrap_or_default();
  let mut document = shape["content"].clone();
  document["_links"] = json!({
    "self": { "href": format!("{me}/pacticipants/{}/versions/{}/provider-shape", encode(provider), encode(version)) },
    "pb:provider-version": { "name": version, "href": format!("{me}/pacticipants/{}/versions/{}", encode(provider), encode(version)) },
    "pb:publish-subsumption-report": { "href": format!("{me}/provider-shapes/{index}/subsumption-reports") },
  });
  (200, document)
}

fn not_found() -> (u16, Value) {
  (
    404,
    json!({ "error": "The requested document was not found on this server." }),
  )
}

fn decoded(content: &Value) -> Option<Value> {
  let bytes = BASE64.decode(content.as_str()?).ok()?;
  serde_json::from_slice(&bytes).ok()
}

fn decode(segment: &str) -> String {
  let bytes = segment.as_bytes();
  let mut out = Vec::with_capacity(bytes.len());
  let mut at = 0;
  while at < bytes.len() {
    if bytes[at] == b'%'
      && let Some(byte) = segment
        .get(at + 1..at + 3)
        .and_then(|hex| u8::from_str_radix(hex, 16).ok())
    {
      out.push(byte);
      at += 3;
      continue;
    }
    out.push(bytes[at]);
    at += 1;
  }
  String::from_utf8_lossy(&out).to_string()
}

fn encode(segment: &str) -> String {
  segment
    .bytes()
    .map(|byte| {
      if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
        (byte as char).to_string()
      } else {
        format!("%{byte:02X}")
      }
    })
    .collect()
}
