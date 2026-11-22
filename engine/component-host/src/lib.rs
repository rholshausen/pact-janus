//! The WASM component loader (component-interfaces spec §9.2, §10; plan task 8.1).
//!
//! An out-of-tree component is a WASI 0.2 component exporting the frozen pipe world
//! (`Documentation/specs/component-interfaces/wit/component.wit`): one JSON request frame in, one
//! response frame out. This crate loads one from a `file` source or an `oci` one ([`oci`], plan task
//! 8.2), checks its digest and its imports against its grants, handshakes it, and binds the
//! interfaces it declared to the kernel's native traits — so the kernel calls a third-party CSV
//! handler exactly as it calls the in-tree JSON one.
//!
//! It is its own crate, not part of the kernel, because hosting needs a runtime with code
//! generation and the kernel must build for `wasm32-wasip2`, where there is none (ADR 0013). An
//! embedding that can host registers a [`WasmLoader`] with the engine; one that cannot, doesn't,
//! and `engine/hello` says so.
//!
//! The host obligations spec §9.2 makes normative, and where each lives:
//!
//! - **deny by default** — [`WasmComponent::store`] builds a WASI context holding exactly the grants;
//! - **governed imports** — [`check_imports`], at load, before anything is instantiated;
//! - **bounded execution** — an epoch ticker and a per-call deadline;
//! - **loud poisoning** — every call gets a fresh instance (spec §3.4 allows an instance per call,
//!   and requires identical behaviour either way), so a trapped instance is never reused because no
//!   instance ever is.

