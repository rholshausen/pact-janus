//! ADR 0063: a contract crosses the engine boundary as its text. A host that declares
//! `contract-text` gets an outbound contract as its canonical bytes, and every request member that
//! carries a contract or pact may carry the document's text. `corpora/contracts/` pins how the
//! engine writes numbers (contract-file spec §2.4).

use pact_janus_component_http::HttpTransport;
use pact_janus_component_json::JsonContent;
use pact_janus_kernel::component::TransportComponent;
use pact_janus_kernel::contract::{self, IdentifyMode};
use pact_janus_kernel::protocol::{Engine, Threads};
use pretty_assertions::assert_eq;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

fn send(engine: &mut Engine, id: &str, op: &str, body: Value) -> Value {
  let request = json!({ "type": "request", "id": id, "op": op, "body": body });
  let bytes = engine.dispatch(&serde_json::to_vec(&request).expect("a json! literal always serializes"));
  serde_json::from_slice(&bytes).expect("Engine::dispatch always returns valid JSON")
}

/// An engine with the real HTTP transport and threads, after a hello that declared
/// `contract-text` or did not.
fn engine(contract_text: bool) -> Engine {
  let mut transports: HashMap<String, Arc<dyn TransportComponent>> = HashMap::new();
  transports.insert("http".to_string(), Arc::new(HttpTransport::new()));
  let mut engine = Engine::with_components(transports, Some(Arc::new(JsonContent::new())));
  engine.register_background(Arc::new(Threads));
  let capabilities = if contract_text {
    json!({ "contract-text": {} })
  } else {
    json!({})
  };
  let hello = send(
    &mut engine,
    "h-1",
    "engine/hello",
    json!({ "protocol-versions": [1], "host": { "name": "janus-test", "version": "0.0.0" }, "capabilities": capabilities }),
  );
  assert_eq!(hello["ok"]["capabilities"]["contract-text"], json!({}), "{hello}");
  engine
}

fn corpus() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpora/contracts")
}

/// Each case under `corpora/contracts/`: its name, its pact's text, and the contract's bytes.
fn cases() -> Vec<(String, String, String)> {
  let mut cases: Vec<(String, String, String)> = std::fs::read_dir(corpus())
    .expect("corpora/contracts exists")
    .map(|entry| entry.expect("a directory entry").path())
    .filter(|path| path.is_dir())
    .map(|dir| {
      let name = dir.file_name().unwrap().to_string_lossy().to_string();
      let pact = std::fs::read_to_string(dir.join("pact.json")).expect("a case has pact.json");
      let contract =
        std::fs::read_to_string(dir.join("contract.janus.json")).expect("a case has contract.janus.json");
      (name, pact, contract)
    })
    .collect();
  cases.sort();
  assert!(!cases.is_empty(), "corpora/contracts has cases");
  cases
}

#[test]
fn an_upgraded_contract_is_written_byte_for_byte_as_the_corpus_says() {
  for (name, pact, expected) in cases() {
    let mut engine = engine(true);
    let upgraded = send(&mut engine, "u-1", "upgrade/pact", json!({ "pact": pact }));
    assert!(upgraded["ok"].get("contract").is_none(), "{name}: {upgraded}");
    assert_eq!(
      upgraded["ok"]["contract-text"]
        .as_str()
        .expect("contract-text is a string"),
      expected,
      "{name}"
    );
  }
}

#[test]
fn reading_a_corpus_contract_and_writing_it_again_changes_no_byte() {
  for (name, _, expected) in cases() {
    let read = contract::read(expected.as_bytes(), IdentifyMode::Strict).expect("the corpus contract reads");
    let written = contract::write_canonical(&read).expect("it writes");
    assert_eq!(String::from_utf8(written).unwrap(), expected, "{name}");
  }
}

#[test]
fn a_host_that_did_not_declare_contract_text_gets_the_document() {
  for (name, pact, expected) in cases() {
    let mut engine = engine(false);
    let upgraded = send(
      &mut engine,
      "u-1",
      "upgrade/pact",
      json!({ "pact": serde_json::from_str::<Value>(&pact).unwrap() }),
    );
    assert!(
      upgraded["ok"].get("contract-text").is_none(),
      "{name}: {upgraded}"
    );
    let document: Value = serde_json::from_str(&expected).unwrap();
    assert_eq!(upgraded["ok"]["contract"], document, "{name}");
  }
}

#[test]
fn a_pact_whose_text_is_not_json_is_the_documents_error() {
  let mut engine = engine(true);
  let upgraded = send(
    &mut engine,
    "u-1",
    "upgrade/pact",
    json!({ "pact": "{ \"consumer\": " }),
  );
  assert_eq!(upgraded["error"]["code"], "contract-invalid", "{upgraded}");
  assert_eq!(
    upgraded["error"]["details"]["problems"][0]["pointer"], "/pact",
    "{upgraded}"
  );
}

