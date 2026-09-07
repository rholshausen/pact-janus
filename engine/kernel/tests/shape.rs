//! Plan task 3.2: the shape parser/validator, exercised against the worked examples in
//! `Documentation/specs/shape-language/examples/` — the well-formed trees there must parse
//! clean, and the "ill-formed shapes, and what the engine says" cases in
//! `composition-edges.md` §6 must fail with the rule the spec names.

use pact_janus_kernel::shape::{CoreShape, ShapeKind, parse};
use rstest::rstest;
use serde_json::{Value, json};

fn assert_ok(value: &Value) -> pact_janus_kernel::shape::ShapeNode {
  match parse(value, "/body") {
    Ok(node) => node,
    Err(problems) => panic!("expected a well-formed shape, got problems: {problems:?}"),
  }
}

fn assert_problem(value: &Value, expected_pointer: &str, message_contains: &str) {
  let problems = match parse(value, "/body") {
    Ok(node) => panic!("expected a problem, parsed cleanly as {node:?}"),
    Err(problems) => problems,
  };
  assert!(
    problems
      .iter()
      .any(|p| p.pointer == expected_pointer && p.message.contains(message_contains)),
    "expected a problem at {expected_pointer} containing {message_contains:?}, got: {problems:?}"
  );
}

// --- shape-language spec examples/order-payload.md §2: the canonical order shape ---

#[test]
fn the_rfc_order_payload_shape_is_well_formed() {
  let shape = json!({ "shape": "object",
    "members": {
      "id": { "shape": "integer", "example": 42 },
      "status": { "shape": "any-of",
                  "options": ["PENDING", "SHIPPED", "DELIVERED"],
                  "example": "PENDING" },
      "shippedAt": { "shape": "optional",
                     "of": { "shape": "datetime",
                             "format": "yyyy-MM-dd'T'HH:mm:ssX",
                             "example": "2026-07-30T10:00:00Z" } },
      "payment": { "shape": "one-of",
                   "discriminator": "type",
                   "default": "card",
                   "alternatives": {
                     "card": { "shape": "object",
                               "members": {
                                 "type": { "shape": "equality", "example": "card" },
                                 "last4": { "shape": "regex", "pattern": "\\d{4}", "example": "1234" } } },
                     "invoice": { "shape": "object",
                                  "members": {
                                    "type": { "shape": "equality", "example": "invoice" },
                                    "dueDate": { "shape": "date", "format": "yyyy-MM-dd",
                                                 "example": "2026-08-30" } } } } },
      "items": { "shape": "each-like",
                 "min": 1,
                 "items": { "shape": "object",
                            "members": {
                              "sku": { "shape": "string", "example": "SKU-1" },
                              "qty": { "shape": "integer", "example": 1 } } } } } });

  let node = assert_ok(&shape);
  let ShapeKind::Core(CoreShape::Object { members }) = &node.kind else {
    panic!("expected an object shape");
  };
  assert_eq!(members.len(), 5);
  assert!(matches!(
    members["payment"].kind,
    ShapeKind::Core(CoreShape::OneOf { .. })
  ));
}

// --- composition-edges.md §1: presence versus nullability ---

#[test]
fn optional_may_wrap_nullable() {
  let shape = json!({ "shape": "object",
    "members": {
      "a": { "shape": "optional", "of": { "shape": "string", "example": "x" } },
      "b": { "shape": "nullable", "of": { "shape": "string", "example": "x" } },
      "c": { "shape": "optional", "of": { "shape": "nullable",
                                          "of": { "shape": "string", "example": "x" } } } } });
  assert_ok(&shape);
}

#[test]
fn nullable_must_not_wrap_optional() {
  // composition-edges.md §6, second ill-formed example.
  let shape = json!({ "shape": "nullable",
    "of": { "shape": "optional", "of": { "shape": "string", "example": "x" } } });
  assert_problem(&shape, "/body/of", "spec §5.1");
}

#[test]
fn nullable_must_not_wrap_nullable() {
  let shape = json!({ "shape": "nullable",
    "of": { "shape": "nullable", "of": { "shape": "string", "example": "x" } } });
  assert_problem(&shape, "/body/of", "spec §5.2");
}

// --- composition-edges.md §2: object vs each-entry ---

#[test]
fn each_entry_with_keys_and_values_is_well_formed() {
  let shape = json!({ "shape": "object",
    "members": {
      "id": { "shape": "integer", "example": 42 },
      "labels": { "shape": "each-entry",
                  "min": 1,
                  "keys": { "shape": "regex", "pattern": "^[a-z.]+$", "example": "env" },
                  "values": { "shape": "string", "example": "prod" } } } });
  assert_ok(&shape);
}

// --- composition-edges.md §3: array, each-like, contains ---

