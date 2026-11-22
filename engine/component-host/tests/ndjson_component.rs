//! S5.1d's exit (ADR 0104): `third-party/janus-ndjson`, a component written from the component author
//! guide alone, is published to a registry, with `wkg oci push` when `JANUS_WKG` names one and with
//! `janus component push`'s loader otherwise, signed (by a Sigstore made for the test, as
//! `signature.rs` does), declared pinned by its digest and naming its signer, and used by the real
//! engine in a consumer session and a verification, each of which records it.

mod support;

use pact_janus_component_host::WasmLoader;
use pact_janus_component_http::HttpTransport;
use pact_janus_component_json::JsonContent;
use pact_janus_kernel::component::TransportComponent;
use pact_janus_kernel::protocol::Engine;
use pretty_assertions::assert_eq;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use support::ndjson_wasm;
use support::registry::Registry;
use support::sigstore::{self, Signing, Sigstore};

/// The name the component's handshake gives, and so the name a project declares it under.
const NAME: &str = "ndjson";
const MEDIA_TYPE: &str = "application/x-ndjson";
const WORKFLOW: &str = "https://github.com/acme/janus-ndjson/.github/workflows/release.yml@refs/tags/v1.0.0";
const GITHUB: &str = "https://token.actions.githubusercontent.com";

fn scratch(label: &str) -> PathBuf {
  static N: AtomicU64 = AtomicU64::new(0);
  let dir = std::env::temp_dir().join(format!(
    "janus-ndjson-{label}-{}-{}",
    std::process::id(),
    N.fetch_add(1, Ordering::Relaxed)
  ));
  let _ = std::fs::remove_dir_all(&dir);
  std::fs::create_dir_all(&dir).unwrap();
  dir
}

/// Push the component to `reference` as its author would, and return the digest to pin: with `wkg`
/// when `JANUS_WKG` names one, which is the exit's, and otherwise with the loader `janus component
/// push` uses.
fn publish(registry: &Registry, reference: &str) -> String {
  let wasm = std::fs::read(ndjson_wasm()).unwrap();
  let Ok(wkg) = std::env::var("JANUS_WKG") else {
    eprintln!("JANUS_WKG is not set; publishing with janus component push's loader instead of wkg");
    return WasmLoader::with_cache(scratch("push"))
      .unwrap()
      .push(&wasm, reference, None)
      .unwrap_or_else(|err| panic!("{err:?}"))
      .digest;
  };
  let pushed = std::process::Command::new(wkg)
    .args(["oci", "push", "--insecure", &registry.host, reference])
    .arg(ndjson_wasm())
    .output()
    .expect("wkg runs");
  let stdout = String::from_utf8_lossy(&pushed.stdout);
  assert!(
    pushed.status.success(),
    "wkg oci push failed: {stdout}{}",
    String::from_utf8_lossy(&pushed.stderr)
  );
  stdout
    .lines()
    .find_map(|line| line.trim().strip_prefix("digest: "))
    .unwrap_or_else(|| panic!("wkg printed no digest: {stdout}"))
    .to_string()
}

/// Sign `digest` as the workflow [`WORKFLOW`], and attach the bundle as `cosign sign` does.
fn sign(registry: &Registry, repository: &str, sigstore: &Sigstore, digest: &str) {
  let bundle = sigstore.sign(
    digest,
    &Signing {
      identity: WORKFLOW,
      ..Signing::default()
    },
  );
  let size = registry
    .manifest(repository, digest)
    .map_or(0, |manifest| manifest.len());
  let (manifest, blobs) = sigstore::referrer(digest, size, &bundle);
  registry.attach(repository, digest, &manifest, &blobs);
}

fn send(engine: &mut Engine, op: &str, body: Value) -> Value {
  let request = json!({ "type": "request", "id": "1", "op": op, "body": body });
  serde_json::from_slice(&engine.dispatch(&serde_json::to_vec(&request).unwrap())).unwrap()
}

