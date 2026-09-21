//! Plan task 7.1: the subsumption checker, against the worked examples in
//! `Documentation/specs/subsumption-check/examples/` — the order payload carried from the shape
//! language's own example through to a report and its rendering, the two comparisons that example
//! does not exercise, and the policy example's excluded-combination caveat.
//!
//! The property test that checks the walk's verdicts against brute-force `admits` sampling — the
//! other half of what task 7.1 asks for — lives in `subsumption_properties.rs`.

use pact_janus_kernel::contract::{Contract, IdentifyMode, read as read_contract};
use pact_janus_kernel::shape::{ShapeNode, parse as parse_shape};
use pact_janus_kernel::subsumption::{
  Finding, ProviderShape, Severity, SubsumptionReport, Verdict, check, compare, read_provider_shape, render,
};
use pretty_assertions::assert_eq;
use serde_json::{Value, json};

// --- the shapes from shape-language examples/order-payload.md §2 and §7 -------------------------

fn consumer_shape() -> Value {
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

fn published_shape() -> Value {
  json!({ "shape": "object",
    "members": {
      "id": { "shape": "integer", "example": 42 },
      "status": { "shape": "any-of",
                  "options": ["PENDING", "SHIPPED", "DELIVERED", "CANCELLED"],
                  "example": "PENDING" },
      "shippedAt": { "shape": "optional",
                     "of": { "shape": "datetime", "format": "yyyy-MM-dd'T'HH:mm:ssX",
                             "example": "2026-07-30T10:00:00Z" } },
      "payment": { "shape": "one-of",
                   "discriminator": "type",
                   "alternatives": {
                     "card": { "shape": "object",
                               "members": {
                                 "type": { "shape": "equality", "example": "card" },
                                 "last4": { "shape": "regex", "pattern": "[0-9]{4}", "example": "1234" } } },
                     "invoice": { "shape": "object",
                                  "members": {
                                    "type": { "shape": "equality", "example": "invoice" },
                                    "dueDate": { "shape": "date", "format": "yyyy-MM-dd",
                                                 "example": "2026-08-30" } } } } },
      "items": { "shape": "each-like",
                 "min": 0,
                 "items": { "shape": "object",
                            "members": {
                              "sku": { "shape": "string", "example": "SKU-1" },
                              "qty": { "shape": "number", "example": 1 },
                              "backordered": { "shape": "boolean", "example": false } } } } } })
}

fn shape(value: &Value) -> ShapeNode {
  parse_shape(value, "/body").unwrap_or_else(|problems| panic!("ill-formed shape: {problems:?}"))
}

/// Compare two shape documents at one slot root, as the walk's top-level loop does (spec §3.1).
fn walk(provider: &Value, consumer: &Value) -> (Verdict, Vec<Finding>) {
  compare(&shape(provider), &shape(consumer), "response.body")
}

fn finding_at<'a>(findings: &'a [Finding], path: &str) -> &'a Finding {
  findings
    .iter()
    .find(|finding| finding.path == path)
    .unwrap_or_else(|| {
      panic!(
        "no finding at {path}; got {:?}",
        findings.iter().map(|f| &f.path).collect::<Vec<_>>()
      )
    })
}

/// Assert the members the spec's worked example fixes for a finding, leaving anything additive
/// (`excluded-by`, a future member) to the schema.
fn assert_finding(finding: &Finding, expected: &Value) {
  let actual = serde_json::to_value(finding).expect("a finding always serializes");
  for (member, value) in expected.as_object().expect("an object") {
    assert_eq!(
      actual.get(member),
      Some(value),
      "finding at {} differs on '{member}'",
      finding.path
    );
  }
}

// --- examples/order-payload-subsumption.md §2: the walk, node by node --------------------------

#[test]
fn the_order_payload_walk_reaches_the_examples_verdicts() {
  let (verdict, findings) = walk(&published_shape(), &consumer_shape());
  assert_eq!(verdict, Verdict::No);

  let paths: Vec<&str> = findings.iter().map(|f| f.path.as_str()).collect();
  assert_eq!(
    paths,
    vec![
      "response.body.items",
      "response.body.items[*].qty",
      "response.body.payment@card.last4",
      "response.body.status",
    ],
    "three decided findings and one honest review, and nothing else — \
     `items[*].backordered` is must-ignore and every identical node is the identity floor"
  );
}