pub mod oci;
pub mod signature;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use pact_janus_kernel::component::{
  Apply, ApplyResult, Compare, CompareResult, Compile, CompileResult, ComponentDeclaration, ComponentError,
  ComponentLoader, ContentComponent, Decode, DecodeResult, Detect, DetectResult, EmbeddedDecode,
  EmbeddedEncode, Encode, EncodeResult, Fetched, Grants, Label, LabelResult, Loaded, MatcherComponent,
  Prefix, PrefixResult, Signer, SlotValue, media_type_matches,
};
use pact_janus_kernel::persisted::Persisted;
use pact_janus_kernel::plan::RuntimeValue;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread;
use std::time::Duration;
use wasmtime::component::{Component, ComponentExportIndex, InstancePre, Linker, ResourceTable};
use wasmtime::{Config, Engine, Store, Trap};
use wasmtime_wasi::{FsPerms, WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

/// The interface the pipe world exports (`wit/component.wit`). A component built before a rename
/// exports it under an earlier spelling, and still loads (staged plan S0.5).
const PIPE_INTERFACE: Persisted = pact_janus_kernel::persisted::COMPONENT_INTERFACE;
/// Epoch granularity. A deadline is a number of these, so it is exact to within one tick.
const TICK: Duration = Duration::from_millis(10);
/// Spec §3.4: every call has a deadline. A declaration's `limits.deadline-ms` overrides this.
const DEFAULT_DEADLINE_MS: u64 = 10_000;
/// The component protocol versions this host speaks (spec §3.2).
const COMPONENT_PROTOCOL_VERSIONS: &[u64] = &[1];

/// Loads `file` and `oci` sources as WASM components. One per embedding: it owns a wasmtime engine,
/// the thread that ticks its epoch — which stops when the loader is dropped — and the OCI client and
/// its cache.
pub struct WasmLoader {
  engine: Engine,
  ticking: Arc<AtomicBool>,
  oci: oci::Oci,
}

/// What `janus component push` published: the reference, the manifest digest a declaration pins,
/// the config blob it wrote, and the handshake that config was written from.
pub struct Pushed {
  pub reference: String,
  pub digest: String,
  pub config: Value,
  pub hello: Value,
}

impl WasmLoader {
  /// A loader caching OCI components where [`oci::default_cache`] says. A run's registry credential
  /// is its component's declaration's `source.auth`, and nothing else (ADR 0052).
  pub fn new() -> Result<WasmLoader, String> {
    WasmLoader::with_cache(oci::default_cache())
  }

  /// A loader with its own component cache — tests, and a CI job that keeps one per workspace.
  pub fn with_cache(cache: impl Into<PathBuf>) -> Result<WasmLoader, String> {
    let mut config = Config::new();
    config.epoch_interruption(true);
    let engine = Engine::new(&config).map_err(|err| format!("wasmtime could not start: {err}"))?;
    let ticking = Arc::new(AtomicBool::new(true));
    let (ticker, running) = (engine.clone(), Arc::clone(&ticking));
    thread::Builder::new()
      .name("janus-component-epoch".to_string())
      .spawn(move || {
        while running.load(Ordering::Relaxed) {
          thread::sleep(TICK);
          ticker.increment_epoch();
        }
      })
      .map_err(|err| format!("the epoch ticker could not start: {err}"))?;
    Ok(WasmLoader {
      engine,
      ticking,
      oci: oci::Oci::new(cache),
    })
  }

  /// Publish a component as an OCI artifact (spec §10.3). The config blob is written from the
  /// component's own handshake — not from anything the publisher typed — so what the artifact says
  /// it is and what answers when it is loaded cannot drift apart; the loader checks the two agree.
  pub fn push(
    &self,
    wasm: &[u8],
    reference: &str,
    auth: Option<&oci::Credentials>,
  ) -> Result<Pushed, ComponentError> {
    let parsed = oci::Reference::parse(reference).map_err(|message| load_error(&message))?;
    let hello = self.describe(wasm, reference)?;
    let config = artifact_config(&hello);
    let digest = self.oci.push(&parsed, wasm, &config, auth)?;
    Ok(Pushed {
      reference: parsed.to_string(),
      digest,
      config,
      hello,
    })
  }

  /// Fetch a component into the cache, check it, and say what it is — the digest to pin, the
  /// handshake that will answer, and who signed it — without a project or a run. What `janus
  /// component pull` prints. A `trust` that names a signer is held to as a declaration's is
  /// (ADR 0099); one that names none reports every signer whose signature verifies against its
  /// trusted root, which is what a declaration would then name.
  pub fn pull(
    &self,
    reference: &str,
    digest: Option<&str>,
    auth: Option<&oci::Credentials>,
    trust: &signature::Trust,
  ) -> Result<(oci::Pulled, Value, Vec<Signer>), ComponentError> {
    let parsed = oci::Reference::parse(reference).map_err(|message| load_error(&message))?;
    let pulled = self.oci.pull(&parsed, digest, auth)?;
    // Signatures before the handshake, as at load: a signer the trust names is checked before a byte
    // of the component is compiled.
    let signers = if trust.names_a_signer() {
      let attached = self.oci.bundles(&parsed, &pulled.digest, auth)?;
      vec![signed_by(&parsed, &pulled.digest, &attached, trust)?]
    } else {
      // Naming no signer, a pull is not about signatures: one the registry will not list is a
      // warning, and the pull reports no signer.
      let attached = self
        .oci
        .bundles(&parsed, &pulled.digest, auth)
        .unwrap_or_else(|err| {
          tracing::warn!(reference = %parsed, "the component's signatures could not be listed: {}", err.message);
          Vec::new()
        });
      let mut signers: Vec<Signer> = attached
        .iter()
        .filter_map(|bundle| match check(trust, &pulled.digest, bundle) {
          Ok(signer) => Some(signer),
          Err(why) => {
            tracing::info!(reference = %parsed, referrer = %bundle.referrer, "a signature that does not verify: {why}");
            None
          }
        })
        .collect();
      signers.sort_by(|a, b| (&a.issuer, &a.identity).cmp(&(&b.issuer, &b.identity)));
      signers.dedup();
      signers
    };
    let hello = self.describe(&pulled.wasm, reference)?;
    check_artifact_config(&pulled, &hello)?;
    Ok((pulled, hello, signers))
  }

  /// ADR 0099 decision 4: the bundles the cache remembers for this digest first, so a pinned component
  /// whose signature verified loads again with no request; the registry's when none of those is from
  /// the trusted signer — a signature added since, or a declaration that now trusts someone else.
  fn verify_signature(
    &self,
    reference: &oci::Reference,
    digest: &str,
    auth: Option<&oci::Credentials>,
    trust: &signature::Trust,
  ) -> Result<Signer, ComponentError> {
    if let Some(cached) = self.oci.cached_bundles(reference, digest)
      && let Ok(signer) = signed_by(reference, digest, &cached, trust)
    {
      return Ok(signer);
    }
    let attached = self.oci.bundles(reference, digest, auth)?;
    signed_by(reference, digest, &attached, trust)
  }

  /// A component's handshake, under no grants and with no import check — the question is only what
  /// it says it is. Instantiating grants nothing: every import a denied capability would need is
  /// linked to a context that holds none.
  fn describe(&self, bytes: &[u8], what: &str) -> Result<Value, ComponentError> {
    let wasm = self.bind(what, "component", bytes, &Grants::default(), None)?;
    wasm.hello()
  }

  /// The bytes a declaration names, checked: a `file` against its declared digest, an `oci`
  /// reference through [`oci::Oci::pull`], which checks every byte it hands back.
  fn fetch(
    &self,
    declaration: &ComponentDeclaration,
  ) -> Result<(Vec<u8>, String, Fetched, Option<oci::Pulled>), ComponentError> {
    let Some(reference) = declaration.source.reference.as_deref() else {
      return Err(load_error(&format!(
        "a '{}' source needs a 'reference'",
        declaration.source.kind
      )));
    };
    // ADR 0099 decision 3: a declaration that is not one the loader can verify fails before anything
    // is fetched, as a malformed credential does.
    let trust = declaration
      .source
      .signature
      .as_ref()
      .map(signature::Trust::from_declared)
      .transpose()
      .map_err(|message| load_error(&format!("component '{}': {message}", declaration.name)))?;
    if declaration.source.kind == "oci" {
      let parsed = oci::Reference::parse(reference).map_err(|message| load_error(&message))?;
      // ADR 0052: the declaration's credential, and only that. A run reads no environment, so a
      // registry that wants one it was not given says where it belongs.
      let auth = declaration
        .source
        .auth
        .as_ref()
        .map(oci::Credentials::from_declared)
        .transpose()
        .map_err(|message| load_error(&message))?;
      let mut pulled = self
        .oci
        .pull(&parsed, declaration.source.digest.as_deref(), auth.as_ref())
        .map_err(|mut err| {
          if err.code == "unauthorized" && auth.is_none() {
            err
              .message
              .push_str("; the registry wants credentials: declare them as the component source's 'auth'");
          }
          err
        })?;
      tracing::info!(component = %declaration.name, reference = %parsed, digest = %pulled.digest, "component resolved");
      // ADR 0099: after the digest, before a byte is compiled.
      let signer = trust
        .as_ref()
        .map(|trust| self.verify_signature(&parsed, &pulled.digest, auth.as_ref(), trust))
        .transpose()?;
      if let Some(signer) = &signer {
        tracing::info!(component = %declaration.name, identity = %signer.identity, issuer = %signer.issuer, "component signature verified");
      }
      let bytes = std::mem::take(&mut pulled.wasm);
      let fetched = Fetched {
        reference: Some(parsed.without_digest().to_string()),
        digest: pulled.digest.clone(),
        signer,
      };
      return Ok((
        bytes,
        format!("{parsed} ({})", pulled.digest),
        fetched,
        Some(pulled),
      ));
    }

    if declaration.source.auth.is_some() {
      return Err(load_error(&format!(
        "a '{}' source takes no 'auth': a credential is for a registry",
        declaration.source.kind
      )));
    }
    if trust.is_some() {
      return Err(load_error(&format!(
        "a '{}' source takes no 'signature': a signature is found beside an artifact in a registry, and a file is pinned by its 'digest'",
        declaration.source.kind
      )));
    }
    let path = reference;
    let bytes = std::fs::read(path).map_err(|err| load_error(&format!("could not read '{path}': {err}")))?;
    // Spec §10.3 step 2: a declared digest is checked before any bytes are instantiated — or even
    // compiled, since compiling is where a malicious binary gets its first chance. Computed whether
    // or not one is declared, since it is what the run records (ADR 0097 decision 3).
    let actual = oci::sha256(&bytes);
    if let Some(expected) = &declaration.source.digest
      && !actual.eq_ignore_ascii_case(expected)
    {
      return Err(ComponentError {
        code: "digest-mismatch".to_string(),
        category: "component".to_string(),
        message: format!("'{path}' has digest {actual}, and the declaration pins {expected}"),
        source: Some("engine".into()),
        details: Some(Box::new(json!({ "expected": expected, "actual": actual }))),
        chain: None,
      });
    }
    let fetched = Fetched {
      reference: None,
      digest: actual,
      signer: None,
    };
    Ok((bytes, format!("'{path}'"), fetched, None))
  }

  /// Compile and link a component's bytes, ready to instantiate per call.
  fn bind(
    &self,
    what: &str,
    name: &str,
    bytes: &[u8],
    grants: &Grants,
    deadline_ms: Option<u64>,
  ) -> Result<WasmComponent, ComponentError> {
    let component = Component::from_binary(&self.engine, bytes)
      .map_err(|err| load_error(&format!("{what} is not a WASM component: {err:#}")))?;
    let mut linker: Linker<Host> = Linker::new(&self.engine);
    wasmtime_wasi::p2::add_to_linker_sync(&mut linker).map_err(|err| load_error(&format!("{err:#}")))?;
    let pre = linker
      .instantiate_pre(&component)
      .map_err(|err| load_error(&format!("{what} cannot be linked: {err:#}")))?;
    let call = std::iter::once(PIPE_INTERFACE.writes())
      .chain(PIPE_INTERFACE.earlier().iter().copied())
      .find_map(|spelling| {
        component
          .get_export_index(None, spelling)
          .and_then(|interface| component.get_export_index(Some(&interface), "call"))
      })
      .ok_or_else(|| {
        load_error(&format!(
          "{what} does not export {}#call (wit/component.wit)",
          PIPE_INTERFACE.writes()
        ))
      })?;
    Ok(WasmComponent {
      name: name.to_string(),
      component,
      pre,
      call,
      grants: grants.clone(),
      deadline_ticks: deadline_ms
        .unwrap_or(DEFAULT_DEADLINE_MS)
        .div_ceil(TICK.as_millis() as u64),
      media_types: Vec::new(),
      next_id: AtomicU64::new(0),
      engine: self.engine.clone(),
    })
  }
}

fn check(trust: &signature::Trust, digest: &str, bundle: &oci::Attached) -> Result<Signer, String> {
  match &bundle.bytes {
    Ok(bytes) => trust.check(digest, bytes),
    Err(why) => Err(why.clone()),
  }
}

/// The signer `trust` accepts among `attached`, or why there is none (ADR 0099 decision 5):
/// `signature-missing` when nothing is attached, `signature-untrusted` when something is and none of
/// it verifies as from that signer — naming, for each, who it is from or why it does not verify.
fn signed_by(
  reference: &oci::Reference,
  digest: &str,
  attached: &[oci::Attached],
  trust: &signature::Trust,
) -> Result<Signer, ComponentError> {
  let mut refused = Vec::new();
  for bundle in attached {
    match check(trust, digest, bundle) {
      Ok(signer) => return Ok(signer),
      Err(why) => refused.push(json!({ "referrer": bundle.referrer, "reason": why })),
    }
  }
  let wanted = trust.describe();
  let (code, message) = if refused.is_empty() {
    (
      "signature-missing",
      format!("'{reference}' ({digest}) has no signature attached, and one from {wanted} is required"),
    )
  } else {
    let reasons: Vec<&str> = refused.iter().filter_map(|r| r["reason"].as_str()).collect();
    (
      "signature-untrusted",
      format!(
        "'{reference}' ({digest}) has no signature from {wanted}: {}",
        reasons.join("; ")
      ),
    )
  };
  Err(ComponentError {
    code: code.to_string(),
    category: "component".to_string(),
    message,
    source: Some("engine".into()),
    details: Some(Box::new(json!({
      "reference": reference.to_string(), "digest": digest, "signatures": refused,
    }))),
    chain: None,
  })
}

/// The artifact's config blob: the identity the handshake declared, and nothing more (spec §10.3 —
/// "a config blob carrying its name and version"). Not the interfaces, not the contributions: the
/// handshake is the only source of truth about those, and there is no manifest to agree with it
/// (ADR 0012). Name and version are there so a registry, a broker or a person can say which
/// component an artifact is without running it — and they are checked against the handshake on
/// every load, so that saying is never wrong.
fn artifact_config(hello: &Value) -> Value {
  json!({
    "name": hello.pointer("/component/name").cloned().unwrap_or(Value::Null),
    "version": hello.pointer("/component/version").cloned().unwrap_or(Value::Null),
  })
}

/// An artifact whose config names a different component than the one that answers is lying about
/// one of them. The handshake is the source of truth (spec §2.3), so the config is the one that is
/// wrong — but a registry, a broker or a person read the config, and they were told something false.
///
/// An artifact in the CNCF Wasm layout says no name or version, so there is nothing to disagree with
/// and the handshake alone says which component it is (ADR 0098 decision 3).
fn check_artifact_config(pulled: &oci::Pulled, hello: &Value) -> Result<(), ComponentError> {
  if pulled.layout == oci::Layout::Wasm {
    return Ok(());
  }
  let says = artifact_config(hello);
  for member in ["name", "version"] {
    if pulled.config.get(member) != says.get(member) {
      return Err(ComponentError {
        code: "artifact-mismatch".to_string(),
        category: "component".to_string(),
        message: format!(
          "'{}' says its {member} is {}, and the component's handshake says {}",
          pulled.reference,
          pulled.config.get(member).unwrap_or(&Value::Null),
          says[member]
        ),
        source: Some("engine".into()),
        details: Some(Box::new(
          json!({ "reference": pulled.reference.to_string(), "digest": pulled.digest,
                              "config": pulled.config, "handshake": says }),
        )),
        chain: None,
      });
    }
  }
  Ok(())
}

impl Drop for WasmLoader {
  fn drop(&mut self) {
    self.ticking.store(false, Ordering::Relaxed);
  }
}

impl ComponentLoader for WasmLoader {
  fn name(&self) -> &str {
    "wasm"
  }

  fn sources(&self) -> &[&str] {
    &["file", "oci"]
  }

  fn load(&self, declaration: &ComponentDeclaration) -> Result<Loaded, ComponentError> {
    let (bytes, what, fetched, pulled) = self.fetch(declaration)?;
    let mut wasm = self.bind(
      &what,
      &declaration.name,
      &bytes,
      &declaration.grants,
      declaration.limits.deadline_ms,
    )?;
    check_imports(&self.engine, &wasm.component, &declaration.grants)?;
    let hello = wasm.hello()?;
    wasm.media_types = hello
      .pointer("/contributes/content-types")
      .and_then(Value::as_array)
      .into_iter()
      .flatten()
      .filter_map(|entry| {
        entry
          .get("media-type")
          .and_then(Value::as_str)
          .map(str::to_string)
      })
      .collect();
    tracing::debug!(component = %declaration.name, ?hello, "component handshake");
    if let Some(pulled) = &pulled {
      check_artifact_config(pulled, &hello)?;
    }

    let declares = |interface: &str| {
      hello
        .get("interfaces")
        .and_then(Value::as_array)
        .is_some_and(|interfaces| interfaces.iter().any(|i| i == interface))
    };
    let (declares_content, declares_matcher) = (declares("content"), declares("matcher"));
    let wasm = Arc::new(wasm);
    // No transport over this binding yet: a WASM transport needs sockets, which is the grant a
    // sandbox exists to withhold (spec §13), and 8.3's escape hatch is the subprocess binding.
    Ok(Loaded {
      hello,
      content: declares_content.then(|| Arc::clone(&wasm) as Arc<dyn ContentComponent>),
      transport: None,
      matcher: declares_matcher.then_some(wasm as Arc<dyn MatcherComponent>),
      fetched: Some(fetched),
    })
  }
}

/// Spec §9.2's governed imports, per WASI 0.2 package: what `std` brings is linked to nothing (or to
/// exactly the grant), sockets need `network`, and anything else is refused — at load, naming the
/// import, never at first use.
fn check_imports(engine: &Engine, component: &Component, grants: &Grants) -> Result<(), ComponentError> {
  const LINKED: &[&str] = &[
    "wasi:io/",
    "wasi:clocks/",
    "wasi:random/",
    "wasi:cli/",
    "wasi:filesystem/",
  ];
  const NETWORK: &[&str] = &["wasi:sockets/", "wasi:http/"];
  for (name, _) in component.component_type().imports(engine) {
    if LINKED.iter().any(|prefix| name.starts_with(prefix)) {
      continue;
    }
    if NETWORK.iter().any(|prefix| name.starts_with(prefix)) {
      if grants.network {
        continue;
      }
      return Err(ComponentError {
        code: "capability-denied".to_string(),
        category: "component".to_string(),
        message: format!("the component imports {name}, and its declaration does not grant network"),
        source: Some("engine".into()),
        details: Some(Box::new(json!({ "import": name, "grant": "network" }))),
        chain: None,
      });
    }
    return Err(ComponentError {
      code: "capability-denied".to_string(),
      category: "component".to_string(),
      message: format!("the component imports {name}, which no grant provides"),
      source: Some("engine".into()),
      details: Some(Box::new(json!({ "import": name }))),
      chain: None,
    });
  }
  Ok(())
}

/// One loaded component. Holds what every call needs and nothing a call leaves behind.
struct WasmComponent {
  name: String,
  component: Component,
  pre: InstancePre<Host>,
  call: ComponentExportIndex,
  grants: Grants,
  deadline_ticks: u64,
  /// The `media-type` of every `content-types` entry the handshake declared: what
  /// [`ContentComponent::handles`] answers from.
  media_types: Vec<String>,
  next_id: AtomicU64,
  engine: Engine,
}

/// Store data: the WASI context the grants built, and its resource table.
struct Host {
  ctx: WasiCtx,
  table: ResourceTable,
}

impl WasiView for Host {
  fn ctx(&mut self) -> WasiCtxView<'_> {
    WasiCtxView {
      ctx: &mut self.ctx,
      table: &mut self.table,
    }
  }
}