/// What `janus-engine` builds: the in-tree HTTP transport and JSON content, and the WASM loader.
fn engine() -> Engine {
  let mut transports: HashMap<String, Arc<dyn TransportComponent>> = HashMap::new();
  transports.insert("http".to_string(), Arc::new(HttpTransport::new()));
  let mut engine = Engine::with_components(transports, Some(Arc::new(JsonContent::new())));
  engine.register_background(Arc::new(pact_janus_kernel::protocol::Threads));
  engine.declare_in_tree("content", "json", "1.0.0");
  engine.register_component_loader(Arc::new(WasmLoader::with_cache(scratch("cache")).unwrap()));
  send(
    &mut engine,
    "engine/hello",
    json!({ "protocol-versions": [1], "host": { "name": "janus-test", "version": "0.0.0" }, "capabilities": {} }),
  );
  engine
}

/// Orders as NDJSON: one JSON object per line, each with the members this consumer reads.
fn interaction() -> Value {
  json!({
    "description": "every order, as NDJSON",
    "transport": { "kind": "http", "mode": "passive" },
    "requires": [ { "component": format!("content/{NAME}"), "min-version": 1 } ],
    "content-types": { "response": { "body": MEDIA_TYPE } },
    "parts": {
      "request": { "method": { "shape": "equality", "example": "GET" },
                   "path": { "shape": "equality", "example": "/orders.ndjson" } },
      "response": {
        "status": { "shape": "equality", "example": 200 },
        "body": { "shape": "each-like", "min": 1, "max": 1,
                  "items": { "shape": "object", "members": {
                    "id": { "shape": "integer", "example": 66 },
                    "status": { "shape": "string", "example": "PENDING" } } } } } }
  })
}

fn http_get(addr: &str, path: &str) -> (u16, Vec<(String, String)>, String) {
  let mut stream = TcpStream::connect(addr).unwrap();
  stream.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
  write!(
    stream,
    "GET {path} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n"
  )
  .unwrap();
  let mut response = Vec::new();
  let _ = stream.read_to_end(&mut response);
  let text = String::from_utf8(response).unwrap();
  let (head, body) = text.split_once("\r\n\r\n").unwrap();
  let status = head.split_whitespace().nth(1).unwrap().parse().unwrap();
  let headers = head
    .lines()
    .skip(1)
    .filter_map(|line| line.split_once(':'))
    .map(|(name, value)| (name.trim().to_ascii_lowercase(), value.trim().to_string()))
    .collect();
  (status, headers, body.to_string())
}

/// A consumer session declaring `component`: every variant served and read back, then the contract.
fn consumer_run(engine: &mut Engine, component: &Value) -> (Vec<String>, Value) {
  let created = send(
    engine,
    "consumer-session/create",
    json!({ "config": { "consumer": { "name": "reporting" }, "provider": { "name": "order-service" },
                        "components": [component] } }),
  );
  let session = created["ok"]["session"]
    .as_str()
    .unwrap_or_else(|| panic!("create: {created}"))
    .to_string();
  let added = send(
    engine,
    "consumer-session/add-interaction",
    json!({ "session": session, "interaction": interaction() }),
  );
  let handle = added["ok"]["handle"]
    .as_str()
    .unwrap_or_else(|| panic!("add: {added}"))
    .to_string();
  let variants = send(
    engine,
    "consumer-session/variants",
    json!({ "session": session, "handle": handle }),
  );
  let started = send(
    engine,
    "consumer-session/start-transport",
    json!({ "session": session, "transport": "http" }),
  );
  let endpoint = &started["ok"]["endpoint"];
  let addr = format!("{}:{}", endpoint["host"].as_str().unwrap(), endpoint["port"]);
  let mut bodies = Vec::new();
  for variant in variants["ok"]["variants"].as_array().unwrap() {
    send(
      engine,
      "consumer-session/serve-variant",
      json!({ "session": session, "handle": handle, "variant": variant["id"] }),
    );
    let (status, headers, body) = http_get(&addr, "/orders.ndjson");
    assert_eq!(status, 200, "{body}");
    assert!(
      headers
        .iter()
        .any(|(name, value)| name == "content-type" && value.starts_with(MEDIA_TYPE)),
      "{headers:?}"
    );
    bodies.push(body);
  }
  let finalised = send(engine, "consumer-session/finalise", json!({ "session": session }));
  (bodies, finalised["ok"]["contract"].clone())
}

/// A provider that answers every request with `body` as NDJSON, until the test ends.
fn provider(body: &'static str) -> String {
  let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
  let addr = listener.local_addr().unwrap();
  std::thread::spawn(move || {
    for stream in listener.incoming() {
      let Ok(mut stream) = stream else { continue };
      let mut request = [0u8; 4096];
      let _ = stream.read(&mut request);
      let _ = write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: {MEDIA_TYPE}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
      );
    }
  });
  format!("http://{addr}")
}

