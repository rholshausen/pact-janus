//! The client against a stub broker on a real socket: the requests it makes, in order, and what it
//! makes of the answers. The client is tested against a real broker by the differential harness
//! (`tools/verifier-diff`'s broker cases); these pin its own behaviour and its errors.

use super::*;
use pact_janus_kernel::broker::{ContractToPublish, InteractionResult};
use pretty_assertions::assert_eq;
use rstest::rstest;
use std::sync::{Arc, Mutex};

/// One request the stub saw: method, path and JSON body.
type Seen = Arc<Mutex<Vec<(String, String, Value)>>>;

/// A broker that answers by path, with links back to itself, and records every request. `status`
/// is what every request is answered with when it is not 200.
fn stub(status: u16) -> (String, Seen) {
  let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
  let base = format!("http://{}", server.server_addr().to_ip().unwrap());
  let seen: Seen = Arc::default();
  let record = Arc::clone(&seen);
  let me = base.clone();
  std::thread::spawn(move || {
    for mut request in server.incoming_requests() {
      let mut body = String::new();
      let _ = request.as_reader().read_to_string(&mut body);
      let path = request.url().to_string();
      record.lock().unwrap().push((
        request.method().as_str().to_string(),
        path.clone(),
        serde_json::from_str(&body).unwrap_or(Value::Null),
      ));
      let answer = match (status, path.as_str()) {
        (200, "/") => json!({ "_links": {
          "pb:provider-pacts-for-verification": { "href": format!("{me}/pacts/provider/{{provider}}/for-verification"), "templated": true }
        } }),
        (200, "/pacts/provider/order%20api/for-verification") => json!({ "_embedded": { "pacts": [{
          "shortDescription": "latest from main",
          "verificationProperties": { "pending": true, "wip": false, "notices": [{ "when": "before_verification", "text": "it is pending" }] },
          "_links": { "self": { "href": format!("{me}/pacts/1") } }
        }] } }),
        (200, "/pacts/1") => json!({
          "consumer": { "name": "web" }, "provider": { "name": "order api" }, "interactions": [],
          "_links": {
            "pb:provider": { "href": format!("{me}/pacticipants/order%20api") },
            "pb:publish-verification-results": { "href": format!("{me}/pacts/1/verification-results") }
          }
        }),
        (200, "/pacticipants/order%20api") => json!({ "_links": {
          "pb:branch-version": { "href": format!("{me}/pacticipants/order%20api/branches/{{branch}}/versions/{{version}}") },
          "pb:version-tag": { "href": format!("{me}/pacticipants/order%20api/versions/{{version}}/tags/{{tag}}") }
        } }),
        (200, _) => json!({}),
        (_, _) => json!({ "errors": ["no, with s3cret in it"] }),
      };
      let _ = request.respond(
        tiny_http::Response::from_string(answer.to_string())
          .with_status_code(status)
          .with_header(tiny_http::Header::from_bytes("Content-Type", "application/hal+json").unwrap()),
      );
    }
  });
  (base, seen)
}

fn descriptor(url: &str) -> BrokerDescriptor {
  serde_json::from_value(json!({ "url": url, "auth": { "kind": "bearer", "token": "s3cret" } })).unwrap()
}

/// One `GET` through the client a descriptor's options describe, retries and all.
fn get(broker: &BrokerDescriptor, url: &str) -> Result<Value, ComponentError> {
  HalBrokerClient::new().send(&connect(broker)?, broker, "GET", url, None)
}

fn fetch_request(url: &str) -> FetchRequest {
  FetchRequest {
    broker: descriptor(url),
    provider: "order api".to_string(),
    selectors: vec![json!({ "mainBranch": true })],
    provider_branch: Some("main".to_string()),
    provider_tags: vec![],
    include_pending: true,
    include_wip_since: Some("2026-01-01".to_string()),
  }
}

