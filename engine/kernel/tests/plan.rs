//! Plan task 3.3: the shape/interaction-spec -> plan compiler (plan-grammar spec §5), checked
//! operator by operator against §5.2's table, and against the worked example in
//! `Documentation/specs/plan-grammar/examples/order-payload-plan.md`.

use pact_janus_kernel::interaction_spec::{InteractionSpec, parse};
use pact_janus_kernel::plan::{Assignment, DocumentKind, Node, NodeKind, Plan, compile};
use serde_json::{Value, json};

fn interaction(body: Value) -> InteractionSpec {
  let spec = json!({
    "description": "get an order",
    "parts": { "response": { "body": body } }
  });
  parse(&spec).unwrap_or_else(|err| panic!("expected a well-formed spec, got {:?}", err.problems))
}

/// Compile the response body's shape, unpinned, and hand back the `:body(...)` container itself —
/// for operators that splice more than one node at the slot root (`object`, `array`, `each-like`,
/// `each-entry`).
fn compile_slot(body: Value) -> Node {
  let plan = compile(&interaction(body), &Assignment::new(), None);
  slot_child(&plan)
}

fn compile_slot_under(body: Value, assignment: &Assignment) -> Node {
  let plan = compile(&interaction(body), assignment, None);
  slot_child(&plan)
}

fn slot_child(plan: &Plan) -> Node {
  let root_children = children_of(&plan.root);
  assert_eq!(root_children.len(), 1, "one part: response");
  let response_children = children_of(&root_children[0]);
  assert_eq!(response_children.len(), 1, "one slot: body");
  response_children[0].clone()
}

/// Compile the response body's shape, unpinned, and hand back the single node it compiles to —
/// every operator except `object`, `array`, `each-like` and `each-entry` splices exactly one
/// (spec §5.2), so this unwraps the `:body(...)` container's one child.
fn compile_body(body: Value) -> Node {
  only_child(&compile_slot(body))
}

fn compile_body_under(body: Value, assignment: &Assignment) -> Node {
  only_child(&compile_slot_under(body, assignment))
}

/// Compile a shape that admits absence (`optional`, `forbidden`) as an object member — the only
/// position spec §5.1 allows one — and hand back the compiled node under
/// `$.response.body.field`. `dim_id_prefix` for these shapes is `"response.body.field"`.
fn compile_field(field_shape: Value) -> Node {
  compile_field_under(field_shape, &Assignment::new())
}

fn compile_field_under(field_shape: Value, assignment: &Assignment) -> Node {
  let body = json!({ "shape": "object", "members": { "field": field_shape } });
  let plan = compile(&interaction(body), assignment, None);
  let member_container = only_child(&slot_child(&plan));
  assert_eq!(label_of(&member_container), Some("$.response.body.field"));
  only_child(&member_container)
}

fn children_of(node: &Node) -> &Vec<Node> {
  match &node.kind {
    NodeKind::Container { children, .. } => children,
    other => panic!("expected a container, got {other:?}"),
  }
}

fn label_of(node: &Node) -> Option<&str> {
  match &node.kind {
    NodeKind::Container { label, .. } => label.as_deref(),
    other => panic!("expected a container, got {other:?}"),
  }
}

/// Unwrap the one child of a container that exists only to bundle a single compiled node under
/// the slot label — every value operator, and every dimensional operator's *node* (`if`, an
/// `expect:*`, `match:any-of`, ...) compiles to exactly one node (spec §5.2), so a slot whose root
/// shape is one of those has a single-child slot container.
fn only_child(node: &Node) -> Node {
  let children = children_of(node);
  assert_eq!(children.len(), 1, "expected exactly one child, got {children:?}");
  children[0].clone()
}

/// Owned rather than borrowed, so `action(&compile_body(...))` can be called on an unbound
/// temporary without the borrow checker objecting to its lifetime.
fn action(node: &Node) -> (String, Vec<Node>) {
  match &node.kind {
    NodeKind::Action { name, children } => (name.clone(), children.clone()),
    other => panic!("expected an action, got {other:?}"),
  }
}

fn resolve_path(node: &Node) -> &str {
  match &node.kind {
    NodeKind::Resolve { path } => path,
    other => panic!("expected a resolve node, got {other:?}"),
  }
}

fn resolve_current_path(node: &Node) -> &str {
  match &node.kind {
    NodeKind::ResolveCurrent { path } => path,
    other => panic!("expected a resolve-current node, got {other:?}"),
  }
}

fn literal_value(node: &Node) -> &Value {
  match &node.kind {
    NodeKind::Value(literal) => &literal.value,
    other => panic!("expected a value node, got {other:?}"),
  }
}

