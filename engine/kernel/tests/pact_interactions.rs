//! ADR 0066: `consumer-session/add-pact-interaction` adds the interaction a consumer library's
//! `pact_ffi` calls build, converted as `upgrade/pact` converts a pact's. Driven through the protocol
//! against the real HTTP transport: a facade test, written as pact-js's http example is, serves its
//! base variant, and the contract it records holds what `janus upgrade` makes of pact-js's pact.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use pact_janus_component_http::HttpTransport;
use pact_janus_component_json::JsonContent;
use pact_janus_kernel::component::TransportComponent;
use pact_janus_kernel::protocol::Engine;
use pretty_assertions::assert_eq;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::Arc;
use std::time::Duration;

fn send(engine: &mut Engine, op: &str, body: Value) -> Value {
  let request = json!({ "type": "request", "id": "r", "op": op, "body": body });
  let bytes = engine.dispatch(&serde_json::to_vec(&request).unwrap());
  serde_json::from_slice(&bytes).unwrap()
}

fn engine() -> (Engine, String) {
  let mut transports: HashMap<String, Arc<dyn TransportComponent>> = HashMap::new();
  transports.insert("http".to_string(), Arc::new(HttpTransport::new()));
  let mut engine = Engine::with_components(transports, Some(Arc::new(JsonContent::new())));
  engine.register_background(Arc::new(pact_janus_kernel::protocol::Threads));
  // The HTTP component's matcher, as the native embeddings register it: an upgraded `Accept` is its
  // `http:media-type` (ADR 0083).
  {
    use pact_janus_component_http::status;
    engine.register_matcher(
      status::NAME,
      "1.0.0",
      status::ACTIONS,
      Arc::new(status::HttpMatcher::new()),
    );
  }
  send(
    &mut engine,
    "engine/hello",
    json!({ "protocol-versions": [1], "host": { "name": "janus-test", "version": "0.0.0" },
            "capabilities": { "test-verdicts": {} } }),
  );
  let created = send(
    &mut engine,
    "consumer-session/create",
    json!({ "config": { "consumer": { "name": "UserConsumer" }, "provider": { "name": "UserProvider" } } }),
  );
  let session = created["ok"]["session"].as_str().unwrap().to_string();
  (engine, session)
}

fn like(value: Value) -> Value {
  json!({ "pact:matcher:type": "type", "value": value })
}

/// pact-js's http example's first test, as its V4 builder calls the FFI.
fn get_user() -> Value {
  json!({ "specification": "V4", "steps": [
    { "call": "given", "description": "a user with ID 1 exists" },
    { "call": "upon_receiving", "description": "a GET request for user 1" },
    { "call": "with_request", "method": "GET", "path": "/users/1" },
    { "call": "with_header_v2", "part": "request", "name": "Accept", "index": 0, "value": "application/json" },
    { "call": "response_status_v2", "status": "200" },
    { "call": "with_header_v2", "part": "response", "name": "Content-Type", "index": 0, "value": "application/json" },
    { "call": "with_body", "part": "response", "content_type": "application/json", "body": like(json!({
        "id": { "pact:matcher:type": "integer", "value": 1 },
        "name": like(json!("Alice")),
        "email": like(json!("alice@example.com"))
    })) }
  ] })
}

/// One GET with an `Accept` header, answering the status and the body.
fn get(addr: &str, path: &str) -> (u16, Value) {
  let mut stream = TcpStream::connect(addr).unwrap();
  stream.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
  let request =
    format!("GET {path} HTTP/1.1\r\nHost: {addr}\r\nAccept: application/json\r\nConnection: close\r\n\r\n");
  stream.write_all(request.as_bytes()).unwrap();
  let mut response = Vec::new();
  let _ = stream.read_to_end(&mut response);
  let text = String::from_utf8_lossy(&response).to_string();
  let status = text
    .split_whitespace()
    .nth(1)
    .and_then(|s| s.parse().ok())
    .unwrap();
  let body = text.split("\r\n\r\n").nth(1).unwrap_or_default();
  (status, serde_json::from_str(body).unwrap_or(Value::Null))
}

