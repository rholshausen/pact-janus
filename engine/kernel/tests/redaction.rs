//! ADR 0074 (staged plan S2.8): credentials are kept out of what the engine records and reports. A
//! contract records a redacted member's value as a placeholder; nothing a run, an explanation or a
//! mock reports repeats a value found under one. The HTTP transport declares which members carry
//! credentials, so these tests run with it.

use pact_janus_component_http::HttpTransport;
use pact_janus_component_json::JsonContent;
use pact_janus_kernel::component::TransportComponent;
use pact_janus_kernel::protocol::{Engine, Threads};
use pretty_assertions::assert_eq;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

fn send(engine: &mut Engine, id: &str, op: &str, body: Value) -> Value {
  let request = json!({ "type": "request", "id": id, "op": op, "body": body });
  let bytes = engine.dispatch(&serde_json::to_vec(&request).expect("a json! literal always serializes"));
  serde_json::from_slice(&bytes).expect("Engine::dispatch always returns valid JSON")
}

fn engine() -> Engine {
  let mut transports: HashMap<String, Arc<dyn TransportComponent>> = HashMap::new();
  transports.insert("http".to_string(), Arc::new(HttpTransport::new()));
  let mut engine = Engine::with_components(transports, Some(Arc::new(JsonContent::new())));
  engine.register_background(Arc::new(Threads));
  send(
    &mut engine,
    "h-1",
    "engine/hello",
    json!({ "protocol-versions": [1], "host": { "name": "janus-test", "version": "0.0.0" }, "capabilities": {} }),
  );
  engine
}

/// A GET carrying `headers`, answered with its status.
fn get(addr: &str, path: &str, headers: &[(&str, &str)]) -> u16 {
  let mut stream = TcpStream::connect(addr).expect("the mock is listening");
  stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
  let mut request = format!("GET {path} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n");
  for (name, value) in headers {
    request.push_str(&format!("{name}: {value}\r\n"));
  }
  request.push_str("\r\n");
  stream.write_all(request.as_bytes()).unwrap();
  let mut response = String::new();
  let _ = stream.read_to_string(&mut response);
  response
    .split_whitespace()
    .nth(1)
    .and_then(|status| status.parse().ok())
    .expect("a status line")
}

/// An interaction whose request carries `headers`, each an `equality` over its one value.
fn interaction(headers: &[(&str, &str)]) -> Value {
  let members: serde_json::Map<String, Value> = headers
    .iter()
    .map(|(name, value)| {
      (
        name.to_string(),
        json!({ "shape": "equality", "example": [value] }),
      )
    })
    .collect();
  json!({
    "description": "get an order",
    "transport": { "kind": "http", "mode": "passive" },
    "parts": {
      "request": { "method": { "shape": "equality", "example": "GET" },
                   "path": { "shape": "equality", "example": "/orders/1" },
                   "headers": { "shape": "object", "members": members } },
      "response": { "status": { "shape": "equality", "example": 200 } } }
  })
}

/// Run one consumer session: create it with `config`, add `interaction`, serve its one variant and
/// send it `sent`. Answers `finalise`'s result.
fn record(config: Value, interaction: Value, sent: &[(&str, &str)]) -> Value {
  let mut engine = engine();
  let created = send(
    &mut engine,
    "c-1",
    "consumer-session/create",
    json!({ "config": config }),
  );
  let session = created["ok"]["session"]
    .as_str()
    .unwrap_or_else(|| panic!("{created}"))
    .to_string();
  let added = send(
    &mut engine,
    "c-2",
    "consumer-session/add-interaction",
    json!({ "session": session, "interaction": interaction }),
  );
  let handle = added["ok"]["handle"]
    .as_str()
    .unwrap_or_else(|| panic!("{added}"))
    .to_string();
  send(
    &mut engine,
    "c-3",
    "consumer-session/variants",
    json!({ "session": session, "handle": handle }),
  );
  let started = send(
    &mut engine,
    "c-4",
    "consumer-session/start-transport",
    json!({ "session": session, "transport": "http" }),
  );
  let endpoint = &started["ok"]["endpoint"];
  let addr = format!("{}:{}", endpoint["host"].as_str().unwrap(), endpoint["port"]);
  send(
    &mut engine,
    "c-5",
    "consumer-session/serve-variant",
    json!({ "session": session, "handle": handle, "variant": "base" }),
  );
  get(&addr, "/orders/1", sent);
  send(
    &mut engine,
    "c-9",
    "consumer-session/finalise",
    json!({ "session": session }),
  )["ok"]
    .clone()
}

