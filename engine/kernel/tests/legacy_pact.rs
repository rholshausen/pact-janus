//! Plan task 3.1: v1–v4 pact reading via `pact_models`, exercised against real-world fixtures
//! (`tests/fixtures/legacy-pacts/README.md`) rather than only synthetic ones.

use pact_janus_kernel::legacy::compile::{LegacyBody, LegacyPart};
use pact_janus_kernel::legacy::pact as legacy_pact;
use pact_janus_kernel::plan::RuntimeValue;
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
    Some(document(
      serde_json::json!({ "a": { "prop1": { "value": "x" } } })
    )),
    "the body must be the decoded content, not the {{content, contentType, encoded}} envelope itself"
  );
}

/// pact-jvm's XML pact reads on both sides, its bodies as their elements (ADR 0039).
#[test]
fn an_xml_body_is_read_into_its_elements() {
  let json = fixture("XMLConsumer-XMLProvider.json");
  let pact = legacy_pact::read("XMLConsumer-XMLProvider.json", &json).expect("reads clean");
  let interactions = legacy_pact::http_interactions(pact.as_ref());
  assert_eq!(interactions.len(), 1);
  let interaction = &interactions[0];
  let request = legacy_pact::legacy_request(&interaction.request).expect("the request reads");
  assert!(
    matches!(request.body, Some(LegacyBody::Xml(_))),
    "{:?}",
    request.body
  );
  let response = legacy_pact::legacy_response(&interaction.response).expect("the response reads");
  assert_eq!(
    response.body,
    Some(LegacyBody::Xml(RuntimeValue::from_json(&json!({
      "Message": {
        "Head": [{ "Server": [{
          "Name": [{ "#text": "server_name" }],
          "Version": [{ "#text": "server_version" }],
          "Timestamp": [{ "#text": "server_timestamp" }]
        }] }],
        "Body": [{ "Result": [{ "@state": "SUCCESS" }] }]
      }
    }))))
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

/// A body as the legacy layer holds it: a document in the engine's model (staged plan S1.20).
fn document(value: serde_json::Value) -> LegacyBody {
  LegacyBody::Document(RuntimeValue::from_json(&value))
}

fn bytes(octets: Vec<u8>) -> LegacyBody {
  LegacyBody::Document(RuntimeValue::Bytes(octets))
}

fn body_of(response: serde_json::Value) -> Result<Option<LegacyBody>, String> {
  legacy_pact::legacy_response(&response_of(response))
    .map(|response| response.body)
    .map_err(|err| err.to_string())
}

#[rstest]
#[case::json(json!({ "status": 200, "headers": { "Content-Type": "application/json" }, "body": { "a": 1 } }),
  Some(document(json!({ "a": 1 }))))]
#[case::json_detected(json!({ "status": 200, "body": [1, 2] }), Some(document(json!([1, 2]))))]
#[case::text(json!({ "status": 200, "headers": { "Content-Type": "text/plain" }, "body": "Hello World!" }),
  Some(document(json!("Hello World!"))))]
#[case::text_detected(json!({ "status": 200, "body": "Hello World!" }), Some(document(json!("Hello World!"))))]
#[case::html(json!({ "status": 200, "headers": { "Content-Type": "text/html; charset=utf-8" }, "body": "<p>hi</p>" }),
  Some(document(json!("<p>hi</p>"))))]
#[case::binary(json!({ "status": 200, "headers": { "Content-Type": "image/png" }, "body": "iVBORw0KGgo=" }),
  Some(bytes(vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a])))]
#[case::octet_stream(json!({ "status": 200, "headers": { "Content-Type": "application/octet-stream" }, "body": "AAEC" }),
  Some(bytes(vec![0, 1, 2])))]
#[case::an_unknown_application_type_is_binary(
  json!({ "status": 200, "headers": { "Content-Type": "application/vnd.acme" }, "body": "AAEC" }),
  Some(bytes(vec![0, 1, 2])))]
#[case::form(
  json!({ "status": 200, "headers": { "Content-Type": "application/x-www-form-urlencoded" }, "body": "b=2&a=1&b=x+y" }),
  Some(LegacyBody::Form(RuntimeValue::from_json(&json!({ "b": ["2", "x y"], "a": ["1"] })))))]
#[case::xml(
  json!({ "status": 200, "headers": { "Content-Type": "application/xml" }, "body": "<a x=\"1\"><b>t</b><b/></a>" }),
  Some(LegacyBody::Xml(RuntimeValue::from_json(&json!({ "a": { "@x": "1", "b": [{ "#text": "t" }, {}] } })))))]
#[case::xml_suffix(
  json!({ "status": 200, "headers": { "Content-Type": "application/atom+xml" }, "body": "<feed/>" }),
  Some(LegacyBody::Xml(RuntimeValue::from_json(&json!({ "feed": {} })))))]
#[case::xml_detected(json!({ "status": 200, "body": "<?xml version=\"1.0\"?><a/>" }),
  Some(LegacyBody::Xml(RuntimeValue::from_json(&json!({ "a": {} })))))]
#[case::empty(json!({ "status": 200, "body": "" }), Some(LegacyBody::Empty))]
#[case::missing(json!({ "status": 200 }), None)]
fn a_body_is_read_as_todays_verifier_compares_its_content_type(
  #[case] response: serde_json::Value,
  #[case] expected: Option<LegacyBody>,
) {
  assert_eq!(body_of(response), Ok(expected));
}

