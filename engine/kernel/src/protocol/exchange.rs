//! The live passive exchange (plan task 4.5, variant-semantics spec §4.1–§4.2): what actually
//! drives a transport once `start-transport` binds one to a session. It is two parts (ADR 0062
//! decision 4). [`run`] waits for the next arrival, `poll-inbound` with its timeout, and runs as
//! background work the embedding provides, one task per transport instance: it is what a real
//! inbound request waits on, so it cannot be the dispatcher that answers `consumer-session/*`
//! frames. [`handle_arrival`] handles one arrival and waits on nothing, so a driver that waits for
//! arrivals some other way calls the same step.
//!
//! What this closes: `serve-variant` (plan task 4.3) already computed the concrete payload a
//! variant should produce; nothing before this task ever put it on the wire, or looked at what
//! came back. A transport instance holds every interaction armed on it, each until its variant is
//! reported, and an arrival goes to the arming it matches best (ADR 0069, staged plan S2.13).
//!
//! An embedding with no background work to offer, the WASM component, refuses `start-transport` by
//! name before this module is reached. Gate W's driver would call [`handle_arrival`] from whatever
//! that target waits on (ADR 0062 decision 7).

use super::contributions::Contributions;
use super::events::Stream;
use super::session::{ExchangeOutcome, Exercised, Payload};
use super::wire::{encode_slot, find_container, mismatch_json, octets_slot, parts_resolver, plain_slot};
use crate::common::ContentTypes;
use crate::component::{
  ContentComponent, ContentSlots, Dispose, Inbound, Part, Parts, PollInbound, Reply, TransportComponent,
};
use crate::plan::{Plan, Status, execute_with, outcome};
use crate::variant::generate::Generated;
use serde_json::Value;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

/// What `serve-variant` arms for a passive HTTP interaction (spec §4.1, §8.2): everything the
/// background loop needs to match the next inbound request and answer it, with no further lookup
/// into session state — the loop runs on its own thread and this is the whole handoff. What it holds
/// is shared with the session's cache of the variant's arming (S2.3), so arming copies no document.
pub(crate) struct ArmedExchange {
  pub handle: String,
  pub variant_id: String,
  /// The interaction's plan, pinned to this variant's assignment (plan-grammar spec §5.1); only
  /// its `request` subtree is read here.
  pub request_plan: Arc<Plan>,
  /// The response [`crate::variant::generate::interaction`] produced for this variant — replayed
  /// verbatim on a match, the consumer side's own mirror of variant-semantics spec §5.2's
  /// replay-by-example.
  pub response_parts: Arc<Payload>,
  /// The interaction's declared content types (contract-file spec §5.5): how the inbound request's
  /// slots are decoded and the reply's encoded.
  pub content_types: Arc<ContentTypes>,
  /// The interaction's shapes, by part, for the parts whose shapes name a generator (ADR 0048): a
  /// reply is filled with fresh values there each time it is served. The recorded evidence stays
  /// `response_parts`, so a contract recorded twice is the same contract.
  pub fresh: Arc<BTreeMap<String, crate::contract::ShapePart>>,
  /// The variant's resolved provider-state parameters, which a reply's generators read.
  pub provider_state: Arc<serde_json::Map<String, Value>>,
  /// The members whose values a mismatch never repeats, and the strings under them in the
  /// interaction's own examples (ADR 0074 decision 7).
  pub redacted: Arc<Vec<crate::redaction::Member>>,
  pub secrets: Arc<crate::redaction::Secrets>,
}

/// Shared between `serve_variant` (arms) and this module's background loop (matches, replies,
/// records) for one transport instance. A mutex rather than a channel: both sides need to read
/// back what the other last did — arming replaces any previous arming, and the loop clears it once
/// consumed — which a one-way channel does not give without a second one back the other way.
#[derive(Default)]
pub(crate) struct ExchangeState {
  /// Every arming on this transport, in the order it was armed (ADR 0069 decision 2). Each stays
  /// until its variant is reported or its handle is armed again.
  armed: Vec<Arc<ArmedExchange>>,
  /// The `(handle, variant id)` of every arming an arrival is being matched against, until its
  /// outcome is recorded. The reply is handed to the transport before the outcome is recorded, so a
  /// client can hold the response first: `report-variant` waits on [`Exchange::settled`] while this
  /// names its variant (ADR 0064 decision 1, ADR 0069 decision 4).
  in_flight: Vec<(String, String)>,
  /// `(handle, variant id) -> evidence`, drained into the session's own per-interaction map when
  /// the transport stops ([`super::session::ConsumerSession::stop_transports`]) or the variant is
  /// reported. Every exchange of a variant is merged into its one entry ([`Exercised::merge`]).
  pub outcomes: BTreeMap<(String, String), Exercised>,
}

