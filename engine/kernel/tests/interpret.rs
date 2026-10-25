//! Plan task 3.4: the plan interpreter and resolvers, checked against the worked examples in
//! `Documentation/specs/plan-grammar/examples/corpus-case.md` and
//! `Documentation/specs/plan-grammar/examples/order-payload-plan.md` §3, plus per-action coverage
//! of the compiled forms plan task 3.3 actually produces.

use pact_janus_kernel::interaction_spec::{InteractionSpec, parse};
use pact_janus_kernel::plan::{
  Assignment, CapturedValues, ContentDetector, Executed, ExecutedKind, GRAMMAR_VERSION, Literal, Node,
  NodeResult, Plan, RuntimeValue, Status, actions, compile, execute, execute_with_content, outcome,
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
  // `payment`'s one-of chain is the first `flow:if` a depth-first walk meets (member order is
  // lexicographic: id, items, payment, shippedAt, status).
  let payment_if = find_action(&executed, "flow:if").expect("payment compiles to a one-of if-chain");
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

/// `legacy:array-contains` (ADR 0042) is today's `ArrayContains` variant: some element satisfies the
/// template, and no element is claimed, so one element satisfies two variants where
/// `match:contains` above would fail. A variant none matched fails alone, in today's words.
#[test]
fn array_contains_lets_one_element_satisfy_two_variants() {
  let variant = |index: u64| {
    Node::action(
      "legacy:array-contains",
      vec![
        Node::resolve("$.response.body"),
        Node::container(
          None,
          vec![Node::action(
            "match:integer",
            vec![Node::resolve_current("~>.id")],
          )],
        ),
        Node::value(Literal::number(index)),
        Node::value(Literal::from_json(&json!({ "id": index }))),
      ],
    )
  };
  let plan = Plan {
    grammar: GRAMMAR_VERSION,
    root: Node::container(None, vec![variant(0), variant(1)]),
    variant: None,
  };

  let one_element = CapturedValues::from_json(&captured(&[("$.response.body", json!([{ "id": 7 }]))]));
  assert_eq!(outcome(&execute(&plan, &one_element)).0, Status::Matched);

  let none = CapturedValues::from_json(&captured(&[("$.response.body", json!([{ "id": "x" }]))]));
  let (status, mismatches) = outcome(&execute(&plan, &none));
  assert_eq!(status, Status::Mismatched);
  let messages: Vec<&str> = mismatches.iter().map(|m| m.message.as_str()).collect();
  assert_eq!(
    messages,
    vec![
      "Variant at index 0 ({\"id\":0}) was not found in the actual list",
      "Variant at index 1 ({\"id\":1}) was not found in the actual list",
    ],
    "only the variants, not what each element failed on"
  );
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

// --- match:content-type asks a component, plan task 4.2 (kernel-boundary-review.md finding 1) ---

struct FakeDetector {
  answer: Option<&'static str>,
}

impl ContentDetector for FakeDetector {
  fn detect(&self, _value: &RuntimeValue, _at: Option<&str>) -> Option<String> {
    self.answer.map(str::to_string)
  }
}

fn content_type_body() -> Value {
  json!({ "shape": "content-type", "content-type": "application/json" })
}

#[test]
fn content_type_matches_via_a_real_detector() {
  let plan = compile(&interaction(content_type_body()), &Assignment::new(), None);
  let resolver = CapturedValues::from_json(&captured(&[("$.response.body", json!("{\"a\":1}"))]));
  let detector = FakeDetector {
    answer: Some("application/json"),
  };
  let executed = execute_with_content(&plan, &resolver, Some(&detector));
  let (status, mismatches) = outcome(&executed);
  assert_eq!(status, Status::Matched, "mismatches: {mismatches:?}");
}

#[test]
fn content_type_reports_the_detectors_disagreement() {
  let plan = compile(&interaction(content_type_body()), &Assignment::new(), None);
  let resolver = CapturedValues::from_json(&captured(&[("$.response.body", json!("<a/>"))]));
  let detector = FakeDetector {
    answer: Some("application/xml"),
  };
  let executed = execute_with_content(&plan, &resolver, Some(&detector));
  let (status, mismatches) = outcome(&executed);
  assert_eq!(status, Status::Mismatched);
  assert_eq!(
    mismatches[0].message,
    "Expected content type 'application/json' but detected 'application/xml'"
  );
}

#[test]
fn content_type_without_a_detector_is_a_distinct_error_not_a_guess() {
  // Before plan task 4.2, this case's kernel-side guess happened to succeed on `{"a":1}` — the
  // exact finding kernel-boundary-review.md's finding 1 flagged. `execute` (no detector) must now
  // say plainly that nothing is loaded, not fall back to guessing.
  let executed = run_body(
    content_type_body(),
    captured(&[("$.response.body", json!("{\"a\":1}"))]),
  );
  let (status, mismatches) = outcome(&executed);
  assert_eq!(status, Status::Mismatched);
  assert_eq!(
    mismatches[0].message,
    "no content component is loaded to detect content type"
  );
}

// --- staged plan S0.13: a malformed action node is an error on the node, never a panic ---
//
// The engine's own compilers always give an action the children it takes, so nothing above can
// reach these paths. A component's fragment can, and so can a plan document handed back in.

/// Executes `node` on its own, and says which node it was if the interpreter panics.
fn run_node(node: Node, what: &str) -> Executed {
  let plan = Plan {
    grammar: GRAMMAR_VERSION,
    root: Node::container(Some("probe".to_string()), vec![node]),
    variant: None,
  };
  let values = captured(&[("$.v", json!([1, "two", { "three": 3 }]))]);
  let resolver = CapturedValues::from_json(&values);
  std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| execute(&plan, &resolver)))
    .unwrap_or_else(|_| panic!("the interpreter panicked on {what}"))
}