// --- plan structure: root/part/slot wrapping ---

#[test]
fn the_plan_root_is_wrapped_by_description_part_and_slot() {
  let plan = compile(
    &interaction(json!({ "shape": "integer", "example": 42 })),
    &Assignment::new(),
    None,
  );
  assert_eq!(plan.grammar, "v0");
  assert_eq!(plan.variant, None);
  assert_eq!(label_of(&plan.root), Some("get an order"));

  let parts = children_of(&plan.root);
  assert_eq!(label_of(&parts[0]), Some("response"));

  let slots = children_of(&parts[0]);
  assert_eq!(label_of(&slots[0]), Some("body"));
}

#[test]
fn a_variant_id_is_stamped_onto_the_plan() {
  let plan = compile(
    &interaction(json!({ "shape": "integer", "example": 42 })),
    &Assignment::new(),
    Some("response.body.status#value=SHIPPED"),
  );
  assert_eq!(
    plan.variant.as_deref(),
    Some("response.body.status#value=SHIPPED")
  );
}

// --- value operators (plan-grammar spec §4.3, §5.2) ---

#[test]
fn any_compiles_to_a_bare_match_action() {
  let (name, children) = action(&compile_body(json!({ "shape": "any" })));
  assert_eq!(name, "match:any");
  assert_eq!(resolve_path(&children[0]), "$.response.body");
}

#[test]
fn equality_carries_its_example_as_a_parameter() {
  let (name, children) = action(&compile_body(
    json!({ "shape": "equality", "example": "PENDING" }),
  ));
  assert_eq!(name, "match:equality");
  assert_eq!(literal_value(&children[1]), &json!("PENDING"));
}

#[test]
fn type_carries_its_example_as_a_parameter() {
  let (name, children) = action(&compile_body(json!({ "shape": "type", "example": 42 })));
  assert_eq!(name, "match:type");
  assert_eq!(literal_value(&children[1]), &json!(42));
}

#[test]
fn kind_predicates_take_only_the_resolved_value() {
  for (shape, expected) in [
    ("string", "match:string"),
    ("number", "match:number"),
    ("integer", "match:integer"),
    ("decimal", "match:decimal"),
    ("boolean", "match:boolean"),
    ("null", "match:null"),
  ] {
    let (name, children) = action(&compile_body(json!({ "shape": shape })));
    assert_eq!(name, expected);
    assert_eq!(children.len(), 1, "{shape} takes only the resolved value");
  }
}

#[test]
fn not_empty_is_a_structural_assertion_not_a_match() {
  let (name, _) = action(&compile_body(json!({ "shape": "not-empty" })));
  assert_eq!(name, "expect:not-empty");
}

#[test]
fn regex_carries_its_pattern() {
  let (name, children) = action(&compile_body(
    json!({ "shape": "regex", "pattern": "\\d{4}", "example": "1234" }),
  ));
  assert_eq!(name, "match:regex");
  assert_eq!(literal_value(&children[1]), &json!("\\d{4}"));
}

#[test]
fn temporal_operators_omit_the_format_child_when_format_is_absent() {
  let (name, children) = action(&compile_body(
    json!({ "shape": "datetime", "example": "2026-07-30T10:00:00Z" }),
  ));
  assert_eq!(name, "match:datetime");
  assert_eq!(children.len(), 1);
}

#[test]
fn temporal_operators_carry_their_format_when_present() {
  let (name, children) = action(&compile_body(
    json!({ "shape": "date", "format": "yyyy-MM-dd", "example": "2026-08-30" }),
  ));
  assert_eq!(name, "match:date");
  assert_eq!(literal_value(&children[1]), &json!("yyyy-MM-dd"));
}

#[test]
fn include_and_content_type_carry_their_parameter() {
  let (name, children) = action(&compile_body(
    json!({ "shape": "include", "substring": "application/json" }),
  ));
  assert_eq!(name, "match:include");
  assert_eq!(literal_value(&children[1]), &json!("application/json"));

  let (name, children) = action(&compile_body(
    json!({ "shape": "content-type", "content-type": "image/png" }),
  ));
  assert_eq!(name, "match:content-type");
  assert_eq!(literal_value(&children[1]), &json!("image/png"));
}

#[test]
fn semver_takes_only_the_resolved_value() {
  let (name, children) = action(&compile_body(json!({ "shape": "semver" })));
  assert_eq!(name, "match:semver");
  assert_eq!(children.len(), 1);
}

// --- object (plan-grammar spec §5.2) ---

