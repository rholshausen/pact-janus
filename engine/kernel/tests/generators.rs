//! Staged plan S1.4, ADR 0034: **a v1–v4 pact's request generators are applied before the request
//! is sent**, as today's verifiers apply them, with the provider-state values a state-change
//! endpoint returns reaching a `ProviderState` generator.
//!
//! Each test verifies one pact against a stub that records every request it is sent and answers
//! the state-change calls too, so a test can see both what the state endpoint was asked and what
//! the provider then received.

use pact_janus_hooks_host::{ExecHooks, HttpHooks};
use pact_janus_kernel::component::TransportComponent;
use pact_janus_kernel::hooks::HookInvoker;
use pact_janus_kernel::protocol::Engine;
use pretty_assertions::assert_eq;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

fn send(engine: &mut Engine, id: &str, op: &str, body: Value) -> Value {
  let request = json!({ "type": "request", "id": id, "op": op, "body": body });
  let bytes = engine.dispatch(&serde_json::to_vec(&request).expect("a json! literal serializes"));
  serde_json::from_slice(&bytes).expect("dispatch always returns valid JSON")
}

fn engine() -> Engine {
  let mut transports: HashMap<String, Arc<dyn TransportComponent>> = HashMap::new();
  transports.insert(
    "http".to_string(),
    Arc::new(pact_janus_component_http::HttpTransport::new()),
  );
  let mut engine = Engine::with_components(
    transports,
    Some(Arc::new(pact_janus_component_json::JsonContent::new())),
  );
  engine.register_hook_invoker("exec", Arc::new(ExecHooks::new()) as Arc<dyn HookInvoker>);
  engine.register_hook_invoker("http", Arc::new(HttpHooks::new()) as Arc<dyn HookInvoker>);
  send(
    &mut engine,
    "r-1",
    "engine/hello",
    json!({ "protocol-versions": [1], "host": { "name": "janus-test", "version": "0.0.0" }, "capabilities": {} }),
  );
  engine
}

/// Verify `pact` against `base_url`, with `states` as the `pact-state-change` endpoint when given,
/// and return every event the run emitted.
fn verify(engine: &mut Engine, pact: Value, base_url: &str, states: Option<&str>) -> Vec<Value> {
  let mut target = json!({
    "transports": [ { "transport": "http", "options": { "base-url": base_url } } ]
  });
  if let Some(states) = states {
    target["hooks"] = json!({
      "version": 1,
      "hooks": {
        "state-setup": [
          { "name": "fixtures", "run": { "kind": "http", "url": states, "format": "pact-state-change" } } ]
      }
    });
  }
  let started = send(
    engine,
    "v-1",
    "verification/verify",
    json!({ "source": { "kind": "inline", "contracts": [pact] }, "target": target }),
  );
  let stream = started["ok"]["stream"]
    .as_str()
    .unwrap_or_else(|| panic!("verify failed: {started}"))
    .to_string();
  let mut events = Vec::new();
  for poll in 0..100 {
    let response = send(
      engine,
      &format!("p-{poll}"),
      "events/poll",
      json!({ "streams": [stream], "wait-ms": 5_000 }),
    );
    let polled = response["ok"]["events"].as_array().cloned().unwrap_or_default();
    let last = polled.iter().any(|event| event["last"] == json!(true));
    events.extend(polled);
    if last {
      return events;
    }
  }
  panic!("the run never finished: {events:?}");
}

fn of_kind<'a>(events: &'a [Value], kind: &str) -> Vec<&'a Value> {
  events
    .iter()
    .filter(|event| event["kind"] == json!(kind))
    .map(|event| &event["payload"])
    .collect()
}

/// One request the stub received: its request line, its headers (names lower-cased) and its body.
#[derive(Debug)]
struct Received {
  line: String,
  headers: HashMap<String, String>,
  body: String,
}

/// A stub provider that answers `/states` with `state_reply` and everything else with `reply`,
/// reporting every request it receives, state calls included, in order.
fn stub(reply: &'static str, state_reply: &'static str) -> (String, Receiver<Received>) {
  let listener = TcpListener::bind("127.0.0.1:0").expect("an OS-assigned port always binds");
  let base_url = format!("http://{}", listener.local_addr().unwrap());
  let (tx, rx) = mpsc::channel();
  thread::spawn(move || {
    for stream in listener.incoming() {
      let Ok(stream) = stream else { return };
      stream.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
      let mut reader = BufReader::new(stream);
      let mut line = String::new();
      if reader.read_line(&mut line).is_err() {
        return;
      }
      let mut headers = HashMap::new();
      loop {
        let mut header = String::new();
        if reader.read_line(&mut header).is_err() {
          return;
        }
        let header = header.trim_end();
        if header.is_empty() {
          break;
        }
        if let Some((name, value)) = header.split_once(':') {
          headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_string());
        }
      }
      let length = headers
        .get("content-length")
        .and_then(|length| length.parse().ok())
        .unwrap_or(0usize);
      let mut body = vec![0u8; length];
      let _ = reader.read_exact(&mut body);
      let line = line.trim_end().to_string();
      let answer = if line.contains(" /states ") {
        state_reply
      } else {
        reply
      };
      let response = format!(
        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{answer}",
        answer.len()
      );
      let stream = reader.get_mut();
      let _ = stream.write_all(response.as_bytes());
      let _ = stream.flush();
      let _ = tx.send(Received {
        line,
        headers,
        body: String::from_utf8_lossy(&body).to_string(),
      });
    }
  });
  (base_url, rx)
}