#[test]
fn each_finding_says_what_the_example_says_it_says() {
  let (_, findings) = walk(&published_shape(), &consumer_shape());

  assert_finding(
    finding_at(&findings, "response.body.status"),
    &json!({ "verdict": "no", "severity": "finding", "kind": "wider-values",
      "provider": { "summary": "one of 'PENDING' | 'SHIPPED' | 'DELIVERED' | 'CANCELLED'" },
      "consumer": { "summary": "one of 'PENDING' | 'SHIPPED' | 'DELIVERED'" },
      "reason": "4 options vs 3; finite sets, exact" }),
  );
  assert_finding(
    finding_at(&findings, "response.body.items"),
    &json!({ "verdict": "no", "severity": "finding", "kind": "wider-cardinality",
      "provider": { "summary": "0 to unbounded elements" },
      "consumer": { "summary": "1 to unbounded elements" },
      "reason": "the empty array is admitted by the provider, not by the consumer" }),
  );
  assert_finding(
    finding_at(&findings, "response.body.items[*].qty"),
    &json!({ "verdict": "no", "severity": "finding", "kind": "broader-type",
      "provider": { "summary": "any number" },
      "consumer": { "summary": "a whole number" },
      "reason": "kind lattice: integer subset of number, exact" }),
  );
  assert_finding(
    finding_at(&findings, "response.body.payment@card.last4"),
    &json!({ "verdict": "unknown", "severity": "review", "kind": "unreviewable",
      "provider": { "summary": "strings matching '[0-9]{4}'" },
      "consumer": { "summary": "strings matching '\\d{4}'" },
      "reason": "two different regexes: conservative class, no guessing" }),
  );
}

#[test]
fn a_review_is_never_escalated_and_never_dropped() {
  // Spec §4.3: `unknown` is a first-class answer. The whole interaction's verdict is `no` here,
  // and the `unknown` still appears as its own `review`-severity finding rather than being
  // absorbed into it.
  let (_, findings) = walk(&published_shape(), &consumer_shape());
  let review = finding_at(&findings, "response.body.payment@card.last4");
  assert_eq!(review.severity, Severity::Review);
  assert_eq!(review.verdict, Verdict::Unknown);
}

#[test]
fn a_provider_shape_identical_to_the_consumers_is_the_identity_floor() {
  let (verdict, findings) = walk(&consumer_shape(), &consumer_shape());
  assert_eq!(verdict, Verdict::Yes);
  assert!(findings.is_empty(), "got {findings:?}");
}

// --- examples/order-payload-subsumption.md §5: two comparisons the payload does not exercise ---

#[test]
fn a_member_the_provider_does_not_name_is_the_widest_possible_claim() {
  let consumer = json!({ "shape": "object",
    "members": { "region": { "shape": "any-of", "options": ["AU", "NZ"], "example": "AU" } } });
  let provider = json!({ "shape": "object", "members": { } });
  let (verdict, findings) = walk(&provider, &consumer);
  assert_eq!(verdict, Verdict::No);
  assert_eq!(findings.len(), 1);
  assert_finding(
    &findings[0],
    &json!({ "path": "response.body.region", "verdict": "no", "severity": "finding",
      "kind": "undeclared-member",
      "provider": { "summary": "unconstrained: any value, or absent" },
      "consumer": { "summary": "one of 'AU' | 'NZ'" },
      "reason": "the provider's shape does not name this member; an unnamed member is the widest \
                 possible claim, not a narrow one (spec §3.2 Rule 2)" }),
  );
}

#[test]
fn the_only_consumer_shape_an_unnamed_member_is_inside_is_optional_any() {
  // Rule 2's own exception: "unless `C`'s shape at that name already admits everything `⊥`
  // included (an `optional` wrapping `any`, in practice never written)".
  let consumer = json!({ "shape": "object",
    "members": { "region": { "shape": "optional", "of": { "shape": "any" } } } });
  let provider = json!({ "shape": "object", "members": { } });
  let (verdict, findings) = walk(&provider, &consumer);
  assert_eq!(verdict, Verdict::Yes, "got {findings:?}");
}

