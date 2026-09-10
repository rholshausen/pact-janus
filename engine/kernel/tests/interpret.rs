//! Plan task 3.4: the plan interpreter and resolvers, checked against the worked examples in
//! `Documentation/specs/plan-grammar/examples/corpus-case.md` and
//! `Documentation/specs/plan-grammar/examples/order-payload-plan.md` §3, plus per-action coverage
//! of the compiled forms plan task 3.3 actually produces.

use pact_janus_kernel::interaction_spec::{InteractionSpec, parse};
use pact_janus_kernel::plan::{
  Assignment, CapturedValues, Executed, ExecutedKind, NodeResult, RuntimeValue, Status, compile, execute,
  outcome,
};
use serde_json::{Value, json};
use std::collections::BTreeMap;

fn interaction(body: Value) -> InteractionSpec {
  let spec = json!({
    "description": "get an order",
    "parts": { "response": { "body": body } }
  });
  parse(&spec).unwrap_or_else(|err| panic!("expected a well-formed spec, got {:?}", err.problems))
}

fn run_body(body: Value, values: BTreeMap<String, Value>) -> Executed {
  let plan = compile(&interaction(body), &Assignment::new(), None);
  let resolver = CapturedValues::from_json(&values);
  execute(&plan, &resolver)
}

fn captured(pairs: &[(&str, Value)]) -> BTreeMap<String, Value> {
  pairs.iter().map(|(k, v)| (k.to_string(), v.clone())).collect()
}

// --- corpus-case.md end to end ---

#[test]
fn optional_absent_matches_and_must_ignore_holds() {
  let body = json!({ "shape": "object",
    "members": {
      "id": { "shape": "integer", "example": 42 },
      "shippedAt": { "shape": "optional",
                     "of": { "shape": "datetime", "format": "yyyy-MM-dd'T'HH:mm:ssX",
                             "example": "2026-07-30T10:00:00Z" } } } });
  let values = captured(&[("$.response.body", json!({ "id": 7, "warehouse": "AKL-1" }))]);
  let executed = run_body(body, values);
  let (status, mismatches) = outcome(&executed);
  assert_eq!(status, Status::Matched, "mismatches: {mismatches:?}");
}

#[test]
fn a_result_diff_is_a_behaviour_change_absent_required_member_fails() {
  let body = json!({ "shape": "object",
    "members": { "id": { "shape": "integer", "example": 42 } } });
  let values = captured(&[("$.response.body", json!({}))]);
  let executed = run_body(body, values);
  let (status, mismatches) = outcome(&executed);
  assert_eq!(status, Status::Mismatched);
  assert_eq!(mismatches.len(), 1);
  assert_eq!(mismatches[0].path.as_deref(), Some("$.response.body.id"));
  assert_eq!(mismatches[0].action.as_deref(), Some("match:integer"));
}

// --- order-payload-plan.md §3: "Executed, and failing" ---

fn order_payload_body() -> Value {
  json!({ "shape": "object",
    "members": {
      "id": { "shape": "integer", "example": 42 },
      "status": { "shape": "any-of", "options": ["PENDING", "SHIPPED", "DELIVERED"], "example": "PENDING" },
      "shippedAt": { "shape": "optional",
                     "of": { "shape": "datetime", "format": "yyyy-MM-dd'T'HH:mm:ssX",
                             "example": "2026-07-30T10:00:00Z" } },
      "payment": { "shape": "one-of", "discriminator": "type", "default": "card",
        "alternatives": {
          "card": { "shape": "object", "members": {
            "type": { "shape": "equality", "example": "card" },
            "last4": { "shape": "regex", "pattern": "\\d{4}", "example": "1234" } } },
          "invoice": { "shape": "object", "members": {
            "type": { "shape": "equality", "example": "invoice" },
            "dueDate": { "shape": "date", "format": "yyyy-MM-dd", "example": "2026-08-30" } } } } },
      "items": { "shape": "each-like", "min": 1,
                 "items": { "shape": "object", "members": {
                   "sku": { "shape": "string", "example": "SKU-1" },
                   "qty": { "shape": "integer", "example": 1 } } } } } })
}

#[test]
fn the_rfc_order_payload_reports_every_failure_in_one_run() {
  let values = captured(&[(
    "$.response.body",
    json!({
      "id": 42,
      "status": "REFUNDED",
      "payment": { "type": "invoice", "dueDate": "2026-08-30T00:00:00Z" },
      "items": []
    }),
  )]);
  let executed = run_body(order_payload_body(), values);
  let (status, mismatches) = outcome(&executed);
  assert_eq!(status, Status::Mismatched);

  let by_path: Vec<(&str, &str)> = mismatches
    .iter()
    .map(|m| (m.path.as_deref().unwrap_or(""), m.action.as_deref().unwrap_or("")))
    .collect();
  assert_eq!(
    by_path,
    // Object members compile depth-first in lexicographic order (shape spec §6.2): id, items,
    // payment, shippedAt, status.
    vec![
      ("$.response.body.items", "expect:size"),
      ("$.response.body.payment.dueDate", "match:date"),
      ("$.response.body.status", "match:any-of"),
    ],
    "every failure must be reported from one run, not just the first"
  );

  // The `card` alternative's subtree must never have executed: `type` was `invoice`.
  // `payment`'s one-of chain is the first `if` a depth-first walk meets (member order is
  // lexicographic: id, items, payment, shippedAt, status).
  let payment_if = find_action(&executed, "if").expect("payment compiles to a one-of if-chain");
  let card_branch = &children_of(payment_if)[1];
  assert!(
    all_unexecuted(card_branch),
    "the untaken 'card' branch must carry no results anywhere in its subtree"
  );
}

