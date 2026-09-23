//! Plan task 8.2: the 8.1 CSV component pushed as an OCI artifact, pulled, cached and
//! integrity-checked (component-interfaces spec §10.3) — against a registry in this process that
//! can be told to lie (`support/registry.rs`), and against a real one when `JANUS_OCI_REGISTRY`
//! names it (CI runs `registry:2` for this).

mod support;

use pact_janus_component_host::WasmLoader;
use pact_janus_component_host::oci::{self, Oci, Reference};
use pact_janus_component_http::HttpTransport;
use pact_janus_component_json::JsonContent;
use pact_janus_kernel::component::{
  ComponentDeclaration, ComponentError, ComponentLoader, Decode, Grants, Limits, SlotValue, Source,
  TransportComponent,
};
use pact_janus_kernel::protocol::Engine;
use pretty_assertions::assert_eq;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use support::csv_wasm;
use support::registry::{Registry, Tamper};

/// A cache no other test shares: every test here is about what is, or is not, already in it.
fn fresh_cache(label: &str) -> PathBuf {
  static N: AtomicU64 = AtomicU64::new(0);
  let dir = std::env::temp_dir().join(format!(
    "janus-oci-{label}-{}-{}",
    std::process::id(),
    N.fetch_add(1, Ordering::Relaxed)
  ));
  let _ = std::fs::remove_dir_all(&dir);
  dir
}

fn csv_bytes() -> Vec<u8> {
  std::fs::read(csv_wasm()).unwrap()
}

fn declaration(reference: &str, digest: Option<&str>) -> ComponentDeclaration {
  ComponentDeclaration {
    name: "csv".to_string(),
    source: Source {
      kind: "oci".to_string(),
      reference: Some(reference.to_string()),
      digest: digest.map(str::to_string),
    },
    grants: Grants::default(),
    limits: Limits::default(),
  }
}

fn refused(loader: &WasmLoader, declaration: &ComponentDeclaration) -> ComponentError {
  match loader.load(declaration) {
    Ok(_) => panic!("{:?} loaded", declaration.source),
    Err(err) => err,
  }
}

/// Load, then prove the thing that answers is the CSV component: decode a CSV body.
fn load_and_decode(loader: &WasmLoader, declaration: &ComponentDeclaration) -> Value {
  let loaded = loader.load(declaration).unwrap_or_else(|err| panic!("{err:?}"));
  assert_eq!(loaded.hello["component"]["name"], "csv");
  loaded
    .content
    .expect("the CSV component declares content")
    .decode(Decode {
      content_type: "text/csv".to_string(),
      value: SlotValue {
        content: json!("id,status\n66,PENDING\n"),
        encoded: Some("text".to_string()),
        content_type: None,
      },
      options: None,
    })
    .unwrap()
    .document
    .to_json()
}

/// Push the component to `registry` as `janus-csv:1.0.0`; the manifest digest it answers with.
fn push(registry: &str) -> String {
  let loader = WasmLoader::with_cache(fresh_cache("push")).unwrap();
  let pushed = loader
    .push(&csv_bytes(), &format!("{registry}/janus-csv:1.0.0"))
    .unwrap_or_else(|err| panic!("{err:?}"));
  pushed.digest
}

#[test]
fn a_pushed_component_carries_its_handshake_and_has_one_digest_however_often_it_is_pushed() {
  let registry = Registry::start();
  let loader = WasmLoader::with_cache(fresh_cache("push")).unwrap();
  let reference = format!("{}/janus-csv:1.0.0", registry.host);

  let first = loader.push(&csv_bytes(), &reference).unwrap();
  let again = loader.push(&csv_bytes(), &reference).unwrap();

  // The config is what the component said, not what the publisher typed.
  assert_eq!(first.config, json!({ "name": "csv", "version": "1.0.0" }));
  assert_eq!(
    first.digest, again.digest,
    "no timestamp, no host: same bytes, same artifact"
  );
  // The second push found both blobs already there and uploaded neither.
  let log = registry.take_log();
  let uploads = log
    .iter()
    .filter(|line| line.starts_with("PUT /v2/janus-csv/blobs/"))
    .count();
  assert_eq!(uploads, 2, "{log:#?}");
}

