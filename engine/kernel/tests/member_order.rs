//! Staged plan S1.20, ADR 0037: an object's members keep the order they arrived in, and nothing the
//! engine decides depends on it. Each test states one half of that: order carried (a generated
//! document, a recorded contract, a decoded value), or order not read (a plan, a variant space, a
//! comparison, a v1–v4 pact's plan).

use pact_janus_kernel::contract::{self, Contract};
use pact_janus_kernel::interaction_spec::{InteractionSpec, parse};
use pact_janus_kernel::legacy::compile::{LegacyBody, LegacyResponse, compile_response};
use pact_janus_kernel::plan::{
  Assignment, CapturedValues, RuntimeValue, Status, compile, execute, outcome, render_pretty, variant_space,
};
use pact_janus_kernel::shape;
use pact_janus_kernel::variant::generate;
use pact_models::matchingrules::MatchingRules;
use pretty_assertions::assert_eq;
use serde_json::{Value, json};
use std::collections::BTreeMap;

/// A response body whose members are declared out of name order, with a nested object and an
/// object example that are too.
fn body(members: Value) -> InteractionSpec {
  parse(&json!({
    "description": "an order",
    "parts": { "response": { "body": { "shape": "object", "members": members } } }
  }))
  .unwrap_or_else(|err| panic!("expected a well-formed spec, got {:?}", err.problems))
}

fn declared() -> Value {
  json!({
    "status": { "shape": "any-of", "options": ["PENDING", "SHIPPED"], "example": "PENDING" },
    "id": { "shape": "integer", "example": 66 },
    "shipping": { "shape": "object", "members": {
      "street": { "shape": "string", "example": "1 High St" },
      "city": { "shape": "string", "example": "London" } } },
    "meta": { "shape": "equality", "example": { "zeta": true, "alpha": false } },
    "note": { "shape": "optional", "of": { "shape": "string", "example": "leave at door" } }
  })
}

/// The same members, written by name.
fn by_name() -> Value {
  json!({
    "id": { "shape": "integer", "example": 66 },
    "meta": { "shape": "equality", "example": { "zeta": true, "alpha": false } },
    "note": { "shape": "optional", "of": { "shape": "string", "example": "leave at door" } },
    "shipping": { "shape": "object", "members": {
      "city": { "shape": "string", "example": "London" },
      "street": { "shape": "string", "example": "1 High St" } } },
    "status": { "shape": "any-of", "options": ["PENDING", "SHIPPED"], "example": "PENDING" }
  })
}

fn names(value: &Value) -> Vec<&str> {
  value
    .as_object()
    .expect("an object")
    .keys()
    .map(String::as_str)
    .collect()
}

#[test]
fn a_generated_document_has_its_members_in_the_order_the_shape_declares_them() {
  let parts = generate::interaction(&body(declared()), &Assignment::new());
  let generated = &parts["response"]["body"];
  assert_eq!(names(generated), ["status", "id", "shipping", "meta", "note"]);
  assert_eq!(names(&generated["shipping"]), ["street", "city"]);
  assert_eq!(
    names(&generated["meta"]),
    ["zeta", "alpha"],
    "an example is a document, served as it was written"
  );
}

#[test]
fn a_plan_is_the_same_whatever_order_the_shape_declares_its_members_in() {
  let declared = compile(&body(declared()), &Assignment::new(), None);
  let by_name = compile(&body(by_name()), &Assignment::new(), None);
  assert_eq!(render_pretty(&declared), render_pretty(&by_name));
}

#[test]
fn a_variant_space_is_the_same_whatever_order_the_shape_declares_its_members_in() {
  let ids = |spec: &InteractionSpec| -> Vec<String> {
    variant_space(spec)
      .dimensions
      .iter()
      .map(|dimension| dimension.id.clone())
      .collect()
  };
  let declared = ids(&body(declared()));
  assert_eq!(declared, ids(&body(by_name())));
  assert_eq!(
    declared,
    ["response.body.note#presence", "response.body.status#value"],
    "dimensions in the lexicographic order shape spec §6.2 requires"
  );
}

#[test]
fn objects_with_the_same_members_in_another_order_are_equal() {
  let spec = parse(&json!({
    "description": "an order",
    "parts": { "response": { "body": { "shape": "equality", "example": { "b": 1, "a": [{ "y": 2, "x": 1 }] } } } }
  }))
  .unwrap();
  let plan = compile(&spec, &Assignment::new(), None);
  let values = CapturedValues::from_json(&BTreeMap::from([(
    "$.response.body".to_string(),
    json!({ "a": [{ "x": 1, "y": 2 }], "b": 1 }),
  )]));
  let (status, mismatches) = outcome(&execute(&plan, &values));
  assert_eq!(
    status,
    Status::Matched,
    "member order is carried, never compared: {mismatches:?}"
  );
}

