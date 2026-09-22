//! Plan task 7.4: the compatibility decision — design 2.8 §7's policy, and the `can-i-deploy`
//! page over it.
//!
//! Driven through the protocol (`subsumption/check`, `subsumption/decide` — engine-protocol spec
//! §8.6) rather than through the kernel's Rust API, because that is how every host reaches this:
//! the CLI, an SDK, and whatever a broker would eventually be. A decision that could only be
//! reached from Rust would not be the decision task 7.4 is supposed to deliver.
//!
//! Three groups. The policy's own arithmetic — layering, exemption scoping, expiry — comes first,
//! because it is the part with rules a reader can get wrong. Then the decision: which facts block,
//! which warn, and the two that no policy governs. Then the page, against the RFC's own sketch.

use pact_janus_kernel::protocol::Engine;
use pact_janus_kernel::subsumption::{Action, SubsumptionPolicy};
use pretty_assertions::assert_eq;
use serde_json::{Value, json};

// --- the harness --------------------------------------------------------------------------------

fn engine() -> Engine {
  let mut engine = Engine::new();
  let hello = send(&mut engine, "engine/hello", json!({ "protocol-versions": [1] }));
  assert_eq!(
    hello["ok"]["capabilities"]["subsumption-check"],
    json!({}),
    "the operations these tests use are behind a capability, and a host may not rely on one the \
     engine did not declare (spec §5.3)"
  );
  engine
}

fn send(engine: &mut Engine, op: &str, body: Value) -> Value {
  let request = json!({ "type": "request", "id": "r-1", "op": op, "body": body });
  let bytes = engine.dispatch(&serde_json::to_vec(&request).expect("a json! literal serializes"));
  serde_json::from_slice(&bytes).expect("Engine::dispatch always returns valid JSON")
}

fn ok(engine: &mut Engine, op: &str, body: Value) -> Value {
  let response = send(engine, op, body);
  assert!(response.get("ok").is_some(), "{op} failed: {}", response["error"]);
  response["ok"].clone()
}

/// A validator for one `$def` of the protocol's v1 schema set, with every schema in the set
/// registered so a cross-file `$ref` resolves (`subsumption.schema.json` reuses
/// `consumer-session.schema.json`'s `Party`, as `frame.schema.json` reuses `engine-error`'s).
fn protocol_validator(reference: &str) -> jsonschema::Validator {
  struct InMemory(std::collections::HashMap<String, Value>);
  impl jsonschema::Retrieve for InMemory {
    fn retrieve(
      &self,
      uri: &jsonschema::Uri<String>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
      self
        .0
        .get(uri.as_str())
        .cloned()
        .ok_or_else(|| format!("schema not registered: {uri}").into())
    }
  }

  const BASE: &str = "https://pact.io/janus/protocol/v1/";
  let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    .join("../../Documentation/specs/engine-protocol/schemas/v1");
  let mut schemas = std::collections::HashMap::new();
  for entry in std::fs::read_dir(&dir).unwrap_or_else(|err| panic!("reading {dir:?}: {err}")) {
    let path = entry.expect("a directory entry").path();
    if path.to_string_lossy().ends_with(".schema.json") {
      let document: Value =
        serde_json::from_slice(&std::fs::read(&path).expect("read")).expect("valid schema JSON");
      let id = document["$id"]
        .as_str()
        .expect("every schema has an $id")
        .to_string();
      schemas.insert(id, document);
    }
  }
  jsonschema::options()
    .with_retriever(InMemory(schemas))
    .build(&json!({ "$ref": format!("{BASE}{reference}") }))
    .unwrap_or_else(|err| panic!("compiling {reference}: {err}"))
}

/// The RFC's own scenario: a consumer that tested two statuses, a provider whose shape admits
/// three, and one optional field the consumer only ever saw present.
fn consumer_contract() -> Value {
  json!({ "$format": "janus-contract/1",
    "consumer": { "name": "order-consumer" },
    "provider": { "name": "order-service" },
    "interactions": [
      { "description": "get an order",
        "states": [ { "name": "an order exists" } ],
        "parts": { "response": { "body": {
          "shape": "object",
          "members": {
            "status": { "shape": "any-of", "options": ["PENDING", "SHIPPED"], "example": "PENDING" },
            "shippedAt": { "shape": "string", "example": "2026-07-30T10:00:00Z" } } } } },
        "selection": { "variants": [], "report": {} } } ] })
}

