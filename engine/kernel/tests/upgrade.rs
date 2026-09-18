//! Plan task 5.5, contract-file spec §8: **`upgrade/pact`** — a v1–v4 pact converted into a Janus
//! contract, and honest about everything it could not carry across.
//!
//! The conversion is not required to be lossless; it is required to say so. So most of what is
//! asserted here is the *findings* list, not the contract: an empty one is a claim, and a wrong
//! one is worse than a conversion that refused. The claim that the converted contract still
//! verifies the same provider the pact verifies lives in `legacy_verification.rs`, where the
//! provider is.

use pact_janus_kernel::upgrade::{self, Finding};
use serde_json::{Value, json};

fn fixture(name: &str) -> Value {
  let path = format!(
    "{}/tests/fixtures/legacy-pacts/{name}",
    env!("CARGO_MANIFEST_DIR")
  );
  let text = std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("reading {path}: {err}"));
  serde_json::from_str(&text).unwrap_or_else(|err| panic!("parsing {path}: {err}"))
}

fn upgrade(pact: &Value) -> upgrade::Upgraded {
  upgrade::pact("test", pact).expect("a readable pact converts")
}

/// The shape document at one slot of one interaction.
fn shape<'a>(upgraded: &'a upgrade::Upgraded, index: usize, part: &str, slot: &str) -> &'a Value {
  upgraded.contract.interactions[index].parts[part]
    .get(slot)
    .unwrap_or_else(|| panic!("no {part}/{slot} slot"))
}

fn codes(findings: &[Finding]) -> Vec<&str> {
  findings.iter().map(|f| f.code.as_str()).collect()
}

fn find<'a>(findings: &'a [Finding], code: &str) -> &'a Finding {
  findings
    .iter()
    .find(|f| f.code == code)
    .unwrap_or_else(|| panic!("no '{code}' finding in {:?}", codes(findings)))
}

/// A v3 pact with one interaction, built around whatever request/response the caller wants.
fn pact_with(request: Value, response: Value) -> Value {
  json!({
    "consumer": { "name": "c" },
    "provider": { "name": "p" },
    "interactions": [{ "description": "an interaction", "request": request, "response": response }],
    "metadata": { "pactSpecification": { "version": "3.0.0" } }
  })
}

fn get_slash() -> Value {
  json!({ "method": "GET", "path": "/" })
}

// ---------------------------------------------------------------------------------------------
// §8.2 — matching rules become shapes
// ---------------------------------------------------------------------------------------------

#[test]
fn each_matcher_becomes_the_operator_the_table_names() {
  let response = json!({
    "status": 200,
    "body": {
      "plain": "x", "typed": "x", "pattern": "abc", "part": "abcdef",
      "count": 1, "whole": 1, "fraction": 1.5, "flag": true, "nothing": null,
      "filled": "x", "version": "1.2.3", "when": "2026-07-30T09:00:00Z"
    },
    "matchingRules": { "body": {
      "$.typed": { "matchers": [{ "match": "type" }] },
      "$.pattern": { "matchers": [{ "match": "regex", "regex": "a.c" }] },
      "$.part": { "matchers": [{ "match": "include", "value": "cde" }] },
      "$.count": { "matchers": [{ "match": "number" }] },
      "$.whole": { "matchers": [{ "match": "integer" }] },
      "$.fraction": { "matchers": [{ "match": "decimal" }] },
      "$.flag": { "matchers": [{ "match": "boolean" }] },
      "$.nothing": { "matchers": [{ "match": "null" }] },
      "$.filled": { "matchers": [{ "match": "notEmpty" }] },
      "$.version": { "matchers": [{ "match": "semver" }] },
      "$.when": { "matchers": [{ "match": "timestamp", "timestamp": "yyyy-MM-dd'T'HH:mm:ssXXX" }] }
    } }
  });
  let upgraded = upgrade(&pact_with(get_slash(), response));
  let members = &shape(&upgraded, 0, "response", "body")["members"];

  // No rule at a position is not a gap: it is what v1–v4 already meant by the example.
  assert_eq!(members["plain"], json!({ "shape": "equality", "example": "x" }));
  assert_eq!(members["typed"], json!({ "shape": "type", "example": "x" }));
  assert_eq!(
    members["pattern"],
    json!({ "shape": "regex", "pattern": "a.c", "example": "abc" })
  );
  assert_eq!(
    members["part"],
    json!({ "shape": "include", "substring": "cde", "example": "abcdef" })
  );
  assert_eq!(members["count"]["shape"], json!("number"));
  assert_eq!(members["whole"]["shape"], json!("integer"));
  assert_eq!(members["fraction"]["shape"], json!("decimal"));
  assert_eq!(members["flag"]["shape"], json!("boolean"));
  assert_eq!(members["nothing"]["shape"], json!("null"));
  assert_eq!(members["filled"]["shape"], json!("not-empty"));
  assert_eq!(members["version"]["shape"], json!("semver"));
  // Format strings carry across character for character (spec §8.2).
  assert_eq!(
    members["when"],
    json!({ "shape": "datetime", "format": "yyyy-MM-dd'T'HH:mm:ssXXX",
            "example": "2026-07-30T09:00:00Z" })
  );
}

