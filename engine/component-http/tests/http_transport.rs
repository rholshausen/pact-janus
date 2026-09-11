//! Plan task 4.2: the built-in HTTP transport component, mock (serve-role) side
//! (component-interfaces spec §5) — proven with a real socket: a hand-rolled HTTP/1.1 client
//! (no second HTTP dependency just for tests) drives requests at a server this component starts
//! through the transport interface, exactly as the worked example in
//! `Documentation/specs/component-interfaces/examples/http-transport-both-bindings.md` describes.

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use pact_janus_component_http::HttpTransport;
use pact_janus_kernel::component::{
  Dispose, Parts, PollInbound, Reply, Send, SlotValue, Start, Stop, TransportComponent,
};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::time::Duration;

fn start_serve(transport: &HttpTransport, instance: &str) -> (String, u16) {
  let result = transport
    .start(Start {
      instance: instance.to_string(),
      kind: "http".to_string(),
      role: "serve".to_string(),
      options: Some(json!({ "host": "127.0.0.1", "port": 0 })),
    })
    .expect("a fresh, OS-assigned port always binds");
  let endpoint = result.endpoint;
  (
    endpoint["host"].as_str().unwrap().to_string(),
    endpoint["port"].as_u64().unwrap() as u16,
  )
}

/// A minimal, blocking HTTP/1.1 client: exactly enough to drive the tests below without a second
/// HTTP dependency. Reads headers line by line, then exactly `Content-Length` body bytes.
struct RawResponse {
  status: u16,
  headers: BTreeMap<String, String>,
  body: Vec<u8>,
}

fn raw_http_request(
  host: &str,
  port: u16,
  method: &str,
  path: &str,
  headers: &[(&str, &str)],
  body: &[u8],
) -> RawResponse {
  let mut stream = TcpStream::connect((host, port)).expect("mock server is listening");
  let mut request = format!("{method} {path} HTTP/1.1\r\nHost: {host}:{port}\r\n");
  for (name, value) in headers {
    request.push_str(&format!("{name}: {value}\r\n"));
  }
  request.push_str(&format!("Content-Length: {}\r\n\r\n", body.len()));
  stream.write_all(request.as_bytes()).unwrap();
  stream.write_all(body).unwrap();
  stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();

  let mut reader = BufReader::new(stream);
  let mut status_line = String::new();
  reader.read_line(&mut status_line).expect("a status line");
  let status: u16 = status_line
    .split_whitespace()
    .nth(1)
    .and_then(|code| code.parse().ok())
    .expect("a numeric status code");

  let mut headers = BTreeMap::new();
  loop {
    let mut line = String::new();
    reader
      .read_line(&mut line)
      .expect("a header line or the blank line ending headers");
    let line = line.trim_end();
    if line.is_empty() {
      break;
    }
    if let Some((name, value)) = line.split_once(':') {
      headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_string());
    }
  }

  let content_length: usize = headers
    .get("content-length")
    .and_then(|v| v.parse().ok())
    .unwrap_or(0);
  let mut body = vec![0u8; content_length];
  if content_length > 0 {
    reader
      .read_exact(&mut body)
      .expect("exactly content-length body bytes");
  }
  RawResponse {
    status,
    headers,
    body,
  }
}

fn response_parts(status: u16, content_type: &str, body: &[u8]) -> Parts {
  let mut response_part = pact_janus_kernel::component::Part::new();
  response_part.insert(
    "status".to_string(),
    SlotValue {
      content: json!(status),
      encoded: None,
      content_type: None,
    },
  );
  response_part.insert(
    "headers".to_string(),
    SlotValue {
      content: json!({ "content-type": [content_type] }),
      encoded: None,
      content_type: None,
    },
  );
  response_part.insert(
    "body".to_string(),
    SlotValue {
      content: Value::String(BASE64.encode(body)),
      encoded: Some("base64".to_string()),
      content_type: Some(content_type.to_string()),
    },
  );
  let mut parts = Parts::new();
  parts.insert("response".to_string(), response_part);
  parts
}

