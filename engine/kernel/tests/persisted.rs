//! Staged plan S0.5, ADR 0031: a renamed engine writes the new spelling of every persisted
//! identifier and still reads documents recorded under an earlier one.
//!
//! Until a rename, `persisted/earlier.rs` is empty and the reading tests below check nothing; the
//! rename rehearsal (`.github/scripts/rename.sh`, run on a schedule) is what gives them spellings
//! to read, and fails if it does not.

use pact_janus_kernel::contract::{self, IdentifyMode};
use pact_janus_kernel::persisted::{self, CONTRACT_FORMAT, PROVIDER_SHAPE_FORMAT};
use pact_janus_kernel::protocol::Engine;
use pact_janus_kernel::subsumption::read_provider_shape;
use serde_json::{Value, json};
use std::path::PathBuf;

fn repo(path: &str) -> PathBuf {
  PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..").join(path)
}

fn document(path: &str) -> Value {
  serde_json::from_slice(&std::fs::read(repo(path)).expect(path)).expect(path)
}

/// The sample consumer's v3 pact, upgraded: a contract as this engine writes it.
fn contract() -> Value {
  let pact = document("samples/order-service/pacts/web-app-order-service.json");
  let upgraded =
    pact_janus_kernel::legacy::upgrade::pact("web-app-order-service.json", &pact).expect("upgrades");
  serde_json::from_slice(&contract::write_canonical(&upgraded.contract).expect("writes")).expect("reads")
}

fn send(engine: &mut Engine, op: &str, body: Value) -> Value {
  let request = json!({ "type": "request", "id": "1", "op": op, "body": body });
  serde_json::from_slice(&engine.dispatch(&serde_json::to_vec(&request).unwrap())).unwrap()
}

/// An engine past its handshake.
fn engine() -> Engine {
  let mut engine = Engine::new();
  let hello = send(
    &mut engine,
    "engine/hello",
    json!({ "protocol-versions": [1], "host": { "name": "persisted-test" } }),
  );
  assert!(hello["ok"].is_object(), "{hello}");
  engine
}

fn explain(engine: &mut Engine, contract: &Value) -> Value {
  send(
    engine,
    "verification/explain",
    json!({ "interaction": { "kind": "contract-interaction", "contract": contract } }),
  )
}

#[test]
fn what_is_written_is_the_current_spelling() {
  assert_eq!(contract()["$format"], CONTRACT_FORMAT.writes());
  assert_eq!(contract::FORMAT, CONTRACT_FORMAT.writes());
  assert_eq!(
    pact_janus_kernel::subsumption::PROVIDER_SHAPE_FORMAT,
    PROVIDER_SHAPE_FORMAT.writes()
  );
}

#[test]
fn a_contract_recorded_under_an_earlier_name_is_still_read() {
  let mut engine = engine();
  let response = explain(&mut engine, &contract());
  assert!(response["ok"].is_object(), "{response}");
  for earlier in CONTRACT_FORMAT.earlier() {
    let mut old = contract();
    old["$format"] = json!(earlier);
    let response = explain(&mut engine, &old);
    assert!(response["ok"].is_object(), "{earlier}: {response}");
    assert!(contract::read(old.to_string().as_bytes(), IdentifyMode::Strict).is_ok());
  }
}

#[test]
fn a_provider_shape_recorded_under_an_earlier_name_is_still_read() {
  let shape = document("samples/order-service/shapes/order-service.provider-shape.json");
  assert!(read_provider_shape(shape.to_string().as_bytes()).is_ok());
  for earlier in PROVIDER_SHAPE_FORMAT.earlier() {
    let mut old = shape.clone();
    old["$format"] = json!(earlier);
    assert!(
      read_provider_shape(old.to_string().as_bytes()).is_ok(),
      "{earlier}"
    );
  }
}

#[test]
fn no_earlier_spelling_is_written() {
  for (name, persisted) in persisted::ALL {
    assert!(!persisted.earlier().contains(&persisted.writes()), "{name}");
  }
}
