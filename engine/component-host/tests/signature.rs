//! S5.1c (ADR 0099): a component's Sigstore signature, attached to its manifest as an OCI referrer,
//! verified when a declaration names the signer it trusts — at load and at pull — and recorded with
//! the component. Signed here by a Sigstore made for the test (`support/sigstore.rs`) and verified
//! against its trusted root, declared as a file, with nothing about the verification relaxed; and
//! signed keylessly by `cosign` itself against the public-good instance when CI can
//! (`signed_keylessly_in_ci`).

mod support;

use pact_janus_component_host::WasmLoader;
use pact_janus_component_host::signature::Trust;
use pact_janus_component_http::HttpTransport;
use pact_janus_component_json::JsonContent;
use pact_janus_kernel::component::{
  ComponentDeclaration, ComponentError, ComponentLoader, Decode, Signer, SlotValue, TransportComponent,
};
use pact_janus_kernel::protocol::Engine;
use pretty_assertions::assert_eq;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use support::csv_wasm;
use support::registry::Registry;
use support::sigstore::{self, Signing, Sigstore};

const WORKFLOW: &str = "https://github.com/acme/janus-csv/.github/workflows/release.yml@refs/heads/main";
const GITHUB: &str = "https://token.actions.githubusercontent.com";

fn scratch(label: &str) -> PathBuf {
  static N: AtomicU64 = AtomicU64::new(0);
  let dir = std::env::temp_dir().join(format!(
    "janus-signature-{label}-{}-{}",
    std::process::id(),
    N.fetch_add(1, Ordering::Relaxed)
  ));
  let _ = std::fs::remove_dir_all(&dir);
  std::fs::create_dir_all(&dir).unwrap();
  dir
}

/// The test Sigstore's trusted root, written where a declaration can name it.
fn trusted_root(sigstore: &Sigstore) -> String {
  let path = scratch("root").join("trusted_root.json");
  std::fs::write(&path, sigstore.trusted_root()).unwrap();
  path.display().to_string()
}

/// The CSV component pushed to `repository`; its manifest digest.
fn push(registry: &Registry, repository: &str) -> String {
  let loader = WasmLoader::with_cache(scratch("push")).unwrap();
  loader
    .push(
      &std::fs::read(csv_wasm()).unwrap(),
      &format!("{}/{repository}:1.0.0", registry.host),
      None,
    )
    .unwrap_or_else(|err| panic!("{err:?}"))
    .digest
}

/// Sign `digest` as `signing` says, and attach the bundle to it in `repository`, as `cosign sign` does.
fn sign(registry: &Registry, repository: &str, sigstore: &Sigstore, digest: &str, signing: &Signing<'_>) {
  let bundle = sigstore.sign(digest, signing);
  attach(registry, repository, digest, &bundle);
}

fn attach(registry: &Registry, repository: &str, digest: &str, bundle: &[u8]) {
  let size = registry
    .manifest(repository, digest)
    .map_or(0, |manifest| manifest.len());
  let (manifest, blobs) = sigstore::referrer(digest, size, bundle);
  registry.attach(repository, digest, &manifest, &blobs);
}

fn declaration(
  registry: &Registry,
  repository: &str,
  digest: &str,
  signature: Value,
) -> ComponentDeclaration {
  serde_json::from_value(json!({
    "name": "csv",
    "source": { "kind": "oci", "reference": format!("{}/{repository}:1.0.0", registry.host), "digest": digest,
                "signature": signature },
  }))
  .unwrap()
}

fn trusting(identity: &str, issuer: &str, root: &str) -> Value {
  json!({ "identity": identity, "issuer": issuer, "trusted-root": root })
}

fn refused(loader: &WasmLoader, declaration: &ComponentDeclaration) -> ComponentError {
  match loader.load(declaration) {
    Ok(_) => panic!("{:?} loaded", declaration.source),
    Err(err) => err,
  }
}

