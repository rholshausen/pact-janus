//! ADR 0065: `finalise`'s `write` has the engine's host write the contract file, merging into what
//! it holds. Interactions join by identity (contract-file spec §4.2) and are written in identity
//! order, so sessions merging into one file write what one session recording them all would, in
//! whichever order they came; an interaction two sessions recorded differently is a conflict, and
//! the file is left as it was.
//!
//! The file here is a map in memory: the writer's own file and lock are `cli`'s, and tested there.

use pact_janus_component_http::HttpTransport;
use pact_janus_component_json::JsonContent;
use pact_janus_kernel::component::TransportComponent;
use pact_janus_kernel::contract::files::{ContractWriter, Next};
use pact_janus_kernel::protocol::{Engine, EngineError};
use pretty_assertions::assert_eq;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::time::Duration;

const PATH: &str = "/contracts/web-app-orders-api.janus.json";

/// Files in memory, each rewritten whole under the map's lock.
#[derive(Default)]
struct Files(Mutex<HashMap<String, Vec<u8>>>);

impl Files {
  fn read(&self, path: &str) -> Option<String> {
    let files = self.0.lock().unwrap();
    files
      .get(path)
      .map(|bytes| String::from_utf8(bytes.clone()).unwrap())
  }

  fn put(&self, path: &str, text: &str) {
    self
      .0
      .lock()
      .unwrap()
      .insert(path.to_string(), text.as_bytes().to_vec());
  }
}

/// The writer an engine is given: the test's files, shared.
struct Writer(Arc<Files>);

impl ContractWriter for Writer {
  fn update(&self, path: &str, next: &mut Next<'_>) -> Result<(), EngineError> {
    let mut files = self.0.0.lock().unwrap();
    let existing = files.get(path).cloned().unwrap_or_default();
    files.insert(path.to_string(), next(&existing)?);
    Ok(())
  }
}

fn send(engine: &mut Engine, op: &str, body: Value) -> Value {
  let request = json!({ "type": "request", "id": "r", "op": op, "body": body });
  let bytes = engine.dispatch(&serde_json::to_vec(&request).unwrap());
  serde_json::from_slice(&bytes).unwrap()
}

fn engine(files: Option<&Arc<Files>>) -> (Engine, Value) {
  let mut transports: HashMap<String, Arc<dyn TransportComponent>> = HashMap::new();
  transports.insert("http".to_string(), Arc::new(HttpTransport::new()));
  let mut engine = Engine::with_components(transports, Some(Arc::new(JsonContent::new())));
  engine.register_background(Arc::new(pact_janus_kernel::protocol::Threads));
  if let Some(files) = files {
    engine.register_contract_writer(Arc::new(Writer(Arc::clone(files))));
  }
  let hello = send(
    &mut engine,
    "engine/hello",
    json!({ "protocol-versions": [1], "host": { "name": "janus-test", "version": "0.0.0" } }),
  );
  (engine, hello)
}

/// An interaction answering `GET <path>` with `status`.
fn interaction(description: &str, path: &str, status: u16) -> Value {
  json!({
    "description": description,
    "transport": { "kind": "http", "mode": "passive" },
    "parts": {
      "request": { "method": { "shape": "equality", "example": "GET" },
                   "path": { "shape": "equality", "example": path } },
      "response": { "status": { "shape": "equality", "example": status } } }
  })
}

/// One GET, answering its status.
fn get(addr: &str, path: &str) -> u16 {
  let mut stream = TcpStream::connect(addr).unwrap();
  stream.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
  let request = format!("GET {path} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n");
  stream.write_all(request.as_bytes()).unwrap();
  let mut response = Vec::new();
  let _ = stream.read_to_end(&mut response);
  let head = String::from_utf8_lossy(&response);
  head
    .split_whitespace()
    .nth(1)
    .and_then(|s| s.parse().ok())
    .unwrap()
}

