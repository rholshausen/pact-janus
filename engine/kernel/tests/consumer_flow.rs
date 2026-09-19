//! Plan task 4.5: the protocol-level consumer flow, proven against the real transport/content
//! components (`pact_janus_component_http`, `pact_janus_component_json`) rather than a resolver
//! standing in for a wire — the same bar 1.2/1.3's own "against the real engine, not the toy"
//! sets. A real HTTP client drives the mock the engine itself starts and serves: submit the RFC
//! order interaction, iterate its variants, `serve-variant` then send a real request per variant,
//! `finalise`, and assert the written contract.
//!
//! The HTTP client is hand-rolled (`std::net::TcpStream`, one GET, no body, an empty JSON object
//! back): a request this trivial doesn't earn a new dependency, and CLAUDE.md keeps this
//! prototype's dependency list part of the thin-SDK story it's telling.

use pact_janus_component_http::HttpTransport;
use pact_janus_component_json::JsonContent;
use pact_janus_kernel::component::{self, TransportComponent};
use pact_janus_kernel::protocol::Engine;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::Arc;
use std::time::Duration;

fn send(engine: &mut Engine, id: &str, op: &str, body: Value) -> Value {
  let request = json!({ "type": "request", "id": id, "op": op, "body": body });
  let bytes = engine.dispatch(&serde_json::to_vec(&request).expect("a json! literal always serializes"));
  serde_json::from_slice(&bytes).expect("Engine::dispatch always returns valid JSON")
}

fn engine_with_real_components() -> Engine {
  let mut transports: HashMap<String, Arc<dyn TransportComponent>> = HashMap::new();
  transports.insert("http".to_string(), Arc::new(HttpTransport::new()));
  Engine::with_components(transports, Some(Arc::new(JsonContent::new())))
}

fn hello(engine: &mut Engine) {
  send(
    engine,
    "r-1",
    "engine/hello",
    json!({ "protocol-versions": [1], "host": { "name": "janus-test", "version": "0.0.0" }, "capabilities": {} }),
  );
}

// Same fixture as tests/protocol.rs's own `order_interaction()`: an empty-object body has no
// variant dimensions, so this interaction's space is the degenerate `base`-only case — enough to
// prove the wiring without also re-proving plan task 4.3's sampler.
fn order_interaction() -> Value {
  json!({
    "description": "a request for an order",
    "transport": { "kind": "http", "mode": "passive" },
    "parts": {
      "request": { "method": { "shape": "equality", "example": "GET" },
                   "path": { "shape": "equality", "example": "/orders/66" } },
      "response": { "status": { "shape": "equality", "example": 200 },
                    "body": { "shape": "object", "members": {} } } }
  })
}

/// One GET, parsed just enough to get a status code and a JSON body: read until the peer closes
/// (or a 2s safety timeout, in case it doesn't) rather than a real header parser, since a partial
/// read still leaves everything read so far in `response` (`Read::read_to_end`'s own guarantee).
fn http_get(addr: &str, path: &str) -> (u16, Value) {
  let (status, _, body) = http_get_with_headers(addr, path);
  (status, body)
}

/// As [`http_get`], with the response's header lines too (names lower-cased).
fn http_get_with_headers(addr: &str, path: &str) -> (u16, Vec<(String, String)>, Value) {
  let mut stream =
    TcpStream::connect(addr).unwrap_or_else(|err| panic!("connecting to the mock at {addr}: {err}"));
  stream
    .set_read_timeout(Some(Duration::from_secs(2)))
    .expect("setting a read timeout");
  let request = format!("GET {path} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n");
  stream.write_all(request.as_bytes()).expect("sending the request");

  let mut response = Vec::new();
  let _ = stream.read_to_end(&mut response);
  let split = response
    .windows(4)
    .position(|w| w == b"\r\n\r\n")
    .unwrap_or_else(|| {
      panic!(
        "no header/body separator in: {}",
        String::from_utf8_lossy(&response)
      )
    });
  let head = std::str::from_utf8(&response[..split]).expect("headers are ASCII");
  let body = &response[split + 4..];

  let status_line = head.lines().next().expect("a status line");
  let status: u16 = status_line
    .split_whitespace()
    .nth(1)
    .expect("a status code")
    .parse()
    .expect("a numeric status code");
  let json = if body.is_empty() {
    Value::Null
  } else {
    serde_json::from_slice(body).unwrap_or_else(|err| panic!("parsing the response body as JSON: {err}"))
  };
  let headers = head
    .lines()
    .skip(1)
    .filter_map(|line| line.split_once(':'))
    .map(|(name, value)| (name.trim().to_ascii_lowercase(), value.trim().to_string()))
    .collect();
  (status, headers, json)
}