impl ExchangeState {
  /// Arm `armed`, replacing its handle's arming if it has one; it goes last (ADR 0069 decision 2).
  pub fn arm(&mut self, armed: ArmedExchange) {
    self.armed.retain(|other| other.handle != armed.handle);
    self.armed.push(Arc::new(armed));
  }
}

/// One transport instance's exchange state, and the signal that an exchange under way has been
/// recorded.
#[derive(Default)]
pub(crate) struct Exchange {
  pub state: Mutex<ExchangeState>,
  settled: Condvar,
  /// The session's event stream, when its host asked for one (spec §9.6, staged plan S3.5): each
  /// arrival's outcome is a `consumer-session/exchange` event, which the host surfaces while its test
  /// runs, so the engine's own log says it only at `debug`.
  events: Option<Arc<Stream>>,
}

/// The kind of event an arrival's outcome is (spec §9.6).
pub(crate) const EXCHANGE_EVENT: &str = "consumer-session/exchange";

/// How long `report-variant` waits for an exchange under way to be recorded. The exchange waits only
/// on the transport's `reply` or `dispose`, so this bounds a transport that never returns.
const SETTLE_TIMEOUT: Duration = Duration::from_secs(10);

impl Exchange {
  /// An exchange whose arrivals are told on `events`, when the session has a stream.
  pub fn new(events: Option<Arc<Stream>>) -> Self {
    Exchange {
      events,
      ..Exchange::default()
    }
  }

  /// Tell the host what became of an arrival, when it asked (spec §9.6). Called with no lock held: a
  /// full stream waits for the host to drain it.
  fn tell(&self, payload: Value) {
    if let Some(events) = &self.events {
      events.emit(EXCHANGE_EVENT, payload);
    }
  }

  /// Wait until no exchange of `handle`'s `variant` is under way, then take its recorded outcome and
  /// disarm it if it is still armed (ADR 0064 decision 1). `None` when nothing was recorded for it.
  pub fn settle(&self, handle: &str, variant: &str) -> Option<Exercised> {
    let key = (handle.to_string(), variant.to_string());
    let state = self.state.lock().expect("exchange state lock poisoned");
    let (mut state, waited) = self
      .settled
      .wait_timeout_while(state, SETTLE_TIMEOUT, |state| state.in_flight.contains(&key))
      .expect("exchange state lock poisoned");
    if waited.timed_out() {
      tracing::warn!(
        handle,
        variant,
        "an exchange of this variant was still under way when its test was reported"
      );
    }
    state
      .armed
      .retain(|armed| !(armed.handle == handle && armed.variant_id == variant));
    state.outcomes.remove(&key)
  }
}

const POLL_TIMEOUT_MS: u64 = 200;

/// Poll `instance` until `stop` is set or the transport reports it gone (which is what its own
/// `stop` looks like from here, once `ConsumerSession::stop_transports` has called it) — never
/// erroring outward, since there is no dispatch frame left to carry an error to by the time this
/// runs: whatever a poll or reply attempt fails at, the loop simply moves on or ends.
pub(crate) fn run(
  transport: Arc<dyn TransportComponent>,
  content: Option<Arc<dyn ContentComponent>>,
  contributions: Arc<Contributions>,
  instance: String,
  stop: Arc<AtomicBool>,
  exchange: Arc<Exchange>,
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
    handle_arrival(
      transport.as_ref(),
      content.as_deref(),
      &contributions,
      &instance,
      inbound,
      &exchange,
    );
  }
}

/// One arming's request plan, executed against an arrival.
struct Candidate {
  armed: Arc<ArmedExchange>,
  status: Status,
  mismatches: Vec<crate::plan::Mismatch>,
  /// How many plan actions in the request passed: what makes one match more specific than another.
  passed: usize,
}

/// The arming an arrival goes to (ADR 0069 decisions 2 and 3): one it matches, else the one it came
/// closest to; the fewest mismatches, then the most actions passed, then the first armed.
fn best(candidates: Vec<Candidate>) -> Option<Candidate> {
  candidates
    .into_iter()
    .enumerate()
    .min_by_key(|(order, candidate)| {
      (
        candidate.mismatches.len(),
        std::cmp::Reverse(candidate.passed),
        *order,
      )
    })
    .map(|(_, candidate)| candidate)
}