fn provider_shape() -> Value {
  json!({ "$format": "janus-provider-shape/1",
    "provider": { "name": "order-service" },
    "provenance": "recorded",
    "interactions": [
      { "description": "get an order",
        "states": [ { "name": "an order exists" } ],
        "parts": { "response": { "body": {
          "shape": "object",
          "members": {
            "status": { "shape": "any-of",
                        "options": ["PENDING", "SHIPPED", "DELIVERED"],
                        "example": "PENDING" },
            "shippedAt": { "shape": "optional",
                           "of": { "shape": "string", "example": "2026-07-30T10:00:00Z" } } } } } } } ] })
}

/// One pair, checked, ready for `decide`.
fn checked_pair(engine: &mut Engine) -> Value {
  let result = ok(
    engine,
    "subsumption/check",
    json!({ "contract": consumer_contract(), "provider-shape": provider_shape() }),
  );
  json!({
    "consumer": { "name": "order-consumer" },
    "provider": { "name": "order-service" },
    "format": result["format"],
    "subsumption": result["report"],
  })
}

/// A verification summary shaped exactly as a run's terminal event carries one (spec §9.6).
fn verified_summary() -> Value {
  json!({ "status": "verified", "contracts": 1,
          "consumers": ["order-consumer"], "providers": ["order-service"],
          "interactions": 1, "filtered": false,
          "variants": { "total": 4, "verified": 4, "failed": 0, "state-unavailable": 0, "skipped": 0 },
          "failures": [] })
}

fn decide(engine: &mut Engine, body: Value) -> (Value, String) {
  let result = ok(engine, "subsumption/decide", body);
  let text = result["text"].as_str().unwrap_or_default().to_string();
  (result["report"].clone(), text)
}

fn reason_codes(pair: &Value) -> Vec<String> {
  pair["reasons"]
    .as_array()
    .expect("reasons")
    .iter()
    .map(|reason| reason["code"].as_str().unwrap_or_default().to_string())
    .collect()
}

// --- the policy: layers, scopes, expiry ----------------------------------------------------------

/// Design 2.8 §7.1: scalars override, `exemptions` accumulate — the treatment ADR 0008 gives
/// `exclude` and ADR 0016 repeats here. A per-run `--on-finding block` that silently dropped the
/// project's accepted exemptions would block on exactly the findings a team had already decided
/// about.
#[test]
fn a_later_policy_layer_overrides_scalars_and_adds_to_the_exemptions() {
  let project = json!({ "on-finding": "warn",
    "exemptions": [ { "path": "response.body.status", "reason": "tracked as ORD-451" } ] });
  let over_run = json!({ "on-finding": "block",
    "exemptions": [ { "consumer": "legacy", "reason": "retired next quarter" } ] });

  let policy = SubsumptionPolicy::resolve(&[Some(&project), Some(&over_run)]).expect("both layers");

  assert_eq!(policy.on_finding, Action::Block, "the later layer wins");
  assert_eq!(
    policy.on_review,
    Action::Warn,
    "nobody set it, so ADR 0016's default stands"
  );
  assert_eq!(
    policy.exemptions.len(),
    2,
    "accumulated, not replaced: {:?}",
    policy.exemptions
  );
}

#[test]
fn the_specifications_own_defaults_are_what_no_layers_resolve_to() {
  let policy = SubsumptionPolicy::resolve(&[]).expect("no layers is a policy");
  assert_eq!(policy, SubsumptionPolicy::default());
  assert_eq!(policy.on_finding, Action::Warn);
  assert_eq!(policy.on_review, Action::Warn);
}

/// `reason` is required (design 2.8 §7.2), and a document that omits it is refused with a
/// position rather than accepted as an anonymous exemption.
#[test]
fn an_exemption_without_a_reason_is_not_a_policy() {
  let layer = json!({ "exemptions": [ { "path": "response.body.status" } ] });
  let problem = SubsumptionPolicy::resolve(&[Some(&layer)]).expect_err("no reason, no exemption");
  assert!(
    problem.pointer.starts_with("/policy/0/exemptions/0"),
    "the position names the exemption: {problem:?}"
  );
  assert!(problem.message.contains("reason"), "{problem:?}");
}

