//! Plan task 7.2: recording a provider shape from the provider's own tests.
//!
//! Two halves. The first pins the inference rules one at a time — what evidence produces a literal
//! set, what produces a kind predicate, what produces `optional` — because those are judgements
//! (`subsumption::record`'s module docs) and a judgement nobody can see the boundary of is a
//! surprise waiting to happen. The second drives the *real* sample provider (task 5.6) the way its
//! own tests do, records what it answers, and checks the artifact against the consumer pact that
//! provider already has — which is M5's scenario, reached from the recording side.

use pact_janus_kernel::contract::{IdentifyMode, read as read_contract};
use pact_janus_kernel::shape::parse as parse_shape;
use pact_janus_kernel::subsumption::{
  ProviderShape, Recorder, RecordingPolicy, Severity, check, read_provider_shape,
};
use pact_janus_sample_order_service::{Config, DEFAULT_TOKEN, start};
use serde_json::{Value, json};
use std::collections::BTreeMap;

// --- harness ------------------------------------------------------------------------------------

/// Record `bodies` as successive observations of one interaction's response body, and hand back
/// the shape recorded at `response.body`.
fn record_body(bodies: &[Value]) -> Value {
  record_body_under(bodies, RecordingPolicy::default())
}

fn record_body_under(bodies: &[Value], policy: RecordingPolicy) -> Value {
  let mut recorder = Recorder::with_policy("orders-api", policy);
  for body in bodies {
    recorder.observe("get an order", &[], &parts(body.clone()));
  }
  let recorded = recorder.finish();
  let shape = recorded.interactions[0].parts["response"]["body"].clone();
  // Everything this module emits has to be a shape the engine will actually accept — a recorder
  // that writes a document the parser rejects has recorded nothing.
  parse_shape(&shape, "/body")
    .unwrap_or_else(|problems| panic!("the recorded shape is ill-formed: {problems:?}"));
  shape
}

fn parts(body: Value) -> BTreeMap<String, BTreeMap<String, Value>> {
  BTreeMap::from([(
    "response".to_string(),
    BTreeMap::from([("body".to_string(), body)]),
  )])
}

fn member(shape: &Value, name: &str) -> Value {
  shape["members"][name].clone()
}

// --- scalars: a set of values, or samples of an open domain ---------------------------------------

#[test]
fn a_scalar_seen_once_is_recorded_as_the_value_it_was() {
  let shape = record_body(&[json!({ "channel": "web" })]);
  assert_eq!(
    member(&shape, "channel"),
    json!({ "shape": "equality", "example": "web" })
  );
}

#[test]
fn values_that_repeat_are_a_set_the_provider_chooses_from() {
  // The RFC's own example: a status that takes three values over six responses is an enum, and
  // recording it as one is what lets a consumer's narrower `any-of` be found by the walk.
  let shape = record_body(&[
    json!({ "status": "PENDING" }),
    json!({ "status": "SHIPPED" }),
    json!({ "status": "PENDING" }),
    json!({ "status": "DELIVERED" }),
    json!({ "status": "SHIPPED" }),
    json!({ "status": "CANCELLED" }),
  ]);
  assert_eq!(
    member(&shape, "status"),
    json!({ "shape": "any-of",
            "options": ["CANCELLED", "DELIVERED", "PENDING", "SHIPPED"],
            "example": "CANCELLED" }),
    "options are sorted, so two runs of the same suite record the same document"
  );
}

#[test]
fn values_that_never_repeat_are_an_open_domain() {
  // Three timestamps in three observations is not a three-option enum, and the difference from
  // the test above is the repetition, which is the only evidence there is.
  let shape = record_body(&[
    json!({ "shippedAt": "2026-07-30T10:00:00Z" }),
    json!({ "shippedAt": "2026-08-01T09:15:00Z" }),
    json!({ "shippedAt": "2026-08-04T17:42:00Z" }),
  ]);
  assert_eq!(
    member(&shape, "shippedAt"),
    json!({ "shape": "string", "example": "2026-07-30T10:00:00Z" })
  );
}

