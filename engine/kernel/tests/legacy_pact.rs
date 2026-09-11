//! Plan task 3.1: v1–v4 pact reading via `pact_models`, exercised against real-world fixtures
//! (`tests/fixtures/legacy-pacts/README.md`) rather than only synthetic ones.

use pact_janus_kernel::legacy_pact;
use pact_models::PactSpecification;
use rstest::rstest;
use std::path::PathBuf;

fn fixture(name: &str) -> serde_json::Value {
  let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    .join("tests/fixtures/legacy-pacts")
    .join(name);
  let bytes = std::fs::read(&path).unwrap_or_else(|err| panic!("reading {path:?}: {err}"));
  serde_json::from_slice(&bytes).unwrap_or_else(|err| panic!("parsing {path:?}: {err}"))
}

#[rstest]
#[case::v4_http("consumer-provider-v4.json", "consumer", "provider", 2, PactSpecification::V4)]
#[case::v3_provider_states(
  "V3Consumer-ProviderStateService.json",
  "V3Consumer",
  "ProviderStateService",
  1,
  PactSpecification::V3
)]
#[case::v4_matching_rules(
  "eachkeylike__consumer-eachkeylike_provider.json",
  "eachkeylike__consumer",
  "eachkeylike_provider",
  1,
  PactSpecification::V4
)]
#[case::v3_message_pact(
  "test_consumer_v3-MessageProvider.json",
  "test_consumer_v3",
  "MessageProvider",
  3,
  PactSpecification::V3
)]
#[case::v3_xml_body(
  "XMLConsumer-XMLProvider.json",
  "XMLConsumer",
  "XMLProvider",
  1,
  PactSpecification::V3
)]
fn reads_real_world_pacts(
  #[case] file: &str,
  #[case] consumer: &str,
  #[case] provider: &str,
  #[case] interactions: usize,
  #[case] spec: PactSpecification,
) {
  let json = fixture(file);
  let pact = legacy_pact::read(file, &json).expect("pact_models should parse a real-world pact");
  assert_eq!(pact.consumer().name, consumer);
  assert_eq!(pact.provider().name, provider);
  assert_eq!(pact.interactions().len(), interactions);
  assert_eq!(pact.specification_version(), spec);
}

#[test]
fn a_document_that_cannot_be_a_pact_is_a_structured_error_not_a_panic() {
  let json = serde_json::json!(["not", "an", "object"]);
  let err = legacy_pact::read("inline", &json).expect_err("a JSON array is never a pact");
  assert_eq!(err.code(), "contract-invalid");
}

// --- the Pact -> LegacyRequest/LegacyResponse adapter (feeds plan task 3.5's compiler) ---

#[test]
fn http_interactions_extracts_every_request_response_pair() {
  let json = fixture("consumer-provider-v4.json");
  let pact = legacy_pact::read("consumer-provider-v4.json", &json).expect("reads clean");
  let interactions = legacy_pact::http_interactions(pact.as_ref());
  assert_eq!(interactions.len(), 2);
  assert_eq!(interactions[0].0, "first");

  let (_, request, response) = &interactions[0];
  let legacy_request =
    legacy_pact::legacy_request(request).expect("a plain GET / has a JSON-compatible body");
  assert_eq!(legacy_request.method, "GET");
  assert_eq!(legacy_request.path, "/");
  assert!(legacy_request.body.is_none(), "no body was ever mentioned");

  let legacy_response =
    legacy_pact::legacy_response(response).expect("a plain 200 has a JSON-compatible body");
  assert_eq!(legacy_response.status, 200);
}

#[test]
fn http_interactions_excludes_message_interactions() {
  let json = fixture("test_consumer_v3-MessageProvider.json");
  let pact = legacy_pact::read("test_consumer_v3-MessageProvider.json", &json).expect("reads clean");
  assert_eq!(pact.interactions().len(), 3, "the fixture is all messages");
  assert!(
    legacy_pact::http_interactions(pact.as_ref()).is_empty(),
    "as_v4_http() must return None for every one of them"
  );
}

#[test]
fn a_v4_bodys_content_contenttype_encoded_envelope_is_already_unwrapped() {
  let json = fixture("eachkeylike__consumer-eachkeylike_provider.json");
  let pact =
    legacy_pact::read("eachkeylike__consumer-eachkeylike_provider.json", &json).expect("reads clean");
  let interactions = legacy_pact::http_interactions(pact.as_ref());
  assert_eq!(interactions.len(), 1);
  let (_, request, _response) = &interactions[0];
  let legacy_request = legacy_pact::legacy_request(request)
    .expect("pact_models decodes the content/contentType/encoded envelope itself");
  assert_eq!(
    legacy_request.body,
    Some(serde_json::json!({ "a": { "prop1": { "value": "x" } } })),
    "the body must be the decoded content, not the {{content, contentType, encoded}} envelope itself"
  );
}

#[test]
fn a_non_json_body_is_a_named_error_not_a_panic() {
  let json = fixture("XMLConsumer-XMLProvider.json");
  let pact = legacy_pact::read("XMLConsumer-XMLProvider.json", &json).expect("reads clean");
  let interactions = legacy_pact::http_interactions(pact.as_ref());
  assert_eq!(interactions.len(), 1);
  let (_, request, response) = &interactions[0];
  // Whichever side actually carries the XML body, converting it must fail with a message naming
  // why — design 3.5 is JSON-only (its own module docs) — rather than panicking on the bytes.
  let request_err = legacy_pact::legacy_request(request);
  let response_err = legacy_pact::legacy_response(response);
  assert!(
    request_err.is_err() || response_err.is_err(),
    "expected the XML body to be rejected on at least one side"
  );
}