/// Design 2.8 §7.2's three scopes, in one run: the worked example's own exemptions, against the
/// findings of the RFC's scenario. An unset selector matches every value on that axis.
#[test]
fn every_selector_an_exemption_sets_has_to_match_and_the_unset_ones_match_everything() {
  let mut engine = engine();
  let pair = checked_pair(&mut engine);

  let cases = [
    (
      json!({ "path": "response.body.status", "reason": "ORD-451" }),
      1,
      "one field, every consumer",
    ),
    (
      json!({ "interaction": { "description": "get an order", "states": ["an order exists"] },
              "path": "response.body.status", "reason": "ORD-451" }),
      1,
      "the same field, scoped to one interaction by description and states",
    ),
    (
      json!({ "interaction": { "description": "place an order" }, "path": "response.body.status",
              "reason": "ORD-451" }),
      0,
      "a description that names another interaction matches nothing",
    ),
    (
      json!({ "consumer": "order-consumer", "reason": "this consumer reads only the id" }),
      2,
      "no path and no interaction: every finding for that consumer",
    ),
    (
      json!({ "consumer": "someone-else", "reason": "not this consumer" }),
      0,
      "the consumer axis is set, and does not match",
    ),
    (
      json!({ "reason": "everything, deliberately" }),
      2,
      "no selectors at all",
    ),
  ];

  for (exemption, expected, why) in cases {
    let (report, _) = decide(
      &mut engine,
      json!({ "pairs": [pair.clone()],
              "policy": [ { "exemptions": [exemption] } ],
              "as-of": "2026-09-22" }),
    );
    assert_eq!(
      report["pairs"][0]["subsumption"]["exempt"],
      json!(expected),
      "{why}"
    );
  }
}

/// The finding an exemption silences is still in the report, marked. A decision that dropped it
/// would hide the gap a team accepted — which is the one thing an exemption must not buy.
#[test]
fn an_exempted_finding_is_listed_as_exempt_and_the_verdict_still_says_no() {
  let mut engine = engine();
  let pair = checked_pair(&mut engine);
  let (report, text) = decide(
    &mut engine,
    json!({ "pairs": [pair],
            "verification": [verified_summary()],
            "policy": [ { "on-finding": "block",
                          "exemptions": [ { "path": "response.body.status",
                                            "reason": "DELIVERED ships next sprint; ORD-451",
                                            "expires": "2026-12-01" } ] } ],
            "as-of": "2026-09-22" }),
  );
  let pair = &report["pairs"][0];

  assert_eq!(
    pair["subsumption"]["verdict"],
    json!("no"),
    "the walk's verdict is the walk's: policy decides what to do about it, never what it was"
  );
  let exempt: Vec<&Value> = pair["findings"]
    .as_array()
    .expect("findings")
    .iter()
    .filter(|entry| entry["disposition"] == json!("exempt"))
    .collect();
  assert_eq!(exempt.len(), 1);
  assert_eq!(exempt[0]["finding"]["path"], json!("response.body.status"));
  assert_eq!(
    exempt[0]["exemption"]["reason"],
    json!("DELIVERED ships next sprint; ORD-451")
  );
  assert_eq!(pair["exemptions"][0]["status"], json!("applied"));
  assert_eq!(pair["exemptions"][0]["matched"], json!(1));
  assert!(
    text.contains("~ interaction 'get an order', response body $.status:"),
    "an exempted finding is printed with the page's own marker: {text}"
  );
  assert!(
    text.contains("exempted until 2026-12-01: DELIVERED ships next sprint; ORD-451"),
    "{text}"
  );
}