#[test]
fn a_fetch_asks_as_todays_verifier_does_and_reads_pending_and_notices() {
  let (url, seen) = stub(200);
  let fetched = HalBrokerClient::new()
    .fetch_for_verification(&fetch_request(&url))
    .unwrap();

  assert_eq!(fetched.len(), 1);
  assert!(fetched[0].pending && !fetched[0].wip);
  assert_eq!(fetched[0].notices, vec!["it is pending".to_string()]);
  assert_eq!(fetched[0].document["consumer"]["name"], "web");
  assert!(
    fetched[0].handle["pb:publish-verification-results"].is_object(),
    "the handle is the pact's links"
  );

  let seen = seen.lock().unwrap();
  let requests: Vec<(&str, &str)> = seen.iter().map(|(m, p, _)| (m.as_str(), p.as_str())).collect();
  assert_eq!(
    requests,
    vec![
      ("GET", "/"),
      ("POST", "/pacts/provider/order%20api/for-verification"),
      ("GET", "/pacts/1"),
    ]
  );
  assert_eq!(
    seen[1].2,
    json!({
      "consumerVersionSelectors": [{ "mainBranch": true }],
      "includePendingStatus": true,
      "providerVersionBranch": "main",
      "includeWipPactsSince": "2026-01-01",
    })
  );
}

#[test]
fn a_publish_records_the_branch_and_tags_then_posts_the_result_keyed_by_interaction_id() {
  let (url, seen) = stub(200);
  let client = HalBrokerClient::new();
  let fetched = client.fetch_for_verification(&fetch_request(&url)).unwrap();
  let broker = descriptor(&url);
  let tags = vec!["prod".to_string()];
  client
    .publish_results(&PublishRequest {
      broker: &broker,
      handle: &fetched[0].handle,
      provider_version: "2.0.0",
      branch: Some("main"),
      tags: &tags,
      build_url: Some("https://ci.example/1"),
      success: false,
      results: vec![
        InteractionResult {
          id: Some("i-1".to_string()),
          description: "one".to_string(),
          success: true,
          mismatches: vec![],
          error: None,
        },
        InteractionResult {
          id: Some("i-2".to_string()),
          description: "two".to_string(),
          success: false,
          mismatches: vec![json!({ "path": "$.response.body.colour", "message": "missing" })],
          error: None,
        },
      ],
    })
    .unwrap();

  let seen = seen.lock().unwrap();
  let after_fetch: Vec<(&str, &str)> = seen[3..]
    .iter()
    .map(|(m, p, _)| (m.as_str(), p.as_str()))
    .collect();
  assert_eq!(
    after_fetch,
    vec![
      ("GET", "/pacticipants/order%20api"),
      ("PUT", "/pacticipants/order%20api/branches/main/versions/2.0.0"),
      ("PUT", "/pacticipants/order%20api/versions/2.0.0/tags/prod"),
      ("POST", "/pacts/1/verification-results"),
    ]
  );
  let result = &seen.last().unwrap().2;
  assert_eq!(result["success"], false);
  assert_eq!(result["providerApplicationVersion"], "2.0.0");
  assert_eq!(result["buildUrl"], "https://ci.example/1");
  assert_eq!(
    result["testResults"],
    json!([
      { "interactionId": "i-1", "interactionDescription": "one", "success": true },
      { "interactionId": "i-2", "interactionDescription": "two", "success": false,
        "mismatches": [{ "attribute": "body", "identifier": "$.colour", "description": "missing" }] }
    ])
  );
}

#[test]
fn a_broker_that_says_no_is_refused_and_the_error_carries_no_credential() {
  let (url, _) = stub(401);
  let err = HalBrokerClient::new()
    .fetch_for_verification(&fetch_request(&url))
    .unwrap_err();
  assert_eq!(err.code, "broker-refused");
  assert_eq!(err.details.as_ref().unwrap()["status"], 401);
  assert!(!err.message.contains("s3cret"), "{}", err.message);
}

#[test]
fn a_broker_nobody_answers_for_is_unreachable() {
  let err = HalBrokerClient::new()
    .fetch_for_verification(&fetch_request("http://127.0.0.1:1"))
    .unwrap_err();
  assert_eq!(err.code, "broker-unreachable");
}

