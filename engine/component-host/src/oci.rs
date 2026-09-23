//! OCI distribution for WASM components (component-interfaces spec §10.3, plan task 8.2).
//!
//! A component travels as an OCI artifact: an image manifest whose `artifactType` — and, for
//! registries and tools that predate OCI 1.1, whose config media type — says "Janus component", a
//! config blob carrying the name and version the component's own handshake declared, and one
//! `application/wasm` layer carrying the component.
//!
//! This is a deliberately small client for the distribution API: resolve a manifest, fetch two
//! blobs, and — for `janus component push` — upload two blobs and a manifest. Anonymous access, the
//! registry token dance and HTTP Basic are covered; credential helpers and `~/.docker/config.json`
//! are not. What it is careful about is integrity:
//!
//! - **nothing is trusted by name.** Every manifest and blob is hashed on arrival — from the
//!   registry *and* from the cache — and one that does not hash to the digest it was asked for is a
//!   `digest-mismatch`, before a single byte is compiled. A registry's `Docker-Content-Digest`
//!   header is never consulted; the bytes are the evidence.
//! - **a pin is fetched by digest.** With a declared digest the tag is a label and is never resolved,
//!   so a moved tag cannot change what runs, and a second run of the same pin reads the cache and
//!   makes no request at all (spec §10.3 step 3).
//! - **the cache is content-addressed and untrusted.** `blobs/sha256/<hex>`, the same layout an OCI
//!   image-layout directory uses. A cached blob that no longer hashes to its name is discarded and
//!   fetched again, never used.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use pact_janus_kernel::component::ComponentError;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fmt;
use std::io::Write;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

/// The manifest's `artifactType`: what makes a reference a Janus component rather than an image.
pub const ARTIFACT_TYPE: &str = "application/vnd.pact.janus.component.v1";
/// The config blob's media type — also the artifact type, to a reader that predates OCI 1.1's
/// `artifactType` (OCI image spec, "Guidelines for Artifact Usage").
pub const CONFIG_MEDIA_TYPE: &str = "application/vnd.pact.janus.component.config.v1+json";
/// The layer carrying the component: the registered media type for WebAssembly, so any wasm-aware
/// registry UI recognises it.
pub const WASM_MEDIA_TYPE: &str = "application/wasm";
const MANIFEST_MEDIA_TYPE: &str = "application/vnd.oci.image.manifest.v1+json";
const INDEX_MEDIA_TYPE: &str = "application/vnd.oci.image.index.v1+json";
const ACCEPT_MANIFESTS: &str =
  "application/vnd.oci.image.manifest.v1+json, application/vnd.oci.image.index.v1+json";
/// The OCI distribution spec's recommended ceiling on a manifest a registry will accept.
const MAX_MANIFEST_BYTES: u64 = 4 * 1024 * 1024;

/// A parsed artifact reference: `registry/repository[:tag][@sha256:…]`, with an optional `oci://`.
#[derive(Debug, Clone, PartialEq)]
pub struct Reference {
  pub registry: String,
  pub repository: String,
  pub tag: Option<String>,
  pub digest: Option<String>,
}

