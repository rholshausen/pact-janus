//! Verification sessions and the run they drive (engine-protocol spec §7.3, §8.3; plan task 5.1):
//! the provider side of the same engine, replaying a contract's recorded variants at a running
//! provider and reporting every outcome as events.
//!
//! **The verifier is the consumer session turned around, and that is load-bearing.** It compiles
//! the *same* interaction document through the *same* plan compiler, resolves slots through the
//! *same* wire helpers ([`super::wire`]), and scores the result with the *same* interpreter. The
//! only differences are direction — `transport/send` where the mock polls, the `response` subtree
//! where the mock reads `request` — and which shape is pinned. If matching behaviour ever diverged
//! between the two sides, a green consumer test and a green verification would stop meaning the
//! same thing, which is the claim the whole design rests on.
//!
//! **Replay is by example** (variant-semantics spec §5.2): the provider sees the bytes the
//! consumer actually sent, taken from the recorded variant, never a re-derivation from the shape.
//! Nothing here re-samples (§5.1) — the contract is the sample, and the run replays all of it in
//! recorded order.
//!
//! **The run owns a thread**, because `verify` must return before the run finishes (spec §8.3) and
//! a replayed request blocks. The same `wasm32-wasip2` caveat [`super::exchange`] documents applies
//! here and for the same reason: this module compiles for that target, but `thread::spawn` has
//! nothing to schedule onto without the threads proposal, so a WASM-component embedding needs
//! either wasi-threads or a single-threaded run model. Every embedding wired up today (the CLI,
//! the subprocess binary, the integration tests) is native, so this is a documented gap rather
//! than a silent one.
//!
//! Provider states are **resolved here, per variant** (variant-semantics spec §6.4, plan task
//! 5.2) rather than read from the contract's recorded values: resolution is a total function of
//! the assignment and the binding, so the verifier's own answer must equal the consumer's, and
//! checking that is free. A disagreement is reported as a warning naming both — it means the two
//! ends ran different implementations, which is worth knowing and is not the provider's fault.
//!
//! **A v1–v4 pact is a source too** (plan task 5.4), and it is deliberately *not* upgraded on the
//! way in. Each of its interactions is compiled by design 3.5's own matching-rule compiler — the
//! single home of v1–v4 cascading and precedence — and its recorded request is replayed verbatim.
//! Nothing on that path invents a shape, so nothing on it can lose one, which is what "providers
//! upgrade first at no cost" has to mean to be worth claiming. The two paths meet at [`Prepared`]:
//! past that point there is one loop, one hook sequence and one result vocabulary, so "the same
//! engine verifies both" is a property of the code rather than a claim about it. What differs is
//! confined to the two preparers — where the plan comes from, where the request comes from, and
//! how a reply's values are read back.
//!
//! What this module still does not do, and a later task picks up: `upgrade/pact` (5.5), which
//! turns a pact's matching rules into *shapes*. That is a different and lossier operation than
//! verifying the pact where it stands, and it is the reason this path exists at all.

use super::events::Stream;
use super::wire::{find_container, mismatch_json, parts_resolver};
use crate::component::{ContentComponent, Parts, Send as SendRequest, Stop, TransportComponent};
use crate::contract::{self, Contract};
use crate::error::Problem;
use crate::hooks::{HookRunner, Occurrence, PointOutcome};
use crate::interaction_spec::{self, InteractionSpec};
use crate::legacy_pact::{self, LegacyInteraction};
use crate::plan::{self, Assignment, CapturedValues, Status, execute, outcome};
use crate::variant::params;
use serde::Deserialize;
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::thread::{self, JoinHandle};

/// Which recorded variants a run replays (variant-semantics spec §5.1). The default is all of
/// them; a host may narrow it for debugging, and a narrowed run **must be reported as filtered** —
/// a partial run is not a pass, and a summary that did not say so would read like one.
#[derive(Debug, Clone, Default)]
struct Filter {
  variants: Option<Vec<String>>,
}

/// When a run emits `verification/executed-plan` (spec §9.6) — the event `explain --executed`
/// reads. Off by default: an executed plan is the largest document a run produces, and a host
/// that wanted one asks.
///
/// `on-failure` is the setting that matters, and the reason the event exists at all: the moment a
/// user wants the executed plan is the moment something failed, and making them re-run the
/// interaction by hand against a values file they had to write is exactly the diagnosis experience
/// the RFC complains about.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum ExecutedPlans {
  #[default]
  Never,
  OnFailure,
  Always,
}

impl ExecutedPlans {
  fn from_options(options: Option<&Value>) -> ExecutedPlans {
    match options
      .and_then(|options| options.get("executed-plan"))
      .and_then(Value::as_str)
    {
      Some("always") => ExecutedPlans::Always,
      Some("on-failure") => ExecutedPlans::OnFailure,
      Some(other) => {
        tracing::warn!(value = %other, "unknown 'executed-plan' option; no executed plans will be emitted");
        ExecutedPlans::Never
      }
      None => ExecutedPlans::Never,
    }
  }

  fn wants(&self, matched: bool) -> bool {
    match self {
      ExecutedPlans::Never => false,
      ExecutedPlans::OnFailure => !matched,
      ExecutedPlans::Always => true,
    }
  }
}

impl Filter {
  /// `options.variants` (a list) or `options.variant` (one), both naming recorded variant ids.
  fn from_options(options: Option<&Value>) -> Filter {
    let Some(options) = options else {
      return Filter::default();
    };
    let listed = options.get("variants").and_then(Value::as_array).map(|ids| {
      ids
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect::<Vec<_>>()
    });
    let single = options
      .get("variant")
      .and_then(Value::as_str)
      .map(|id| vec![id.to_string()]);
    Filter {
      variants: listed.or(single),
    }
  }

  fn admits(&self, id: &str) -> bool {
    match &self.variants {
      None => true,
      Some(ids) => ids.iter().any(|wanted| wanted == id),
    }
  }

  fn is_filtered(&self) -> bool {
    self.variants.is_some()
  }
}

/// How long one replayed request waits for the provider before it counts as a failure of that
/// variant. A run must terminate even when a provider hangs; the host's own timeout would
/// otherwise be the only thing that ends it, and it has no way to attribute the hang to a variant.
const SEND_TIMEOUT_MS: u64 = 30_000;

/// The default transport kind for an interaction that records none. Contract-file spec §4.3 makes
/// the binding optional, and the whole prototype's default is HTTP.
const DEFAULT_TRANSPORT_KIND: &str = "http";