impl WasmComponent {
  /// A WASI context holding exactly the grants (spec §10.4): the variables `env` names, if set; the
  /// directories `fs` names; sockets only with `network`. stdin is empty and stdout/stderr go
  /// nowhere — the pipe is `call`, never a stream.
  fn store(&self) -> Result<Store<Host>, ComponentError> {
    let mut builder = WasiCtxBuilder::new();
    for name in &self.grants.env {
      if let Ok(value) = std::env::var(name) {
        builder.env(name, value);
      }
    }
    for grant in &self.grants.fs {
      let perms = match grant.access.as_deref() {
        Some("read-write") => FsPerms::ReadWrite,
        _ => FsPerms::ReadOnly,
      };
      builder
        .preopened_dir(Path::new(&grant.path), &grant.path, perms)
        .map_err(|err| load_error(&format!("fs grant '{}' cannot be opened: {err:#}", grant.path)))?;
    }
    if self.grants.network {
      builder.inherit_network().allow_ip_name_lookup(true);
    }
    let mut store = Store::new(
      &self.engine,
      Host {
        ctx: builder.build(),
        table: ResourceTable::new(),
      },
    );
    store.set_epoch_deadline(self.deadline_ticks);
    Ok(store)
  }

  /// One instance's life: instantiate, handshake, make one call, drop. `component/hello` precedes
  /// every other operation on a pipe (spec §3.2), and every call here is a new pipe.
  fn instance_call(&self, op: &str, body: Value) -> Result<Value, ComponentError> {
    let mut store = self.store()?;
    let instance = self
      .pre
      .instantiate(&mut store)
      .map_err(|err| self.synthesise(&err, "instantiating the component"))?;
    let call = instance
      .get_typed_func::<(Vec<u8>,), (Vec<u8>,)>(&mut store, self.call)
      .map_err(|err| {
        load_error(&format!(
          "{}#call has the wrong type: {err:#}",
          PIPE_INTERFACE.writes()
        ))
      })?;
    let mut exchange = |op: &str, body: Value| -> Result<Value, ComponentError> {
      let id = self.next_id.fetch_add(1, Ordering::Relaxed).to_string();
      let frame = json!({ "type": "request", "id": id, "op": op, "body": body });
      let request = serde_json::to_vec(&frame).map_err(|err| ComponentError::internal(err.to_string()))?;
      let (response,) = call
        .call(&mut store, (request,))
        .map_err(|err| self.synthesise(&err, op))?;
      read_response(&response, &id, op)
    };
    if op == "component/hello" {
      return exchange(op, body);
    }
    exchange("component/hello", hello_body(&self.grants))?;
    exchange(op, body)
  }

