//! Consumer sessions (engine-protocol spec §7, §8.2): the only resource a host can hold.
//! Everything a session allocates — here, its interactions — is released when the session ends;
//! there is no per-object cleanup call (spec §7.1).

use super::exchange::{self, ArmedExchange, ExchangeState};
use crate::component::{ComponentError, ContentComponent, Start, Stop, TransportComponent};
use crate::contract::{self, Contract, Party};
use crate::error::Problem;
use crate::interaction_spec::{self, InteractionSpec, InteractionSpecError};
use crate::plan::{self, Assignment, Plan};
use crate::variant::{Selected, SelectionReport, VariantError, generate, params, select};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

/// The variant a `serve-variant` call has most recently armed for one interaction (variant-
/// semantics spec §4.1): its id, and the concrete payload the generator produced for it, ready
/// for whatever eventually drives a transport from it (no transport is bound to a consumer
/// session yet — `start-transport` isn't wired into [`super::engine`]'s dispatch).
pub struct Armed {
  #[allow(dead_code)]
  pub variant: String,
  #[allow(dead_code)]
  pub parts: BTreeMap<String, BTreeMap<String, Value>>,
}

/// One interaction as validated and compiled by `add-interaction`. Spec §8.2: "the engine
/// validates it and compiles what it needs" — `plan::compile` is infallible once `spec` parsed
/// (matching happens at execution, not compile time), so this always succeeds once parsing does.
/// The compiled `plan` isn't consumed by anything in 4.1; it's here so 4.2 (transport) has it
/// without re-deriving session state layout.
pub(crate) struct InteractionEntry {
  pub spec: InteractionSpec,
  #[allow(dead_code)]
  pub plan: Plan,
  /// The interaction's `parts` exactly as submitted (shape-language spec §3.6, contract-file spec
  /// §5.1): a shape node is opaque past parsing, so the contract writer (plan task 4.4) records
  /// the author's own JSON rather than re-deriving it from the typed [`InteractionSpec`].
  pub raw_parts: BTreeMap<String, contract::ShapePart>,
  /// The most recent selection computed for this interaction by `consumer-session/variants`
  /// (variant-semantics spec §3.9). `None` until a host calls it — a host that never does gets
  /// the honest, minimal `not-exercised` report 4.1's finalise already gave.
  pub selection: Option<Selected>,
  /// The variant `serve-variant` most recently armed (spec §4.1: at most one at a time; re-arming
  /// replaces it).
  pub armed: Option<Armed>,
  /// Variants actually exercised (variant-semantics spec §4.2), keyed by variant id — what
  /// `results` and `contract` read. Populated by [`ConsumerSession::stop_transports`] draining
  /// the live exchange loop's evidence (plan task 4.5), or directly by
  /// [`ConsumerSession::record_exercised`]'s test callers for a session with no transport bound.
  pub exercised: BTreeMap<String, Exercised>,
}

/// The evidence one exercised variant produced (variant-semantics spec §4.4): whether the
/// exchange it drove came back matching, and the concrete payload it produced — the same
/// `part -> slot -> value` document [`Armed`] carries, generated the same way.
pub(crate) struct Exercised {
  pub outcome: ExchangeOutcome,
  pub parts: BTreeMap<String, BTreeMap<String, Value>>,
  /// Why a `Failed` exchange failed, as the protocol carries mismatches ([`super::wire::mismatch_json`]):
  /// `finalise` reports them per variant (spec §8.2, "unmatched-request … detail rides in
  /// `results`"), so a host can say what went wrong without reading the engine's log.
  pub mismatches: Vec<Value>,
}

/// Statuses: `verified`, `failed`, `not-exercised` (variant-semantics spec §4.2). The third is
/// the *absence* of an [`Exercised`] entry, not a variant of this enum — nothing distinguishes
/// "skipped" from "never got there", and nothing should.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExchangeOutcome {
  Verified,
  Failed,
}