#[test]
fn a_facade_test_records_what_upgrade_makes_of_the_pact() {
  let (mut engine, session) = engine();
  let added = send(
    &mut engine,
    "consumer-session/add-pact-interaction",
    json!({ "session": session, "pact": get_user() }),
  );
  let handle = added["ok"]["handle"]
    .as_str()
    .unwrap_or_else(|| panic!("{added}"))
    .to_string();

  // One run, against the example, as pact-js runs the test, whatever policy is asked for: the base
  // variant, pinning nothing, as upgrade records it (ADR 0066 decision 4).
  let selected = send(
    &mut engine,
    "consumer-session/variants",
    json!({ "session": session, "handle": handle, "policy": { "strategy": "exhaustive" } }),
  );
  let variants = selected["ok"]["variants"].as_array().unwrap();
  assert_eq!(variants.len(), 1, "{selected}");
  assert_eq!(variants[0]["assignment"], json!([]), "{selected}");
  let base = variants[0]["id"].as_str().unwrap().to_string();

  let started = send(
    &mut engine,
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
    "consumer-session/serve-variant",
    json!({ "session": session, "handle": handle, "variant": base }),
  );
  assert_eq!(
    get(&addr, "/users/1"),
    (
      200,
      json!({ "id": 1, "name": "Alice", "email": "alice@example.com" })
    )
  );
  let reported = send(
    &mut engine,
    "consumer-session/report-variant",
    json!({ "session": session, "handle": handle, "variant": base, "status": "passed" }),
  );
  assert_eq!(reported["ok"]["result"]["status"], "verified", "{reported}");

  let finalised = send(
    &mut engine,
    "consumer-session/finalise",
    json!({ "session": session }),
  );
  let recorded = &finalised["ok"]["contract"]["interactions"][0];

  let path = format!(
    "{}/../../corpora/verifier/examples/pact-js/http/pact.json",
    env!("CARGO_MANIFEST_DIR")
  );
  let upgraded = send(
    &mut engine,
    "upgrade/pact",
    json!({ "pact": serde_json::from_str::<Value>(&std::fs::read_to_string(path).unwrap()).unwrap() }),
  );
  let expected = upgraded["ok"]["contract"]["interactions"]
    .as_array()
    .unwrap()
    .iter()
    .find(|interaction| interaction["description"] == "a GET request for user 1")
    .unwrap()
    .clone();
  // Everything, the recorded variant too: its values are the interaction's own example, as upgrade
  // records it (ADR 0066 decision 4), so a verifier replays the request today's verifier sends. A
  // JSON body's text is compared as its document: pact_ffi wrote pact-js's pact with its members
  // sorted, and the engine keeps them in the order the test wrote them (ADR 0037).
  assert_eq!(documents(recorded.clone()), documents(expected));
}

/// A contract interaction with each recorded JSON body's text parsed, so member order is not compared.
fn documents(mut interaction: Value) -> Value {
  for variant in interaction["selection"]["variants"].as_array_mut().unwrap() {
    for part in variant["parts"].as_object_mut().unwrap().values_mut() {
      if let Some(body) = part.get_mut("body")
        && body["encoded"] == "text"
        && let Some(document) = body["content"]
          .as_str()
          .and_then(|text| serde_json::from_str::<Value>(text).ok())
      {
        body["content"] = document;
      }
    }
  }
  interaction
}

