//! Consumer sessions (engine-protocol spec §7, §8.2): the only resource a host can hold.
//! Everything a session allocates — here, its interactions — is released when the session ends;
//! there is no per-object cleanup call (spec §7.1).

use super::background::{Background, SpawnError, Task};
use super::exchange::{self, ArmedExchange, Exchange};
use super::scope::Scope;
use crate::common::{ContentTypes, declared_type};
use crate::component::{ComponentError, Parts, Start, Stop, TransportComponent, Unavailable};
use crate::contract::{self, Contract, Party};
use crate::error::Problem;
use crate::interaction_spec::{self, InteractionSpec, InteractionSpecError};
use crate::plan::{self, Assignment, Plan};
use crate::variant::generate::Generated;
use crate::variant::{Selected, SelectionReport, VariantError, generate, params, select};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// A generated payload: `part -> slot -> value`, a slot's value a document or octets (S2.15).
pub(crate) type Payload = BTreeMap<String, BTreeMap<String, Generated>>;

/// The variant a `serve-variant` call has most recently armed for one interaction (variant-
/// semantics spec §4.1): its id, and the concrete payload the generator produced for it.
pub struct Armed {
  pub variant: String,
  #[allow(dead_code)]
  pub(crate) parts: Arc<Payload>,
}

/// What arming one variant takes, worked out the first time it is armed and shared by every arming
/// after (staged plan S2.3, Phase 9 finding 28). The selection is fixed once `variants` answers, and
/// a variant's id names its assignment (variant-semantics spec §2.2), so none of it can change. A
/// generator's fresh values are not here: they are filled each time a reply is served (ADR 0048).
pub(crate) struct Arming {
  /// The payload the generator produced for the variant.
  parts: Arc<Payload>,
  /// The interaction's plan pinned to the variant, for a passive interaction.
  request_plan: Option<Arc<Plan>>,
  /// The variant's resolved provider-state parameters, which a reply's generators read.
  provider_state: Arc<serde_json::Map<String, Value>>,
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
  /// The test's verdict on each variant, keyed by variant id, as the latest `report-variant` gave
  /// it (ADR 0064).
  pub reports: BTreeMap<String, TestVerdict>,
  /// Each variant's [`Arming`], by variant id, from the first time it was armed.
  armings: HashMap<String, Arming>,
  /// The parts whose shapes name a generator (ADR 0048), which a reply is filled from fresh.
  fresh: Arc<BTreeMap<String, contract::ShapePart>>,
  /// The interaction's declared content types, as an exchange reads them.
  content_types: Arc<ContentTypes>,
  /// Converted from a v1–v4 interaction (`add-pact-interaction`, ADR 0066): its selection is the
  /// one `upgrade` records, a `base` variant that pins nothing, whatever policy a host asks for, and
  /// that variant is the interaction's own example.
  converted: Option<Converted>,
}

/// A converted interaction's example (ADR 0066 decisions 3 and 4), as today's mock server serves it
/// and as `upgrade` records it.
struct Converted {
  /// What a mock serves: the example's slots, decoded as a reply's are, over what the shapes generate.
  served: Arc<Payload>,
  /// What the contract records as the base variant's values: the example as `upgrade` records it,
  /// so a verifier replays the request today's verifier sends, octets and all.
  recorded: BTreeMap<String, contract::ValuePart>,
}

/// The test's verdict on one variant (ADR 0064 decision 1): whether it passed, and what it failed
/// with. Any status but `passed` is a failure, so an unknown verdict verifies nothing.
#[derive(Debug, Clone)]
pub(crate) struct TestVerdict {
  pub passed: bool,
  /// The status as the host sent it, which a result repeats.
  pub status: String,
  pub reason: Option<String>,
}

impl TestVerdict {
  pub fn new(status: String, reason: Option<String>) -> Self {
    TestVerdict {
      passed: status == "passed",
      status,
      reason,
    }
  }

  fn to_json(&self) -> Value {
    match &self.reason {
      Some(reason) => serde_json::json!({ "status": self.status, "reason": reason }),
      None => serde_json::json!({ "status": self.status }),
    }
  }
}

