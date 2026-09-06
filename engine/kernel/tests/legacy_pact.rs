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