  fn hello(&self) -> Result<Value, ComponentError> {
    self.instance_call("component/hello", hello_body(&self.grants))
  }

  /// Spec §11.2: a trap or an expired deadline is still a failure that reaches the caller as a
  /// value, marked as the engine's, because "the component died" sends a reader somewhere other
  /// than "the component said no".
  fn synthesise(&self, err: &wasmtime::Error, doing: &str) -> ComponentError {
    let (code, what) = match err.downcast_ref::<Trap>() {
      Some(Trap::Interrupt) => ("component-timeout", "did not answer within its deadline"),
      Some(_) => ("component-trapped", "trapped"),
      None => ("component-trapped", "failed"),
    };
    tracing::warn!(component = %self.name, doing, error = %format!("{err:#}"), "component {what}");
    let mut error = ComponentError::synthesised(
      code,
      format!("component '{}' {what} during {doing}: {err:#}", self.name),
    );
    error.details = Some(Box::new(json!({ "component": self.name, "op": doing })));
    error
  }
}

fn hello_body(grants: &Grants) -> Value {
  json!({
    "component-protocol-versions": COMPONENT_PROTOCOL_VERSIONS,
    // No `plan-grammar-versions`: a component writes no plan (ADR 0100 decision 5).
    "engine": { "name": "janus-engine", "version": pact_janus_kernel::ENGINE_VERSION },
    "grants": grants,
    // The engine answers a content component's embedded content (spec §6.6, ADR 0103).
    "capabilities": { "embedded-content": {} },
  })
}