/// Load, decode a CSV body to prove the CSV component answers, and say who the loader verified.
fn load(loader: &WasmLoader, declaration: &ComponentDeclaration) -> Option<Signer> {
  let loaded = loader.load(declaration).unwrap_or_else(|err| panic!("{err:?}"));
  let decoded = loaded
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
      embedded: None,
    })
    .unwrap()
    .document
    .to_json();
  assert_eq!(decoded, json!([{ "id": "66", "status": "PENDING" }]));
  loaded.fetched.and_then(|fetched| fetched.signer)
}

fn signer(identity: &str, issuer: &str) -> Signer {
  Signer {
    identity: identity.to_string(),
    issuer: issuer.to_string(),
  }
}

/// The exit, against the test Sigstore: signed, it loads under a declaration naming its signer and is
/// refused under any other, or with its signature removed.
#[test]
fn a_signed_component_loads_under_its_signer_and_under_no_other() {
  let registry = Registry::start();
  let sigstore = Sigstore::new();
  let root = trusted_root(&sigstore);
  let digest = push(&registry, "janus-csv");
  sign(&registry, "janus-csv", &sigstore, &digest, &Signing::default());
  let loader = WasmLoader::with_cache(scratch("cache")).unwrap();

  let verified = load(
    &loader,
    &declaration(&registry, "janus-csv", &digest, trusting(WORKFLOW, GITHUB, &root)),
  );
  assert_eq!(verified, Some(signer(WORKFLOW, GITHUB)));

  // A pattern matching the whole identity is trusted too.
  let pattern = json!({ "identity-pattern": r"https://github\.com/acme/janus-csv/\.github/workflows/release\.yml@refs/.*",
                        "issuer": GITHUB, "trusted-root": root });
  assert_eq!(
    load(&loader, &declaration(&registry, "janus-csv", &digest, pattern)),
    Some(signer(WORKFLOW, GITHUB))
  );

  // Another identity, or the same one vouched for by another issuer, is not who signed.
  let other = "https://github.com/acme/janus-csv/.github/workflows/release.yml@refs/heads/fork";
  for (identity, issuer) in [(other, GITHUB), (WORKFLOW, "https://accounts.google.com")] {
    let err = refused(
      &loader,
      &declaration(&registry, "janus-csv", &digest, trusting(identity, issuer, &root)),
    );
    assert_eq!(err.code, "signature-untrusted", "{err:?}");
    assert_eq!(err.source.as_deref(), Some("engine"));
    assert!(
      err
        .message
        .contains(&format!("signed by '{WORKFLOW}', vouched for by '{GITHUB}'")),
      "{}",
      err.message
    );
  }

  // The same bytes, with no signature attached: pushed to a repository nothing was attached in.
  let unsigned = push(&registry, "janus-csv-unsigned");
  assert_eq!(unsigned, digest, "the same component, the same digest");
  let err = refused(
    &WasmLoader::with_cache(scratch("cache")).unwrap(),
    &declaration(
      &registry,
      "janus-csv-unsigned",
      &digest,
      trusting(WORKFLOW, GITHUB, &root),
    ),
  );
  assert_eq!(err.code, "signature-missing", "{err:?}");
  assert!(
    err.message.contains("has no signature attached"),
    "{}",
    err.message
  );

  // And a declaration naming no signer loads it as before, verifying nothing.
  let declared: ComponentDeclaration = serde_json::from_value(json!({
    "name": "csv", "source": { "kind": "oci", "reference": format!("{}/janus-csv-unsigned:1.0.0", registry.host), "digest": digest } }))
  .unwrap();
  assert_eq!(load(&loader, &declared), None);
}

