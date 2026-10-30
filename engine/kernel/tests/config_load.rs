//! `config/load` (engine-protocol spec §8.8, ADR 0055): an engine with a registered loader answers
//! with what the loader resolved and says so in the handshake; one without answers by name.

use pact_janus_kernel::hooks::ConfigLoader;
use pact_janus_kernel::protocol::{Engine, EngineError};
use pretty_assertions::assert_eq;
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};

fn send(engine: &mut Engine, id: &str, op: &str, body: Value) -> Value {
  let request = json!({ "type": "request", "id": id, "op": op, "body": body });
  let bytes = engine.dispatch(&serde_json::to_vec(&request).unwrap());
  serde_json::from_slice(&bytes).unwrap()
}

fn hello(engine: &mut Engine) -> Value {
  send(
    engine,
    "r-0",
    "engine/hello",
    json!({ "protocol-versions": [1], "host": { "name": "t", "version": "0" }, "capabilities": {} }),
  )
}

/// Answers every path with one document, or with one error, and remembers what it was asked.
struct FakeLoader {
  answer: Result<Value, EngineError>,
  asked: Mutex<Vec<String>>,
}

impl ConfigLoader for FakeLoader {
  fn load(&self, path: &str) -> Result<Value, EngineError> {
    self.asked.lock().unwrap().push(path.to_string());
    self.answer.clone()
  }
}

#[test]
fn an_engine_with_a_loader_says_so_in_the_handshake() {
  let mut with = Engine::new();
  with.register_config_loader(Arc::new(FakeLoader {
    answer: Ok(json!({})),
    asked: Mutex::default(),
  }));
  assert_eq!(
    hello(&mut with)["ok"]["capabilities"]["config-loading"],
    json!({})
  );
  let without = hello(&mut Engine::new());
  assert!(
    without["ok"]["capabilities"].get("config-loading").is_none(),
    "{without}"
  );
}

#[test]
fn the_answer_is_what_the_loader_resolved() {
  let resolved = json!({ "version": 1, "hooks": { "state-setup": [] }, "components": [] });
  let loader = Arc::new(FakeLoader {
    answer: Ok(resolved.clone()),
    asked: Mutex::default(),
  });
  let mut engine = Engine::new();
  engine.register_config_loader(loader.clone());
  hello(&mut engine);
  let answered = send(
    &mut engine,
    "r-1",
    "config/load",
    json!({ "path": "/project/verifier.janus.yaml" }),
  );
  assert_eq!(answered["ok"], json!({ "config": resolved }));
  assert_eq!(
    *loader.asked.lock().unwrap(),
    vec!["/project/verifier.janus.yaml"]
  );
}

#[test]
fn a_loader_error_is_the_answer() {
  let mut engine = Engine::new();
  engine.register_config_loader(Arc::new(FakeLoader {
    answer: Err(EngineError::hook_unresolved(
      "environment variable 'TOKEN' is not set",
      json!({ "variable": "TOKEN" }),
      Vec::new(),
    )),
    asked: Mutex::default(),
  }));
  hello(&mut engine);
  let answered = send(&mut engine, "r-1", "config/load", json!({ "path": "/p.yaml" }));
  assert_eq!(answered["error"]["code"], "hook-unresolved");
  assert_eq!(answered["error"]["category"], "document");
  assert_eq!(answered["error"]["details"], json!({ "variable": "TOKEN" }));
}

#[test]
fn an_engine_without_a_loader_answers_by_name() {
  let mut engine = Engine::new();
  hello(&mut engine);
  let answered = send(&mut engine, "r-1", "config/load", json!({ "path": "/p.yaml" }));
  assert_eq!(answered["error"]["code"], "component-unavailable");
  assert_eq!(answered["error"]["details"]["component"], "config-loader");
}

#[test]
fn a_body_with_no_path_is_malformed() {
  let mut engine = Engine::new();
  hello(&mut engine);
  let answered = send(&mut engine, "r-1", "config/load", json!({}));
  assert_eq!(answered["error"]["code"], "malformed-frame");
}