// ---------------------------------------------------------------------------------------------
// Request documents (spec §8.3, `schemas/v1/verification.schema.json`).
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct Verify {
  pub source: ContractSource,
  pub target: VerificationTarget,
  #[serde(default)]
  pub options: Option<Value>,
}

/// Where the contracts come from. An open vocabulary whose v1 value is `inline`: files, URLs and
/// brokers are the host's business in the prototype, which is also what keeps filesystem access
/// out of a kernel that must build for `wasm32-wasip2`.
#[derive(Debug, Deserialize)]
pub struct ContractSource {
  pub kind: String,
  #[serde(default)]
  pub contracts: Vec<Value>,
}

#[derive(Debug, Deserialize)]
pub struct VerificationTarget {
  pub transports: Vec<TransportBinding>,
  /// The **resolved** hook configuration (lifecycle-hooks spec §6, ADR 0014): interpolated, with
  /// script sources inline and no paths left. It rides with the target because that is what it
  /// describes — how to talk to this provider, including putting it into a state — and protocol
  /// spec §8.3 says so in as many words ("open options such as state-change configuration").
  #[serde(default)]
  pub hooks: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub struct TransportBinding {
  pub transport: String,
  #[serde(default)]
  pub options: Option<Value>,
}

/// Why a `verify` call could not start. Everything after the run has started is an *event*, not an
/// error (spec §10.2: "a verification that ran and found mismatches is a successful operation") —
/// so this enum covers exactly the window before the first event.
#[derive(Debug)]
pub enum VerifyError {
  /// `kind` names a source the engine cannot read (`file`, `broker`, …).
  SourceUnsupported(String),
  /// The document is not a Janus contract at all (ADR 0011's identification).
  NotAContract {
    index: usize,
    found: Option<String>,
  },
  ContractInvalid {
    problems: Vec<Problem>,
  },
}

// ---------------------------------------------------------------------------------------------
// Reading the source.
// ---------------------------------------------------------------------------------------------

/// One document a run verifies. Both arms are first-class: a contract this engine wrote, and a
/// v1–v4 pact some other SDK wrote years ago (plan task 5.4).
pub(crate) enum Source {
  Janus(Contract),
  Legacy(Box<LegacyPact>),
}

/// A v1–v4 pact, read once and held in the terms the run needs. `specification` is the version the
/// document itself declared — carried into events because a report that says a run verified needs
/// to say what it verified.
pub(crate) struct LegacyPact {
  consumer: String,
  provider: String,
  specification: String,
  interactions: Vec<LegacyInteraction>,
}

impl Source {
  pub(crate) fn consumer(&self) -> &str {
    match self {
      Source::Janus(contract) => &contract.consumer.name,
      Source::Legacy(pact) => &pact.consumer,
    }
  }

  pub(crate) fn provider(&self) -> &str {
    match self {
      Source::Janus(contract) => &contract.provider.name,
      Source::Legacy(pact) => &pact.provider,
    }
  }

  /// How this document identifies itself (ADR 0011): the contract format string, or
  /// `pact/<version>`. It rides in every interaction reference, so no reader of a result has to
  /// guess which kind of document produced it.
  fn format(&self) -> String {
    match self {
      Source::Janus(_) => contract::FORMAT.to_string(),
      Source::Legacy(pact) => format!("pact/{}", pact.specification),
    }
  }

  fn interactions(&self) -> usize {
    match self {
      Source::Janus(contract) => contract.interactions.len(),
      Source::Legacy(pact) => pact.interactions.len(),
    }
  }

  /// How many results this document will produce. A contract's interaction has as many as it
  /// recorded; a pact's interaction has exactly one, because a single example is a single variant
  /// (contract-file spec §8.3) — the degenerate case of the same arithmetic, not a special case
  /// beside it.
  fn variants(&self) -> usize {
    match self {
      Source::Janus(contract) => contract
        .interactions
        .iter()
        .map(|i| i.selection.variants.len())
        .sum(),
      Source::Legacy(pact) => pact.interactions.len(),
    }
  }