#[test]
fn a_real_http_client_drives_every_variant_through_the_mock_to_a_written_contract() {
  let mut engine = engine_with_real_components();
  hello(&mut engine);

  let create = send(
    &mut engine,
    "r-2",
    "consumer-session/create",
    json!({ "config": { "consumer": { "name": "web-app" }, "provider": { "name": "order-api" } } }),
  );
  let session = create["ok"]["session"].as_str().unwrap().to_string();

  let added = send(
    &mut engine,
    "r-3",
    "consumer-session/add-interaction",
    json!({ "session": session, "interaction": order_interaction() }),
  );
  let handle = added["ok"]["handle"].as_str().unwrap().to_string();

  let variants = send(
    &mut engine,
    "r-4",
    "consumer-session/variants",
    json!({ "session": session, "handle": handle }),
  );
  let variant_ids: Vec<String> = variants["ok"]["variants"]
    .as_array()
    .unwrap()
    .iter()
    .map(|v| v["id"].as_str().unwrap().to_string())
    .collect();
  assert_eq!(
    variant_ids,
    vec!["base"],
    "an empty-object body has no variant dimensions"
  );

  let started = send(
    &mut engine,
    "r-5",
    "consumer-session/start-transport",
    json!({ "session": session, "transport": "http" }),
  );
  let endpoint = &started["ok"]["endpoint"];
  let addr = format!(
    "{}:{}",
    endpoint["host"].as_str().expect("endpoint.host"),
    endpoint["port"].as_u64().expect("endpoint.port")
  );

  for (i, variant_id) in variant_ids.iter().enumerate() {
    let served = send(
      &mut engine,
      &format!("r-{}", 10 + i),
      "consumer-session/serve-variant",
      json!({ "session": session, "handle": handle, "variant": variant_id }),
    );
    assert_eq!(served["ok"], json!({}), "serve-variant armed '{variant_id}'");

    let (status, body) = http_get(&addr, "/orders/66");
    assert_eq!(
      status, 200,
      "the mock answers a request matching '{variant_id}' with the declared status"
    );
    assert_eq!(
      body,
      json!({}),
      "the mock answers with '{variant_id}'s generated body"
    );
  }

  let request = json!({ "type": "request", "id": "r-99", "op": "consumer-session/finalise",
                        "body": { "session": session } });
  let frame = engine.dispatch(&serde_json::to_vec(&request).unwrap());
  let finalised: Value = serde_json::from_slice(&frame).unwrap();
  assert_eq!(
    finalised["ok"]["results"],
    json!([ { "handle": "i-1", "status": "verified",
              "variants": [ { "variant": "base", "status": "verified" } ] } ]),
    "the real exchange marked the only selected variant verified"
  );

  let contract = &finalised["ok"]["contract"];
  assert!(
    contract.is_object(),
    "every variant verified, so the contract is present"
  );
  assert_eq!(contract["$format"], "janus-contract/1");
  assert_eq!(contract["consumer"]["name"], "web-app");
  assert_eq!(contract["provider"]["name"], "order-api");
  let recorded_variants = contract["interactions"][0]["selection"]["variants"]
    .as_array()
    .expect("recorded variants");
  assert_eq!(
    recorded_variants.len(),
    1,
    "the honesty rule: exactly the exercised variant"
  );
  assert_eq!(recorded_variants[0]["id"], "base");
  assert_eq!(
    recorded_variants[0]["parts"]["response"]["status"]["content"], 200,
    "the recorded evidence is what the mock actually sent back, not a re-derivation"
  );

  // The frame carries the contract exactly as the canonical writer writes it — members in the
  // specified order, `$format` first (contract-file spec §2.4) — so a host that writes the bytes
  // it received writes a canonical contract.
  let model: pact_janus_kernel::contract::Contract = serde_json::from_value(contract.clone()).unwrap();
  let canonical = pact_janus_kernel::contract::write_canonical(&model).unwrap();
  let canonical = std::str::from_utf8(&canonical).unwrap().trim_end();
  assert!(
    std::str::from_utf8(&frame)
      .unwrap()
      .contains(&format!("\"contract\":{canonical}")),
    "finalise carries the canonical bytes:\n{canonical}"
  );
}

