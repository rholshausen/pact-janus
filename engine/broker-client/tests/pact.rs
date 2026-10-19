//! Staged plan S1.3's third test layer: the client's own consumer pact with the Pact Broker,
//! written here with `pact_consumer` and verified against the pinned broker by today's verifier.
//!
//! A pact nobody verifies is a mock that agrees with itself, so the test that writes it also
//! verifies it when it can: with `JANUS_PACT_BROKER` naming the pinned broker and
//! `PACT_VERIFIER_CLI` today's verifier, as in CI's `verifier-diff` job, it runs
//! `pact_verifier_cli` over the pact against that broker, with a provider-state endpoint of its
//! own that sets each state up through the broker's API. Without them it writes the pact, checks
//! the client against it, and says it did not verify it.
//!
//! The parties are the Ruby broker's own: Condor, the consumer, and the Pricing Service, its
//! provider, with the Ruby broker's provider-state names where one fits
//! (`spec/service_consumers/shared_provider_states.rb`), so the pact can be proposed upstream as it
//! stands. Each state starts from nothing, as the Ruby broker's do on a fresh database: it deletes
//! both pacticipants first.
//!
//! What each request carries is what the client sends, exactly: that is the evidence. What each
//! response is expected to hold is only what the client reads, matched by type, because what a
//! client expects back is not evidence of what a broker sends; verification against the real one
//! is.

use pact_consumer::prelude::*;
use pact_janus_broker_client::HalBrokerClient;
use pact_janus_kernel::broker::{
  BrokerClient, BrokerDescriptor, ContractToPublish, ContractsToPublish, FetchRequest, InteractionResult,
  PublishRequest,
};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::process::Command;

const CONSUMER: &str = "Pact Janus";
const PROVIDER: &str = "Pact Broker";

const BY_MAIN: &str = "a pact between Condor and the Pricing Service exists with branch main";
const BY_FEATURE: &str = "a pact between Condor and the Pricing Service exists with branch feature";
const PACT_EXISTS: &str = "a pact between Condor and the Pricing Service exists";
const BOTH_EXIST: &str = "the 'Pricing Service' and 'Condor' already exist in the pact-broker";
const ALREADY_PUBLISHED: &str = "the 'Pricing Service' and 'Condor' already exist in the pact-broker, and Condor already has a pact published for version 1.3.0";

const ACCEPT: &str = "application/hal+json, application/json";

/// The pact the states publish for Condor 1.3.0.
fn seeded_pact() -> Value {
  json!({
    "consumer": { "name": "Condor" },
    "provider": { "name": "Pricing Service" },
    "interactions": [{
      "description": "a request for prices",
      "request": { "method": "GET", "path": "/prices" },
      "response": { "status": 200 }
    }],
    "metadata": { "pactSpecification": { "version": "3.0.0" } }
  })
}

/// The broker's id for [`seeded_pact`]'s content, which names its verification results resource:
/// the same content always has the same one. If the seeded pact changes, so does this.
const SEEDED_PACT_VERSION: &str = "eb9aa800bd15e370dc6ac4f32187011ec5261fcc";

fn pact_dir() -> PathBuf {
  PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("pacts")
}

fn builder() -> PactBuilder {
  let mut builder = PactBuilder::new(CONSUMER, PROVIDER);
  builder.with_output_dir(pact_dir());
  builder
}

/// The index, as every scenario asks for it: the links the client follows from it, and nothing
/// else.
fn index(builder: &mut PactBuilder) -> &mut PactBuilder {
  builder.interaction("a request for the index resource", "", |mut i| {
    i.request.get().path("/").header("Accept", ACCEPT);
    i.response
      .status(200)
      .header("Content-Type", "application/hal+json")
      .json_body(json_pattern!({
        "_links": {
          "pb:provider-pacts-for-verification": {
            "href": like!("http://localhost:9292/pacts/provider/{provider}/for-verification")
          },
          "pb:publish-contracts": { "href": like!("http://localhost:9292/contracts/publish") }
        }
      }));
    i
  })
}