  fn prepare(&self, index: usize) -> Prepared {
    match self {
      Source::Janus(contract) => prepare_janus(contract, index, &self.format()),
      Source::Legacy(pact) => prepare_legacy(pact, index, &self.format()),
    }
  }
}

/// Read the inline documents, identifying each before parsing it (contract-file spec §2.3, ADR
/// 0011). Three outcomes, and the middle one is the point of this task:
///
/// - `$format` names the Janus contract format: parse it as a contract;
/// - no `$format`, but the document carries a pact file's own required members: read it as a v1–v4
///   pact and verify it where it stands;
/// - anything else — a hand-written blob, a truncated file, a `$format` from a future engine — is
///   a refusal **naming what was found**, never a half-verified run.
pub(crate) fn read_source(source: &ContractSource) -> Result<Vec<Source>, VerifyError> {
  if source.kind != "inline" {
    return Err(VerifyError::SourceUnsupported(source.kind.clone()));
  }
  let mut sources = Vec::with_capacity(source.contracts.len());
  for (index, doc) in source.contracts.iter().enumerate() {
    match doc.get("$format").and_then(Value::as_str) {
      Some(format) if format == contract::FORMAT => sources.push(Source::Janus(read_contract(index, doc)?)),
      // A `$format` this engine does not know is never guessed at, even when the document also
      // looks pact-shaped: the member exists precisely to stop a reader guessing.
      Some(other) => {
        return Err(VerifyError::NotAContract {
          index,
          found: Some(other.to_string()),
        });
      }
      None => sources.push(Source::Legacy(Box::new(read_legacy(index, doc)?))),
    }
  }
  Ok(sources)
}

fn read_contract(index: usize, doc: &Value) -> Result<Contract, VerifyError> {
  serde_path_to_error::deserialize(doc).map_err(|err| {
    let pointer = format!(
      "/source/contracts/{index}{}",
      crate::error::json_pointer(err.path())
    );
    VerifyError::ContractInvalid {
      problems: vec![Problem {
        pointer,
        message: err.inner().to_string(),
      }],
    }
  })
}

/// A v1–v4 pact, identified before it is parsed. `pact_models` is deliberately lenient — it will
/// happily read `{}` as an empty pact — so identification happens **here**, against the members the
/// pact specification itself requires, rather than being delegated to a parser whose tolerance
/// would turn "this is not a contract" into "a contract with nothing in it". A run that verified
/// nothing and reported success is exactly the misparse ADR 0011 exists to prevent.
fn read_legacy(index: usize, doc: &Value) -> Result<LegacyPact, VerifyError> {
  if !looks_like_a_pact(doc) {
    return Err(VerifyError::NotAContract {
      index,
      found: legacy_hint(doc),
    });
  }
  let pact =
    crate::legacy_pact::read(&format!("source.contracts[{index}]"), doc).map_err(|err| match err {
      contract::ContractError::Invalid { problems } => VerifyError::ContractInvalid {
        problems: problems
          .into_iter()
          .map(|problem| Problem {
            pointer: format!("/source/contracts/{index}{}", problem.pointer),
            message: problem.message,
          })
          .collect(),
      },
      other => VerifyError::ContractInvalid {
        problems: vec![Problem {
          pointer: format!("/source/contracts/{index}"),
          message: other.to_string(),
        }],
      },
    })?;
  Ok(LegacyPact {
    consumer: pact.consumer().name,
    provider: pact.provider().name,
    specification: pact.specification_version().version_str(),
    interactions: legacy_pact::http_interactions(pact.as_ref()),
  })
}

/// The members every v1–v4 pact file has by its own specification: named parties and a body of
/// interactions (or messages, for a pact this engine reads and then finds nothing HTTP in — which
/// is a pact that verifies zero interactions, a different and honest answer from a misparse).
fn looks_like_a_pact(doc: &Value) -> bool {
  let has = |name: &str| doc.get(name).is_some_and(Value::is_object);
  has("consumer")
    && has("provider")
    && (doc.get("interactions").is_some_and(Value::is_array)
      || doc.get("messages").is_some_and(Value::is_array))
}

/// What a document that is neither a contract nor a readable pact looks like, for the refusal's
/// message: a pact file says so in its own metadata, and naming "pactSpecification 3.0.0" is far
/// more useful than "no `$format`".
fn legacy_hint(doc: &Value) -> Option<String> {
  let metadata = doc.get("metadata")?;
  let version = metadata
    .get("pactSpecification")
    .or_else(|| metadata.get("pact-specification"))
    .and_then(|spec| spec.get("version"))
    .and_then(Value::as_str)?;
  Some(format!("pactSpecification {version}"))
}

// ---------------------------------------------------------------------------------------------
// The run.
// ---------------------------------------------------------------------------------------------

/// One transport the target bound, in the drive role, ready to send.
pub(crate) struct Target {
  pub kind: String,
  pub component: Arc<dyn TransportComponent>,
  pub instance: String,
  /// What `transport/start` answered: the descriptor a hook sees as `endpoint`
  /// (lifecycle-hooks spec §3.3) — never assumed to be a URL, just passed along.
  pub endpoint: Value,
}

/// A verification session's live state (spec §7.1: the session is the only resource). It holds
/// nothing a host can address individually — the stream id is a plain identifier, and the thread
/// is joined when the stream's terminal event is delivered.
pub(crate) struct Run {
  pub stream: Arc<Stream>,
  pub thread: JoinHandle<()>,
}

/// Start the run. Returns immediately (spec §8.3: "`verify` starts a verification session and
/// returns immediately") — everything after this point is events on `stream`.
///
/// `options.variants`/`options.variant` narrow the run (variant-semantics spec §5.1); everything
/// else in `options` is ignored for now and logged rather than silently dropped.
pub(crate) fn start(
  sources: Vec<Source>,
  targets: Vec<Target>,
  content: Option<Arc<dyn ContentComponent>>,
  hooks: Option<HookRunner>,
  options: Option<Value>,
  stream: Arc<Stream>,
) -> Run {
  let filter = Filter::from_options(options.as_ref());
  let executed_plans = ExecutedPlans::from_options(options.as_ref());
  let thread_stream = Arc::clone(&stream);
  let thread = thread::spawn(move || {
    run(
      sources,
      targets,
      content,
      hooks,
      filter,
      executed_plans,
      thread_stream,
    )
  });
  Run { stream, thread }
}

fn run(
  sources: Vec<Source>,
  targets: Vec<Target>,
  content: Option<Arc<dyn ContentComponent>>,
  hooks: Option<HookRunner>,
  filter: Filter,
  executed_plans: ExecutedPlans,
  stream: Arc<Stream>,
) {
  let interactions: usize = sources.iter().map(Source::interactions).sum();
  let variants: usize = sources.iter().map(Source::variants).sum();
  stream.emit(
    "verification/started",
    json!({
      "contracts": sources.len(),
      "interactions": interactions,
      "variants": variants,
      "providers": sources.iter().map(|s| s.provider().to_string()).collect::<Vec<_>>(),
      // Positionally parallel to `providers`: which format each source document was written in
      // (plan task 5.4). A run over a mix of the two says so in its first event.
      "formats": sources.iter().map(Source::format).collect::<Vec<_>>(),
      "transports": targets.iter().map(|t| t.kind.clone()).collect::<Vec<_>>(),
      "filtered": filter.is_filtered(),
      "hooks": hooks.is_some(),
    }),
  );

  let mut hooks = hooks;
  let mut tally = Tally::default();

  // `before-verification` (spec §3.1): where a run acquires the thing every later hook needs — a
  // token, a container, a connection string. Its `data` lands in `run.data` under the hook's name
  // and is visible for the rest of the run, which is the mechanism that keeps a token fetch out of
  // `before-request`, where it would run once per exchange.
  let mut aborted = run_point(
    hooks.as_mut(),
    "before-verification",
    &Occurrence::default(),
    None,
    &mut Map::new(),
    &stream,
  );

  if aborted.is_none() {
    'sources: for source in &sources {
      for index in 0..source.interactions() {
        let flow = verify_prepared(
          source.prepare(index),
          &mut RunContext {
            targets: &targets,
            content: content.as_deref(),
            filter: &filter,
            executed_plans,
            stream: &stream,
            hooks: hooks.as_mut(),
          },
          &mut tally,
        );
        if let Flow::Abort(abort) = flow {
          aborted = Some(abort);
          break 'sources;
        }
      }
    }
  }

  if aborted.is_some() {
    // Everything the run did not reach is counted as not run, never folded into the failed tally
    // (spec §10.2): a verification that stopped early and one that failed are different results.
    let not_run = variants.saturating_sub(tally.verified + tally.failed + tally.state_unavailable);
    if let Some(runner) = hooks.as_mut() {
      runner.note_exchanges_not_run(not_run);
    }
  }