// Widened with an `optional` member (same fixture shape as tests/protocol.rs's own), so `variants`
// selects two: `base` and the boundary where `shippedAt` is absent.
fn order_interaction_with_optional_field() -> Value {
  let mut interaction = order_interaction();
  interaction["parts"]["response"]["body"] = json!({
    "shape": "object",
    "members": { "shippedAt": { "shape": "optional", "of": { "shape": "string", "example": "2026-07-30" } } }
  });
  interaction
}

#[test]
fn a_real_http_client_drives_more_than_one_variant_in_sequence() {
  let mut engine = engine_with_real_components();
  hello(&mut engine);
  let create = send(
    &mut engine,
    "r-2",
    "consumer-session/create",
    json!({ "config": { "consumer": { "name": "web-app" }, "provider": { "name": "order-api" } } }),
  );
  let session = create["ok"]["session"].as_str().unwrap().to_string();

  let added = send(
    &mut engine,
    "r-3",
    "consumer-session/add-interaction",
    json!({ "session": session, "interaction": order_interaction_with_optional_field() }),
  );
  let handle = added["ok"]["handle"].as_str().unwrap().to_string();

  let variants = send(
    &mut engine,
    "r-4",
    "consumer-session/variants",
    json!({ "session": session, "handle": handle }),
  );
  let variant_ids: Vec<String> = variants["ok"]["variants"]
    .as_array()
    .unwrap()
    .iter()
    .map(|v| v["id"].as_str().unwrap().to_string())
    .collect();
  assert_eq!(
    variant_ids.len(),
    2,
    "the optional member contributes a boundary variant"
  );

  let started = send(
    &mut engine,
    "r-5",
    "consumer-session/start-transport",
    json!({ "session": session, "transport": "http" }),
  );
  let endpoint = &started["ok"]["endpoint"];
  let addr = format!(
    "{}:{}",
    endpoint["host"].as_str().unwrap(),
    endpoint["port"].as_u64().unwrap()
  );

  let mut bodies = Vec::new();
  for (i, variant_id) in variant_ids.iter().enumerate() {
    send(
      &mut engine,
      &format!("r-{}", 10 + i),
      "consumer-session/serve-variant",
      json!({ "session": session, "handle": handle, "variant": variant_id }),
    );
    let (status, body) = http_get(&addr, "/orders/66");
    assert_eq!(status, 200);
    bodies.push(body);
  }
  // The two variants produce different bodies (one with `shippedAt`, one without) — proof the
  // loop re-armed between requests rather than replaying the first variant's response twice.
  assert_ne!(
    bodies[0], bodies[1],
    "each variant's request got that variant's own response"
  );

  let finalised = send(
    &mut engine,
    "r-99",
    "consumer-session/finalise",
    json!({ "session": session }),
  );
  let variant_results = finalised["ok"]["results"][0]["variants"].as_array().unwrap();
  assert!(
    variant_results.iter().all(|v| v["status"] == "verified"),
    "both variants verified: {variant_results:?}"
  );
  assert!(finalised["ok"]["contract"].is_object());
}