/// `min`/`max`/`minmax` on an array are the one row that changes the *structure*: they become
/// `each-like`'s own cardinality, and a bare `type` on an array becomes the same operator with the
/// default `eachLike` has always had.
#[test]
fn a_min_matcher_on_an_array_becomes_each_likes_cardinality() {
  let response = json!({
    "status": 200,
    "body": { "items": [{ "sku": "a" }] },
    "matchingRules": { "body": { "$.items": { "matchers": [{ "match": "type", "min": 2 }] } } }
  });
  let upgraded = upgrade(&pact_with(get_slash(), response));
  let items = &shape(&upgraded, 0, "response", "body")["members"]["items"];
  assert_eq!(items["shape"], json!("each-like"));
  assert_eq!(items["min"], json!(2));
  // The template is the first element, walked with the *same cascading* the plan compiler uses:
  // the rule declared on `$.items` reaches `$.items[0].sku` too, because nothing more specific was
  // declared closer to it — so the member is `type`, not a frozen `equality`.
  assert_eq!(
    items["items"],
    json!({ "shape": "object", "members": { "sku": { "shape": "type", "example": "a" } } })
  );
}

/// An array with no rule asserted an exact list, and `array` is the operator that says so.
#[test]
fn an_array_with_no_rule_becomes_a_fixed_array() {
  let response = json!({ "status": 200, "body": { "items": ["a", "b"] } });
  let upgraded = upgrade(&pact_with(get_slash(), response));
  let items = &shape(&upgraded, 0, "response", "body")["members"]["items"];
  assert_eq!(items["shape"], json!("array"));
  assert_eq!(items["entries"].as_array().expect("entries").len(), 2);
}

/// Headers are `{name: [value, …]}` on the wire, so they convert to an `object` of `array`s — not
/// a flat string map, which would not match anything a transport produces.
#[test]
fn headers_convert_to_the_document_form_a_transport_actually_produces() {
  let response = json!({
    "status": 200,
    "headers": { "Content-Type": "application/json", "X-Trace": "abc" }
  });
  let upgraded = upgrade(&pact_with(get_slash(), response));
  let headers = shape(&upgraded, 0, "response", "headers");
  assert_eq!(headers["shape"], json!("object"));
  assert_eq!(
    headers["members"]["content-type"],
    json!({ "shape": "array", "entries": [{ "shape": "equality", "example": "application/json" }] }),
    "names are lower-cased and values are a list, exactly as the HTTP component reads them back"
  );
  assert!(headers["members"].get("x-trace").is_some());
}

// ---------------------------------------------------------------------------------------------
// §8.3 — the single example becomes the sole variant
// ---------------------------------------------------------------------------------------------

#[test]
fn the_single_example_becomes_the_sole_base_variant() {
  let upgraded = upgrade(&fixture("V3Consumer-ProviderStateService.json"));
  let selection = &upgraded.contract.interactions[0].selection;
  assert_eq!(selection.variants.len(), 1);
  assert_eq!(selection.variants[0].id, "base");
  assert!(
    selection.variants[0].assignment.is_empty(),
    "the base variant pins nothing"
  );
  assert_eq!(selection.report["strategy"], json!("base-only"));
  assert_eq!(selection.report["selected"], json!(1));
  assert_eq!(selection.report["space"]["size"], json!(1));
  assert_eq!(selection.report["space"]["dimensions"], json!(0));

  // Both examples ride with it — the pact's own request and response, byte for byte.
  let parts = &selection.variants[0].parts;
  assert_eq!(parts["request"]["method"].content, json!("POST"));
  assert_eq!(parts["response"]["status"].content, json!(200));
}

