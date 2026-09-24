//! The engine the CLI drives, and how it talks to it.
//!
//! **The CLI is a host like any other.** It does not call the kernel's Rust API: it builds frames,
//! hands them to [`Engine::dispatch`] and reads frames back — the protocol's own pipe 3.3
//! (engine-protocol spec §3.3, "native/in-process: no additional rules"). That is the point of
//! plan task 5.5's "same engine": every byte this CLI sends is a byte an SDK could send, and the
//! `janus-engine` binary beside it carries the identical frames over stdio for the embeddings that
//! need a subprocess (ADR 0023). If a command needed something the protocol cannot express, that
//! would be a finding about the protocol, and it would surface here first.
//!
//! What the CLI *does* bring is capabilities the kernel deliberately lacks, and it brings exactly
//! the ones `janus-engine` does: both start from [`crate::register::native_engine`].

use pact_janus_kernel::protocol::Engine;
use serde_json::{Value, json};

/// A `janus` engine with everything a native host can offer it, past the handshake.
pub fn start() -> Result<Engine, String> {
  let mut engine = crate::register::native_engine();
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