/// A session recording `interactions` (each exercised unless `skip` names it), finalised with `write`.
fn record(
  files: &Arc<Files>,
  consumer: &str,
  interactions: &[Value],
  skip: Option<&str>,
  write: Value,
) -> Value {
  let (mut engine, _) = engine(Some(files));
  let created = send(
    &mut engine,
    "consumer-session/create",
    json!({ "config": { "consumer": { "name": consumer }, "provider": { "name": "orders-api" } } }),
  );
  let session = created["ok"]["session"].as_str().unwrap().to_string();
  let started = send(
    &mut engine,
    "consumer-session/start-transport",
    json!({ "session": session, "transport": "http" }),
  );
  let endpoint = &started["ok"]["endpoint"];
  let addr = format!("{}:{}", endpoint["host"].as_str().unwrap(), endpoint["port"]);
  for spec in interactions {
    let added = send(
      &mut engine,
      "consumer-session/add-interaction",
      json!({ "session": session, "interaction": spec }),
    );
    let handle = added["ok"]["handle"].as_str().unwrap().to_string();
    let selected = send(
      &mut engine,
      "consumer-session/variants",
      json!({ "session": session, "handle": handle }),
    );
    if skip == spec["description"].as_str() {
      continue;
    }
    for variant in selected["ok"]["variants"].as_array().unwrap() {
      send(
        &mut engine,
        "consumer-session/serve-variant",
        json!({ "session": session, "handle": handle, "variant": variant["id"] }),
      );
      let path = spec["parts"]["request"]["path"]["example"].as_str().unwrap();
      get(&addr, path);
    }
  }
  send(
    &mut engine,
    "consumer-session/finalise",
    json!({ "session": session, "write": write }),
  )
}

fn merge() -> Value {
  json!({ "path": PATH })
}

#[test]
fn the_engine_declares_contract_writing_when_its_embedding_gave_it_a_writer() {
  let (_, without) = engine(None);
  assert_eq!(without["ok"]["capabilities"].get("contract-writing"), None);
  let (_, with) = engine(Some(&Arc::default()));
  assert_eq!(with["ok"]["capabilities"]["contract-writing"], json!({}));
}

#[test]
fn sessions_merged_into_one_file_write_what_one_session_recording_them_all_would() {
  let (get_order, delete_order, list_orders) = (
    interaction("get an order", "/orders/1", 200),
    interaction("delete an order", "/orders/1/delete", 204),
    interaction("list orders", "/orders", 200),
  );
  let one = Arc::<Files>::default();
  let answered = record(
    &one,
    "web-app",
    &[list_orders.clone(), get_order.clone(), delete_order.clone()],
    None,
    merge(),
  );
  assert_eq!(answered["ok"]["written"], PATH, "{answered}");
  assert_eq!(answered["ok"].get("contract-text"), None);

  let files = Arc::<Files>::default();
  record(&files, "web-app", std::slice::from_ref(&get_order), None, merge());
  record(
    &files,
    "web-app",
    &[delete_order.clone(), list_orders.clone()],
    None,
    merge(),
  );
  let other_way = Arc::<Files>::default();
  record(&other_way, "web-app", &[list_orders, delete_order], None, merge());
  record(&other_way, "web-app", &[get_order], None, merge());

  let written = one.read(PATH).unwrap();
  assert_eq!(files.read(PATH).unwrap(), written);
  assert_eq!(other_way.read(PATH).unwrap(), written);
  let contract: Value = serde_json::from_str(&written).unwrap();
  let descriptions: Vec<&str> = contract["interactions"]
    .as_array()
    .unwrap()
    .iter()
    .map(|i| i["description"].as_str().unwrap())
    .collect();
  assert_eq!(
    descriptions,
    vec!["delete an order", "get an order", "list orders"]
  );
}

#[test]
fn an_interaction_recorded_the_same_twice_is_recorded_once() {
  let files = Arc::<Files>::default();
  record(
    &files,
    "web-app",
    &[interaction("get an order", "/orders/1", 200)],
    None,
    merge(),
  );
  let first = files.read(PATH).unwrap();
  record(
    &files,
    "web-app",
    &[interaction("get an order", "/orders/1", 200)],
    None,
    merge(),
  );
  assert_eq!(files.read(PATH).unwrap(), first);
}

#[test]
fn an_interaction_recorded_differently_is_a_conflict_naming_it_and_the_file_is_kept() {
  let files = Arc::<Files>::default();
  record(
    &files,
    "web-app",
    &[interaction("get an order", "/orders/1", 200)],
    None,
    merge(),
  );
  let first = files.read(PATH).unwrap();
  let answered = record(
    &files,
    "web-app",
    &[
      interaction("get an order", "/orders/1", 404),
      interaction("list orders", "/orders", 200),
    ],
    None,
    merge(),
  );
  let err = &answered["error"];
  assert_eq!(err["code"], "contract-conflict", "{answered}");
  assert_eq!(err["category"], "document");
  assert_eq!(
    err["details"],
    json!({ "path": PATH, "interactions": [{ "description": "get an order" }] })
  );
  assert!(
    err["message"].as_str().unwrap().contains("'get an order'"),
    "{err}"
  );
  assert_eq!(files.read(PATH).unwrap(), first);
}