/// A variant's status in `results` and `report-variant`'s answer, decided in the order spec §8.2
/// gives (ADR 0064 decision 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VariantStatus {
  Failed,
  NotExercised,
  NotReported,
  Verified,
}

impl VariantStatus {
  fn as_str(self) -> &'static str {
    match self {
      VariantStatus::Failed => "failed",
      VariantStatus::NotExercised => "not-exercised",
      VariantStatus::NotReported => "not-reported",
      VariantStatus::Verified => "verified",
    }
  }
}

/// The evidence one exercised variant produced (variant-semantics spec §4.4): whether the
/// exchange it drove came back matching, and the concrete payload it produced — the same
/// `part -> slot -> value` document [`Armed`] carries, generated the same way.
pub(crate) struct Exercised {
  pub outcome: ExchangeOutcome,
  pub parts: Arc<Payload>,
  /// Why a `Failed` exchange failed, as the protocol carries mismatches ([`super::wire::mismatch_json`]):
  /// `finalise` reports them per variant (spec §8.2, "unmatched-request … detail rides in
  /// `results`"), so a host can say what went wrong without reading the engine's log.
  pub mismatches: Vec<Value>,
}

impl Exercised {
  /// One more exchange of the same variant (ADR 0069 decision 1): failed if either failed, with
  /// every failed exchange's mismatches.
  pub fn merge(&mut self, later: Exercised) {
    if later.outcome == ExchangeOutcome::Failed {
      self.outcome = ExchangeOutcome::Failed;
    }
    self.mismatches.extend(later.mismatches);
  }
}

/// Record `exercised` as one more of `variant_id`'s exchanges.
fn record(exercised_by: &mut BTreeMap<String, Exercised>, variant_id: String, exercised: Exercised) {
  match exercised_by.entry(variant_id) {
    std::collections::btree_map::Entry::Occupied(mut earlier) => earlier.get_mut().merge(exercised),
    std::collections::btree_map::Entry::Vacant(first) => {
      first.insert(exercised);
    }
  }
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
  exchange: Arc<Exchange>,
  task: Option<Task>,
}

/// `consumer-session/variants` (variant-semantics spec §3.9): a malformed policy vs. an unknown
/// handle are different failures at the protocol boundary.
#[derive(Debug)]
pub enum VariantsError {
  HandleNotFound,
  Variant(VariantError),
}

/// `consumer-session/start-transport` (spec §8.2): the component refused to start, or it started and
/// the embedding could not start the exchange loop that drives it (ADR 0062).
#[derive(Debug)]
pub enum StartTransportError {
  Component(ComponentError),
  Spawn(SpawnError),
}

#[derive(Debug)]
pub enum ServeVariantError {
  HandleNotFound,
  VariantNotFound {
    selection: Vec<String>,
  },
  /// An emissive interaction whose kind a started transport serves: delivering through it is S6.2's
  /// (ADR 0067 decision 5), and until it is built the engine says so rather than claim a delivery.
  DeliveryUnsupported {
    kind: String,
  },
}

/// `consumer-session/report-variant` (ADR 0064) fails as `serve-variant` does.
pub type ReportVariantError = ServeVariantError;

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
  /// loop (plan task 4.5). A session MAY start several; `serve_variant` arms every one of the
  /// interaction's transport kind.
  transports: Vec<TransportRun>,
  /// The components this session can call (component-interfaces spec §10): the ones `create`'s
  /// `config.components` declared, ahead of the embedding's in-tree ones. They end with the session.
  scope: Scope,
  /// Whether the host that created this session declared `test-verdicts` (ADR 0064 decision 4): an
  /// exercised variant with no report is then `not-reported`, and a refused interaction withholds
  /// the contract.
  verdicts_required: bool,
  /// Every `add-interaction` this session refused, as `finalise`'s `refused` lists them.
  refused: Vec<Value>,
}

/// Why `add-interaction` refused an interaction: it is malformed (`interaction-invalid`), or it
/// needs a component this session does not have (`component-unavailable`, spec §2.3).
#[derive(Debug)]
pub enum AddInteractionError {
  Invalid(InteractionSpecError),
  Unavailable(Unavailable),
}