fn children_of(executed: &Executed) -> &[Executed] {
  match &executed.kind {
    ExecutedKind::Container { children, .. } | ExecutedKind::Action { children, .. } => children,
    _ => &[],
  }
}

fn find_action<'a>(executed: &'a Executed, name: &str) -> Option<&'a Executed> {
  if let ExecutedKind::Action { name: n, .. } = &executed.kind
    && n == name
  {
    return Some(executed);
  }
  for child in children_of(executed) {
    if let Some(found) = find_action(child, name) {
      return Some(found);
    }
  }
  None
}

fn all_unexecuted(executed: &Executed) -> bool {
  executed.result.is_none() && children_of(executed).iter().all(all_unexecuted)
}

// --- per-operator runtime behaviour ---

#[test]
fn each_like_reports_the_failing_elements_index() {
  let body = json!({ "shape": "each-like", "min": 1,
    "items": { "shape": "integer", "example": 1 } });
  let plan = compile(&interaction(body), &Assignment::new(), None);
  let resolver = CapturedValues::from_json(&captured(&[("$.response.body", json!([1, "not-a-number", 3]))]));
  let executed = execute(&plan, &resolver);
  let (status, mismatches) = outcome(&executed);
  assert_eq!(status, Status::Mismatched);
  assert_eq!(mismatches.len(), 1);
  assert_eq!(mismatches[0].action.as_deref(), Some("match:integer"));
}

#[test]
fn each_entry_checks_every_key_and_value() {
  let body = json!({ "shape": "each-entry", "min": 1,
    "keys": { "shape": "string", "example": "a" },
    "values": { "shape": "integer", "example": 1 } });
  let plan = compile(&interaction(body), &Assignment::new(), None);
  let resolver = CapturedValues::from_json(&captured(&[(
    "$.response.body",
    json!({ "a": 1, "b": "not-a-number" }),
  )]));
  let executed = execute(&plan, &resolver);
  let (status, mismatches) = outcome(&executed);
  assert_eq!(status, Status::Mismatched);
  assert_eq!(mismatches.len(), 1);
  assert_eq!(mismatches[0].action.as_deref(), Some("match:integer"));
}

#[test]
fn nullable_admits_null_and_the_wrapped_shape_but_nothing_else() {
  let body = json!({ "shape": "nullable", "of": { "shape": "integer", "example": 1 } });
  let plan = compile(&interaction(body.clone()), &Assignment::new(), None);

  let null_resolver = CapturedValues::from_json(&captured(&[("$.response.body", Value::Null)]));
  assert_eq!(outcome(&execute(&plan, &null_resolver)).0, Status::Matched);

  let int_resolver = CapturedValues::from_json(&captured(&[("$.response.body", json!(42))]));
  assert_eq!(outcome(&execute(&plan, &int_resolver)).0, Status::Matched);

  let string_resolver = CapturedValues::from_json(&captured(&[("$.response.body", json!("nope"))]));
  assert_eq!(outcome(&execute(&plan, &string_resolver)).0, Status::Mismatched);
}

#[test]
fn any_of_pinned_narrows_matching_at_runtime_too() {
  let body = json!({ "shape": "any-of", "options": ["PENDING", "SHIPPED"], "example": "PENDING" });
  let mut assignment = Assignment::new();
  assignment.insert("response.body#value".to_string(), "SHIPPED".to_string());
  let plan = compile(&interaction(body), &assignment, None);

  let shipped = CapturedValues::from_json(&captured(&[("$.response.body", json!("SHIPPED"))]));
  assert_eq!(outcome(&execute(&plan, &shipped)).0, Status::Matched);

  // Pinned to SHIPPED: PENDING (a value the *unpinned* shape would admit) now fails.
  let pending = CapturedValues::from_json(&captured(&[("$.response.body", json!("PENDING"))]));
  assert_eq!(outcome(&execute(&plan, &pending)).0, Status::Mismatched);
}

