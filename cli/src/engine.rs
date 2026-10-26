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

use pact_janus_kernel::chain;
use pact_janus_kernel::protocol::Engine;
use serde_json::{Value, json};
use std::sync::atomic::{AtomicBool, Ordering};

/// An error the CLI met itself, and its causes, as one sentence: each cause said once, without the
/// platform's `(os error N)`.
pub fn sentence(err: &dyn std::error::Error) -> String {
  chain::joined(&chain::layers_of(err, chain::without_os_error))
}

/// How a command prints what the engine sends: as text for a person, which it colours from the
/// segments the engine marks it with (spec §2.7, ADR 0054), or as the documents themselves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Prints {
  Text,
  Documents,
}

/// A `janus` engine with everything a native host can offer it, past the handshake. It asks for
/// display text's segments only when the command prints text: a document printed for a script has
/// no use for them.
pub fn start(prints: Prints) -> Result<Engine, String> {
  let capabilities = match prints {
    Prints::Text => json!({ "segments": {} }),
    Prints::Documents => json!({}),
  };
  let mut engine = crate::register::native_engine();
  let hello = call(
    &mut engine,
    "engine/hello",
    json!({
      "protocol-versions": [pact_janus_kernel::PROTOCOL_VERSION],
      "host": { "name": "janus-cli", "version": pact_janus_kernel::ENGINE_VERSION },
      "capabilities": capabilities,
    }),
  )?;
  tracing::debug!(%hello, "handshake complete");
  Ok(engine)
}

/// One request frame in, one result document out. An `err` frame becomes this function's `Err`,
/// rendered the way the protocol's own taxonomy reads (spec §10.2): the code first, because that
/// is the part a user can look up, then whatever detail the engine attached.
pub fn call(engine: &mut Engine, op: &str, body: Value) -> Result<Value, String> {
  request(engine, op, body).map_err(|error| render_error(&error))
}

/// [`call`] for an operation handed `documents` as `source.contracts`, read from these files in
/// this order: an error about one of them names its file, and its positions are within that file.
/// The engine can only say "contract 1", since it was never told there were files.
pub fn call_with_files(
  engine: &mut Engine,
  op: &str,
  body: Value,
  files: &[String],
) -> Result<Value, String> {
  request(engine, op, body).map_err(|mut error| match locate(&mut error, files) {
    Some(file) => format!("{file}: {}", render_error(&error)),
    None => render_error(&error),
  })
}

fn request(engine: &mut Engine, op: &str, body: Value) -> Result<Value, Value> {
  let request = json!({ "type": "request", "id": next_id(), "op": op, "body": body });
  let bytes =
    engine.dispatch(&serde_json::to_vec(&request).map_err(|err| json!({ "message": err.to_string() }))?);
  let response: Value =
    serde_json::from_slice(&bytes).map_err(|err| json!({ "message": err.to_string() }))?;
  if let Some(ok) = response.get("ok") {
    return Ok(ok.clone());
  }
  Err(response["error"].clone())
}

/// The file an error is about, from its `details.index` or its problems' `/source/contracts/<i>`
/// pointers, which are rewritten to start within that file.
fn locate<'f>(error: &mut Value, files: &'f [String]) -> Option<&'f str> {
  const PREFIX: &str = "/source/contracts/";
  let index_of = |pointer: &str| -> Option<(usize, usize)> {
    let rest = pointer.strip_prefix(PREFIX)?;
    let digits = rest.find('/').unwrap_or(rest.len());
    Some((rest[..digits].parse().ok()?, PREFIX.len() + digits))
  };
  let index = match error["details"]["index"].as_u64() {
    Some(index) => usize::try_from(index).ok()?,
    None => {
      let pointer = error["details"]["problems"][0]["pointer"].as_str()?;
      index_of(pointer)?.0
    }
  };
  let file = files.get(index)?;
  let problems = error
    .get_mut("details")
    .and_then(|details| details.get_mut("problems"))
    .and_then(Value::as_array_mut);
  for problem in problems.into_iter().flatten() {
    let Some(pointer) = problem["pointer"].as_str() else {
      continue;
    };
    if let Some((_, end)) = index_of(pointer).filter(|(at, _)| *at == index) {
      problem["pointer"] = json!(pointer[end..].to_string());
    }
  }
  Some(file)
}

/// Whether an error is printed with every layer of its chain: the global `--error-chain`.
static FULL_CHAIN: AtomicBool = AtomicBool::new(false);

/// Print every layer of each error's chain from now on (`--error-chain`).
pub fn print_full_chains() {
  FULL_CHAIN.store(true, Ordering::Relaxed);
}

/// An engine error as a line a user can act on: the code first, because that is the part a user
/// can look up, then the message, then each problem at its position.
///
/// An error with a chain (ADR 0054, protocol spec §10.1) leads with its innermost layer that has a
/// code, since that is the most specific thing that failed and the part a user can look up, then
/// says its root cause beneath, when that is another layer. `component-failed`'s own layer only says
/// that a component did. `--error-chain` prints every layer instead. Other `details` are for
/// programs, and are left to `--output json`: every message says in words what a person needs from
/// them.
pub fn render_error(error: &Value) -> String {
  let chain: Vec<(Option<&str>, &str)> = match error["chain"].as_array().filter(|chain| !chain.is_empty()) {
    Some(chain) => chain
      .iter()
      .map(|layer| {
        (
          layer["code"].as_str(),
          layer["message"].as_str().unwrap_or_default(),
        )
      })
      .collect(),
    None => vec![(
      error["code"].as_str(),
      error["message"].as_str().unwrap_or_default(),
    )],
  };
  let said = |(code, message): (Option<&str>, &str)| match (code, message) {
    (Some(code), "") => code.to_string(),
    (Some(code), message) => format!("{code}: {message}"),
    (None, message) => message.to_string(),
  };
  let mut text = if FULL_CHAIN.load(Ordering::Relaxed) {
    let mut layers = chain.iter().copied().map(said);
    let mut text = layers.next().unwrap_or_default();
    for layer in layers {
      text.push_str(&format!("\n  caused by: {layer}"));
    }
    text
  } else {
    let lead = chain.iter().rposition(|(code, _)| code.is_some()).unwrap_or(0);
    let (code, message) = chain[lead];
    let code = code.or(error["code"].as_str()).unwrap_or("internal");
    let mut text = said((Some(code), message));
    if lead + 1 < chain.len() {
      text.push_str(&format!("\n  caused by: {}", chain[chain.len() - 1].1));
    }
    text
  };
  for problem in error["details"]["problems"].as_array().into_iter().flatten() {
    let message = problem["message"].as_str().unwrap_or("");
    // An empty pointer is the whole document, which needs no position.
    match problem["pointer"].as_str().filter(|pointer| !pointer.is_empty()) {
      Some(pointer) => text.push_str(&format!("\n  {pointer}: {message}")),
      None => text.push_str(&format!("\n  {message}")),
    }
  }
  text
}

fn next_id() -> String {
  use std::sync::atomic::{AtomicU64, Ordering};
  static NEXT: AtomicU64 = AtomicU64::new(1);
  format!("r-{}", NEXT.fetch_add(1, Ordering::Relaxed))
}