  // `after-verification` runs once, after the last exchange, **whether the run passed, failed or
  // aborted** (spec §3.8) — and it is handed the same summary document the terminal event carries,
  // so a reporting hook reads what the host reads. Nothing is mutable: a hook cannot revise a
  // verdict that has already been reached.
  let summary_for_hooks = summary(&tally, &sources, interactions, &filter, hooks.as_ref(), &aborted);
  if hooks
    .as_ref()
    .is_some_and(|runner| runner.has("after-verification"))
  {
    let occurrence = Occurrence {
      summary: Some(summary_for_hooks),
      ..Occurrence::default()
    };
    run_point(
      hooks.as_mut(),
      "after-verification",
      &occurrence,
      None,
      &mut Map::new(),
      &stream,
    );
  }

  // Transports are stopped *before* the terminal event, not after: spec §7.1 says everything the
  // session allocated is released when it ends, and the host learns it ended from that event. A
  // host that saw `finished` while a client instance was still open would be right to call it a
  // leak.
  for target in &targets {
    if let Err(err) = target.component.stop(Stop {
      instance: target.instance.clone(),
    }) {
      tracing::warn!(kind = %target.kind, instance = %target.instance, ?err, "transport did not stop cleanly");
    }
  }

  stream.finish(
    "verification/finished",
    summary(&tally, &sources, interactions, &filter, hooks.as_ref(), &aborted),
  );
}

/// Run one point's hooks and emit each invocation as a `verification/hook` event (spec §10.1).
/// Returns the abort, if one happened — every other outcome is the caller's to interpret, because
/// only it knows what an exchange is.
fn run_point(
  hooks: Option<&mut HookRunner>,
  point: &str,
  occurrence: &Occurrence,
  parts: Option<&mut Parts>,
  exchange_data: &mut Map<String, Value>,
  stream: &Stream,
) -> Option<Abort> {
  let runner = hooks?;
  let outcome = runner.run_point(point, occurrence, parts, exchange_data, &mut |payload| {
    stream.emit("verification/hook", payload);
  });
  match outcome {
    PointOutcome::Abort { hook, error } => Some(Abort {
      point: point.to_string(),
      hook,
      error,
    }),
    _ => None,
  }
}

/// A hook ended the run (spec §5.2's `abort-run`).
#[derive(Debug, Clone)]
struct Abort {
  point: String,
  hook: String,
  error: Value,
}

/// Whether the run carries on after an interaction.
enum Flow {
  Continue,
  Abort(Abort),
}

/// Per-run counters and the failure list the summary carries, so a host that only reads the
/// terminal event still knows what failed and where (the CLI, plan task 5.5, reads both).
#[derive(Default)]
struct Tally {
  verified: usize,
  failed: usize,
  /// Variants whose state the provider could not reach (variant-semantics spec §6.7). Counted
  /// separately from `failed` because its remedy is a contract change, not a code change, and a
  /// summary that conflates them sends the reader to the wrong team.
  state_unavailable: usize,
  /// Variants the filter excluded. Counted, never reported as anything else: an unreplayed variant
  /// is not a passing one (variant-semantics spec §5.1).
  skipped: usize,
  failures: Vec<Value>,
}

impl Tally {
  fn record(&mut self, payload: &Value) {
    match payload["status"].as_str() {
      Some("verified") => self.verified += 1,
      Some("state-unavailable") => {
        self.state_unavailable += 1;
        self.failures.push(payload.clone());
      }
      _ => {
        self.failed += 1;
        self.failures.push(payload.clone());
      }
    }
  }
}

/// The run summary (protocol spec §9.6): the document the terminal event carries, the one
/// `after-verification` is handed, and the one a report is written from.
fn summary(
  tally: &Tally,
  sources: &[Source],
  interactions: usize,
  filter: &Filter,
  hooks: Option<&HookRunner>,
  aborted: &Option<Abort>,
) -> Value {
  // `state-unavailable` fails the run by default (variant-semantics spec §6.7): the consumer
  // demonstrated it can handle that variant and the provider has not demonstrated it can produce
  // it, so reporting it as a pass would be the false confidence variant testing exists to remove.
  let failed = tally.failed + tally.state_unavailable;
  let mut summary = json!({
    "status": if failed == 0 && aborted.is_none() { "verified" } else { "failed" },
    "contracts": sources.len(),
    "interactions": interactions,
    // `filtered` rides in the summary as well as in `started`, because the summary is the
    // document a report is written from and "verified" without "filtered" beside it would be a
    // lie of omission.
    "filtered": filter.is_filtered(),
    "variants": {
      "total": tally.verified + failed + tally.skipped,
      "verified": tally.verified,
      "failed": tally.failed,
      "state-unavailable": tally.state_unavailable,
      "skipped": tally.skipped,
    },
    "failures": tally.failures,
  });
  let map = summary.as_object_mut().expect("a json! object literal");
  if let Some(runner) = hooks
    && !runner.report.is_empty()
  {
    map.insert("hooks".to_string(), runner.report.to_json());
  }
  if let Some(abort) = aborted {
    map.insert(
      "aborted".to_string(),
      json!({ "point": abort.point, "hook": abort.hook, "error": abort.error }),
    );
  }
  summary
}

/// Everything one run carries through every interaction and every variant: what it may drive,
/// what decodes a slot, what it was asked to replay, where it reports, and its hooks.
struct RunContext<'a> {
  targets: &'a [Target],
  content: Option<&'a dyn ContentComponent>,
  filter: &'a Filter,
  executed_plans: ExecutedPlans,
  stream: &'a Stream,
  hooks: Option<&'a mut HookRunner>,
}

/// One interaction, prepared for replay. **This is where the two source kinds stop differing**:
/// a Janus contract's interaction and a v1–v4 pact's interaction both arrive here, and everything
/// downstream — the hook sequence, the events, the result vocabulary, the tally — is one code
/// path over this one type.
struct Prepared {
  /// How this interaction is named in every event of the run.
  reference: Value,
  /// Which transport kind drives it.
  transport: String,
  replays: Vec<Replay>,
}

/// One interaction × variant: the unit a verification result reports on (spec §9.6).
enum Replay {
  Ready(Box<Ready>),
  /// The source document itself defeated preparation. Reported against **this variant** rather
  /// than as one interaction-level event, because interaction × variant is the unit of a result
  /// and collapsing it here would make a summary's variant counts stop adding up.
  Broken {
    variant: String,
    error: Value,
  },
}

impl Replay {
  fn variant(&self) -> &str {
    match self {
      Replay::Ready(ready) => &ready.variant,
      Replay::Broken { variant, .. } => variant,
    }
  }
}