#[test]
fn a_provider_that_may_omit_what_the_consumer_always_saw_is_weaker_presence() {
  let consumer = json!({ "shape": "object",
    "members": { "shippedAt": { "shape": "datetime", "format": "yyyy-MM-dd'T'HH:mm:ssX",
                                "example": "2026-07-30T10:00:00Z" } } });
  let provider = json!({ "shape": "object",
    "members": { "shippedAt": { "shape": "optional",
                                "of": { "shape": "datetime", "format": "yyyy-MM-dd'T'HH:mm:ssX",
                                        "example": "2026-07-30T10:00:00Z" } } } });
  let (verdict, findings) = walk(&provider, &consumer);
  assert_eq!(verdict, Verdict::No);
  assert_eq!(findings.len(), 1);
  assert_finding(
    &findings[0],
    &json!({ "path": "response.body.shippedAt", "verdict": "no", "severity": "finding",
      "kind": "weaker-presence",
      "provider": { "summary": "a datetime, or absent" },
      "consumer": { "summary": "a datetime, always present" },
      "reason": "the provider admits absence (optional); the consumer's shape does not" }),
  );
}

#[test]
fn the_nullable_column_is_decided_by_set_containment_not_a_presence_rule() {
  let consumer = json!({ "shape": "object",
    "members": { "note": { "shape": "string", "example": "hi" } } });
  let provider = json!({ "shape": "object",
    "members": { "note": { "shape": "nullable", "of": { "shape": "string", "example": "hi" } } } });
  let (verdict, findings) = walk(&provider, &consumer);
  assert_eq!(verdict, Verdict::No);
  assert_eq!(findings[0].kind, "weaker-presence");
  assert_eq!(
    findings[0].provider.as_ref().map(|side| side.summary.as_str()),
    Some("any string, or null")
  );
}

#[test]
fn a_member_the_consumer_forbids_and_the_provider_may_send_is_decided_no() {
  // Shape spec §8: "extra provider members are `yes` (must-ignore) unless the consumer marked the
  // member `forbidden`, in which case `admits(P) ⊄ admits(C)` is decided `no`".
  let consumer = json!({ "shape": "object", "members": { "ssn": { "shape": "forbidden" } } });
  let named = json!({ "shape": "object",
    "members": { "ssn": { "shape": "string", "example": "123-45-6789" } } });
  assert_eq!(walk(&named, &consumer).0, Verdict::No);

  // And the same when the provider's shape does not name it at all: not naming a member is the
  // widest claim (Rule 2), which includes sending one.
  let unnamed = json!({ "shape": "object", "members": { } });
  assert_eq!(walk(&unnamed, &consumer).0, Verdict::No);
}

// --- the degradation ladder: conservative and opaque classes -----------------------------------

#[test]
fn an_unknown_component_operator_degrades_only_where_the_shapes_differ() {
  let consumer = json!({ "shape": "object",
    "members": {
      "id": { "shape": "integer", "example": 42 },
      "csv": { "shape": "acme:csv", "columns": 3 } } });
  let same = json!({ "shape": "object",
    "members": {
      "id": { "shape": "integer", "example": 42 },
      "csv": { "shape": "acme:csv", "columns": 3 } } });
  assert_eq!(
    walk(&same, &consumer).0,
    Verdict::Yes,
    "the identity floor holds for an operator the kernel knows nothing about"
  );

  let differing = json!({ "shape": "object",
    "members": {
      "id": { "shape": "integer", "example": 42 },
      "csv": { "shape": "acme:csv", "columns": 4 } } });
  let (verdict, findings) = walk(&differing, &consumer);
  assert_eq!(verdict, Verdict::Unknown);
  assert_eq!(findings.len(), 1, "degraded locally, not globally: {findings:?}");
  assert_eq!(findings[0].path, "response.body.csv");
  assert_eq!(findings[0].kind, "unreviewable");
  assert_eq!(findings[0].severity, Severity::Review);
}

#[test]
fn contains_is_opaque_and_says_so() {
  let consumer = json!({ "shape": "contains",
    "entries": [ { "shape": "equality", "example": "a" } ] });
  let provider = json!({ "shape": "contains",
    "entries": [ { "shape": "equality", "example": "b" } ] });
  let (verdict, findings) = walk(&provider, &consumer);
  assert_eq!(verdict, Verdict::Unknown);
  assert_eq!(findings[0].kind, "unreviewable");
  assert!(
    findings[0]
      .reason
      .as_deref()
      .is_some_and(|r| r.contains("opaque class")),
    "got {:?}",
    findings[0].reason
  );
}