/// Design 2.8 §7.2, as amended by this task: an exemption past its `expires` silences nothing,
/// and the report says an exemption lapsed rather than letting the finding reappear unexplained.
#[test]
fn an_exemption_past_its_expiry_lapses_and_what_it_covered_is_decided_again() {
  let mut engine = engine();
  let pair = checked_pair(&mut engine);
  let policy = json!([ { "on-finding": "block",
                         "exemptions": [ { "path": "response.body.status",
                                           "reason": "ORD-451", "expires": "2026-12-01" } ] } ]);

  let (before, _) = decide(
    &mut engine,
    json!({ "pairs": [pair.clone()], "policy": policy, "as-of": "2026-12-01" }),
  );
  assert_eq!(
    before["pairs"][0]["subsumption"]["exempt"],
    json!(1),
    "inclusive: an exemption expiring on the 1st still applies on the 1st"
  );
  assert_eq!(before["pairs"][0]["exemptions"][0]["status"], json!("applied"));

  let (after, text) = decide(
    &mut engine,
    json!({ "pairs": [pair], "policy": policy, "as-of": "2026-12-02" }),
  );
  let pair = &after["pairs"][0];
  assert_eq!(pair["subsumption"]["exempt"], json!(0));
  assert_eq!(
    pair["subsumption"]["findings"],
    json!(2),
    "both findings are live again"
  );
  assert_eq!(pair["exemptions"][0]["status"], json!("lapsed"));
  assert_eq!(pair["exemptions"][0]["matched"], json!(0));
  assert!(reason_codes(pair).contains(&"exemption-lapsed".to_string()));
  assert_eq!(pair["decision"], json!("block"));
  assert!(text.contains("lapsed exemption (2026-12-01)"), "{text}");
}

/// The engine has no clock, so a host that supplies no date gets exemptions applied and every
/// date reported as unevaluated — never a guess about what "today" is.
#[test]
fn with_no_date_to_judge_against_no_expiry_is_evaluated() {
  let mut engine = engine();
  let pair = checked_pair(&mut engine);
  let (report, _) = decide(
    &mut engine,
    json!({ "pairs": [pair],
            "policy": [ { "exemptions": [ { "path": "response.body.status", "reason": "ORD-451",
                                            "expires": "1999-01-01" } ] } ] }),
  );
  let pair = &report["pairs"][0];
  assert_eq!(pair["exemptions"][0]["status"], json!("applied"));
  assert!(reason_codes(pair).contains(&"expiry-not-evaluated".to_string()));
  assert!(
    report.get("as-of").is_none(),
    "a report that invented a date would be unreproducible: {report}"
  );
}

/// Design 2.8 §7.2 asks task 7.4 to surface an exemption with no expiry, and one that matched
/// nothing. Both are notes: the first is a smell, not a fault, and the second is often a gap that
/// closed.
#[test]
fn an_exemption_with_no_expiry_and_one_that_matched_nothing_are_both_surfaced() {
  let mut engine = engine();
  let pair = checked_pair(&mut engine);
  let (report, _) = decide(
    &mut engine,
    json!({ "pairs": [pair],
            "policy": [ { "exemptions": [
              { "path": "response.body.status", "reason": "the provider will never narrow this" },
              { "path": "response.body.nothing", "reason": "fixed last month, probably" } ] } ],
            "as-of": "2026-09-22" }),
  );
  let pair = &report["pairs"][0];
  let codes = reason_codes(pair);
  assert!(codes.contains(&"exemption-no-expiry".to_string()), "{codes:?}");
  assert!(codes.contains(&"exemption-unused".to_string()), "{codes:?}");
  assert_eq!(pair["exemptions"][1]["status"], json!("unused"));
  assert_eq!(
    pair["decision"],
    json!("warn"),
    "neither is a fault: the live findings are what warned"
  );
}

// --- the decision: what blocks, what warns, and what no policy governs ---------------------------

#[test]
fn the_default_policy_warns_on_findings_and_does_not_block() {
  let mut engine = engine();
  let pair = checked_pair(&mut engine);
  let (report, _) = decide(
    &mut engine,
    json!({ "pairs": [pair], "verification": [verified_summary()] }),
  );
  assert_eq!(report["decision"], json!("warn"));
  assert_eq!(report["policy"]["on-finding"], json!("warn"));
  assert_eq!(
    report["summary"],
    json!({ "pairs": 1, "blocked": 0, "warned": 1, "passed": 0,
                                        "findings": 2, "reviews": 0, "exempt": 0 })
  );
}

#[test]
fn on_finding_block_is_what_turns_the_same_report_into_a_no() {
  let mut engine = engine();
  let pair = checked_pair(&mut engine);
  let (report, _) = decide(
    &mut engine,
    json!({ "pairs": [pair], "verification": [verified_summary()],
            "policy": [ { "on-finding": "block" } ] }),
  );
  assert_eq!(report["decision"], json!("block"));
  let reasons = report["pairs"][0]["reasons"].as_array().unwrap().clone();
  let blocked: Vec<&Value> = reasons
    .iter()
    .filter(|reason| reason["action"] == json!("block"))
    .collect();
  assert_eq!(blocked.len(), 1);
  assert_eq!(blocked[0]["code"], json!("subsumption-findings"));
}