#[test]
fn object_compiles_to_one_container_per_named_member_and_ignores_nothing_else() {
  let body = compile_slot(json!({ "shape": "object",
    "members": {
      "id": { "shape": "integer", "example": 42 },
      "status": { "shape": "string", "example": "PENDING" } } }));
  let members = children_of(&body);
  assert_eq!(
    members.len(),
    2,
    "must-ignore: no node for members not named (spec §5.2)"
  );
  assert_eq!(label_of(&members[0]), Some("$.response.body.id"));
  assert_eq!(label_of(&members[1]), Some("$.response.body.status"));

  let (name, children) = action(&children_of(&members[0])[0]);
  assert_eq!(name, "match:integer");
  assert_eq!(resolve_path(&children[0]), "$.response.body.id");
}

// --- array (plan-grammar spec §5.2) ---

#[test]
fn array_asserts_the_exact_count_and_compiles_one_container_per_index() {
  let body = compile_slot(json!({ "shape": "array",
    "entries": [ { "shape": "integer", "example": 1 }, { "shape": "string", "example": "a" } ] }));
  let nodes = children_of(&body);
  assert_eq!(nodes.len(), 3, "expect:count, then one container per index");
  let (name, count_children) = action(&nodes[0]);
  assert_eq!(name, "expect:count");
  assert_eq!(literal_value(&count_children[1]), &json!(2));

  assert_eq!(label_of(&nodes[1]), Some("$.response.body[0]"));
  assert_eq!(label_of(&nodes[2]), Some("$.response.body[1]"));
  let (name, index_children) = action(&children_of(&nodes[1])[0]);
  assert_eq!(name, "match:integer");
  assert_eq!(resolve_path(&index_children[0]), "$.response.body[0]");
}

// --- each-like (plan-grammar spec §5.2) ---

fn each_like_body() -> Value {
  json!({ "shape": "each-like", "min": 1,
          "items": { "shape": "object",
                     "members": {
                       "sku": { "shape": "string", "example": "SKU-1" },
                       "qty": { "shape": "integer", "example": 1 } } } })
}

#[test]
fn each_like_unpinned_asserts_size_and_for_eachs_over_a_splat() {
  let body = compile_slot(each_like_body());
  let nodes = children_of(&body);
  assert_eq!(nodes.len(), 2);

  let (name, size_children) = action(&nodes[0]);
  assert_eq!(name, "expect:size");
  assert_eq!(literal_value(&size_children[1]), &json!(1));
  assert_eq!(literal_value(&size_children[2]), &Value::Null);

  let (name, for_each_children) = action(&nodes[1]);
  assert_eq!(name, "for-each");
  let (splat_name, splat_children) = match &for_each_children[0].kind {
    NodeKind::Splat { children } => ("splat", children),
    other => panic!("expected a splat, got {other:?}"),
  };
  assert_eq!(splat_name, "splat");
  assert_eq!(resolve_path(&splat_children[0]), "$.response.body");

  let item = &for_each_children[1];
  assert_eq!(label_of(item), Some("$.response.body[*]"));
  let item_members = children_of(item);
  // `BTreeMap` iteration is lexicographic, so "qty" sorts before "sku".
  assert_eq!(label_of(&item_members[0]), Some("$.response.body[*].qty"));
  assert_eq!(label_of(&item_members[1]), Some("$.response.body[*].sku"));
  let (name, sku_children) = action(&children_of(&item_members[1])[0]);
  assert_eq!(name, "match:string");
  assert_eq!(resolve_current_path(&sku_children[0]), "~>.sku");
}

#[test]
fn each_like_pinned_to_min_plus_1_narrows_to_expect_count() {
  let mut assignment = Assignment::new();
  assignment.insert("response.body#cardinality".to_string(), "min+1".to_string());
  let body = compile_slot_under(each_like_body(), &assignment);
  let nodes = children_of(&body);
  let (name, children) = action(&nodes[0]);
  assert_eq!(name, "expect:count");
  assert_eq!(literal_value(&children[1]), &json!(2));
}

#[test]
fn each_like_pinned_to_an_unknown_point_falls_back_to_the_general_form() {
  let mut assignment = Assignment::new();
  assignment.insert("response.body#cardinality".to_string(), "bogus".to_string());
  let body = compile_slot_under(each_like_body(), &assignment);
  let (name, _) = action(&children_of(&body)[0]);
  assert_eq!(name, "expect:size");
}

// --- each-entry ---