#[rstest]
#[case(json!({ "path": "$.response.status", "message": "m" }), json!({ "attribute": "status", "description": "m" }))]
#[case(json!({ "path": "$.response.headers.Content-Type", "message": "m" }), json!({ "attribute": "header", "identifier": "Content-Type", "description": "m" }))]
#[case(json!({ "path": "$.response.headers['x-a.b']", "message": "m" }), json!({ "attribute": "header", "identifier": "x-a.b", "description": "m" }))]
#[case(json!({ "path": "$.response.body", "message": "m" }), json!({ "attribute": "body", "identifier": "$", "description": "m" }))]
#[case(json!({ "path": "$.response.body.items[0].id", "message": "m" }), json!({ "attribute": "body", "identifier": "$.items[0].id", "description": "m" }))]
fn a_mismatch_is_put_in_the_brokers_terms(#[case] mismatch: Value, #[case] expected: Value) {
  assert_eq!(broker_mismatch(&mismatch), expected);
}

#[test]
fn basic_credentials_are_encoded_and_bearer_ones_passed_through() {
  let basic: BrokerDescriptor = serde_json::from_value(json!({
    "url": "http://b", "auth": { "kind": "basic", "username": "u", "password": "p" }
  }))
  .unwrap();
  assert_eq!(authorization(&basic).as_deref(), Some("Basic dTpw"));
  assert_eq!(
    authorization(&descriptor("http://b")).as_deref(),
    Some("Bearer s3cret")
  );
}

// ---------------------------------------------------------------------------------------------
// Publishing contracts
// ---------------------------------------------------------------------------------------------

/// How the publishing stub behaves.
#[derive(Clone, Default)]
struct Publishing {
  /// The index's `Accept-Encoding`, when it sends one (RFC 7694).
  accept_encoding: Option<&'static str>,
  /// Answer a gzipped publication with 415, as a broker that cannot read one should.
  refuse_gzip: bool,
  /// The index has no `pb:publish-contracts` link, as before Pact Broker 2.86.0.
  no_link: bool,
  /// What the publication is answered with, when not 200 and the notices below.
  refusal: Option<(u16, Value)>,
}

/// One request the publishing stub saw: method, path, its `Content-Encoding`, and its body,
/// decompressed.
type Recorded = Arc<Mutex<Vec<(String, String, Option<String>, Value)>>>;

fn publishing_stub(behaviour: Publishing) -> (String, Recorded) {
  let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
  let base = format!("http://{}", server.server_addr().to_ip().unwrap());
  let seen: Recorded = Arc::default();
  let record = Arc::clone(&seen);
  let me = base.clone();
  std::thread::spawn(move || {
    for mut request in server.incoming_requests() {
      let mut bytes = Vec::new();
      let _ = request.as_reader().read_to_end(&mut bytes);
      let encoding = request
        .headers()
        .iter()
        .find(|header| header.field.equiv("content-encoding"))
        .map(|header| header.value.as_str().to_string());
      let body = match encoding.as_deref() {
        Some("gzip") => {
          let mut text = String::new();
          let _ =
            std::io::Read::read_to_string(&mut flate2::read::GzDecoder::new(bytes.as_slice()), &mut text);
          text
        }
        _ => String::from_utf8_lossy(&bytes).to_string(),
      };
      let path = request.url().to_string();
      record.lock().unwrap().push((
        request.method().as_str().to_string(),
        path.clone(),
        encoding.clone(),
        serde_json::from_str(&body).unwrap_or(Value::Null),
      ));
      let (status, answer) = match path.as_str() {
        "/" if behaviour.no_link => (200, json!({ "_links": {} })),
        "/" => (
          200,
          json!({ "_links": { "pb:publish-contracts": { "href": format!("{me}/contracts/publish") } } }),
        ),
        _ if behaviour.refuse_gzip && encoding.as_deref() == Some("gzip") => (415, json!({})),
        _ => behaviour.refusal.clone().unwrap_or((
          200,
          json!({ "notices": [
            { "type": "success", "text": "Pact successfully published for web version 1.0.0 and provider order api." },
            { "type": "prompt", "text": "  Next steps:" }
          ] }),
        )),
      };
      let mut response = tiny_http::Response::from_string(answer.to_string())
        .with_status_code(status)
        .with_header(tiny_http::Header::from_bytes("Content-Type", "application/hal+json").unwrap());
      if let (Some(accept), "/") = (behaviour.accept_encoding, path.as_str()) {
        response = response.with_header(tiny_http::Header::from_bytes("Accept-Encoding", accept).unwrap());
      }
      let _ = request.respond(response);
    }
  });
  (base, seen)
}