/// The two severities are independent questions (design 2.8 §7.1), which is why the CLI spells
/// them as two flags: "block on what is decided, warn on what cannot be" is the combination ADR
/// 0016 expects a team to reach first.
#[test]
fn on_review_is_resolved_separately_from_on_finding() {
  let mut engine = engine();
  // A consumer and a provider whose only difference is one the checker cannot decide: two
  // regexes (shape spec §8's conservative class).
  let contract = json!({ "$format": "janus-contract/1",
    "consumer": { "name": "order-consumer" }, "provider": { "name": "order-service" },
    "interactions": [ { "description": "get an order",
      "parts": { "response": { "body": { "shape": "regex", "pattern": "\\d{4}", "example": "1234" } } },
      "selection": { "variants": [], "report": {} } } ] });
  let shape = json!({ "$format": "janus-provider-shape/1",
    "provider": { "name": "order-service" },
    "interactions": [ { "description": "get an order",
      "parts": { "response": { "body": { "shape": "regex", "pattern": "[0-9]{4}", "example": "1234" } } } } ] });
  let checked = ok(
    &mut engine,
    "subsumption/check",
    json!({ "contract": contract, "provider-shape": shape }),
  );
  let pair = json!({ "consumer": { "name": "order-consumer" }, "provider": { "name": "order-service" },
                     "subsumption": checked["report"] });

  let (warned, _) = decide(
    &mut engine,
    json!({ "pairs": [pair.clone()], "verification": [verified_summary()],
            "policy": [ { "on-finding": "block" } ] }),
  );
  assert_eq!(warned["pairs"][0]["subsumption"]["reviews"], json!(1));
  assert_eq!(
    warned["decision"],
    json!("warn"),
    "on-finding says nothing about a review"
  );

  let (blocked, _) = decide(
    &mut engine,
    json!({ "pairs": [pair], "verification": [verified_summary()],
            "policy": [ { "on-review": "block" } ] }),
  );
  assert_eq!(blocked["decision"], json!("block"));
}

/// Verification is not policy-governed: a run that found mismatches is a decided incompatibility
/// on the evidence, and an exemption list has nothing to say about it.
#[test]
fn a_failed_verification_blocks_whatever_the_policy_says() {
  let mut engine = engine();
  let pair = checked_pair(&mut engine);
  let mut summary = verified_summary();
  summary["status"] = json!("failed");
  summary["variants"] =
    json!({ "total": 4, "verified": 3, "failed": 1, "state-unavailable": 0, "skipped": 0 });
  summary["failures"] = json!([
    { "interaction": { "contract": { "consumer": "order-consumer", "provider": "order-service" },
                       "description": "get an order" },
      "variant": "v-1", "status": "failed",
      "mismatches": [ { "path": "$.status", "message": "expected 'PENDING' but got 'DELIVERED'" } ] } ]);

  let (report, _) = decide(
    &mut engine,
    json!({ "pairs": [pair], "verification": [summary],
            "policy": [ { "on-finding": "warn", "on-review": "warn",
                          "exemptions": [ { "reason": "everything, deliberately" } ] } ],
            "as-of": "2026-09-22" }),
  );
  let pair = &report["pairs"][0];
  assert_eq!(pair["verification"]["status"], json!("failed"));
  assert_eq!(pair["decision"], json!("block"));
  assert!(reason_codes(pair).contains(&"verification-failed".to_string()));
  assert_eq!(
    pair["subsumption"]["exempt"],
    json!(2),
    "the exemption silenced every subsumption finding, and the pair still blocks"
  );
}

#[test]
fn a_run_a_hook_aborted_is_incomplete_and_blocks() {
  let mut engine = engine();
  let pair = checked_pair(&mut engine);
  let mut summary = verified_summary();
  summary["status"] = json!("failed");
  summary["aborted"] = json!({ "point": "before-verification", "hook": "auth" });

  let (report, _) = decide(&mut engine, json!({ "pairs": [pair], "verification": [summary] }));
  let pair = &report["pairs"][0];
  assert_eq!(pair["verification"]["status"], json!("incomplete"));
  assert!(reason_codes(pair).contains(&"verification-incomplete".to_string()));
  assert_eq!(pair["decision"], json!("block"));
}