impl From<InteractionSpecError> for AddInteractionError {
  fn from(err: InteractionSpecError) -> Self {
    AddInteractionError::Invalid(err)
  }
}

impl ConsumerSession {
  fn new(
    consumer: Party,
    provider: Party,
    policy: Option<Value>,
    scope: Scope,
    verdicts_required: bool,
  ) -> Self {
    ConsumerSession {
      consumer,
      provider,
      policy,
      interactions: BTreeMap::new(),
      order: Vec::new(),
      next_handle: 1,
      transports: Vec::new(),
      scope,
      verdicts_required,
      refused: Vec::new(),
    }
  }

  /// Record that `add-interaction` refused `interaction` with `code` (ADR 0064 decision 4).
  pub fn record_refusal(&mut self, interaction: &Value, code: &str, message: &str) {
    let mut refusal = serde_json::Map::new();
    if let Some(description) = interaction.get("description").and_then(Value::as_str) {
      refusal.insert("description".to_string(), Value::from(description));
    }
    refusal.insert("code".to_string(), Value::from(code));
    refusal.insert("message".to_string(), Value::from(message));
    self.refused.push(Value::Object(refusal));
  }

  /// `finalise`'s `refused` (ADR 0064 decision 4): `None` to a host that did not declare
  /// `test-verdicts`, whose results stay protocol version 1's.
  pub fn refused(&self) -> Option<&[Value]> {
    self.verdicts_required.then_some(self.refused.as_slice())
  }

  /// `consumer-session/add-interaction` (spec §8.2): validate and compile, or a structured error a
  /// caller maps to `interaction-invalid` or `component-unavailable`.
  pub fn add_interaction(&mut self, interaction: &Value) -> Result<String, AddInteractionError> {
    let mut spec = interaction_spec::parse(interaction)?;
    // Component-interfaces spec §2.3: what an interaction cannot run without is checked when it
    // arrives, while the author is looking at it — not when its first body does.
    // So is any plan fragment a declared component contributes for its slots (plan task 8.4): one
    // written against a grammar this engine cannot read fails here, naming the skew.
    let raw_parts = raw_parts(interaction);
    self
      .scope
      .check(
        spec.requires.iter().flatten(),
        spec.content_types.as_ref(),
        &raw_parts,
      )
      .map_err(AddInteractionError::Unavailable)?;
    // Variant-bound state bindings are validated here and nowhere later (variant-semantics spec
    // §6.5): the reference resolves against this interaction's variant space, which does not exist
    // until the shapes parse, and the author is looking at the DSL that produced it right now.
    // What is kept is the resolved id, not the author's shorthand (§6.3): the contract records
    // `response.body.shippedAt#presence` where the DSL wrote `shippedAt`, so a verifier reading it
    // resolves nothing and cannot resolve it differently.
    if let Some(states) = &spec.states {
      let space = plan::variant_space(&spec);
      match params::bind(states, &space, "/states") {
        Ok(bound) => spec.states = Some(params::with_resolved_dimensions(states, &bound)),
        Err(problems) => return Err(InteractionSpecError { problems }.into()),
      }
    }
    let contributions = self.scope.contributions();
    let compiled = contributions.compile(&spec, &raw_parts, &Assignment::new(), None);
    let handle = format!("i-{}", self.next_handle);
    self.next_handle += 1;
    let fresh = raw_parts
      .iter()
      .filter(|(_, slots)| slots.values().any(crate::shape::fresh::has_generators))
      .map(|(name, slots)| (name.clone(), slots.clone()))
      .collect();
    let content_types = Arc::new(spec.content_types.clone().unwrap_or_default());
    self.interactions.insert(
      handle.clone(),
      InteractionEntry {
        spec,
        plan: compiled,
        raw_parts,
        selection: None,
        armed: None,
        exercised: BTreeMap::new(),
        reports: BTreeMap::new(),
        armings: HashMap::new(),
        fresh: Arc::new(fresh),
        content_types,
        converted: None,
      },
    );
    self.order.push(handle.clone());
    Ok(handle)
  }

