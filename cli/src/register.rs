//! The engine every native Janus binary starts from: `janus` (through [`crate::engine`]) and
//! `janus-engine` (by `#[path]`, since the binaries share no library target) both call
//! [`native_engine`], so the two cannot offer a host different capabilities. They did, until
//! phase-9 finding 30: `janus-engine` registered no hook implementations, so a verification whose
//! provider states came from an `http` hook ran through the CLI and through no SDK.
//!
//! What a native embedding brings is what the kernel deliberately lacks (lifecycle-hooks spec §8.5,
//! ADR 0013): a filesystem, processes and sockets. So it registers the real HTTP transport and JSON
//! content components, the `exec` and `http` hook implementations, the built-in `oauth2` hook
//! component, and the WASM loader for out-of-tree components (plan task 8.1). An engine handed none
//! of those refuses a configuration naming them — by name, before a run starts.

use pact_janus_hooks_host::{ExecHooks, HttpHooks};
use pact_janus_kernel::component::TransportComponent;
use pact_janus_kernel::hooks::HookInvoker;
use pact_janus_kernel::protocol::Engine;
use std::collections::HashMap;
use std::sync::Arc;

/// An engine with everything a native host can offer it, before the handshake. A WASM loader that
/// cannot start leaves the engine with `["in-tree"]`, which `engine/hello` then says — a declared
/// component fails by name rather than the engine failing to start at all.
pub fn native_engine() -> Engine {
  let mut transports: HashMap<String, Arc<dyn TransportComponent>> = HashMap::new();
  transports.insert(
    "http".to_string(),
    Arc::new(pact_janus_component_http::HttpTransport::new()),
  );
  let mut engine = Engine::with_components(
    transports,
    Some(Arc::new(pact_janus_component_json::JsonContent::new())),
  );
  engine.declare_in_tree("content", "json", "1.0.0");
  match pact_janus_component_host::WasmLoader::new() {
    Ok(loader) => engine.register_component_loader(Arc::new(loader)),
    Err(err) => tracing::warn!(error = %err, "the WASM component loader is unavailable"),
  }
  engine.register_hook_invoker("exec", Arc::new(ExecHooks::new()) as Arc<dyn HookInvoker>);
  engine.register_hook_invoker("http", Arc::new(HttpHooks::new()) as Arc<dyn HookInvoker>);
  engine.register_hook_component("oauth2", Arc::new(pact_janus_component_oauth2::Oauth2Hook::new()));
  engine
}