fn contracts() -> Vec<ContractToPublish> {
  vec![
    ContractToPublish {
      provider: "order api".to_string(),
      content: br#"{"consumer":{"name":"web"},"interactions":[],"provider":{"name":"order api"}}"#.to_vec(),
    },
    ContractToPublish {
      provider: "stock api".to_string(),
      content: br#"{"$format":"janus-contract/1","consumer":{"name":"web"},"interactions":[],"provider":{"name":"stock api"}}"#.to_vec(),
    },
  ]
}

fn publish(url: &str) -> Result<ContractsPublished, ComponentError> {
  let broker = descriptor(url);
  let tags = vec!["v1".to_string()];
  HalBrokerClient::new().publish_contracts(&ContractsToPublish {
    broker: &broker,
    consumer: "web",
    consumer_version: "1.0.0",
    branch: Some("main"),
    tags: &tags,
    build_url: Some("https://ci.example/7"),
    contracts: &contracts(),
  })
}

#[test]
fn a_publication_posts_every_contract_as_a_pact_in_one_request_as_todays_clients_do() {
  let (url, seen) = publishing_stub(Publishing::default());
  let published = publish(&url).unwrap();
  assert_eq!(
    published.notices,
    vec![
      "Pact successfully published for web version 1.0.0 and provider order api.".to_string(),
      "  Next steps:".to_string(),
    ]
  );

  let seen = seen.lock().unwrap();
  let requests: Vec<(&str, &str)> = seen.iter().map(|(m, p, _, _)| (m.as_str(), p.as_str())).collect();
  assert_eq!(requests, vec![("GET", "/"), ("POST", "/contracts/publish")]);
  assert_eq!(seen[1].2, None, "nothing said the broker reads a compressed body");
  let encoded: Vec<String> = contracts()
    .iter()
    .map(|contract| BASE64.encode(&contract.content))
    .collect();
  assert_eq!(
    seen[1].3,
    json!({
      "pacticipantName": "web",
      "pacticipantVersionNumber": "1.0.0",
      "branch": "main",
      "tags": ["v1"],
      "buildUrl": "https://ci.example/7",
      "contracts": [
        { "consumerName": "web", "providerName": "order api", "specification": "pact",
          "contentType": "application/json", "content": encoded[0] },
        { "consumerName": "web", "providerName": "stock api", "specification": "pact",
          "contentType": "application/json", "content": encoded[1] },
      ],
    })
  );
}

#[test]
fn a_broker_that_says_it_reads_gzip_is_sent_the_publication_compressed() {
  let (url, seen) = publishing_stub(Publishing {
    accept_encoding: Some("gzip"),
    ..Publishing::default()
  });
  publish(&url).unwrap();
  let seen = seen.lock().unwrap();
  assert_eq!(seen[1].2.as_deref(), Some("gzip"));
  assert_eq!(
    seen[1].3["pacticipantName"], "web",
    "the body decompresses to the same request"
  );
}

#[test]
fn a_compressed_publication_refused_as_unsupported_is_sent_again_uncompressed() {
  let (url, seen) = publishing_stub(Publishing {
    accept_encoding: Some("gzip"),
    refuse_gzip: true,
    ..Publishing::default()
  });
  publish(&url).unwrap();
  let seen = seen.lock().unwrap();
  let encodings: Vec<Option<&str>> = seen[1..].iter().map(|(_, _, e, _)| e.as_deref()).collect();
  assert_eq!(encodings, vec![Some("gzip"), None]);
}

#[test]
fn a_broker_older_than_the_publish_contracts_api_is_named_as_such() {
  let (url, _) = publishing_stub(Publishing {
    no_link: true,
    ..Publishing::default()
  });
  let err = publish(&url).unwrap_err();
  assert_eq!(err.code, "broker-response-invalid");
  assert!(err.message.contains("2.86.0"), "{}", err.message);
}