/// The plan actions in an executed subtree that ran and did not fail.
fn passed_actions(executed: &crate::plan::Executed) -> usize {
  use crate::plan::{ExecutedKind, NodeResult};
  let own = match (&executed.kind, &executed.result) {
    (ExecutedKind::Action { .. }, Some(result)) if !matches!(result, NodeResult::Error { .. }) => 1,
    _ => 0,
  };
  let children: &[crate::plan::Executed] = match &executed.kind {
    ExecutedKind::Container { children, .. }
    | ExecutedKind::Action { children, .. }
    | ExecutedKind::Pipeline { children }
    | ExecutedKind::Splat { children } => children,
    _ => &[],
  };
  own + children.iter().map(passed_actions).sum::<usize>()
}

/// One arrival: match it against every arming, reply as the one it goes to, and record that
/// variant's outcome. It waits on nothing but the transport's `reply` or `dispose` (ADR 0062
/// decision 4). Nothing armed → this exchange proves nothing about any variant, so it is refused
/// rather than guessed at (variant-semantics spec §4.2's honesty applies here too, not only to
/// recording).
pub(crate) fn handle_arrival(
  transport: &dyn TransportComponent,
  content: Option<&dyn ContentComponent>,
  contributions: &Contributions,
  instance: &str,
  inbound: Inbound,
  exchange: &Exchange,
) {
  let armings = {
    let mut state = exchange.state.lock().expect("exchange state lock poisoned");
    let armings = state.armed.clone();
    state.in_flight = armings
      .iter()
      .map(|armed| (armed.handle.clone(), armed.variant_id.clone()))
      .collect();
    armings
  };
  if armings.is_empty() {
    // Plan task 4.6's own finding: this is the one line that names an arrival at all when
    // nothing was armed for it — without it, a request that missed its `serve-variant` window
    // entirely (a race, a typo'd variant id upstream) leaves no trace anywhere.
    if exchange.events.is_some() {
      tracing::debug!(instance, event = %inbound.event, "inbound request arrived with nothing armed; answering unmatched");
    } else {
      tracing::warn!(instance, event = %inbound.event, "inbound request arrived with nothing armed; answering unmatched");
    }
    let _ = transport.dispose(Dispose {
      instance: instance.to_string(),
      event: inbound.event,
      disposition: "unmatched".to_string(),
      details: None,
    });
    exchange.tell(serde_json::json!({ "transport": instance, "status": "unmatched" }));
    return;
  }

  // The part names are the transport's (kernel-boundary review finding 8): the arrival is matched
  // against the plan's subtree for the part that starts an exchange, and answered with the part
  // that answers one.
  let names = transport.exchange_parts();
  let candidates = armings
    .into_iter()
    .map(|armed| {
      let resolver = parts_resolver(&inbound.parts, &armed.content_types, content);
      // `match:content-type` reads the arrival's own octets and labels (staged plan S1.10, ADR 0049).
      let detection = content.map(|content| crate::component::ContentDetection {
        content,
        parts: Some(&inbound.parts),
        declared: Some(&armed.content_types),
        by_label: true,
      });
      let executed = execute_with(
        &armed.request_plan,
        &resolver,
        detection
          .as_ref()
          .map(|detection| detection as &dyn crate::plan::ContentDetector),
        Some(contributions),
      );
      let request_subtree = find_container(&executed, &names.initiating).unwrap_or(&executed);
      let (status, mismatches) = outcome(request_subtree);
      let passed = passed_actions(request_subtree);
      Candidate {
        armed,
        status,
        mismatches,
        passed,
      }
    })
    .collect();
  let Candidate {
    armed,
    status,
    mut mismatches,
    ..
  } = best(candidates).expect("an arrival is matched against at least one arming");
  // What the consumer sent under a redacted member is a secret, whichever arming it went to.
  let mut secrets = (*armed.secrets).clone();
  secrets.collect_parts(&armed.redacted, &inbound.parts);
  secrets.scrub_mismatches(&mut mismatches);

  // The one line plan task 4.6's report leans on: `RUST_LOG=debug` names the variant a failure
  // belongs to, which nothing else on this loop's own thread does — a consumer's own crash
  // handling this response has no way to ask the engine "which variant was that" after the fact.
  match status {
    Status::Matched => {
      tracing::debug!(instance, handle = %armed.handle, variant = %armed.variant_id, "request matched the armed variant")
    }
    Status::Mismatched => {
      // Where and what, as a person reads them: the marks are for a host's display, not a log.
      let said: Vec<String> = mismatches
        .iter()
        .map(|mismatch| match &mismatch.path {
          Some(path) => format!("{path}: {}", mismatch.message),
          None => mismatch.message.clone(),
        })
        .collect();
      if exchange.events.is_some() {
        tracing::debug!(instance, handle = %armed.handle, variant = %armed.variant_id, mismatches = ?said, "request did not match the armed variant")
      } else {
        tracing::warn!(instance, handle = %armed.handle, variant = %armed.variant_id, mismatches = ?said, "request did not match the armed variant")
      }
    }
  }

  // A match is answered with the variant's reply; a mismatch is rejected, and how a kind says
  // "no" is the transport's business (kernel-boundary review finding 9): HTTP answers a 500 naming
  // every mismatch.
  let delivered = match (status, &names.answering) {
    (Status::Matched, Some(answering)) => transport
      .reply(Reply {
        instance: instance.to_string(),
        event: inbound.event,
        parts: response_parts(
          &fresh_reply(&armed, answering),
          answering,
          &transport.content_slots(),
          armed.content_types.get(answering),
          content,
        ),
      })
      .map(|_| ()),
    (Status::Matched, None) => transport
      .dispose(Dispose {
        instance: instance.to_string(),
        event: inbound.event,
        disposition: "accept".to_string(),
        details: None,
      })
      .map(|_| ()),
    (Status::Mismatched, _) => transport
      .dispose(Dispose {
        instance: instance.to_string(),
        event: inbound.event,
        disposition: "reject".to_string(),
        details: Some(serde_json::json!({
          "mismatches": mismatches.iter().map(|mismatch| mismatch_json(mismatch, false)).collect::<Vec<_>>(),
        })),
      })
      .map(|_| ()),
  };

  // A variant is verified only if the consumer actually got the response it was armed with: a
  // request that matched but whose reply never reached the client (the client gave up, or the
  // connection was unusable) exercised nothing the consumer could have handled.
  let (exchange_outcome, recorded, told) = match (status, delivered) {
    (Status::Matched, Ok(_)) => (ExchangeOutcome::Verified, Vec::new(), "matched"),
    (Status::Matched, Err(err)) => {
      tracing::warn!(instance, handle = %armed.handle, variant = %armed.variant_id, error = %err.message, "the matched request's response could not be delivered");
      (
        ExchangeOutcome::Failed,
        vec![serde_json::json!({
          "message": format!("the request matched, but its response could not be delivered to the consumer: {}", err.message),
          "action": "transport:reply",
        })],
        "undelivered",
      )
    }
    (Status::Mismatched, _) => (
      ExchangeOutcome::Failed,
      mismatches
        .iter()
        .map(|mismatch| mismatch_json(mismatch, false))
        .collect(),
      "mismatched",
    ),
  };
  let mut event = serde_json::json!({
    "transport": instance,
    "handle": armed.handle,
    "variant": armed.variant_id,
    "status": told,
  });
  if !recorded.is_empty() {
    event["mismatches"] = Value::from(recorded.clone());
  }
  let mut state = exchange.state.lock().expect("exchange state lock poisoned");
  state.in_flight.clear();
  let exercised = Exercised {
    outcome: exchange_outcome,
    parts: Arc::clone(&armed.response_parts),
    mismatches: recorded,
  };
  match state
    .outcomes
    .entry((armed.handle.clone(), armed.variant_id.clone()))
  {
    std::collections::btree_map::Entry::Occupied(mut earlier) => earlier.get_mut().merge(exercised),
    std::collections::btree_map::Entry::Vacant(first) => {
      first.insert(exercised);
    }
  }
  drop(state);
  exchange.settled.notify_all();
  exchange.tell(event);
}