/// pact-js's messages example, as `addAsynchronousInteraction` and its builder call the FFI.
fn order_placed() -> Value {
  let string = |value: &str| like(json!(value));
  json!({ "specification": "V4", "steps": [
    { "call": "new_async_message", "description": "" },
    { "call": "message_given", "description": "an order has been placed" },
    { "call": "message_expects_to_receive", "description": "an OrderPlaced event" },
    { "call": "message_with_contents", "content_type": "application/json", "body": {
        "orderId": string("order-123"),
        "customerId": string("customer-456"),
        "totalAmount": { "pact:matcher:type": "decimal", "value": 99.99 },
        "currency": { "pact:matcher:type": "regex", "regex": "^[A-Z]{3}$", "value": "USD" },
        "items": { "pact:matcher:type": "type", "min": 1, "value": [{
          "productId": string("prod-789"),
          "quantity": { "pact:matcher:type": "integer", "value": 2 }
        }] }
    } },
    { "call": "message_with_metadata_v2", "name": "queue", "value": "\"orders.placed\"" },
    { "call": "message_with_metadata_v2", "name": "contentType", "value": "\"application/json\"" }
  ] })
}

/// S2.5c: a message's calls build a message, which `serve-variant` hands the host with no transport
/// started (ADR 0067 decision 5), its example as pact-js's test would have been handed it, and the
/// contract records what `janus upgrade` makes of pact-js's pact.
#[test]
fn a_facade_message_test_is_handed_the_example_and_records_what_upgrade_makes_of_the_pact() {
  let (mut engine, session) = engine();
  let added = send(
    &mut engine,
    "consumer-session/add-pact-interaction",
    json!({ "session": session, "pact": order_placed() }),
  );
  let handle = added["ok"]["handle"]
    .as_str()
    .unwrap_or_else(|| panic!("{added}"))
    .to_string();
  let selected = send(
    &mut engine,
    "consumer-session/variants",
    json!({ "session": session, "handle": handle }),
  );
  let base = selected["ok"]["variants"][0]["id"].as_str().unwrap().to_string();
  let served = send(
    &mut engine,
    "consumer-session/serve-variant",
    json!({ "session": session, "handle": handle, "variant": base }),
  );
  let message = &served["ok"]["parts"]["message"];
  assert_eq!(
    message["metadata"]["content"],
    json!({ "queue": "orders.placed", "contentType": "application/json" }),
    "{served}"
  );
  assert_eq!(message["body"]["content-type"], "application/json", "{served}");
  // The body is its octets, as a transport carries it (ADR 0067 decision 5).
  assert_eq!(message["body"]["encoded"], "base64", "{served}");
  let octets = BASE64
    .decode(message["body"]["content"].as_str().unwrap())
    .unwrap();
  let body: Value = serde_json::from_slice(&octets).unwrap();
  assert_eq!(
    body,
    json!({ "orderId": "order-123", "customerId": "customer-456", "totalAmount": 99.99, "currency": "USD",
            "items": [{ "productId": "prod-789", "quantity": 2 }] })
  );
  let reported = send(
    &mut engine,
    "consumer-session/report-variant",
    json!({ "session": session, "handle": handle, "variant": base, "status": "passed" }),
  );
  assert_eq!(reported["ok"]["result"]["status"], "verified", "{reported}");

  let finalised = send(
    &mut engine,
    "consumer-session/finalise",
    json!({ "session": session }),
  );
  let recorded = &finalised["ok"]["contract"]["interactions"][0];
  let path = format!(
    "{}/../../corpora/verifier/messages/pact-js-order-placed/pact.json",
    env!("CARGO_MANIFEST_DIR")
  );
  let upgraded = send(
    &mut engine,
    "upgrade/pact",
    json!({ "pact": serde_json::from_str::<Value>(&std::fs::read_to_string(path).unwrap()).unwrap() }),
  );
  let expected = upgraded["ok"]["contract"]["interactions"][0].clone();
  assert_eq!(documents(recorded.clone()), documents(expected));
}

/// Octets that are not UTF-8, so no component reads them as text.
const OCTETS: [u8; 4] = [0, 1, 2, 0xff];

