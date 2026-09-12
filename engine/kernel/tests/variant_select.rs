//! Plan task 4.3: the `janus-ipog-v1` selection algorithm (variant-semantics spec §3.4), checked
//! against `Documentation/specs/variant-semantics/examples/order-payload-sampling.md` — the
//! worked example says plainly "if an implementation disagrees with them, one of the two is
//! wrong and the test says which."

use pact_janus_kernel::interaction_spec::{InteractionSpec, parse};
use pact_janus_kernel::plan::variant_space;
use pact_janus_kernel::variant::{Origin, SamplingPolicy, VariantError, select};
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

fn interaction_with(body: Value) -> InteractionSpec {
  let spec = json!({
    "description": "get an order",
    "parts": { "response": { "body": body } }
  });
  parse(&spec).unwrap_or_else(|err| panic!("expected a well-formed spec, got {:?}", err.problems))
}

fn ids(variants: &[pact_janus_kernel::variant::Variant]) -> Vec<&str> {
  variants.iter().map(|v| v.id.as_str()).collect()
}

#[test]
fn the_rfc_order_payload_selects_eight_variants_in_the_documented_order() {
  let interaction = interaction_with(order_payload_body());
  let space = variant_space(&interaction);
  let policy = SamplingPolicy::default();

  let selected = select(&space, &policy).expect("well-formed policy");

  assert_eq!(selected.report.space.size, 24);
  assert!(selected.report.space.exact);
  assert_eq!(selected.report.strategy, "t-wise");
  assert_eq!(selected.report.strength, Some(2));
  assert_eq!(selected.report.algorithm, "janus-ipog-v1");
  assert_eq!(selected.report.coverage.targets, 30);
  assert_eq!(selected.report.coverage.covered, 30);
  assert_eq!(selected.report.coverage.removed, 0);
  assert_eq!(selected.report.coverage.dropped, 0);
  assert_eq!(selected.report.selected, 8);

  assert_eq!(
    ids(&selected.variants),
    vec![
      "base",
      "response.body.shippedAt#presence=absent",
      "response.body.items#cardinality=min+1",
      "response.body.payment#alternative=invoice;response.body.status#value=SHIPPED",
      "response.body.items#cardinality=min+1;response.body.shippedAt#presence=absent;response.body.status#value=SHIPPED",
      "response.body.status#value=DELIVERED",
      "response.body.items#cardinality=min+1;response.body.payment#alternative=invoice;response.body.shippedAt#presence=absent;response.body.status#value=DELIVERED",
      "response.body.payment#alternative=invoice",
    ]
  );

  assert_eq!(selected.variants[0].origin, Origin::Base);
  assert_eq!(selected.variants[1].origin, Origin::Boundary);
  assert_eq!(selected.variants[2].origin, Origin::Boundary);
  for v in &selected.variants[3..] {
    assert_eq!(v.origin, Origin::Covering);
  }

  assert_eq!(selected.variants[1].label, "shippedAt=absent");
  assert_eq!(selected.variants[2].label, "items=min+1");
  assert_eq!(selected.variants[3].label, "payment=invoice;status=SHIPPED");
  assert_eq!(selected.variants[7].label, "payment=invoice");

  let base = &selected.variants[0].assignment;
  assert_eq!(base["response.body.items#cardinality"], "min");
  assert_eq!(base["response.body.payment#alternative"], "card");
  assert_eq!(base["response.body.shippedAt#presence"], "present");
  assert_eq!(base["response.body.status#value"], "PENDING");
}

#[test]
fn a_smaller_space_runs_exhaustively() {
  // Drop any-of and each-like: two dimensions remain (payment, shippedAt), space of 4.
  let mut body = order_payload_body();
  body["members"].as_object_mut().unwrap().remove("status");
  body["members"].as_object_mut().unwrap().remove("items");
  let interaction = interaction_with(body);
  let space = variant_space(&interaction);

  let selected = select(&space, &SamplingPolicy::default()).unwrap();
  assert_eq!(selected.report.space.size, 4);
  assert_eq!(selected.report.strategy, "exhaustive");
  assert_eq!(selected.report.selected, 4);
}