fn parties() -> Value {
  json!({ "consumer": { "name": "web-app" }, "provider": { "name": "orders" } })
}

#[test]
fn a_token_the_test_declares_is_recorded_as_a_placeholder() {
  let finalised = record(
    parties(),
    interaction(&[
      ("authorization", "Bearer s3cret-token"),
      ("accept", "application/json"),
    ]),
    &[
      ("Authorization", "Bearer s3cret-token"),
      ("Accept", "application/json"),
    ],
  );
  let contract = &finalised["contract"];
  assert!(contract.is_object(), "{finalised}");
  let recorded = &contract["interactions"][0];
  assert_eq!(recorded["redacted"], json!(["request.headers.authorization"]));
  assert_eq!(
    recorded["parts"]["request"]["headers"]["members"]["authorization"],
    json!({ "shape": "type", "example": ["[redacted]"] }),
    "the contract says a value of that kind was sent, not what it was"
  );
  assert_eq!(
    recorded["parts"]["request"]["headers"]["members"]["accept"],
    json!({ "shape": "equality", "example": ["application/json"] }),
    "a member that carries no credential is recorded as declared"
  );
  assert_eq!(
    recorded["selection"]["variants"][0]["parts"]["request"]["headers"]["content"],
    json!({ "accept": ["application/json"], "authorization": ["[redacted]"] })
  );
  assert!(!contract.to_string().contains("s3cret"), "{contract}");
}

#[test]
fn a_session_keeps_a_fixed_token_and_adds_a_member_of_its_own() {
  let mut config = parties();
  config["redaction"] =
    json!({ "keep": ["request.headers.Authorization"], "add": ["request.headers.x-api-key"] });
  let finalised = record(
    config,
    interaction(&[("authorization", "Bearer test-token"), ("x-api-key", "k-123")]),
    &[("Authorization", "Bearer test-token"), ("X-Api-Key", "k-123")],
  );
  let recorded = &finalised["contract"]["interactions"][0];
  assert_eq!(
    recorded["redacted"],
    json!(["request.headers.x-api-key"]),
    "{finalised}"
  );
  assert_eq!(
    recorded["selection"]["variants"][0]["parts"]["request"]["headers"]["content"],
    json!({ "authorization": ["Bearer test-token"], "x-api-key": ["[redacted]"] })
  );
}

#[test]
fn an_interaction_with_no_credential_is_written_as_before() {
  let finalised = record(
    parties(),
    interaction(&[("accept", "text/plain")]),
    &[("Accept", "text/plain")],
  );
  let recorded = &finalised["contract"]["interactions"][0];
  assert!(recorded.is_object(), "{finalised}");
  assert_eq!(recorded.get("redacted"), None);
}

#[test]
fn a_redaction_that_names_no_path_fails_the_create() {
  let mut engine = engine();
  let mut config = parties();
  config["redaction"] = json!({ "add": ["headers"] });
  let created = send(
    &mut engine,
    "c-1",
    "consumer-session/create",
    json!({ "config": config }),
  );
  assert_eq!(created["error"]["code"], "malformed-frame", "{created}");
  assert_eq!(
    created["error"]["details"]["problems"][0]["pointer"],
    "/config/redaction/add/0"
  );
}

#[test]
fn a_request_the_mock_rejects_is_reported_without_the_token_it_carried() {
  let finalised = record(
    parties(),
    interaction(&[("authorization", "Bearer declared-token")]),
    &[("Authorization", "Bearer sent-token")],
  );
  assert_eq!(finalised["results"][0]["status"], "failed", "{finalised}");
  let reported = finalised["results"].to_string();
  assert!(reported.contains("[redacted]"), "{reported}");
  assert!(!reported.contains("sent-token"), "{reported}");
  assert!(!reported.contains("declared-token"), "{reported}");
}

#[test]
fn an_explanation_does_not_repeat_the_values_it_was_given() {
  let mut engine = engine();
  let explained = send(
    &mut engine,
    "e-1",
    "verification/explain",
    json!({
      "interaction": { "kind": "spec", "spec": interaction(&[("authorization", "Bearer declared-token")]) },
      "values": {
        "$.request.method": "GET",
        "$.request.path": "/orders/1",
        "$.request.headers": { "authorization": ["Bearer captured-token"] },
        "$.response.status": 200
      },
      "options": { "plan": true }
    }),
  );
  let explained = explained["ok"].to_string();
  assert!(explained.contains("mismatched"), "{explained}");
  for secret in ["declared-token", "captured-token"] {
    assert!(!explained.contains(secret), "{secret} in {explained}");
  }
}

