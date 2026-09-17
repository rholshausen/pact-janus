//! Plan task 5.1: the built-in HTTP transport component, **drive-role** side
//! (component-interfaces spec §5.1–§5.2) — proven against a real socket, the mirror of
//! `http_transport.rs`'s serve-role tests. A hand-rolled stub provider (no second HTTP dependency
//! just for tests, same reasoning as that file) records what actually went out and answers what
//! the test tells it to, so every assertion here is about bytes on a wire rather than about the
//! client library's own view of them.
//!
//! The scenario is the worked example's §3 "Driving: provider verification"
//! (`Documentation/specs/component-interfaces/examples/http-transport-both-bindings.md`), down to
//! its 500-with-a-body case: a verification run must *observe* a provider's failure, so a non-2xx
//! status is a reply like any other and never a transport error.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use pact_janus_component_http::HttpTransport;
use pact_janus_kernel::component::{Parts, PollInbound, Send, SlotValue, Start, Stop, TransportComponent};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

/// What the stub provider saw: the request line, the headers as sent, and the body bytes.
struct Received {
  request_line: String,
  headers: Vec<(String, String)>,
  body: Vec<u8>,
}

/// A one-shot stub provider. Returns its base URL and a channel carrying the single request it
/// receives, so the test can assert on what the component actually put on the wire.
fn stub_provider(
  status_line: &str,
  response_headers: &[(&str, &str)],
  body: &[u8],
) -> (String, Receiver<Received>) {
  let listener = TcpListener::bind("127.0.0.1:0").expect("an OS-assigned port always binds");
  let base_url = format!("http://{}", listener.local_addr().unwrap());
  let (tx, rx) = mpsc::channel();

  let status_line = status_line.to_string();
  let response_headers: Vec<(String, String)> = response_headers
    .iter()
    .map(|(n, v)| (n.to_string(), v.to_string()))
    .collect();
  let body = body.to_vec();
  thread::spawn(move || {
    let (stream, _) = listener.accept().expect("the client connects");
    stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    let mut reader = BufReader::new(stream);

    let mut request_line = String::new();
    reader.read_line(&mut request_line).unwrap();
    let mut headers = Vec::new();
    let mut length = 0usize;
    loop {
      let mut line = String::new();
      reader.read_line(&mut line).unwrap();
      let line = line.trim_end().to_string();
      if line.is_empty() {
        break;
      }
      let (name, value) = line.split_once(':').expect("a header line has a colon");
      let (name, value) = (name.trim().to_string(), value.trim().to_string());
      if name.eq_ignore_ascii_case("content-length") {
        length = value.parse().unwrap_or(0);
      }
      headers.push((name, value));
    }
    let mut request_body = vec![0u8; length];
    reader.read_exact(&mut request_body).unwrap();

    let mut response = format!("HTTP/1.1 {status_line}\r\n");
    for (name, value) in &response_headers {
      response.push_str(&format!("{name}: {value}\r\n"));
    }
    response.push_str(&format!("Content-Length: {}\r\n\r\n", body.len()));
    let stream = reader.get_mut();
    stream.write_all(response.as_bytes()).unwrap();
    stream.write_all(&body).unwrap();
    stream.flush().unwrap();

    let _ = tx.send(Received {
      request_line: request_line.trim_end().to_string(),
      headers,
      body: request_body,
    });
  });

  (base_url, rx)
}

fn start_drive(transport: &HttpTransport, instance: &str, base_url: &str) -> Value {
  transport
    .start(Start {
      instance: instance.to_string(),
      kind: "http".to_string(),
      role: "drive".to_string(),
      options: Some(json!({ "base-url": base_url })),
    })
    .expect("a drive instance binds nothing and cannot fail")
    .endpoint
}

fn text_slot(text: &str) -> SlotValue {
  SlotValue {
    content: Value::String(text.to_string()),
    encoded: None,
    content_type: None,
  }
}

fn request_parts(slots: Vec<(&str, SlotValue)>) -> Parts {
  let mut part = BTreeMap::new();
  for (name, slot) in slots {
    part.insert(name.to_string(), slot);
  }
  let mut parts = Parts::new();
  parts.insert("request".to_string(), part);
  parts
}