#[test]
fn a_publication_the_broker_will_not_take_is_refused_in_its_own_words() {
  let (url, _) = publishing_stub(Publishing {
    refusal: Some((
      409,
      json!({
        "notices": [{ "type": "error", "text": "Cannot change the content of the pact for web version 1.0.0" }],
        "errors": { "contracts": ["Cannot change the content of the pact for web version 1.0.0"] }
      }),
    )),
    ..Publishing::default()
  });
  let err = publish(&url).unwrap_err();
  assert_eq!(err.code, "broker-refused");
  assert_eq!(err.details.as_ref().unwrap()["status"], 409);
  assert!(
    err
      .message
      .ends_with("with 409: contracts: Cannot change the content of the pact for web version 1.0.0"),
    "{}",
    err.message
  );
}

#[rstest]
#[case("gzip", true)]
#[case("identity, GZIP", true)]
#[case("gzip;q=0.5, identity", true)]
#[case("x-gzip", true)]
#[case("gzip;q=0", false)]
#[case("identity", false)]
#[case("", false)]
fn accept_encoding_names_gzip_only_with_a_weight_above_zero(#[case] value: &str, #[case] names: bool) {
  assert_eq!(names_gzip(value), names);
}

// ---------------------------------------------------------------------------------------------
// Retrying
// ---------------------------------------------------------------------------------------------

/// A broker that gives `answers` in turn (a status, and a `Retry-After` when it sends one), then 200
/// `{}`, and counts what it was asked.
fn answering(answers: Vec<(u16, Option<&'static str>)>) -> (String, Arc<Mutex<usize>>) {
  let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
  let base = format!("http://{}", server.server_addr().to_ip().unwrap());
  let asked = Arc::new(Mutex::new(0));
  let count = Arc::clone(&asked);
  std::thread::spawn(move || {
    let mut answers = answers.into_iter();
    for request in server.incoming_requests() {
      *count.lock().unwrap() += 1;
      let (status, retry_after) = answers.next().unwrap_or((200, None));
      let body = if status == 200 {
        "{}".to_string()
      } else {
        json!({ "error": format!("answered {status}") }).to_string()
      };
      let mut response = tiny_http::Response::from_string(body).with_status_code(status);
      if let Some(retry_after) = retry_after {
        response = response.with_header(tiny_http::Header::from_bytes("Retry-After", retry_after).unwrap());
      }
      let _ = request.respond(response);
    }
  });
  (base, asked)
}

#[test]
fn a_429_with_no_retry_after_is_a_limit_and_is_not_retried() {
  let (url, asked) = answering(vec![(429, None)]);
  let err = get(&descriptor(&url), &url).unwrap_err();
  assert_eq!(err.code, "broker-refused");
  assert_eq!(err.details.as_ref().unwrap()["status"], 429);
  assert!(err.message.ends_with("with 429: answered 429"), "{}", err.message);
  assert_eq!(*asked.lock().unwrap(), 1);
}

#[test]
fn a_429_that_says_when_to_retry_is_tried_again_then() {
  let (url, asked) = answering(vec![(429, Some("0"))]);
  get(&descriptor(&url), &url).unwrap();
  assert_eq!(*asked.lock().unwrap(), 2);
}

#[test]
fn a_retry_after_longer_than_the_client_waits_is_reported_at_once() {
  let (url, asked) = answering(vec![(503, Some("120"))]);
  let err = get(&descriptor(&url), &url).unwrap_err();
  assert_eq!(err.code, "broker-refused");
  assert_eq!(err.details.as_ref().unwrap()["retry-after"], 120);
  assert!(err.message.contains("tried again in 120s"), "{}", err.message);
  assert_eq!(*asked.lock().unwrap(), 1);
}

#[test]
fn a_5xx_is_retried_until_the_attempts_run_out() {
  let (url, asked) = answering(vec![
    (502, None),
    (503, Some("Wed, 21 Oct 2026 07:28:00 GMT")),
    (500, None),
  ]);
  let err = get(&descriptor(&url), &url).unwrap_err();
  assert_eq!(err.details.as_ref().unwrap()["status"], 500);
  assert_eq!(*asked.lock().unwrap(), 3);
}

// ---------------------------------------------------------------------------------------------
// Links
// ---------------------------------------------------------------------------------------------

/// pact_verifier's own cases (`resolve_path_test`, `normalize_path_inner`'s, and the context-path
/// fetches), as today's verifier resolves them.
#[rstest]
#[case("http://b:1234", "http://b:1234/pacts/1", "http://b:1234/pacts/1")]
#[case("http://b:1234", "http://elsewhere.internal/pacts/1", "http://b:1234/pacts/1")]
#[case("http://b:1234", "/pacts/1", "http://b:1234/pacts/1")]
#[case("http://b:1234", "pacts/1", "http://b:1234/pacts/1")]
#[case("http://b:1234/", "pacts/1", "http://b:1234/pacts/1")]
#[case("http://b/broker", "http://b/broker/pacts/1", "http://b/broker/pacts/1")]
#[case(
  "http://b/broker",
  "http://elsewhere.internal/pacts/provider/Example%20API/for-verification",
  "http://b/broker/pacts/provider/Example%20API/for-verification"
)]
#[case(
  "http://b/broker",
  "http://elsewhere.internal/broker/pacts/1",
  "http://b/broker/pacts/1"
)]
#[case("http://b/broker/", "/pacts/1", "http://b/broker/pacts/1")]
#[case("http://b/broker", "/brokerage/pacts/1", "http://b/broker/brokerage/pacts/1")]
#[case("http://b/broker", "pacts/1", "http://b/broker/pacts/1")]
#[case(
  "http://b/broker",
  "http://elsewhere.internal/broker/pacts?page=2&size=10",
  "http://b/broker/pacts?page=2&size=10"
)]
#[case("http://b/broker", "http://elsewhere.internal", "http://b/broker/")]
fn a_link_is_fetched_from_the_broker_the_user_named(
  #[case] broker: &str,
  #[case] link: &str,
  #[case] fetched: &str,
) {
  assert_eq!(absolute(&descriptor(broker), link), fetched);
}