impl Reference {
  /// Docker's rules for the parts, because they are what every reference a user has ever written
  /// follows: the first segment is a registry when it looks like a host (a `.`, a `:` or
  /// `localhost`), and otherwise the registry is Docker Hub. One departure: there is no implied
  /// `latest`. A reference names a tag or a digest, because a component resolved from a tag nobody
  /// wrote down is a component nobody chose (spec §10.2: declared, never discovered).
  pub fn parse(text: &str) -> Result<Reference, String> {
    let text = text.strip_prefix("oci://").unwrap_or(text);
    let (rest, digest) = match text.split_once('@') {
      Some((rest, digest)) => (rest, Some(check_digest(digest)?)),
      None => (text, None),
    };
    let (name, tag) = match rest.rfind(':') {
      Some(at) if !rest[at..].contains('/') => (&rest[..at], Some(rest[at + 1..].to_string())),
      _ => (rest, None),
    };
    let (registry, repository) = match name.split_once('/') {
      Some((first, repository)) if first.contains(['.', ':']) || first == "localhost" => {
        (first.to_string(), repository.to_string())
      }
      _ if name.contains('/') => ("docker.io".to_string(), name.to_string()),
      _ => ("docker.io".to_string(), format!("library/{name}")),
    };
    let component_ok = |c: &str| {
      !c.is_empty()
        && c
          .chars()
          .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || "._-".contains(ch))
    };
    if repository.is_empty() || !repository.split('/').all(component_ok) {
      return Err(format!(
        "'{text}' has no valid repository: lowercase letters, digits, '.', '_', '-' and '/'"
      ));
    }
    if let Some(tag) = &tag
      && (tag.is_empty()
        || tag.len() > 128
        || tag.starts_with(['.', '-'])
        || !tag
          .chars()
          .all(|ch| ch.is_ascii_alphanumeric() || "._-".contains(ch)))
    {
      return Err(format!("'{tag}' is not a valid tag"));
    }
    if tag.is_none() && digest.is_none() {
      return Err(format!(
        "'{text}' names neither a tag nor a digest; write one (no 'latest' is assumed)"
      ));
    }
    Ok(Reference {
      registry,
      repository,
      tag,
      digest,
    })
  }

  /// The registry's API root. Loopback registries speak plain HTTP — that is what `registry:2` on
  /// a developer's machine or a CI service container does — and everything else HTTPS.
  fn api(&self) -> String {
    let host = if self.registry == "docker.io" {
      "registry-1.docker.io"
    } else {
      &self.registry
    };
    let scheme = if is_loopback(host) { "http" } else { "https" };
    format!("{scheme}://{host}/v2/{}", self.repository)
  }
}

impl fmt::Display for Reference {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}/{}", self.registry, self.repository)?;
    if let Some(tag) = &self.tag {
      write!(f, ":{tag}")?;
    }
    if let Some(digest) = &self.digest {
      write!(f, "@{digest}")?;
    }
    Ok(())
  }
}

fn is_loopback(host: &str) -> bool {
  let bare = match host.strip_prefix('[') {
    Some(v6) => v6.split(']').next().unwrap_or_default(),
    None => host.split(':').next().unwrap_or_default(),
  };
  bare == "localhost" || bare.parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback())
}

/// `sha256:` and 64 lowercase hex digits — the only algorithm v1 accepts, and the one every
/// registry defaults to.
fn check_digest(digest: &str) -> Result<String, String> {
  match digest.strip_prefix("sha256:") {
    Some(hex) if hex.len() == 64 && hex.chars().all(|c| c.is_ascii_hexdigit()) => {
      Ok(digest.to_ascii_lowercase())
    }
    _ => Err(format!(
      "'{digest}' is not a digest this engine checks: 'sha256:' and 64 hex digits"
    )),
  }
}

pub fn sha256(bytes: &[u8]) -> String {
  format!("sha256:{:x}", Sha256::digest(bytes))
}

/// What a pull produced: the manifest digest it resolved to — the value to pin — the config blob,
/// and the component's bytes, every one of them checked.
pub struct Pulled {
  pub reference: Reference,
  pub digest: String,
  pub config: Value,
  pub wasm: Vec<u8>,
}

/// Registry credentials, from `JANUS_OCI_USERNAME` and `JANUS_OCI_PASSWORD` in the environment of
/// whatever runs the loader, and nowhere else. That is an exception to design 2.7 §7.1 — a run
/// should not depend on its host process's environment — and a deliberate one: a credential decides
/// whether bytes can be fetched, never *which* bytes run (the digest decides that), so it changes
/// whether a run starts and never what it does. Putting it in the declaration would put a secret in
/// a document the component-config schema has no redaction rule for.
#[derive(Clone)]
pub struct Credentials {
  pub username: String,
  pub password: String,
}

impl Credentials {
  pub fn from_env() -> Option<Credentials> {
    Some(Credentials {
      username: std::env::var("JANUS_OCI_USERNAME").ok()?,
      password: std::env::var("JANUS_OCI_PASSWORD").ok()?,
    })
  }

  fn basic(&self) -> String {
    format!(
      "Basic {}",
      BASE64.encode(format!("{}:{}", self.username, self.password))
    )
  }
}

/// The client and its cache. One per loader.
pub struct Oci {
  cache: PathBuf,
  agent: ureq::Agent,
  credentials: Option<Credentials>,
  /// The `Authorization` a registry last accepted, per `registry/repository`.
  authorizations: Mutex<HashMap<String, String>>,
}