#[test]
fn a_conservative_operator_inside_its_exact_container_is_decided_yes() {
  // Shape spec §8: "`yes` where the container is exactly wider (`regex ⊆ string ⊆ any`)".
  let consumer = json!({ "shape": "string", "example": "x" });
  let provider = json!({ "shape": "regex", "pattern": "\\d{4}", "example": "1234" });
  assert_eq!(walk(&provider, &consumer).0, Verdict::Yes);

  // The other direction is not `no` — it is `unknown`, because deciding it would take a
  // containment procedure the conservative class deliberately does not promise.
  let (verdict, findings) = walk(&consumer, &provider);
  assert_eq!(verdict, Verdict::Unknown);
  assert_eq!(findings[0].kind, "unreviewable");
}

#[test]
fn a_kind_a_conservative_operator_can_never_produce_is_still_decided() {
  // Disjoint kinds decide `no` without needing to compare patterns at all.
  let consumer = json!({ "shape": "regex", "pattern": "\\d{4}", "example": "1234" });
  let provider = json!({ "shape": "integer", "example": 1234 });
  let (verdict, findings) = walk(&provider, &consumer);
  assert_eq!(verdict, Verdict::No);
  assert_eq!(findings[0].kind, "broader-type");
}

// --- discriminated unions (spec §3.3) ----------------------------------------------------------

#[test]
fn a_discriminator_value_the_consumer_names_no_alternative_for_is_decided_no() {
  let consumer = json!({ "shape": "one-of", "discriminator": "type",
    "alternatives": {
      "card": { "shape": "object",
                "members": { "type": { "shape": "equality", "example": "card" } } },
      "invoice": { "shape": "object",
                   "members": { "type": { "shape": "equality", "example": "invoice" } } } } });
  let provider = json!({ "shape": "one-of", "discriminator": "type",
    "alternatives": {
      "card": { "shape": "object",
                "members": { "type": { "shape": "equality", "example": "card" } } },
      "invoice": { "shape": "object",
                   "members": { "type": { "shape": "equality", "example": "invoice" } } },
      "cheque": { "shape": "object",
                  "members": { "type": { "shape": "equality", "example": "cheque" } } } } });
  let (verdict, findings) = walk(&provider, &consumer);
  assert_eq!(verdict, Verdict::No);
  assert_eq!(findings.len(), 1);
  assert_eq!(findings[0].path, "response.body@cheque");
  assert_eq!(findings[0].kind, "wider-values");
}

#[test]
fn a_consumer_union_wider_than_the_provider_exercises_is_not_a_finding() {
  // Spec §3.3: "`C` binding more discriminator values than `P` uses is not a finding: `P` simply
  // exercises a subset of the union, and subsetting a union is what narrower means."
  let consumer = json!({ "shape": "one-of", "discriminator": "type",
    "alternatives": {
      "card": { "shape": "object",
                "members": { "type": { "shape": "equality", "example": "card" } } },
      "invoice": { "shape": "object",
                   "members": { "type": { "shape": "equality", "example": "invoice" } } } } });
  let provider = json!({ "shape": "one-of", "discriminator": "type",
    "alternatives": {
      "card": { "shape": "object",
                "members": { "type": { "shape": "equality", "example": "card" } } },
      "invoice": { "shape": "object",
                   "members": { "type": { "shape": "equality", "example": "invoice" } } } } });
  assert_eq!(walk(&provider, &consumer).0, Verdict::Yes);
}

#[test]
fn alternatives_are_keyed_by_discriminator_literal_not_by_author_label() {
  // The label is the author's; the literal is the identity (spec §3.3's "keyed by distinct literal
  // values of `discriminator`").
  let consumer = json!({ "shape": "one-of", "discriminator": "type",
    "alternatives": {
      "byCard": { "shape": "object",
                  "members": { "type": { "shape": "equality", "example": "card" },
                               "last4": { "shape": "string", "example": "1234" } } },
      "byInvoice": { "shape": "object",
                     "members": { "type": { "shape": "equality", "example": "invoice" } } } } });
  let provider = json!({ "shape": "one-of", "discriminator": "type",
    "alternatives": {
      "card": { "shape": "object",
                "members": { "type": { "shape": "equality", "example": "card" },
                             "last4": { "shape": "string", "example": "1234" } } },
      "invoice": { "shape": "object",
                   "members": { "type": { "shape": "equality", "example": "invoice" } } } } });
  assert_eq!(walk(&provider, &consumer).0, Verdict::Yes);
}

// --- the report (spec §6) ----------------------------------------------------------------------