#[test]
fn pinning_the_shipped_with_no_timestamp_case() {
  let interaction = interaction_with(order_payload_body());
  let space = variant_space(&interaction);
  let patch = json!({ "pin": [
    { "assignment": [ { "dimension": "status", "point": "SHIPPED" },
                      { "dimension": "shippedAt", "point": "absent" } ],
      "reason": "the case the order-history screen renders differently" } ] });
  let policy = SamplingPolicy::resolve(&[Some(&patch)]).unwrap();

  let selected = select(&space, &policy).expect("well-formed policy");
  assert_eq!(selected.report.selected, 9);
  assert_eq!(selected.report.coverage.covered, 30);
  assert_eq!(selected.variants[3].origin, Origin::Pinned);
  assert_eq!(
    selected.variants[3].assignment["response.body.status#value"],
    "SHIPPED"
  );
  assert_eq!(
    selected.variants[3].assignment["response.body.shippedAt#presence"],
    "absent"
  );
}

#[test]
fn excluding_a_combination_the_provider_cannot_produce() {
  let interaction = interaction_with(order_payload_body());
  let space = variant_space(&interaction);
  let patch = json!({ "exclude": [
    { "when": [ { "dimension": "status", "point": "SHIPPED" },
                { "dimension": "shippedAt", "point": "absent" } ],
      "reason": "the order service sets shippedAt whenever it sets SHIPPED" } ] });
  let policy = SamplingPolicy::resolve(&[Some(&patch)]).unwrap();

  let selected = select(&space, &policy).expect("well-formed policy");
  assert_eq!(selected.report.selected, 8);
  assert_eq!(selected.report.coverage.targets, 29);
  assert_eq!(selected.report.coverage.covered, 29);
  assert_eq!(selected.report.coverage.removed, 1);

  for v in &selected.variants {
    let excluded = v.assignment.get("response.body.status#value").map(String::as_str) == Some("SHIPPED")
      && v
        .assignment
        .get("response.body.shippedAt#presence")
        .map(String::as_str)
        == Some("absent");
    assert!(!excluded, "variant {} matches the excluded region", v.id);
  }
}

#[test]
fn gating_adds_a_fifth_dimension_and_moves_the_maximal_variant() {
  let mut body = order_payload_body();
  body["members"]["payment"]["alternatives"]["invoice"]["members"]["dueDate"] = json!({
    "shape": "optional",
    "of": { "shape": "date", "format": "yyyy-MM-dd", "example": "2026-08-30" }
  });
  let interaction = interaction_with(body);
  let space = variant_space(&interaction);

  let selected = select(&space, &SamplingPolicy::default()).unwrap();
  assert_eq!(selected.report.space.size, 36);
  assert_eq!(selected.report.coverage.targets, 46);
  assert_eq!(selected.report.coverage.covered, 46);
  assert_eq!(selected.report.selected, 10);

  // The maximal boundary now opens `invoice` with `dueDate` present, not `card`.
  let maximal = selected
    .variants
    .iter()
    .find(|v| {
      v.origin == Origin::Boundary
        && v
          .assignment
          .get("response.body.items#cardinality")
          .map(String::as_str)
          == Some("min+1")
    })
    .expect("a maximal boundary variant");
  assert_eq!(maximal.assignment["response.body.payment#alternative"], "invoice");
  assert_eq!(
    maximal.assignment["response.body.payment@invoice.dueDate#presence"],
    "present"
  );
}

#[test]
fn the_budget_bites_on_two_wide_enumerations() {
  let body = json!({ "shape": "object",
    "members": {
      "currency": { "shape": "any-of", "options": (0..30).map(|n| n.to_string()).collect::<Vec<_>>(), "example": "0" },
      "settlementCurrency": { "shape": "any-of", "options": (0..30).map(|n| (n + 100).to_string()).collect::<Vec<_>>(), "example": "100" } } });
  let interaction = interaction_with(body);
  let space = variant_space(&interaction);

  let err = select(&space, &SamplingPolicy::default()).unwrap_err();
  match err {
    VariantError::BudgetExceeded {
      space,
      selected,
      budget,
      ..
    } => {
      assert_eq!(space, 900);
      assert_eq!(selected, 900);
      assert_eq!(budget, 50);
    }
    other => panic!("expected a budget-exceeded error, got {other:?}"),
  }
}

#[test]
fn base_only_strategy_selects_the_base_and_pins_alone() {
  let interaction = interaction_with(order_payload_body());
  let space = variant_space(&interaction);
  let policy = SamplingPolicy {
    strategy: "base-only".to_string(),
    ..SamplingPolicy::default()
  };

  let selected = select(&space, &policy).unwrap();
  assert_eq!(selected.report.selected, 1);
  assert_eq!(selected.report.strategy, "base-only");
  assert_eq!(selected.variants[0].id, "base");
}