/// What does not verify is no signature, whoever it names: a signature changed in transit, one from a
/// Sigstore the declaration does not trust, one over another digest, and a statement about the
/// component that is not one of its publication.
#[test]
fn a_bundle_that_does_not_verify_is_no_signature() {
  let registry = Registry::start();
  let sigstore = Sigstore::new();
  let root = trusted_root(&sigstore);
  let digest = push(&registry, "janus-csv");
  let trusted = || declaration(&registry, "janus-csv", &digest, trusting(WORKFLOW, GITHUB, &root));
  let loader = WasmLoader::with_cache(scratch("cache")).unwrap();

  let cases: Vec<(&str, Vec<u8>, &str)> = vec![
    (
      "tampered",
      sigstore::tampered(&sigstore.sign(&digest, &Signing::default())),
      "does not verify",
    ),
    (
      "another Sigstore",
      Sigstore::new().sign(&digest, &Signing::default()),
      "does not verify",
    ),
    (
      "another digest",
      sigstore.sign(&format!("sha256:{}", "0".repeat(64)), &Signing::default()),
      "does not verify",
    ),
    (
      "a scan result",
      sigstore.sign(
        &digest,
        &Signing {
          predicate: "https://cosign.sigstore.dev/attestation/vuln/v1",
          ..Signing::default()
        },
      ),
      "says nothing about who published the component",
    ),
  ];
  for (what, bundle, why) in cases {
    let repository = format!("janus-csv-{}", what.replace(' ', "-").to_lowercase());
    push(&registry, &repository);
    attach(&registry, &repository, &digest, &bundle);
    let err = refused(
      &loader,
      &declaration(&registry, &repository, &digest, trusting(WORKFLOW, GITHUB, &root)),
    );
    assert_eq!(err.code, "signature-untrusted", "{what}: {err:?}");
    assert!(err.message.contains(why), "{what}: {}", err.message);
  }

  // One that verifies, beside one that does not, is enough: nor does a referrer whose bundle is gone
  // from the registry hide it.
  let lost = sigstore.sign(&digest, &Signing::default());
  let (manifest, blobs) = sigstore::referrer(&digest, 0, &lost);
  registry.attach("janus-csv", &digest, &manifest, &blobs[..1]);
  attach(
    &registry,
    "janus-csv",
    &digest,
    &sigstore::tampered(&sigstore.sign(&digest, &Signing::default())),
  );
  sign(&registry, "janus-csv", &sigstore, &digest, &Signing::default());
  assert_eq!(load(&loader, &trusted()), Some(signer(WORKFLOW, GITHUB)));
}

/// `registry:2` has no referrers API: `cosign` then lists a signature in the referrers tag schema's
/// index, `sha256-<hex>`, and the loader finds it there.
#[test]
fn a_signature_is_found_through_the_referrers_tag_where_there_is_no_referrers_api() {
  let registry = Registry::start();
  registry.without_referrers_api();
  let sigstore = Sigstore::new();
  let root = trusted_root(&sigstore);
  let digest = push(&registry, "janus-csv");
  sign(&registry, "janus-csv", &sigstore, &digest, &Signing::default());
  registry.take_log();

  let loader = WasmLoader::with_cache(scratch("cache")).unwrap();
  let verified = load(
    &loader,
    &declaration(&registry, "janus-csv", &digest, trusting(WORKFLOW, GITHUB, &root)),
  );
  assert_eq!(verified, Some(signer(WORKFLOW, GITHUB)));
  let log = registry.take_log();
  let tag = digest.replace(':', "-");
  assert!(
    log.contains(&format!("GET /v2/janus-csv/referrers/{digest}"))
      && log.contains(&format!("GET /v2/janus-csv/manifests/{tag}")),
    "{log:#?}"
  );
}