/// One transport instance bound to this session by `consumer-session/start-transport` (spec §8.2):
/// the running background exchange loop ([`exchange::run`]), and what stops it cleanly.
pub(crate) struct TransportRun {
  pub instance: String,
  pub kind: String,
  component: Arc<dyn TransportComponent>,
  stop: Arc<AtomicBool>,
  state: Arc<Mutex<ExchangeState>>,
  handle: Option<JoinHandle<()>>,
}

/// `consumer-session/variants` (variant-semantics spec §3.9): a malformed policy vs. an unknown
/// handle are different failures at the protocol boundary.
#[derive(Debug)]
pub enum VariantsError {
  HandleNotFound,
  Variant(VariantError),
}

pub enum ServeVariantError {
  HandleNotFound,
  VariantNotFound { selection: Vec<String> },
}

/// One `consumer-session/*` session: config plus the interactions added to it so far. `order`
/// tracks submission order explicitly — `i-{n}` handles are unpadded, so string-sorting them
/// (as a `BTreeMap` would) stops matching submission order past nine interactions.
pub(crate) struct ConsumerSession {
  /// The Janus contract's `consumer`/`provider` (design 2.5), read by [`ConsumerSession::contract`].
  pub consumer: Party,
  pub provider: Party,
  /// The session-wide sampling policy layer (variant-semantics spec §3.8 layer 2), from
  /// `create`'s `config.policy`.
  policy: Option<Value>,
  interactions: BTreeMap<String, InteractionEntry>,
  order: Vec<String>,
  next_handle: u64,
  /// Transports bound by `start-transport` (spec §8.2), each driving its own background exchange
  /// loop (plan task 4.5). A session MAY start several; `serve_variant` arms every `"http"` one.
  transports: Vec<TransportRun>,
}

impl ConsumerSession {
  fn new(consumer: Party, provider: Party, policy: Option<Value>) -> Self {
    ConsumerSession {
      consumer,
      provider,
      policy,
      interactions: BTreeMap::new(),
      order: Vec::new(),
      next_handle: 1,
      transports: Vec::new(),
    }
  }

  /// `consumer-session/add-interaction` (spec §8.2): validate and compile, or a structured
  /// `InteractionSpecError` a caller maps to `interaction-invalid`.
  pub fn add_interaction(&mut self, interaction: &Value) -> Result<String, InteractionSpecError> {
    let spec = interaction_spec::parse(interaction)?;
    // Variant-bound state bindings are validated here and nowhere later (variant-semantics spec
    // §6.5): the reference resolves against this interaction's variant space, which does not exist
    // until the shapes parse, and the author is looking at the DSL that produced it right now.
    if let Some(states) = &spec.states {
      let space = plan::variant_space(&spec);
      if let Err(problems) = params::bind(states, &space, "/states") {
        return Err(InteractionSpecError { problems });
      }
    }
    let compiled = plan::compile(&spec, &Assignment::new(), None);
    let raw_parts = raw_parts(interaction);
    let handle = format!("i-{}", self.next_handle);
    self.next_handle += 1;
    self.interactions.insert(
      handle.clone(),
      InteractionEntry {
        spec,
        plan: compiled,
        raw_parts,
        selection: None,
        armed: None,
        exercised: BTreeMap::new(),
      },
    );
    self.order.push(handle.clone());
    Ok(handle)
  }

  /// `consumer-session/start-transport` (spec §8.2): starts `component` under engine-assigned
  /// `instance`, then spawns the background exchange loop (plan task 4.5, [`exchange::run`]) that
  /// actually drives it — `poll-inbound` blocks, so nothing on the dispatch thread can pump it.
  /// Returns the transport's endpoint descriptor.
  pub fn start_transport(
    &mut self,
    kind: &str,
    component: Arc<dyn TransportComponent>,
    content: Option<Arc<dyn ContentComponent>>,
    instance: String,
    options: Option<Value>,
  ) -> Result<Value, ComponentError> {
    let result = component.start(Start {
      instance: instance.clone(),
      kind: kind.to_string(),
      role: "serve".to_string(),
      options,
    })?;

    let stop = Arc::new(AtomicBool::new(false));
    let state = Arc::new(Mutex::new(ExchangeState::default()));
    let thread_component = Arc::clone(&component);
    let thread_stop = Arc::clone(&stop);
    let thread_state = Arc::clone(&state);
    let thread_instance = instance.clone();
    let handle = thread::spawn(move || {
      exchange::run(
        thread_component,
        content,
        thread_instance,
        thread_stop,
        thread_state,
      );
    });

    self.transports.push(TransportRun {
      instance,
      kind: kind.to_string(),
      component,
      stop,
      state,
      handle: Some(handle),
    });
    Ok(result.endpoint)
  }

