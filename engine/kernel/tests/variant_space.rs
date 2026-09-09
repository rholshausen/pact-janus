//! Plan task 3.3: computing a shape's variant space (shape-language spec §6), exercised against
//! the worked examples in `Documentation/specs/shape-language/examples/order-payload.md` §3 and
//! `Documentation/specs/variant-semantics/examples/order-payload-sampling.md` §6 — the dimension
//! lists there are what an implementation is checked against (design 2.3's worked example says so
//! explicitly: "if an implementation disagrees with them, one of the two is wrong").

use pact_janus_kernel::interaction_spec::parse;
use pact_janus_kernel::plan::variant_space;
use serde_json::{Value, json};

fn order_payload_body() -> Value {
  json!({ "shape": "object",
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
                              "qty": { "shape": "integer", "example": 1 } } } } } })
}

fn interaction_with(body: Value) -> pact_janus_kernel::interaction_spec::InteractionSpec {
  let spec = json!({
    "description": "get an order",
    "parts": { "response": { "body": body } }
  });
  parse(&spec).unwrap_or_else(|err| panic!("expected a well-formed spec, got {:?}", err.problems))
}

// shape-language spec examples/order-payload.md §3.
#[test]
fn the_rfc_order_payload_has_the_documented_four_dimensions() {
  let interaction = interaction_with(order_payload_body());
  let space = variant_space(&interaction);

  let ids: Vec<&str> = space.dimensions.iter().map(|d| d.id.as_str()).collect();
  assert_eq!(
    ids,
    vec![
      "response.body.items#cardinality",
      "response.body.payment#alternative",
      "response.body.shippedAt#presence",
      "response.body.status#value",
    ],
    "dimensions must be depth-first, object members lexicographic (spec §6.2)"
  );

  let items = &space.dimensions[0];
  assert_eq!(items.path, "response.body.items");
  assert_eq!(items.facet, "cardinality");
  assert_eq!(items.operator, "each-like");
  assert_eq!(items.default, "min");
  assert_eq!(
    items
      .points
      .iter()
      .map(|p| (p.name.as_str(), p.size))
      .collect::<Vec<_>>(),
    vec![("min", Some(1)), ("min+1", Some(2))]
  );
  assert!(items.gated_by.is_empty());

  let payment = &space.dimensions[1];
  assert_eq!(payment.facet, "alternative");
  assert_eq!(payment.operator, "one-of");
  assert_eq!(payment.default, "card");
  assert_eq!(
    payment.points.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(),
    vec!["card", "invoice"]
  );

  let shipped_at = &space.dimensions[2];
  assert_eq!(shipped_at.facet, "presence");
  assert_eq!(shipped_at.operator, "optional");
  assert_eq!(shipped_at.default, "present");
  assert_eq!(
    shipped_at
      .points
      .iter()
      .map(|p| p.name.as_str())
      .collect::<Vec<_>>(),
    vec!["present", "absent"]
  );

  let status = &space.dimensions[3];
  assert_eq!(status.facet, "value");
  assert_eq!(status.operator, "any-of");
  assert_eq!(status.default, "PENDING");
  assert_eq!(
    status
      .points
      .iter()
      .map(|p| (p.name.as_str(), p.value.clone()))
      .collect::<Vec<_>>(),
    vec![
      ("PENDING", Some(Value::String("PENDING".to_string()))),
      ("SHIPPED", Some(Value::String("SHIPPED".to_string()))),
      ("DELIVERED", Some(Value::String("DELIVERED".to_string()))),
    ]
  );

  // No dimension is gated: nothing inside `optional` or the `one-of` alternatives carries one
  // (spec §3's own note).
  assert!(space.dimensions.iter().all(|d| d.gated_by.is_empty()));
}

// variant-semantics spec examples/order-payload-sampling.md §6: give `invoice` an optional
// `dueDate` and a fifth, gated dimension appears.
#[test]
fn a_dimension_inside_a_one_of_alternative_is_gated_by_it() {
  let mut body = order_payload_body();
  body["members"]["payment"]["alternatives"]["invoice"]["members"]["dueDate"] = json!({
    "shape": "optional",
    "of": { "shape": "date", "format": "yyyy-MM-dd", "example": "2026-08-30" }
  });
  let interaction = interaction_with(body);
  let space = variant_space(&interaction);

  let due_date = space
    .dimensions
    .iter()
    .find(|d| d.id == "response.body.payment@invoice.dueDate#presence")
    .expect("dueDate should contribute a presence dimension");
  assert_eq!(due_date.path, "response.body.payment@invoice.dueDate");
  assert_eq!(due_date.facet, "presence");
  assert_eq!(
    due_date.gated_by,
    vec![pact_janus_kernel::shape::variant_space::Gate {
      dimension: "response.body.payment#alternative".to_string(),
      point: "invoice".to_string(),
    }]
  );

  // The gating dimension is emitted before the dimension it gates (spec §6.6: "parents before the
  // dimensions they gate").
  let alt_index = space
    .dimensions
    .iter()
    .position(|d| d.id == "response.body.payment#alternative")
    .unwrap();
  let due_date_index = space
    .dimensions
    .iter()
    .position(|d| d.id == "response.body.payment@invoice.dueDate#presence")
    .unwrap();
  assert!(alt_index < due_date_index);
}

#[test]
fn a_single_option_any_of_contributes_no_dimension() {
  let interaction = interaction_with(json!({ "shape": "object",
    "members": { "status": { "shape": "any-of", "options": ["ONLY"], "example": "ONLY" } } }));
  assert!(variant_space(&interaction).dimensions.is_empty());
}

#[test]
fn an_each_like_with_equal_min_and_max_contributes_no_dimension() {
  let interaction = interaction_with(json!({ "shape": "object",
    "members": { "items": { "shape": "each-like", "min": 2, "max": 2,
                             "items": { "shape": "string", "example": "x" } } } }));
  assert!(variant_space(&interaction).dimensions.is_empty());
}

#[test]
fn an_each_like_with_a_finite_max_contributes_three_points() {
  let interaction = interaction_with(json!({ "shape": "object",
    "members": { "items": { "shape": "each-like", "min": 0, "max": 3,
                             "items": { "shape": "string", "example": "x" } } } }));
  let space = variant_space(&interaction);
  assert_eq!(space.dimensions.len(), 1);
  assert_eq!(
    space.dimensions[0]
      .points
      .iter()
      .map(|p| (p.name.as_str(), p.size))
      .collect::<Vec<_>>(),
    vec![("min", Some(0)), ("min+1", Some(1)), ("max", Some(3))]
  );
}

#[test]
fn a_component_operator_contributes_no_dimension() {
  let interaction = interaction_with(json!({ "shape": "object",
    "members": { "enumField": { "shape": "protobuf:enum", "example": "SHIPPED" } } }));
  assert!(variant_space(&interaction).dimensions.is_empty());
}