#[test]
fn a_pair_nobody_verified_warns_and_a_filtered_run_says_so() {
  let mut engine = engine();
  let pair = checked_pair(&mut engine);

  let (missing, _) = decide(&mut engine, json!({ "pairs": [pair.clone()] }));
  let missing = &missing["pairs"][0];
  assert_eq!(missing["verification"]["status"], json!("unknown"));
  assert_eq!(missing["verification"]["runs"], json!(0));
  assert!(reason_codes(missing).contains(&"verification-missing".to_string()));

  let mut summary = verified_summary();
  summary["filtered"] = json!(true);
  let (filtered, _) = decide(&mut engine, json!({ "pairs": [pair], "verification": [summary] }));
  let filtered = &filtered["pairs"][0];
  assert_eq!(filtered["verification"]["filtered"], json!(true));
  assert!(reason_codes(filtered).contains(&"verification-filtered".to_string()));
}

/// A run covering several contracts is attributed per pair by its own `consumers`/`providers` and
/// `failures` — and its variant counts, which belong to the whole run, are left out rather than
/// divided between pairs.
#[test]
fn a_run_over_two_contracts_is_attributed_to_each_pair_separately() {
  let mut engine = engine();
  let pair = checked_pair(&mut engine);
  let summary = json!({ "status": "failed", "contracts": 2,
    "consumers": ["order-consumer", "billing-sync"], "providers": ["order-service", "order-service"],
    "interactions": 2, "filtered": false,
    "variants": { "total": 8, "verified": 7, "failed": 1, "state-unavailable": 0, "skipped": 0 },
    "failures": [ { "interaction": { "contract": { "consumer": "billing-sync", "provider": "order-service" } },
                    "variant": "v-1", "status": "failed" } ] });

  let (report, _) = decide(
    &mut engine,
    json!({ "pairs": [ pair,
                       { "consumer": { "name": "billing-sync" }, "provider": { "name": "order-service" } } ],
            "verification": [summary] }),
  );
  assert_eq!(report["pairs"][0]["verification"]["status"], json!("verified"));
  assert_eq!(report["pairs"][1]["verification"]["status"], json!("failed"));
  assert!(
    report["pairs"][0]["verification"].get("variants").is_none(),
    "one set of counts for two pairs is not a per-pair fact: {}",
    report["pairs"][0]["verification"]
  );
  assert_eq!(report["decision"], json!("block"));
  assert_eq!(report["summary"]["blocked"], json!(1));
  assert_eq!(report["summary"]["warned"], json!(1));
}

/// The RFC's per-provider adoption path: a provider that publishes no shape gets replay-only
/// semantics — today's behaviour, which is a pass, and a note rather than silence.
#[test]
fn a_provider_that_published_no_shape_passes_on_its_verification_alone() {
  let mut engine = engine();
  let (report, text) = decide(
    &mut engine,
    json!({ "pairs": [ { "consumer": { "name": "order-consumer" },
                         "provider": { "name": "order-service" },
                         "format": "janus-contract/1" } ],
            "verification": [verified_summary()],
            "policy": [ { "on-finding": "block", "on-review": "block" } ] }),
  );
  let pair = &report["pairs"][0];
  assert_eq!(pair["subsumption"]["verdict"], json!("not-checked"));
  assert_eq!(pair["decision"], json!("pass"));
  assert_eq!(report["decision"], json!("pass"));
  assert!(reason_codes(pair).contains(&"provider-shape-missing".to_string()));
  assert!(
    text.contains("? order-consumer was not checked against order-service: no shapes published"),
    "{text}"
  );
}