  /// `add-pact-interaction` (ADR 0066): an interaction specification converted from a v1–v4
  /// interaction, added as [`Self::add_interaction`] adds one. Its one variant is the example, as
  /// an upgraded pact's is (variant-semantics spec §9): `base`, pinning no dimension, so an
  /// `each-like` admits any length its rules do, as the pact's matching rules did, rather than the
  /// one element a pinned base point would (ADR 0024). The mock serves it `example`, the
  /// interaction's own, as today's mock server does, and the contract records `example` as its
  /// values, as `upgrade` does.
  pub fn add_converted_interaction(
    &mut self,
    interaction: &Value,
    example: &BTreeMap<String, contract::ValuePart>,
  ) -> Result<String, AddInteractionError> {
    let handle = self.add_interaction(interaction)?;
    let content = self.scope.content();
    if let Some(entry) = self.interactions.get_mut(&handle) {
      // The example's slots as a mock serves them (`wire::served_slot`), over what the shapes
      // generate: octets no component decodes are served as those octets (S2.15).
      let mut served = generate::interaction(&entry.spec, &Assignment::new());
      for (part, slots) in example {
        for (slot, value) in slots {
          let declared = declared_type(Some(&entry.content_types), part, slot);
          if let Some(generated) = super::wire::served_slot(value, declared, content.as_deref()) {
            served
              .entry(part.clone())
              .or_default()
              .insert(slot.clone(), generated);
          }
        }
      }
      entry.converted = Some(Converted {
        served: Arc::new(served),
        recorded: example.clone(),
      });
    }
    Ok(handle)
  }

  /// A transport this session's project declared, contributing `kind` (plan task 8.3).
  pub fn transport(&self, kind: &str) -> Option<Arc<dyn TransportComponent>> {
    self.scope.transport(kind)
  }

  /// `consumer-session/start-transport` (spec §8.2): starts `component` under engine-assigned
  /// `instance`, then asks `background` to run the exchange loop (plan task 4.5, [`exchange::run`])
  /// that drives it — `poll-inbound` waits, so nothing on the dispatch thread can pump it
  /// (ADR 0062). Returns the transport's endpoint descriptor.
  pub fn start_transport(
    &mut self,
    kind: &str,
    component: Arc<dyn TransportComponent>,
    instance: String,
    options: Option<Value>,
    background: &dyn Background,
  ) -> Result<Value, StartTransportError> {
    let result = component
      .start(Start {
        instance: instance.clone(),
        kind: kind.to_string(),
        role: "serve".to_string(),
        options,
      })
      .map_err(StartTransportError::Component)?;

    let stop = Arc::new(AtomicBool::new(false));
    let exchange = Arc::new(Exchange::default());
    let thread_component = Arc::clone(&component);
    let thread_stop = Arc::clone(&stop);
    let thread_exchange = Arc::clone(&exchange);
    let thread_instance = instance.clone();
    let content = self.scope.content();
    let contributions = self.scope.contributions();
    let spawned = background.spawn(
      &format!("janus exchange {instance}"),
      Box::new(move || {
        exchange::run(
          thread_component,
          content,
          contributions,
          thread_instance,
          thread_stop,
          thread_exchange,
        );
      }),
    );
    let task = match spawned {
      Ok(task) => task,
      Err(err) => {
        // Nothing will drive the instance, so it must not outlive this answer.
        let _ = component.stop(Stop {
          instance: instance.clone(),
        });
        return Err(StartTransportError::Spawn(err));
      }
    };

    self.transports.push(TransportRun {
      instance,
      kind: kind.to_string(),
      component,
      stop,
      exchange,
      task: Some(task),
    });
    Ok(result.endpoint)
  }