/// The armed reply, with every position the answering part's shapes give a generator filled with a
/// fresh value, on the consumer's side (ADR 0048): a new value for every exchange. A generator that
/// produces nothing leaves the armed value, and says so in the log. A reply with no generators is the
/// armed one, shared rather than copied.
fn fresh_reply(armed: &ArmedExchange, answering: &str) -> Arc<Payload> {
  fill_fresh(
    &armed.response_parts,
    &armed.fresh,
    &[answering],
    &armed.provider_state,
    &armed.handle,
    &armed.variant_id,
  )
}

/// Every part of an armed payload with its generators' fresh values filled, as a reply's are: what
/// an emissive interaction hands its test (ADR 0067 decision 5).
pub(crate) fn fresh_payload(
  payload: &Arc<Payload>,
  fresh: &BTreeMap<String, crate::contract::ShapePart>,
  provider_state: &serde_json::Map<String, Value>,
  handle: &str,
  variant: &str,
) -> Arc<Payload> {
  let parts: Vec<&str> = fresh.keys().map(String::as_str).collect();
  fill_fresh(payload, fresh, &parts, provider_state, handle, variant)
}

/// `payload` with every position the named parts' shapes give a generator filled with a fresh value
/// (ADR 0048). A generator that produces nothing leaves the armed value, and says so in the log. A
/// payload with no generators in those parts is the armed one, shared rather than copied.
fn fill_fresh(
  payload: &Arc<Payload>,
  fresh: &BTreeMap<String, crate::contract::ShapePart>,
  parts: &[&str],
  provider_state: &serde_json::Map<String, Value>,
  handle: &str,
  variant: &str,
) -> Arc<Payload> {
  if !parts
    .iter()
    .any(|part| fresh.contains_key(*part) && payload.contains_key(*part))
  {
    return Arc::clone(payload);
  }
  let mut filled = Payload::clone(payload);
  let generators = crate::shape::fresh::Fresh {
    generators: &crate::component::PactGenerators,
    side: "consumer",
    provider_state,
  };
  let mut failures = Vec::new();
  for part in parts {
    let (Some(shapes), Some(slots)) = (fresh.get(*part), filled.get_mut(*part)) else {
      continue;
    };
    for (slot_name, value) in slots.iter_mut() {
      // Octets have no generator to fill: a generator's value is a document (S2.15).
      if let (Some(shape), Generated::Document(value)) = (shapes.get(slot_name), value) {
        crate::shape::fresh::fill(
          shape,
          value,
          &format!("$.{part}.{slot_name}"),
          &generators,
          &mut failures,
        );
      }
    }
  }
  for failure in failures {
    tracing::warn!(handle, variant, target = %failure.target, generator = %failure.generator, message = %failure.message, "a generator produced no value; the recorded one was served");
  }
  Arc::new(filled)
}

