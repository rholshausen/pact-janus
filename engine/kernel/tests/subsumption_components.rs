//! ADR 0087 (staged plan S4.7): the subsumption walk decides the built-in HTTP component's operators,
//! each against a literal, itself and its neighbours. A finite provider is decided by the consumer's
//! own matcher (shape spec §8); anything wider is the component's `matcher/compare`, for an operator
//! it declared `exact` or `conservative`. An operator no component implements stays opaque.

use pact_janus_component_http::status::{self, HttpMatcher};
use pact_janus_kernel::component::InTreeMatcher;
use pact_janus_kernel::protocol::in_tree_actions;
use pact_janus_kernel::shape::parse;
use pact_janus_kernel::subsumption::{Verdict, compare_with};
use rstest::rstest;
use serde_json::{Value, json};
use std::sync::Arc;

fn verdict(provider: Value, consumer: Value) -> Verdict {
  let components = in_tree_actions(&[InTreeMatcher {
    name: status::NAME.to_string(),
    actions: status::ACTIONS.iter().map(|action| action.to_string()).collect(),
    matcher: Arc::new(HttpMatcher::new()),
  }]);
  let p = parse(&provider, "/provider").expect("a provider shape");
  let c = parse(&consumer, "/consumer").expect("a consumer shape");
  compare_with(&p, &c, "response.x", &components).0
}

fn literal(value: Value) -> Value {
  json!({ "shape": "equality", "example": value })
}

fn class(class: &str) -> Value {
  json!({ "shape": "http:status-class", "class": class, "example": 200 })
}

fn media_type(media_type: &str) -> Value {
  json!({ "shape": "http:media-type", "type": media_type, "example": media_type })
}

fn list(elements: &[&str]) -> Value {
  json!({ "shape": "http:list", "elements": elements, "example": [elements.join(", ")] })
}

fn path(template: &str) -> Value {
  json!({ "shape": "http:path", "template": template, "example": template })
}

#[rstest]
// http:status-class: exact.
#[case::a_status_in_the_class(literal(json!(200)), class("success"), Verdict::Yes)]
#[case::a_status_outside_it(literal(json!(404)), class("success"), Verdict::No)]
#[case::statuses_all_in_it(json!({ "shape": "any-of", "options": [200, 204], "example": 200 }), class("success"), Verdict::Yes)]
#[case::the_class_itself(class("success"), class("success"), Verdict::Yes)]
#[case::a_narrower_class(class("success"), class("non-error"), Verdict::Yes)]
#[case::a_wider_class(class("non-error"), class("success"), Verdict::No)]
#[case::the_class_against_one_status(class("success"), literal(json!(200)), Verdict::No)]
#[case::the_class_against_some(class("success"), json!({ "shape": "any-of", "options": [200, 201], "example": 200 }), Verdict::No)]
#[case::any_integer(json!({ "shape": "integer", "example": 200 }), class("success"), Verdict::No)]
#[case::the_class_against_any(class("success"), json!({ "shape": "any" }), Verdict::Yes)]
// http:media-type: exact.
#[case::the_type(literal(json!("application/json")), media_type("application/json"), Verdict::Yes)]
#[case::the_type_with_a_parameter(literal(json!("application/json; charset=utf-8")), media_type("application/json"), Verdict::Yes)]
#[case::another_type(literal(json!("text/plain")), media_type("application/json"), Verdict::No)]
#[case::the_media_type_itself(media_type("application/json"), media_type("application/json"), Verdict::Yes)]
#[case::one_naming_more(
  media_type("application/json; charset=utf-8"),
  media_type("application/json"),
  Verdict::Yes
)]
#[case::one_naming_less(
  media_type("application/json"),
  media_type("application/json; charset=utf-8"),
  Verdict::No
)]
#[case::the_media_type_against_its_text(media_type("application/json"), literal(json!("application/json")), Verdict::No)]
#[case::the_media_type_against_a_string(media_type("application/json"), json!({ "shape": "string", "example": "x" }), Verdict::Yes)]
#[case::a_string_against_the_media_type(json!({ "shape": "string", "example": "x" }), media_type("application/json"), Verdict::No)]
// http:list: exact.
#[case::the_list_as_one_line(literal(json!(["a, b"])), list(&["a", "b"]), Verdict::Yes)]
#[case::the_list_as_two_lines(literal(json!(["a", "b"])), list(&["a", "b"]), Verdict::Yes)]
#[case::another_order(literal(json!(["b, a"])), list(&["a", "b"]), Verdict::No)]
#[case::the_list_itself(list(&["a", "b"]), list(&["a", "b"]), Verdict::Yes)]
#[case::another_list(list(&["a", "b"]), list(&["a"]), Verdict::No)]
#[case::the_list_against_one_framing(list(&["a", "b"]), literal(json!(["a, b"])), Verdict::No)]
// http:path: conservative.
#[case::a_path_it_matches(literal(json!("/orders/66")), path("/orders/{id}"), Verdict::Yes)]
#[case::a_path_it_does_not(literal(json!("/orders")), path("/orders/{id}"), Verdict::No)]
#[case::the_template_itself(path("/orders/{id}"), path("/orders/{id}"), Verdict::Yes)]
#[case::a_template_named_otherwise(path("/orders/{id}"), path("/orders/{orderId}"), Verdict::Unknown)]
#[case::a_template_with_no_parameter(path("/orders"), literal(json!("/orders")), Verdict::Yes)]
#[case::a_template_against_one_path(path("/orders/{id}"), literal(json!("/orders/1")), Verdict::No)]
#[case::the_template_against_a_string(path("/orders/{id}"), json!({ "shape": "string", "example": "x" }), Verdict::Yes)]
#[case::a_string_against_the_template(json!({ "shape": "string", "example": "x" }), path("/orders/{id}"), Verdict::No)]
#[case::the_template_against_a_regex(path("/orders/{id}"), json!({ "shape": "regex", "pattern": "^/orders/.+$", "example": "/orders/1" }), Verdict::Unknown)]
// An operator no component implements stays opaque: `yes` on identity, `unknown` otherwise.
#[case::an_operator_nobody_runs(literal(json!(["a"])), json!({ "shape": "pact:ignore-order", "example": ["a"] }), Verdict::Unknown)]
fn the_walk_decides_the_http_components_operators(
  #[case] provider: Value,
  #[case] consumer: Value,
  #[case] expected: Verdict,
) {
  assert_eq!(verdict(provider, consumer), expected);
}

#[test]
fn without_the_component_its_operators_stay_opaque() {
  // A walk nothing answers for (the public `compare`) cannot decide what the HTTP component knows.
  let p = parse(&literal(json!("application/json")), "/p").unwrap();
  let c = parse(&media_type("application/json"), "/c").unwrap();
  assert_eq!(
    pact_janus_kernel::subsumption::compare(&p, &c, "x").0,
    Verdict::Unknown
  );
}

#[test]
fn a_decided_no_says_why() {
  let components = in_tree_actions(&[InTreeMatcher {
    name: status::NAME.to_string(),
    actions: status::ACTIONS.iter().map(|action| action.to_string()).collect(),
    matcher: Arc::new(HttpMatcher::new()),
  }]);
  let p = parse(&class("non-error"), "/p").unwrap();
  let c = parse(&class("success"), "/c").unwrap();
  let (verdict, findings) = compare_with(&p, &c, "response.status", &components);
  assert_eq!(verdict, Verdict::No);
  assert_eq!(findings[0].kind, "wider-values");
  assert_eq!(
    findings[0].reason.as_deref(),
    Some("the provider may produce 100, 101, 102, …")
  );
}