/// The pact the broker selected, by the link it gave for it.
fn the_pact<'a>(builder: &'a mut PactBuilder, description: &str, state: &str) -> &'a mut PactBuilder {
  builder.interaction(description, "", |mut i| {
    i.given(state);
    i.request
      .get()
      .path("/pacts/provider/Pricing%20Service/consumer/Condor/version/1.3.0")
      .header("Accept", ACCEPT);
    i.response
      .status(200)
      .header("Content-Type", "application/hal+json")
      .json_body(json_pattern!({
        "consumer": { "name": "Condor" },
        "provider": { "name": "Pricing Service" },
        "interactions": each_like!({
          "_id": like!("fa0f768446d11727ca949500c9331de67f48634d"),
          "description": like!("a request for prices")
        }),
        "_links": {
          "pb:provider": { "href": like!("http://localhost:9292/pacticipants/Pricing%20Service") },
          "pb:publish-verification-results": {
            "href": like!(format!("http://localhost:9292/pacts/provider/Pricing%20Service/consumer/Condor/pact-version/{SEEDED_PACT_VERSION}/verification-results"))
          }
        }
      }));
    i
  })
}

fn for_verification(builder: &mut PactBuilder, description: &str, state: &str, wip: bool) {
  builder.interaction(description, "", |mut i| {
    i.given(state);
    i.request
      .post()
      .path("/pacts/provider/Pricing%20Service/for-verification")
      .header("Accept", ACCEPT)
      .header("Content-Type", "application/json")
      .json_body(json_pattern!({
        "consumerVersionSelectors": [{ "mainBranch": true }],
        "includePendingStatus": true,
        "providerVersionBranch": "main",
        "includeWipPactsSince": "2020-01-01"
      }));
    let properties = if wip {
      json_pattern!({
        "pending": true,
        "wip": true,
        "notices": each_like!({ "text": like!("This pact is being verified because it is a 'work in progress' pact") })
      })
    } else {
      json_pattern!({
        "pending": true,
        "notices": each_like!({ "text": like!("This pact is in pending state for this version of Pricing Service") })
      })
    };
    i.response
      .status(200)
      .header("Content-Type", "application/hal+json")
      .json_body(json_pattern!({
        "_embedded": {
          "pacts": each_like!({
            "verificationProperties": properties,
            "_links": {
              "self": { "href": like!("http://localhost:9292/pacts/provider/Pricing%20Service/consumer/Condor/version/1.3.0") }
            }
          })
        }
      }));
    i
  });
}

fn fetch_request(url: &str) -> FetchRequest {
  FetchRequest {
    broker: descriptor(url),
    provider: "Pricing Service".to_string(),
    selectors: vec![json!({ "mainBranch": true })],
    provider_branch: Some("main".to_string()),
    provider_tags: vec![],
    include_pending: true,
    include_wip_since: Some("2020-01-01".to_string()),
  }
}

fn descriptor(url: &str) -> BrokerDescriptor {
  serde_json::from_value(json!({ "url": url.trim_end_matches('/') })).unwrap()
}