/// Everything one replay needs, with nothing left to decide: what to send, what scores the reply,
/// how the reply's values are read back, and what the hooks are told.
struct Ready {
  variant: String,
  origin: Value,
  assignment: Value,
  states: Vec<contract::ResolvedState>,
  /// The bytes the consumer actually sent (variant-semantics spec §5.2) — a contract's recorded
  /// variant or a pact's own request example. Never re-derived from a shape on either path.
  request: Parts,
  /// Pinned to this variant on the shape path, compiled from matching rules on the v1–v4 path.
  plan: plan::Plan,
  /// How the reply becomes the values `plan` resolves against. A function rather than a flag
  /// because the difference *is* a different reading of the same parts: design 3.5's plans
  /// address a header as one value per name (`legacy_pact::header_captures`), shapes address the
  /// slot the transport actually produced.
  resolve: fn(&Parts, Option<&dyn ContentComponent>) -> CapturedValues,
  /// Emitted before this variant's exchange starts, so a warning precedes the result it qualifies.
  warnings: Vec<Value>,
  /// The `interaction` document a hook's occurrence carries (lifecycle-hooks spec §3.2), with this
  /// variant's states already resolved.
  interaction: Value,
}

/// A Janus contract's interaction (plan tasks 5.1–5.3): compiled from its recorded shapes, one
/// replay per recorded variant.
fn prepare_janus(contract: &Contract, index: usize, format: &str) -> Prepared {
  let interaction = &contract.interactions[index];
  let reference = interaction_ref(contract, index, interaction, format);
  let transport = interaction
    .transport
    .as_ref()
    .map(|t| t.kind.as_str())
    .unwrap_or(DEFAULT_TRANSPORT_KIND)
    .to_string();

  // A shape the engine cannot parse is the contract's problem, not the provider's.
  let spec = match interaction_spec(interaction) {
    Ok(spec) => spec,
    Err(problems) => {
      return Prepared {
        reference,
        transport,
        replays: interaction
          .selection
          .variants
          .iter()
          .map(|variant| Replay::Broken {
            variant: variant.id.clone(),
            error: json!({ "code": "contract-invalid", "problems": problems }),
          })
          .collect(),
      };
    }
  };

  // The variant space is recomputed from the recorded shapes rather than read out of the
  // contract, because §6.4's agreement claim is only worth anything if both ends derive it the
  // same way from the same input.
  let space = plan::variant_space(&spec);

  let replays = interaction
    .selection
    .variants
    .iter()
    .map(|variant| {
      let assignment = assignment_of(variant);
      let states = params::resolve_states(interaction.states.as_ref(), &space, &assignment);
      let mut warnings = Vec::new();
      if let Some(disagreement) = state_disagreement(states.as_ref(), variant) {
        // Not a failure of the provider, so not a failed variant: the two ends resolved the same
        // binding differently, which means they are not the same implementation. Naming both is
        // the whole value of recording resolved states (§6.4).
        warnings.push(json!({
          "code": "state-resolution-disagreement",
          "interaction": reference, "variant": variant.id,
          "resolved": disagreement.0, "recorded": disagreement.1,
        }));
      }
      let Some(request) = recorded_request(variant) else {
        return Replay::Broken {
          variant: variant.id.clone(),
          error: json!({ "code": "contract-invalid", "problems": [{
            "pointer": "/parts/request",
            "message": "the recorded variant has no request part to replay",
          }] }),
        };
      };
      let states_json = serde_json::to_value(&states).unwrap_or(Value::Null);
      Replay::Ready(Box::new(Ready {
        variant: variant.id.clone(),
        origin: json!(variant.origin),
        assignment: json!(variant.assignment),
        // Pinned to this variant (shape spec §7.1, variant-semantics spec §5.2 step 3): an
        // `optional` pinned to `absent` admits only absence, an `any-of` pinned to `SHIPPED` only
        // that. Matching against the unpinned shape would accept `PENDING` where the consumer
        // demonstrated `SHIPPED`, and the variant would have proved nothing.
        plan: plan::compile(&spec, &assignment, Some(&variant.id)),
        resolve: parts_resolver,
        states: states.unwrap_or_default(),
        request,
        warnings,
        interaction: json!({
          "description": interaction.description,
          "states": states_json,
          "transport": interaction.transport,
        }),
      }))
    })
    .collect();

  Prepared {
    reference,
    transport,
    replays,
  }
}

/// A v1–v4 pact's interaction (plan task 5.4): compiled by design 3.5's matching-rule compiler,
/// replayed from the pact's own request example, **one** variant.
///
/// That single variant is the degenerate case of the same arithmetic, not a branch around it
/// (contract-file spec §8.3, variant-semantics spec §9): one example is one variant, its id is
/// `base` and its assignment is empty, so every count in the summary means what it means for a
/// contract. What the pact under-covers it under-covers honestly — the fix for that is to run the
/// consumer's suite under Janus, which is the incentive the migration path wants anyway.
///
/// Only the **response** is compiled. The request is replayed verbatim, so matching it against
/// itself would assert nothing (the same rule the shape path follows, [`drive`]). That has a
/// pleasant consequence: a request body in a content type design 3.5 cannot compile still replays,
/// byte for byte, instead of failing an interaction whose response is perfectly checkable.
fn prepare_legacy(pact: &LegacyPact, index: usize, format: &str) -> Prepared {
  let interaction = &pact.interactions[index];
  let states: Vec<contract::ResolvedState> = interaction
    .provider_states
    .iter()
    .map(|state| contract::ResolvedState {
      name: state.name.clone(),
      // v3 state parameters travel as they stand — a `state-setup` hook in `pact-state-change`
      // format hands the provider exactly the `{state, params, action}` it received before Janus
      // existed (lifecycle-hooks spec §8.4). v1/v2's bare state string arrives here as a state
      // with no parameters, which is what it is.
      params: (!state.params.is_empty())
        .then(|| state.params.iter().map(|(k, v)| (k.clone(), v.clone())).collect()),
    })
    .collect();
  let reference = json!({
    "contract": { "consumer": pact.consumer, "provider": pact.provider },
    "index": index,
    "description": interaction.description,
    "states": states.iter().map(|state| state.name.clone()).collect::<Vec<_>>(),
    "format": format,
  });
  // v1–v4 pacts are HTTP request/response documents by their own specification, so there is
  // nothing to read a transport kind out of: the interaction has one.
  let transport = DEFAULT_TRANSPORT_KIND.to_string();

  let response = match legacy_pact::legacy_response(&interaction.response) {
    Ok(response) => response,
    Err(err) => {
      return Prepared {
        reference,
        transport,
        replays: vec![Replay::Broken {
          variant: LEGACY_VARIANT.to_string(),
          error: unsupported_body(&err, index, "response"),
        }],
      };
    }
  };

  let states_json = serde_json::to_value(&states).unwrap_or(Value::Null);
  Prepared {
    reference,
    transport,
    replays: vec![Replay::Ready(Box::new(Ready {
      variant: LEGACY_VARIANT.to_string(),
      origin: json!(crate::variant::Origin::Base),
      assignment: json!([]),
      plan: plan::compile_legacy_response(&response),
      resolve: legacy_resolver,
      states,
      request: legacy_pact::request_parts(&interaction.request),
      warnings: Vec::new(),
      interaction: json!({
        "description": interaction.description,
        "states": states_json,
        "transport": { "kind": DEFAULT_TRANSPORT_KIND },
      }),
    }))],
  }
}