fn contract_document(interactions: Value) -> Value {
  json!({ "$format": "janus-contract/1",
    "consumer": { "name": "order-consumer" },
    "provider": { "name": "orders-api" },
    "interactions": interactions })
}

fn order_contract() -> Contract {
  let document = contract_document(json!([
    { "description": "get an order",
      "states": [ { "name": "an order exists" } ],
      "parts": { "response": { "body": consumer_shape() } },
      "selection": { "variants": [], "report": { } } }
  ]));
  read_contract(document.to_string().as_bytes(), IdentifyMode::Tolerant).expect("a well-formed contract")
}

fn order_provider_shape() -> ProviderShape {
  let document = json!({ "$format": "janus-provider-shape/1",
    "provider": { "name": "orders-api" },
    "provenance": "recorded",
    "interactions": [
      { "description": "get an order",
        "states": [ { "name": "an order exists" } ],
        "source": { "test-run": "orders-api#4821" },
        "parts": { "response": { "body": published_shape() } } } ] });
  read_provider_shape(document.to_string().as_bytes()).expect("a well-formed provider shape")
}

fn order_report() -> SubsumptionReport {
  check(&order_contract(), &order_provider_shape()).expect("both documents parse")
}

#[test]
fn the_report_is_the_examples_report() {
  let report = order_report();
  assert_eq!(report.format, "janus-subsumption-report/1");
  assert_eq!(report.consumer.name, "order-consumer");
  assert_eq!(report.provider.name, "orders-api");
  assert_eq!(report.interactions.len(), 1);

  let interaction = &report.interactions[0];
  assert_eq!(interaction.description, "get an order");
  assert!(interaction.matched);
  assert_eq!(interaction.verdict, "no");
  assert_eq!(interaction.findings.len(), 4);

  assert_eq!(report.summary.interactions, 1);
  assert_eq!(report.summary.matched, 1);
  assert_eq!(report.summary.findings, 3);
  assert_eq!(report.summary.reviews, 1);
}

#[test]
fn an_interaction_the_provider_published_nothing_for_is_not_published() {
  // Spec §6.3: a distinct report state, "that means *no check ran*, not *the check passed*".
  let contract = read_contract(
    contract_document(json!([
      { "description": "place an order",
        "parts": { "response": { "body": consumer_shape() } },
        "selection": { "variants": [], "report": { } } }
    ]))
    .to_string()
    .as_bytes(),
    IdentifyMode::Tolerant,
  )
  .expect("a well-formed contract");
  let report = check(&contract, &order_provider_shape()).expect("both documents parse");
  let interaction = &report.interactions[0];
  assert!(!interaction.matched);
  assert_eq!(interaction.verdict, "not-published");
  assert!(interaction.findings.is_empty());
  assert_eq!(report.summary.matched, 0);
}

#[test]
fn an_interaction_is_matched_by_description_and_state_names_together() {
  // Spec §2.2: the same identity contract spec §4.2 uses, minus the params a provider cannot know.
  let contract = read_contract(
    contract_document(json!([
      { "description": "get an order",
        "states": [ { "name": "an order exists", "params": { "id": 42 } } ],
        "parts": { "response": { "body": consumer_shape() } },
        "selection": { "variants": [], "report": { } } }
    ]))
    .to_string()
    .as_bytes(),
    IdentifyMode::Tolerant,
  )
  .expect("a well-formed contract");
  let report = check(&contract, &order_provider_shape()).expect("both documents parse");
  assert!(
    report.interactions[0].matched,
    "the consumer's state params are not part of the match"
  );
}

#[test]
fn a_slot_only_one_document_carries_contributes_nothing() {
  // Spec §2.1: "a slot present in one document and absent from the other contributes no finding —
  // the checker has nothing on one side to compare against".
  let contract = read_contract(
    contract_document(json!([
      { "description": "get an order",
        "states": [ { "name": "an order exists" } ],
        "parts": { "response": { "body": consumer_shape(),
                                 "status": { "shape": "equality", "example": 200 } } },
        "selection": { "variants": [], "report": { } } }
    ]))
    .to_string()
    .as_bytes(),
    IdentifyMode::Tolerant,
  )
  .expect("a well-formed contract");
  let report = check(&contract, &order_provider_shape()).expect("both documents parse");
  assert_eq!(
    report.interactions[0].findings.len(),
    4,
    "the four from the body, and nothing for `status`"
  );
}

