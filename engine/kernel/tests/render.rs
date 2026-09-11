//! Plan task 3.6: the pretty and executed text forms (plan-grammar spec §3), checked byte-for-byte
//! against the worked examples checked into the spec —
//! `Documentation/specs/plan-grammar/examples/corpus-case.md` (§2 `plan.txt`, §3 `executed.txt`)
//! and `order-payload-plan.md` (§2's pinned-variant subtrees, §3's executed-and-failing subtrees).
//! `order-payload-plan.md` §1's full tree additionally carries a content-component annotation
//! (`#{'decoded by the content component for application/json'}`) that no compiler can emit yet —
//! content components are plan task 4.2 — so this file checks that document's *structural* subtrees
//! (§2, §3) rather than diffing the whole annotated tree.

use pact_janus_kernel::interaction_spec::{InteractionSpec, parse};
use pact_janus_kernel::plan::{Assignment, CapturedValues, compile, execute, render_executed, render_pretty};
use serde_json::{Value, json};
use std::collections::BTreeMap;

fn interaction(body: Value) -> InteractionSpec {
  let spec = json!({
    "description": "a request for an order",
    "parts": { "response": { "body": body } }
  });
  parse(&spec).unwrap_or_else(|err| panic!("expected a well-formed spec, got {:?}", err.problems))
}

fn captured(pairs: &[(&str, Value)]) -> BTreeMap<String, Value> {
  pairs.iter().map(|(k, v)| (k.to_string(), v.clone())).collect()
}

/// `corpus-case.md` §1's shape.
fn corpus_case_body() -> Value {
  json!({ "shape": "object",
    "members": {
      "id": { "shape": "integer", "example": 42 },
      "shippedAt": { "shape": "optional",
                     "of": { "shape": "datetime", "format": "yyyy-MM-dd'T'HH:mm:ssX",
                             "example": "2026-07-30T10:00:00Z" } } } })
}

#[test]
fn pretty_form_matches_corpus_case_plan_txt() {
  let plan = compile(&interaction(corpus_case_body()), &Assignment::new(), None);
  let expected = r#"(
  :"a request for an order" (
    :response (
      :body (
        :"$.id" (
          %match:integer (
            $.response.body.id
          )
        ),
        :"$.shippedAt" (
          %if (
            %check:exists (
              $.response.body.shippedAt
            ),
            %match:datetime (
              $.response.body.shippedAt,
              'yyyy-MM-dd\'T\'HH:mm:ssX'
            )
          )
        )
      )
    )
  )
)"#;
  pretty_assertions::assert_eq!(render_pretty(&plan), expected);
}

#[test]
fn executed_form_matches_corpus_case_executed_txt() {
  let plan = compile(&interaction(corpus_case_body()), &Assignment::new(), None);
  let values = captured(&[("$.response.body", json!({ "id": 7, "warehouse": "AKL-1" }))]);
  let resolver = CapturedValues::from_json(&values);
  let executed = execute(&plan, &resolver);
  let expected = r#"(
  :"a request for an order" (
    :response (
      :body (
        :"$.id" (
          %match:integer (
            $.response.body.id => 7
          ) => BOOL(true)
        ) => BOOL(true),
        :"$.shippedAt" (
          %if (
            %check:exists (
              $.response.body.shippedAt => NULL
            ) => BOOL(false),
            %match:datetime (
              $.response.body.shippedAt,
              'yyyy-MM-dd\'T\'HH:mm:ssX'
            )
          ) => BOOL(true)
        ) => BOOL(true)
      ) => BOOL(true)
    ) => BOOL(true)
  ) => BOOL(true)
)"#;
  pretty_assertions::assert_eq!(render_executed(&executed), expected);
}

/// `order-payload-plan.md` §2: the response body's shape (design 2.2's worked example, minus the
/// non-JSON parts no compiler here touches) pinned under
/// `response.body.shippedAt#presence=absent;response.body.status#value=SHIPPED` narrows the
/// `any-of`/`optional` subtrees exactly as §2's two blocks show.
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

fn find_container<'a>(plan_text: &'a str, label: &str) -> &'a str {
  plan_text
    .find(label)
    .map(|i| &plan_text[i..])
    .unwrap_or_else(|| panic!("label {label:?} not found in:\n{plan_text}"))
}

#[test]
fn pretty_form_pinned_variant_matches_order_payload_plan_section_2() {
  let mut assignment = Assignment::new();
  assignment.insert(
    "response.body.shippedAt#presence".to_string(),
    "absent".to_string(),
  );
  assignment.insert("response.body.status#value".to_string(), "SHIPPED".to_string());
  let plan = compile(&interaction(order_payload_body()), &assignment, None);
  let text = render_pretty(&plan);

  // Members render alphabetically (`compile_object`'s `BTreeMap` iteration): id, items,
  // shippedAt, status — so `shippedAt` is followed by a sibling (a trailing comma) and `status`,
  // alphabetically last, is not.
  let status = find_container(&text, ":\"$.status\" (");
  assert!(
    status.starts_with(
      r#":"$.status" (
          %match:equality (
            $.response.body.status,
            'SHIPPED'
          )
        )"#
    ),
    "status subtree:\n{status}"
  );

  let shipped_at = find_container(&text, ":\"$.shippedAt\" (");
  assert!(
    shipped_at.starts_with(
      r#":"$.shippedAt" (
          %expect:absent (
            $.response.body.shippedAt
          )
        ),"#
    ),
    "shippedAt subtree:\n{shipped_at}"
  );
}