#[test]
fn verify_reads_a_contracts_text_and_points_into_text_that_is_not_json() {
  let (_, pact, contract) = cases().remove(0);
  let mut engine = engine(true);
  let target =
    json!({ "transports": [ { "transport": "http", "options": { "base-url": "http://127.0.0.1:9" } } ] });
  let refused = send(
    &mut engine,
    "v-1",
    "verification/verify",
    json!({ "source": { "kind": "inline", "contracts": [contract, pact, "not json"] }, "target": target }),
  );
  assert_eq!(refused["error"]["code"], "contract-invalid", "{refused}");
  assert_eq!(
    refused["error"]["details"]["problems"][0]["pointer"], "/source/contracts/2",
    "the contract's and the pact's texts read; the third is the one that is not JSON: {refused}"
  );
}

#[test]
fn subsumption_check_reads_a_contracts_text() {
  let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/order-service");
  let pact = std::fs::read_to_string(root.join("pacts/web-app-order-service.json")).unwrap();
  let shape: Value = serde_json::from_str(
    &std::fs::read_to_string(root.join("shapes/order-service.provider-shape.json")).unwrap(),
  )
  .unwrap();
  let mut engine = engine(true);
  let upgraded = send(&mut engine, "u-1", "upgrade/pact", json!({ "pact": pact }));
  let contract = upgraded["ok"]["contract-text"].clone();
  assert!(contract.is_string(), "{upgraded}");
  let checked = send(
    &mut engine,
    "s-1",
    "subsumption/check",
    json!({ "contract": contract, "provider-shape": shape }),
  );
  assert!(checked["ok"]["report"].is_object(), "{checked}");
}

/// One GET on a fresh connection; the body is not read.
fn http_get(addr: &str, path: &str) {
  let mut stream =
    TcpStream::connect(addr).unwrap_or_else(|err| panic!("connecting to the mock at {addr}: {err}"));
  stream.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
  let request = format!("GET {path} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n");
  stream.write_all(request.as_bytes()).unwrap();
  let mut response = Vec::new();
  let _ = stream.read_to_end(&mut response);
  assert!(
    response.starts_with(b"HTTP/1.1 200"),
    "{}",
    String::from_utf8_lossy(&response)
  );
}

#[test]
fn finalise_answers_the_contracts_canonical_text_to_a_host_that_asked() {
  let mut engine = engine(true);
  let created = send(
    &mut engine,
    "c-1",
    "consumer-session/create",
    json!({ "config": { "consumer": { "name": "web-app" }, "provider": { "name": "price-api" } } }),
  );
  let session = created["ok"]["session"].as_str().unwrap().to_string();
  let added = send(
    &mut engine,
    "c-2",
    "consumer-session/add-interaction",
    json!({ "session": session, "interaction": {
      "description": "a price",
      "transport": { "kind": "http", "mode": "passive" },
      "parts": {
        "request": { "method": { "shape": "equality", "example": "GET" },
                     "path": { "shape": "equality", "example": "/price" } },
        "response": { "status": { "shape": "equality", "example": 200 },
                      "body": { "shape": "object", "members": {
                        "amount": { "shape": "equality", "example": 1.0 },
                        "id": { "shape": "equality", "example": 9007199254740993_u64 } } } } }
    } }),
  );
  let handle = added["ok"]["handle"].as_str().unwrap().to_string();
  send(
    &mut engine,
    "c-3",
    "consumer-session/variants",
    json!({ "session": session, "handle": handle }),
  );
  let started = send(
    &mut engine,
    "c-4",
    "consumer-session/start-transport",
    json!({ "session": session, "transport": "http" }),
  );
  let endpoint = &started["ok"]["endpoint"];
  let addr = format!(
    "{}:{}",
    endpoint["host"].as_str().unwrap(),
    endpoint["port"].as_u64().unwrap()
  );
  send(
    &mut engine,
    "c-5",
    "consumer-session/serve-variant",
    json!({ "session": session, "handle": handle, "variant": "base" }),
  );
  http_get(&addr, "/price");

  let finalised = send(
    &mut engine,
    "c-6",
    "consumer-session/finalise",
    json!({ "session": session }),
  );
  assert!(finalised["ok"].get("contract").is_none(), "{finalised}");
  let text = finalised["ok"]["contract-text"]
    .as_str()
    .expect("contract-text is a string");
  assert!(text.starts_with("{\"$format\":"), "{text}");
  assert!(
    text.ends_with("}\n"),
    "the text is the whole file, its LF included"
  );
  assert!(text.contains("\"example\":1.0"), "{text}");
  assert!(text.contains("\"example\":9007199254740993"), "{text}");
  let read = contract::read(text.as_bytes(), IdentifyMode::Strict).expect("the text is a contract");
  assert_eq!(
    String::from_utf8(contract::write_canonical(&read).unwrap()).unwrap(),
    text
  );
}