/// The sole variant of a converted interaction (contract-file spec §8.3).
const LEGACY_VARIANT: &str = "base";

/// A reply read the way design 3.5's plans address it: the generic slot captures, plus one string
/// per header name ([`legacy_pact::header_captures`] documents why the two forms differ).
fn legacy_resolver(parts: &Parts, content: Option<&dyn ContentComponent>) -> CapturedValues {
  legacy_pact::header_captures(parts_resolver(parts, content), parts)
}

/// A v1–v4 body design 3.5 cannot compile, as the error a result carries. The two cases are
/// genuinely different and get different codes: a content type the engine has no compiler for is
/// a **missing component** (spec §10.2 — the identifier names what is missing, and a content
/// component is exactly what would supply it), while a body that claimed to be JSON and was not is
/// a **document** problem, carrying a pointer into the pact its author can act on.
fn unsupported_body(err: &legacy_pact::Unsupported, index: usize, part: &str) -> Value {
  match err {
    legacy_pact::Unsupported::ContentType(content_type) => json!({
      "code": "component-unavailable",
      "component": format!("content/{content_type}"),
      "message": err.to_string(),
    }),
    legacy_pact::Unsupported::Malformed(message) => json!({
      "code": "contract-invalid",
      "problems": [{ "pointer": format!("/interactions/{index}/{part}/body"), "message": message }],
    }),
  }
}

/// Replay one interaction's variants: the one loop both source kinds run through.
fn verify_prepared(prepared: Prepared, ctx: &mut RunContext<'_>, tally: &mut Tally) -> Flow {
  let targets = ctx.targets;
  let target = targets.iter().find(|target| target.kind == prepared.transport);
  let reference = prepared.reference;

  for replay in prepared.replays {
    if !ctx.filter.admits(replay.variant()) {
      tally.skipped += 1;
      tracing::debug!(variant = %replay.variant(), "variant excluded by the run's filter");
      continue;
    }

    let ready = match replay {
      Replay::Ready(ready) => ready,
      Replay::Broken { variant, error } => {
        report_failure(ctx, tally, &reference, &variant, error);
        continue;
      }
    };

    // Contract-file spec §4.3: an unknown kind is `component-unavailable` naming it, never a
    // silent skip — a verifier that cannot speak an interaction's transport has not verified it.
    let Some(target) = target else {
      let error = json!({
        "code": "component-unavailable",
        "component": format!("transport/{}", prepared.transport),
      });
      report_failure(ctx, tally, &reference, &ready.variant, error);
      continue;
    };

    for warning in &ready.warnings {
      ctx.stream.emit("verification/warning", warning.clone());
    }
    ctx.stream.emit(
      "verification/interaction-started",
      json!({
        "interaction": reference, "variant": ready.variant, "origin": ready.origin,
        // The states this variant needs, resolved (§6.4) — what `state-setup` is about to be asked
        // for, visible whether or not a hook is configured to do the asking.
        "states": ready.interaction["states"],
      }),
    );

    let exchange = Exchange {
      id: format!(
        "x-{}",
        tally.verified + tally.failed + tally.state_unavailable + 1
      ),
      reference: &reference,
      ready: &ready,
      transport: &prepared.transport,
    };
    let (payload, flow) = verify_exchange(&exchange, target, ctx);
    tally.record(&payload);
    if let Flow::Abort(abort) = flow {
      return Flow::Abort(abort);
    }
  }
  Flow::Continue
}

/// A variant that failed before any exchange happened: emitted and tallied exactly like one that
/// ran, because a result a host never saw is a variant a report would silently drop.
fn report_failure(ctx: &RunContext<'_>, tally: &mut Tally, reference: &Value, variant: &str, error: Value) {
  let payload = json!({
    "interaction": reference, "variant": variant, "status": "failed", "error": error,
  });
  ctx
    .stream
    .emit("verification/interaction-result", payload.clone());
  tally.record(&payload);
}

/// One interaction exercised at one variant — the unit a verification result reports on, and the
/// unit a hook failure can fail without ending the run (lifecycle-hooks spec §2.2).
struct Exchange<'a> {
  id: String,
  reference: &'a Value,
  ready: &'a Ready,
  transport: &'a str,
}