/// Where the cache lives when nothing says otherwise: `JANUS_COMPONENT_CACHE`, else the platform's
/// user cache directory, else the temporary directory.
pub fn default_cache() -> PathBuf {
  if let Some(dir) = std::env::var_os("JANUS_COMPONENT_CACHE") {
    return PathBuf::from(dir);
  }
  let base = std::env::var_os("XDG_CACHE_HOME")
    .map(PathBuf::from)
    .or_else(|| std::env::var_os("LOCALAPPDATA").map(PathBuf::from))
    .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))
    .unwrap_or_else(std::env::temp_dir);
  base.join("pact-janus").join("components")
}

impl Oci {
  pub fn new(cache: impl Into<PathBuf>, credentials: Option<Credentials>) -> Oci {
    // `http_status_as_error(false)`: a 401 is the start of the token dance and a 404 is an answer,
    // not a transport failure. Redirects are followed — registries hand blobs to a CDN — and ureq
    // does not forward the registry's `Authorization` to wherever they go.
    let agent: ureq::Agent = ureq::Agent::config_builder()
      .http_status_as_error(false)
      .timeout_connect(Some(Duration::from_secs(10)))
      .timeout_global(Some(Duration::from_secs(300)))
      .build()
      .into();
    Oci {
      cache: cache.into(),
      agent,
      credentials,
      authorizations: Mutex::new(HashMap::new()),
    }
  }

  pub fn cache(&self) -> &Path {
    &self.cache
  }

  /// Spec §10.3 steps 1–3: resolve, check, cache. `declared` is the declaration's `digest`; it and a
  /// digest in the reference itself must agree, and whichever is present is the pin.
  pub fn pull(&self, reference: &Reference, declared: Option<&str>) -> Result<Pulled, ComponentError> {
    let declared = declared.map(check_digest).transpose().map_err(|message| {
      failure(
        "invalid-reference",
        message,
        json!({ "reference": reference.to_string() }),
      )
    })?;
    let pin = match (&reference.digest, declared) {
      (Some(inline), Some(declared)) if *inline != declared => {
        return Err(failure(
          "digest-mismatch",
          format!(
            "'{reference}' pins {inline}, and the declaration pins {declared}; a component has one digest"
          ),
          json!({ "reference": reference.to_string(), "expected": declared, "actual": inline }),
        ));
      }
      (inline, declared) => declared.or_else(|| inline.clone()),
    };

    let (digest, manifest) = match pin {
      Some(pin) => {
        let manifest = self.content(reference, &pin, "manifests", MAX_MANIFEST_BYTES)?;
        (pin, manifest)
      }
      None => {
        let tag = reference
          .tag
          .as_deref()
          .expect("a reference has a tag or a digest");
        let manifest = self.fetch(reference, "manifests", tag, MAX_MANIFEST_BYTES)?;
        let digest = sha256(&manifest);
        tracing::info!(%reference, %digest, "resolved a tag; pin this digest for reproducible runs");
        self.store(&digest, &manifest);
        (digest, manifest)
      }
    };

    let (config, layer) = parse_manifest(&manifest, reference, &digest)?;
    let config_bytes = self.content(reference, &config.digest, "blobs", config.size)?;
    check_size(reference, &config, &config_bytes)?;
    let config: Value = serde_json::from_slice(&config_bytes).map_err(|err| {
      failure(
        "not-a-component",
        format!("'{reference}' has a config blob that is not JSON: {err}"),
        json!({ "reference": reference.to_string() }),
      )
    })?;
    let wasm = self.content(reference, &layer.digest, "blobs", layer.size)?;
    check_size(reference, &layer, &wasm)?;
    Ok(Pulled {
      reference: reference.clone(),
      digest,
      config,
      wasm,
    })
  }