#[test]
fn a_component_declared_by_tag_is_resolved_and_answers() {
  let registry = Registry::start();
  let digest = push(&registry.host);
  let loader = WasmLoader::with_cache(fresh_cache("tag")).unwrap();

  let decoded = load_and_decode(
    &loader,
    &declaration(&format!("{}/janus-csv:1.0.0", registry.host), None),
  );
  assert_eq!(decoded, json!([{ "id": "66", "status": "PENDING" }]));

  // And the pull a person would run to find out what to pin says the same digest.
  let (pulled, hello) = loader
    .pull(&format!("oci://{}/janus-csv:1.0.0", registry.host), None)
    .unwrap();
  assert_eq!(pulled.digest, digest);
  assert_eq!(hello["component"]["version"], "1.0.0");
}

#[test]
fn a_pinned_component_is_fetched_once_and_then_never_again() {
  let registry = Registry::start();
  let digest = push(&registry.host);
  let reference = format!("{}/janus-csv:1.0.0", registry.host);
  let cache = fresh_cache("pinned");
  registry.take_log();

  let first = WasmLoader::with_cache(&cache).unwrap();
  load_and_decode(&first, &declaration(&reference, Some(&digest)));
  let fetched = registry.take_log();
  assert_eq!(
    fetched,
    vec![
      format!("GET /v2/janus-csv/manifests/{digest}"),
      fetched[1].clone(),
      fetched[2].clone(),
    ],
    "a pin is fetched by digest — the tag is never resolved"
  );

  // A second run — a new loader, as a new process would have — reads the cache and asks nothing.
  let second = WasmLoader::with_cache(&cache).unwrap();
  load_and_decode(&second, &declaration(&reference, Some(&digest)));
  assert_eq!(registry.take_log(), Vec::<String>::new());

  // Not even whether the registry is there.
  drop(registry);
  let offline = WasmLoader::with_cache(&cache).unwrap();
  load_and_decode(&offline, &declaration(&reference, Some(&digest)));
  // A tag, though, is a name, and resolving a name needs the registry.
  let unpinned = refused(&offline, &declaration(&reference, None));
  assert_eq!(unpinned.code, "unavailable", "{unpinned:?}");
}

#[test]
fn a_registry_that_serves_other_bytes_is_caught_before_anything_is_compiled() {
  let registry = Registry::start();
  let digest = push(&registry.host);
  let reference = format!("{}/janus-csv:1.0.0", registry.host);

  // A manifest that is not the one the pin names.
  registry.tamper(Tamper::Manifests);
  let loader = WasmLoader::with_cache(fresh_cache("tamper")).unwrap();
  let err = refused(&loader, &declaration(&reference, Some(&digest)));
  assert_eq!(err.code, "digest-mismatch", "{err:?}");
  assert_eq!(err.source.as_deref(), Some("engine"));
  assert_eq!(err.details.as_ref().unwrap()["expected"], json!(digest));

  // A layer that is not the one the manifest names — caught with no pin at all, because the
  // manifest's descriptors are digests too.
  registry.tamper(Tamper::Blobs);
  let loader = WasmLoader::with_cache(fresh_cache("tamper")).unwrap();
  let err = refused(&loader, &declaration(&reference, None));
  assert_eq!(err.code, "digest-mismatch", "{err:?}");
}

#[test]
fn a_pin_the_registry_has_never_heard_of_is_not_found() {
  let registry = Registry::start();
  push(&registry.host);
  let other = format!("sha256:{}", "0".repeat(64));
  let loader = WasmLoader::with_cache(fresh_cache("unknown")).unwrap();
  let err = refused(
    &loader,
    &declaration(&format!("{}/janus-csv:1.0.0", registry.host), Some(&other)),
  );
  assert_eq!(err.code, "not-found", "{err:?}");
}

#[test]
fn a_reference_and_a_declaration_that_pin_different_digests_are_refused() {
  let registry = Registry::start();
  let digest = push(&registry.host);
  let other = format!("sha256:{}", "0".repeat(64));
  let loader = WasmLoader::with_cache(fresh_cache("two-pins")).unwrap();
  registry.take_log();
  let err = refused(
    &loader,
    &declaration(&format!("{}/janus-csv@{other}", registry.host), Some(&digest)),
  );
  assert_eq!(err.code, "digest-mismatch", "{err:?}");
  assert_eq!(
    registry.take_log().len(),
    0,
    "decided without asking the registry"
  );
}