/// §8.3's own refinement: a converted `min` contributes a cardinality dimension, so the space is
/// bigger than one while the pact still demonstrates a single example. The invariant that holds
/// for *every* conversion is `selected: 1`, and the report says by how much the contract is
/// under-covered rather than hiding it.
#[test]
fn a_converted_cardinality_makes_the_space_larger_than_the_selection() {
  let response = json!({
    "status": 200,
    "body": { "items": [{ "sku": "a" }] },
    "matchingRules": { "body": { "$.items": { "matchers": [{ "match": "type", "min": 1 }] } } }
  });
  let upgraded = upgrade(&pact_with(get_slash(), response));
  let report = &upgraded.contract.interactions[0].selection.report;
  assert!(
    report["space"]["size"].as_u64().expect("a size") > 1,
    "each-like contributes a cardinality dimension: {report:?}"
  );
  assert_eq!(report["selected"], json!(1), "exactly one variant, always");
}

// ---------------------------------------------------------------------------------------------
// §8.4 — findings
// ---------------------------------------------------------------------------------------------

/// An empty `findings` list is a claim, and a strong one. This is the pact that earns it: a rule
/// at every position, no headers, no generators, no request body.
#[test]
fn an_empty_findings_list_is_a_claim_the_converter_can_actually_make() {
  let pact = pact_with(
    json!({
      "method": "GET", "path": "/orders/66",
      "matchingRules": {
        "method": { "$": { "matchers": [{ "match": "equality" }] } },
        "path": { "$": { "matchers": [{ "match": "regex", "regex": "/orders/\\d+" }] } }
      }
    }),
    json!({
      "status": 200,
      "headers": { "Content-Type": "application/json" },
      "body": { "id": "66" },
      "matchingRules": {
        "status": { "$": { "matchers": [{ "match": "equality" }] } },
        "header": { "Content-Type": { "matchers": [{ "match": "regex", "regex": "application/json.*" }] } },
        "body": { "$.id": { "matchers": [{ "match": "type" }] } }
      }
    }),
  );
  let upgraded = upgrade(&pact);
  assert!(
    upgraded.findings.is_empty(),
    "this conversion was exact, and said so: {:?}",
    upgraded.findings
  );
}

/// The shape language has no intersection operator, so an `AND` of two unrelated rules cannot be
/// carried. The narrower does not survive by accident — the first does, and the finding says the
/// others were dropped.
#[test]
fn an_and_of_unrelated_rules_keeps_the_first_and_reports_it_lossy() {
  let response = json!({
    "status": 200, "body": { "id": "abc" },
    "matchingRules": { "body": { "$.id": { "combine": "AND", "matchers": [
      { "match": "regex", "regex": "a.c" }, { "match": "include", "value": "b" }
    ] } } }
  });
  let upgraded = upgrade(&pact_with(get_slash(), response));
  let finding = find(&upgraded.findings, "rule-unmapped");
  assert_eq!(finding.kind, "lossy");
  assert_eq!(finding.path, "/interactions/0/response/body/id");
  assert_eq!(
    finding.target.as_deref(),
    Some("/interactions/0/parts/response/body/members/id"),
    "a finding names the position in both documents"
  );
  assert_eq!(
    shape(&upgraded, 0, "response", "body")["members"]["id"]["shape"],
    json!("regex"),
    "the first rule survived"
  );
}

#[test]
fn an_or_combination_is_reported_as_its_own_kind_of_loss() {
  let response = json!({
    "status": 200, "body": { "id": "abc" },
    "matchingRules": { "body": { "$.id": { "combine": "OR", "matchers": [
      { "match": "regex", "regex": "a.c" }, { "match": "null" }
    ] } } }
  });
  let upgraded = upgrade(&pact_with(get_slash(), response));
  assert_eq!(find(&upgraded.findings, "rule-combination-or").kind, "lossy");
}

/// A dropped generator is lossy but never dangerous: it weakens what the contract can *produce*,
/// never what it *accepts*, and the message says so.
#[test]
fn generators_are_dropped_and_counted() {
  let upgraded = upgrade(&fixture("V3Consumer-ProviderStateService.json"));
  let finding = find(&upgraded.findings, "generator-dropped");
  assert_eq!(finding.kind, "lossy");
  assert!(
    finding.message.contains("accepts is unchanged"),
    "{}",
    finding.message
  );
}

/// v1–v4 request bodies reject members the pact did not name; ADR 0007 refuses closed objects
/// outright. The contract therefore admits more than the pact did, at exactly one position.
#[test]
fn an_opened_request_body_is_reported_because_the_contract_now_admits_more() {
  let upgraded = upgrade(&pact_with(
    json!({ "method": "POST", "path": "/", "body": { "id": 1 } }),
    json!({ "status": 200 }),
  ));
  assert_eq!(find(&upgraded.findings, "request-body-opened").kind, "lossy");
}