  /// `consumer-session/finalise` (spec §7.1, §8.2): "transports stopped, all state released."
  /// Signals every bound transport's exchange loop to stop, tells the component itself to stop
  /// (which also unblocks a loop mid-`poll-inbound`, since the instance then stops existing),
  /// waits for each loop to end, and drains its recorded evidence into the interactions it belongs to
  /// — after this, `results`/`contract` see exactly what really happened on the wire.
  pub fn stop_transports(&mut self) {
    for mut run in self.transports.drain(..) {
      run.stop.store(true, Ordering::Relaxed);
      let _ = run.component.stop(Stop {
        instance: run.instance.clone(),
      });
      if let Some(task) = run.task.take() {
        let _ = task.join();
      }
      let outcomes = std::mem::take(
        &mut run
          .exchange
          .state
          .lock()
          .expect("exchange state lock poisoned")
          .outcomes,
      );
      for ((handle, variant_id), exercised) in outcomes {
        if let Some(entry) = self.interactions.get_mut(&handle) {
          record(&mut entry.exercised, variant_id, exercised);
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
    let selected = if entry.converted.is_some() {
      // The selection `upgrade` records for a converted interaction (ADR 0066 decision 3).
      let policy = crate::variant::SamplingPolicy {
        strategy: "base-only".to_string(),
        boundaries: false,
        ..crate::variant::SamplingPolicy::default()
      };
      let mut selected = select(&space, &policy).map_err(VariantsError::Variant)?;
      for variant in &mut selected.variants {
        variant.assignment.clear();
      }
      selected
    } else {
      let layers = [self.policy.as_ref(), call_policy];
      let policy = crate::variant::SamplingPolicy::resolve(&layers)
        .map_err(|problem| VariantsError::Variant(VariantError::InvalidPolicy(vec![problem])))?;
      select(&space, &policy).map_err(VariantsError::Variant)?
    };

    let variants: Vec<Value> = selected.variants.iter().map(|v| v.to_json(&space)).collect();
    let result = serde_json::json!({ "variants": variants, "report": selected.report.to_json() });

    let entry = self.interactions.get_mut(handle).expect("checked above");
    entry.selection = Some(selected);
    Ok(result)
  }

  /// `consumer-session/serve-variant` (spec §4.1, §8.2): arm the named variant of a previously
  /// selected interaction, generating its concrete payload now (plan task 4.3's generator). For a
  /// passive interaction with a transport bound, this also arms every transport of its kind's
  /// exchange loop (plan task 4.5) with the pinned request plan to match and the response to
  /// answer with; an emissive interaction, or one with no transport bound, stops at recording the
  /// arming, exactly as before.
  ///
  /// An emissive interaction with no transport of its kind started is handed to the host instead
  /// (ADR 0067 decision 5): the answer is the variant's parts, as a transport would carry them, and
  /// the variant is exercised, its test's verdict deciding it.
  pub fn serve_variant(
    &mut self,
    handle: &str,
    variant_id: &str,
  ) -> Result<Option<Parts>, ServeVariantError> {
    let contributions = self.scope.contributions();
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
    // Arm every bound transport of the interaction's own kind. Until plan task 8.3 this read
    // `kind == "http"` — HTTP knowledge in the kernel, which no transport of another kind could
    // get past (kernel-boundary review, finding 7).
    let passive_kind = entry
      .spec
      .transport
      .as_ref()
      .filter(|t| t.mode.as_deref() != Some("emissive"))
      .map(|t| t.kind.clone());
    if !entry.armings.contains_key(variant_id) {
      let arming = Arming {
        parts: match &entry.converted {
          Some(converted) => Arc::clone(&converted.served),
          None => Arc::new(generate::interaction(&entry.spec, &assignment)),
        },
        request_plan: passive_kind.is_some().then(|| {
          Arc::new(contributions.compile(&entry.spec, &entry.raw_parts, &assignment, Some(variant_id)))
        }),
        provider_state: Arc::new(provider_state(&entry.spec, &assignment)),
      };
      entry.armings.insert(variant_id.to_string(), arming);
    }
    let arming = &entry.armings[variant_id];
    entry.armed = Some(Armed {
      variant: variant_id.to_string(),
      parts: Arc::clone(&arming.parts),
    });

    let emissive = entry
      .spec
      .transport
      .as_ref()
      .filter(|t| t.mode.as_deref() == Some("emissive"))
      .map(|t| t.kind.clone());
    if let Some(kind) = emissive {
      if self.transports.iter().any(|run| run.kind == kind) {
        return Err(ServeVariantError::DeliveryUnsupported { kind });
      }
      let delivered = exchange::fresh_payload(
        &arming.parts,
        &entry.fresh,
        &arming.provider_state,
        handle,
        variant_id,
      );
      let parts = exchange::wire_parts(&delivered, &entry.content_types, self.scope.content().as_deref());
      entry.exercised.insert(
        variant_id.to_string(),
        Exercised {
          outcome: ExchangeOutcome::Verified,
          parts: Arc::clone(&arming.parts),
          mismatches: Vec::new(),
        },
      );
      return Ok(Some(parts));
    }

    if let (Some(kind), Some(request_plan)) = (passive_kind, &arming.request_plan) {
      for run in self.transports.iter().filter(|run| run.kind == kind) {
        let mut state = run.exchange.state.lock().expect("exchange state lock poisoned");
        state.arm(ArmedExchange {
          handle: handle.to_string(),
          variant_id: variant_id.to_string(),
          request_plan: Arc::clone(request_plan),
          response_parts: Arc::clone(&arming.parts),
          content_types: Arc::clone(&entry.content_types),
          fresh: Arc::clone(&entry.fresh),
          provider_state: Arc::clone(&arming.provider_state),
        });
      }
    }
    Ok(None)
  }

  /// `consumer-session/report-variant` (ADR 0064 decision 1): the test's verdict on one variant of a
  /// selected interaction. It ends the variant's run: every transport waits out an exchange of it
  /// under way and disarms it, and what they recorded moves into the interaction now rather than at
  /// `finalise`. Answers the variant's result as `results` would report it.
  pub fn report_variant(
    &mut self,
    handle: &str,
    variant_id: &str,
    verdict: TestVerdict,
  ) -> Result<Value, ReportVariantError> {
    let Some(entry) = self.interactions.get(handle) else {
      return Err(ReportVariantError::HandleNotFound);
    };
    let selection: Vec<String> = entry
      .selection
      .as_ref()
      .map(|selection| selection.variants.iter().map(|v| v.id.clone()).collect())
      .unwrap_or_default();
    if !selection.iter().any(|id| id == variant_id) {
      return Err(ReportVariantError::VariantNotFound { selection });
    }
    let settled: Vec<Exercised> = self
      .transports
      .iter()
      .filter_map(|run| run.exchange.settle(handle, variant_id))
      .collect();
    let entry = self.interactions.get_mut(handle).expect("checked above");
    for exercised in settled {
      record(&mut entry.exercised, variant_id.to_string(), exercised);
    }
    if entry
      .armed
      .as_ref()
      .is_some_and(|armed| armed.variant == variant_id)
    {
      entry.armed = None;
    }
    entry.reports.insert(variant_id.to_string(), verdict);
    let entry = &self.interactions[handle];
    Ok(self.variant_result(entry, variant_id).1)
  }

  /// One variant's status, in the order spec §8.2 gives (ADR 0064 decision 2).
  fn variant_status(&self, entry: &InteractionEntry, variant_id: &str) -> VariantStatus {
    let report = entry.reports.get(variant_id);
    if report.is_some_and(|report| !report.passed) {
      return VariantStatus::Failed;
    }
    match entry.exercised.get(variant_id).map(|e| e.outcome) {
      Some(ExchangeOutcome::Failed) => VariantStatus::Failed,
      None => VariantStatus::NotExercised,
      Some(ExchangeOutcome::Verified) if self.verdicts_required && report.is_none() => {
        VariantStatus::NotReported
      }
      Some(ExchangeOutcome::Verified) => VariantStatus::Verified,
    }
  }

  /// One variant's status and its `VariantResult` document.
  fn variant_result(&self, entry: &InteractionEntry, variant_id: &str) -> (VariantStatus, Value) {
    let status = self.variant_status(entry, variant_id);
    let mut result = serde_json::Map::new();
    result.insert("variant".to_string(), Value::from(variant_id));
    result.insert("status".to_string(), Value::from(status.as_str()));
    if let Some(exercised) = entry
      .exercised
      .get(variant_id)
      .filter(|e| !e.mismatches.is_empty())
    {
      result.insert(
        "mismatches".to_string(),
        Value::from(exercised.mismatches.clone()),
      );
    }
    if let Some(report) = entry.reports.get(variant_id) {
      result.insert("test".to_string(), report.to_json());
    }
    (status, Value::Object(result))
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
        parts: Arc::new(parts),
        mismatches: Vec::new(),
      },
    );
  }

  /// `consumer-session/finalise`'s `results` (spec §8.2): one entry per interaction, in
  /// submission order, with a `variants` breakdown once `variants` has been called for it
  /// (variant-semantics spec §4.2: every selected variant is required, and honestly
  /// `not-exercised` until something exercises it). An interaction is `failed` when any variant
  /// failed, else the first of `not-exercised` and `not-reported` any variant is, else `verified`.
  pub fn results(&self) -> Vec<Value> {
    self
      .order
      .iter()
      .map(|handle| {
        let entry = &self.interactions[handle];
        match &entry.selection {
          None => serde_json::json!({ "handle": handle, "status": "not-exercised" }),
          Some(selected) => {
            let (statuses, variants): (Vec<VariantStatus>, Vec<Value>) = selected
              .variants
              .iter()
              .map(|v| self.variant_result(entry, &v.id))
              .unzip();
            let status = statuses
              .iter()
              .copied()
              .min_by_key(|status| *status as u8)
              .unwrap_or(VariantStatus::Verified);
            serde_json::json!({ "handle": handle, "status": status.as_str(), "variants": variants })
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
    if self.verdicts_required && !self.refused.is_empty() {
      return Ok(None);
    }
    let mut interactions = Vec::with_capacity(self.order.len());
    for handle in &self.order {
      let entry = &self.interactions[handle];
      let Some(selected) = &entry.selection else {
        return Ok(None);
      };
      let all_verified = selected
        .variants
        .iter()
        .all(|v| self.variant_status(entry, &v.id) == VariantStatus::Verified);
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
            parts: match &entry.converted {
              Some(converted) => converted.recorded.clone(),
              None => to_value_parts(&exercised.parts, entry.spec.content_types.as_ref()),
            },
          }
        })
        .collect();

      interactions.push(contract::Interaction {
        description: entry.spec.description.clone(),
        transport: entry.spec.transport.clone(),
        states: entry.spec.states.clone(),
        parts: entry.raw_parts.clone(),
        content_types: entry.spec.content_types.clone(),
        requires: entry.spec.requires.clone(),
        selection: contract::RecordedSelection {
          variants,
          report: report_to_map(&selected.report),
        },
      });
    }

    // Identity order (contract-file spec §2.4, ADR 0065), before the check, so its pointers are
    // positions in the array that is written.
    contract::merge::sort(&mut interactions);
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

/// What a reply's generators read for a variant (ADR 0048): its resolved state parameters.
fn provider_state(spec: &InteractionSpec, assignment: &Assignment) -> serde_json::Map<String, Value> {
  let space = plan::variant_space(spec);
  let mut provider_state = serde_json::Map::new();
  for state in params::resolve_states(spec.states.as_ref(), &space, assignment)
    .into_iter()
    .flatten()
  {
    provider_state.extend(state.params.into_iter().flatten());
  }
  provider_state
}

/// An `add-interaction` request's `parts` member, taken verbatim (contract-file spec §5.1: "which
/// parts exist and which slots they expose is the transport and content components' business,
/// never the kernel's" — a shape is opaque here, so there is nothing to reconstruct). `parse` has
/// already rejected anything not shaped this way by the time this is called, so the fallbacks
/// below never actually trigger; they just avoid a panic on a document this function is not the
/// one validating.
pub(crate) fn raw_parts(interaction: &Value) -> BTreeMap<String, contract::ShapePart> {
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
/// §5.3): a document as-is with no `encoded` tag, which is the default `json` encoding, and octets
/// as `base64` (S2.15), so a verifier replays the octets themselves. A declared content slot also
/// carries its type (§5.5), which is how a verifier knows which component turns the recorded
/// document back into the octets the consumer sent.
fn to_value_parts(
  parts: &Payload,
  content_types: Option<&ContentTypes>,
) -> BTreeMap<String, contract::ValuePart> {
  parts
    .iter()
    .map(|(part, slots)| {
      let slots = slots
        .iter()
        .map(|(slot, value)| {
          let content_type = declared_type(content_types, part, slot);
          let recorded = match value {
            Generated::Document(document) => contract::SlotValue {
              content: document.clone(),
              encoded: None,
              content_type: content_type.map(str::to_string),
            },
            Generated::Octets(bytes) => super::wire::octets_slot(bytes, content_type),
          };
          (slot.clone(), recorded)
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
  pub fn create(
    &mut self,
    consumer: Party,
    provider: Party,
    policy: Option<Value>,
    scope: Scope,
    verdicts_required: bool,
  ) -> String {
    self.next_session += 1;
    let id = format!("cs-{}", self.next_session);
    self.sessions.insert(
      id.clone(),
      ConsumerSession::new(consumer, provider, policy, scope, verdicts_required),
    );
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

  /// `engine/shutdown` (spec §6): ends every session, stopping its transports, and reports none of
  /// them — a host that wanted results would have finalised first.
  pub fn end_all(&mut self) {
    for (_, mut session) in self.sessions.drain() {
      session.stop_transports();
    }
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
    let a = store.create(party("web-app"), party("order-api"), None, Scope::none(), false);
    let b = store.create(party("web-app"), party("order-api"), None, Scope::none(), false);
    assert_ne!(a, b);
    assert!(store.get_mut(&a).is_some());
    assert!(store.get_mut(&b).is_some());
  }

  #[test]
  fn handles_are_allocated_in_submission_order() {
    let mut session = ConsumerSession::new(party("web-app"), party("order-api"), None, Scope::none(), false);
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
    let id = store.create(party("web-app"), party("order-api"), None, Scope::none(), false);
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
    let mut session = ConsumerSession::new(party("web-app"), party("order-api"), None, Scope::none(), false);
    session.add_interaction(&degenerate_interaction()).unwrap();
    assert_eq!(session.contract().unwrap(), None);
  }

  #[test]
  fn contract_is_withheld_until_every_selected_variant_is_verified() {
    let mut session = ConsumerSession::new(party("web-app"), party("order-api"), None, Scope::none(), false);
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

  /// S2.3, Phase 9 finding 28: a variant is generated and compiled the first time it is armed, and
  /// every later arming shares that work rather than repeating it.
  #[test]
  fn re_arming_a_variant_reuses_what_its_first_arming_built() {
    let mut session = ConsumerSession::new(party("web-app"), party("order-api"), None, Scope::none(), false);
    let handle = session.add_interaction(&widened_interaction()).unwrap();
    let ids = variant_ids(&mut session, &handle);

    session.serve_variant(&handle, &ids[0]).unwrap();
    let first = Arc::clone(&session.interactions[&handle].armed.as_ref().unwrap().parts);
    session.serve_variant(&handle, &ids[1]).unwrap();
    session.serve_variant(&handle, &ids[0]).unwrap();
    let again = &session.interactions[&handle].armed.as_ref().unwrap().parts;

    assert!(
      Arc::ptr_eq(&first, again),
      "the second arming of a variant generated nothing new"
    );
    assert_eq!(
      session.interactions[&handle].armings.len(),
      2,
      "one arming per variant armed"
    );
  }

  #[test]
  fn contract_is_withheld_if_any_selected_variant_failed() {
    let mut session = ConsumerSession::new(party("web-app"), party("order-api"), None, Scope::none(), false);
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
    let mut session = ConsumerSession::new(party("web-app"), party("order-api"), None, Scope::none(), false);
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
    let mut session = ConsumerSession::new(party("web-app"), party("order-api"), None, Scope::none(), false);
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
    let mut session = ConsumerSession::new(party("web-app"), party("order-api"), None, Scope::none(), false);
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
    let mut session = ConsumerSession::new(party("web-app"), party("order-api"), None, Scope::none(), false);
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
    let mut session = ConsumerSession::new(party("web-app"), party("order-api"), None, Scope::none(), false);
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