#[test]
fn serves_a_request_and_replies() {
  let transport = HttpTransport::new();
  let (host, port) = start_serve(&transport, "t-1");

  let client = std::thread::spawn(move || {
    raw_http_request(
      &host,
      port,
      "POST",
      "/orders",
      &[("Content-Type", "application/json")],
      br#"{"id":"o-1"}"#,
    )
  });

  let polled = transport
    .poll_inbound(PollInbound {
      instance: "t-1".to_string(),
      timeout_ms: 2000,
    })
    .expect("no transport error")
    .inbound
    .expect("the client's request arrived within the timeout");
  assert!(polled.expects_reply);

  let request = &polled.parts["request"];
  assert_eq!(request["method"].content, json!("POST"));
  assert_eq!(request["path"].content, json!("/orders"));
  assert_eq!(
    request["headers"].content["content-type"],
    json!(["application/json"]),
    "headers.content: {}",
    request["headers"].content
  );
  let body = &request["body"];
  assert_eq!(body.encoded.as_deref(), Some("base64"));
  assert_eq!(body.content_type.as_deref(), Some("application/json"));
  assert_eq!(
    BASE64.decode(body.content.as_str().unwrap()).unwrap(),
    br#"{"id":"o-1"}"#
  );

  transport
    .reply(Reply {
      instance: "t-1".to_string(),
      event: polled.event,
      parts: response_parts(201, "application/json", br#"{"id":"o-1","status":"new"}"#),
    })
    .expect("the arrival is still pending a reply");

  let response = client.join().unwrap();
  assert_eq!(response.status, 201);
  assert_eq!(
    response.headers.get("content-type").map(String::as_str),
    Some("application/json")
  );
  assert_eq!(response.body, br#"{"id":"o-1","status":"new"}"#);

  transport
    .stop(Stop {
      instance: "t-1".to_string(),
    })
    .unwrap();
}

#[test]
fn query_and_repeated_headers_are_grouped() {
  let transport = HttpTransport::new();
  let (host, port) = start_serve(&transport, "t-2");

  let client = std::thread::spawn(move || {
    raw_http_request(&host, port, "GET", "/orders?status=new&status=open", &[], b"")
  });

  let polled = transport
    .poll_inbound(PollInbound {
      instance: "t-2".to_string(),
      timeout_ms: 2000,
    })
    .unwrap()
    .inbound
    .unwrap();
  let request = &polled.parts["request"];
  assert_eq!(request["path"].content, json!("/orders"));
  assert_eq!(request["query"].content, json!({ "status": ["new", "open"] }));

  transport
    .reply(Reply {
      instance: "t-2".to_string(),
      event: polled.event,
      parts: response_parts(200, "application/json", b"{}"),
    })
    .unwrap();
  client.join().unwrap();
  transport
    .stop(Stop {
      instance: "t-2".to_string(),
    })
    .unwrap();
}

#[test]
fn dispose_without_a_reply_still_completes_the_connection() {
  let transport = HttpTransport::new();
  let (host, port) = start_serve(&transport, "t-3");

  let client = std::thread::spawn(move || raw_http_request(&host, port, "GET", "/health", &[], b""));

  let polled = transport
    .poll_inbound(PollInbound {
      instance: "t-3".to_string(),
      timeout_ms: 2000,
    })
    .unwrap()
    .inbound
    .unwrap();

  transport
    .dispose(Dispose {
      instance: "t-3".to_string(),
      event: polled.event,
      disposition: "reject".to_string(),
    })
    .unwrap();

  // The point of the test: the client does not hang waiting for a reply that will never come.
  let response = client.join().unwrap();
  assert_eq!(response.status, 500);

  transport
    .stop(Stop {
      instance: "t-3".to_string(),
    })
    .unwrap();
}

#[test]
fn poll_inbound_times_out_with_no_arrival() {
  let transport = HttpTransport::new();
  let (_, _) = start_serve(&transport, "t-4");
  let polled = transport
    .poll_inbound(PollInbound {
      instance: "t-4".to_string(),
      timeout_ms: 50,
    })
    .unwrap();
  assert!(polled.inbound.is_none());
  transport
    .stop(Stop {
      instance: "t-4".to_string(),
    })
    .unwrap();
}

#[test]
fn drive_role_is_not_implemented_by_this_task() {
  let transport = HttpTransport::new();
  let err = transport
    .start(Start {
      instance: "t-5".to_string(),
      kind: "http".to_string(),
      role: "drive".to_string(),
      options: None,
    })
    .expect_err("mock-side only (plan task 4.2); drive role is Phase 5");
  assert_eq!(err.code, "operation-unsupported");

  let err = transport
    .send(Send {
      instance: "t-5".to_string(),
      parts: Parts::new(),
      await_reply: false,
      timeout_ms: None,
    })
    .expect_err("send is drive-only");
  assert_eq!(err.code, "operation-unsupported");
}