  /// Upload a component: its config blob, its layer, and a manifest naming both, tagged as the
  /// reference says. Returns the manifest digest — the value a declaration pins. The manifest is
  /// built from the inputs alone (no timestamp, no host name), so the same component pushed twice
  /// has the same digest.
  pub fn push(&self, reference: &Reference, wasm: &[u8], config: &Value) -> Result<String, ComponentError> {
    let config = serde_json::to_vec(config).map_err(|err| ComponentError::internal(err.to_string()))?;
    let config_digest = self.upload(reference, &config)?;
    let wasm_digest = self.upload(reference, wasm)?;
    let manifest = serde_json::to_vec(&json!({
      "schemaVersion": 2,
      "mediaType": MANIFEST_MEDIA_TYPE,
      "artifactType": ARTIFACT_TYPE,
      "config": { "mediaType": CONFIG_MEDIA_TYPE, "digest": config_digest, "size": config.len() },
      "layers": [ { "mediaType": WASM_MEDIA_TYPE, "digest": wasm_digest, "size": wasm.len() } ],
    }))
    .map_err(|err| ComponentError::internal(err.to_string()))?;
    let digest = sha256(&manifest);
    if let Some(named) = &reference.digest
      && *named != digest
    {
      return Err(failure(
        "digest-mismatch",
        format!("'{reference}' names {named}, and this component's manifest is {digest}"),
        json!({ "reference": reference.to_string(), "expected": named, "actual": digest }),
      ));
    }
    let target = reference.tag.clone().unwrap_or_else(|| digest.clone());
    let url = format!("{}/manifests/{target}", reference.api());
    let response = self.send(
      reference,
      "PUT",
      &url,
      &[("content-type", MANIFEST_MEDIA_TYPE)],
      Some(&manifest),
    )?;
    expect(reference, response, &[201], "pushing the manifest")?;
    Ok(digest)
  }

  /// One blob, monolithically (distribution spec: POST then PUT), unless the registry has it.
  fn upload(&self, reference: &Reference, bytes: &[u8]) -> Result<String, ComponentError> {
    let digest = sha256(bytes);
    let head = self.send(
      reference,
      "HEAD",
      &format!("{}/blobs/{digest}", reference.api()),
      &[],
      None,
    )?;
    if head.status() == 200 {
      return Ok(digest);
    }
    let started = self.send(
      reference,
      "POST",
      &format!("{}/blobs/uploads/", reference.api()),
      &[],
      Some(&[]),
    )?;
    let started = expect(reference, started, &[202], "starting an upload")?;
    let location = started
      .headers()
      .get("location")
      .and_then(|value| value.to_str().ok())
      .ok_or_else(|| {
        unreachable_registry(reference, "the registry started an upload and gave no Location")
      })?;
    let location = absolute(&reference.api(), location);
    let separator = if location.contains('?') { '&' } else { '?' };
    let url = format!("{location}{separator}digest={}", digest.replace(':', "%3A"));
    let finished = self.send(
      reference,
      "PUT",
      &url,
      &[("content-type", "application/octet-stream")],
      Some(bytes),
    )?;
    expect(reference, finished, &[201], "uploading a blob")?;
    Ok(digest)
  }

  /// A manifest or blob by digest: from the cache when it is there and still hashes to its name,
  /// otherwise from the registry — checked, then cached.
  fn content(
    &self,
    reference: &Reference,
    digest: &str,
    kind: &str,
    limit: u64,
  ) -> Result<Vec<u8>, ComponentError> {
    if let Some(bytes) = self.cached(digest) {
      return Ok(bytes);
    }
    let bytes = self.fetch(reference, kind, digest, limit)?;
    let actual = sha256(&bytes);
    if actual != digest {
      return Err(failure(
        "digest-mismatch",
        format!("'{reference}' served {actual} when asked for {digest}; nothing was instantiated"),
        json!({ "reference": reference.to_string(), "expected": digest, "actual": actual }),
      ));
    }
    self.store(digest, &bytes);
    Ok(bytes)
  }

  fn fetch(
    &self,
    reference: &Reference,
    kind: &str,
    target: &str,
    limit: u64,
  ) -> Result<Vec<u8>, ComponentError> {
    let url = format!("{}/{kind}/{target}", reference.api());
    // The index type too, though a component is never one: ghcr.io answers a request that does not
    // accept an index with a 404 for a reference that exists, and "no such thing" sends a reader
    // looking for a typo. Accepted, an index reaches `parse_manifest` and is refused by name.
    let accept = [("accept", ACCEPT_MANIFESTS)];
    let headers: &[(&str, &str)] = if kind == "manifests" { &accept } else { &[] };
    let response = self.send(reference, "GET", &url, headers, None)?;
    let mut response = expect(reference, response, &[200], &format!("fetching {kind}/{target}"))?;
    tracing::debug!(%reference, %target, "fetched from the registry");
    // One byte over the descriptor's size: ureq refuses a body that *reaches* its limit, and a blob
    // exactly its declared size is the one that should arrive. `check_size` then holds it to exact.
    response
      .body_mut()
      .with_config()
      .limit(limit + 1)
      .read_to_vec()
      .map_err(|err| unreachable_registry(reference, &format!("reading {kind}/{target}: {err}")))
  }

