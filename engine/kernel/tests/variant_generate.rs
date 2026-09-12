//! Plan task 4.3's fourth deliverable: generators producing a variant's concrete payload
//! ([`pact_janus_kernel::variant::generate`]).

use pact_janus_kernel::interaction_spec::parse;
use pact_janus_kernel::plan::variant_space;
use pact_janus_kernel::variant::generate;
use pact_janus_kernel::variant::{SamplingPolicy, select};
use serde_json::json;

fn order_payload_interaction() -> pact_janus_kernel::interaction_spec::InteractionSpec {
  let spec = json!({
    "description": "get an order",
    "parts": { "response": { "body": {
      "shape": "object",
      "members": {
        "status": { "shape": "any-of", "options": ["PENDING", "SHIPPED", "DELIVERED"], "example": "PENDING" },
        "shippedAt": { "shape": "optional", "of": { "shape": "string", "example": "2026-07-30" } },
        "payment": { "shape": "one-of", "discriminator": "type", "default": "card",
          "alternatives": {
            "card": { "shape": "object", "members": { "type": { "shape": "equality", "example": "card" } } },
            "invoice": { "shape": "object", "members": { "type": { "shape": "equality", "example": "invoice" } } } } },
        "items": { "shape": "each-like", "min": 1,
          "items": { "shape": "object", "members": { "sku": { "shape": "string", "example": "SKU-1" } } } } } } } }
  });
  parse(&spec).unwrap_or_else(|err| panic!("expected a well-formed spec, got {:?}", err.problems))
}

#[test]
fn the_base_variant_generates_the_authors_own_example() {
  let interaction = order_payload_interaction();
  let space = variant_space(&interaction);
  let selected = select(&space, &SamplingPolicy::default()).unwrap();
  let base = selected.variants.iter().find(|v| v.id == "base").unwrap();

  let parts = generate::interaction(&interaction, &base.assignment);
  let body = &parts["response"]["body"];
  assert_eq!(body["status"], "PENDING");
  assert_eq!(body["shippedAt"], "2026-07-30");
  assert_eq!(body["payment"]["type"], "card");
  assert_eq!(body["items"].as_array().unwrap().len(), 1);
}

#[test]
fn an_absent_optional_omits_the_member_entirely() {
  let interaction = order_payload_interaction();
  let space = variant_space(&interaction);
  let selected = select(&space, &SamplingPolicy::default()).unwrap();
  let absent = selected
    .variants
    .iter()
    .find(|v| v.id == "response.body.shippedAt#presence=absent")
    .expect("the boundary variant with shippedAt absent");

  let parts = generate::interaction(&interaction, &absent.assignment);
  let body = &parts["response"]["body"];
  assert!(
    body.as_object().unwrap().get("shippedAt").is_none(),
    "shippedAt should be omitted, got {body}"
  );
}

#[test]
fn a_covering_variant_selects_its_alternative_and_any_of_point() {
  let interaction = order_payload_interaction();
  let space = variant_space(&interaction);
  let selected = select(&space, &SamplingPolicy::default()).unwrap();
  let invoice_shipped = selected
    .variants
    .iter()
    .find(|v| {
      v.assignment
        .get("response.body.payment#alternative")
        .map(String::as_str)
        == Some("invoice")
        && v.assignment.get("response.body.status#value").map(String::as_str) == Some("SHIPPED")
    })
    .expect("a variant pairing invoice with SHIPPED");

  let parts = generate::interaction(&interaction, &invoice_shipped.assignment);
  let body = &parts["response"]["body"];
  assert_eq!(body["payment"]["type"], "invoice");
  assert_eq!(body["status"], "SHIPPED");
}

#[test]
fn a_maximal_boundary_generates_two_items() {
  let interaction = order_payload_interaction();
  let space = variant_space(&interaction);
  let selected = select(&space, &SamplingPolicy::default()).unwrap();
  let maximal = selected
    .variants
    .iter()
    .find(|v| v.id == "response.body.items#cardinality=min+1")
    .unwrap();

  let parts = generate::interaction(&interaction, &maximal.assignment);
  let body = &parts["response"]["body"];
  assert_eq!(body["items"].as_array().unwrap().len(), 2);
}