/// A provider that answers every request with a 401, a cookie and a body that repeats the cookie, and
/// reports each request's header lines.
fn rejecting_provider() -> (String, Receiver<Vec<String>>) {
  let listener = TcpListener::bind("127.0.0.1:0").unwrap();
  let base_url = format!("http://{}", listener.local_addr().unwrap());
  let (tx, rx) = mpsc::channel();
  thread::spawn(move || {
    for stream in listener.incoming() {
      let Ok(stream) = stream else { return };
      let mut reader = BufReader::new(stream);
      let mut lines = Vec::new();
      loop {
        let mut line = String::new();
        if reader.read_line(&mut line).is_err() || line.trim_end().is_empty() {
          break;
        }
        lines.push(line.trim_end().to_string());
      }
      let body = r#"{"error":"session=live-cookie-value"}"#;
      let response = format!(
        "HTTP/1.1 401 Unauthorized\r\ncontent-type: application/json\r\nset-cookie: session=live-cookie-value\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
      );
      let _ = tx.send(lines);
      let _ = reader.get_mut().write_all(response.as_bytes());
    }
  });
  (base_url, rx)
}

#[test]
fn a_verifier_sends_the_placeholder_and_reports_neither_it_nor_the_providers_cookie() {
  let pact = json!({
    "consumer": { "name": "web-app" }, "provider": { "name": "orders" },
    "interactions": [{
      "description": "get an order",
      "request": { "method": "GET", "path": "/orders/1", "headers": { "Authorization": "Bearer recorded-token" } },
      "response": { "status": 200, "headers": { "Set-Cookie": "session=recorded-cookie" }, "body": { "id": 1 } }
    }],
    "metadata": { "pactSpecification": { "version": "3.0.0" } }
  });
  let mut engine = engine();
  let upgraded = send(&mut engine, "u-1", "upgrade/pact", json!({ "pact": pact }));
  let contract = upgraded["ok"]["contract"].clone();
  let redacted: Vec<&str> = upgraded["ok"]["findings"]
    .as_array()
    .unwrap()
    .iter()
    .filter(|finding| finding["code"] == "value-redacted")
    .map(|finding| finding["message"].as_str().unwrap())
    .collect();
  assert_eq!(redacted.len(), 2, "{upgraded}");
  assert!(!contract.to_string().contains("recorded-"), "{contract}");

  let (base_url, requests) = rejecting_provider();
  let started = send(
    &mut engine,
    "v-1",
    "verification/verify",
    json!({
      "source": { "kind": "inline", "contracts": [contract] },
      "target": { "transports": [ { "transport": "http", "options": { "base-url": base_url } } ] },
      "options": { "executed-plan": "always" },
    }),
  );
  let stream = started["ok"]["stream"]
    .as_str()
    .unwrap_or_else(|| panic!("{started}"))
    .to_string();
  let mut events = Vec::new();
  for poll in 0..50 {
    let polled = send(
      &mut engine,
      &format!("p-{poll}"),
      "events/poll",
      json!({ "streams": [stream], "wait-ms": 5_000 }),
    );
    events.extend(polled["ok"]["events"].as_array().unwrap().clone());
    if events.last().is_some_and(|event| event["last"] == json!(true)) {
      break;
    }
  }
  let sent = requests
    .recv_timeout(Duration::from_secs(5))
    .expect("the provider was asked");
  assert!(
    sent
      .iter()
      .any(|line| line.eq_ignore_ascii_case("authorization: [redacted]")),
    "the verifier sends what was recorded: {sent:?}"
  );
  let result = events
    .iter()
    .find(|event| event["kind"] == "verification/interaction-result")
    .map(|event| &event["payload"])
    .expect("a result");
  assert_eq!(result["status"], "failed", "{result}");
  assert_eq!(
    result["redacted"],
    json!(["request.headers.authorization"]),
    "{result}"
  );
  let reported = serde_json::to_string(&events).unwrap();
  assert!(reported.contains("verification/executed-plan"), "{reported}");
  assert!(!reported.contains("live-cookie-value"), "{reported}");
}