/// The shape of an exchange (spec §4.1): state setup per state in recorded order, `before-request`,
/// the exchange itself, `after-response`, then teardown per state in **reverse** order — which
/// runs whether the exchange passed, failed or never ran, because the state a failed exchange left
/// behind is exactly the state that will make the next variant fail for the wrong reason.
fn verify_exchange(exchange: &Exchange<'_>, target: &Target, ctx: &mut RunContext<'_>) -> (Value, Flow) {
  let mut exchange_data = Map::new();
  let base = Occurrence {
    interaction: Some(exchange.ready.interaction.clone()),
    variant: Some(json!({
      "id": exchange.ready.variant,
      "assignment": exchange.ready.assignment,
    })),
    exchange: Some(json!({ "id": exchange.id })),
    endpoint: Some(target.endpoint.clone()),
    transport: Some(exchange.transport.to_string()),
    ..Occurrence::default()
  };

  let mut result: Option<Value> = None;
  let mut flow = Flow::Continue;

  // 1. state-setup, once per state, in recorded order — for **every** variant, including
  //    consecutive ones whose resolved parameters are identical (variant-semantics spec §6.6).
  for state in &exchange.ready.states {
    if result.is_some() {
      break;
    }
    let occurrence = Occurrence {
      state: Some(serde_json::to_value(state).unwrap_or(Value::Null)),
      ..base.clone()
    };
    let (status, aborted) = hook_point(ctx, "state-setup", &occurrence, None, &mut exchange_data);
    if let Some(abort) = aborted {
      flow = Flow::Abort(abort.clone());
      result = Some(hook_failure_payload(
        exchange,
        "failed",
        &abort.hook,
        &abort.error,
      ));
      break;
    }
    match status {
      HookStatus::Continue => {}
      HookStatus::StateUnavailable { hook, error } => {
        // Its own status, not a failure of the provider's behaviour: the provider cannot be put
        // into a state the consumer declared, and the remedy is a contract change.
        result = Some(hook_failure_payload(exchange, "state-unavailable", &hook, &error));
      }
      HookStatus::FailExchange { hook, error } => {
        result = Some(hook_failure_payload(exchange, "failed", &hook, &error));
      }
    }
  }

  // 2. before-request, then the exchange itself. The request is the recorded one, cloned because
  //    `before-request` may rewrite it and the recording must survive the run unchanged.
  let mut parts = (result.is_none()).then(|| exchange.ready.request.clone());

  if let (None, Some(request)) = (&result, parts.as_mut()) {
    let (status, aborted) = hook_point(ctx, "before-request", &base, Some(request), &mut exchange_data);
    if let Some(abort) = aborted {
      flow = Flow::Abort(abort.clone());
      result = Some(hook_failure_payload(
        exchange,
        "failed",
        &abort.hook,
        &abort.error,
      ));
    } else if let HookStatus::FailExchange { hook, error } | HookStatus::StateUnavailable { hook, error } =
      status
    {
      // A `before-request` hook that failed to sign a request would otherwise produce a 401 the
      // report would attribute to the provider; the hook's own error names the real cause.
      result = Some(hook_failure_payload(exchange, "failed", &hook, &error));
    }
  }

  let mut reply = None;
  let mut executed_plan = None;
  if result.is_none()
    && let Some(request) = parts
  {
    match drive(exchange, request, target, ctx.content, ctx.executed_plans) {
      Ok(driven) => {
        executed_plan = driven.executed;
        result = Some(driven.payload);
        reply = driven.reply;
      }
      Err(payload) => result = Some(payload),
    }
  }

  // 3. after-response — observing only (spec §3.6): a hook that could rewrite the response before
  //    matching would be editing the evidence, and matching has already happened by here anyway.
  if let Some(inbound) = reply {
    let mut parts = inbound;
    let (_, aborted) = hook_point(ctx, "after-response", &base, Some(&mut parts), &mut exchange_data);
    if let Some(abort) = aborted {
      flow = Flow::Abort(abort);
    }
  }

  let payload = result.unwrap_or_else(
    || json!({ "interaction": exchange.reference, "variant": exchange.ready.variant, "status": "failed" }),
  );

  // The result is reported *before* teardown, not after: the exchange has its outcome by here —
  // teardown is even handed it, so it can differ after a failure (spec §3.7) — and a host
  // rendering hook activity inline reads the cleanup as what it is, work after the answer.
  ctx
    .stream
    .emit("verification/interaction-result", payload.clone());

  // The evidence follows the verdict it produced (spec §9.6's `verification/executed-plan`, which
  // is where `explain --executed` gets its input): a reader learns *that* a variant failed, then
  // reads the tree showing where. The other order makes them scroll back.
  if let Some(executed) = executed_plan {
    ctx.stream.emit("verification/executed-plan", executed);
  }

  // 4. state-teardown, reverse order, always.
  let outcome = payload["status"].as_str().unwrap_or("failed");
  let outcome = match outcome {
    "verified" => "passed",
    other => other,
  };
  for state in exchange.ready.states.iter().rev() {
    let occurrence = Occurrence {
      state: Some(serde_json::to_value(state).unwrap_or(Value::Null)),
      exchange: Some(json!({ "id": exchange.id, "outcome": outcome })),
      ..base.clone()
    };
    let (_, aborted) = hook_point(ctx, "state-teardown", &occurrence, None, &mut exchange_data);
    if let Some(abort) = aborted {
      flow = Flow::Abort(abort);
    }
  }

  (payload, flow)
}

/// What a point's hooks decided about this exchange, with the abort separated out because it ends
/// the whole run rather than this attempt.
enum HookStatus {
  Continue,
  FailExchange { hook: String, error: Value },
  StateUnavailable { hook: String, error: Value },
}

fn hook_point(
  ctx: &mut RunContext<'_>,
  point: &str,
  occurrence: &Occurrence,
  parts: Option<&mut Parts>,
  exchange_data: &mut Map<String, Value>,
) -> (HookStatus, Option<Abort>) {
  let stream = ctx.stream;
  let Some(runner) = ctx.hooks.as_mut() else {
    return (HookStatus::Continue, None);
  };
  let outcome = runner.run_point(point, occurrence, parts, exchange_data, &mut |payload| {
    stream.emit("verification/hook", payload);
  });
  match outcome {
    PointOutcome::Continue => (HookStatus::Continue, None),
    PointOutcome::FailExchange { hook, error } => (HookStatus::FailExchange { hook, error }, None),
    PointOutcome::StateUnavailable { hook, error } => (HookStatus::StateUnavailable { hook, error }, None),
    PointOutcome::Abort { hook, error } => (
      HookStatus::Continue,
      Some(Abort {
        point: point.to_string(),
        hook,
        error,
      }),
    ),
  }
}

/// A variant whose result a hook decided. The hook's own error is the cause, carried verbatim —
/// rewriting it into an engine code would lose the only thing it was carrying (spec §11).
fn hook_failure_payload(exchange: &Exchange<'_>, status: &str, hook: &str, error: &Value) -> Value {
  json!({
    "interaction": exchange.reference, "variant": exchange.ready.variant, "status": status,
    "hook": hook, "error": error,
  })
}

/// Send the recorded request and score the reply against this replay's plan — pinned to the
/// variant on the shape path, compiled from matching rules on the v1–v4 path, and by here just a
/// plan either way. `Ok` carries the result payload and the inbound parts (for `after-response`);
/// `Err` carries a payload for the cases where nothing came back to match.
/// What one driven exchange produced: the result payload, the inbound parts (for
/// `after-response`), and the executed plan when the run was asked for one.
struct Driven {
  payload: Value,
  reply: Option<Parts>,
  executed: Option<Value>,
}