  fn blob_path(&self, digest: &str) -> PathBuf {
    let hex = digest.strip_prefix("sha256:").unwrap_or(digest);
    self.cache.join("blobs").join("sha256").join(hex)
  }

  fn cached(&self, digest: &str) -> Option<Vec<u8>> {
    let path = self.blob_path(digest);
    let bytes = std::fs::read(&path).ok()?;
    if sha256(&bytes) == digest {
      tracing::debug!(%digest, "from the component cache");
      return Some(bytes);
    }
    tracing::warn!(%digest, path = %path.display(), "a cached blob no longer matches its digest; discarding it");
    let _ = std::fs::remove_file(&path);
    None
  }

  /// Best effort: a cache that cannot be written (read-only home, full disk) costs the next run a
  /// fetch, and is not a reason to fail this one.
  fn store(&self, digest: &str, bytes: &[u8]) {
    let path = self.blob_path(digest);
    let write = || -> std::io::Result<()> {
      let dir = path.parent().expect("a blob path has a directory");
      std::fs::create_dir_all(dir)?;
      // Unique per writer: two loads of one pin may race to cache it, and each must rename a file
      // it wrote whole.
      static WRITES: AtomicU64 = AtomicU64::new(0);
      let temporary = dir.join(format!(
        ".{}.{}.partial",
        std::process::id(),
        WRITES.fetch_add(1, Ordering::Relaxed)
      ));
      std::fs::File::create(&temporary)?.write_all(bytes)?;
      std::fs::rename(&temporary, &path)
    };
    if let Err(err) = write() {
      tracing::warn!(%digest, cache = %self.cache.display(), error = %err, "could not write the component cache");
    }
  }

  /// One request, with the registry's auth challenge answered once if it makes one.
  fn send(
    &self,
    reference: &Reference,
    method: &str,
    url: &str,
    headers: &[(&str, &str)],
    body: Option<&[u8]>,
  ) -> Result<ureq::http::Response<ureq::Body>, ComponentError> {
    let key = format!("{}/{}", reference.registry, reference.repository);
    let attempt = |authorization: Option<&str>| {
      let mut request = ureq::http::Request::builder().method(method).uri(url);
      for (name, value) in headers {
        request = request.header(*name, *value);
      }
      if let Some(authorization) = authorization {
        request = request.header("authorization", authorization);
      }
      let sent = match body {
        Some(body) => request.body(body).map(|request| self.agent.run(request)),
        None => request.body(()).map(|request| self.agent.run(request)),
      };
      match sent {
        Ok(Ok(response)) => Ok(response),
        Ok(Err(err)) => Err(unreachable_registry(reference, &err.to_string())),
        Err(err) => Err(unreachable_registry(
          reference,
          &format!("building {method} {url}: {err}"),
        )),
      }
    };
    let known = self
      .authorizations
      .lock()
      .expect("authorization lock poisoned")
      .get(&key)
      .cloned();
    let response = attempt(known.as_deref())?;
    if response.status() != 401 {
      return Ok(response);
    }
    let challenge = response
      .headers()
      .get("www-authenticate")
      .and_then(|value| value.to_str().ok())
      .unwrap_or_default()
      .to_string();
    let authorization = self.authorize(reference, &challenge)?;
    let response = attempt(Some(&authorization))?;
    if response.status() != 401 {
      self
        .authorizations
        .lock()
        .expect("authorization lock poisoned")
        .insert(key, authorization);
    }
    Ok(response)
  }

