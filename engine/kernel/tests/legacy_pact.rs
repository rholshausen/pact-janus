//! Plan task 3.1: v1–v4 pact reading via `pact_models`, exercised against real-world fixtures
//! (`tests/fixtures/legacy-pacts/README.md`) rather than only synthetic ones.

use pact_janus_kernel::legacy::compile::LegacyBody;
use pact_janus_kernel::legacy::pact as legacy_pact;
use pact_models::PactSpecification;
use pretty_assertions::assert_eq;
use rstest::rstest;
use serde_json::json;
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
  assert_eq!(interactions[0].description, "first");

  let interaction = &interactions[0];
  let legacy_request =
    legacy_pact::legacy_request(&interaction.request).expect("a plain GET / has a JSON-compatible body");
  assert_eq!(legacy_request.method, "GET");
  assert_eq!(legacy_request.path, "/");
  assert!(legacy_request.body.is_none(), "no body was ever mentioned");

  let legacy_response =
    legacy_pact::legacy_response(&interaction.response).expect("a plain 200 has a JSON-compatible body");
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
  let legacy_request = legacy_pact::legacy_request(&interactions[0].request)
    .expect("pact_models decodes the content/contentType/encoded envelope itself");
  assert_eq!(
    legacy_request.body,
    Some(LegacyBody::Document(
      serde_json::json!({ "a": { "prop1": { "value": "x" } } })
    )),
    "the body must be the decoded content, not the {{content, contentType, encoded}} envelope itself"
  );
}

#[test]
fn a_non_json_body_is_a_named_error_not_a_panic() {
  let json = fixture("XMLConsumer-XMLProvider.json");
  let pact = legacy_pact::read("XMLConsumer-XMLProvider.json", &json).expect("reads clean");
  let interactions = legacy_pact::http_interactions(pact.as_ref());
  assert_eq!(interactions.len(), 1);
  let interaction = &interactions[0];
  // Whichever side actually carries the XML body, converting it must fail with a message naming
  // why — XML is S1.22's — rather than panicking on the bytes.
  let request_err = legacy_pact::legacy_request(&interaction.request);
  let response_err = legacy_pact::legacy_response(&interaction.response);
  assert!(
    request_err.is_err() || response_err.is_err(),
    "expected the XML body to be rejected on at least one side"
  );
}

/// The response of a one-interaction v3 pact.
fn response_of(response: serde_json::Value) -> pact_models::v4::http_parts::HttpResponse {
  let pact = json!({
    "consumer": { "name": "c" },
    "provider": { "name": "p" },
    "interactions": [{ "description": "i", "request": { "method": "GET", "path": "/" }, "response": response }],
    "metadata": { "pactSpecification": { "version": "3.0.0" } }
  });
  let pact = legacy_pact::read("inline", &pact).expect("reads clean");
  legacy_pact::http_interactions(pact.as_ref()).remove(0).response
}

fn body_of(response: serde_json::Value) -> Result<Option<LegacyBody>, String> {
  legacy_pact::legacy_response(&response_of(response))
    .map(|response| response.body)
    .map_err(|err| err.to_string())
}

#[rstest]
#[case::json(json!({ "status": 200, "headers": { "Content-Type": "application/json" }, "body": { "a": 1 } }),
  Some(LegacyBody::Document(json!({ "a": 1 }))))]
#[case::json_detected(json!({ "status": 200, "body": [1, 2] }), Some(LegacyBody::Document(json!([1, 2]))))]
#[case::text(json!({ "status": 200, "headers": { "Content-Type": "text/plain" }, "body": "Hello World!" }),
  Some(LegacyBody::Document(json!("Hello World!"))))]
#[case::text_detected(json!({ "status": 200, "body": "Hello World!" }), Some(LegacyBody::Document(json!("Hello World!"))))]
#[case::html(json!({ "status": 200, "headers": { "Content-Type": "text/html; charset=utf-8" }, "body": "<p>hi</p>" }),
  Some(LegacyBody::Document(json!("<p>hi</p>"))))]
#[case::binary(json!({ "status": 200, "headers": { "Content-Type": "image/png" }, "body": "iVBORw0KGgo=" }),
  Some(LegacyBody::Bytes(vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a])))]
#[case::octet_stream(json!({ "status": 200, "headers": { "Content-Type": "application/octet-stream" }, "body": "AAEC" }),
  Some(LegacyBody::Bytes(vec![0, 1, 2])))]
#[case::an_unknown_application_type_is_binary(
  json!({ "status": 200, "headers": { "Content-Type": "application/vnd.acme" }, "body": "AAEC" }),
  Some(LegacyBody::Bytes(vec![0, 1, 2])))]
#[case::empty(json!({ "status": 200, "body": "" }), Some(LegacyBody::Empty))]
#[case::missing(json!({ "status": 200 }), None)]
fn a_body_is_read_as_todays_verifier_compares_its_content_type(
  #[case] response: serde_json::Value,
  #[case] expected: Option<LegacyBody>,
) {
  assert_eq!(body_of(response), Ok(expected));
}

#[rstest]
#[case::xml("application/xml", "<a/>")]
#[case::form("application/x-www-form-urlencoded", "a=1")]
#[case::multipart("multipart/form-data; boundary=x", "--x--")]
fn a_body_kind_staged_for_later_is_refused_by_its_content_type(
  #[case] content_type: &str,
  #[case] body: &str,
) {
  let error = body_of(json!({ "status": 200, "headers": { "Content-Type": content_type }, "body": body }))
    .expect_err("not compiled yet");
  assert!(error.contains("not compiled yet"), "{error}");
}

fn response_types_of(response: serde_json::Value) -> Option<String> {
  let response = legacy_pact::legacy_response(&response_of(response)).expect("reads");
  legacy_pact::response_types(&response)
    .get("response")
    .and_then(|part| part.get("body"))
    .cloned()
}

#[rstest]
#[case::the_header(json!({ "status": 200, "headers": { "Content-Type": "text/plain" }, "body": "abc" }), Some("text/plain"))]
#[case::detected(json!({ "status": 200, "body": { "a": 1 } }), Some("application/json"))]
#[case::binary(json!({ "status": 200, "headers": { "Content-Type": "image/png" }, "body": "iVBORw0KGgo=" }), Some("image/png"))]
#[case::no_body(json!({ "status": 200 }), None)]
fn the_reply_body_is_decoded_as_the_pacts_content_type(
  #[case] response: serde_json::Value,
  #[case] expected: Option<&str>,
) {
  assert_eq!(response_types_of(response), expected.map(str::to_string));
}