#[test]
fn executed_form_matches_order_payload_plan_section_3() {
  let plan = compile(&interaction(order_payload_body()), &Assignment::new(), None);
  let values = captured(&[(
    "$.response.body",
    json!({
      "id": 42,
      "status": "REFUNDED",
      "payment": { "type": "invoice", "dueDate": "2026-08-30T00:00:00Z" },
      "items": []
    }),
  )]);
  let resolver = CapturedValues::from_json(&values);
  let executed = execute(&plan, &resolver);
  let text = render_executed(&executed);

  // Members render alphabetically: id, items, payment, shippedAt, status — status is last.
  let status = find_container(&text, ":\"$.status\" (");
  assert!(
    status.starts_with(
      r#":"$.status" (
          %match:any-of (
            $.response.body.status => 'REFUNDED',
            'PENDING',
            'SHIPPED',
            'DELIVERED'
          ) => ERROR(Expected 'REFUNDED' to be one of 'PENDING', 'SHIPPED', 'DELIVERED')
        ) => BOOL(false)"#
    ),
    "status subtree:\n{status}"
  );

  // The `:card` alternative was never taken (`%if` is lazy) — no `=> ` appears anywhere under it.
  let payment = find_container(&text, ":\"$.payment\" (");
  let card_start = payment.find(":card (").expect(":card subtree");
  let card_end = card_start + payment[card_start..].find("),\n").expect("end of :card subtree");
  let card = &payment[card_start..card_end];
  assert!(
    !card.contains("=>"),
    "the untaken :card branch was rendered as executed:\n{card}"
  );

  let items = find_container(&text, ":\"$.items\" (");
  assert!(
    items.contains("%expect:size (\n            $.response.body.items => [],\n            1,\n            NULL\n          ) => ERROR(Expected at least 1 item(s) but got 0)"),
    "items subtree:\n{items}"
  );
}

/// Not part of a worked example: the empty-conjunction `and()` (`nullable`'s null branch, plan task
/// 3.3) and a `pipeline`/`resolve-current` fragment (corpus-case.md §5's hoisting illustration) —
/// covering the two node kinds the byte-for-byte checks above never happen to exercise.
mod fragments {
  use pact_janus_kernel::plan::{Literal, Node};

  #[test]
  fn empty_and_renders_as_a_single_line() {
    let node = Node::action("and", vec![]);
    pretty_assertions::assert_eq!(super::render_one(&node), "%and ()");
  }

  #[test]
  fn pipeline_and_resolve_current_render_with_their_own_sigils() {
    let node = Node::pipeline(vec![
      Node::resolve("$.response.body.shippedAt"),
      Node::action(
        "if",
        vec![
          Node::action("check:exists", vec![Node::resolve_current("~>")]),
          Node::action(
            "match:datetime",
            vec![
              Node::resolve_current("~>"),
              Node::value(Literal::string("yyyy-MM-dd")),
            ],
          ),
        ],
      ),
    ]);
    let expected = r#"-> (
  $.response.body.shippedAt,
  %if (
    %check:exists (
      ~>
    ),
    %match:datetime (
      ~>,
      'yyyy-MM-dd'
    )
  )
)"#;
    pretty_assertions::assert_eq!(super::render_one(&node), expected);
  }
}

/// Renders a single [`Node`] as a whole document (the same envelope [`render_pretty`] wraps a
/// [`Plan`]'s root in), for fragment-level tests that build a `Node` directly rather than through
/// the shape compiler.
fn render_one(node: &pact_janus_kernel::plan::Node) -> String {
  use pact_janus_kernel::plan::{GRAMMAR_VERSION, Plan};
  let plan = Plan {
    grammar: GRAMMAR_VERSION,
    root: node.clone(),
    variant: None,
  };
  // A bare node has no outer envelope of its own to strip — render_pretty always adds one layer
  // of `(...)`, so unwrap it back off for a fragment-level comparison against the node's own text.
  let wrapped = render_pretty(&plan);
  wrapped
    .strip_prefix("(\n")
    .and_then(|s| s.strip_suffix("\n)"))
    .map(|s| {
      s.lines()
        .map(|line| line.strip_prefix("  ").unwrap_or(line))
        .collect::<Vec<_>>()
        .join("\n")
    })
    .unwrap_or(wrapped)
}