  /// Answer a `WWW-Authenticate` challenge: `Bearer` is the registry token dance (anonymous unless
  /// credentials are set), `Basic` needs credentials.
  fn authorize(&self, reference: &Reference, challenge: &str) -> Result<String, ComponentError> {
    let (scheme, params) = challenge.split_once(' ').unwrap_or((challenge, ""));
    let denied = |why: &str| {
      failure(
        "unauthorized",
        format!("'{reference}': {why}"),
        json!({ "reference": reference.to_string(), "challenge": challenge }),
      )
    };
    if scheme.eq_ignore_ascii_case("basic") {
      return self.credentials.as_ref().map(Credentials::basic).ok_or_else(|| {
        denied("the registry wants credentials; set JANUS_OCI_USERNAME and JANUS_OCI_PASSWORD")
      });
    }
    if !scheme.eq_ignore_ascii_case("bearer") {
      return Err(denied(&format!(
        "the registry asked for '{scheme}' authentication"
      )));
    }
    let params = challenge_params(params);
    let realm = params
      .get("realm")
      .ok_or_else(|| denied("the registry's Bearer challenge names no realm"))?;
    let mut request = self.agent.get(realm);
    if let Some(service) = params.get("service") {
      request = request.query("service", service);
    }
    let scope = params
      .get("scope")
      .cloned()
      .unwrap_or_else(|| format!("repository:{}:pull", reference.repository));
    request = request.query("scope", &scope);
    if let Some(credentials) = &self.credentials {
      request = request.header("authorization", &credentials.basic());
    }
    let mut response = request
      .call()
      .map_err(|err| unreachable_registry(reference, &format!("asking {realm} for a token: {err}")))?;
    if response.status() != 200 {
      return Err(denied(&format!(
        "the token service at {realm} answered {}",
        response.status()
      )));
    }
    let token: Value = response
      .body_mut()
      .with_config()
      .limit(64 * 1024)
      .read_to_vec()
      .map_err(|err| err.to_string())
      .and_then(|bytes| serde_json::from_slice(&bytes).map_err(|err| err.to_string()))
      .map_err(|err| {
        denied(&format!(
          "the token service answered with something other than a token: {err}"
        ))
      })?;
    token
      .get("token")
      .or_else(|| token.get("access_token"))
      .and_then(Value::as_str)
      .map(|token| format!("Bearer {token}"))
      .ok_or_else(|| denied("the token service's answer carries no token"))
  }
}

/// `realm="https://…",service="…",scope="repository:x:pull,push"` — commas inside quotes are part of
/// the value, which is exactly where a push scope puts one.
fn challenge_params(text: &str) -> HashMap<String, String> {
  let mut params = HashMap::new();
  let mut rest = text.trim();
  while let Some((key, after)) = rest.split_once('=') {
    let key = key.trim().trim_start_matches(',').trim().to_ascii_lowercase();
    let (value, remainder) = match after.strip_prefix('"') {
      Some(quoted) => match quoted.split_once('"') {
        Some((value, remainder)) => (value, remainder),
        None => (quoted, ""),
      },
      None => after.split_once(',').unwrap_or((after, "")),
    };
    params.insert(key, value.to_string());
    rest = remainder.trim_start_matches(',').trim();
  }
  params
}

#[derive(Debug)]
struct Descriptor {
  digest: String,
  size: u64,
}

/// A manifest is a Janus component's when it says so: `artifactType`, or — for a reader that
/// predates OCI 1.1 — its config media type. Exactly one layer is the component; any others are
/// someone else's business (open world), and an image index is a question this engine does not
/// ask, because a WASM component has one platform.
fn parse_manifest(
  bytes: &[u8],
  reference: &Reference,
  digest: &str,
) -> Result<(Descriptor, Descriptor), ComponentError> {
  let not_a_component = |why: String| {
    failure(
      "not-a-component",
      format!("'{reference}' ({digest}) is not a Janus component: {why}"),
      json!({ "reference": reference.to_string(), "digest": digest }),
    )
  };
  let manifest: Value = serde_json::from_slice(bytes)
    .map_err(|err| not_a_component(format!("its manifest is not JSON ({err})")))?;
  match manifest.get("mediaType").and_then(Value::as_str) {
    Some(MANIFEST_MEDIA_TYPE) | None => {}
    Some(INDEX_MEDIA_TYPE) => return Err(not_a_component("it is an image index".to_string())),
    Some(other) => return Err(not_a_component(format!("its manifest is '{other}'"))),
  }
  let descriptor = |value: &Value| -> Option<Descriptor> {
    Some(Descriptor {
      digest: check_digest(value.get("digest")?.as_str()?).ok()?,
      size: value.get("size")?.as_u64()?,
    })
  };
  let config = manifest.get("config").unwrap_or(&Value::Null);
  let config_type = config
    .get("mediaType")
    .and_then(Value::as_str)
    .unwrap_or_default();
  let artifact_type = manifest.get("artifactType").and_then(Value::as_str);
  if config_type != CONFIG_MEDIA_TYPE || artifact_type.is_some_and(|t| t != ARTIFACT_TYPE) {
    return Err(not_a_component(format!(
      "its artifact type is '{}'",
      artifact_type.unwrap_or(config_type)
    )));
  }
  let config =
    descriptor(config).ok_or_else(|| not_a_component("its config descriptor is malformed".to_string()))?;
  let wasm: Vec<&Value> = manifest
    .get("layers")
    .and_then(Value::as_array)
    .into_iter()
    .flatten()
    .filter(|layer| layer.get("mediaType").and_then(Value::as_str) == Some(WASM_MEDIA_TYPE))
    .collect();
  let [layer] = wasm.as_slice() else {
    return Err(not_a_component(format!(
      "it has {} '{WASM_MEDIA_TYPE}' layers, and a component is exactly one",
      wasm.len()
    )));
  };
  let layer =
    descriptor(layer).ok_or_else(|| not_a_component("its layer descriptor is malformed".to_string()))?;
  Ok((config, layer))
}