/// Adds `pact`, selects its one variant and arms it, answering the handle, the variant and what
/// `serve-variant` answered.
fn armed(engine: &mut Engine, session: &str, pact: Value) -> (String, String, Value) {
  let added = send(
    engine,
    "consumer-session/add-pact-interaction",
    json!({ "session": session, "pact": pact }),
  );
  let handle = added["ok"]["handle"]
    .as_str()
    .unwrap_or_else(|| panic!("{added}"))
    .to_string();
  let selected = send(
    engine,
    "consumer-session/variants",
    json!({ "session": session, "handle": handle }),
  );
  let base = selected["ok"]["variants"][0]["id"].as_str().unwrap().to_string();
  let served = send(
    engine,
    "consumer-session/serve-variant",
    json!({ "session": session, "handle": handle, "variant": base }),
  );
  (handle, base, served)
}

/// S2.15: a message given octets is handed exactly those octets, and its contract records them.
#[test]
fn a_facade_message_given_octets_is_handed_those_octets() {
  let (mut engine, session) = engine();
  let (handle, base, served) = armed(
    &mut engine,
    &session,
    json!({ "specification": "V4", "steps": [
      { "call": "new_async_message", "description": "" },
      { "call": "message_expects_to_receive", "description": "a blob" },
      { "call": "message_with_contents", "content_type": "application/octet-stream", "contents": BASE64.encode(OCTETS) }
    ] }),
  );
  let body = &served["ok"]["parts"]["message"]["body"];
  assert_eq!(body["encoded"], "base64", "{served}");
  assert_eq!(BASE64.decode(body["content"].as_str().unwrap()).unwrap(), OCTETS);

  send(
    &mut engine,
    "consumer-session/report-variant",
    json!({ "session": session, "handle": handle, "variant": base, "status": "passed" }),
  );
  let finalised = send(
    &mut engine,
    "consumer-session/finalise",
    json!({ "session": session }),
  );
  let recorded =
    &finalised["ok"]["contract"]["interactions"][0]["selection"]["variants"][0]["parts"]["message"]["body"];
  assert_eq!(recorded["encoded"], "base64", "{finalised}");
  assert_eq!(
    BASE64.decode(recorded["content"].as_str().unwrap()).unwrap(),
    OCTETS
  );
}

/// S2.15: an HTTP reply given octets is served exactly those octets.
#[test]
fn a_facade_reply_given_octets_is_served_those_octets() {
  let (mut engine, session) = engine();
  let started = send(
    &mut engine,
    "consumer-session/start-transport",
    json!({ "session": session, "transport": "http" }),
  );
  let endpoint = &started["ok"]["endpoint"];
  let addr = format!(
    "{}:{}",
    endpoint["host"].as_str().unwrap(),
    endpoint["port"].as_u64().unwrap()
  );
  armed(
    &mut engine,
    &session,
    json!({ "specification": "V4", "steps": [
      { "call": "upon_receiving", "description": "a GET for a blob" },
      { "call": "with_request", "method": "GET", "path": "/blob" },
      { "call": "response_status_v2", "status": "200" },
      { "call": "with_binary_file", "part": "response", "content_type": "application/octet-stream", "body": BASE64.encode(OCTETS) }
    ] }),
  );

  let mut stream = TcpStream::connect(&addr).unwrap();
  stream.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
  let request = format!("GET /blob HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n");
  stream.write_all(request.as_bytes()).unwrap();
  let mut response = Vec::new();
  let _ = stream.read_to_end(&mut response);
  let head_end = response.windows(4).position(|w| w == b"\r\n\r\n").unwrap() + 4;
  let head = String::from_utf8_lossy(&response[..head_end]).to_lowercase();
  assert!(head.starts_with("http/1.1 200"), "{head}");
  assert!(head.contains("content-type: application/octet-stream"), "{head}");
  assert_eq!(&response[head_end..], OCTETS);
}