  /// `consumer-session/finalise` (spec §7.1, §8.2): "transports stopped, all state released."
  /// Signals every bound transport's exchange loop to stop, tells the component itself to stop
  /// (which also unblocks a loop mid-`poll-inbound`, since the instance then stops existing),
  /// joins each loop thread, and drains its recorded evidence into the interactions it belongs to
  /// — after this, `results`/`contract` see exactly what really happened on the wire.
  pub fn stop_transports(&mut self) {
    for mut run in self.transports.drain(..) {
      run.stop.store(true, Ordering::Relaxed);
      let _ = run.component.stop(Stop {
        instance: run.instance.clone(),
      });
      if let Some(handle) = run.handle.take() {
        let _ = handle.join();
      }
      let outcomes = std::mem::take(&mut run.state.lock().expect("exchange state lock poisoned").outcomes);
      for ((handle, variant_id), exercised) in outcomes {
        if let Some(entry) = self.interactions.get_mut(&handle) {
          entry.exercised.insert(variant_id, exercised);
        }
      }
    }
  }

  /// `consumer-session/variants` (variant-semantics spec §3.9): resolve the layered policy
  /// (defaults, session config, this call's override — spec §3.8) against the interaction's
  /// variant space, run the selection algorithm, cache it as this interaction's current
  /// selection — what `serve-variant` and `finalise` read next — and return the result document
  /// (spec §3.9's schema: `variants`, each with its id/label/origin/assignment, plus `report`).
  pub fn variants(&mut self, handle: &str, call_policy: Option<&Value>) -> Result<Value, VariantsError> {
    let Some(entry) = self.interactions.get(handle) else {
      return Err(VariantsError::HandleNotFound);
    };
    let space = plan::variant_space(&entry.spec);
    let layers = [self.policy.as_ref(), call_policy];
    let policy = crate::variant::SamplingPolicy::resolve(&layers)
      .map_err(|problem| VariantsError::Variant(VariantError::InvalidPolicy(vec![problem])))?;
    let selected = select(&space, &policy).map_err(VariantsError::Variant)?;

    let variants: Vec<Value> = selected.variants.iter().map(|v| v.to_json(&space)).collect();
    let result = serde_json::json!({ "variants": variants, "report": selected.report.to_json() });

    let entry = self.interactions.get_mut(handle).expect("checked above");
    entry.selection = Some(selected);
    Ok(result)
  }

  /// `consumer-session/serve-variant` (spec §4.1, §8.2): arm the named variant of a previously
  /// selected interaction, generating its concrete payload now (plan task 4.3's generator). For a
  /// passive HTTP interaction with a transport bound, this also arms every `"http"` transport's
  /// exchange loop (plan task 4.5) with the pinned request plan to match and the response to
  /// answer with; an emissive interaction, or one with no transport bound, stops at recording the
  /// arming, exactly as before.
  pub fn serve_variant(&mut self, handle: &str, variant_id: &str) -> Result<(), ServeVariantError> {
    let Some(entry) = self.interactions.get_mut(handle) else {
      return Err(ServeVariantError::HandleNotFound);
    };
    let assignment = {
      let Some(selection) = &entry.selection else {
        return Err(ServeVariantError::VariantNotFound {
          selection: Vec::new(),
        });
      };
      match selection.variants.iter().find(|v| v.id == variant_id) {
        Some(variant) => variant.assignment.clone(),
        None => {
          let selection = selection.variants.iter().map(|v| v.id.clone()).collect();
          return Err(ServeVariantError::VariantNotFound { selection });
        }
      }
    };
    let parts = generate::interaction(&entry.spec, &assignment);
    entry.armed = Some(Armed {
      variant: variant_id.to_string(),
      parts: parts.clone(),
    });

    let is_passive_http = entry
      .spec
      .transport
      .as_ref()
      .is_some_and(|t| t.kind == "http" && t.mode.as_deref() != Some("emissive"));
    if is_passive_http {
      let request_plan = plan::compile(&entry.spec, &assignment, Some(variant_id));
      for run in self.transports.iter().filter(|run| run.kind == "http") {
        let mut state = run.state.lock().expect("exchange state lock poisoned");
        state.armed = Some(ArmedExchange {
          handle: handle.to_string(),
          variant_id: variant_id.to_string(),
          request_plan: request_plan.clone(),
          response_parts: parts.clone(),
        });
      }
    }
    Ok(())
  }