#[test]
fn two_observations_are_not_yet_evidence_of_anything() {
  // Below `min_evidence`, "every value was new" says nothing — two different values might be an
  // open domain or a two-case enum, and the recorder does not get to pick.
  let shape = record_body(&[json!({ "tier": "gold" }), json!({ "tier": "silver" })]);
  assert_eq!(
    member(&shape, "tier"),
    json!({ "shape": "any-of", "options": ["gold", "silver"], "example": "gold" })
  );
}

#[test]
fn more_distinct_values_than_the_policy_allows_widens_whatever_the_repetition() {
  let bodies: Vec<Value> = (0..12)
    .flat_map(|n| {
      [
        json!({ "sku": format!("sku-{n}") }),
        json!({ "sku": format!("sku-{n}") }),
      ]
    })
    .collect();
  let shape = record_body(&bodies);
  assert_eq!(
    member(&shape, "sku"),
    json!({ "shape": "string", "example": "sku-0" }),
    "12 distinct values repeat, and are still past max_options"
  );

  // ...and the boundary is the policy's, not a constant nobody can move.
  let roomier = RecordingPolicy {
    max_options: 12,
    ..RecordingPolicy::default()
  };
  let shape = record_body_under(&bodies, roomier);
  assert_eq!(member(&shape, "sku")["shape"], json!("any-of"));
}

#[test]
fn numbers_record_the_kind_lattice_they_were_seen_at() {
  for (values, expected) in [
    (vec![json!(1), json!(2), json!(3), json!(4)], "integer"),
    (vec![json!(1.5), json!(2.5), json!(3.5), json!(4.5)], "decimal"),
    (vec![json!(1), json!(2.5), json!(3), json!(4.5)], "number"),
  ] {
    let bodies: Vec<Value> = values.into_iter().map(|n| json!({ "amount": n })).collect();
    let shape = record_body(&bodies);
    assert_eq!(member(&shape, "amount")["shape"], json!(expected));
  }
}

#[test]
fn a_status_code_is_an_enum_and_an_order_id_is_not() {
  // Both are numbers; what separates them is that one repeats and the other does not, which is
  // the whole of the rule and the reason it is not "strings get sets, numbers get kinds".
  let bodies: Vec<Value> = [200, 200, 404, 200, 404, 500]
    .iter()
    .zip([11, 12, 13, 14, 15, 16])
    .map(|(status, id)| json!({ "status": status, "id": id }))
    .collect();
  let shape = record_body(&bodies);
  assert_eq!(member(&shape, "status")["shape"], json!("any-of"));
  assert_eq!(member(&shape, "id"), json!({ "shape": "integer", "example": 11 }));
}

#[test]
fn both_booleans_are_the_kind_predicate_rather_than_a_two_option_set() {
  let shape = record_body(&[
    json!({ "expedited": true }),
    json!({ "expedited": false }),
    json!({ "expedited": true }),
  ]);
  assert_eq!(member(&shape, "expedited"), json!({ "shape": "boolean" }));
}

// --- presence, nullability and the kinds that do not fit together ---------------------------------

#[test]
fn a_member_missing_from_some_responses_is_optional() {
  let shape = record_body(&[
    json!({ "id": "1", "shippedAt": "2026-07-30T10:00:00Z" }),
    json!({ "id": "2" }),
    json!({ "id": "3", "shippedAt": "2026-08-01T09:15:00Z" }),
  ]);
  assert_eq!(member(&shape, "shippedAt")["shape"], json!("optional"));
  assert_eq!(
    member(&shape, "id")["shape"],
    json!("string"),
    "a member present every time carries no presence modifier"
  );
}