/// Spec §10.3 step 3 holds for a signed pin: once its signature has verified, a second run reads the
/// component and its signature from the cache, verifies it again, and asks the registry nothing.
#[test]
fn a_signed_pin_verified_once_loads_again_without_a_request() {
  let registry = Registry::start();
  let sigstore = Sigstore::new();
  let root = trusted_root(&sigstore);
  let digest = push(&registry, "janus-csv");
  sign(&registry, "janus-csv", &sigstore, &digest, &Signing::default());
  let cache = scratch("cache");
  let declared = declaration(&registry, "janus-csv", &digest, trusting(WORKFLOW, GITHUB, &root));
  load(&WasmLoader::with_cache(&cache).unwrap(), &declared);
  registry.take_log();

  drop(registry);
  let offline = WasmLoader::with_cache(&cache).unwrap();
  assert_eq!(load(&offline, &declared), Some(signer(WORKFLOW, GITHUB)));

  // The cache decides nothing: a declaration trusting someone else asks the registry, which is gone.
  let other = ComponentDeclaration {
    source: pact_janus_kernel::component::Source {
      signature: Some(serde_json::from_value(trusting("someone-else", GITHUB, &root)).unwrap()),
      ..declared.source.clone()
    },
    ..declared.clone()
  };
  let err = refused(&offline, &other);
  assert_eq!(err.code, "unavailable", "{err:?}");
}

/// A declaration the loader cannot verify against is refused before anything is fetched.
#[test]
fn a_signature_declaration_the_loader_cannot_use_is_refused() {
  let registry = Registry::start();
  let sigstore = Sigstore::new();
  let root = trusted_root(&sigstore);
  let digest = push(&registry, "janus-csv");
  let loader = WasmLoader::with_cache(scratch("cache")).unwrap();
  registry.take_log();

  for (signature, says) in [
    (
      json!({ "issuer": GITHUB, "trusted-root": root }),
      "'identity' or 'identity-pattern'",
    ),
    (
      json!({ "identity": WORKFLOW, "identity-pattern": ".*", "issuer": GITHUB }),
      "not both",
    ),
    (
      json!({ "identity-pattern": "(", "issuer": GITHUB }),
      "not a regular expression",
    ),
    (
      json!({ "identity": WORKFLOW, "issuer": GITHUB, "trusted-root": "/no/such/root.json" }),
      "could not be read",
    ),
  ] {
    let err = refused(&loader, &declaration(&registry, "janus-csv", &digest, signature));
    assert_eq!(err.code, "unavailable", "{err:?}");
    assert!(err.message.contains(says), "{}", err.message);
  }
  assert_eq!(registry.take_log(), Vec::<String>::new(), "nothing was fetched");

  // A file has a digest, and nowhere to find a signature beside it.
  let file: ComponentDeclaration = serde_json::from_value(json!({
    "name": "csv", "source": { "kind": "file", "reference": csv_wasm(), "signature": trusting(WORKFLOW, GITHUB, &root) } }))
  .unwrap();
  let err = refused(&loader, &file);
  assert!(
    err.message.contains("a 'file' source takes no 'signature'"),
    "{}",
    err.message
  );
}

/// `janus component pull` reports who signed, verified against the trusted root, when it names no
/// signer; and holds a pull to one when it does.
#[test]
fn a_pull_reports_every_signer_that_verifies_and_holds_to_one_it_names() {
  let registry = Registry::start();
  let sigstore = Sigstore::new();
  let root = trusted_root(&sigstore);
  let digest = push(&registry, "janus-csv");
  let reference = format!("{}/janus-csv:1.0.0", registry.host);
  let loader = WasmLoader::with_cache(scratch("cache")).unwrap();
  let anyone = Trust::new(None, None, None, Some(&root)).unwrap();

  let (_, _, signers) = loader.pull(&reference, None, None, &anyone).unwrap();
  assert_eq!(signers, Vec::<Signer>::new());

  let maintainer = "maintainer@acme.example";
  sign(&registry, "janus-csv", &sigstore, &digest, &Signing::default());
  sign(
    &registry,
    "janus-csv",
    &sigstore,
    &digest,
    &Signing {
      identity: maintainer,
      issuer: "https://accounts.google.com",
      ..Signing::default()
    },
  );
  attach(
    &registry,
    "janus-csv",
    &digest,
    &Sigstore::new().sign(&digest, &Signing::default()),
  );
  let (_, _, signers) = loader.pull(&reference, None, None, &anyone).unwrap();
  assert_eq!(
    signers,
    vec![
      signer(maintainer, "https://accounts.google.com"),
      signer(WORKFLOW, GITHUB)
    ],
    "both that verify, and not the one from another Sigstore"
  );

  let named = Trust::new(Some(WORKFLOW), None, Some(GITHUB), Some(&root)).unwrap();
  let (_, _, signers) = loader.pull(&reference, Some(&digest), None, &named).unwrap();
  assert_eq!(signers, vec![signer(WORKFLOW, GITHUB)]);
  let someone_else = Trust::new(Some("someone-else"), None, Some(GITHUB), Some(&root)).unwrap();
  let err = loader
    .pull(&reference, Some(&digest), None, &someone_else)
    .err()
    .expect("refused");
  assert_eq!(err.code, "signature-untrusted", "{err:?}");
}