#[test]
fn each_entry_checks_keys_and_values_against_the_current_entry() {
  let body = compile_slot(json!({ "shape": "each-entry", "min": 1,
    "keys": { "shape": "string", "example": "content-type" },
    "values": { "shape": "string", "example": "application/json" } }));
  let nodes = children_of(&body);
  let (name, for_each_children) = action(&nodes[1]);
  assert_eq!(name, "for-each");
  let item = &for_each_children[1];
  let item_children = children_of(item);
  assert_eq!(item_children.len(), 2, "one container for keys, one for values");

  let (key_name, key_children) = action(&children_of(&item_children[0])[0]);
  assert_eq!(key_name, "match:string");
  assert_eq!(resolve_current_path(&key_children[0]), "~>.key");

  let (value_name, value_children) = action(&children_of(&item_children[1])[0]);
  assert_eq!(value_name, "match:string");
  assert_eq!(resolve_current_path(&value_children[0]), "~>.value");
}

// --- optional (plan-grammar spec §5.2) ---

#[test]
fn optional_unpinned_is_an_if_on_check_exists_with_an_unwrapped_present_branch() {
  let body = compile_field(json!({ "shape": "optional",
    "of": { "shape": "datetime", "format": "yyyy-MM-dd'T'HH:mm:ssX", "example": "2026-07-30T10:00:00Z" } }));
  let (name, children) = action(&body);
  assert_eq!(name, "if");
  let (cond_name, cond_children) = action(&children[0]);
  assert_eq!(cond_name, "check:exists");
  assert_eq!(resolve_path(&cond_children[0]), "$.response.body.field");
  // The present branch is the bare match node, not wrapped in a container (matches
  // order-payload-plan.md's `%if(%check:exists(...), %match:datetime(...))`).
  let (branch_name, _) = action(&children[1]);
  assert_eq!(branch_name, "match:datetime");
  assert_eq!(
    children.len(),
    2,
    "no explicit else: absence is 'ok' by the if action's default"
  );
}

#[test]
fn optional_pinned_absent_narrows_to_expect_absent() {
  let mut assignment = Assignment::new();
  assignment.insert("response.body.field#presence".to_string(), "absent".to_string());
  let body = compile_field_under(
    json!({ "shape": "optional", "of": { "shape": "integer", "example": 1 } }),
    &assignment,
  );
  let (name, _) = action(&body);
  assert_eq!(name, "expect:absent");
}

#[test]
fn optional_pinned_present_narrows_to_the_wrapped_shape_with_no_branch() {
  let mut assignment = Assignment::new();
  assignment.insert("response.body.field#presence".to_string(), "present".to_string());
  let body = compile_field_under(
    json!({ "shape": "optional", "of": { "shape": "integer", "example": 1 } }),
    &assignment,
  );
  let (name, _) = action(&body);
  assert_eq!(name, "match:integer");
}

// --- forbidden ---

#[test]
fn forbidden_is_always_expect_absent() {
  let (name, _) = action(&compile_field(json!({ "shape": "forbidden" })));
  assert_eq!(name, "expect:absent");
}

// --- nullable ---

#[test]
fn nullable_unpinned_is_an_if_on_check_null_with_an_explicit_ok_branch() {
  let body = compile_body(json!({ "shape": "nullable", "of": { "shape": "integer", "example": 1 } }));
  let (name, children) = action(&body);
  assert_eq!(name, "if");
  assert_eq!(children.len(), 3);
  let (cond_name, _) = action(&children[0]);
  assert_eq!(cond_name, "check:null");
  let (ok_name, ok_children) = action(&children[1]);
  assert_eq!(ok_name, "and");
  assert!(ok_children.is_empty());
  let (else_name, _) = action(&children[2]);
  assert_eq!(else_name, "match:integer");
}

#[test]
fn nullable_pinned_null_narrows_to_match_null() {
  let mut assignment = Assignment::new();
  assignment.insert("response.body#nullability".to_string(), "null".to_string());
  let body = compile_body_under(
    json!({ "shape": "nullable", "of": { "shape": "integer", "example": 1 } }),
    &assignment,
  );
  let (name, _) = action(&body);
  assert_eq!(name, "match:null");
}

#[test]
fn nullable_pinned_non_null_narrows_to_the_wrapped_shape() {
  let mut assignment = Assignment::new();
  assignment.insert("response.body#nullability".to_string(), "non-null".to_string());
  let body = compile_body_under(
    json!({ "shape": "nullable", "of": { "shape": "integer", "example": 1 } }),
    &assignment,
  );
  let (name, _) = action(&body);
  assert_eq!(name, "match:integer");
}

// --- any-of ---