#[test]
fn a_member_sometimes_null_is_nullable_and_one_always_null_is_the_null_kind() {
  let sometimes = record_body(&[
    json!({ "note": "gift wrap" }),
    json!({ "note": Value::Null }),
    json!({ "note": "no rush" }),
  ]);
  assert_eq!(member(&sometimes, "note")["shape"], json!("nullable"));

  let always = record_body(&[json!({ "note": Value::Null }), json!({ "note": Value::Null })]);
  assert_eq!(member(&always, "note"), json!({ "shape": "null" }));
}

#[test]
fn absence_and_null_are_recorded_as_the_two_different_things_they_are() {
  let shape = record_body(&[
    json!({ "note": "gift wrap" }),
    json!({ "note": Value::Null }),
    json!({}),
  ]);
  let note = member(&shape, "note");
  assert_eq!(note["shape"], json!("optional"));
  assert_eq!(note["of"]["shape"], json!("nullable"));
}

#[test]
fn a_position_that_held_different_kinds_of_thing_is_any() {
  // Not a guess about which one "really" belongs there: `any` is the only core operator that
  // admits both, and a recorder that picked would be inventing evidence.
  let shape = record_body(&[
    json!({ "detail": { "code": 7 } }),
    json!({ "detail": "unavailable" }),
  ]);
  assert_eq!(member(&shape, "detail"), json!({ "shape": "any" }));
}

// --- collections ----------------------------------------------------------------------------------

#[test]
fn a_list_records_the_lengths_it_was_seen_at_and_one_shape_for_its_elements() {
  let shape = record_body(&[
    json!({ "items": [] }),
    json!({ "items": [ { "sku": "a", "quantity": 1 } ] }),
    json!({ "items": [ { "sku": "b", "quantity": 2 }, { "sku": "c", "quantity": 3 } ] }),
  ]);
  let items = member(&shape, "items");
  assert_eq!(items["shape"], json!("each-like"));
  assert_eq!(items["min"], json!(0));
  assert_eq!(items["max"], json!(2));
  assert_eq!(items["items"]["shape"], json!("object"));
  assert_eq!(
    items["items"]["members"]["quantity"],
    json!({ "shape": "integer", "example": 1 }),
    "every element contributes to one shape (shape spec §6.5)"
  );
}

#[test]
fn a_list_only_ever_seen_empty_records_its_cardinality_and_says_nothing_about_elements() {
  let shape = record_body(&[json!({ "items": [] }), json!({ "items": [] })]);
  assert_eq!(
    member(&shape, "items"),
    json!({ "shape": "each-like", "min": 0, "max": 0, "items": { "shape": "any" } })
  );
}

// --- the document ---------------------------------------------------------------------------------

#[test]
fn interactions_are_kept_apart_by_description_and_state_names() {
  let mut recorder = Recorder::new("orders-api");
  recorder.observe(
    "get an order",
    &["an order exists".to_string()],
    &parts(json!({ "id": "1" })),
  );
  recorder.observe(
    "get an order",
    &["no orders exist".to_string()],
    &parts(json!({ "error": "not found" })),
  );
  recorder.observe("list orders", &[], &parts(json!([])));
  let recorded = recorder.finish();

  assert_eq!(recorded.interactions.len(), 3);
  let found = recorded
    .find("get an order", &["an order exists".to_string()])
    .expect("matched by description and state name together");
  assert_eq!(
    found.parts["response"]["body"]["members"]["id"]["shape"],
    json!("equality")
  );
}

#[test]
fn the_recorded_document_says_how_it_came_to_exist() {
  let mut recorder = Recorder::new("orders-api");
  for _ in 0..4 {
    recorder.observe("get an order", &[], &parts(json!({ "id": "1" })));
  }
  let recorded = recorder.finish();
  assert_eq!(recorded.provenance.as_deref(), Some("recorded"));
  assert_eq!(recorded.provenance_of(&recorded.interactions[0]), "recorded");
  assert_eq!(
    recorded.interactions[0].source.as_ref().unwrap()["observations"],
    json!(4),
    "how much evidence is behind the shape, which is the first thing a reader of a recorded \
     shape wants to know"
  );
}