fn send(engine: &mut Engine, op: &str, body: Value) -> Value {
  let request = json!({ "type": "request", "id": "1", "op": op, "body": body });
  serde_json::from_slice(&engine.dispatch(&serde_json::to_vec(&request).unwrap())).unwrap()
}

/// The record S5.1a writes carries the signer the loader verified: a verification run's summary here,
/// and a consumer session's contract by the same `RecordedSource` (`csv_component.rs` holds the
/// contract to that record).
#[test]
fn a_run_records_the_signer_it_verified() {
  let registry = Registry::start();
  let sigstore = Sigstore::new();
  let root = trusted_root(&sigstore);
  let digest = push(&registry, "janus-csv");
  sign(&registry, "janus-csv", &sigstore, &digest, &Signing::default());

  let mut transports: HashMap<String, Arc<dyn TransportComponent>> = HashMap::new();
  transports.insert("http".to_string(), Arc::new(HttpTransport::new()));
  let mut engine = Engine::with_components(transports, Some(Arc::new(JsonContent::new())));
  engine.register_background(Arc::new(pact_janus_kernel::protocol::Threads));
  engine.declare_in_tree("content", "json", "1.0.0");
  engine.register_component_loader(Arc::new(WasmLoader::with_cache(scratch("engine")).unwrap()));
  send(
    &mut engine,
    "engine/hello",
    json!({ "protocol-versions": [1], "host": { "name": "janus-test", "version": "0.0.0" }, "capabilities": {} }),
  );
  let reference = format!("{}/janus-csv:1.0.0", registry.host);
  let verify = send(
    &mut engine,
    "verification/verify",
    json!({ "source": { "kind": "inline", "contracts": [ { "$format": "janus-contract/1",
              "consumer": { "name": "reporting" }, "provider": { "name": "order-service" }, "interactions": [] } ] },
            "target": { "transports": [ { "transport": "http", "options": { "base-url": "http://127.0.0.1:9" } } ],
                        "components": [ { "name": "csv",
                                          "source": { "kind": "oci", "reference": reference, "digest": digest,
                                                      "signature": trusting(WORKFLOW, GITHUB, &root) } } ] } }),
  );
  let stream = verify["ok"]["stream"]
    .as_str()
    .unwrap_or_else(|| panic!("verify: {verify}"))
    .to_string();
  let summary = (0..100)
    .find_map(|_| {
      let polled = send(
        &mut engine,
        "events/poll",
        json!({ "streams": [stream], "wait-ms": 5_000 }),
      );
      polled["ok"]["events"]
        .as_array()?
        .iter()
        .find(|event| event["last"] == json!(true))
        .map(|event| event["payload"].clone())
    })
    .expect("the run finishes");
  assert_eq!(
    summary["components"],
    json!([ { "name": "csv", "version": "1.0.0",
              "source": { "kind": "oci", "reference": reference, "digest": digest,
                          "signer": { "identity": WORKFLOW, "issuer": GITHUB } } } ]),
    "{summary}"
  );

  // A signature from no one trusted fails the run before it starts, naming the component.
  let untrusted = send(
    &mut engine,
    "verification/verify",
    json!({ "source": { "kind": "inline", "contracts": [] },
            "target": { "transports": [ { "transport": "http", "options": { "base-url": "http://127.0.0.1:9" } } ],
                        "components": [ { "name": "csv",
                                          "source": { "kind": "oci", "reference": reference, "digest": digest,
                                                      "signature": trusting("someone-else", GITHUB, &root) } } ] } }),
  );
  assert_eq!(untrusted["error"]["code"], "component-unavailable", "{untrusted}");
  assert_eq!(
    untrusted["error"]["details"]["error"]["code"],
    "signature-untrusted"
  );
}