/// Header values with no rule are where this conversion is least exact: v1–v4 compared them with
/// their own defaulted comparison, and `equality` is strictly narrower.
#[test]
fn narrowing_a_header_comparison_to_equality_is_a_judgement_not_a_silent_tightening() {
  let upgraded = upgrade(&pact_with(
    get_slash(),
    json!({ "status": 200, "headers": { "Content-Type": "application/json; charset=UTF-8" } }),
  ));
  let finding = find(&upgraded.findings, "rule-narrowed");
  assert_eq!(finding.kind, "judgement");
  assert!(finding.message.contains("MIME parameters"), "{}", finding.message);
}

/// The one body the engine cannot read: carried as bytes on the variant, asserted about nowhere,
/// and reported. A shape invented for it would claim something the conversion does not know.
#[test]
fn a_body_no_content_component_can_read_is_carried_as_bytes_and_asserted_about_nowhere() {
  let upgraded = upgrade(&fixture("XMLConsumer-XMLProvider.json"));
  let finding = find(&upgraded.findings, "body-not-parsed");
  assert_eq!(finding.kind, "lossy");
  let interaction = &upgraded.contract.interactions[0];
  assert!(
    !interaction.parts["response"].contains_key("body"),
    "no shape was invented for a body nothing could read"
  );
  assert!(
    interaction.selection.variants[0].parts["response"].contains_key("body"),
    "the example itself still rides with the variant"
  );
}

/// A message interaction has no contract form yet. Vanishing silently is exactly the outcome the
/// honesty rule forbids, so the count is reported.
#[test]
fn interactions_that_cannot_be_converted_are_counted_rather_than_vanishing() {
  let upgraded = upgrade(&fixture("test_consumer_v3-MessageProvider.json"));
  assert!(upgraded.contract.interactions.is_empty());
  let finding = find(&upgraded.findings, "interaction-dropped");
  assert_eq!(finding.kind, "lossy");
  assert!(finding.message.contains("3 of 3"), "{}", finding.message);
}

/// Contract-file spec §4.2 identifies an interaction by description *and* states, so a pact with
/// two identical ones would produce a contract that cannot be read back. Disambiguated, not
/// dropped — and reported either way.
#[test]
fn a_colliding_interaction_identity_is_disambiguated_and_reported() {
  let mut pact = pact_with(get_slash(), json!({ "status": 200 }));
  let first = pact["interactions"][0].clone();
  pact["interactions"].as_array_mut().unwrap().push(first);
  let upgraded = upgrade(&pact);
  assert_eq!(find(&upgraded.findings, "duplicate-description").kind, "lossy");
  assert_eq!(upgraded.contract.interactions[0].description, "an interaction");
  assert_eq!(
    upgraded.contract.interactions[1].description,
    "an interaction (2)"
  );
}

/// v3 state parameters travel as they stand; v1/v2's bare string arrives as a state with none.
#[test]
fn provider_states_convert_with_their_parameters_and_a_note_about_their_types() {
  let upgraded = upgrade(&fixture("V3Consumer-ProviderStateService.json"));
  let states = upgraded.contract.interactions[0].states.as_ref().expect("states");
  assert_eq!(states[0].name, "a provider state with injectable values");
  assert_eq!(states[0].params.as_ref().expect("params")["valueB"], json!(100));
  assert_eq!(find(&upgraded.findings, "state-params-untyped").kind, "note");

  let zoo = upgrade(&fixture("zoo_app-animal_service.json"));
  let states = zoo.contract.interactions[0].states.as_ref().expect("states");
  assert_eq!(states[0].name, "there are alligators");
  assert!(
    states[0].params.is_none(),
    "a v1 state has no parameters to carry"
  );
}

/// Every converted contract must read back as one — the conversion parses its own output as an
/// interaction specification on the way through, which is also where the variant space comes from.
#[test]
fn every_converted_contract_reads_back_as_a_contract() {
  for name in [
    "V3Consumer-ProviderStateService.json",
    "consumer-provider-v4.json",
    "zoo_app-animal_service.json",
    "eachkeylike__consumer-eachkeylike_provider.json",
  ] {
    let upgraded = upgrade(&fixture(name));
    let bytes = pact_janus_kernel::contract::write_canonical(&upgraded.contract)
      .unwrap_or_else(|err| panic!("{name} writes: {err:?}"));
    pact_janus_kernel::contract::read(&bytes, pact_janus_kernel::contract::IdentifyMode::Strict)
      .unwrap_or_else(|err| panic!("{name} reads back: {err:?}"));
  }
}
