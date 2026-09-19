//! The live passive exchange (plan task 4.5, variant-semantics spec §4.1–§4.2): what actually
//! drives a transport once `start-transport` binds one to a session. Runs on its own thread per
//! transport instance — `poll-inbound` blocks up to its own timeout, and the whole point is that
//! this loop is what a real inbound request waits on, so it cannot be the same thread that answers
//! `consumer-session/*` frames.
//!
//! What this closes: `serve-variant` (plan task 4.3) already computed the concrete payload a
//! variant should produce; nothing before this task ever put it on the wire, or looked at what
//! came back. What it does not attempt: routing one inbound request across *several* concurrently
//! armed interactions (spec §4.1 allows this) — one transport instance here holds at most one
//! armed exchange at a time, which is exactly what the sequential `serve-variant` loop a consumer
//! test runs actually needs; broader routing is for whoever exercises that concurrency for real.
//!
//! **A WASM-component caveat for whoever builds that embedding next** (ADR 0003, ADR 0013): this
//! module builds cleanly for `wasm32-wasip2` (`std::thread`/`Mutex`/`AtomicBool` all compile
//! there), but `thread::spawn` has nothing to schedule onto without the threads proposal, so
//! `start-transport` would fail at run time on a plain wasip2 host. Every embedding this task
//! actually wires up (the CLI, the subprocess binary, the Rust/TS integration tests) is native, so
//! this is a documented gap, not a silent one: the WASM-component embedding will need either
//! wasi-threads or a single-threaded poll model here, not this one unmodified.

use super::session::{ExchangeOutcome, Exercised};
use super::wire::{encode_slot, find_container, mismatch_json, parts_resolver, plain_slot};
use crate::component::{
  ContentComponent, Dispose, Inbound, Part, Parts, PollInbound, Reply, TransportComponent,
};
use crate::plan::{Mismatch, Plan, Status, execute, outcome};
use serde_json::Value;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// What `serve-variant` arms for a passive HTTP interaction (spec §4.1, §8.2): everything the
/// background loop needs to match the next inbound request and answer it, with no further lookup
/// into session state — the loop runs on its own thread and this is the whole handoff.
pub(crate) struct ArmedExchange {
  pub handle: String,
  pub variant_id: String,
  /// The interaction's plan, pinned to this variant's assignment (plan-grammar spec §5.1); only
  /// its `request` subtree is read here.
  pub request_plan: Plan,
  /// The response [`crate::variant::generate::interaction`] produced for this variant — replayed
  /// verbatim on a match, the consumer side's own mirror of variant-semantics spec §5.2's
  /// replay-by-example.
  pub response_parts: BTreeMap<String, BTreeMap<String, Value>>,
}

/// Shared between `serve_variant` (arms) and this module's background loop (matches, replies,
/// records) for one transport instance. A mutex rather than a channel: both sides need to read
/// back what the other last did — arming replaces any previous arming, and the loop clears it once
/// consumed — which a one-way channel does not give without a second one back the other way.
#[derive(Default)]
pub(crate) struct ExchangeState {
  pub armed: Option<ArmedExchange>,
  /// `(handle, variant id) -> evidence`, drained into the session's own per-interaction map when
  /// the transport stops ([`super::session::ConsumerSession::stop_transports`]) — this loop only
  /// ever inserts into it, one entry per exchange.
  pub outcomes: BTreeMap<(String, String), Exercised>,
}

const POLL_TIMEOUT_MS: u64 = 200;

/// Poll `instance` until `stop` is set or the transport reports it gone (which is what its own
/// `stop` looks like from here, once `ConsumerSession::stop_transports` has called it) — never
/// erroring outward, since there is no dispatch frame left to carry an error to by the time this
/// runs: whatever a poll or reply attempt fails at, the loop simply moves on or ends.
pub(crate) fn run(
  transport: Arc<dyn TransportComponent>,
  content: Option<Arc<dyn ContentComponent>>,
  instance: String,
  stop: Arc<AtomicBool>,
  state: Arc<Mutex<ExchangeState>>,
) {
  while !stop.load(Ordering::Relaxed) {
    let polled = transport.poll_inbound(PollInbound {
      instance: instance.clone(),
      timeout_ms: POLL_TIMEOUT_MS,
    });
    let inbound = match polled {
      Ok(result) => match result.inbound {
        Some(inbound) => inbound,
        None => continue, // the poll simply timed out with nothing waiting
      },
      Err(_) => break, // the instance is gone (stopped) or the transport itself failed
    };
    handle_inbound(transport.as_ref(), content.as_deref(), &instance, inbound, &state);
  }
}