fn drive(
  exchange: &Exchange<'_>,
  request: Parts,
  target: &Target,
  content: Option<&dyn ContentComponent>,
  executed_plans: ExecutedPlans,
) -> Result<Driven, Value> {
  let reference = exchange.reference;
  let variant = &exchange.ready.variant;

  let sent = target.component.send(SendRequest {
    instance: target.instance.clone(),
    parts: request,
    await_reply: true,
    timeout_ms: Some(SEND_TIMEOUT_MS),
  });
  let reply = match sent {
    Ok(result) => result.reply,
    Err(error) => {
      tracing::warn!(%variant, ?error, "the provider could not be driven for this variant");
      return Err(json!({
        "interaction": reference, "variant": variant, "status": "failed",
        "error": { "code": "component-failed", "component": format!("transport/{}", target.kind), "error": error },
      }));
    }
  };
  let Some(reply) = reply else {
    // Spec §5.2: "no reply expected" and "reply expected but absent" are different observations,
    // and this is the second one — a transport that answered a driven `send` with no reply cannot
    // be matched against a response shape.
    return Err(json!({
      "interaction": reference, "variant": variant, "status": "failed",
      "error": { "code": "component-failed", "component": format!("transport/{}", target.kind),
                 "error": { "code": "no-reply", "message": "the transport returned no reply to match" } },
    }));
  };

  let resolver = (exchange.ready.resolve)(&reply, content);
  let executed = execute(&exchange.ready.plan, &resolver);
  // Only the response half is scored. The request was replayed verbatim, so matching it against
  // itself would assert nothing; the provider answers for the response (variant-semantics §5.2).
  // A shape-compiled plan carries both parts and the `response` subtree is picked out of it; a
  // v1–v4 plan is compiled from the response alone, so its root already *is* that subtree and the
  // fallback is the whole of it.
  let response = find_container(&executed, "response").unwrap_or(&executed);
  let (status, mismatches) = outcome(response);

  let matched = status == Status::Matched;
  let payload = match status {
    Status::Matched => {
      tracing::debug!(%variant, "variant verified");
      json!({ "interaction": reference, "variant": variant, "status": "verified" })
    }
    Status::Mismatched => {
      tracing::warn!(%variant, count = mismatches.len(), "variant failed");
      json!({
        "interaction": reference, "variant": variant, "status": "failed",
        "mismatches": mismatches.iter().map(mismatch_json).collect::<Vec<_>>(),
      })
    }
  };

  // The whole executed tree, not just the response subtree that was scored: `explain --executed`
  // is a diagnosis tool, and what the request replayed as is part of the diagnosis.
  let executed = executed_plans.wants(matched).then(|| {
    json!({
      "interaction": reference,
      "variant": variant,
      "status": if matched { "verified" } else { "failed" },
      "text": plan::render_executed(&executed),
      "plan": plan::plan_json(&exchange.ready.plan),
    })
  });

  Ok(Driven {
    payload,
    reply: Some(reply),
    executed,
  })
}

/// This engine's resolution against the one the contract recorded, when they differ (§6.4).
/// `None` when they agree, or when the contract recorded none — an older writer that recorded no
/// resolved states is not disagreeing with anything.
fn state_disagreement(
  resolved: Option<&Vec<contract::ResolvedState>>,
  variant: &contract::RecordedVariant,
) -> Option<(Value, Value)> {
  let recorded = variant.states.as_ref()?;
  let resolved = resolved?;
  (resolved != recorded).then(|| {
    (
      serde_json::to_value(resolved).expect("ResolvedState always serializes"),
      serde_json::to_value(recorded).expect("ResolvedState always serializes"),
    )
  })
}

/// How an interaction is named in every event of the run: the contract it came from, its index,
/// and its identity (description plus states — contract-file spec §4.2). Enough for a host to
/// point a user at one line of one file without the engine assuming anything about file layout.
fn interaction_ref(
  contract: &Contract,
  index: usize,
  interaction: &contract::Interaction,
  format: &str,
) -> Value {
  json!({
    "contract": { "consumer": contract.consumer.name, "provider": contract.provider.name },
    "index": index,
    "description": interaction.description,
    "states": interaction.states.as_ref().map(|states| {
      states.iter().map(|state| state.name.clone()).collect::<Vec<_>>()
    }),
    // Which document this interaction came from (ADR 0011). A run may mix a contract and a v1–v4
    // pact, and a result that did not say which it came from would send its reader to the wrong
    // file.
    "format": format,
  })
}

/// The recorded interaction as the document the plan compiler already understands. A contract
/// interaction *is* an interaction specification plus evidence (`interaction_spec::model`'s own
/// header says so), so this rebuilds the specification rather than teaching the compiler a second
/// input shape — which is also what makes the verifier's plans the consumer's plans.
fn interaction_spec(interaction: &contract::Interaction) -> Result<InteractionSpec, Vec<Problem>> {
  let mut document = json!({
    "description": interaction.description,
    "parts": interaction.parts,
  });
  let map = document.as_object_mut().expect("a json! object literal");
  if let Some(transport) = &interaction.transport {
    map.insert("transport".to_string(), json!(transport));
  }
  if let Some(states) = &interaction.states {
    map.insert("states".to_string(), json!(states));
  }
  if let Some(requires) = &interaction.requires {
    map.insert("requires".to_string(), json!(requires));
  }
  interaction_spec::parse(&document).map_err(|err| err.problems)
}

/// A recorded assignment (`[{dimension, point}, …]`, contract-file spec §5.2) as the compiler's
/// pinning map. An entry the engine cannot read is skipped rather than fatal: the compiler treats
/// an unknown dimension as unpinned (`plan::compile`'s own documented leniency), so a contract
/// written by a newer engine degrades to a wider match instead of refusing to verify at all.
fn assignment_of(variant: &contract::RecordedVariant) -> Assignment {
  let mut assignment = Assignment::new();
  for entry in &variant.assignment {
    let dimension = entry.get("dimension").and_then(Value::as_str);
    let point = entry.get("point").and_then(Value::as_str);
    if let (Some(dimension), Some(point)) = (dimension, point) {
      assignment.insert(dimension.to_string(), point.to_string());
    } else {
      tracing::warn!(variant = %variant.id, entry = %entry, "unreadable assignment entry; that dimension stays unpinned");
    }
  }
  assignment
}

/// The recorded request example, wired for `transport/send` (variant-semantics spec §5.2 step 2:
/// "the provider sees the bytes the consumer actually sent"). No conversion happens because none
/// is needed — a contract's `SlotValue` *is* the component interface's `SlotValue`
/// (component-interfaces spec §4), which is the whole point of that reuse.
fn recorded_request(variant: &contract::RecordedVariant) -> Option<Parts> {
  let request = variant.parts.get("request")?;
  let mut parts: Parts = BTreeMap::new();
  parts.insert("request".to_string(), request.clone());
  Some(parts)
}