#[test]
fn a_request_that_does_not_match_the_armed_variant_fails_it_and_withholds_the_contract() {
  let mut engine = engine_with_real_components();
  hello(&mut engine);
  let create = send(
    &mut engine,
    "r-2",
    "consumer-session/create",
    json!({ "config": { "consumer": { "name": "web-app" }, "provider": { "name": "order-api" } } }),
  );
  let session = create["ok"]["session"].as_str().unwrap().to_string();

  let added = send(
    &mut engine,
    "r-3",
    "consumer-session/add-interaction",
    json!({ "session": session, "interaction": order_interaction() }),
  );
  let handle = added["ok"]["handle"].as_str().unwrap().to_string();
  send(
    &mut engine,
    "r-4",
    "consumer-session/variants",
    json!({ "session": session, "handle": handle }),
  );
  let started = send(
    &mut engine,
    "r-5",
    "consumer-session/start-transport",
    json!({ "session": session, "transport": "http" }),
  );
  let endpoint = &started["ok"]["endpoint"];
  let addr = format!(
    "{}:{}",
    endpoint["host"].as_str().unwrap(),
    endpoint["port"].as_u64().unwrap()
  );

  send(
    &mut engine,
    "r-6",
    "consumer-session/serve-variant",
    json!({ "session": session, "handle": handle, "variant": "base" }),
  );
  // The interaction expects GET /orders/66; this asks for a different order entirely.
  let (status, body) = http_get(&addr, "/orders/99");
  assert_eq!(
    status, 500,
    "a request the armed variant does not admit is answered, not hung"
  );
  assert!(
    body.as_array().is_some_and(|mismatches| !mismatches.is_empty()),
    "the mismatch reply names what didn't match: {body:?}"
  );

  let finalised = send(
    &mut engine,
    "r-99",
    "consumer-session/finalise",
    json!({ "session": session }),
  );
  assert_eq!(finalised["ok"]["results"][0]["status"], "failed");
  assert_eq!(finalised["ok"]["results"][0]["variants"][0]["status"], "failed");
  // Why it failed rides in the result too (spec §8.2) — the same mismatches the reply named —
  // so a host can report it without reading the engine's log.
  assert_eq!(
    finalised["ok"]["results"][0]["variants"][0]["mismatches"], body,
    "finalise reports the mismatches the mock answered with"
  );
  assert!(
    finalised["ok"].get("contract").is_none(),
    "the honesty rule withholds the contract for a failed variant"
  );
}

/// A transport whose one arrival matches, but whose reply can never be delivered — the client gave
/// up, or the connection was unusable (as it was for an HTTP/2-upgrade offer before the HTTP
/// transport learnt to answer one).
struct UndeliverableTransport {
  released: std::sync::Mutex<bool>,
}

impl TransportComponent for UndeliverableTransport {
  fn content_slots(&self) -> component::ContentSlots {
    component::ContentSlots::new()
  }
  fn start(&self, _: component::Start) -> Result<component::StartResult, component::ComponentError> {
    Ok(component::StartResult {
      endpoint: json!({ "kind": "http", "base-url": "http://unused" }),
    })
  }
  fn stop(&self, _: component::Stop) -> Result<component::StopResult, component::ComponentError> {
    Ok(component::StopResult {})
  }
  fn send(&self, _: component::Send) -> Result<component::SendResult, component::ComponentError> {
    Err(component::ComponentError::transport_failed("no drive role here"))
  }
  fn poll_inbound(
    &self,
    req: component::PollInbound,
  ) -> Result<component::PollInboundResult, component::ComponentError> {
    let mut released = self.released.lock().unwrap();
    if !*released {
      drop(released);
      std::thread::sleep(Duration::from_millis(req.timeout_ms.min(20)));
      return Ok(component::PollInboundResult { inbound: None });
    }
    *released = false;
    let slot = |v: Value| component::SlotValue {
      content: v,
      encoded: None,
      content_type: None,
    };
    let mut request = component::Part::new();
    request.insert("method".to_string(), slot(json!("GET")));
    request.insert("path".to_string(), slot(json!("/orders/66")));
    let mut parts = component::Parts::new();
    parts.insert("request".to_string(), request);
    Ok(component::PollInboundResult {
      inbound: Some(component::Inbound {
        event: "e-1".to_string(),
        parts,
        expects_reply: true,
      }),
    })
  }
  fn reply(&self, _: component::Reply) -> Result<component::ReplyResult, component::ComponentError> {
    Err(component::ComponentError::transport_failed(
      "connection reset by peer",
    ))
  }
  fn dispose(&self, _: component::Dispose) -> Result<component::DisposeResult, component::ComponentError> {
    Ok(component::DisposeResult {})
  }
}

