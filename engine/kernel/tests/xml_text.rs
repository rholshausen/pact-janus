//! ADR 0075 (staged plan S2.10): a contract's XML text is its text as written. A mock serves it with
//! the whitespace around it, and a request's CDATA section matches a contract's text by its contents
//! as written, which a request writing the same characters trimmed does not.

use pact_janus_component_http::HttpTransport;
use pact_janus_component_json::JsonContent;
use pact_janus_component_xml::XmlContent;
use pact_janus_kernel::component::TransportComponent;
use pact_janus_kernel::protocol::{Engine, Threads};
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

fn engine() -> Engine {
  let mut transports: HashMap<String, Arc<dyn TransportComponent>> = HashMap::new();
  transports.insert("http".to_string(), Arc::new(HttpTransport::new()));
  let mut engine = Engine::with_components(transports, Some(Arc::new(JsonContent::new())));
  engine.register_content(Arc::new(XmlContent::new()));
  engine.declare_in_tree("content", "xml", "1.0.0");
  engine.register_background(Arc::new(Threads));
  send(
    &mut engine,
    "h-1",
    "engine/hello",
    json!({ "protocol-versions": [1], "host": { "name": "janus-test", "version": "0.0.0" }, "capabilities": {} }),
  );
  engine
}

/// One HTTP exchange: its status and its body.
fn exchange(addr: &str, method: &str, body: &str) -> (u16, String) {
  let mut stream = TcpStream::connect(addr).expect("the mock is listening");
  stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
  let request = format!(
    "{method} /note HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\nContent-Type: application/xml\r\n\
     Content-Length: {}\r\n\r\n{body}",
    body.len()
  );
  stream.write_all(request.as_bytes()).unwrap();
  let mut response = String::new();
  let _ = stream.read_to_string(&mut response);
  let status = response
    .split_whitespace()
    .nth(1)
    .and_then(|status| status.parse().ok())
    .expect("a status line");
  let body = response
    .split_once("\r\n\r\n")
    .map(|(_, body)| body.to_string())
    .unwrap_or_default();
  (status, body)
}

/// A text element: `{ <name>: [ { "#text": <text> } ] }`'s shape, its text an `equality`.
fn text_element(text: &str) -> Value {
  json!({ "shape": "array", "entries": [
    { "shape": "object", "members": { "#text": { "shape": "equality", "example": text } } }
  ] })
}

/// A session with one interaction armed: `request` a `method` to `/note` whose body is `request_body`
/// (when it has one), answered with `response_body`. Answers the engine, the session and the mock's
/// address.
fn armed(method: &str, request_body: Option<Value>, response_body: Value) -> (Engine, String, String) {
  let mut engine = engine();
  let created = send(
    &mut engine,
    "c-1",
    "consumer-session/create",
    json!({ "config": { "consumer": { "name": "web-app" }, "provider": { "name": "notes" } } }),
  );
  let session = created["ok"]["session"]
    .as_str()
    .unwrap_or_else(|| panic!("{created}"))
    .to_string();
  let mut request = json!({
    "method": { "shape": "equality", "example": method },
    "path": { "shape": "equality", "example": "/note" },
  });
  let mut content_types = json!({ "response": { "body": "application/xml" } });
  if let Some(body) = request_body {
    request["body"] = body;
    content_types["request"] = json!({ "body": "application/xml" });
  }
  let interaction = json!({
    "description": "a note",
    "transport": { "kind": "http", "mode": "passive" },
    "requires": [{ "component": "content/xml" }],
    "content-types": content_types,
    "parts": {
      "request": request,
      "response": { "status": { "shape": "equality", "example": 200 }, "body": response_body } }
  });
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
  let served = send(
    &mut engine,
    "c-5",
    "consumer-session/serve-variant",
    json!({ "session": session, "handle": handle, "variant": "base" }),
  );
  assert!(served.get("ok").is_some(), "{served}");
  (engine, session, addr)
}

fn note(members: Value) -> Value {
  json!({ "shape": "object", "members": { "note": { "shape": "object", "members": members } } })
}

#[test]
fn a_mock_serves_xml_text_as_the_contract_writes_it() {
  let (_engine, _session, addr) = armed(
    "GET",
    None,
    note(json!({ "to": text_element("  Ada  "), "script": text_element(" if (a < b) { go(); } ") })),
  );
  let (status, body) = exchange(&addr, "GET", "");
  assert_eq!(status, 200, "{body}");
  assert!(body.contains("<to>  Ada  </to>"), "{body}");
  assert!(
    body.contains("<script> if (a &lt; b) { go(); } </script>"),
    "{body}"
  );
}

#[test]
fn a_cdata_section_matches_text_written_as_its_contents_are() {
  let request = note(json!({ "script": text_element("  if (a < b) { go(); }  ") }));
  let response = note(json!({}));

  let (_engine, _session, addr) = armed("POST", Some(request.clone()), response.clone());
  let (status, body) = exchange(
    &addr,
    "POST",
    "<note><script><![CDATA[  if (a < b) { go(); }  ]]></script></note>",
  );
  assert_eq!(status, 200, "{body}");

  let (_engine, _session, addr) = armed("POST", Some(request), response);
  let (status, body) = exchange(
    &addr,
    "POST",
    "<note><script><![CDATA[if (a < b) { go(); }]]></script></note>",
  );
  assert_eq!(status, 500, "the same text trimmed is a mismatch: {body}");
  assert!(body.contains("#text"), "{body}");
}