/// An interaction with no published entry is `not-published` (design 2.8 §6.3): not checked, and
/// not silently passing either.
#[test]
fn an_interaction_the_provider_published_nothing_for_is_counted_not_passed() {
  let mut engine = engine();
  let mut contract = consumer_contract();
  let mut second = contract["interactions"][0].clone();
  second["description"] = json!("get an order that does not exist");
  contract["interactions"].as_array_mut().unwrap().push(second);

  let checked = ok(
    &mut engine,
    "subsumption/check",
    json!({ "contract": contract, "provider-shape": provider_shape() }),
  );
  let (report, _) = decide(
    &mut engine,
    json!({ "pairs": [ { "consumer": { "name": "order-consumer" },
                         "provider": { "name": "order-service" },
                         "subsumption": checked["report"] } ],
            "verification": [verified_summary()] }),
  );
  let pair = &report["pairs"][0];
  assert_eq!(pair["subsumption"]["interactions"], json!(2));
  assert_eq!(pair["subsumption"]["matched"], json!(1));
  assert_eq!(pair["subsumption"]["not-published"], json!(1));
  assert!(reason_codes(pair).contains(&"interactions-not-published".to_string()));
}

/// A shape that covers none of the contract's interactions has decided as much as no shape at
/// all, and says so — `yes` is what an empty conjunction folds to, and it would read as coverage.
#[test]
fn a_shape_that_matches_nothing_is_not_checked_rather_than_passing() {
  let mut engine = engine();
  let mut shape = provider_shape();
  shape["interactions"][0]["description"] = json!("some other operation");
  let checked = ok(
    &mut engine,
    "subsumption/check",
    json!({ "contract": consumer_contract(), "provider-shape": shape }),
  );
  let (report, text) = decide(
    &mut engine,
    json!({ "pairs": [ { "consumer": { "name": "order-consumer" },
                         "provider": { "name": "order-service" },
                         "subsumption": checked["report"] } ],
            "verification": [verified_summary()],
            "policy": [ { "on-finding": "block" } ] }),
  );
  let pair = &report["pairs"][0];
  assert_eq!(pair["subsumption"]["verdict"], json!("not-checked"));
  assert_eq!(pair["subsumption"]["matched"], json!(0));
  assert_eq!(pair["decision"], json!("pass"));
  assert!(text.contains("no shapes published"), "{text}");
}

// --- the operations themselves -------------------------------------------------------------------

/// A v1–v4 pact is a consumer document like any other (plan task 5.4's rule, applied to the
/// check): converted on the way in, and `format` says what it was.
#[test]
fn check_reads_a_v1_v4_pact_as_the_consumer_side() {
  let mut engine = engine();
  let pact = json!({ "consumer": { "name": "order-consumer" },
    "provider": { "name": "order-service" },
    "interactions": [ { "description": "get an order",
      "providerStates": [ { "name": "an order exists" } ],
      "request": { "method": "GET", "path": "/orders/66" },
      "response": { "status": 200, "body": { "status": "SHIPPED" } } } ],
    "metadata": { "pactSpecification": { "version": "3.0.0" } } });

  let result = ok(
    &mut engine,
    "subsumption/check",
    json!({ "contract": pact, "provider-shape": provider_shape() }),
  );
  assert_eq!(result["format"], json!("pact/3.0.0"));
  let findings = result["report"]["interactions"][0]["findings"]
    .as_array()
    .expect("findings");
  assert!(
    findings
      .iter()
      .any(|finding| finding["path"] == json!("response.body.status")
        && finding["kind"] == json!("wider-values")),
    "the pact froze one example as `equality`, so the provider's three options are wider: \
     {findings:?}"
  );
}

#[test]
fn a_contract_and_a_shape_about_different_providers_are_refused_by_name() {
  let mut engine = engine();
  let mut shape = provider_shape();
  shape["provider"]["name"] = json!("another-service");
  let response = send(
    &mut engine,
    "subsumption/check",
    json!({ "contract": consumer_contract(), "provider-shape": shape }),
  );
  assert_eq!(response["error"]["code"], json!("document-mismatched"));
  assert_eq!(response["error"]["category"], json!("document"));
  assert_eq!(response["error"]["details"]["contract"], json!("order-service"));
  assert_eq!(
    response["error"]["details"]["provider-shape"],
    json!("another-service")
  );
}