  /// Record the outcome of actually exercising `variant_id` of `handle` (variant-semantics spec
  /// §4.2: the armed variant's request sent and its exchange completed) — normally driven by a
  /// transport exchange, which is not wired to a consumer session yet (plan task 4.5's gap, not
  /// this one's). Exists so the honesty rule (contract-file spec §2.2) and the contract writer
  /// can be exercised without a live transport; `results` and `contract` read what it records.
  #[allow(dead_code)]
  pub(crate) fn record_exercised(&mut self, handle: &str, variant_id: &str, outcome: ExchangeOutcome) {
    let entry = self
      .interactions
      .get_mut(handle)
      .expect("handle must have been returned by add_interaction");
    let assignment = entry
      .selection
      .as_ref()
      .and_then(|selection| selection.variants.iter().find(|v| v.id == variant_id))
      .expect("variant must be in the interaction's current selection")
      .assignment
      .clone();
    let parts = generate::interaction(&entry.spec, &assignment);
    entry.exercised.insert(
      variant_id.to_string(),
      Exercised {
        outcome,
        parts,
        mismatches: Vec::new(),
      },
    );
  }

  /// `consumer-session/finalise`'s `results` (spec §8.2): one entry per interaction, in
  /// submission order, with a `variants` breakdown once `variants` has been called for it
  /// (variant-semantics spec §4.2: every selected variant is required, and honestly
  /// `not-exercised` until something exercises it).
  pub fn results(&self) -> Vec<Value> {
    self
      .order
      .iter()
      .map(|handle| {
        let entry = &self.interactions[handle];
        match &entry.selection {
          None => serde_json::json!({ "handle": handle, "status": "not-exercised" }),
          Some(selected) => {
            let mut any_failed = false;
            let mut all_verified = true;
            let variants: Vec<Value> = selected
              .variants
              .iter()
              .map(|v| {
                let exercised = entry.exercised.get(&v.id);
                let status = match exercised.map(|e| e.outcome) {
                  Some(ExchangeOutcome::Verified) => "verified",
                  Some(ExchangeOutcome::Failed) => {
                    any_failed = true;
                    all_verified = false;
                    "failed"
                  }
                  None => {
                    all_verified = false;
                    "not-exercised"
                  }
                };
                match exercised.filter(|e| !e.mismatches.is_empty()) {
                  Some(e) => {
                    serde_json::json!({ "variant": v.id, "status": status, "mismatches": e.mismatches })
                  }
                  None => serde_json::json!({ "variant": v.id, "status": status }),
                }
              })
              .collect();
            let status = if any_failed {
              "failed"
            } else if all_verified {
              "verified"
            } else {
              "not-exercised"
            };
            serde_json::json!({ "handle": handle, "status": status, "variants": variants })
          }
        }
      })
      .collect()
  }