/// The provider fetches from `main`, where Condor's pact has never been verified: it is pending.
/// Then it publishes its result: the provider version's branch and tag first, then the result.
fn fetching_and_publishing_results() {
  let mut builder = builder();
  index(&mut builder);
  for_verification(
    &mut builder,
    "a request for the pacts for verification for the Pricing Service from main, with pending and work-in-progress pacts",
    BY_MAIN,
    false,
  );
  the_pact(&mut builder, "a request for a pact", PACT_EXISTS);
  builder
    .interaction("a request for the Pricing Service", "", |mut i| {
      i.given(BOTH_EXIST);
      i.request
        .get()
        .path("/pacticipants/Pricing%20Service")
        .header("Accept", ACCEPT);
      i.response
        .status(200)
        .header("Content-Type", "application/hal+json")
        .json_body(json_pattern!({
          "_links": {
            "pb:branch-version": {
              "href": like!("http://localhost:9292/pacticipants/Pricing%20Service/branches/{branch}/versions/{version}")
            },
            "pb:version-tag": {
              "href": like!("http://localhost:9292/pacticipants/Pricing%20Service/versions/{version}/tags/{tag}")
            }
          }
        }));
      i
    })
    .interaction("a request to add version 4.5.6 of the Pricing Service to its main branch", "", |mut i| {
      i.given(BOTH_EXIST);
      i.request
        .put()
        .path("/pacticipants/Pricing%20Service/branches/main/versions/4.5.6")
        .header("Accept", ACCEPT)
        .header("Content-Type", "application/json")
        .json_body(json_pattern!({}));
      i.response.status(201);
      i
    })
    .interaction("a request to tag version 4.5.6 of the Pricing Service prod", "", |mut i| {
      i.given(BOTH_EXIST);
      i.request
        .put()
        .path("/pacticipants/Pricing%20Service/versions/4.5.6/tags/prod")
        .header("Accept", ACCEPT)
        .header("Content-Type", "application/json")
        .json_body(json_pattern!({}));
      i.response.status(201);
      i
    })
    .interaction("a request to publish a verification result", "", |mut i| {
      i.given(PACT_EXISTS);
      i.request
        .post()
        .path(format!(
          "/pacts/provider/Pricing%20Service/consumer/Condor/pact-version/{SEEDED_PACT_VERSION}/verification-results"
        ))
        .header("Accept", ACCEPT)
        .header("Content-Type", "application/json")
        .json_body(json_pattern!({
          "success": false,
          "providerApplicationVersion": "4.5.6",
          "buildUrl": "https://ci.example/builds/1",
          "verifiedBy": { "implementation": "Pact Janus", "version": like!("0.1.0") },
          "testResults": [
            {
              "interactionId": "fa0f768446d11727ca949500c9331de67f48634d",
              "interactionDescription": "a request for prices",
              "success": false,
              "mismatches": [{ "attribute": "status", "description": "expected 200 but was 404" }]
            }
          ]
        }));
      i.response.status(201);
      i
    });

  let server = builder.start_mock_server(None, None);
  let client = HalBrokerClient::new();
  let url = server.url().to_string();
  let fetched = client
    .fetch_for_verification(&fetch_request(&url))
    .expect("fetches");
  assert_eq!(fetched.len(), 1);
  assert!(fetched[0].pending && !fetched[0].wip);
  assert_eq!(fetched[0].document["consumer"]["name"], "Condor");

  let broker = descriptor(&url);
  client
    .publish_results(&PublishRequest {
      broker: &broker,
      handle: &fetched[0].handle,
      provider_version: "4.5.6",
      branch: Some("main"),
      tags: &["prod".to_string()],
      build_url: Some("https://ci.example/builds/1"),
      success: false,
      results: vec![InteractionResult {
        id: fetched[0].document["interactions"][0]["_id"]
          .as_str()
          .map(str::to_string),
        description: "a request for prices".to_string(),
        success: false,
        mismatches: vec![json!({ "path": "$.response.status", "message": "expected 200 but was 404" })],
        error: None,
      }],
    })
    .expect("publishes");
}

/// Condor's pact is on a feature branch, which no selector selects: it is fetched as a
/// work-in-progress pact, and pending.
fn fetching_a_work_in_progress_pact() {
  let mut builder = builder();
  index(&mut builder);
  for_verification(
    &mut builder,
    "a request for the pacts for verification for the Pricing Service from main, when the only pact is on a feature branch",
    BY_FEATURE,
    true,
  );
  the_pact(&mut builder, "a request for a work-in-progress pact", BY_FEATURE);
  let server = builder.start_mock_server(None, None);
  let fetched = HalBrokerClient::new()
    .fetch_for_verification(&fetch_request(server.url().as_str()))
    .expect("fetches");
  assert!(fetched[0].pending && fetched[0].wip);
}

/// The contract a consumer publishes: its bytes, as the client sends them.
fn condor_contract(status: u16) -> ContractToPublish {
  let mut pact = seeded_pact();
  pact["interactions"][0]["response"]["status"] = json!(status);
  ContractToPublish {
    provider: "Pricing Service".to_string(),
    content: serde_json::to_vec(&pact).unwrap(),
  }
}

