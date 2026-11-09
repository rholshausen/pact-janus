//! ADR 0077 on the consumer side: a `content-type` shape on a multipart part's body is matched against
//! the part's own `Content-Type`, the label the multipart component decoded it by, as a whole slot's is
//! matched against its label (ADR 0049). A part with no type of its own is detected from its content.

use pact_janus_component_http::HttpTransport;
use pact_janus_component_json::JsonContent;
use pact_janus_component_multipart::MultipartContent;
use pact_janus_component_text::TextContent;
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
  let request = json!({ "type": "request", "id": "1", "op": op, "body": body });
  serde_json::from_slice(&engine.dispatch(&serde_json::to_vec(&request).unwrap())).unwrap()
}

fn engine() -> Engine {
  let mut transports: HashMap<String, Arc<dyn TransportComponent>> = HashMap::new();
  transports.insert("http".to_string(), Arc::new(HttpTransport::new()));
  let mut engine = Engine::with_components(transports, Some(Arc::new(JsonContent::new())));
  engine.register_background(Arc::new(pact_janus_kernel::protocol::Threads));
  engine.register_content(Arc::new(TextContent::new()));
  engine.register_content(Arc::new(MultipartContent::new()));
  engine.declare_in_tree("content", "text", "1.0.0");
  engine.declare_in_tree("content", "multipart", "1.0.0");
  send(
    &mut engine,
    "engine/hello",
    json!({ "protocol-versions": [1], "host": { "name": "janus-test", "version": "0.0.0" }, "capabilities": {} }),
  );
  engine
}

/// One POST of a multipart body with a single `file` part: the mock's status, and its reply's body.
fn upload(addr: &str, part_type: Option<&str>, content: &str) -> (u16, String) {
  let part_type = part_type
    .map(|t| format!("Content-Type: {t}\r\n"))
    .unwrap_or_default();
  let body = format!(
    "--b\r\nContent-Disposition: form-data; name=\"file\"; filename=\"f\"\r\n{part_type}\r\n{content}\r\n--b--\r\n"
  );
  let mut stream = TcpStream::connect(addr).unwrap();
  stream.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
  stream
    .write_all(
      format!(
        "POST /upload HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\
         Content-Type: multipart/form-data; boundary=b\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
      )
      .as_bytes(),
    )
    .unwrap();
  let mut response = Vec::new();
  let _ = stream.read_to_end(&mut response);
  let response = String::from_utf8(response).unwrap();
  let (head, reply) = response.split_once("\r\n\r\n").unwrap();
  let status = head.split(' ').nth(1).unwrap().parse().unwrap();
  (status, reply.to_string())
}

/// The status of one upload to a mock serving a contract whose `file` part's body is a `content-type`
/// of `expected`, as `janus upgrade` writes a v1–v4 `contentType` rule on a part.
fn served(expected: &str, part_type: Option<&str>, content: &str) -> (u16, String) {
  let mut engine = engine();
  let created = send(
    &mut engine,
    "consumer-session/create",
    json!({ "config": { "consumer": { "name": "c" }, "provider": { "name": "p" } } }),
  );
  let session = created["ok"]["session"].clone();
  let added = send(
    &mut engine,
    "consumer-session/add-interaction",
    json!({ "session": session, "interaction": {
      "description": "an upload",
      "transport": { "kind": "http", "mode": "passive" },
      "content-types": { "request": { "body": "multipart/form-data" } },
      "parts": {
        "request": { "method": { "shape": "equality", "example": "POST" },
                     "path": { "shape": "equality", "example": "/upload" },
                     "body": { "shape": "object", "members": {
                       "file": { "shape": "array", "entries": [{ "shape": "object", "members": {
                         "body": { "shape": "content-type", "content-type": expected, "example": content } } }] } } } },
        "response": { "status": { "shape": "equality", "example": 204 } } } } }),
  );
  let handle = added["ok"]["handle"].clone();
  assert!(handle.is_string(), "{added}");
  send(
    &mut engine,
    "consumer-session/variants",
    json!({ "session": session, "handle": handle }),
  );
  let started = send(
    &mut engine,
    "consumer-session/start-transport",
    json!({ "session": session, "transport": "http" }),
  );
  let endpoint = &started["ok"]["endpoint"];
  let addr = format!("{}:{}", endpoint["host"].as_str().unwrap(), endpoint["port"]);
  send(
    &mut engine,
    "consumer-session/serve-variant",
    json!({ "session": session, "handle": handle, "variant": "base" }),
  );
  upload(&addr, part_type, content)
}

#[test]
fn a_parts_body_is_the_type_its_own_content_type_labels_it() {
  assert_eq!(served("text/plain", Some("text/plain"), "some text").0, 204);
  assert_eq!(
    served("text/plain", Some("text/plain; charset=utf-8"), "some text").0,
    204,
    "compared by essence"
  );
}

#[test]
fn a_part_labelled_another_type_is_refused_whatever_its_content_looks_like() {
  let (status, reply) = served("application/json", Some("text/plain"), "{\"a\":1}");
  assert_eq!(status, 500, "{reply}");
  assert!(reply.contains("$.request.body.file[0].body"), "{reply}");
}

#[test]
fn a_part_with_no_type_of_its_own_is_detected_from_its_content() {
  assert_eq!(served("application/json", None, "{\"a\":1}").0, 204);
  let (status, reply) = served("text/plain", None, "some text");
  assert_eq!(
    status, 500,
    "text is not detected from its octets (component-interfaces spec §6.5): {reply}"
  );
}