/// A multipart body is its parts, each read by its own type (ADR 0040 decision 5): a field, a JSON file,
/// a part with no type as text, and a part that is itself multipart as octets.
#[test]
fn a_multipart_body_is_read_into_its_parts_each_by_its_own_type() {
  let body = "--x\r\nContent-Disposition: form-data; name=\"name\"\r\n\r\nMary\r\n\
--x\r\nContent-Disposition: form-data; name=\"order\"; filename=\"o.json\"\r\nContent-Type: application/json\r\n\r\n{\"id\": 1}\r\n\
--x\r\nX-Note: n\r\n\r\nplain\r\n\
--x\r\nContent-Type: multipart/mixed; boundary=y\r\n\r\n--y--\r\n--x--";
  let read = body_of(json!({
    "status": 200,
    "headers": { "Content-Type": "multipart/form-data; boundary=x" },
    "body": body
  }));
  let part = |name: &str,
              index: usize,
              filename: Option<&str>,
              field: bool,
              headers: &[(&str, &str)],
              content_type: Option<&str>,
              body: LegacyBody| LegacyPart {
    name: name.to_string(),
    index,
    filename: filename.map(str::to_string),
    field,
    headers: headers
      .iter()
      .map(|(n, v)| (n.to_string(), v.to_string()))
      .collect(),
    content_type: content_type.map(str::to_string),
    body,
  };
  assert_eq!(
    read,
    Ok(Some(LegacyBody::Multipart(vec![
      part(
        "name",
        0,
        None,
        true,
        &[("content-disposition", "form-data; name=\"name\"")],
        None,
        LegacyBody::Document(RuntimeValue::String("Mary".to_string()))
      ),
      part(
        "order",
        1,
        Some("o.json"),
        false,
        &[
          (
            "content-disposition",
            "form-data; name=\"order\"; filename=\"o.json\""
          ),
          ("content-type", "application/json"),
        ],
        Some("application/json"),
        LegacyBody::Document(RuntimeValue::from_json(&json!({ "id": 1 })))
      ),
      part(
        "",
        2,
        None,
        false,
        &[("x-note", "n")],
        None,
        LegacyBody::Document(RuntimeValue::String("plain".to_string()))
      ),
      part(
        "",
        3,
        None,
        false,
        &[("content-type", "multipart/mixed; boundary=y")],
        Some("multipart/mixed; boundary=y"),
        bytes(b"--y--".to_vec())
      ),
    ])))
  );
}

#[rstest]
#[case::no_boundary("multipart/form-data", "--x--", "names no boundary")]
#[case::a_part_that_is_not_its_type(
  "multipart/form-data; boundary=x",
  "--x\r\nContent-Disposition: form-data; name=\"a\"\r\nContent-Type: application/json\r\n\r\n{\r\n--x--",
  "part 'a' is not valid JSON"
)]
fn a_multipart_body_that_does_not_split_is_malformed(
  #[case] content_type: &str,
  #[case] body: &str,
  #[case] message: &str,
) {
  let error = body_of(json!({ "status": 200, "headers": { "Content-Type": content_type }, "body": body }))
    .expect_err("malformed");
  assert!(
    error.contains("is not a valid multipart body") && error.contains(message),
    "{error}"
  );
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
#[case::multipart_under_its_own_label(
  json!({ "status": 200, "headers": { "Content-Type": "multipart/mixed; boundary=x" }, "body": "--x--" }),
  None
)]
fn the_reply_body_is_decoded_as_the_pacts_content_type(
  #[case] response: serde_json::Value,
  #[case] expected: Option<&str>,
) {
  assert_eq!(response_types_of(response), expected.map(str::to_string));
}

/// A body that claims to be XML and is not is a problem with the document, named as one.
#[test]
fn an_xml_body_that_is_not_xml_is_malformed() {
  let error =
    body_of(json!({ "status": 200, "headers": { "Content-Type": "application/xml" }, "body": "<a>" }))
      .expect_err("not XML");
  assert!(error.contains("body is not valid XML"), "{error}");
}

/// Staged plan S1.8: which interactions need a pact plugin. One names it in its
/// `pluginConfiguration`; one using a matcher `pact_models` does not know needs the plugins the pact
/// names; and a rule like that in a pact that names no plugin is nobody's, so the interaction is
/// HTTP and its rule compiles to an error node, as an unknown rule always has.
#[rstest]
#[case::configuration("v4-plugins-mixed.json", vec![vec![], vec!["csv"], vec!["protobuf"], vec!["avro"]])]
#[case::matcher_only("v4-plugin-matcher-only.json", vec![vec!["creditcard"], vec![]])]
fn an_interaction_needs_the_plugins_it_names_or_whose_rules_it_uses(
  #[case] name: &str,
  #[case] expected: Vec<Vec<&str>>,
) {
  let pact = legacy_pact::read(name, &fixture(name)).expect("the fixture reads");
  let needed: Vec<Vec<String>> = legacy_pact::interactions(pact.as_ref())
    .into_iter()
    .map(|interaction| match interaction {
      legacy_pact::PactInteraction::Http(_) | legacy_pact::PactInteraction::Message(_) => vec![],
      legacy_pact::PactInteraction::NeedsPlugins(needs) => needs.plugins,
    })
    .collect();
  assert_eq!(needed, expected);
}

#[test]
fn a_rule_no_named_plugin_provides_is_not_a_plugins() {
  let mut document = fixture("v4-plugin-matcher-only.json");
  document["metadata"].as_object_mut().unwrap().remove("plugins");
  let pact = legacy_pact::read("test", &document).expect("the pact reads");
  let interactions = legacy_pact::interactions(pact.as_ref());
  assert_eq!(interactions.len(), 2);
  assert!(
    interactions
      .iter()
      .all(|interaction| matches!(interaction, legacy_pact::PactInteraction::Http(_)))
  );
}
