//! The engine the CLI drives, and how it talks to it.
//!
//! **The CLI is a host like any other.** It does not call the kernel's Rust API: it builds frames,
//! hands them to [`Engine::dispatch`] and reads frames back — the protocol's own pipe 3.3
//! (engine-protocol spec §3.3, "native/in-process: no additional rules"). That is the point of
//! plan task 5.5's "same engine": every byte this CLI sends is a byte an SDK could send, and the
//! `janus-engine` binary beside it carries the identical frames over stdio for the embeddings that
//! need a subprocess (ADR 0003). If a command needed something the protocol cannot express, that
//! would be a finding about the protocol, and it would surface here first.
//!
//! What the CLI *does* bring is capabilities the kernel deliberately lacks (lifecycle-hooks spec
//! §8.5, ADR 0013): a filesystem, processes and sockets. So it registers the real HTTP transport
//! and JSON content components, the `exec` and `http` hook implementations, and the built-in
//! `oauth2` hook component, and the WASM loader for out-of-tree components (plan task 8.1). An
//! engine handed none of those refuses a configuration naming them — by name, before a run starts
//! — which is the behaviour a degraded run would hide.

use pact_janus_hooks_host::{ExecHooks, HttpHooks};
use pact_janus_kernel::component::TransportComponent;
use pact_janus_kernel::hooks::HookInvoker;
use pact_janus_kernel::protocol::Engine;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;

/// A `janus` engine with everything a native host can offer it, past the handshake.
pub fn start() -> Result<Engine, String> {
  let mut transports: HashMap<String, Arc<dyn TransportComponent>> = HashMap::new();
  transports.insert(
    "http".to_string(),
    Arc::new(pact_janus_component_http::HttpTransport::new()),
  );
  let mut engine = Engine::with_components(
    transports,
    Some(Arc::new(pact_janus_component_json::JsonContent::new())),
  );
  register_components(&mut engine);
  engine.register_hook_invoker("exec", Arc::new(ExecHooks::new()) as Arc<dyn HookInvoker>);
  engine.register_hook_invoker("http", Arc::new(HttpHooks::new()) as Arc<dyn HookInvoker>);
  engine.register_hook_component("oauth2", Arc::new(pact_janus_component_oauth2::Oauth2Hook::new()));

  let hello = call(
    &mut engine,
    "engine/hello",
    json!({
      "protocol-versions": [pact_janus_kernel::PROTOCOL_VERSION],
      "host": { "name": "janus-cli", "version": pact_janus_kernel::ENGINE_VERSION },
      "capabilities": {},
    }),
  )?;
  tracing::debug!(?hello, "handshake complete");
  Ok(engine)
}

/// What `janus` and `janus-engine` both give the engine beyond its transports: the in-tree content
/// component's name, and the WASM loader for the components a project declares (component-interfaces
/// spec §10, plan task 8.1). A loader that cannot start leaves the engine with `["in-tree"]`, which
/// `engine/hello` then says — a declared component fails by name rather than the engine failing to
/// start at all.
pub fn register_components(engine: &mut Engine) {
  engine.declare_in_tree("content", "json", "1.0.0");
  match pact_janus_component_host::WasmLoader::new() {
    Ok(loader) => engine.register_component_loader(Arc::new(loader)),
    Err(err) => tracing::warn!(error = %err, "the WASM component loader is unavailable"),
  }
}

/// One request frame in, one result document out. An `err` frame becomes this function's `Err`,
/// rendered the way the protocol's own taxonomy reads (spec §10.2): the code first, because that
/// is the part a user can look up, then whatever detail the engine attached.
pub fn call(engine: &mut Engine, op: &str, body: Value) -> Result<Value, String> {
  let request = json!({ "type": "request", "id": next_id(), "op": op, "body": body });
  let bytes = engine.dispatch(&serde_json::to_vec(&request).map_err(|err| err.to_string())?);
  let response: Value = serde_json::from_slice(&bytes).map_err(|err| err.to_string())?;
  if let Some(ok) = response.get("ok") {
    return Ok(ok.clone());
  }
  Err(render_error(&response["error"]))
}

/// An engine error as a line a user can act on.
pub fn render_error(error: &Value) -> String {
  let code = error["code"].as_str().unwrap_or("internal");
  let mut text = match error["message"].as_str() {
    Some(message) => format!("{code}: {message}"),
    None => code.to_string(),
  };
  if let Some(problems) = error["details"]["problems"].as_array() {
    for problem in problems {
      let pointer = problem["pointer"].as_str().unwrap_or("");
      let message = problem["message"].as_str().unwrap_or("");
      text.push_str(&format!("\n  {pointer}: {message}"));
    }
  } else if let Some(details) = error.get("details").filter(|d| !d.is_null()) {
    text.push_str(&format!("\n  {details}"));
  }
  text
}

fn next_id() -> String {
  use std::sync::atomic::{AtomicU64, Ordering};
  static NEXT: AtomicU64 = AtomicU64::new(1);
  format!("r-{}", NEXT.fetch_add(1, Ordering::Relaxed))
}