fn check_size(reference: &Reference, descriptor: &Descriptor, bytes: &[u8]) -> Result<(), ComponentError> {
  if bytes.len() as u64 == descriptor.size {
    return Ok(());
  }
  Err(failure(
    "digest-mismatch",
    format!(
      "'{reference}': {} is {} bytes, and its descriptor says {}",
      descriptor.digest,
      bytes.len(),
      descriptor.size
    ),
    json!({ "reference": reference.to_string(), "digest": descriptor.digest }),
  ))
}

fn expect(
  reference: &Reference,
  response: ureq::http::Response<ureq::Body>,
  statuses: &[u16],
  doing: &str,
) -> Result<ureq::http::Response<ureq::Body>, ComponentError> {
  let status = response.status().as_u16();
  if statuses.contains(&status) {
    return Ok(response);
  }
  let mut response = response;
  let body = response
    .body_mut()
    .with_config()
    .limit(4096)
    .read_to_string()
    .unwrap_or_default();
  let (code, why) = match status {
    401 | 403 => ("unauthorized", "the registry refused access"),
    404 => ("not-found", "the registry has no such thing"),
    _ => ("unavailable", "the registry answered"),
  };
  Err(failure(
    code,
    format!("'{reference}': {doing}: {why} ({status}) {}", body.trim()),
    json!({ "reference": reference.to_string(), "status": status }),
  ))
}

fn absolute(api: &str, location: &str) -> String {
  if location.starts_with("http://") || location.starts_with("https://") {
    return location.to_string();
  }
  // `api` is `scheme://host/v2/repo`; a relative Location is relative to the host.
  let origin_end = api
    .find("://")
    .and_then(|at| api[at + 3..].find('/').map(|slash| at + 3 + slash))
    .unwrap_or(api.len());
  format!("{}{location}", &api[..origin_end])
}

fn unreachable_registry(reference: &Reference, why: &str) -> ComponentError {
  failure(
    "unavailable",
    format!("'{reference}': {why}"),
    json!({ "reference": reference.to_string() }),
  )
}

