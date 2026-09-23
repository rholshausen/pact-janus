//! The WASM component loader (component-interfaces spec §9.2, §10; plan task 8.1).
//!
//! An out-of-tree component is a WASI 0.2 component exporting the frozen pipe world
//! (`Documentation/specs/component-interfaces/wit/component.wit`): one JSON request frame in, one
//! response frame out. This crate loads one from a `file` source, checks its digest and its imports
//! against its grants, handshakes it, and binds the interfaces it declared to the kernel's native
//! traits — so the kernel calls a third-party CSV handler exactly as it calls the in-tree JSON one.
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

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use pact_janus_kernel::component::{
  Compile, CompileResult, ComponentDeclaration, ComponentError, ComponentLoader, ContentComponent, Decode,
  DecodeResult, Detect, DetectResult, Encode, EncodeResult, Grants, Loaded, SlotValue, media_type_matches,
};
use pact_janus_kernel::plan::RuntimeValue;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread;
use std::time::Duration;
use wasmtime::component::{Component, ComponentExportIndex, InstancePre, Linker, ResourceTable};
use wasmtime::{Config, Engine, Store, Trap};
use wasmtime_wasi::{DirPerms, FilePerms, WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

/// The interface the pipe world exports (`wit/component.wit`).
const PIPE_INTERFACE: &str = "pact:janus-component/pipe@1.0.0";
/// Epoch granularity. A deadline is a number of these, so it is exact to within one tick.
const TICK: Duration = Duration::from_millis(10);
/// Spec §3.4: every call has a deadline. A declaration's `limits.deadline-ms` overrides this.
const DEFAULT_DEADLINE_MS: u64 = 10_000;
/// The component protocol versions this host speaks (spec §3.2).
const COMPONENT_PROTOCOL_VERSIONS: &[u64] = &[1];

/// Loads `file` sources as WASM components. One per embedding: it owns a wasmtime engine and the
/// thread that ticks its epoch, which stops when the loader is dropped.
pub struct WasmLoader {
  engine: Engine,
  ticking: Arc<AtomicBool>,
}

impl WasmLoader {
  pub fn new() -> Result<WasmLoader, String> {
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
    Ok(WasmLoader { engine, ticking })
  }
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
    &["file"]
  }

  fn load(&self, declaration: &ComponentDeclaration) -> Result<Loaded, ComponentError> {
    let Some(path) = declaration.source.reference.as_deref() else {
      return Err(load_error("a 'file' source needs a 'reference' naming the .wasm"));
    };
    let bytes = std::fs::read(path).map_err(|err| load_error(&format!("could not read '{path}': {err}")))?;

    // Spec §10.3 step 2: a declared digest is checked before any bytes are instantiated — or even
    // compiled, since compiling is where a malicious binary gets its first chance.
    if let Some(expected) = &declaration.source.digest {
      let actual = format!("sha256:{:x}", Sha256::digest(&bytes));
      if !actual.eq_ignore_ascii_case(expected) {
        return Err(ComponentError {
          code: "digest-mismatch".to_string(),
          category: "component".to_string(),
          message: format!("'{path}' has digest {actual}, and the declaration pins {expected}"),
          source: Some("engine".into()),
          details: Some(json!({ "expected": expected, "actual": actual })),
        });
      }
    }

    let component = Component::from_binary(&self.engine, &bytes)
      .map_err(|err| load_error(&format!("'{path}' is not a WASM component: {err:#}")))?;
    check_imports(&self.engine, &component, &declaration.grants)?;

    let mut linker: Linker<Host> = Linker::new(&self.engine);
    wasmtime_wasi::p2::add_to_linker_sync(&mut linker).map_err(|err| load_error(&format!("{err:#}")))?;
    let pre = linker
      .instantiate_pre(&component)
      .map_err(|err| load_error(&format!("'{path}' cannot be linked: {err:#}")))?;
    let call = component
      .get_export_index(None, PIPE_INTERFACE)
      .and_then(|interface| component.get_export_index(Some(&interface), "call"))
      .ok_or_else(|| {
        load_error(&format!(
          "'{path}' does not export {PIPE_INTERFACE}#call (wit/component.wit)"
        ))
      })?;

    let mut wasm = WasmComponent {
      name: declaration.name.clone(),
      pre,
      call,
      grants: declaration.grants.clone(),
      deadline_ticks: declaration
        .limits
        .deadline_ms
        .unwrap_or(DEFAULT_DEADLINE_MS)
        .div_ceil(TICK.as_millis() as u64),
      media_types: Vec::new(),
      next_id: AtomicU64::new(0),
      engine: self.engine.clone(),
    };
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

    let declares_content = hello
      .get("interfaces")
      .and_then(Value::as_array)
      .is_some_and(|interfaces| interfaces.iter().any(|i| i == "content"));
    let wasm = Arc::new(wasm);
    Ok(Loaded {
      hello,
      content: declares_content.then_some(wasm as Arc<dyn ContentComponent>),
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
        details: Some(json!({ "import": name, "grant": "network" })),
      });
    }
    return Err(ComponentError {
      code: "capability-denied".to_string(),
      category: "component".to_string(),
      message: format!("the component imports {name}, which no grant provides"),
      source: Some("engine".into()),
      details: Some(json!({ "import": name })),
    });
  }
  Ok(())
}