  /// `consumer-session/finalise`'s `contract` (protocol spec §8.2, contract-file spec §2.2): the
  /// honesty rule. `Ok(None)` when any interaction lacks a selection, or has a selected variant
  /// that came back anything other than `verified` — finalise withholds the contract exactly as
  /// it would for a failure. `Err` only for a document this session's own state can never validly
  /// produce: two interactions sharing a description and state list (contract-file spec §4.2),
  /// detectable only once every interaction is assembled.
  pub fn contract(&self) -> Result<Option<Contract>, Vec<Problem>> {
    let mut interactions = Vec::with_capacity(self.order.len());
    for handle in &self.order {
      let entry = &self.interactions[handle];
      let Some(selected) = &entry.selection else {
        return Ok(None);
      };
      let all_verified = selected.variants.iter().all(|v| {
        matches!(
          entry.exercised.get(&v.id),
          Some(Exercised {
            outcome: ExchangeOutcome::Verified,
            ..
          })
        )
      });
      if !all_verified {
        return Ok(None);
      }

      let space = plan::variant_space(&entry.spec);
      let variants = selected
        .variants
        .iter()
        .map(|v| {
          let exercised = &entry.exercised[&v.id];
          contract::RecordedVariant {
            id: v.id.clone(),
            origin: v.origin,
            assignment: v.assignment_json(&space),
            // Resolved per variant, not once per interaction (variant-semantics spec §6.4): that
            // is the whole point of a binding, and recording the resolved values is what lets a
            // verifier's own resolution be checked against this one rather than trusted.
            states: params::resolve_states(entry.spec.states.as_ref(), &space, &v.assignment),
            parts: to_value_parts(&exercised.parts),
          }
        })
        .collect();

      interactions.push(contract::Interaction {
        description: entry.spec.description.clone(),
        transport: entry.spec.transport.clone(),
        states: entry.spec.states.clone(),
        parts: entry.raw_parts.clone(),
        requires: entry.spec.requires.clone(),
        selection: contract::RecordedSelection {
          variants,
          report: report_to_map(&selected.report),
        },
      });
    }

    if let Some(problems) = duplicate_identity_problems(&interactions) {
      return Err(problems);
    }

    Ok(Some(Contract {
      format: contract::FORMAT.to_string(),
      schema: None,
      consumer: self.consumer.clone(),
      provider: self.provider.clone(),
      interactions,
      metadata: None,
    }))
  }
}

/// An `add-interaction` request's `parts` member, taken verbatim (contract-file spec §5.1: "which
/// parts exist and which slots they expose is the transport and content components' business,
/// never the kernel's" — a shape is opaque here, so there is nothing to reconstruct). `parse` has
/// already rejected anything not shaped this way by the time this is called, so the fallbacks
/// below never actually trigger; they just avoid a panic on a document this function is not the
/// one validating.
fn raw_parts(interaction: &Value) -> BTreeMap<String, contract::ShapePart> {
  let Some(parts) = interaction.get("parts").and_then(Value::as_object) else {
    return BTreeMap::new();
  };
  parts
    .iter()
    .map(|(part, slots)| {
      let slots: contract::ShapePart = slots
        .as_object()
        .map(|slots| {
          slots
            .iter()
            .map(|(slot, shape)| (slot.clone(), shape.clone()))
            .collect()
        })
        .unwrap_or_default();
      (part.clone(), slots)
    })
    .collect()
}

/// Wrap a generated payload's values as the contract format's slot values (contract-file spec
/// §5.3): `content` as-is and no `encoded`/`content-type` tag, which is the default `json`
/// encoding — exactly right for a value [`generate::interaction`] already produced as a document-
/// model `Value`.
fn to_value_parts(
  parts: &BTreeMap<String, BTreeMap<String, Value>>,
) -> BTreeMap<String, contract::ValuePart> {
  parts
    .iter()
    .map(|(part, slots)| {
      let slots = slots
        .iter()
        .map(|(slot, value)| {
          (
            slot.clone(),
            contract::SlotValue {
              content: value.clone(),
              encoded: None,
              content_type: None,
            },
          )
        })
        .collect();
      (part.clone(), slots)
    })
    .collect()
}