/// A response frame's `ok` document, or its `error` as the component's own error, passed through
/// untranslated (spec §11.2). A frame that is not one is the component's failure, marked as such.
fn read_response(bytes: &[u8], id: &str, op: &str) -> Result<Value, ComponentError> {
  let malformed = |why: &str| {
    ComponentError::synthesised(
      "component-malformed",
      format!("the component answered '{op}' with {why}"),
    )
  };
  let frame: Value =
    serde_json::from_slice(bytes).map_err(|err| malformed(&format!("a frame that is not JSON ({err})")))?;
  if frame.get("type").and_then(Value::as_str) != Some("response") {
    return Err(malformed("a frame that is not a response"));
  }
  if let Some(error) = frame.get("error") {
    return Err(
      serde_json::from_value(error.clone())
        .unwrap_or_else(|_| malformed("an error that is not a ComponentError")),
    );
  }
  let answered = frame.get("id").and_then(Value::as_str).unwrap_or_default();
  if answered != id {
    return Err(malformed(&format!(
      "a response to '{answered}' where '{id}' was asked"
    )));
  }
  frame
    .get("ok")
    .cloned()
    .ok_or_else(|| malformed("a response carrying neither 'ok' nor 'error'"))
}

fn load_error(message: &str) -> ComponentError {
  ComponentError {
    code: "unavailable".to_string(),
    category: "component".to_string(),
    message: message.to_string(),
    source: Some("engine".into()),
    details: None,
    chain: None,
  }
}