#[test]
fn a_document_that_is_neither_a_contract_nor_a_pact_is_not_read_as_an_empty_one() {
  let mut engine = engine();
  let response = send(
    &mut engine,
    "subsumption/check",
    json!({ "contract": { "hello": "world" }, "provider-shape": provider_shape() }),
  );
  assert_eq!(response["error"]["code"], json!("contract-invalid"));
  assert_eq!(response["error"]["category"], json!("document"));
  assert_eq!(
    response["error"]["details"]["member"],
    json!("contract"),
    "a check reads two documents of different kinds, and the error says which one: {}",
    response["error"]
  );

  let response = send(
    &mut engine,
    "subsumption/check",
    json!({ "contract": consumer_contract(),
            "provider-shape": { "$format": "janus-provider-shape/9", "provider": { "name": "x" },
                                "interactions": [] } }),
  );
  assert_eq!(
    response["error"]["code"],
    json!("contract-version-unsupported"),
    "a major this checker does not implement is a different fact from 'not a provider shape' \
     (design 2.8 §8)"
  );
  assert_eq!(response["error"]["details"]["member"], json!("provider-shape"));
  assert_eq!(
    response["error"]["details"]["found"],
    json!("janus-provider-shape/9")
  );
}

#[test]
fn a_policy_layer_that_is_not_one_fails_the_decision_with_a_position() {
  let mut engine = engine();
  let pair = checked_pair(&mut engine);
  let response = send(
    &mut engine,
    "subsumption/decide",
    json!({ "pairs": [pair], "policy": [ { "on-finding": "maybe" } ] }),
  );
  assert_eq!(response["error"]["code"], json!("contract-invalid"));
  let problems = response["error"]["details"]["problems"]
    .as_array()
    .expect("problems");
  assert!(
    problems[0]["pointer"]
      .as_str()
      .unwrap_or_default()
      .starts_with("/policy/0"),
    "which layer, and where in it: {problems:?}"
  );
}

// --- the page, and the document -----------------------------------------------------------------

/// M5, as the RFC sketches it: the header line and the two lines under it, verbatim. Design 2.8
/// §6.4 fixes that block; this page puts a decision around it without touching it.
#[test]
fn the_page_reproduces_the_rfcs_own_sketch() {
  let mut engine = engine();
  let pair = checked_pair(&mut engine);
  let (_, text) = decide(
    &mut engine,
    json!({ "pairs": [pair], "verification": [verified_summary()],
            "policy": [ { "on-finding": "block" } ], "as-of": "2026-09-22" }),
  );

  for line in [
    "✗ order-consumer is not compatible with order-service",
    "  interaction 'get an order', response body $.status:",
    "    provider may produce: 'PENDING' | 'SHIPPED' | 'DELIVERED'",
    "    consumer has only tested: 'PENDING' | 'SHIPPED'",
    "  interaction 'get an order', response body $.shippedAt:",
    "    provider may produce any string, or absent",
    "    consumer has only tested any string, always present",
  ] {
    assert!(text.contains(line), "missing line {line:?} in:\n{text}");
  }
  assert!(
    text.contains("  verification: verified (4 of 4 variant(s))"),
    "{text}"
  );
  assert!(
    text.contains("=> BLOCK: order-consumer -> order-service"),
    "{text}"
  );
  assert!(
    text.contains("BLOCK: 1 pair(s) blocked, 0 warned, 0 passed"),
    "{text}"
  );
}

#[test]
fn the_report_validates_against_the_shipped_schema() {
  let mut engine = engine();
  let pair = checked_pair(&mut engine);
  let (report, _) = decide(
    &mut engine,
    json!({ "pairs": [ pair,
                       { "consumer": { "name": "billing-sync" }, "provider": { "name": "order-service" } } ],
            "verification": [verified_summary()],
            "policy": [ { "on-finding": "block",
                          "exemptions": [
                            { "path": "response.body.status", "reason": "ORD-451", "expires": "2026-12-01" },
                            { "interaction": { "description": "get an order", "states": ["an order exists"] },
                              "consumer": "order-consumer", "reason": "no expiry, deliberately" } ] } ],
            "as-of": "2026-09-22" }),
  );

  let validator = protocol_validator("subsumption.schema.json#/$defs/CompatibilityReport");
  let problems: Vec<String> = validator
    .iter_errors(&report)
    .map(|err| format!("  {}: {err}", err.instance_path()))
    .collect();
  assert!(
    problems.is_empty(),
    "the report does not validate:\n{}\n{report:#}",
    problems.join("\n")
  );
  assert_eq!(report["$format"], json!("janus-compatibility-report/1"));
}