#[test]
fn a_recorded_document_validates_against_the_shipped_schema_and_reads_back() {
  let mut recorder = Recorder::new("orders-api");
  recorder.observe(
    "get an order",
    &["an order exists".to_string()],
    &parts(json!({ "id": "66", "items": [ { "sku": "a" } ] })),
  );
  let recorded = recorder.finish();
  let instance = serde_json::to_value(&recorded).expect("a provider shape always serializes");

  let schema_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    .join("../../Documentation/specs/subsumption-check/schemas/v1/provider-shape.schema.json");
  let schema_bytes =
    std::fs::read(&schema_path).unwrap_or_else(|err| panic!("reading {schema_path:?}: {err}"));
  let schema: Value = serde_json::from_slice(&schema_bytes).expect("valid schema JSON");
  jsonschema::validate(&schema, &instance)
    .unwrap_or_else(|err| panic!("the recorded document does not validate: {err}"));

  let round_tripped = read_provider_shape(instance.to_string().as_bytes()).expect("reads back");
  assert_eq!(round_tripped, recorded);
}

// --- the sample provider (plan task 5.6), recorded and then checked -------------------------------

fn get(base_url: &str, path: &str) -> Value {
  let agent: ureq::Agent = ureq::Agent::config_builder()
    .http_status_as_error(false)
    .build()
    .into();
  let mut response = agent
    .get(format!("{base_url}{path}"))
    .header("authorization", &format!("Bearer {DEFAULT_TOKEN}"))
    .call()
    .expect("the provider answers");
  let body = response.body_mut().read_to_string().expect("a body");
  serde_json::from_str(&body).unwrap_or(Value::Null)
}

fn set_state(base_url: &str, state: Value) {
  let agent: ureq::Agent = ureq::Agent::config_builder()
    .http_status_as_error(false)
    .build()
    .into();
  agent
    .post(format!("{base_url}/_pact/provider-states"))
    .header("content-type", "application/json")
    .send(serde_json::to_string(&state).unwrap())
    .expect("the provider answers");
}

/// The provider's own tests, as a recorder sees them: set a state, make the call, record what came
/// back. Six responses covering the variance the sample provider was built to have (task 5.6).
fn record_the_sample_provider() -> ProviderShape {
  let provider = start(Config::default()).expect("an OS-assigned port always binds");
  let base_url = provider.base_url();
  let mut recorder = Recorder::new("order-service");

  for (status, shipped, items) in [
    ("PENDING", false, 1),
    ("SHIPPED", true, 2),
    ("PENDING", false, 3),
    ("CANCELLED", false, 0),
    ("SHIPPED", true, 1),
    ("PENDING", false, 2),
  ] {
    set_state(
      base_url,
      json!({ "action": "setup", "state": "an order exists",
              "params": { "id": "66", "status": status, "shipped": shipped, "items": items } }),
    );
    let body = get(base_url, "/orders/66");
    recorder.observe(
      "a request for an order",
      &["an order exists".to_string()],
      &parts(body),
    );
  }
  recorder.finish()
}

#[test]
fn the_sample_provider_records_the_variance_it_was_built_to_have() {
  let recorded = record_the_sample_provider();
  let interaction = recorded
    .find("a request for an order", &["an order exists".to_string()])
    .expect("one interaction, recorded");
  let body = &interaction.parts["response"]["body"];
  parse_shape(body, "/body").expect("a well-formed shape");

  assert_eq!(
    body["members"]["status"]["options"],
    json!(["CANCELLED", "PENDING", "SHIPPED"]),
    "the provider can produce CANCELLED, and now says so — the RFC's own scenario, from the \
     provider's side"
  );
  assert_eq!(
    body["members"]["shippedAt"]["shape"],
    json!("optional"),
    "absent on an order that has not shipped, which is variance, not an error"
  );
  assert_eq!(body["members"]["items"]["min"], json!(0));
  assert!(
    body["members"].get("channel").is_some(),
    "the member no consumer contract declares is recorded like any other: the recorder does not \
     know which members anyone asked for"
  );
}