/// The content interface over the pipe (spec §6): the kernel's in-memory documents turned into the
/// frames' JSON and back. The document model's bytes travel as `encoded: base64` (ADR 0006).
impl ContentComponent for WasmComponent {
  fn handles(&self, content_type: &str) -> bool {
    self
      .media_types
      .iter()
      .any(|pattern| media_type_matches(pattern, content_type))
  }

  fn decode(&self, req: Decode) -> Result<DecodeResult, ComponentError> {
    let mut body = json!({ "content-type": req.content_type, "value": slot_json(&req.value)? });
    if let Some(options) = req.options {
      body["options"] = options;
    }
    // What the registry made of the content it asked for (spec §6.6, ADR 0103), on the repeated request.
    if let Some(embedded) = req.embedded {
      body["embedded"] = answers_json(embedded, |result| {
        let mut answer = tagged_document(&result.document, "document");
        if !result.degradations.is_empty() {
          answer["degradations"] = Value::Array(result.degradations);
        }
        Ok(answer)
      })?;
    }
    let result = self.instance_call("content/decode", body)?;
    let embedded = match result.get("embedded") {
      None => Vec::new(),
      Some(embedded) => embedded_decodes(embedded)?,
    };
    if !embedded.is_empty() {
      return Ok(DecodeResult::asking(embedded));
    }
    let document = document_of(&result, "document", "decode")?;
    let degradations = result
      .get("degradations")
      .and_then(Value::as_array)
      .cloned()
      .unwrap_or_default();
    if !degradations.is_empty() {
      tracing::debug!(component = %self.name, ?degradations, "decode degradations");
    }
    Ok(DecodeResult {
      document,
      degradations,
      embedded: Vec::new(),
    })
  }