/// A payload in the form a transport carries, with no transport to say which slots are content
/// (ADR 0067 decision 5): a slot the interaction declares a type for is encoded through the content
/// component for it, and every other slot is its plain value.
pub(crate) fn wire_parts(
  payload: &Payload,
  declared: &ContentTypes,
  content: Option<&dyn ContentComponent>,
) -> Parts {
  payload
    .iter()
    .map(|(part_name, slots)| {
      let types = declared.get(part_name);
      let part: Part = slots
        .iter()
        .map(|(slot_name, value)| {
          let declared = types.and_then(|types| types.get(slot_name)).map(String::as_str);
          let wired = match (value, declared) {
            (Generated::Octets(bytes), declared) => octets_slot(bytes, declared),
            (Generated::Document(value), Some(content_type)) => {
              encode_slot(value, Some(content_type), content)
            }
            (Generated::Document(value), None) => plain_slot(value),
          };
          (slot_name.clone(), wired)
        })
        .collect();
      (part_name.clone(), part)
    })
    .collect()
}

/// The generated payload of the part that answers, wired for reply (component-interfaces spec §4).
/// Only the slots the transport declares as content go through the content component
/// (component-interfaces spec §5.5); every other slot — an HTTP response's `headers` map is as
/// structured as any body — is handed over as its plain JSON value.
fn response_parts(
  response: &Payload,
  answering: &str,
  content_slots: &ContentSlots,
  declared_types: Option<&BTreeMap<String, String>>,
  content: Option<&dyn ContentComponent>,
) -> Parts {
  let mut parts = Parts::new();
  if let Some(slots) = response.get(answering) {
    let declared = content_slots.get(answering);
    let mut part = Part::new();
    for (slot_name, value) in slots {
      let is_content = declared.is_some_and(|names| names.contains(slot_name));
      let declared_type = declared_types
        .filter(|_| is_content)
        .and_then(|slots| slots.get(slot_name))
        .map(String::as_str);
      let wired = match value {
        Generated::Octets(bytes) => octets_slot(bytes, declared_type),
        Generated::Document(value) if is_content => encode_slot(value, declared_type, content),
        Generated::Document(value) => plain_slot(value),
      };
      part.insert(slot_name.clone(), wired);
    }
    parts.insert(answering.to_string(), part);
  }
  parts
}