#[test]
fn an_unreadable_provider_shape_names_the_position_of_the_fault() {
  let document = json!({ "$format": "janus-provider-shape/1",
    "provider": { "name": "orders-api" },
    "interactions": [
      { "description": "get an order",
        "states": [ { "name": "an order exists" } ],
        "parts": { "response": { "body": { "shape": "no-such-operator" } } } } ] });
  let published =
    read_provider_shape(document.to_string().as_bytes()).expect("the document itself is well-formed");
  let error = check(&order_contract(), &published).expect_err("the shape is not");
  assert_eq!(error.code(), "interaction-invalid");
}

#[test]
fn a_provider_shape_of_an_unimplemented_major_is_named_as_that() {
  let document = json!({ "$format": "janus-provider-shape/99",
    "provider": { "name": "orders-api" }, "interactions": [] });
  let error = read_provider_shape(document.to_string().as_bytes()).expect_err("a future major");
  assert_eq!(error.code(), "contract-version-unsupported");

  let not_one = json!({ "$format": "janus-contract/1" });
  let error = read_provider_shape(not_one.to_string().as_bytes()).expect_err("not a provider shape");
  assert_eq!(error.code(), "contract-invalid");
}

// --- the text rendering (spec §6.4) ------------------------------------------------------------

#[test]
fn the_rendering_is_the_examples_rendering() {
  let rendered = render(&order_report());
  assert_eq!(
    rendered,
    "\
✗ order-consumer is not compatible with orders-api
  interaction 'get an order', response body $.items:
    provider may produce an empty list
    consumer has only tested at least one item
  interaction 'get an order', response body $.items[*].qty:
    provider may produce any number
    consumer has only tested a whole number
  ? interaction 'get an order', response body $.payment.last4 (card):
    provider pattern '[0-9]{4}' cannot be compared against consumer pattern '\\d{4}' — review manually
  interaction 'get an order', response body $.status:
    provider may produce: 'PENDING' | 'SHIPPED' | 'DELIVERED' | 'CANCELLED'
    consumer has only tested: 'PENDING' | 'SHIPPED' | 'DELIVERED'"
  );
}

#[test]
fn a_report_with_nothing_to_say_says_that() {
  let contract = order_contract();
  let document = json!({ "$format": "janus-provider-shape/1",
    "provider": { "name": "orders-api" },
    "interactions": [
      { "description": "get an order",
        "states": [ { "name": "an order exists" } ],
        "parts": { "response": { "body": consumer_shape() } } } ] });
  let published = read_provider_shape(document.to_string().as_bytes()).expect("well-formed");
  let report = check(&contract, &published).expect("both documents parse");
  assert_eq!(report.interactions[0].verdict, "yes");
  assert_eq!(render(&report), "✓ order-consumer is compatible with orders-api");
}

// --- examples/policy-and-exemptions.md §3: an excluded combination (spec §5) --------------------

#[test]
fn a_passing_node_over_an_excluded_combination_is_reported_as_an_advisory() {
  let body = json!({ "shape": "object",
    "members": { "handlingFee": { "shape": "integer", "example": 500 } } });
  let contract = read_contract(
    contract_document(json!([
      { "description": "place an order",
        "parts": { "response": { "body": body } },
        "selection": { "variants": [],
          "report": { "exclusions": [
            { "when": [ { "dimension": "response.body.handlingFee#presence", "point": "present" },
                        { "dimension": "request.body.giftWrap#presence", "point": "present" } ],
              "reason": "expedited + gift-wrapped orders are not offered together by the checkout UI",
              "removed": 2 } ] } } }
    ]))
    .to_string()
    .as_bytes(),
    IdentifyMode::Tolerant,
  )
  .expect("a well-formed contract");
  let document = json!({ "$format": "janus-provider-shape/1",
    "provider": { "name": "orders-api" },
    "interactions": [
      { "description": "place an order",
        "parts": { "response": { "body": body } } } ] });
  let published = read_provider_shape(document.to_string().as_bytes()).expect("well-formed");
  let report = check(&contract, &published).expect("both documents parse");

  let interaction = &report.interactions[0];
  assert_eq!(
    interaction.verdict, "yes",
    "`excluded-by` never manufactures a fourth verdict value (spec §5)"
  );
  assert_eq!(interaction.findings.len(), 1);
  let advisory = &interaction.findings[0];
  assert_eq!(advisory.path, "response.body.handlingFee");
  assert_eq!(advisory.verdict, Verdict::Yes);
  assert_eq!(advisory.severity, Severity::Advisory);
  assert_eq!(advisory.kind, "excluded-combination");
  assert_eq!(
    advisory.excluded_by,
    vec![json!({
      "when": [ { "dimension": "response.body.handlingFee#presence", "point": "present" },
                { "dimension": "request.body.giftWrap#presence", "point": "present" } ],
      "reason": "expedited + gift-wrapped orders are not offered together by the checkout UI" })],
    "design 2.3's Exclusion document, reproduced and not otherwise interpreted"
  );
  assert_eq!(
    report.summary.findings, 0,
    "advisory is policy-inert by construction (spec §4.3)"
  );
  assert_eq!(report.summary.reviews, 0);
}