#[test]
fn the_three_array_operators_are_well_formed() {
  let shape = json!({ "shape": "object",
    "members": {
      "coordinates": { "shape": "array",
                       "entries": [ { "shape": "decimal", "example": -36.85 },
                                    { "shape": "decimal", "example": 174.76 } ] },
      "items": { "shape": "each-like",
                 "min": 1, "max": 3,
                 "items": { "shape": "string", "example": "SKU-1" } },
      "audit": { "shape": "contains",
                 "entries": [ { "shape": "object",
                                "members": { "event": { "shape": "equality", "example": "created" } } } ] } } });
  assert_ok(&shape);
}

// --- composition-edges.md §5: gated dimensions under one-of ---

#[test]
fn one_of_with_optional_member_in_one_alternative_is_well_formed() {
  let shape = json!({ "shape": "one-of",
    "discriminator": "kind",
    "default": "email",
    "alternatives": {
      "email": { "shape": "object",
                 "members": {
                   "kind": { "shape": "equality", "example": "email" },
                   "address": { "shape": "regex", "pattern": "@", "example": "a@example.com" },
                   "verified": { "shape": "optional", "of": { "shape": "boolean", "example": true } } } },
      "sms": { "shape": "object",
               "members": {
                 "kind": { "shape": "equality", "example": "sms" },
                 "number": { "shape": "regex", "pattern": "^\\+", "example": "+6421000000" } } } } });
  assert_ok(&shape);
}

// --- composition-edges.md §6: ill-formed shapes, and what the engine says ---

#[test]
fn optional_outside_a_slot_is_rejected() {
  let shape = json!({ "shape": "each-like",
    "items": { "shape": "optional", "of": { "shape": "string", "example": "x" } } });
  assert_problem(&shape, "/body/items", "spec §5.1");
}

#[test]
fn an_untagged_discriminator_is_rejected() {
  let shape = json!({ "shape": "one-of",
    "discriminator": "type",
    "alternatives": {
      "a": { "shape": "object", "members": { "type": { "shape": "string", "example": "a" } } },
      "b": { "shape": "object", "members": { "type": { "shape": "string", "example": "b" } } } } });
  assert_problem(&shape, "/body/alternatives/a/members/type", "spec §5.4");
}

#[test]
fn an_any_of_example_outside_its_options_is_rejected() {
  let shape = json!({ "shape": "any-of", "options": ["PENDING", "SHIPPED"], "example": "DELIVERED" });
  assert_problem(&shape, "/body/example", "spec §5.3");
}

#[test]
fn an_unknown_unnamespaced_operator_is_rejected_by_name() {
  let shape = json!({ "shape": "object",
    "members": { "total": { "shape": "sum-of", "example": 42 } } });
  assert_problem(&shape, "/body/members/total", "sum-of");
}

// --- namespaced (component) operators: opaque, never guessed at ---

#[test]
fn a_namespaced_operator_is_parsed_opaquely_with_no_well_formedness_checks() {
  // A component operator's `admits` is opaque to the kernel (spec §3.5): even something that
  // would be ill-formed for a core operator — nested absence, whatever — is not the kernel's
  // business to judge.
  let shape = json!({ "shape": "protobuf:enum", "example": "SHIPPED", "generator": {} });
  let node = assert_ok(&shape);
  let ShapeKind::Component { operator, .. } = &node.kind else {
    panic!("expected a component shape");
  };
  assert_eq!(operator, "protobuf:enum");
  assert!(node.generator.is_some());
}

// --- other well-formedness rules (spec §5) not covered by the shipped worked examples ---

#[rstest]
#[case::regex_without_pattern(json!({ "shape": "regex", "example": "x" }), "requires 'pattern'")]
#[case::equality_without_example(json!({ "shape": "equality" }), "requires 'example'")]
#[case::each_like_without_items(json!({ "shape": "each-like" }), "requires 'items'")]
fn missing_required_members_are_named(#[case] shape: Value, #[case] message_contains: &str) {
  assert_problem(&shape, "/body", message_contains);
}

#[test]
fn any_of_options_must_be_distinct() {
  let shape = json!({ "shape": "any-of", "options": ["PENDING", "PENDING"], "example": "PENDING" });
  assert_problem(&shape, "/body/options/1", "distinct");
}

#[test]
fn each_like_min_must_not_exceed_max() {
  let shape = json!({ "shape": "each-like", "min": 3, "max": 1, "items": { "shape": "any" } });
  assert_problem(&shape, "/body", "spec §5.5");
}

#[test]
fn forbidden_at_a_parts_slot_root_is_rejected() {
  // The root of a part's shape is not a slot position (spec §5.1) — the same rule that rejects
  // `optional` inside `each-like`'s items, applied at the top of the tree.
  let shape = json!({ "shape": "forbidden" });
  assert_problem(&shape, "/body", "spec §5.1");
}

#[test]
fn a_datetime_shape_may_omit_format() {
  // Spec prose §4.2: format absent means "any ISO-8601 string of this kind", so a standard-format
  // author never has to spell it out.
  let shape = json!({ "shape": "datetime", "example": "2026-07-30T10:00:00Z" });
  assert_ok(&shape);
}