#[test]
fn contains_finds_a_distinct_element_per_entry() {
  let body = json!({ "shape": "contains",
    "entries": [ { "shape": "equality", "example": 1 }, { "shape": "equality", "example": 1 } ] });
  let plan = compile(&interaction(body.clone()), &Assignment::new(), None);

  let two_matches = CapturedValues::from_json(&captured(&[("$.response.body", json!([1, 2, 1]))]));
  assert_eq!(outcome(&execute(&plan, &two_matches)).0, Status::Matched);

  // Only one `1` in the array: two entries cannot each claim a distinct element.
  let one_match = CapturedValues::from_json(&captured(&[("$.response.body", json!([1, 2, 3]))]));
  assert_eq!(outcome(&execute(&plan, &one_match)).0, Status::Mismatched);
}

#[test]
fn a_component_operator_is_reported_as_unavailable_not_silently_skipped() {
  let body = json!({ "shape": "protobuf:enum", "example": "SHIPPED" });
  let plan = compile(&interaction(body), &Assignment::new(), None);
  let resolver = CapturedValues::from_json(&captured(&[("$.response.body", json!("SHIPPED"))]));
  let (status, mismatches) = outcome(&execute(&plan, &resolver));
  assert_eq!(status, Status::Mismatched);
  assert!(mismatches[0].message.contains("protobuf:enum"));
}

// --- match: family details ---

#[test]
fn match_regex_is_unanchored_like_v1_v4() {
  let body = json!({ "shape": "regex", "pattern": "\\d{4}", "example": "1234" });
  let plan = compile(&interaction(body), &Assignment::new(), None);
  let resolver = CapturedValues::from_json(&captured(&[("$.response.body", json!("card-1234-ok"))]));
  assert_eq!(outcome(&execute(&plan, &resolver)).0, Status::Matched);
}

#[test]
fn match_semver_validates_parsing() {
  let body = json!({ "shape": "semver" });
  let plan = compile(&interaction(body), &Assignment::new(), None);
  let ok = CapturedValues::from_json(&captured(&[("$.response.body", json!("1.2.3"))]));
  assert_eq!(outcome(&execute(&plan, &ok)).0, Status::Matched);
  let bad = CapturedValues::from_json(&captured(&[("$.response.body", json!("not-a-version"))]));
  assert_eq!(outcome(&execute(&plan, &bad)).0, Status::Mismatched);
}

#[test]
fn match_equality_treats_1_and_1_0_as_the_same_number() {
  let body = json!({ "shape": "equality", "example": 1 });
  let plan = compile(&interaction(body), &Assignment::new(), None);
  let resolver = CapturedValues::from_json(&captured(&[("$.response.body", json!(1.0))]));
  assert_eq!(outcome(&execute(&plan, &resolver)).0, Status::Matched);
}

// --- resolver / navigate ---

#[test]
fn captured_values_resolve_the_longest_matching_prefix() {
  let resolver = CapturedValues::from_json(&captured(&[(
    "$.response.body",
    json!({ "id": 7, "nested": { "x": 1 } }),
  )]));
  assert_eq!(
    resolver_value(&resolver, "$.response.body.id"),
    RuntimeValue::from_json(&json!(7))
  );
  assert_eq!(
    resolver_value(&resolver, "$.response.body.nested.x"),
    RuntimeValue::from_json(&json!(1))
  );
  assert_eq!(
    resolver_value(&resolver, "$.response.body.missing"),
    RuntimeValue::Absent
  );
  assert_eq!(
    resolver_value(&resolver, "$.request.method"),
    RuntimeValue::Absent
  );
}

fn resolver_value(resolver: &CapturedValues, path: &str) -> RuntimeValue {
  use pact_janus_kernel::plan::Resolver;
  resolver.resolve(path)
}

#[test]
fn a_captured_values_resolver_also_serves_as_the_interaction_context_resolver() {
  // The same mechanism plan task 3.4 names as "resolvers for HTTP request/response": capture
  // each already-decoded part under its own path.
  let resolver = CapturedValues::from_json(&captured(&[
    ("$.request.method", json!("GET")),
    ("$.request.path", json!("/orders/42")),
    ("$.response.status", json!(200)),
    ("$.response.body", json!({ "id": 42 })),
  ]));
  assert_eq!(
    resolver_value(&resolver, "$.request.method"),
    RuntimeValue::from_json(&json!("GET"))
  );
  assert_eq!(
    resolver_value(&resolver, "$.response.body.id"),
    RuntimeValue::from_json(&json!(42))
  );
}

// --- explicit NodeResult shape at the boundary ---

#[test]
fn check_exists_yields_a_boolean_never_an_error() {
  let body = json!({ "shape": "optional", "of": { "shape": "integer", "example": 1 } });
  let plan = compile(
    &interaction(json!({ "shape": "object", "members": { "field": body } })),
    &Assignment::new(),
    None,
  );
  let resolver = CapturedValues::from_json(&BTreeMap::new());
  let executed = execute(&plan, &resolver);
  let check = find_action(&executed, "check:exists").expect("optional compiles check:exists");
  assert_eq!(check.result, Some(NodeResult::Value(RuntimeValue::Bool(false))));
}