/// Verify `contract` against `base_url` with `component` declared, and return (summary, every event).
fn verify(engine: &mut Engine, contract: &Value, base_url: &str, component: &Value) -> (Value, Vec<Value>) {
  let started = send(
    engine,
    "verification/verify",
    json!({
      "source": { "kind": "inline", "contracts": [contract] },
      "target": { "transports": [ { "transport": "http", "options": { "base-url": base_url } } ],
                  "components": [component] },
    }),
  );
  let stream = started["ok"]["stream"]
    .as_str()
    .unwrap_or_else(|| panic!("verify: {started}"))
    .to_string();
  let mut events = Vec::new();
  for _ in 0..100 {
    let polled = send(
      engine,
      "events/poll",
      json!({ "streams": [stream], "wait-ms": 5_000 }),
    );
    for event in polled["ok"]["events"].as_array().unwrap() {
      events.push(event.clone());
      if event["last"] == json!(true) {
        return (event["payload"].clone(), events);
      }
    }
  }
  panic!("the run never finished");
}

/// The exit: published, signed, declared by digest and signer, and recorded, with that digest and
/// signer, in the contract a consumer session wrote and in the summary of the verification of it.
#[test]
fn a_component_written_from_the_guide_is_published_signed_declared_and_recorded() {
  let registry = Registry::start();
  let sigstore = Sigstore::new();
  let root = scratch("root").join("trusted_root.json");
  std::fs::write(&root, sigstore.trusted_root()).unwrap();
  let reference = format!("{}/janus-ndjson:1.0.0", registry.host);
  let digest = publish(&registry, &reference);
  sign(&registry, "janus-ndjson", &sigstore, &digest);

  let component = json!({
    "name": NAME,
    "source": { "kind": "oci", "reference": reference, "digest": digest,
                "signature": { "identity": WORKFLOW, "issuer": GITHUB, "trusted-root": root.display().to_string() } }
  });
  let recorded = json!([ { "name": NAME, "version": "1.0.0",
                           "source": { "kind": "oci", "reference": reference, "digest": digest,
                                       "signer": { "identity": WORKFLOW, "issuer": GITHUB } } } ]);

  let mut engine = engine();
  let (bodies, contract) = consumer_run(&mut engine, &component);
  assert_eq!(
    bodies,
    ["{\"id\":66,\"status\":\"PENDING\"}\n"],
    "the mock serves NDJSON the component wrote"
  );
  assert_eq!(contract["metadata"]["components"], recorded, "{contract:#}");

  let (summary, events) = verify(
    &mut engine,
    &contract,
    &provider("{\"id\": 1, \"status\": \"SHIPPED\"}\n"),
    &component,
  );
  assert_eq!(summary["status"], "verified", "{summary:#}\n{events:#?}");
  assert_eq!(summary["components"], recorded);

  let (summary, events) = verify(
    &mut engine,
    &contract,
    &provider("{\"id\": \"one\", \"status\": \"SHIPPED\"}\n"),
    &component,
  );
  assert_eq!(summary["status"], "failed", "{summary:#}");
  let paths: Vec<&str> = events
    .iter()
    .filter(|event| event["kind"] == "verification/interaction-result")
    .flat_map(|event| event["payload"]["mismatches"].as_array().into_iter().flatten())
    .filter_map(|mismatch| mismatch["path"].as_str())
    .collect();
  assert_eq!(paths, ["$.response.body[0].id"], "{events:#?}");

  // Signed by someone else, it does not load, and the run never starts.
  let mut untrusted = component.clone();
  untrusted["source"]["signature"]["identity"] = json!("https://github.com/someone/else");
  let refused = send(
    &mut engine,
    "verification/verify",
    json!({ "source": { "kind": "inline", "contracts": [contract] },
            "target": { "transports": [ { "transport": "http", "options": { "base-url": "http://127.0.0.1:9" } } ],
                        "components": [untrusted] } }),
  );
  assert_eq!(refused["error"]["code"], "component-unavailable", "{refused}");
  assert_eq!(
    refused["error"]["details"]["error"]["code"],
    "signature-untrusted"
  );
}