fn publishing(url: &str, contract: ContractToPublish) -> Result<Vec<String>, String> {
  let broker = descriptor(url);
  HalBrokerClient::new()
    .publish_contracts(&ContractsToPublish {
      broker: &broker,
      consumer: "Condor",
      consumer_version: "1.3.0",
      branch: Some("main"),
      tags: &["dev".to_string()],
      build_url: Some("https://ci.example/builds/2"),
      contracts: &[contract],
    })
    .map(|published| published.notices)
    .map_err(|error| format!("{}: {}", error.code, error.message))
}

fn publish_request(contract: &ContractToPublish) -> JsonPattern {
  use base64::Engine as _;
  json_pattern!({
    "pacticipantName": "Condor",
    "pacticipantVersionNumber": "1.3.0",
    "branch": "main",
    "tags": ["dev"],
    "buildUrl": "https://ci.example/builds/2",
    "contracts": [{
      "consumerName": "Condor",
      "providerName": "Pricing Service",
      "specification": "pact",
      "contentType": "application/json",
      "content": base64::engine::general_purpose::STANDARD.encode(&contract.content)
    }]
  })
}

/// Condor publishes a pact for a new version.
fn publishing_a_contract() {
  let contract = condor_contract(200);
  let mut builder = builder();
  index(&mut builder);
  builder.interaction("a request to publish contracts", "", |mut i| {
    i.given(BOTH_EXIST);
    i.request
      .post()
      .path("/contracts/publish")
      .header("Accept", ACCEPT)
      .header("Content-Type", "application/json")
      .json_body(publish_request(&contract));
    i.response
      .status(200)
      .header("Content-Type", "application/hal+json")
      .json_body(json_pattern!({
        "notices": each_like!({
          "text": like!("Pact successfully published for Condor version 1.3.0 and provider Pricing Service.")
        })
      }));
    i
  });
  let server = builder.start_mock_server(None, None);
  let notices = publishing(server.url().as_str(), contract).expect("publishes");
  assert!(!notices.is_empty());
}

/// Condor publishes different content for a version the broker already has a pact for, which the
/// broker refuses.
fn publishing_changed_content_for_a_version() {
  let contract = condor_contract(201);
  let mut builder = builder();
  index(&mut builder);
  builder.interaction(
    "a request to publish contracts with content that differs from what was published for the version",
    "",
    |mut i| {
      i.given(ALREADY_PUBLISHED);
      i.request
        .post()
        .path("/contracts/publish")
        .header("Accept", ACCEPT)
        .header("Content-Type", "application/json")
        .json_body(publish_request(&contract));
      i.response
        .status(409)
        .header("Content-Type", "application/json")
        .json_body(json_pattern!({
          "errors": {
            "contracts": each_like!("Cannot change the content of the pact for Condor version 1.3.0 and provider Pricing Service")
          }
        }));
      i
    },
  );
  let server = builder.start_mock_server(None, None);
  let refused = publishing(server.url().as_str(), contract).unwrap_err();
  assert!(refused.starts_with("broker-refused:"), "{refused}");
  assert!(
    refused.contains("with 409: contracts: Cannot change the content"),
    "{refused}"
  );
}

// ---------------------------------------------------------------------------------------------
// Verifying the pact against the pinned broker
// ---------------------------------------------------------------------------------------------