  fn encode(&self, req: Encode) -> Result<EncodeResult, ComponentError> {
    let mut body = tagged_document(&req.document, "document");
    body["content-type"] = json!(req.content_type);
    if let Some(options) = req.options {
      body["options"] = options;
    }
    // The shape it was produced from (spec §6.2, ADR 0101), when there is one.
    if let Some(shape) = req.shape {
      body["shape"] = shape;
    }
    if let Some(embedded) = req.embedded {
      body["embedded"] = answers_json(embedded, |result| {
        Ok(json!({ "value": slot_json(&result.value)? }))
      })?;
    }
    let result = self.instance_call("content/encode", body)?;
    let embedded = match result.get("embedded") {
      None => Vec::new(),
      Some(embedded) => embedded_encodes(embedded)?,
    };
    if !embedded.is_empty() {
      return Ok(EncodeResult::asking(embedded));
    }
    let value = result
      .get("value")
      .cloned()
      .and_then(|value| serde_json::from_value::<SlotValue>(value).ok())
      .ok_or_else(|| ComponentError::synthesised("component-malformed", "encode returned no slot value"))?;
    Ok(EncodeResult::of(value))
  }

  fn compile(&self, req: Compile) -> Result<CompileResult, ComponentError> {
    let result = self.instance_call(
      "content/compile",
      json!({ "content-type": req.content_type, "shape": req.shape, "path": req.path }),
    )?;
    // Success is the whole answer (spec §6.3). A fragment, which a component built before ADR 0100
    // may still return, is not read: say so where someone debugging that component will look.
    if result.get("fragment").is_some() {
      tracing::warn!(component = %self.name, path = %req.path, "the component returned a plan fragment, which this engine does not read (ADR 0100); declare substitutions instead");
    }
    Ok(CompileResult::default())
  }

  fn detect(&self, req: Detect) -> Result<DetectResult, ComponentError> {
    let mut body = json!({ "value": slot_json(&req.value)? });
    if let Some(hint) = req.hint {
      body["hint"] = json!(hint);
    }
    let result = self.instance_call("content/detect", body)?;
    Ok(DetectResult {
      media_type: result
        .get("media-type")
        .and_then(Value::as_str)
        .map(str::to_string),
      confidence: result.get("confidence").and_then(Value::as_f64),
    })
  }

  fn label(&self, req: Label) -> Result<LabelResult, ComponentError> {
    let result = self.instance_call(
      "content/label",
      json!({ "content-type": req.content_type, "value": slot_json(&req.value)?, "at": req.at }),
    )?;
    Ok(LabelResult {
      media_type: result
        .get("media-type")
        .and_then(Value::as_str)
        .map(str::to_string),
    })
  }
}

/// The matcher interface over the pipe: `apply` (spec §7.1), and the two operations the subsumption
/// walk asks (ADR 0102), `compare` (§7.5) and `prefix` (§7.6). An operator's comparability is not asked
/// here: the engine reads it from the handshake, whichever binding loaded the component.
impl MatcherComponent for WasmComponent {
  fn apply(&self, req: Apply) -> Result<ApplyResult, ComponentError> {
    let mut body = json!({ "action": req.action, "values": req.values });
    if let Some(config) = req.config {
      body["config"] = config;
    }
    let result = self.instance_call("matcher/apply", body)?;
    let results = result
      .get("results")
      .and_then(Value::as_array)
      .cloned()
      .ok_or_else(|| ComponentError::synthesised("component-malformed", "apply returned no results"))?;
    Ok(ApplyResult { results })
  }

  fn compare(&self, req: Compare) -> Result<CompareResult, ComponentError> {
    let result = self.instance_call(
      "matcher/compare",
      json!({ "operator": req.operator, "provider": req.provider, "consumer": req.consumer }),
    )?;
    let verdict = result
      .get("verdict")
      .and_then(Value::as_str)
      .ok_or_else(|| ComponentError::synthesised("component-malformed", "compare returned no verdict"))?;
    Ok(CompareResult {
      verdict: verdict.to_string(),
      reason: result.get("reason").and_then(Value::as_str).map(str::to_string),
    })
  }

  fn prefix(&self, req: Prefix) -> Result<PrefixResult, ComponentError> {
    let result = self.instance_call(
      "matcher/prefix",
      json!({ "operator": req.operator, "node": req.node }),
    )?;
    Ok(PrefixResult {
      prefix: result.get("prefix").and_then(Value::as_str).map(str::to_string),
    })
  }
}