#[test]
fn every_core_action_given_fewer_children_than_it_takes_is_an_error_on_the_node() {
  let mut short_nodes = 0;
  for name in actions::core_actions() {
    let least = actions::least_children(name).expect("a core action has a least child count");
    for given in 0..least {
      let children = (0..given).map(|_| Node::resolve("$.v")).collect();
      let what = format!("'{name}' with {given} of at least {least} children");
      let executed = run_node(Node::action(name, children), &what);
      let node = find_action(&executed, name).expect("the action is in the executed plan");
      match &node.result {
        Some(NodeResult::Error { message, .. }) => assert!(
          message.contains(&format!("'{name}' takes at least {least} child node(s)"))
            && message.contains(&format!("has {given}")),
          "{what}: {message}"
        ),
        other => panic!("{what}: expected an error on the node, got {other:?}"),
      }
      assert!(
        children_of(node).iter().all(all_unexecuted),
        "{what}: nothing under a malformed node runs"
      );
      assert_eq!(outcome(&executed).0, Status::Mismatched, "{what}");
      short_nodes += 1;
    }
  }
  // Plan-grammar spec §4.2's table, counted: 29 actions take at least one child, 15 at least two,
  // 4 at least three and 1 four (and 5 take none), so an action added without its count shows here.
  assert_eq!(
    short_nodes,
    29 + 15 * 2 + 4 * 3 + 4,
    "every short form of every core action"
  );
}

#[test]
fn no_core_action_panics_whatever_its_children_are() {
  type Make = fn() -> Node;
  let kinds: Vec<(&str, Make)> = vec![
    ("a present value", || Node::resolve("$.v")),
    ("an absent value", || Node::resolve("$.nowhere")),
    ("null", || Node::value(Literal::null())),
    ("a string", || Node::value(Literal::string("[unclosed"))),
    ("a number", || Node::value(Literal::number(3))),
    ("an object", || {
      Node::value(Literal::from_json(&json!({ "a": [1, 2] })))
    }),
    ("an array", || {
      Node::value(Literal::from_json(&json!([[], {}, null])))
    }),
    ("an empty splat", || Node::splat(vec![])),
    ("an empty container", || Node::container(None, vec![])),
    ("the current item, outside any iteration", || {
      Node::resolve_current("~>.x")
    }),
    ("an action with nothing under it", || {
      Node::action("flow:and", vec![])
    }),
  ];
  for name in actions::core_actions() {
    let least = actions::least_children(name).unwrap();
    for (kind, make) in &kinds {
      for given in least..=least + 2 {
        // Every child of the one kind, and the same with a real value under test in front.
        let uniform: Vec<Node> = (0..given).map(|_| make()).collect();
        run_node(
          Node::action(name, uniform),
          &format!("'{name}' with {given} × {kind}"),
        );
        if given > 0 {
          let mut led: Vec<Node> = vec![Node::resolve("$.v")];
          led.extend((1..given).map(|_| make()));
          run_node(
            Node::action(name, led),
            &format!("'{name}' with a value and {} × {kind}", given - 1),
          );
        }
      }
    }
  }
}

// --- legacy:body-type (ADR 0036) ---

fn body_type_of(expected: &str, actual: Option<&str>) -> Result<(), String> {
  let plan = Plan {
    grammar: GRAMMAR_VERSION,
    root: Node::action(
      "legacy:body-type",
      vec![
        Node::resolve("$.response.headers.content-type"),
        Node::value(Literal::string(expected)),
      ],
    ),
    variant: None,
  };
  let mut resolver = CapturedValues::new();
  if let Some(actual) = actual {
    resolver = resolver.capture(
      "$.response.headers.content-type",
      RuntimeValue::String(actual.to_string()),
    );
  }
  let (status, mismatches) = outcome(&execute(&plan, &resolver));
  match status {
    Status::Matched => Ok(()),
    _ => Err(
      mismatches
        .into_iter()
        .map(|m| m.message)
        .collect::<Vec<_>>()
        .join("; "),
    ),
  }
}

#[rstest::rstest]
#[case::unlabelled("text/plain", None)]
#[case::the_same_type("text/plain", Some("text/plain"))]
#[case::with_parameters("text/plain", Some("Text/Plain; charset=utf-8"))]
#[case::a_suffix_type("application/json", Some("application/hal+json"))]
#[case::an_unknown_label("text/plain", Some("*/*"))]
#[case::not_a_type_at_all("text/plain", Some("nonsense"))]
#[case::the_same_parameters("text/plain; charset=utf-8", Some("text/plain;charset=utf-8"))]
fn a_body_type_agrees(#[case] expected: &str, #[case] actual: Option<&str>) {
  assert_eq!(body_type_of(expected, actual), Ok(()));
}

#[rstest::rstest]
#[case::another_subtype("text/plain", "text/html")]
#[case::another_type("application/json", "text/plain")]
#[case::a_suffix_that_is_not_the_subtype("application/json", "application/hal+xml")]
#[case::other_parameters("text/plain; charset=utf-8", "text/plain; charset=iso-8859-1")]
fn a_body_type_disagrees(#[case] expected: &str, #[case] actual: &str) {
  assert_eq!(
    body_type_of(expected, Some(actual)),
    Err(format!(
      "Expected a body of '{expected}' but the actual content type was '{actual}'"
    ))
  );
}