#[test]
fn a_decided_finding_over_an_excluded_combination_carries_the_caveat_and_keeps_its_verdict() {
  let consumer = json!({ "shape": "object",
    "members": { "status": { "shape": "any-of", "options": ["PENDING", "SHIPPED"],
                             "example": "PENDING" } } });
  let provider = json!({ "shape": "object",
    "members": { "status": { "shape": "any-of", "options": ["PENDING", "SHIPPED", "CANCELLED"],
                             "example": "PENDING" } } });
  let contract = read_contract(
    contract_document(json!([
      { "description": "get an order",
        "parts": { "response": { "body": consumer } },
        "selection": { "variants": [],
          "report": { "exclusions": [
            { "when": [ { "dimension": "response.body.status#value", "point": "SHIPPED" } ],
              "reason": "SHIPPED is exercised by the other interaction" } ] } } }
    ]))
    .to_string()
    .as_bytes(),
    IdentifyMode::Tolerant,
  )
  .expect("a well-formed contract");
  let document = json!({ "$format": "janus-provider-shape/1",
    "provider": { "name": "orders-api" },
    "interactions": [
      { "description": "get an order", "parts": { "response": { "body": provider } } } ] });
  let published = read_provider_shape(document.to_string().as_bytes()).expect("well-formed");
  let report = check(&contract, &published).expect("both documents parse");

  let finding = &report.interactions[0].findings[0];
  assert_eq!(
    finding.verdict,
    Verdict::No,
    "the caveat never changes what the walk decided"
  );
  assert_eq!(finding.severity, Severity::Finding);
  assert_eq!(finding.kind, "wider-values");
  assert_eq!(finding.excluded_by.len(), 1);
  assert_eq!(report.summary.findings, 1);
}

// --- provenance is policy input, never verdict input (spec §2.3) -------------------------------

#[test]
fn the_same_two_shapes_decide_the_same_whatever_the_provenance() {
  let verdicts: Vec<String> = ["recorded", "derived", "authored", "observed"]
    .iter()
    .map(|provenance| {
      let document = json!({ "$format": "janus-provider-shape/1",
        "provider": { "name": "orders-api" },
        "provenance": provenance,
        "interactions": [
          { "description": "get an order",
            "states": [ { "name": "an order exists" } ],
            "parts": { "response": { "body": published_shape() } } } ] });
      let published = read_provider_shape(document.to_string().as_bytes()).expect("well-formed");
      let report = check(&order_contract(), &published).expect("both documents parse");
      format!(
        "{}:{}",
        report.interactions[0].verdict,
        report.interactions[0].findings.len()
      )
    })
    .collect();
  assert_eq!(verdicts, vec!["no:4", "no:4", "no:4", "no:4"]);
}

#[test]
fn provenance_falls_back_document_then_specification_default() {
  let document = json!({ "$format": "janus-provider-shape/1",
    "provider": { "name": "orders-api" },
    "provenance": "recorded",
    "interactions": [
      { "description": "a", "parts": { } },
      { "description": "b", "provenance": "derived", "parts": { } } ] });
  let published = read_provider_shape(document.to_string().as_bytes()).expect("well-formed");
  assert_eq!(published.provenance_of(&published.interactions[0]), "recorded");
  assert_eq!(published.provenance_of(&published.interactions[1]), "derived");

  let bare = json!({ "$format": "janus-provider-shape/1",
    "provider": { "name": "orders-api" },
    "interactions": [ { "description": "a", "parts": { } } ] });
  let published = read_provider_shape(bare.to_string().as_bytes()).expect("well-formed");
  assert_eq!(published.provenance_of(&published.interactions[0]), "authored");
}