#[test]
fn the_recorded_shape_finds_the_undeclared_variance_in_the_consumers_contract() {
  // M5, reached from the recording side: the provider's own tests produce a status the consumer
  // never tested, and the walk says so. The consumer document here is the v3 pact task 5.4
  // verifies against this provider, upgraded — the same contract, from the other direction.
  let recorded = record_the_sample_provider();
  let consumer = consumer_contract();
  let report = check(&consumer, &recorded).expect("both documents parse");

  let interaction = &report.interactions[0];
  assert_eq!(interaction.verdict, "no");
  let wider: Vec<&str> = interaction
    .findings
    .iter()
    .filter(|finding| finding.kind == "wider-values" && finding.severity == Severity::Finding)
    .map(|finding| finding.path.as_str())
    .collect();
  assert_eq!(
    wider,
    vec!["response.body.status"],
    "one decided finding, and it is the status the consumer never tested: {:?}",
    interaction.findings
  );
  let status = interaction
    .findings
    .iter()
    .find(|finding| finding.path == "response.body.status")
    .expect("the status finding");
  assert_eq!(
    status.provider.as_ref().unwrap().summary,
    "one of 'CANCELLED' | 'PENDING' | 'SHIPPED'"
  );
  assert_eq!(
    status.consumer.as_ref().unwrap().summary,
    "one of 'PENDING' | 'SHIPPED'"
  );
}

/// The document `samples/order-service/shapes/` holds is *this* recording, not a hand-written
/// approximation of it — checked in because `janus check` needs a file to read (plan task 7.4)
/// and a reader needs something to look at, asserted here because a checked-in artifact nobody
/// regenerates is a corpus entry that has quietly stopped being true (CLAUDE.md: corpora are
/// load-bearing). Regenerate it from this recording if the recorder's judgements change.
#[test]
fn the_checked_in_sample_shape_is_what_the_recorder_produces() {
  let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    .join("../../samples/order-service/shapes/order-service.provider-shape.json");
  let checked_in =
    read_provider_shape(&std::fs::read(&path).unwrap_or_else(|err| panic!("reading {path:?}: {err}")))
      .expect("the checked-in document is a provider shape");

  assert_eq!(
    checked_in,
    record_the_sample_provider(),
    "the checked-in provider shape and the recording have diverged; regenerate the file"
  );
}

/// The consumer side of the sample provider's own pact, as a Janus contract: what the consumer
/// declared, not what the provider can do.
fn consumer_contract() -> pact_janus_kernel::contract::Contract {
  let document = json!({ "$format": "janus-contract/1",
    "consumer": { "name": "web-app" },
    "provider": { "name": "order-service" },
    "interactions": [
      { "description": "a request for an order",
        "states": [ { "name": "an order exists" } ],
        "parts": { "response": { "body": {
          "shape": "object",
          "members": {
            "id": { "shape": "string", "example": "66" },
            "status": { "shape": "any-of", "options": ["PENDING", "SHIPPED"], "example": "PENDING" },
            "shippedAt": { "shape": "optional",
                           "of": { "shape": "string", "example": "2026-07-30T10:00:00Z" } },
            "items": { "shape": "each-like", "min": 0,
                       "items": { "shape": "object",
                                  "members": {
                                    "sku": { "shape": "string", "example": "sku-0" },
                                    "quantity": { "shape": "integer", "example": 1 } } } } } } } },
        "selection": { "variants": [], "report": { } } } ] });
  read_contract(document.to_string().as_bytes(), IdentifyMode::Tolerant).expect("a well-formed contract")
}