/// The requests the provider itself received, state-change calls left out.
fn provider_requests(received: &Receiver<Received>) -> Vec<Received> {
  received
    .try_iter()
    .filter(|request| !request.line.contains(" /states "))
    .collect()
}

fn pact(provider_states: Value, request: Value) -> Value {
  json!({
    "consumer": { "name": "SomeConsumer" },
    "provider": { "name": "SomeProvider" },
    "interactions": [{
      "description": "a generated request",
      "providerStates": provider_states,
      "request": request,
      "response": { "status": 200 }
    }],
    "metadata": { "pactSpecification": { "version": "3.0.0" } }
  })
}

/// pact-reference's verifier test for issue #441: a state's own parameter feeds a `ProviderState`
/// path generator, with no state-change endpoint at all.
#[test]
fn a_state_parameter_feeds_a_provider_state_path_generator() {
  let (base_url, received) = stub(r#"{"name": "John"}"#, "{}");
  let pact = pact(
    json!([{ "name": "User exists", "params": { "name": "John" } }]),
    json!({
      "method": "GET",
      "path": "/api/hello/James",
      "generators": {
        "path": { "type": "ProviderState", "expression": "/api/hello/${name}", "dataType": "STRING" }
      }
    }),
  );

  let events = verify(&mut engine(), pact, &base_url, None);

  assert_eq!(
    of_kind(&events, "verification/interaction-result")[0]["status"],
    json!("verified")
  );
  let requests = provider_requests(&received);
  assert_eq!(requests[0].line, "GET /api/hello/John HTTP/1.1");
}

/// The exit's end-to-end case: what the state-change endpoint returned is what the provider is
/// asked for, as today's verifiers do it.
#[test]
fn a_state_change_reply_feeds_a_provider_state_generator() {
  let (base_url, received) = stub("{}", r#"{"id": 1234}"#);
  let pact = pact(
    json!([{ "name": "an order exists" }]),
    json!({
      "method": "GET",
      "path": "/orders/1",
      "generators": { "path": { "type": "ProviderState", "expression": "/orders/${id}" } }
    }),
  );

  let events = verify(
    &mut engine(),
    pact,
    &base_url,
    Some(&format!("{base_url}/states")),
  );

  assert_eq!(
    of_kind(&events, "verification/interaction-result")[0]["status"],
    json!("verified")
  );
  assert!(of_kind(&events, "verification/warning").is_empty(), "{events:?}");
  let requests = provider_requests(&received);
  assert_eq!(requests[0].line, "GET /orders/1234 HTTP/1.1");
}

/// A value generator in each part of the request a provider-side run generates into.
#[test]
fn value_generators_reach_the_header_the_query_and_the_body() {
  let (base_url, received) = stub("{}", "{}");
  let pact = pact(
    json!([]),
    json!({
      "method": "POST",
      "path": "/things",
      "query": { "v1": ["1"] },
      "headers": { "X-Test": "1", "Content-Type": "application/json" },
      "body": { "one": 1, "two": "fixed" },
      "generators": {
        "header": { "X-Test": { "type": "RandomInt", "min": 100, "max": 100 } },
        "query": { "v1": { "type": "RandomInt", "min": 200, "max": 200 } },
        "body": { "$.one": { "type": "RandomInt", "min": 300, "max": 300 } }
      }
    }),
  );

  let events = verify(&mut engine(), pact, &base_url, None);

  assert_eq!(
    of_kind(&events, "verification/interaction-result")[0]["status"],
    json!("verified")
  );
  let requests = provider_requests(&received);
  assert_eq!(requests[0].line, "POST /things?v1=200 HTTP/1.1");
  assert_eq!(requests[0].headers.get("x-test"), Some(&"100".to_string()));
  let body: Value = serde_json::from_str(&requests[0].body).expect("a JSON body");
  assert_eq!(body, json!({ "one": 300, "two": "fixed" }));
}

/// A form body's generators, each applied to one occurrence of its name as today's verifier applies
/// them (ADR 0038 decision 5): `$.b` the first `b`, `$.b[1]` the second, and a state's value through
/// `ProviderState`. The rest of the body is written back as it was.
#[test]
fn generators_reach_a_form_body() {
  let (base_url, received) = stub("{}", r#"{"id": 1234}"#);
  let pact = pact(
    json!([{ "name": "an order exists" }]),
    json!({
      "method": "POST",
      "path": "/orders",
      "headers": { "Content-Type": "application/x-www-form-urlencoded" },
      "body": "a=1&b=2&b=3&id=1&note=as+is",
      "generators": {
        "body": {
          "$.b": { "type": "RandomInt", "min": 7, "max": 7 },
          "$.b[1]": { "type": "RandomInt", "min": 8, "max": 8 },
          "$.id": { "type": "ProviderState", "expression": "${id}" }
        }
      }
    }),
  );

  let events = verify(
    &mut engine(),
    pact,
    &base_url,
    Some(&format!("{base_url}/states")),
  );

  assert_eq!(
    of_kind(&events, "verification/interaction-result")[0]["status"],
    json!("verified")
  );
  assert!(of_kind(&events, "verification/warning").is_empty(), "{events:?}");
  let requests = provider_requests(&received);
  assert_eq!(requests[0].body, "a=1&b=7&b=8&id=1234&note=as+is");
}

/// An XML body's generators, applied to an attribute and an element's text by today's handler, and
/// the document written back as today's verifier writes it (ADR 0039).
#[test]
fn generators_reach_an_xml_body() {
  let (base_url, received) = stub("{}", "{}");
  let pact = pact(
    json!([]),
    json!({
      "method": "POST",
      "path": "/things",
      "headers": { "Content-Type": "application/xml" },
      "body": "<?xml version=\"1.0\" encoding=\"UTF-8\"?><a id=\"1\"><b>2</b><c>as is</c></a>",
      "generators": { "body": {
        "$.a['@id']": { "type": "RandomInt", "min": 7, "max": 7 },
        "$.a.b['#text']": { "type": "RandomInt", "min": 8, "max": 8 }
      } }
    }),
  );

  let events = verify(&mut engine(), pact, &base_url, None);

  assert!(of_kind(&events, "verification/warning").is_empty(), "{events:?}");
  let requests = provider_requests(&received);
  assert_eq!(
    requests[0].body,
    "<?xml version='1.0'?><a id='7'><b>8</b><c>as is</c></a>"
  );
}

/// A body generators cannot reach into is sent as recorded, and each generator on it says so.
#[test]
fn a_generator_on_a_body_it_cannot_read_warns() {
  let (base_url, received) = stub("{}", "{}");
  let body = "--x\r\nContent-Disposition: form-data; name=\"a\"\r\n\r\n1\r\n--x--\r\n";
  let pact = pact(
    json!([]),
    json!({
      "method": "POST",
      "path": "/things",
      "headers": { "Content-Type": "multipart/form-data; boundary=x" },
      "body": body,
      "generators": { "body": { "$.a": { "type": "RandomInt", "min": 7, "max": 7 } } }
    }),
  );

  let events = verify(&mut engine(), pact, &base_url, None);

  let warnings = of_kind(&events, "verification/warning");
  assert_eq!(warnings.len(), 1, "{events:?}");
  assert_eq!(warnings[0]["code"], json!("generator-failed"));
  assert_eq!(warnings[0]["target"], json!("$.request.body.a"));
  let requests = provider_requests(&received);
  assert_eq!(requests[0].body, body);
}

/// A generator with nothing to generate from leaves the recorded value, as today, and says so.
#[test]
fn a_generator_that_produces_nothing_sends_the_recorded_value_and_warns() {
  let (base_url, received) = stub("{}", "{}");
  let pact = pact(
    json!([{ "name": "an order exists" }]),
    json!({
      "method": "GET",
      "path": "/orders/1",
      "generators": { "path": { "type": "ProviderState", "expression": "/orders/${id}" } }
    }),
  );

  let events = verify(
    &mut engine(),
    pact,
    &base_url,
    Some(&format!("{base_url}/states")),
  );

  assert_eq!(
    of_kind(&events, "verification/interaction-result")[0]["status"],
    json!("verified"),
    "a warning never changes a verdict"
  );
  let warnings = of_kind(&events, "verification/warning");
  assert_eq!(warnings.len(), 1, "{events:?}");
  assert_eq!(warnings[0]["code"], json!("generator-failed"));
  assert_eq!(warnings[0]["target"], json!("$.request.path"));
  assert_eq!(warnings[0]["generator"], json!("pact:ProviderState"));
  assert_eq!(warnings[0]["variant"], json!("base"));
  let requests = provider_requests(&received);
  assert_eq!(requests[0].line, "GET /orders/1 HTTP/1.1");
}

/// `MockServerURL` belongs to the consumer side: a provider-side run neither applies it nor
/// warns about it.
#[test]
fn a_consumer_side_generator_is_left_alone() {
  let (base_url, received) = stub("{}", "{}");
  let pact = pact(
    json!([]),
    json!({
      "method": "GET",
      "path": "/things",
      "headers": { "Location": "http://localhost:1234/things/1" },
      "generators": {
        "header": { "Location": { "type": "MockServerURL", "example": "http://localhost:1234/things/1", "regex": ".*(/things/\\d+)$" } }
      }
    }),
  );

  let events = verify(&mut engine(), pact, &base_url, None);

  assert!(of_kind(&events, "verification/warning").is_empty(), "{events:?}");
  let requests = provider_requests(&received);
  assert_eq!(
    requests[0].headers.get("location"),
    Some(&"http://localhost:1234/things/1".to_string())
  );
}
