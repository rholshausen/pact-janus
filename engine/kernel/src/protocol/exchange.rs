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
use crate::component::{
  ContentComponent, Decode, Dispose, Encode, Inbound, Part, Parts, PollInbound, Reply, SlotValue,
  TransportComponent,
};
use crate::plan::{
  CapturedValues, Executed, ExecutedKind, Mismatch, Plan, RuntimeValue, Status, execute, outcome,
};
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
    let _ = transport.dispose(Dispose {
      instance: instance.to_string(),
      event: inbound.event,
      disposition: "unmatched".to_string(),
    });
    return;
  };

  let resolver = request_resolver(&inbound.parts, content);
  let executed = execute(&armed.request_plan, &resolver);
  let request_subtree = find_container(&executed, "request").unwrap_or(&executed);
  let (status, mismatches) = outcome(request_subtree);

  let reply_parts = match status {
    Status::Matched => response_parts(&armed.response_parts, content),
    Status::Mismatched => mismatch_reply(&mismatches, content),
  };
  let _ = transport.reply(Reply {
    instance: instance.to_string(),
    event: inbound.event,
    parts: reply_parts,
  });

  let exchange_outcome = match status {
    Status::Matched => ExchangeOutcome::Verified,
    Status::Mismatched => ExchangeOutcome::Failed,
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
      },
    );
}

/// The plan-executed tree's named part subtree (`"request"`/`"response"`) — the root
/// [`execute`](crate::plan::execute) produces is always a container of part containers
/// (`plan::compile`'s own shape), so this is a one-level lookup, not a general tree search.
fn find_container<'a>(executed: &'a Executed, label: &str) -> Option<&'a Executed> {
  let ExecutedKind::Container { children, .. } = &executed.kind else {
    return None;
  };
  children
    .iter()
    .find(|child| matches!(&child.kind, ExecutedKind::Container { label: Some(l), .. } if l == label))
}

/// A resolver over an inbound arrival's wire-form parts (component-interfaces spec §4), decoded to
/// the document model a plan resolves against — the same `$.<part>.<slot>` paths `plan::compile`
/// roots every slot at (shape spec §6.2).
fn request_resolver(parts: &Parts, content: Option<&dyn ContentComponent>) -> CapturedValues {
  let mut resolver = CapturedValues::new();
  for (part_name, slots) in parts {
    for (slot_name, slot_value) in slots {
      let path = format!("$.{part_name}.{slot_name}");
      resolver = resolver.capture(path, decode_slot(slot_value, content));
    }
  }
  resolver
}

/// A wire-form slot's document value: a content component decodes anything it tagged with a
/// content type (component-interfaces spec §6); everything else — method, path, headers, an
/// untyped body — is already the document model's own value (contract-file spec §5.3's `json`
/// default), so it is taken as-is.
fn decode_slot(slot: &SlotValue, content: Option<&dyn ContentComponent>) -> RuntimeValue {
  match (&slot.content_type, content) {
    (Some(content_type), Some(content)) => match content.decode(Decode {
      content_type: content_type.clone(),
      value: slot.clone(),
      options: None,
    }) {
      Ok(result) => result.document,
      // Undecodable is "not there", not a crash: the plan discovers it via `check:exists`/a
      // failed match, same as any other absent value (plan-grammar spec §2.4).
      Err(_) => RuntimeValue::Absent,
    },
    _ => RuntimeValue::from_json(&slot.content),
  }
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

/// A generated document value, wired for reply: a scalar rides as the transport's plain wire form
/// (component-http reads `status` this way, no content component involved); anything structured
/// has no wire form except through a content component's encoding, so it gets one.
///
/// Which content type a slot actually wants is really a per-slot property of the interaction's
/// declared shape (design 2.6) — this loop doesn't have that wiring (the same "single slot, not a
/// registry" gap [`crate::plan::resolve`] already documents), so it infers structured-vs-scalar
/// from the generated value instead. Correct for the RFC order interaction's JSON body; whoever
/// adds a second content type resolves this properly.
fn encode_slot(value: &Value, content: Option<&dyn ContentComponent>) -> SlotValue {
  match (value, content) {
    (Value::Object(_) | Value::Array(_), Some(content)) => match content.encode(Encode {
      content_type: "application/json".to_string(),
      document: RuntimeValue::from_json(value),
      options: None,
    }) {
      Ok(result) => result.value,
      Err(_) => plain_slot(value),
    },
    _ => plain_slot(value),
  }
}

fn plain_slot(value: &Value) -> SlotValue {
  SlotValue {
    content: value.clone(),
    encoded: None,
    content_type: None,
  }
}

/// The reply sent when the recorded request didn't match what was armed: a `500` naming every
/// mismatch, so the consumer's own HTTP client sees a clear failure rather than a hang or the
/// response it did not earn. Refining this into a real "why your request didn't match" experience
/// is plan task 4.6's job, not this wiring's.
fn mismatch_reply(mismatches: &[Mismatch], content: Option<&dyn ContentComponent>) -> Parts {
  let body = Value::Array(
    mismatches
      .iter()
      .map(|m| serde_json::json!({ "path": m.path, "message": m.message }))
      .collect(),
  );
  let mut part = Part::new();
  part.insert("status".to_string(), plain_slot(&Value::from(500)));
  part.insert("body".to_string(), encode_slot(&body, content));
  let mut parts = Parts::new();
  parts.insert("response".to_string(), part);
  parts
}