/// One loaded component. Holds what every call needs and nothing a call leaves behind.
struct WasmComponent {
  name: String,
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
      let (dirs, files) = match grant.access.as_deref() {
        Some("read-write") => (DirPerms::all(), FilePerms::all()),
        _ => (DirPerms::READ, FilePerms::READ),
      };
      builder
        .preopened_dir(Path::new(&grant.path), &grant.path, dirs, files)
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
      .map_err(|err| load_error(&format!("{PIPE_INTERFACE}#call has the wrong type: {err:#}")))?;
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
    error.details = Some(json!({ "component": self.name, "op": doing }));
    error
  }
}

fn hello_body(grants: &Grants) -> Value {
  json!({
    "component-protocol-versions": COMPONENT_PROTOCOL_VERSIONS,
    "engine": { "name": "janus-engine", "version": pact_janus_kernel::ENGINE_VERSION },
    "grants": grants,
    "capabilities": {},
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
    let result = self.instance_call("content/decode", body)?;
    let document = match result.get("encoded").and_then(Value::as_str) {
      Some("base64") => {
        let text = result.get("document").and_then(Value::as_str).unwrap_or_default();
        RuntimeValue::Bytes(BASE64.decode(text).map_err(|err| {
          ComponentError::synthesised(
            "component-malformed",
            format!("decode returned bad base64: {err}"),
          )
        })?)
      }
      Some("text") => RuntimeValue::String(
        result
          .get("document")
          .and_then(Value::as_str)
          .unwrap_or_default()
          .to_string(),
      ),
      _ => RuntimeValue::from_json(result.get("document").unwrap_or(&Value::Null)),
    };
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
    })
  }

  fn encode(&self, req: Encode) -> Result<EncodeResult, ComponentError> {
    let mut body = match &req.document {
      RuntimeValue::Bytes(bytes) => {
        json!({ "content-type": req.content_type, "document": BASE64.encode(bytes), "encoded": "base64" })
      }
      document => json!({ "content-type": req.content_type, "document": document.to_json() }),
    };
    if let Some(options) = req.options {
      body["options"] = options;
    }
    let result = self.instance_call("content/encode", body)?;
    let value = result
      .get("value")
      .cloned()
      .and_then(|value| serde_json::from_value::<SlotValue>(value).ok())
      .ok_or_else(|| ComponentError::synthesised("component-malformed", "encode returned no slot value"))?;
    Ok(EncodeResult { value })
  }

  fn compile(&self, req: Compile) -> Result<CompileResult, ComponentError> {
    let result = self.instance_call(
      "content/compile",
      json!({ "content-type": req.content_type, "shape": req.shape, "path": req.path }),
    )?;
    Ok(CompileResult {
      fragment: result.get("fragment").cloned(),
      grammar_version: result
        .get("grammar-version")
        .and_then(Value::as_str)
        .map(str::to_string),
    })
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
}

fn slot_json(slot: &SlotValue) -> Result<Value, ComponentError> {
  serde_json::to_value(slot).map_err(|err| ComponentError::internal(err.to_string()))
}