#[test]
fn a_cache_entry_that_no_longer_matches_its_digest_is_fetched_again_not_used() {
  let registry = Registry::start();
  let digest = push(&registry.host);
  let reference = format!("{}/janus-csv:1.0.0", registry.host);
  let cache = fresh_cache("corrupt");
  load_and_decode(
    &WasmLoader::with_cache(&cache).unwrap(),
    &declaration(&reference, Some(&digest)),
  );

  // Corrupt the cached component: the one blob bigger than a kilobyte.
  let blobs = cache.join("blobs/sha256");
  let layer = std::fs::read_dir(&blobs)
    .unwrap()
    .map(|entry| entry.unwrap().path())
    .find(|path| std::fs::metadata(path).unwrap().len() > 1024)
    .expect("the layer is cached");
  let mut bytes = std::fs::read(&layer).unwrap();
  bytes[100] ^= 0xff;
  std::fs::write(&layer, bytes).unwrap();
  registry.take_log();

  load_and_decode(
    &WasmLoader::with_cache(&cache).unwrap(),
    &declaration(&reference, Some(&digest)),
  );
  let log = registry.take_log();
  assert_eq!(log.len(), 1, "only the corrupted blob is fetched again: {log:#?}");
  assert!(log[0].starts_with("GET /v2/janus-csv/blobs/sha256:"), "{log:#?}");
}

#[test]
fn an_artifact_that_is_not_a_janus_component_is_refused_by_what_it_says_it_is() {
  let registry = Registry::start();
  let config = br#"{"architecture":"amd64","os":"linux"}"#;
  let layer = csv_bytes();
  let manifest = serde_json::to_vec(&json!({
    "schemaVersion": 2,
    "mediaType": "application/vnd.oci.image.manifest.v1+json",
    "config": { "mediaType": "application/vnd.oci.image.config.v1+json",
                "digest": oci::sha256(config), "size": config.len() },
    "layers": [ { "mediaType": "application/wasm", "digest": oci::sha256(&layer), "size": layer.len() } ],
  }))
  .unwrap();
  registry.put("an-image", "1", &manifest, &[config, &layer]);

  let loader = WasmLoader::with_cache(fresh_cache("image")).unwrap();
  let err = refused(
    &loader,
    &declaration(&format!("{}/an-image:1", registry.host), None),
  );
  assert_eq!(err.code, "not-a-component", "{err:?}");
  assert!(
    err.message.contains("application/vnd.oci.image.config.v1+json"),
    "{}",
    err.message
  );
  let log = registry.take_log();
  assert!(
    !log.iter().any(|line| line.contains("/blobs/")),
    "refused on the manifest, before fetching a byte of it: {log:#?}"
  );
}

#[test]
fn an_artifact_whose_config_disagrees_with_the_handshake_is_refused() {
  let registry = Registry::start();
  let reference = Reference::parse(&format!("{}/janus-csv:9.9.9", registry.host)).unwrap();
  // A publisher who typed the config by hand, and typed it wrong.
  Oci::new(fresh_cache("lie"), None)
    .push(
      &reference,
      &csv_bytes(),
      &json!({ "name": "csv", "version": "9.9.9" }),
    )
    .unwrap();

  let loader = WasmLoader::with_cache(fresh_cache("lie")).unwrap();
  let err = refused(&loader, &declaration(&reference.to_string(), None));
  assert_eq!(err.code, "artifact-mismatch", "{err:?}");
  assert!(err.message.contains("9.9.9"), "{}", err.message);
}

#[test]
fn a_registry_that_wants_a_token_is_given_one() {
  let registry = Registry::start();
  registry.require_token();
  let digest = push(&registry.host);
  let loader = WasmLoader::with_cache(fresh_cache("token")).unwrap();
  load_and_decode(
    &loader,
    &declaration(&format!("{}/janus-csv:1.0.0", registry.host), Some(&digest)),
  );
  let log = registry.take_log();
  assert!(log.iter().any(|line| line.starts_with("GET /token?")), "{log:#?}");
}

fn send(engine: &mut Engine, op: &str, body: Value) -> Value {
  let request = json!({ "type": "request", "id": "1", "op": op, "body": body });
  serde_json::from_slice(&engine.dispatch(&serde_json::to_vec(&request).unwrap())).unwrap()
}