fn slot_json(slot: &SlotValue) -> Result<Value, ComponentError> {
  serde_json::to_value(slot).map_err(|err| ComponentError::internal(err.to_string()))
}

/// A document under `member`, tagged as the content schema tags one: octets as `encoded: base64`,
/// anything else as its JSON value.
fn tagged_document(document: &RuntimeValue, member: &str) -> Value {
  let mut body = json!({});
  match document {
    RuntimeValue::Bytes(bytes) => {
      body[member] = json!(BASE64.encode(bytes));
      body["encoded"] = json!("base64");
    }
    document => body[member] = document.to_json(),
  }
  body
}

/// The document under `member` of a frame tagged as [`tagged_document`] writes one, `text` included.
fn document_of(frame: &Value, member: &str, op: &str) -> Result<RuntimeValue, ComponentError> {
  let text = || frame.get(member).and_then(Value::as_str).unwrap_or_default();
  Ok(match frame.get("encoded").and_then(Value::as_str) {
    Some("base64") => RuntimeValue::Bytes(BASE64.decode(text()).map_err(|err| {
      ComponentError::synthesised("component-malformed", format!("{op} returned bad base64: {err}"))
    })?),
    Some("text") => RuntimeValue::String(text().to_string()),
    _ => RuntimeValue::from_json(frame.get(member).unwrap_or(&Value::Null)),
  })
}

/// The registry's answers to what a component asked for, as the repeated request carries them (spec
/// §6.6): by id, each the operation's result or `{ error }`.
fn answers_json<T>(
  answers: pact_janus_kernel::component::Embedded<T>,
  result: impl Fn(T) -> Result<Value, ComponentError>,
) -> Result<Value, ComponentError> {
  let mut json = serde_json::Map::new();
  for (id, answer) in answers {
    let answer = match answer {
      Ok(answer) => result(answer)?,
      Err(error) => json!({ "error": error }),
    };
    json.insert(id, answer);
  }
  Ok(Value::Object(json))
}

fn malformed_embedded(op: &str, why: &str) -> ComponentError {
  ComponentError::synthesised(
    "component-malformed",
    format!("the component asked for embedded content in '{op}' with {why}"),
  )
}

/// `content/decode`'s `embedded`: each `{ id, content-type, value }` the component asks the engine to
/// decode.
fn embedded_decodes(embedded: &Value) -> Result<Vec<EmbeddedDecode>, ComponentError> {
  let entries = embedded
    .as_array()
    .ok_or_else(|| malformed_embedded("content/decode", "a value that is not an array"))?;
  entries
    .iter()
    .map(|entry| {
      let id = entry.get("id").and_then(Value::as_str);
      let content_type = entry.get("content-type").and_then(Value::as_str);
      let value = entry
        .get("value")
        .cloned()
        .and_then(|value| serde_json::from_value::<SlotValue>(value).ok());
      match (id, content_type, value) {
        (Some(id), Some(content_type), Some(value)) => Ok(EmbeddedDecode {
          id: id.to_string(),
          content_type: content_type.to_string(),
          value,
        }),
        _ => Err(malformed_embedded(
          "content/decode",
          "an entry missing its id, content-type or value",
        )),
      }
    })
    .collect()
}

/// `content/encode`'s `embedded`: each `{ id, content-type, document, encoded?, at? }` the component
/// asks the engine to encode.
fn embedded_encodes(embedded: &Value) -> Result<Vec<EmbeddedEncode>, ComponentError> {
  let entries = embedded
    .as_array()
    .ok_or_else(|| malformed_embedded("content/encode", "a value that is not an array"))?;
  entries
    .iter()
    .map(|entry| {
      let id = entry.get("id").and_then(Value::as_str);
      let content_type = entry.get("content-type").and_then(Value::as_str);
      match (id, content_type, entry.get("document")) {
        (Some(id), Some(content_type), Some(_)) => Ok(EmbeddedEncode {
          id: id.to_string(),
          content_type: content_type.to_string(),
          document: document_of(entry, "document", "content/encode")?,
          at: entry.get("at").and_then(Value::as_str).map(str::to_string),
        }),
        _ => Err(malformed_embedded(
          "content/encode",
          "an entry missing its id, content-type or document",
        )),
      }
    })
    .collect()
}

#[cfg(test)]
mod tests {
  use super::*;
  use pretty_assertions::assert_eq;

  /// ADR 0100 decision 5: a component writes no plan, so it is told no grammar.
  #[test]
  fn component_hello_names_no_plan_grammar() {
    let hello = hello_body(&Grants::default());
    assert_eq!(hello.get("plan-grammar-versions"), None, "{hello}");
    assert_eq!(hello["component-protocol-versions"], json!([1]));
  }

  /// ADR 0103 decision 5: the engine says it answers embedded content, so a component may ask.
  #[test]
  fn component_hello_says_the_engine_answers_embedded_content() {
    let hello = hello_body(&Grants::default());
    assert_eq!(hello["capabilities"], json!({ "embedded-content": {} }));
  }
}