/// [`SelectionReport`] as the contract format's opaque `report` member (contract-file spec §5.2):
/// the same document [`SelectionReport::to_json`] already renders for the `variants` operation
/// result, re-keyed as a map since that is how [`contract::RecordedSelection::report`] carries an
/// opaque object.
fn report_to_map(report: &SelectionReport) -> BTreeMap<String, Value> {
  let Value::Object(map) = report.to_json() else {
    unreachable!("SelectionReport always serializes to a JSON object")
  };
  map.into_iter().collect()
}

/// Contract-file spec §4.2: no two interactions may share a description and state list. Checked
/// here, once every interaction is assembled, because that is the earliest point the full state
/// list — bindings resolved from nothing but the interaction spec, same for every variant — is
/// known; `pointer`s name the colliding interactions by their position in the array the writer is
/// about to produce.
fn duplicate_identity_problems(interactions: &[contract::Interaction]) -> Option<Vec<Problem>> {
  let mut problems = Vec::new();
  for (i, earlier) in interactions.iter().enumerate() {
    for (j, later) in interactions.iter().enumerate().skip(i + 1) {
      if earlier.description == later.description && earlier.states == later.states {
        problems.push(Problem {
          pointer: format!("/interactions/{j}"),
          message: format!(
            "duplicate interaction identity: same description and states as /interactions/{i}"
          ),
        });
      }
    }
  }
  (!problems.is_empty()).then_some(problems)
}

/// In-memory store of live consumer sessions, scoped to one `Engine` (one pipe, spec §7.1:
/// "session ids are engine-assigned opaque strings, unique within the life of the pipe").
#[derive(Default)]
pub(crate) struct SessionStore {
  sessions: HashMap<String, ConsumerSession>,
  next_session: u64,
}

impl SessionStore {
  /// `consumer-session/create` (spec §8.2). Returns the new session id.
  pub fn create(&mut self, consumer: Party, provider: Party, policy: Option<Value>) -> String {
    self.next_session += 1;
    let id = format!("cs-{}", self.next_session);
    self
      .sessions
      .insert(id.clone(), ConsumerSession::new(consumer, provider, policy));
    id
  }

  pub fn get_mut(&mut self, session: &str) -> Option<&mut ConsumerSession> {
    self.sessions.get_mut(session)
  }