#[test]
fn any_of_unpinned_lists_every_option() {
  let body = compile_body(
    json!({ "shape": "any-of", "options": ["PENDING", "SHIPPED", "DELIVERED"], "example": "PENDING" }),
  );
  let (name, children) = action(&body);
  assert_eq!(name, "match:any-of");
  assert_eq!(children.len(), 4);
  assert_eq!(literal_value(&children[1]), &json!("PENDING"));
  assert_eq!(literal_value(&children[2]), &json!("SHIPPED"));
  assert_eq!(literal_value(&children[3]), &json!("DELIVERED"));
}

#[test]
fn any_of_pinned_narrows_to_equality() {
  let mut assignment = Assignment::new();
  assignment.insert("response.body#value".to_string(), "SHIPPED".to_string());
  let body = compile_body_under(
    json!({ "shape": "any-of", "options": ["PENDING", "SHIPPED", "DELIVERED"], "example": "PENDING" }),
    &assignment,
  );
  let (name, children) = action(&body);
  assert_eq!(name, "match:equality");
  assert_eq!(literal_value(&children[1]), &json!("SHIPPED"));
}

// --- one-of (plan-grammar spec §5.2, worked example order-payload-plan.md) ---

fn payment_body() -> Value {
  json!({ "shape": "one-of", "discriminator": "type", "default": "card",
    "alternatives": {
      "card": { "shape": "object", "members": {
        "type": { "shape": "equality", "example": "card" },
        "last4": { "shape": "regex", "pattern": "\\d{4}", "example": "1234" } } },
      "invoice": { "shape": "object", "members": {
        "type": { "shape": "equality", "example": "invoice" },
        "dueDate": { "shape": "date", "format": "yyyy-MM-dd", "example": "2026-08-30" } } } } })
}

#[test]
fn one_of_unpinned_is_a_nested_if_chain_ending_in_a_named_error() {
  let body = compile_body(payment_body());
  // The chain is built in reverse (last alternative innermost), so the outermost `if` is the
  // lexicographically-first alternative: "card".
  let (name, children) = action(&body);
  assert_eq!(name, "if");
  let (cond_name, cond_children) = action(&children[0]);
  assert_eq!(cond_name, "check:equals");
  assert_eq!(resolve_path(&cond_children[0]), "$.response.body.type");
  assert_eq!(literal_value(&cond_children[1]), &json!("card"));
  assert_eq!(label_of(&children[1]), Some("card"));

  let (next_name, next_children) = action(&children[2]);
  assert_eq!(next_name, "if");
  assert_eq!(label_of(&next_children[1]), Some("invoice"));

  let (error_name, error_children) = action(&next_children[2]);
  assert_eq!(error_name, "error");
  let (join_name, join_children) = action(&error_children[0]);
  assert_eq!(join_name, "join");
  assert_eq!(
    literal_value(&join_children[0]),
    &json!("Expected type to be one of card, invoice but got ")
  );
  assert_eq!(resolve_path(&join_children[1]), "$.response.body.type");
}

#[test]
fn one_of_pinned_narrows_to_the_selected_alternative_only() {
  let mut assignment = Assignment::new();
  assignment.insert("response.body#alternative".to_string(), "invoice".to_string());
  let body = compile_body_under(payment_body(), &assignment);
  assert_eq!(label_of(&body), Some("invoice"));
  let members = children_of(&body);
  assert_eq!(
    members.len(),
    2,
    "the pinned alternative's own object, discriminator included"
  );
}

// --- contains ---

#[test]
fn contains_compiles_to_an_opaque_match_action_with_one_container_per_entry() {
  let body = compile_body(json!({ "shape": "contains",
    "entries": [ { "shape": "equality", "example": "PENDING" } ] }));
  let (name, children) = action(&body);
  assert_eq!(name, "match:contains");
  assert_eq!(children.len(), 2, "resolve, then one container per entry");
  assert_eq!(resolve_path(&children[0]), "$.response.body");
}

// --- component operators (shape spec §3.5) ---

#[test]
fn a_component_operator_compiles_to_a_namespaced_action_passed_through_opaquely() {
  let body = compile_body(json!({ "shape": "protobuf:enum", "example": "SHIPPED", "extra": true }));
  let (name, children) = action(&body);
  assert_eq!(name, "protobuf:enum");
  assert_eq!(resolve_path(&children[0]), "$.response.body");
  let config = literal_value(&children[1]);
  assert_eq!(config["extra"], json!(true));
}

// --- documented literal kind mapping ---

#[test]
fn literals_carry_the_right_document_kind() {
  let body = compile_body(json!({ "shape": "equality", "example": null }));
  let (_, children) = action(&body);
  match &children[1].kind {
    NodeKind::Value(literal) => assert_eq!(literal.of, DocumentKind::Null),
    other => panic!("expected a value node, got {other:?}"),
  }
}