fn send(transport: &HttpTransport, instance: &str, parts: Parts, await_reply: bool) -> Option<Parts> {
  transport
    .send(Send {
      instance: instance.to_string(),
      parts,
      await_reply,
      timeout_ms: Some(5_000),
    })
    .expect("the stub provider answers")
    .reply
}

#[test]
fn a_drive_instance_echoes_its_target_as_the_endpoint_descriptor() {
  let transport = HttpTransport::new();
  // A trailing slash is normalised away so `base-url` + `path` never produces a doubled one.
  let endpoint = start_drive(&transport, "t-1", "http://provider.internal:8443/");
  assert_eq!(
    endpoint,
    json!({ "kind": "http", "base-url": "http://provider.internal:8443" })
  );
}

#[test]
fn send_puts_the_recorded_request_on_the_wire_and_returns_the_reply_as_parts() {
  let (base_url, received) = stub_provider(
    "200 OK",
    &[("content-type", "application/json")],
    br#"{"id":"o-1","status":"SHIPPED"}"#,
  );
  let transport = HttpTransport::new();
  start_drive(&transport, "t-1", &base_url);

  let mut headers = BTreeMap::new();
  headers.insert("authorization".to_string(), json!(["Bearer token-1"]));
  let mut query = BTreeMap::new();
  query.insert("expand".to_string(), json!(["items"]));

  let reply = send(
    &transport,
    "t-1",
    request_parts(vec![
      ("method", text_slot("post")),
      ("path", text_slot("/orders")),
      (
        "query",
        SlotValue {
          content: Value::Object(query.into_iter().collect()),
          encoded: None,
          content_type: None,
        },
      ),
      (
        "headers",
        SlotValue {
          content: Value::Object(headers.into_iter().collect()),
          encoded: None,
          content_type: None,
        },
      ),
      (
        "body",
        SlotValue {
          content: Value::String(BASE64.encode(br#"{"id":"o-1"}"#)),
          encoded: Some("base64".to_string()),
          content_type: Some("application/json".to_string()),
        },
      ),
    ]),
    true,
  )
  .expect("await-reply: true returns the reply");

  let sent = received.recv_timeout(Duration::from_secs(5)).unwrap();
  // Method upper-cased, query re-encoded from the slot, and the body's own content type applied
  // from the slot rather than requiring the author to repeat it in `headers`.
  assert_eq!(sent.request_line, "POST /orders?expand=items HTTP/1.1");
  assert!(
    sent
      .headers
      .iter()
      .any(|(n, v)| n.eq_ignore_ascii_case("authorization") && v == "Bearer token-1"),
    "headers sent: {:?}",
    sent.headers
  );
  assert!(
    sent
      .headers
      .iter()
      .any(|(n, v)| n.eq_ignore_ascii_case("content-type") && v == "application/json")
  );
  assert_eq!(sent.body, br#"{"id":"o-1"}"#);

  let response = &reply["response"];
  assert_eq!(response["status"].content, json!(200));
  assert_eq!(
    response["headers"].content["content-type"],
    json!(["application/json"])
  );
  let body = &response["body"];
  assert_eq!(body.encoded.as_deref(), Some("base64"));
  assert_eq!(body.content_type.as_deref(), Some("application/json"));
  assert_eq!(
    BASE64.decode(body.content.as_str().unwrap()).unwrap(),
    br#"{"id":"o-1","status":"SHIPPED"}"#
  );
}

#[test]
fn a_bare_get_needs_no_slots_but_method_and_path() {
  let (base_url, received) = stub_provider("204 No Content", &[], b"");
  let transport = HttpTransport::new();
  start_drive(&transport, "t-1", &base_url);

  let reply = send(
    &transport,
    "t-1",
    request_parts(vec![("method", text_slot("GET")), ("path", text_slot("/health"))]),
    true,
  )
  .expect("await-reply: true returns the reply");

  let sent = received.recv_timeout(Duration::from_secs(5)).unwrap();
  assert_eq!(sent.request_line, "GET /health HTTP/1.1");
  assert!(sent.body.is_empty());
  assert_eq!(reply["response"]["status"].content, json!(204));
}

#[test]
fn a_provider_failure_is_a_reply_not_a_transport_error() {
  let (base_url, _received) = stub_provider(
    "500 Internal Server Error",
    &[("content-type", "text/plain")],
    b"internal error",
  );
  let transport = HttpTransport::new();
  start_drive(&transport, "t-1", &base_url);

  let reply = send(
    &transport,
    "t-1",
    request_parts(vec![
      ("method", text_slot("GET")),
      ("path", text_slot("/orders/66")),
    ]),
    true,
  )
  .expect("a 500 is an observation, not a failure of the machinery");

  assert_eq!(reply["response"]["status"].content, json!(500));
  assert_eq!(
    BASE64
      .decode(reply["response"]["body"].content.as_str().unwrap())
      .unwrap(),
    b"internal error"
  );
}

#[test]
fn await_reply_false_still_sends_but_reports_no_reply() {
  let (base_url, received) = stub_provider("202 Accepted", &[], b"");
  let transport = HttpTransport::new();
  start_drive(&transport, "t-1", &base_url);

  let reply = send(
    &transport,
    "t-1",
    request_parts(vec![
      ("method", text_slot("POST")),
      ("path", text_slot("/events")),
    ]),
    false,
  );
  assert!(reply.is_none());
  // The request still happened — "no reply expected" is about what the caller wants back, not
  // about whether the provider was contacted (spec §5.2).
  let sent = received.recv_timeout(Duration::from_secs(5)).unwrap();
  assert_eq!(sent.request_line, "POST /events HTTP/1.1");
}

#[test]
fn a_drive_instance_refuses_the_serve_roles_operations_by_name() {
  let transport = HttpTransport::new();
  start_drive(&transport, "t-1", "http://127.0.0.1:1");
  let err = transport
    .poll_inbound(PollInbound {
      instance: "t-1".to_string(),
      timeout_ms: 10,
    })
    .expect_err("a driven instance has no inbound side");
  assert_eq!(err.code, "operation-unsupported");
  assert_eq!(
    err.details.as_ref().unwrap()["op"],
    "transport/poll-inbound (drive role)"
  );
}

#[test]
fn a_serve_instance_refuses_send_by_name() {
  let transport = HttpTransport::new();
  transport
    .start(Start {
      instance: "t-1".to_string(),
      kind: "http".to_string(),
      role: "serve".to_string(),
      options: Some(json!({ "host": "127.0.0.1", "port": 0 })),
    })
    .unwrap();
  let err = transport
    .send(Send {
      instance: "t-1".to_string(),
      parts: request_parts(vec![("method", text_slot("GET")), ("path", text_slot("/"))]),
      await_reply: true,
      timeout_ms: None,
    })
    .expect_err("a served instance does not drive");
  assert_eq!(err.code, "operation-unsupported");
  assert_eq!(err.details.as_ref().unwrap()["op"], "transport/send (serve role)");
}

#[test]
fn starting_a_drive_instance_without_a_base_url_says_so() {
  let transport = HttpTransport::new();
  let err = transport
    .start(Start {
      instance: "t-1".to_string(),
      kind: "http".to_string(),
      role: "drive".to_string(),
      options: Some(json!({})),
    })
    .expect_err("there is nowhere to send");
  assert!(err.message.contains("base-url"), "message was: {}", err.message);
}

#[test]
fn an_unknown_role_is_named_in_the_error() {
  let transport = HttpTransport::new();
  let err = transport
    .start(Start {
      instance: "t-1".to_string(),
      kind: "http".to_string(),
      role: "relay".to_string(),
      options: None,
    })
    .expect_err("there are two roles");
  assert_eq!(err.code, "operation-unsupported");
  assert_eq!(
    err.details.as_ref().unwrap()["op"],
    "transport/start (role: relay)"
  );
}

#[test]
fn stopping_a_drive_instance_releases_it() {
  let transport = HttpTransport::new();
  start_drive(&transport, "t-1", "http://127.0.0.1:1");
  transport
    .stop(Stop {
      instance: "t-1".to_string(),
    })
    .unwrap();
  let err = transport
    .send(Send {
      instance: "t-1".to_string(),
      parts: request_parts(vec![("method", text_slot("GET")), ("path", text_slot("/"))]),
      await_reply: true,
      timeout_ms: Some(100),
    })
    .expect_err("the instance is gone");
  assert!(
    err.message.contains("no such instance"),
    "message was: {}",
    err.message
  );
}