// ---------------------------------------------------------------------------------------------
// What a real broker cannot be asked to do (staged plan S1.3, layer 2's hand-written cases)
// ---------------------------------------------------------------------------------------------

/// A broker that answers each path in `routes` with its status, content type and body, in which
/// `{me}` is the stub's own URL, and 404 to anything else; it records every request's method and
/// path, query included.
fn routes(routes: Vec<(&'static str, u16, &'static str, &'static str)>) -> (String, Arc<Mutex<Vec<String>>>) {
  let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
  let base = format!("http://{}", server.server_addr().to_ip().unwrap());
  let seen = Arc::new(Mutex::new(Vec::new()));
  let record = Arc::clone(&seen);
  let me = base.clone();
  std::thread::spawn(move || {
    for request in server.incoming_requests() {
      let path = request.url().to_string();
      record
        .lock()
        .unwrap()
        .push(format!("{} {path}", request.method().as_str()));
      let (status, content_type, body) = routes
        .iter()
        .find(|(route, ..)| *route == path)
        .map(|(_, status, content_type, body)| (*status, *content_type, body.replace("{me}", &me)))
        .unwrap_or((404, "application/json", "{}".to_string()));
      let _ = request.respond(
        tiny_http::Response::from_string(body)
          .with_status_code(status)
          .with_header(tiny_http::Header::from_bytes("Content-Type", content_type).unwrap()),
      );
    }
  });
  (base, seen)
}

const INDEX: &str = r#"{ "_links": { "pb:provider-pacts-for-verification": { "href": "{me}/pacts/provider/{provider}/for-verification", "templated": true } } }"#;

fn fetch_from(url: &str) -> Result<Vec<FetchedPact>, ComponentError> {
  HalBrokerClient::new().fetch_for_verification(&fetch_request(url))
}

#[test]
fn a_body_that_is_not_json_is_an_answer_the_client_cannot_read() {
  let (url, _) = routes(vec![("/", 200, "text/html", "<html>This is not JSON</html>")]);
  let err = fetch_from(&url).unwrap_err();
  assert_eq!(err.code, "broker-response-invalid");
  assert!(err.message.contains("is not JSON"), "{}", err.message);
}

#[test]
fn a_broker_without_the_pacts_for_verification_api_is_named_as_too_old() {
  let (url, _) = routes(vec![("/", 200, "application/hal+json", r#"{ "_links": {} }"#)]);
  let err = fetch_from(&url).unwrap_err();
  assert_eq!(err.code, "broker-response-invalid");
  assert!(
    err.message.contains("pb:provider-pacts-for-verification"),
    "{}",
    err.message
  );
}

#[test]
fn a_selection_that_is_not_hal_is_an_answer_the_client_cannot_read() {
  let (url, _) = routes(vec![
    ("/", 200, "application/hal+json", INDEX),
    (
      "/pacts/provider/order%20api/for-verification",
      200,
      "application/hal+json",
      r#"{ "_links": { "self": { "href": "{me}/pacts/provider/order%20api/for-verification" } } }"#,
    ),
  ]);
  let err = fetch_from(&url).unwrap_err();
  assert_eq!(err.code, "broker-response-invalid");
  assert!(err.message.contains("'_embedded.pacts'"), "{}", err.message);
}

#[test]
fn selectors_the_broker_will_not_take_are_refused_in_its_own_words() {
  let (url, _) = routes(vec![
    ("/", 200, "application/hal+json", INDEX),
    (
      "/pacts/provider/order%20api/for-verification",
      400,
      "application/json",
      r#"{ "errors": { "consumerVersionSelectors.0": ["mainBranch must be true"] } }"#,
    ),
  ]);
  let err = fetch_from(&url).unwrap_err();
  assert_eq!(err.code, "broker-refused");
  assert_eq!(err.details.as_ref().unwrap()["status"], 400);
  assert!(
    err
      .message
      .ends_with("with 400: consumerVersionSelectors.0: mainBranch must be true"),
    "{}",
    err.message
  );
}

#[test]
fn a_selection_of_nothing_is_no_pacts_not_an_error() {
  let (url, _) = routes(vec![
    ("/", 200, "application/hal+json", INDEX),
    (
      "/pacts/provider/order%20api/for-verification",
      200,
      "application/hal+json",
      r#"{ "_embedded": { "pacts": [] } }"#,
    ),
  ]);
  assert!(fetch_from(&url).unwrap().is_empty());
}

/// A broker at a context path, behind a proxy that does not tell it so: its links name a host the
/// client cannot reach and leave the context path out. Today's verifier fetches each from the
/// broker it was given, and so does this client.
#[test]
fn a_broker_at_a_context_path_is_reached_through_it_whatever_host_its_links_name() {
  let (base, seen) = routes(vec![
    (
      "/broker",
      200,
      "application/hal+json",
      r#"{ "_links": { "pb:provider-pacts-for-verification": { "href": "http://broker.internal:9292/pacts/provider/{provider}/for-verification", "templated": true } } }"#,
    ),
    (
      "/broker/pacts/provider/order%20api/for-verification",
      200,
      "application/hal+json",
      r#"{ "_embedded": { "pacts": [{ "verificationProperties": {}, "_links": { "self": { "href": "http://broker.internal:9292/pacts/1?ref=x" } } }] } }"#,
    ),
    (
      "/broker/pacts/1?ref=x",
      200,
      "application/hal+json",
      r#"{ "consumer": { "name": "web" }, "provider": { "name": "order api" }, "interactions": [] }"#,
    ),
  ]);
  let fetched = fetch_from(&format!("{base}/broker")).unwrap();
  assert_eq!(fetched[0].document["consumer"]["name"], "web");
  assert_eq!(
    *seen.lock().unwrap(),
    vec![
      "GET /broker",
      "POST /broker/pacts/provider/order%20api/for-verification",
      "GET /broker/pacts/1?ref=x",
    ]
  );
}

#[test]
fn the_descriptor_says_how_many_times_to_retry() {
  let (url, asked) = answering(vec![(502, None), (503, None)]);
  let mut broker = descriptor(&url);
  broker.client = Some(json!({ "retries": 0 }));
  let err = get(&broker, &url).unwrap_err();
  assert_eq!(err.details.as_ref().unwrap()["status"], 502);
  assert_eq!(*asked.lock().unwrap(), 1);
}

#[test]
fn client_options_the_client_cannot_act_on_are_named_by_member() {
  let mut broker = descriptor("http://user:pass@broker.example");
  broker.client = Some(json!({ "tls": { "ca-certificates": ["not PEM"] } }));
  let err = get(&broker, "http://broker.example").unwrap_err();
  assert_eq!(err.code, "invalid-config");
  assert!(
    err.message.contains("/client/tls/ca-certificates/0"),
    "{}",
    err.message
  );
  assert!(!err.message.contains("pass"), "{}", err.message);
}