  /// `consumer-session/finalise` (spec §8.2, §7.1): ends the session unconditionally. `None`
  /// means the id was already unknown or already ended — a caller maps that to
  /// `session-not-found`.
  pub fn end(&mut self, session: &str) -> Option<ConsumerSession> {
    self.sessions.remove(session)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn party(name: &str) -> Party {
    Party {
      name: name.to_string(),
    }
  }

  #[test]
  fn session_ids_are_distinct_and_stable() {
    let mut store = SessionStore::default();
    let a = store.create(party("web-app"), party("order-api"), None);
    let b = store.create(party("web-app"), party("order-api"), None);
    assert_ne!(a, b);
    assert!(store.get_mut(&a).is_some());
    assert!(store.get_mut(&b).is_some());
  }

  #[test]
  fn handles_are_allocated_in_submission_order() {
    let mut session = ConsumerSession::new(party("web-app"), party("order-api"), None);
    let spec = serde_json::json!({
      "description": "a request for an order",
      "parts": { "response": { "status": { "shape": "equality", "example": 200 } } }
    });
    let first = session.add_interaction(&spec).unwrap();
    let second = session.add_interaction(&spec).unwrap();
    assert_eq!(first, "i-1");
    assert_eq!(second, "i-2");
    assert_eq!(
      session.results(),
      vec![
        serde_json::json!({ "handle": "i-1", "status": "not-exercised" }),
        serde_json::json!({ "handle": "i-2", "status": "not-exercised" }),
      ]
    );
  }

  #[test]
  fn end_removes_the_session() {
    let mut store = SessionStore::default();
    let id = store.create(party("web-app"), party("order-api"), None);
    assert!(store.end(&id).is_some());
    assert!(store.get_mut(&id).is_none());
    assert!(store.end(&id).is_none());
  }

  fn degenerate_interaction() -> Value {
    serde_json::json!({
      "description": "an order",
      "parts": { "response": { "status": { "shape": "equality", "example": 200 } } }
    })
  }

  // Widened with an `optional` member so `variants` selects two: `base` and the boundary where
  // `shippedAt` is absent (same fixture shape as tests/protocol.rs's own widened interaction).
  fn widened_interaction() -> Value {
    serde_json::json!({
      "description": "an order",
      "parts": {
        "response": {
          "status": { "shape": "equality", "example": 200 },
          "body": { "shape": "object",
                    "members": { "shippedAt": { "shape": "optional",
                                                 "of": { "shape": "string", "example": "2026-07-30" } } } } } }
    })
  }

  fn variant_ids(session: &mut ConsumerSession, handle: &str) -> Vec<String> {
    let result = session.variants(handle, None).expect("variants succeeds");
    result["variants"]
      .as_array()
      .unwrap()
      .iter()
      .map(|v| v["id"].as_str().unwrap().to_string())
      .collect()
  }

  #[test]
  fn contract_is_none_when_variants_was_never_called() {
    let mut session = ConsumerSession::new(party("web-app"), party("order-api"), None);
    session.add_interaction(&degenerate_interaction()).unwrap();
    assert_eq!(session.contract().unwrap(), None);
  }

  #[test]
  fn contract_is_withheld_until_every_selected_variant_is_verified() {
    let mut session = ConsumerSession::new(party("web-app"), party("order-api"), None);
    let handle = session.add_interaction(&degenerate_interaction()).unwrap();
    variant_ids(&mut session, &handle);
    assert_eq!(
      session.contract().unwrap(),
      None,
      "selected but not yet exercised is not-exercised, same as never selected"
    );

    session.record_exercised(&handle, "base", ExchangeOutcome::Verified);
    let contract = session
      .contract()
      .unwrap()
      .expect("the only selected variant verified");
    assert_eq!(contract.interactions.len(), 1);
    assert_eq!(contract.interactions[0].selection.variants.len(), 1);
    assert_eq!(contract.interactions[0].selection.variants[0].id, "base");
  }

  #[test]
  fn contract_is_withheld_if_any_selected_variant_failed() {
    let mut session = ConsumerSession::new(party("web-app"), party("order-api"), None);
    let handle = session.add_interaction(&widened_interaction()).unwrap();
    let ids = variant_ids(&mut session, &handle);
    assert_eq!(
      ids.len(),
      2,
      "an optional member selects base plus its absent boundary"
    );

    session.record_exercised(&handle, &ids[0], ExchangeOutcome::Verified);
    session.record_exercised(&handle, &ids[1], ExchangeOutcome::Failed);
    assert_eq!(
      session.contract().unwrap(),
      None,
      "the honesty rule withholds the contract for one failure, same as for zero evidence"
    );
  }

  #[test]
  fn contract_records_only_exercised_variants_with_their_concrete_parts() {
    let mut session = ConsumerSession::new(party("web-app"), party("order-api"), None);
    let handle = session.add_interaction(&widened_interaction()).unwrap();
    let ids = variant_ids(&mut session, &handle);
    for id in &ids {
      session.record_exercised(&handle, id, ExchangeOutcome::Verified);
    }

    let contract = session
      .contract()
      .unwrap()
      .expect("every selected variant verified");
    let interaction = &contract.interactions[0];
    assert_eq!(
      interaction.parts["response"]["status"],
      serde_json::json!({ "shape": "equality", "example": 200 }),
      "the shape is recorded exactly as submitted"
    );

    let variants = &interaction.selection.variants;
    assert_eq!(
      variants.len(),
      ids.len(),
      "the honesty rule: exactly the exercised variants"
    );
    let base = variants
      .iter()
      .find(|v| v.id == "base")
      .expect("base is always selected");
    assert_eq!(base.origin, crate::variant::Origin::Base);
    assert_eq!(base.parts["response"]["status"].content, serde_json::json!(200));
    assert_eq!(
      base.parts["response"]["body"].content,
      serde_json::json!({ "shippedAt": "2026-07-30" })
    );

    let absent = variants
      .iter()
      .find(|v| v.id != "base")
      .expect("the boundary variant is also recorded");
    assert_eq!(
      absent.parts["response"]["body"].content,
      serde_json::json!({}),
      "shippedAt is omitted, not null, for the absent boundary"
    );
  }

  #[test]
  fn contract_records_a_states_literal_params_per_variant() {
    let mut session = ConsumerSession::new(party("web-app"), party("order-api"), None);
    let interaction = serde_json::json!({
      "description": "an order exists",
      "states": [ { "name": "an order exists", "params": { "id": "42" } } ],
      "parts": { "response": { "status": { "shape": "equality", "example": 200 } } }
    });
    let handle = session.add_interaction(&interaction).unwrap();
    variant_ids(&mut session, &handle);
    session.record_exercised(&handle, "base", ExchangeOutcome::Verified);

    let contract = session.contract().unwrap().expect("verified");
    let variant = &contract.interactions[0].selection.variants[0];
    let states = variant
      .states
      .as_ref()
      .expect("the interaction's state is recorded");
    assert_eq!(states[0].name, "an order exists");
    assert_eq!(states[0].params.as_ref().unwrap()["id"], serde_json::json!("42"));
  }

  #[test]
  fn results_reports_per_variant_status_after_exercising() {
    let mut session = ConsumerSession::new(party("web-app"), party("order-api"), None);
    let handle = session.add_interaction(&widened_interaction()).unwrap();
    let ids = variant_ids(&mut session, &handle);
    session.record_exercised(&handle, &ids[0], ExchangeOutcome::Verified);
    session.record_exercised(&handle, &ids[1], ExchangeOutcome::Failed);

    let results = session.results();
    assert_eq!(
      results[0]["status"], "failed",
      "any failure fails the interaction overall"
    );
    let statuses: BTreeMap<String, String> = results[0]["variants"]
      .as_array()
      .unwrap()
      .iter()
      .map(|v| {
        (
          v["variant"].as_str().unwrap().to_string(),
          v["status"].as_str().unwrap().to_string(),
        )
      })
      .collect();
    assert_eq!(statuses[&ids[0]], "verified");
    assert_eq!(statuses[&ids[1]], "failed");
  }

  #[test]
  fn contract_rejects_duplicate_interaction_identity() {
    let mut session = ConsumerSession::new(party("web-app"), party("order-api"), None);
    let h1 = session.add_interaction(&degenerate_interaction()).unwrap();
    let h2 = session.add_interaction(&degenerate_interaction()).unwrap();
    variant_ids(&mut session, &h1);
    variant_ids(&mut session, &h2);
    session.record_exercised(&h1, "base", ExchangeOutcome::Verified);
    session.record_exercised(&h2, "base", ExchangeOutcome::Verified);

    let problems = session
      .contract()
      .expect_err("both interactions share a description and an empty state list");
    assert_eq!(problems.len(), 1);
    assert_eq!(problems[0].pointer, "/interactions/1");
  }

  #[test]
  fn a_session_built_contract_round_trips_through_canonical_bytes() {
    let mut session = ConsumerSession::new(party("web-app"), party("order-api"), None);
    let handle = session.add_interaction(&widened_interaction()).unwrap();
    let ids = variant_ids(&mut session, &handle);
    for id in &ids {
      session.record_exercised(&handle, id, ExchangeOutcome::Verified);
    }
    let contract = session
      .contract()
      .unwrap()
      .expect("every selected variant verified");

    let first = crate::contract::write_canonical(&contract).expect("writes");
    let second = crate::contract::write_canonical(&contract).expect("writes");
    assert_eq!(
      first, second,
      "contract-file spec §2.4: the same content written twice MUST produce the same bytes"
    );

    let read_back =
      crate::contract::read(&first, crate::contract::IdentifyMode::Strict).expect("reads its own output");
    assert_eq!(contract, read_back);
  }
}