/// One arrival: match it against whatever is currently armed, reply, and record the outcome.
/// Nothing is armed → this exchange proves nothing about any variant, so it is refused rather than
/// guessed at (variant-semantics spec §4.2's honesty applies here too, not only to recording).
fn handle_inbound(
  transport: &dyn TransportComponent,
  content: Option<&dyn ContentComponent>,
  instance: &str,
  inbound: Inbound,
  state: &Mutex<ExchangeState>,
) {
  let armed = state.lock().expect("exchange state lock poisoned").armed.take();
  let Some(armed) = armed else {
    // Plan task 4.6's own finding: this is the one line that names an arrival at all when
    // nothing was armed for it — without it, a request that missed its `serve-variant` window
    // entirely (a race, a typo'd variant id upstream) leaves no trace anywhere.
    tracing::warn!(instance, event = %inbound.event, "inbound request arrived with nothing armed; answering unmatched");
    let _ = transport.dispose(Dispose {
      instance: instance.to_string(),
      event: inbound.event,
      disposition: "unmatched".to_string(),
    });
    return;
  };

  let resolver = parts_resolver(&inbound.parts, content);
  let executed = execute(&armed.request_plan, &resolver);
  let request_subtree = find_container(&executed, "request").unwrap_or(&executed);
  let (status, mismatches) = outcome(request_subtree);

  // The one line plan task 4.6's report leans on: `RUST_LOG=debug` names the variant a failure
  // belongs to, which nothing else on this loop's own thread does — a consumer's own crash
  // handling this response has no way to ask the engine "which variant was that" after the fact.
  match status {
    Status::Matched => {
      tracing::debug!(instance, handle = %armed.handle, variant = %armed.variant_id, "request matched the armed variant")
    }
    Status::Mismatched => {
      tracing::warn!(instance, handle = %armed.handle, variant = %armed.variant_id, ?mismatches, "request did not match the armed variant")
    }
  }

  let reply_parts = match status {
    Status::Matched => response_parts(&armed.response_parts, content),
    Status::Mismatched => mismatch_reply(&mismatches, content),
  };
  let delivered = transport.reply(Reply {
    instance: instance.to_string(),
    event: inbound.event,
    parts: reply_parts,
  });

  // A variant is verified only if the consumer actually got the response it was armed with: a
  // request that matched but whose reply never reached the client (the client gave up, or the
  // connection was unusable) exercised nothing the consumer could have handled.
  let (exchange_outcome, recorded) = match (status, delivered) {
    (Status::Matched, Ok(_)) => (ExchangeOutcome::Verified, Vec::new()),
    (Status::Matched, Err(err)) => {
      tracing::warn!(instance, handle = %armed.handle, variant = %armed.variant_id, error = %err.message, "the matched request's response could not be delivered");
      (
        ExchangeOutcome::Failed,
        vec![serde_json::json!({
          "message": format!("the request matched, but its response could not be delivered to the consumer: {}", err.message),
          "action": "transport:reply",
        })],
      )
    }
    (Status::Mismatched, _) => (
      ExchangeOutcome::Failed,
      mismatches.iter().map(mismatch_json).collect(),
    ),
  };
  state
    .lock()
    .expect("exchange state lock poisoned")
    .outcomes
    .insert(
      (armed.handle, armed.variant_id),
      Exercised {
        outcome: exchange_outcome,
        parts: armed.response_parts,
        mismatches: recorded,
      },
    );
}

/// The generated `response` payload, wired for reply (component-interfaces spec §4).
fn response_parts(
  response: &BTreeMap<String, BTreeMap<String, Value>>,
  content: Option<&dyn ContentComponent>,
) -> Parts {
  let mut parts = Parts::new();
  if let Some(slots) = response.get("response") {
    let mut part = Part::new();
    for (slot_name, value) in slots {
      part.insert(slot_name.clone(), encode_slot(value, content));
    }
    parts.insert("response".to_string(), part);
  }
  parts
}

/// The reply sent when the recorded request didn't match what was armed: a `500` naming every
/// mismatch, so the consumer's own HTTP client sees a clear failure rather than a hang or the
/// response it did not earn. Refining this into a real "why your request didn't match" experience
/// is plan task 4.6's job, not this wiring's.
fn mismatch_reply(mismatches: &[Mismatch], content: Option<&dyn ContentComponent>) -> Parts {
  let body = Value::Array(mismatches.iter().map(mismatch_json).collect());
  let mut part = Part::new();
  part.insert("status".to_string(), plain_slot(&Value::from(500)));
  part.insert("body".to_string(), encode_slot(&body, content));
  let mut parts = Parts::new();
  parts.insert("response".to_string(), part);
  parts
}