/// The provider-state endpoint today's verifier calls before each interaction: it deletes Condor
/// and the Pricing Service, then sets the state up through the broker's own API.
fn provider_states(broker: String) -> String {
  let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
  let url = format!("http://{}", server.server_addr().to_ip().unwrap());
  std::thread::spawn(move || {
    let agent: ureq::Agent = ureq::Agent::config_builder()
      .http_status_as_error(false)
      .build()
      .into();
    let broker = broker.trim_end_matches('/').to_string();
    let send = |method: &str, path: &str, body: Option<Value>| -> u16 {
      let request = ureq::http::Request::builder()
        .method(method)
        .uri(format!("{broker}{path}"))
        .header("accept", "application/hal+json")
        .header("content-type", "application/json")
        .body(body.map(|body| body.to_string()).unwrap_or_default())
        .unwrap();
      agent.run(request).map(|r| r.status().as_u16()).unwrap_or(0)
    };
    let publish = |branch: &str| {
      use base64::Engine as _;
      let content = base64::engine::general_purpose::STANDARD.encode(seeded_pact().to_string());
      send(
        "POST",
        "/contracts/publish",
        Some(json!({
          "pacticipantName": "Condor", "pacticipantVersionNumber": "1.3.0", "branch": branch,
          "contracts": [{ "consumerName": "Condor", "providerName": "Pricing Service",
                          "specification": "pact", "contentType": "application/json", "content": content }]
        })),
      )
    };
    for mut request in server.incoming_requests() {
      let mut body = String::new();
      let _ = request.as_reader().read_to_string(&mut body);
      let change: Value = serde_json::from_str(&body).unwrap_or_default();
      let state = change["state"].as_str().unwrap_or_default();
      let mut statuses = Vec::new();
      // The verifier calls with no state for an interaction that names none; it needs nothing.
      if change["action"] != json!("teardown") && !state.is_empty() {
        for name in ["Condor", "Pricing%20Service"] {
          statuses.push(send("DELETE", &format!("/pacticipants/{name}"), None));
        }
        match state {
          BY_MAIN => statuses.push(publish("main")),
          BY_FEATURE => statuses.push(publish("feature")),
          PACT_EXISTS | ALREADY_PUBLISHED => statuses.push(publish("main")),
          BOTH_EXIST => {
            for name in ["Condor", "Pricing Service"] {
              statuses.push(send("POST", "/pacticipants", Some(json!({ "name": name }))));
            }
          }
          other => eprintln!("provider state '{other}' has no set-up"),
        }
      }
      let ok = statuses.iter().all(|status| matches!(status, 200..=299 | 404));
      let _ = request.respond(
        tiny_http::Response::from_string(json!({ "state": state, "statuses": statuses }).to_string())
          .with_status_code(if ok { 200 } else { 500 }),
      );
    }
  });
  url
}

fn verify(broker: &str, verifier: &str, pact: &Path) {
  let (host, port) = broker
    .trim_end_matches('/')
    .rsplit_once("://")
    .map(|(_, authority)| authority)
    .and_then(|authority| authority.rsplit_once(':'))
    .expect("JANUS_PACT_BROKER is http://host:port");
  let states = provider_states(broker.to_string());
  let output = Command::new(verifier)
    .args([
      "--file",
      &pact.display().to_string(),
      "--hostname",
      host,
      "--port",
      port,
    ])
    .args(["--state-change-url", &states])
    .env("PACT_DO_NOT_TRACK", "true")
    .output()
    .expect("today's verifier runs");
  assert!(
    output.status.success(),
    "today's verifier did not verify the client's pact against {broker}:\n{}\n{}",
    String::from_utf8_lossy(&output.stdout),
    String::from_utf8_lossy(&output.stderr)
  );
  eprintln!("{}", String::from_utf8_lossy(&output.stdout));
}

#[test]
fn the_client_keeps_to_its_pact_with_the_pact_broker_and_the_broker_honours_it() {
  // One test, so the scenarios run one after another into one pact file, which starts empty.
  let _ = std::fs::remove_dir_all(pact_dir());
  fetching_and_publishing_results();
  fetching_a_work_in_progress_pact();
  publishing_a_contract();
  publishing_changed_content_for_a_version();
  let pact = pact_dir().join(format!("{CONSUMER}-{PROVIDER}.json"));
  assert!(pact.is_file(), "{} was not written", pact.display());

  let broker = std::env::var("JANUS_PACT_BROKER").ok().filter(|v| !v.is_empty());
  let verifier = std::env::var("PACT_VERIFIER_CLI").ok().filter(|v| !v.is_empty());
  match (broker, verifier) {
    (Some(broker), Some(verifier)) => verify(&broker, &verifier, &pact),
    _ => eprintln!(
      "{} written and not verified: JANUS_PACT_BROKER and PACT_VERIFIER_CLI name no broker and no verifier",
      pact.display()
    ),
  }
}