#[test]
fn another_pairs_contract_in_the_file_is_a_conflict() {
  let files = Arc::<Files>::default();
  record(
    &files,
    "mobile-app",
    &[interaction("get an order", "/orders/1", 200)],
    None,
    merge(),
  );
  let answered = record(
    &files,
    "web-app",
    &[interaction("get an order", "/orders/1", 200)],
    None,
    merge(),
  );
  assert_eq!(answered["error"]["code"], "contract-conflict", "{answered}");
  assert_eq!(answered["error"]["details"]["consumer"], "mobile-app");
}

#[test]
fn a_withheld_contract_writes_nothing() {
  let files = Arc::<Files>::default();
  files.put(PATH, "what another test file wrote");
  let answered = record(
    &files,
    "web-app",
    &[
      interaction("get an order", "/orders/1", 200),
      interaction("list orders", "/orders", 200),
    ],
    Some("list orders"),
    merge(),
  );
  assert_eq!(answered["ok"].get("written"), None, "{answered}");
  assert_eq!(files.read(PATH).unwrap(), "what another test file wrote");
}

#[test]
fn overwrite_writes_the_sessions_contract_whatever_the_file_held() {
  let files = Arc::<Files>::default();
  record(
    &files,
    "web-app",
    &[interaction("get an order", "/orders/1", 200)],
    None,
    merge(),
  );
  record(
    &files,
    "web-app",
    &[interaction("get an order", "/orders/1", 404)],
    None,
    json!({ "path": PATH, "mode": "overwrite" }),
  );
  let contract: Value = serde_json::from_str(&files.read(PATH).unwrap()).unwrap();
  assert_eq!(
    contract["interactions"][0]["parts"]["response"]["status"]["example"],
    404
  );
}

#[test]
fn a_file_that_is_not_a_contract_fails_as_reading_one_does_with_its_path() {
  let files = Arc::<Files>::default();
  files.put(PATH, "{\"consumer\":{\"name\":\"web-app\"}}");
  let answered = record(
    &files,
    "web-app",
    &[interaction("get an order", "/orders/1", 200)],
    None,
    merge(),
  );
  assert_eq!(answered["error"]["code"], "contract-invalid", "{answered}");
  assert_eq!(answered["error"]["details"]["path"], PATH);
  assert_eq!(files.read(PATH).unwrap(), "{\"consumer\":{\"name\":\"web-app\"}}");
}

#[test]
fn an_empty_file_is_written_as_new() {
  let files = Arc::<Files>::default();
  files.put(PATH, "\n");
  let answered = record(
    &files,
    "web-app",
    &[interaction("get an order", "/orders/1", 200)],
    None,
    merge(),
  );
  assert_eq!(answered["ok"]["written"], PATH, "{answered}");
}

#[test]
fn a_write_with_no_writer_is_component_unavailable_and_still_ends_the_session() {
  let (mut engine, _) = engine(None);
  let created = send(
    &mut engine,
    "consumer-session/create",
    json!({ "config": { "consumer": { "name": "web-app" }, "provider": { "name": "orders-api" } } }),
  );
  let session = created["ok"]["session"].clone();
  let answered = send(
    &mut engine,
    "consumer-session/finalise",
    json!({ "session": session, "write": merge() }),
  );
  assert_eq!(answered["error"]["code"], "component-unavailable");
  assert_eq!(answered["error"]["details"]["component"], "contract-writer");
  let again = send(
    &mut engine,
    "consumer-session/finalise",
    json!({ "session": session }),
  );
  assert_eq!(again["error"]["code"], "session-not-found");
}

#[test]
fn an_unknown_mode_is_malformed() {
  let files = Arc::<Files>::default();
  let answered = record(
    &files,
    "web-app",
    &[interaction("get an order", "/orders/1", 200)],
    None,
    json!({ "path": PATH, "mode": "append" }),
  );
  assert_eq!(answered["error"]["code"], "malformed-frame", "{answered}");
}