/// ADR 0066 decision 4, as amended: a body no generator fills is served as the library wrote it, as
/// today's mocks serve it, rather than re-encoded from its document.
#[test]
fn a_facade_reply_no_generator_fills_is_served_as_written() {
  let (mut engine, session) = engine();
  let started = send(
    &mut engine,
    "consumer-session/start-transport",
    json!({ "session": session, "transport": "http" }),
  );
  let endpoint = &started["ok"]["endpoint"];
  let addr = format!(
    "{}:{}",
    endpoint["host"].as_str().unwrap(),
    endpoint["port"].as_u64().unwrap()
  );
  let written = "{\"responsetest\": true, \"version\": \"v3\"}";
  armed(
    &mut engine,
    &session,
    json!({ "specification": "V4", "steps": [
      { "call": "upon_receiving", "description": "a GET written by hand" },
      { "call": "with_request", "method": "GET", "path": "/written" },
      { "call": "response_status_v2", "status": "200" },
      { "call": "with_binary_body", "part": "response", "content_type": "application/json", "body": BASE64.encode(written) }
    ] }),
  );

  let mut stream = TcpStream::connect(&addr).unwrap();
  stream.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
  let request = format!("GET /written HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n");
  stream.write_all(request.as_bytes()).unwrap();
  let mut response = Vec::new();
  let _ = stream.read_to_end(&mut response);
  let head_end = response.windows(4).position(|w| w == b"\r\n\r\n").unwrap() + 4;
  assert_eq!(String::from_utf8_lossy(&response[head_end..]), written);
}

#[test]
fn a_step_the_engine_cannot_make_is_refused_pointing_at_it() {
  let (mut engine, session) = engine();
  let refused = send(
    &mut engine,
    "consumer-session/add-pact-interaction",
    json!({ "session": session, "pact": { "steps": [
      { "call": "upon_receiving", "description": "a plugin interaction" },
      { "call": "with_plugin_contents", "part": "request" }
    ] } }),
  );
  assert_eq!(refused["error"]["code"], "interaction-invalid", "{refused}");
  assert_eq!(
    refused["error"]["details"]["problems"][0]["pointer"],
    "/pact/steps/1/call"
  );

  // A refused interaction withholds the contract, named by its description (ADR 0064 decision 4).
  let finalised = send(
    &mut engine,
    "consumer-session/finalise",
    json!({ "session": session }),
  );
  assert_eq!(
    finalised["ok"]["refused"][0]["description"], "a plugin interaction",
    "{finalised}"
  );
}

#[test]
fn what_the_conversion_lost_is_in_the_answer() {
  let (mut engine, session) = engine();
  // pact-js's eachKeyMatches: upgrade keeps the keys' rule and leaves the values open.
  let added = send(
    &mut engine,
    "consumer-session/add-pact-interaction",
    json!({ "session": session, "pact": { "steps": [
      { "call": "upon_receiving", "description": "the catalog" },
      { "call": "with_request", "method": "GET", "path": "/catalog" },
      { "call": "response_status_v2", "status": "200" },
      { "call": "with_body", "part": "response", "content_type": "application/json", "body": {
          "pact:matcher:type": "eachKey",
          "rules": [ { "pact:matcher:type": "regex", "regex": "^[A-Z]+-\\d{3}$", "value": "WIDGET-001" } ],
          "value": { "WIDGET-001": { "name": like(json!("Widget")) } }
      } }
    ] } }),
  );
  let findings = added["ok"]["findings"]
    .as_array()
    .unwrap_or_else(|| panic!("{added}"));
  let lossy = findings.iter().find(|f| f["code"] == "rule-unmapped").unwrap();
  assert_eq!(lossy["kind"], "lossy");
  assert_eq!(
    lossy["path"], "/response/body",
    "a pointer into the built interaction"
  );

  // The shapes cannot give the example back (an each-entry of open values, least size 0), so the
  // mock serves the interaction's own, as today's mock server does.
  let handle = added["ok"]["handle"].as_str().unwrap().to_string();
  let selected = send(
    &mut engine,
    "consumer-session/variants",
    json!({ "session": session, "handle": handle }),
  );
  let base = selected["ok"]["variants"][0]["id"].as_str().unwrap().to_string();
  let started = send(
    &mut engine,
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
    "consumer-session/serve-variant",
    json!({ "session": session, "handle": handle, "variant": base }),
  );
  assert_eq!(
    get(&addr, "/catalog"),
    (200, json!({ "WIDGET-001": { "name": "Widget" } }))
  );
}
