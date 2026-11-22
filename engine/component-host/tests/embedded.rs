//! Embedded content over the WASM binding (component-interfaces spec §6.6, ADR 0103), against a
//! component whose octets hold another type's (`tests/fixtures/wrapping`). It asks for its payload in
//! its answer, and the registry decodes or encodes the payload through whatever handles its type, a
//! native component, another WASM one, or the same one again, before asking it a second time. The
//! component is never called while it is working, so a payload of its own type does not re-enter it.

mod support;

use pact_janus_component_host::WasmLoader;
use pact_janus_component_json::JsonContent;
use pact_janus_kernel::component::{
  ComponentDeclaration, ComponentLoader, ContentComponent, ContentRegistry, Decode, Encode, Grants, Limits,
  SlotValue, Source,
};
use pact_janus_kernel::plan::RuntimeValue;
use pretty_assertions::assert_eq;
use serde_json::{Value, json};
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, OnceLock};
use support::csv_wasm;

/// Build the fixture once per test binary and return its path.
fn fixture() -> String {
  static WASM: OnceLock<String> = OnceLock::new();
  WASM
    .get_or_init(|| {
      let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/wrapping");
      let status = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string()))
        .args(["build", "--release", "--target", "wasm32-wasip2"])
        .current_dir(&dir)
        .env_remove("CARGO_TARGET_DIR")
        .env_remove("RUSTFLAGS")
        .status()
        .expect("cargo runs");
      assert!(status.success(), "building the fixture");
      dir
        .join("target/wasm32-wasip2/release/wrapping.wasm")
        .display()
        .to_string()
    })
    .clone()
}

fn load(name: &str, path: &str) -> Arc<dyn ContentComponent> {
  let declaration = ComponentDeclaration {
    name: name.to_string(),
    source: Source {
      kind: "file".to_string(),
      reference: Some(path.to_string()),
      digest: None,
      auth: None,
      signature: None,
    },
    grants: Grants::default(),
    limits: Limits::default(),
  };
  WasmLoader::new()
    .unwrap()
    .load(&declaration)
    .unwrap_or_else(|err| panic!("{err:?}"))
    .content
    .expect("declared content")
}

/// The wrapping component and the CSV one, both WASM, ahead of the native JSON component.
fn registry() -> ContentRegistry {
  ContentRegistry::new()
    .with(load("wrapping", &fixture()))
    .with(load("csv", &csv_wasm().display().to_string()))
    .with(Arc::new(JsonContent::new()))
}

fn wrapped(text: &str) -> SlotValue {
  SlotValue {
    content: json!(text),
    encoded: Some("text".to_string()),
    content_type: Some("x/wrapped".to_string()),
  }
}

fn decoded(text: &str) -> Value {
  registry()
    .decode(Decode::of("x/wrapped", wrapped(text)))
    .unwrap_or_else(|err| panic!("{err:?}"))
    .document
    .to_json()
}

#[test]
fn a_wasm_component_asks_for_its_payload_and_a_native_component_decodes_it() {
  assert_eq!(
    decoded("application/json\n{\"id\": 1}"),
    json!({ "type": "application/json", "payload": { "id": 1 } })
  );
}

#[test]
fn a_payload_one_wasm_component_holds_is_decoded_by_another() {
  assert_eq!(
    decoded("text/csv\nid,status\n1,PENDING\n"),
    json!({ "type": "text/csv", "payload": [ { "id": "1", "status": "PENDING" } ] })
  );
}

/// The component is asked for the inner payload after it answered the outer request, and asked the
/// outer one again after that: a payload of its own type never re-enters an instance at work.
#[test]
fn a_payload_of_the_components_own_type_is_asked_of_it_again() {
  assert_eq!(
    decoded("x/wrapped\napplication/json\n[true]"),
    json!({ "type": "x/wrapped", "payload": { "type": "application/json", "payload": [true] } })
  );
}

#[test]
fn a_payload_nothing_claims_is_its_octets_and_one_that_does_not_decode_is_an_error_the_component_reads() {
  assert_eq!(
    decoded("image/png\nPNG"),
    json!({ "type": "image/png", "payload": "UE5H", "encoded": "base64" })
  );
  assert_eq!(
    decoded("application/json\n{not json"),
    json!({ "type": "application/json", "failed": "decode-failed" })
  );
}

/// The payload is encoded by the component for its type, given the part of the request's shape at the
/// pointer the component named: an empty CSV payload is written as the header its shape declares
/// (ADR 0101), and decodes back to what was encoded.
#[test]
fn a_payload_is_encoded_by_the_component_for_its_type_with_the_shape_where_it_sits() {
  let document = json!({ "type": "text/csv", "payload": [] });
  let shape = json!({ "shape": "object", "members": {
    "type": { "shape": "equality", "example": "text/csv" },
    "payload": { "shape": "each-like", "min": 0, "items": { "shape": "object", "members": {
      "id": { "shape": "string", "example": "1" } } } } } });
  let encoded = registry()
    .encode(Encode {
      shape: Some(shape),
      ..Encode::of("x/wrapped", RuntimeValue::from_json(&document))
    })
    .unwrap_or_else(|err| panic!("{err:?}"))
    .value;
  assert_eq!(encoded.content_type.as_deref(), Some("x/wrapped"));
  let decoded = registry()
    .decode(Decode::of("x/wrapped", encoded.clone()))
    .unwrap()
    .document
    .to_json();
  assert_eq!(decoded, document, "{encoded:?}");
}