/// A load failure the loader produced, not the component — so `source: engine` (spec §11.2).
fn failure(code: &str, message: String, details: Value) -> ComponentError {
  ComponentError {
    code: code.to_string(),
    category: "component".to_string(),
    message,
    source: Some("engine".into()),
    details: Some(details),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use pretty_assertions::assert_eq;

  const D: &str = "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

  fn reference(registry: &str, repository: &str, tag: Option<&str>, digest: Option<&str>) -> Reference {
    Reference {
      registry: registry.to_string(),
      repository: repository.to_string(),
      tag: tag.map(str::to_string),
      digest: digest.map(str::to_string),
    }
  }

  #[test]
  fn references_parse_the_way_docker_writes_them() {
    assert_eq!(
      Reference::parse("ghcr.io/pact-foundation/janus-csv:1.2.0").unwrap(),
      reference("ghcr.io", "pact-foundation/janus-csv", Some("1.2.0"), None)
    );
    assert_eq!(
      Reference::parse(&format!("oci://localhost:5000/janus-csv@{D}")).unwrap(),
      reference("localhost:5000", "janus-csv", None, Some(D))
    );
    assert_eq!(
      Reference::parse(&format!("127.0.0.1:5000/a/b:v1@{D}")).unwrap(),
      reference("127.0.0.1:5000", "a/b", Some("v1"), Some(D))
    );
    assert_eq!(
      Reference::parse("acme/janus-csv:1").unwrap(),
      reference("docker.io", "acme/janus-csv", Some("1"), None)
    );
    assert_eq!(
      Reference::parse("janus-csv:1").unwrap(),
      reference("docker.io", "library/janus-csv", Some("1"), None)
    );
  }

  #[test]
  fn a_reference_without_a_tag_or_digest_is_refused_rather_than_read_as_latest() {
    let err = Reference::parse("ghcr.io/acme/janus-csv").unwrap_err();
    assert!(err.contains("no 'latest' is assumed"), "{err}");
    assert!(Reference::parse("ghcr.io/acme/janus-csv@sha256:abc").is_err());
    assert!(Reference::parse("ghcr.io/Acme/janus-csv:1").is_err());
  }

  #[test]
  fn loopback_registries_are_plain_http_and_the_rest_are_not() {
    let api = |text: &str| Reference::parse(text).unwrap().api();
    assert_eq!(api("localhost:5000/c:1"), "http://localhost:5000/v2/c");
    assert_eq!(api("127.0.0.1:5000/c:1"), "http://127.0.0.1:5000/v2/c");
    assert_eq!(api("[::1]:5000/c:1"), "http://[::1]:5000/v2/c");
    assert_eq!(api("ghcr.io/acme/c:1"), "https://ghcr.io/v2/acme/c");
    assert_eq!(api("acme/c:1"), "https://registry-1.docker.io/v2/acme/c");
  }

  #[test]
  fn a_push_scope_keeps_its_comma() {
    let params = challenge_params(
      r#"realm="https://auth.example/token",service="registry.example",scope="repository:acme/c:pull,push""#,
    );
    assert_eq!(params["realm"], "https://auth.example/token");
    assert_eq!(params["service"], "registry.example");
    assert_eq!(params["scope"], "repository:acme/c:pull,push");
  }

  #[test]
  fn relative_upload_locations_are_relative_to_the_registry() {
    assert_eq!(
      absolute("http://127.0.0.1:5000/v2/c", "/v2/c/blobs/uploads/abc?_state=x"),
      "http://127.0.0.1:5000/v2/c/blobs/uploads/abc?_state=x"
    );
    assert_eq!(
      absolute("http://127.0.0.1:5000/v2/c", "https://elsewhere/u"),
      "https://elsewhere/u"
    );
  }

  #[test]
  fn only_a_janus_component_manifest_is_a_component() {
    let r = reference("localhost:5000", "c", Some("1"), None);
    let config = json!({ "mediaType": CONFIG_MEDIA_TYPE, "digest": D, "size": 2 });
    let layer = json!({ "mediaType": WASM_MEDIA_TYPE, "digest": D, "size": 3 });
    let parse = |manifest: Value| parse_manifest(&serde_json::to_vec(&manifest).unwrap(), &r, D);

    let (c, l) = parse(json!({ "schemaVersion": 2, "mediaType": MANIFEST_MEDIA_TYPE, "artifactType": ARTIFACT_TYPE,
                               "config": config, "layers": [ { "mediaType": "text/markdown", "digest": D, "size": 9 }, layer ] }))
    .unwrap();
    assert_eq!((c.size, l.size), (2, 3));
    // Pre-1.1: the config media type is the artifact type.
    assert!(parse(json!({ "schemaVersion": 2, "config": config, "layers": [layer] })).is_ok());

    let image = parse(json!({ "schemaVersion": 2, "mediaType": MANIFEST_MEDIA_TYPE,
                              "config": { "mediaType": "application/vnd.oci.image.config.v1+json", "digest": D, "size": 2 },
                              "layers": [layer] }))
    .unwrap_err();
    assert_eq!(image.code, "not-a-component");
    assert!(
      image.message.contains("application/vnd.oci.image.config.v1+json"),
      "{}",
      image.message
    );
    let index =
      parse(json!({ "schemaVersion": 2, "mediaType": INDEX_MEDIA_TYPE, "manifests": [] })).unwrap_err();
    assert!(index.message.contains("image index"), "{}", index.message);
    let two = parse(json!({ "schemaVersion": 2, "config": config, "layers": [layer, layer] })).unwrap_err();
    assert!(
      two.message.contains("2 'application/wasm' layers"),
      "{}",
      two.message
    );
  }
}