#[test]
fn a_matched_request_whose_response_never_reached_the_consumer_is_not_verified() {
  let transport = Arc::new(UndeliverableTransport {
    released: std::sync::Mutex::new(false),
  });
  let mut transports: HashMap<String, Arc<dyn TransportComponent>> = HashMap::new();
  transports.insert("http".to_string(), transport.clone());
  let mut engine = Engine::with_components(transports, Some(Arc::new(JsonContent::new())));
  hello(&mut engine);

  let created = send(
    &mut engine,
    "r-2",
    "consumer-session/create",
    json!({ "config": { "consumer": { "name": "c" }, "provider": { "name": "p" } } }),
  );
  let session = created["ok"]["session"].as_str().unwrap().to_string();
  let added = send(
    &mut engine,
    "r-3",
    "consumer-session/add-interaction",
    json!({ "session": session, "interaction": order_interaction() }),
  );
  let handle = added["ok"]["handle"].as_str().unwrap().to_string();
  send(
    &mut engine,
    "r-4",
    "consumer-session/variants",
    json!({ "session": session, "handle": handle }),
  );
  send(
    &mut engine,
    "r-5",
    "consumer-session/start-transport",
    json!({ "session": session, "transport": "http" }),
  );
  send(
    &mut engine,
    "r-6",
    "consumer-session/serve-variant",
    json!({ "session": session, "handle": handle, "variant": "base" }),
  );

  *transport.released.lock().unwrap() = true;
  let deadline = std::time::Instant::now() + Duration::from_secs(5);
  while *transport.released.lock().unwrap() {
    assert!(
      std::time::Instant::now() < deadline,
      "the exchange loop never polled the arrival"
    );
    std::thread::sleep(Duration::from_millis(10));
  }
  std::thread::sleep(Duration::from_millis(100)); // let the loop finish replying and recording

  let finalised = send(
    &mut engine,
    "r-7",
    "consumer-session/finalise",
    json!({ "session": session }),
  );
  let variant = &finalised["ok"]["results"][0]["variants"][0];
  assert_eq!(variant["status"], "failed", "{finalised}");
  assert!(
    variant["mismatches"][0]["message"]
      .as_str()
      .unwrap()
      .contains("could not be delivered"),
    "{variant}"
  );
  assert!(
    finalised["ok"].get("contract").is_none(),
    "no contract for an undelivered response"
  );
}

/// The declared response headers are served as headers, and the JSON body is labelled with its
/// media type — the headers map is as structured as a body, and only the transport's declaration
/// (component-interfaces spec §5.5) tells the engine which of the two is content.
#[test]
fn declared_response_headers_are_served_and_a_json_body_is_labelled() {
  let mut engine = engine_with_real_components();
  hello(&mut engine);
  let created = send(
    &mut engine,
    "r-2",
    "consumer-session/create",
    json!({ "config": { "consumer": { "name": "c" }, "provider": { "name": "p" } } }),
  );
  let session = created["ok"]["session"].as_str().unwrap().to_string();
  let mut interaction = order_interaction();
  interaction["parts"]["response"]["headers"] = json!({ "shape": "object", "members": {
    "x-served": { "shape": "equality", "example": ["yes"] } } });
  interaction["parts"]["response"]["body"] = json!({ "shape": "object", "members": {
    "id": { "shape": "equality", "example": 66 } } });
  let added = send(
    &mut engine,
    "r-3",
    "consumer-session/add-interaction",
    json!({ "session": session, "interaction": interaction }),
  );
  let handle = added["ok"]["handle"].as_str().unwrap().to_string();
  send(
    &mut engine,
    "r-4",
    "consumer-session/variants",
    json!({ "session": session, "handle": handle }),
  );
  let started = send(
    &mut engine,
    "r-5",
    "consumer-session/start-transport",
    json!({ "session": session, "transport": "http" }),
  );
  let base_url = started["ok"]["endpoint"]["base-url"].as_str().unwrap();
  let addr = base_url.trim_start_matches("http://").to_string();
  send(
    &mut engine,
    "r-6",
    "consumer-session/serve-variant",
    json!({ "session": session, "handle": handle, "variant": "base" }),
  );

  let (status, headers, body) = http_get_with_headers(&addr, "/orders/66");
  assert_eq!(status, 200);
  assert_eq!(body, json!({ "id": 66 }));
  let header = |name: &str| headers.iter().find(|(n, _)| n == name).map(|(_, v)| v.as_str());
  assert_eq!(header("x-served"), Some("yes"), "{headers:?}");
  assert_eq!(header("content-type"), Some("application/json"), "{headers:?}");

  let finalised = send(
    &mut engine,
    "r-7",
    "consumer-session/finalise",
    json!({ "session": session }),
  );
  assert_eq!(finalised["ok"]["results"][0]["status"], "verified");
}