/// Through the protocol: a consumer session whose project declares the component by OCI reference
/// and digest, exactly as `components` in `verifier.janus.yaml` or an SDK's options would.
#[test]
fn a_consumer_session_resolves_an_oci_component_through_the_protocol() {
  let registry = Registry::start();
  let digest = push(&registry.host);

  let mut transports: HashMap<String, Arc<dyn TransportComponent>> = HashMap::new();
  transports.insert("http".to_string(), Arc::new(HttpTransport::new()));
  let mut engine = Engine::with_components(transports, Some(Arc::new(JsonContent::new())));
  engine.declare_in_tree("content", "json", "1.0.0");
  engine.register_component_loader(Arc::new(WasmLoader::with_cache(fresh_cache("engine")).unwrap()));
  send(
    &mut engine,
    "engine/hello",
    json!({ "protocol-versions": [1], "host": { "name": "janus-test", "version": "0.0.0" }, "capabilities": {} }),
  );

  let created = send(
    &mut engine,
    "consumer-session/create",
    json!({ "config": { "consumer": { "name": "reporting" }, "provider": { "name": "order-service" },
                        "components": [ { "name": "csv",
                                          "source": { "kind": "oci",
                                                      "reference": format!("{}/janus-csv:1.0.0", registry.host),
                                                      "digest": digest } } ] } }),
  );
  let session = created["ok"]["session"]
    .as_str()
    .unwrap_or_else(|| panic!("create: {created}"));
  let added = send(
    &mut engine,
    "consumer-session/add-interaction",
    json!({ "session": session, "interaction": {
      "description": "every order, as CSV",
      "transport": { "kind": "http", "mode": "passive" },
      "requires": [ { "component": "content/csv", "min-version": 1 } ],
      "content-types": { "response": { "body": "text/csv" } },
      "parts": { "request": { "method": { "shape": "equality", "example": "GET" },
                              "path": { "shape": "equality", "example": "/orders.csv" } },
                 "response": { "status": { "shape": "equality", "example": 200 },
                               "body": { "shape": "each-like", "min": 1, "max": 1,
                                         "items": { "shape": "object", "members": {
                                           "id": { "shape": "string", "example": "66" } } } } } } } }),
  );
  assert!(added["ok"]["handle"].is_string(), "{added}");

  // A pin that is not what the registry holds fails the session, before an interaction is added.
  let tampered = send(
    &mut engine,
    "consumer-session/create",
    json!({ "config": { "consumer": { "name": "c" }, "provider": { "name": "p" },
                        "components": [ { "name": "csv",
                                          "source": { "kind": "oci",
                                                      "reference": format!("{}/janus-csv:1.0.0", registry.host),
                                                      "digest": format!("sha256:{}", "0".repeat(64)) } } ] } }),
  );
  assert_eq!(tampered["error"]["code"], "component-unavailable", "{tampered}");
  assert_eq!(tampered["error"]["details"]["error"]["code"], "not-found");
}

/// The same round trip against a real registry. `JANUS_OCI_REGISTRY=localhost:5000` after
/// `docker run -d -p 5000:5000 registry:2`; CI runs one as a service. Without it this test says
/// so and passes, because the in-process registry above already covered the logic — what this adds
/// is evidence that a registry nobody here wrote agrees.
#[test]
fn against_a_real_registry() {
  let Ok(host) = std::env::var("JANUS_OCI_REGISTRY") else {
    eprintln!("JANUS_OCI_REGISTRY is not set; skipping the real-registry round trip");
    return;
  };
  let reference = format!("{host}/janus-csv:1.0.0");
  let pushed = WasmLoader::with_cache(fresh_cache("real"))
    .unwrap()
    .push(&csv_bytes(), &reference)
    .unwrap_or_else(|err| panic!("{err:?}"));

  let cache = fresh_cache("real");
  let loader = WasmLoader::with_cache(&cache).unwrap();
  let decoded = load_and_decode(&loader, &declaration(&reference, Some(&pushed.digest)));
  assert_eq!(decoded, json!([{ "id": "66", "status": "PENDING" }]));
  let (by_tag, _) = loader.pull(&reference, None).unwrap();
  assert_eq!(
    by_tag.digest, pushed.digest,
    "the registry resolves the tag to what was pushed"
  );
  let by_digest = format!("{host}/janus-csv@{}", pushed.digest);
  load_and_decode(&loader, &declaration(&by_digest, None));
}