#[test]
fn a_decoded_document_keeps_its_members_in_the_order_they_arrived() {
  let value = RuntimeValue::from_json(&json!({ "zeta": 1, "alpha": { "y": 2, "x": 1 } }));
  assert_eq!(value.to_json().to_string(), r#"{"zeta":1,"alpha":{"y":2,"x":1}}"#);
}

/// A v1–v4 response whose JSON body is `body`, with no matching rules.
fn legacy_response(body: Value) -> LegacyResponse {
  LegacyResponse {
    status: 200,
    headers: BTreeMap::from([("Content-Type".to_string(), "application/json".to_string())]),
    body: Some(LegacyBody::Document(RuntimeValue::from_json(&body))),
    content_type: Some("application/json".to_string()),
    matching_rules: MatchingRules::default(),
  }
}

#[test]
fn a_pacts_plan_is_the_same_whatever_order_its_body_is_written_in() {
  let written = compile_response(&legacy_response(
    json!({ "status": "PENDING", "id": 66, "lines": [{ "sku": "A", "qty": 1 }] }),
  ));
  let sorted = compile_response(&legacy_response(
    json!({ "id": 66, "lines": [{ "qty": 1, "sku": "A" }], "status": "PENDING" }),
  ));
  assert_eq!(render_pretty(&written), render_pretty(&sorted));
}

#[test]
fn the_legacy_compiler_takes_any_document_a_content_component_decodes() {
  // Bytes are a document kind: what a binary body is, and what nothing in JSON can be.
  let response = LegacyResponse {
    status: 200,
    headers: BTreeMap::new(),
    body: Some(LegacyBody::Document(RuntimeValue::Bytes(vec![0, 1, 2]))),
    content_type: None,
    matching_rules: MatchingRules::default(),
  };
  let plan = render_pretty(&compile_response(&response));
  assert!(plan.contains("BYTES(3, AAEC)"), "{plan}");
}

/// A recorded contract around one interaction whose response body is `shape`.
fn contract_with(shape: Value) -> Contract {
  serde_json::from_value(json!({
    "$format": "janus-contract/1",
    "consumer": { "name": "web-app" },
    "provider": { "name": "order-api" },
    "interactions": [{
      "description": "an order",
      "parts": { "response": { "body": shape } },
      "selection": {
        "variants": [{ "id": "base", "origin": "base", "assignment": [],
                       "parts": { "response": { "body": { "content": { "status": "PENDING", "id": 66 } } } } }],
        "report": { "strategy": "exhaustive", "space": { "size": 1, "exact": true, "dimensions": 0 } } }
    }]
  }))
  .expect("a well-formed contract")
}

fn written(contract: &Contract) -> String {
  String::from_utf8(contract::write_canonical(contract).unwrap()).unwrap()
}

#[test]
fn a_contract_writes_a_shapes_own_members_by_name_and_an_objects_members_as_declared() {
  let bytes = written(&contract_with(json!({ "shape": "object", "members": {
    "status": { "shape": "equality", "example": "PENDING" },
    "id": { "example": 66, "shape": "equality" } } })));
  assert!(
    bytes.contains(
      r#""body":{"members":{"status":{"example":"PENDING","shape":"equality"},"id":{"example":66,"shape":"equality"}},"shape":"object"}"#
    ),
    "{bytes}"
  );
  assert!(
    bytes.contains(r#""content":{"status":"PENDING","id":66}"#),
    "a recorded body is a document, written as it arrived: {bytes}"
  );
  assert!(
    bytes.contains(r#""report":{"space":{"dimensions":0,"exact":true,"size":1},"strategy":"exhaustive"}"#),
    "the selection report is a record, written by name: {bytes}"
  );
}

#[test]
fn the_same_shape_built_in_another_order_is_the_same_bytes() {
  let one = contract_with(json!({ "shape": "object", "members": {
    "status": { "shape": "equality", "example": "PENDING" } } }));
  let other = contract_with(json!({ "members": {
    "status": { "example": "PENDING", "shape": "equality" } }, "shape": "object" }));
  assert_eq!(written(&one), written(&other));
}

#[test]
fn a_contract_read_and_written_again_keeps_its_bytes() {
  let bytes = written(&contract_with(json!({ "shape": "object", "members": {
    "status": { "shape": "equality", "example": "PENDING" },
    "id": { "shape": "equality", "example": 66 } } })));
  let read = contract::read(bytes.as_bytes(), contract::IdentifyMode::Strict).unwrap();
  assert_eq!(written(&read), bytes);
}

#[test]
fn a_component_operators_members_are_kept_as_written() {
  let node = json!({ "shape": "csv:row", "zeta": 1, "alpha": { "y": 1, "x": 2 } });
  assert_eq!(
    shape::canonical(&node).to_string(),
    node.to_string(),
    "the kernel does not interpret it (shape spec §3.5), so it does not reorder it"
  );
}
