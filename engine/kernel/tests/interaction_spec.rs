//! Plan task 3.2: the interaction-specification document model — parsing the document
//! `consumer-session/add-interaction` carries (engine protocol spec §8.2), per description,
//! transport, states, parts and shapes (shape-language spec §2.2, contract-file spec §4-§7).

use pact_janus_kernel::interaction_spec::{InteractionSpecError, parse};
use pact_janus_kernel::shape::{CoreShape, ShapeKind};
use serde_json::json;

fn assert_problem(error: &InteractionSpecError, expected_pointer: &str, message_contains: &str) {
  assert!(
    error
      .problems
      .iter()
      .any(|p| p.pointer == expected_pointer && p.message.contains(message_contains)),
    "expected a problem at {expected_pointer} containing {message_contains:?}, got: {:?}",
    error.problems
  );
}

// shape-language spec examples/order-payload.md §8: "How this reaches the engine".
#[test]
fn the_rfc_request_interaction_sketch_is_well_formed() {
  let spec = json!({
    "description": "a request for an order",
    "transport": { "kind": "http", "mode": "passive" },
    "parts": {
      "request": { "method": { "shape": "equality", "example": "GET" },
                   "path": { "shape": "equality", "example": "/orders/42" } },
      "response": { "status": { "shape": "equality", "example": 200 },
                    "body": { "shape": "object", "members": {} } } }
  });

  let interaction =
    parse(&spec).unwrap_or_else(|err| panic!("expected a well-formed spec, got {:?}", err.problems));
  assert_eq!(interaction.description, "a request for an order");
  assert_eq!(interaction.transport.as_ref().unwrap().kind, "http");
  assert_eq!(interaction.parts["request"].len(), 2);
  assert!(matches!(
    interaction.parts["response"]["body"].kind,
    ShapeKind::Core(CoreShape::Object { .. })
  ));
}

#[test]
fn states_carry_unresolved_variant_bindings() {
  // contract-file spec §6: bindings appear once, on the interaction; nothing here resolves them.
  let spec = json!({
    "description": "get an order",
    "states": [ { "name": "an order exists",
                  "params": { "id": "42" },
                  "variant-params": [ { "name": "shipped",
                                        "dimension": "response.body.shippedAt#presence",
                                        "cases": [ { "point": "present", "value": true },
                                                   { "point": "absent", "value": false } ] } ] } ],
    "parts": { "response": { "status": { "shape": "equality", "example": 200 } } }
  });

  let interaction =
    parse(&spec).unwrap_or_else(|err| panic!("expected a well-formed spec, got {:?}", err.problems));
  let states = interaction.states.unwrap();
  assert_eq!(states.len(), 1);
  assert_eq!(states[0].name, "an order exists");
  assert!(states[0].variant_params.is_some());
}

#[test]
fn a_missing_description_is_reported() {
  let spec = json!({ "parts": {} });
  let err = parse(&spec).unwrap_err();
  assert_eq!(err.code(), "interaction-invalid");
  assert_problem(&err, "/description", "required");
}

#[test]
fn an_empty_description_is_rejected() {
  let spec = json!({ "description": "", "parts": {} });
  let err = parse(&spec).unwrap_err();
  assert_problem(&err, "/description", "empty");
}

#[test]
fn missing_parts_is_reported() {
  let spec = json!({ "description": "get an order" });
  let err = parse(&spec).unwrap_err();
  assert_problem(&err, "/parts", "required");
}

#[test]
fn a_malformed_transport_is_reported_at_its_member() {
  let spec = json!({
    "description": "get an order",
    "transport": { "kind": 42 },
    "parts": {}
  });
  let err = parse(&spec).unwrap_err();
  assert_problem(&err, "/transport/kind", "");
}

#[test]
fn a_shape_problem_inside_a_part_is_reported_with_the_full_pointer() {
  let spec = json!({
    "description": "get an order",
    "parts": {
      "response": {
        "body": { "shape": "each-like",
                  "items": { "shape": "optional", "of": { "shape": "string", "example": "x" } } } } }
  });
  let err = parse(&spec).unwrap_err();
  assert_problem(&err, "/parts/response/body/items", "spec §5.1");
}

#[test]
fn requirements_are_carried_through() {
  let spec = json!({
    "description": "get an order",
    "parts": { "response": { "body": { "shape": "protobuf:message", "example": {} } } },
    "requires": [ { "component": "content/protobuf", "min-version": 2 } ]
  });
  let interaction =
    parse(&spec).unwrap_or_else(|err| panic!("expected a well-formed spec, got {:?}", err.problems));
  let requires = interaction.requires.unwrap();
  assert_eq!(requires[0].component, "content/protobuf");
  assert_eq!(requires[0].min_version, Some(2));
}

#[test]
fn a_non_object_document_is_reported_at_the_root() {
  let spec = json!("not an interaction");
  let err = parse(&spec).unwrap_err();
  assert_problem(&err, "", "object");
}