/// The exit S5.1c names, against the public-good instance: the CSV component pushed to a real
/// registry, signed keylessly by `cosign` with the identity of the CI workflow running this test, and
/// loaded under a declaration naming that workflow — refused under another, and with its signature
/// removed. `JANUS_COSIGN` names the `cosign` to run and `JANUS_OCI_REGISTRY` the registry; CI's Linux
/// job sets both where it can mint an OIDC token, which a fork's pull request cannot. Without them
/// this test says so and passes, since the Sigstore above covers the verification.
#[test]
fn signed_keylessly_in_ci() {
  let (Ok(cosign), Ok(host), Ok(workflow)) = (
    std::env::var("JANUS_COSIGN"),
    std::env::var("JANUS_OCI_REGISTRY"),
    std::env::var("GITHUB_WORKFLOW_REF"),
  ) else {
    eprintln!(
      "JANUS_COSIGN, JANUS_OCI_REGISTRY or GITHUB_WORKFLOW_REF is not set; skipping the keyless signature"
    );
    return;
  };
  // Fulcio names a GitHub workflow by its URL and the ref it ran at.
  let identity = format!("https://github.com/{workflow}");
  let repository = format!("janus-csv-signed-{}", std::process::id());
  let loader = WasmLoader::with_cache(scratch("keyless")).unwrap();
  let pushed = loader
    .push(
      &std::fs::read(csv_wasm()).unwrap(),
      &format!("{host}/{repository}:1.0.0"),
      None,
    )
    .unwrap_or_else(|err| panic!("{err:?}"));
  let signed = std::process::Command::new(cosign)
    .args(["sign", "--yes", "--allow-http-registry"])
    .arg(format!("{host}/{repository}@{}", pushed.digest))
    .output()
    .expect("cosign runs");
  assert!(
    signed.status.success(),
    "cosign sign failed: {}{}",
    String::from_utf8_lossy(&signed.stdout),
    String::from_utf8_lossy(&signed.stderr)
  );

  let declared = |repository: &str, identity: &str| -> ComponentDeclaration {
    serde_json::from_value(json!({
      "name": "csv",
      "source": { "kind": "oci", "reference": format!("{host}/{repository}:1.0.0"), "digest": pushed.digest,
                  "signature": { "identity": identity, "issuer": GITHUB } } }))
    .unwrap()
  };
  assert_eq!(
    load(&loader, &declared(&repository, &identity)),
    Some(signer(&identity, GITHUB))
  );
  let err = refused(
    &WasmLoader::with_cache(scratch("keyless")).unwrap(),
    &declared(
      &repository,
      "https://github.com/acme/janus-csv/.github/workflows/release.yml@refs/heads/main",
    ),
  );
  assert_eq!(err.code, "signature-untrusted", "{err:?}");

  // Its signature removed: the same bytes, in a repository nothing was attached in.
  let unsigned = format!("{repository}-unsigned");
  loader
    .push(
      &std::fs::read(csv_wasm()).unwrap(),
      &format!("{host}/{unsigned}:1.0.0"),
      None,
    )
    .unwrap();
  let err = refused(
    &WasmLoader::with_cache(scratch("keyless")).unwrap(),
    &declared(&unsigned, &identity),
  );
  assert_eq!(err.code, "signature-missing", "{err:?}");
}
