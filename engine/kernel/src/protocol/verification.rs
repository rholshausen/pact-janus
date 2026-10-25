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

use super::contributions::Contributions;
use super::events::Stream;
use super::frame::EngineError;
use super::scope::Scope;
use super::wire::{encode_slot, find_container, mismatch_json, parts_resolver};
use crate::broker::{
  BrokerClient, BrokerDescriptor, FetchRequest, FetchedPact, InteractionResult, PublishRequest,
};
use crate::common::ContentTypes;
use crate::component::{
  ContentComponent, ExchangeParts, PactGenerators, Parts, Send as SendRequest, Stop, TransportComponent,
  Unavailable,
};
use crate::contract::{self, Contract};
use crate::error::Problem;
use crate::hooks::{ExchangeScratch, HookRunner, Occurrence, PointOutcome};
use crate::interaction_spec::{self, InteractionSpec};
use crate::legacy::compile::{self as legacy_compile, compile_response as compile_legacy_response};
use crate::legacy::pact::{
  self as legacy_pact, MessageInteraction, PactInteraction, PluginInteraction, RequestTemplate,
};
use crate::plan::{self, ActionApplier, Assignment, CapturedValues, Status, execute_with, outcome};
use crate::shape::fresh;
use crate::variant::params;
use serde::Deserialize;
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::thread::{self, JoinHandle};

/// Which recorded variants a run replays (variant-semantics spec §5.1). The default is all of
/// them; a host may narrow it for debugging, and a narrowed run **must be reported as filtered** —
/// a partial run is not a pass, and a summary that did not say so would read like one (ADR 0041).
#[derive(Debug, Clone, Default)]
pub(crate) struct Filter {
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
  /// `options.variants` (a list) or `options.variant` (one), both naming recorded variant ids. A
  /// list naming none is refused: it could only ever replay nothing (spec §8.3).
  pub(crate) fn from_options(options: Option<&Value>) -> Result<Filter, EngineError> {
    let Some(options) = options else {
      return Ok(Filter::default());
    };
    let listed = options.get("variants").and_then(Value::as_array).map(|ids| {
      ids
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect::<Vec<_>>()
    });
    if listed.as_ref().is_some_and(Vec::is_empty) {
      return Err(EngineError::malformed_frame(
        "options.variants names no variant: a run filtered to nothing would replay nothing",
      ));
    }
    let single = options
      .get("variant")
      .and_then(Value::as_str)
      .map(|id| vec![id.to_string()]);
    Ok(Filter {
      variants: listed.or(single),
    })
  }

  fn admits(&self, id: &str) -> bool {
    match &self.variants {
      None => true,
      Some(ids) => ids.iter().any(|wanted| wanted == id),
    }
  }

  /// Refuse a filter that names an id no document records (variant-semantics spec §5.1, ADR 0041):
  /// it is a typo, and a run of it would replay less than it was asked to, or nothing. A run with no
  /// documents has nothing to narrow, so it is not refused.
  pub(crate) fn check(&self, sources: &[Source]) -> Result<(), EngineError> {
    let Some(wanted) = &self.variants else {
      return Ok(());
    };
    if sources.is_empty() {
      return Ok(());
    }
    let recorded: std::collections::BTreeSet<&str> = sources.iter().flat_map(Source::variant_ids).collect();
    let unknown: Vec<String> = wanted
      .iter()
      .filter(|id| !recorded.contains(id.as_str()))
      .cloned()
      .collect();
    if unknown.is_empty() {
      return Ok(());
    }
    let recorded: Vec<String> = recorded.into_iter().map(str::to_string).collect();
    Err(EngineError::variant_not_recorded(&unknown, &recorded))
  }

  /// Whether the filter excludes any variant `source` records: what makes a run, and a pair,
  /// filtered (ADR 0041 decision 2). A filter that admits everything leaves a whole run.
  fn narrows(&self, source: &Source) -> bool {
    self.variants.is_some() && source.variant_ids().iter().any(|id| !self.admits(id))
  }
}

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

/// Where the contracts come from. An open vocabulary. `inline` is read here; files and URLs are the
/// host's business, which keeps filesystem access out of a kernel that must build for
/// `wasm32-wasip2`. `broker` is fetched in the run by a broker client, a native host capability
/// (spec §8.3, ADR 0032); an engine with none refuses it by name before the run starts.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ContractSource {
  pub kind: String,
  #[serde(default)]
  pub contracts: Vec<Value>,
  /// For `broker`: which broker, and how to reach it.
  #[serde(default)]
  pub broker: Option<BrokerDescriptor>,
  /// For `broker`: the provider whose pacts to fetch.
  #[serde(default)]
  pub provider: Option<String>,
  #[serde(default)]
  pub consumer_version_selectors: Vec<Value>,
  #[serde(default)]
  pub provider_branch: Option<String>,
  #[serde(default)]
  pub provider_tags: Vec<String>,
  #[serde(default)]
  pub include_pending: bool,
  #[serde(default)]
  pub include_wip_since: Option<String>,
  #[serde(default = "pacts_required")]
  pub require_pacts: bool,
}

fn pacts_required() -> bool {
  true
}

impl ContractSource {
  /// A `broker` source as the client's request, or what it is missing.
  pub(crate) fn fetch_request(&self) -> Result<FetchRequest, VerifyError> {
    let missing = |member: &str| VerifyError::SourceIncomplete(format!("a 'broker' source needs '{member}'"));
    Ok(FetchRequest {
      broker: self.broker.clone().ok_or_else(|| missing("broker"))?,
      provider: self.provider.clone().ok_or_else(|| missing("provider"))?,
      selectors: self.consumer_version_selectors.clone(),
      provider_branch: self.provider_branch.clone(),
      provider_tags: self.provider_tags.clone(),
      include_pending: self.include_pending,
      include_wip_since: self.include_wip_since.clone(),
    })
  }
}

/// `options.publish` (spec §8.3): publish each fetched pact's result to that pact's own link.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct PublishOptions {
  pub provider_version: String,
  #[serde(default)]
  pub branch: Option<String>,
  #[serde(default)]
  pub tags: Vec<String>,
  #[serde(default)]
  pub build_url: Option<String>,
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
  /// The project's declared components, resolved by the host's loader exactly as for
  /// `consumer-session/create` (spec §8.3). Loaded, and every contract's `requires` checked, before
  /// `verify` answers.
  #[serde(default)]
  pub components: Vec<Value>,
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
  /// A source of a known kind without a member that kind needs.
  SourceIncomplete(String),
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
  interactions: Vec<PactInteraction>,
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

  /// A v4 interaction marked pending.
  fn interaction_pending(&self, index: usize) -> bool {
    match self {
      Source::Janus(_) => false,
      Source::Legacy(pact) => pact.interactions.get(index).is_some_and(PactInteraction::pending),
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

  /// The id of every variant this document records, one per result it will produce, in order.
  fn variant_ids(&self) -> Vec<&str> {
    match self {
      Source::Janus(contract) => contract
        .interactions
        .iter()
        .flat_map(|i| i.selection.variants.iter().map(|v| v.id.as_str()))
        .collect(),
      Source::Legacy(pact) => vec![LEGACY_VARIANT; pact.interactions.len()],
    }
  }

  /// Component-interfaces spec §2.3: everything this document's interactions cannot run without —
  /// their requirements and a component for each content type they declare — checked against the
  /// run's scope before the run starts. A v1–v4 pact declares neither.
  pub(crate) fn check(&self, scope: &Scope) -> Result<(), Unavailable> {
    match self {
      Source::Janus(contract) => contract.interactions.iter().try_for_each(|interaction| {
        // Plan task 8.4: a fragment this engine cannot use fails the run before it starts, too.
        scope.check(
          interaction.requires.iter().flatten(),
          interaction.content_types.as_ref(),
          &interaction.parts,
        )
      }),
      Source::Legacy(_) => Ok(()),
    }
  }

  fn prepare(&self, index: usize, contributions: &Contributions) -> Prepared {
    match self {
      Source::Janus(contract) => prepare_janus(contract, index, &self.format(), contributions),
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
  source
    .contracts
    .iter()
    .enumerate()
    .map(|(index, doc)| read_document(index, doc))
    .collect()
}

/// One source document, identified before it is parsed: [`read_source`]'s rule, for a document
/// from anywhere — inline, or fetched from a broker.
pub(crate) fn read_document(index: usize, doc: &Value) -> Result<Source, VerifyError> {
  match doc.get("$format").and_then(Value::as_str) {
    Some(format) if crate::persisted::CONTRACT_FORMAT.reads(format) => {
      Ok(Source::Janus(read_contract(index, doc)?))
    }
    // A `$format` this engine does not know is never guessed at, even when the document also
    // looks pact-shaped: the member exists precisely to stop a reader guessing.
    Some(other) => Err(VerifyError::NotAContract {
      index,
      found: Some(other.to_string()),
    }),
    None => Ok(Source::Legacy(Box::new(read_legacy(index, doc)?))),
  }
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
  let pact = legacy_pact::read(&format!("source.contracts[{index}]"), doc).map_err(|err| match err {
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
    interactions: legacy_pact::interactions(pact.as_ref()),
  })
}

/// The members every v1–v4 pact file has by its own specification: named parties and a body of
/// interactions (or messages, for a pact this engine reads and then finds nothing HTTP in — which
/// is a pact that verifies zero interactions, a different and honest answer from a misparse).
pub(super) fn looks_like_a_pact(doc: &Value) -> bool {
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

/// What a run's scope lends it: the content components that decode and encode its slots, and what
/// its declared components contribute to its plans (plan task 8.4).
pub(crate) struct RunComponents {
  pub content: Option<Arc<dyn ContentComponent>>,
  pub contributions: Arc<Contributions>,
}

/// What a run verifies: documents already read, or a broker to fetch them from inside the run
/// (spec §8.3: `verify` answers at once, so the fetch cannot happen before it).
pub(crate) enum Sources {
  Read(Vec<Source>),
  Broker(Box<BrokerFetch>),
}

pub(crate) struct BrokerFetch {
  pub client: Arc<dyn BrokerClient>,
  pub request: FetchRequest,
  pub require_pacts: bool,
  /// The run's scope, which every fetched document's requirements are checked against before
  /// anything is verified, as an inline document's are before `verify` answers.
  pub scope: Scope,
}

/// A run's broker, once it has fetched: the client, which broker, and how to publish.
struct BrokerRun {
  client: Arc<dyn BrokerClient>,
  broker: BrokerDescriptor,
  publish: Option<PublishOptions>,
  /// Per source, in order: what the broker said about it.
  fetched: Vec<FetchedPact>,
}

/// Start the run. Returns immediately (spec §8.3: "`verify` starts a verification session and
/// returns immediately") — everything after this point is events on `stream`.
///
/// `filter` narrows the run (variant-semantics spec §5.1). An inline source's documents were
/// checked against it before `verify` answered; a broker source's are checked once fetched.
#[allow(clippy::too_many_arguments)]
pub(crate) fn start(
  sources: Sources,
  targets: Vec<Target>,
  components: RunComponents,
  hooks: Option<HookRunner>,
  options: Option<Value>,
  filter: Filter,
  publish: Option<PublishOptions>,
  stream: Arc<Stream>,
) -> Run {
  let executed_plans = ExecutedPlans::from_options(options.as_ref());
  let thread_stream = Arc::clone(&stream);
  let crashed = match &sources {
    Sources::Read(sources) => crashed_summary(sources, &filter),
    Sources::Broker(_) => crashed_summary(&[], &filter),
  };
  let thread = thread::spawn(move || {
    let stream = Arc::clone(&thread_stream);
    guarded(&stream, crashed, move || {
      let (sources, broker) = match sources {
        Sources::Read(sources) => (sources, None),
        Sources::Broker(fetch) => match fetch_sources(&fetch, &filter, &thread_stream) {
          Ok((sources, fetched)) => (
            sources,
            Some(BrokerRun {
              client: fetch.client,
              broker: fetch.request.broker,
              publish,
              fetched,
            }),
          ),
          Err(summary) => {
            stop_targets(&targets);
            thread_stream.finish("verification/finished", summary);
            return;
          }
        },
      };
      run(
        sources,
        broker,
        targets,
        components,
        hooks,
        filter,
        executed_plans,
        thread_stream,
      )
    })
  });
  Run { stream, thread }
}

/// Fetch a broker source's pacts and read them (spec §8.3), reporting what was fetched as a
/// `verification/fetched` event. A run that cannot get past this ends here: the summary it ends
/// with is the `Err`.
fn fetch_sources(
  fetch: &BrokerFetch,
  filter: &Filter,
  stream: &Stream,
) -> Result<(Vec<Source>, Vec<FetchedPact>), Value> {
  let failed = |error: EngineError| {
    let mut summary = crashed_summary(&[], filter);
    summary["error"] = serde_json::to_value(error).unwrap_or(Value::Null);
    summary
  };
  let fetched = fetch
    .client
    .fetch_for_verification(&fetch.request)
    .map_err(|error| failed(EngineError::component_failed("broker", &error)))?;
  stream.emit(
    "verification/fetched",
    json!({
      "broker": fetch.request.broker.display_url(),
      "pacts": fetched.iter().map(|pact| json!({
        "consumer": pact.document["consumer"]["name"],
        "pending": pact.pending,
        "wip": pact.wip,
        "notices": pact.notices,
      })).collect::<Vec<_>>(),
    }),
  );
  if fetched.is_empty() && fetch.require_pacts {
    return Err(failed(EngineError::pacts_not_found(&fetch.request.provider)));
  }
  let mut sources = Vec::with_capacity(fetched.len());
  for (index, pact) in fetched.iter().enumerate() {
    let source =
      read_document(index, &pact.document).map_err(|err| failed(super::engine::verify_error(err)))?;
    source
      .check(&fetch.scope)
      .map_err(|unavailable| failed(EngineError::component_unresolved(&unavailable)))?;
    sources.push(source);
  }
  // ADR 0041: a filter naming an id none of them records ends the run here, before anything runs
  // or is published, as one that fetched nothing does.
  filter.check(&sources).map_err(failed)?;
  Ok((sources, fetched))
}

fn stop_targets(targets: &[Target]) {
  for target in targets {
    if let Err(err) = target.component.stop(Stop {
      instance: target.instance.clone(),
    }) {
      tracing::warn!(kind = %target.kind, instance = %target.instance, ?err, "transport did not stop cleanly");
    }
  }
}

/// Run `body`, and if it panics, end the stream anyway (spec §9.6): a host polls until the terminal
/// event, so a run that died without one would leave it waiting forever. The panic becomes the
/// summary's `error`, the same `internal` code the dispatch boundary gives a panicking request
/// (§10.1). A run that already finished its stream before panicking keeps the summary it sent.
fn guarded(stream: &Stream, mut crashed: Value, body: impl FnOnce()) {
  if let Err(panic) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(body)) {
    let detail = panic
      .downcast_ref::<&str>()
      .map(|text| text.to_string())
      .or_else(|| panic.downcast_ref::<String>().cloned())
      .unwrap_or_else(|| "no detail".to_string());
    tracing::error!(stream = %stream.id(), %detail, "the verification run panicked");
    let error = EngineError::internal(format!("the verification run stopped unexpectedly: {detail}"));
    crashed["error"] = serde_json::to_value(error).unwrap_or(Value::Null);
    stream.finish("verification/finished", crashed);
  }
}

/// The summary a run that panicked ends with: who it was about and that it failed. What it had
/// tallied died with it, so `variants` and `failures` are left out rather than reported as zero.
fn crashed_summary(sources: &[Source], filter: &Filter) -> Value {
  json!({
    "status": "failed",
    "contracts": sources.len(),
    "consumers": sources.iter().map(|s| s.consumer().to_string()).collect::<Vec<_>>(),
    "providers": sources.iter().map(|s| s.provider().to_string()).collect::<Vec<_>>(),
    "interactions": sources.iter().map(Source::interactions).sum::<usize>(),
    "filtered": sources.iter().any(|source| filter.narrows(source)),
  })
}

#[allow(clippy::too_many_arguments)]
fn run(
  sources: Vec<Source>,
  broker: Option<BrokerRun>,
  targets: Vec<Target>,
  components: RunComponents,
  hooks: Option<HookRunner>,
  filter: Filter,
  executed_plans: ExecutedPlans,
  stream: Arc<Stream>,
) {
  let RunComponents {
    content,
    contributions,
  } = components;
  let interactions: usize = sources.iter().map(Source::interactions).sum();
  let variants: usize = sources.iter().map(Source::variants).sum();
  // Per source, whether the filter excludes any of its variants (ADR 0041 decision 2): known before
  // anything runs, so `started` can say it, and what each pair's status and publication follow.
  let narrowed: Vec<bool> = sources.iter().map(|source| filter.narrows(source)).collect();
  stream.emit(
    "verification/started",
    json!({
      "contracts": sources.len(),
      "interactions": interactions,
      "variants": variants,
      "consumers": sources.iter().map(|s| s.consumer().to_string()).collect::<Vec<_>>(),
      "providers": sources.iter().map(|s| s.provider().to_string()).collect::<Vec<_>>(),
      // Positionally parallel to `providers`: which format each source document was written in
      // (plan task 5.4). A run over a mix of the two says so in its first event.
      "formats": sources.iter().map(Source::format).collect::<Vec<_>>(),
      "transports": targets.iter().map(|t| t.kind.clone()).collect::<Vec<_>>(),
      "filtered": narrowed.contains(&true),
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
    &mut ExchangeScratch::default(),
    &stream,
  );

  // What each source's part of the run was: its results, by position in `tally.results`, and the
  // variants the filter skipped in it. What its `pairs` entry counts and its published result is
  // made of (ADR 0033). A source the run never reached has none.
  let mut ran: Vec<SourceRun> = Vec::with_capacity(sources.len());
  if aborted.is_none() {
    'sources: for (position, source) in sources.iter().enumerate() {
      let first = tally.results.len();
      let skipped = tally.skipped;
      let source_pending = broker
        .as_ref()
        .and_then(|broker| broker.fetched.get(position))
        .is_some_and(|fetched| fetched.pending);
      for index in 0..source.interactions() {
        let flow = verify_prepared(
          source.prepare(index, &contributions),
          &mut RunContext {
            targets: &targets,
            content: content.as_deref(),
            actions: contributions.as_ref(),
            filter: &filter,
            executed_plans,
            stream: &stream,
            hooks: hooks.as_mut(),
            pending: source_pending || source.interaction_pending(index),
          },
          &mut tally,
        );
        if let Flow::Abort(abort) = flow {
          aborted = Some(abort);
          ran.push(SourceRun {
            results: first..tally.results.len(),
            skipped: tally.skipped - skipped,
            complete: false,
          });
          break 'sources;
        }
      }
      ran.push(SourceRun {
        results: first..tally.results.len(),
        skipped: tally.skipped - skipped,
        complete: true,
      });
    }
  }

  let publication = match &broker {
    Some(broker) => publish(broker, &sources, &ran, &narrowed, &tally, &stream),
    None => vec![],
  };
  let report = Report {
    tally: &tally,
    sources: &sources,
    interactions,
    narrowed: &narrowed,
    hooks: None,
    aborted: &aborted,
    pending: broker
      .as_ref()
      .map(|broker| broker.fetched.iter().map(|fetched| fetched.pending).collect()),
    publication: &publication,
    ran: &ran,
  };

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
  let summary_for_hooks = summary(&Report {
    hooks: hooks.as_ref(),
    ..report.clone()
  });
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
      &mut ExchangeScratch::default(),
      &stream,
    );
  }

  // Transports are stopped *before* the terminal event, not after: spec §7.1 says everything the
  // session allocated is released when it ends, and the host learns it ended from that event. A
  // host that saw `finished` while a client instance was still open would be right to call it a
  // leak.
  stop_targets(&targets);

  stream.finish(
    "verification/finished",
    summary(&Report {
      hooks: hooks.as_ref(),
      ..report
    }),
  );
}

/// Publish each source's result to its own link (spec §8.3's `options.publish`), as a
/// `verification/published` event each. What the summary's `publication` lists. A publish that
/// fails is reported, and changes no verdict.
fn publish(
  broker: &BrokerRun,
  sources: &[Source],
  ran: &[SourceRun],
  narrowed: &[bool],
  tally: &Tally,
  stream: &Stream,
) -> Vec<Value> {
  let Some(options) = &broker.publish else {
    return vec![];
  };
  let mut publication = Vec::new();
  // Only a source the run finished: a partial result published as the pact's would be a verdict
  // on interactions nobody verified.
  let finished = ran.iter().filter(|run| run.complete);
  for (((source, fetched), run), narrowed) in sources.iter().zip(&broker.fetched).zip(finished).zip(narrowed)
  {
    // Nor a source the filter narrowed (ADR 0041 decision 4): what it replayed is not the pact's
    // result. Said, not silently left out, so a reader knows why the broker did not hear of it.
    if *narrowed {
      let entry = json!({ "consumer": source.consumer(), "outcome": "skipped", "reason": "filtered" });
      let mut event = entry.clone();
      event["provider-version"] = json!(options.provider_version);
      stream.emit("verification/published", event);
      publication.push(entry);
      continue;
    }
    let results: Vec<InteractionResult> = tally.results[run.results.clone()]
      .iter()
      .map(|payload| InteractionResult {
        id: payload["interaction"]["id"].as_str().map(str::to_string),
        description: payload["interaction"]["description"]
          .as_str()
          .unwrap_or_default()
          .to_string(),
        success: payload["status"] == json!("verified"),
        mismatches: payload["mismatches"].as_array().cloned().unwrap_or_default(),
        error: payload.get("error").filter(|error| !error.is_null()).cloned(),
      })
      .collect();
    let request = PublishRequest {
      broker: &broker.broker,
      handle: &fetched.handle,
      provider_version: &options.provider_version,
      branch: options.branch.as_deref(),
      tags: &options.tags,
      build_url: options.build_url.as_deref(),
      success: results.iter().all(|result| result.success),
      results,
    };
    let mut entry = json!({ "consumer": source.consumer() });
    match broker.client.publish_results(&request) {
      Ok(()) => entry["outcome"] = json!("published"),
      Err(error) => {
        entry["outcome"] = json!("failed");
        entry["error"] =
          serde_json::to_value(EngineError::component_failed("broker", &error)).unwrap_or(Value::Null);
      }
    }
    let mut event = entry.clone();
    event["provider-version"] = json!(options.provider_version);
    stream.emit("verification/published", event);
    publication.push(entry);
  }
  publication
}

/// Run one point's hooks and emit each invocation as a `verification/hook` event (spec §10.1).
/// Returns the abort, if one happened — every other outcome is the caller's to interpret, because
/// only it knows what an exchange is.
fn run_point(
  hooks: Option<&mut HookRunner>,
  point: &str,
  occurrence: &Occurrence,
  parts: Option<&mut Parts>,
  exchange: &mut ExchangeScratch,
  stream: &Stream,
) -> Option<Abort> {
  let runner = hooks?;
  let outcome = runner.run_point(point, occurrence, parts, exchange, &mut |payload| {
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
  /// Of `failed` and `state_unavailable`, the variants that were pending (spec §8.3): reported,
  /// counted, and not failing the run.
  pending_failed: usize,
  failures: Vec<Value>,
  /// Every result, in order: what a source's published result is made of.
  results: Vec<Value>,
}

impl Tally {
  fn record(&mut self, payload: &Value) {
    self.results.push(payload.clone());
    if payload["pending"] == json!(true) && payload["status"] != json!("verified") {
      self.pending_failed += 1;
    }
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

/// One source's part of a run (ADR 0033).
struct SourceRun {
  /// Its results, by position in [`Tally::results`].
  results: std::ops::Range<usize>,
  /// Its variants the filter excluded.
  skipped: usize,
  /// Whether the run got to the end of it: a hook that aborts the run part way leaves the source
  /// it was in incomplete.
  complete: bool,
}

/// One source's entry in the summary's `pairs` (ADR 0033): who it was between, how it went, and
/// its own variant counts, which the run-level `variants` adds up. A source the filter narrowed is
/// `filtered` unless something in it failed (ADR 0041 decision 3): what it replayed passing is not
/// the pair verified.
fn pair_entry(source: &Source, run: Option<&SourceRun>, narrowed: bool, results: &[Value]) -> Value {
  let results = run.map_or(&[][..], |run| &results[run.results.clone()]);
  let count = |status: &str| {
    results
      .iter()
      .filter(|result| result["status"] == json!(status))
      .count()
  };
  let verified = count("verified");
  let state_unavailable = count("state-unavailable");
  let failed = results.len() - verified - state_unavailable;
  let skipped = run.map_or(0, |run| run.skipped);
  let status = if failed + state_unavailable > 0 {
    "failed"
  } else if !run.is_some_and(|run| run.complete) {
    "incomplete"
  } else if narrowed {
    "filtered"
  } else {
    "verified"
  };
  json!({
    "consumer": source.consumer(),
    "provider": source.provider(),
    "status": status,
    "variants": {
      "total": results.len() + skipped,
      "verified": verified,
      "failed": failed,
      "state-unavailable": state_unavailable,
      "skipped": skipped,
    },
  })
}

/// What the run summary is written from.
#[derive(Clone)]
struct Report<'a> {
  tally: &'a Tally,
  sources: &'a [Source],
  interactions: usize,
  /// Per source, whether the filter excluded any of its variants.
  narrowed: &'a [bool],
  hooks: Option<&'a HookRunner>,
  aborted: &'a Option<Abort>,
  /// For a broker source: per source, whether it was pending.
  pending: Option<Vec<bool>>,
  /// For a run that published: per source, what happened.
  publication: &'a [Value],
  /// Per source the run reached, its part of it.
  ran: &'a [SourceRun],
}

/// The run summary (protocol spec §9.6): the document the terminal event carries, the one
/// `after-verification` is handed, and the one a report is written from.
fn summary(report: &Report<'_>) -> Value {
  let Report {
    tally,
    sources,
    interactions,
    narrowed,
    hooks,
    aborted,
    ..
  } = report;
  // `state-unavailable` fails the run by default (variant-semantics spec §6.7): the consumer
  // demonstrated it can handle that variant and the provider has not demonstrated it can produce
  // it, so reporting it as a pass would be the false confidence variant testing exists to remove.
  let failed = tally.failed + tally.state_unavailable;
  // A pending variant's failure is reported and does not fail the run (spec §8.3).
  let failing = failed - tally.pending_failed;
  let filtered = narrowed.contains(&true);
  // A narrowed run that nothing failed is not a pass (variant-semantics spec §5.1, ADR 0041): it
  // says what it replayed passed, and that it did not replay everything.
  let status = if failing > 0 || aborted.is_some() {
    "failed"
  } else if filtered {
    "filtered"
  } else {
    "verified"
  };
  let mut summary = json!({
    "status": status,
    "contracts": sources.len(),
    // Who this run was about, positionally parallel, one entry per source document — the same
    // pairing `started` announces. The summary is the document a report is written from (below),
    // and plan task 7.4's decision is *per pair*: a run that said "verified" without saying which
    // consumer and which provider it verified could not be read back as an answer about either.
    "consumers": sources.iter().map(|s| s.consumer().to_string()).collect::<Vec<_>>(),
    "providers": sources.iter().map(|s| s.provider().to_string()).collect::<Vec<_>>(),
    "interactions": interactions,
    // `filtered` rides in the summary as well as in `started` and `status`, for a reader that
    // reads the run's counts without its status: unreplayed variants are not passing ones.
    "filtered": filtered,
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
  // Positionally beside `consumers` (spec §9.6): which sources were pending, so a reader can tell
  // a pending pact's failures from the ones that fail the run.
  if let Some(pending) = &report.pending {
    map.insert("pending".to_string(), json!(pending));
  }
  if !report.publication.is_empty() {
    map.insert("publication".to_string(), json!(report.publication));
  }
  // Positionally beside `consumers` (ADR 0033, Phase 9 finding 10): each source's own counts,
  // which a pair's report and a broker's matrix row are about.
  map.insert(
    "pairs".to_string(),
    Value::Array(
      sources
        .iter()
        .enumerate()
        .map(|(position, source)| {
          pair_entry(
            source,
            report.ran.get(position),
            narrowed.get(position).copied().unwrap_or(false),
            &tally.results,
          )
        })
        .collect(),
    ),
  );
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
  /// Runs a contributed fragment's component actions (plan task 8.4).
  actions: &'a dyn ActionApplier,
  filter: &'a Filter,
  executed_plans: ExecutedPlans,
  stream: &'a Stream,
  hooks: Option<&'a mut HookRunner>,
  /// The interaction is pending — its pact is, or it is itself (a v4 interaction marked so): its
  /// results say `pending: true`, and its failures do not fail the run (spec §8.3).
  pending: bool,
}

impl RunContext<'_> {
  fn mark(&self, payload: &mut Value) {
    if self.pending {
      payload["pending"] = json!(true);
    }
  }
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

/// Where the parts a replay scores come from.
enum Answer {
  /// The replay's request is sent through the target for its transport, and its reply is scored.
  Driven,
  /// A `produce-message` hook produces them (lifecycle-hooks spec §3.4, staged plan S1.7): a
  /// message, asked of the provider's own producer. `part` is the one the plan scores, named by
  /// whoever prepared the replay.
  Produced { part: String },
}

/// Everything one replay needs, with nothing left to decide: what to send, what scores the reply,
/// how the reply's values are read back, and what the hooks are told.
struct Ready {
  variant: String,
  answer: Answer,
  origin: Value,
  assignment: Value,
  states: Vec<contract::ResolvedState>,
  /// The bytes the consumer actually sent (variant-semantics spec §5.2) — a contract's recorded
  /// variant or a pact's own request example. Never re-derived from a shape on either path.
  request: Parts,
  /// A v1–v4 pact's own request, kept when it has generators to apply (ADR 0034): the request
  /// sent is generated from it once the exchange's states are set up, since a `ProviderState`
  /// generator reads what they returned. `None` on the shape path, and for a pact request with
  /// none, which replays `request` as it stands.
  generate_from: Option<Box<RequestTemplate>>,
  /// A contract's request shapes, by part, for the parts whose shapes name a generator (shape-language
  /// spec §3.4, ADR 0048): the recorded request is filled with fresh values there once the states
  /// are set up, as a pact's is generated. Empty when nothing is generated.
  fresh: BTreeMap<String, contract::ShapePart>,
  /// Pinned to this variant on the shape path, compiled from matching rules on the v1–v4 path.
  plan: plan::Plan,
  /// How the reply becomes the values `plan` resolves against. A function rather than a flag
  /// because the difference *is* a different reading of the same parts: design 3.5's plans
  /// address a header as one value per name (`legacy_pact::header_captures`), shapes address the
  /// slot the transport actually produced.
  resolve: fn(&Parts, &ContentTypes, Option<&dyn ContentComponent>) -> CapturedValues,
  /// The interaction's declared content types (contract-file spec §5.5): the request's declared
  /// slots are encoded through them before sending, the reply's decoded under them. Empty for a
  /// v1–v4 pact, which has no declarations.
  content_types: ContentTypes,
  /// Emitted before this variant's exchange starts, so a warning precedes the result it qualifies.
  warnings: Vec<Value>,
  /// The `interaction` document a hook's occurrence carries (lifecycle-hooks spec §3.2), with this
  /// variant's states already resolved.
  interaction: Value,
}

/// A Janus contract's interaction (plan tasks 5.1–5.3): compiled from its recorded shapes, one
/// replay per recorded variant.
fn prepare_janus(contract: &Contract, index: usize, format: &str, contributions: &Contributions) -> Prepared {
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
      let request = recorded_parts(variant);
      let states_json = serde_json::to_value(&states).unwrap_or(Value::Null);
      Replay::Ready(Box::new(Ready {
        variant: variant.id.clone(),
        answer: Answer::Driven,
        origin: json!(variant.origin),
        assignment: json!(variant.assignment),
        // Pinned to this variant (shape spec §7.1, variant-semantics spec §5.2 step 3): an
        // `optional` pinned to `absent` admits only absence, an `any-of` pinned to `SHIPPED` only
        // that. Matching against the unpinned shape would accept `PENDING` where the consumer
        // demonstrated `SHIPPED`, and the variant would have proved nothing.
        plan: contributions.compile(&spec, &interaction.parts, &assignment, Some(&variant.id)),
        resolve: parts_resolver,
        content_types: interaction.content_types.clone().unwrap_or_default(),
        states: states.unwrap_or_default(),
        request,
        generate_from: None,
        fresh: fresh_parts(&interaction.parts),
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
  let interaction = match &pact.interactions[index] {
    PactInteraction::Http(http) => http,
    PactInteraction::Message(message) => return prepare_legacy_message(pact, message, index, format),
    PactInteraction::NeedsPlugins(needs) => return refuse_plugin_interaction(pact, needs, index, format),
  };
  let states = legacy_states(&interaction.provider_states);
  let reference = legacy_reference(
    pact,
    index,
    &interaction.description,
    &states,
    format,
    interaction.id.as_ref(),
  );
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
      answer: Answer::Driven,
      origin: json!(crate::variant::Origin::Base),
      assignment: json!([]),
      plan: compile_legacy_response(&response),
      resolve: legacy_resolver,
      // The reply's body is read as the pact's content type (ADR 0036 decision 4); the request
      // replays the recorded octets, so it declares nothing.
      content_types: legacy_pact::response_types(&response),
      states,
      request: legacy_pact::request_parts(&interaction.request),
      generate_from: RequestTemplate::of(&interaction.request).map(Box::new),
      fresh: BTreeMap::new(),
      warnings: Vec::new(),
      interaction: json!({
        "description": interaction.description,
        "states": states_json,
        "transport": { "kind": DEFAULT_TRANSPORT_KIND },
      }),
    }))],
  }
}

/// A v1–v4 interaction's provider states as a variant needs them. v3 state parameters travel as
/// they stand — a `state-setup` hook in `pact-state-change` format hands the provider exactly the
/// `{state, params, action}` it received before Janus existed (lifecycle-hooks spec §8.4). v1/v2's
/// bare state string arrives here as a state with no parameters, which is what it is.
fn legacy_states<S>(states: &[S]) -> Vec<contract::ResolvedState>
where
  S: legacy_pact::StateLike,
{
  states.iter().map(legacy_pact::StateLike::resolved).collect()
}

/// How a v1–v4 interaction is named in every event of the run.
fn legacy_reference(
  pact: &LegacyPact,
  index: usize,
  description: &str,
  states: &[contract::ResolvedState],
  format: &str,
  id: Option<&String>,
) -> Value {
  let mut reference = json!({
    "contract": { "consumer": pact.consumer, "provider": pact.provider },
    "index": index,
    "description": description,
    "states": states.iter().map(|state| state.name.clone()).collect::<Vec<_>>(),
    "format": format,
  });
  // The broker's id for the interaction, which a published result is keyed by (spec §8.3).
  if let Some(id) = id {
    reference["id"] = json!(id);
  }
  reference
}

/// A v1–v4 message interaction (staged plan S1.7): one variant, as an HTTP one has, whose message is
/// produced by a `produce-message` hook rather than driven through a transport (lifecycle-hooks spec
/// §3.4), and compared with the one the pact expects as today's verifier compares it. The hooks are
/// told what today's verifier would POST for it (`pact-message`), which the `http` hook's
/// `pact-message` format sends.
fn prepare_legacy_message(
  pact: &LegacyPact,
  message: &MessageInteraction,
  index: usize,
  format: &str,
) -> Prepared {
  let states = legacy_states(&message.provider_states);
  let reference = legacy_reference(
    pact,
    index,
    &message.description,
    &states,
    format,
    message.id.as_ref(),
  );
  // The transport it is routed by, which a hook's `when` selects on: the one the pact names, else
  // the message kind.
  let transport = message
    .transport
    .clone()
    .unwrap_or_else(|| MESSAGE_TRANSPORT_KIND.to_string());

  let expected = match legacy_pact::legacy_message(message.expected.as_ref(), message.responses) {
    Ok(expected) => expected,
    Err(err) => {
      let part = if message.synchronous {
        "response/0"
      } else {
        "contents"
      };
      return Prepared {
        reference,
        transport,
        replays: vec![Replay::Broken {
          variant: LEGACY_VARIANT.to_string(),
          error: json!({
            "code": "contract-invalid",
            "problems": [{ "pointer": format!("/interactions/{index}/{part}"), "message": err.to_string() }],
          }),
        }],
      };
    }
  };

  let states_json = serde_json::to_value(&states).unwrap_or(Value::Null);
  let mut pact_message = json!({ "request": message.request, "synchronous": message.synchronous });
  if let Some(named) = &message.transport {
    pact_message["transport"] = json!(named);
  }
  Prepared {
    reference,
    transport: transport.clone(),
    replays: vec![Replay::Ready(Box::new(Ready {
      variant: LEGACY_VARIANT.to_string(),
      answer: Answer::Produced {
        part: legacy_pact::MESSAGE_PART.to_string(),
      },
      origin: json!(crate::variant::Origin::Base),
      assignment: json!([]),
      plan: legacy_compile::compile_message(&expected, legacy_pact::MESSAGE_PART),
      resolve: legacy_message_resolver,
      content_types: legacy_pact::message_types(&expected),
      states,
      request: Parts::new(),
      generate_from: None,
      fresh: BTreeMap::new(),
      warnings: Vec::new(),
      interaction: json!({
        "description": message.description,
        "states": states_json,
        "transport": { "kind": transport },
        "pact-message": pact_message,
      }),
    }))],
  }
}

/// The kind a v1–v4 message is routed by when its pact names no transport.
const MESSAGE_TRANSPORT_KIND: &str = "message";

/// An interaction that needs a pact-plugins plugin (staged plan S1.8): refused before anything runs,
/// with no state set up and nothing sent, as `component-unavailable` naming each plugin it needs. It
/// is a result like any other, so the summary counts it as failed, and a pact that mixes it with
/// interactions the engine can verify is never reported as verified.
fn refuse_plugin_interaction(
  pact: &LegacyPact,
  needs: &PluginInteraction,
  index: usize,
  format: &str,
) -> Prepared {
  let mut reference = json!({
    "contract": { "consumer": pact.consumer, "provider": pact.provider },
    "index": index,
    "description": needs.description,
    "states": needs.provider_states.iter().map(|state| state.name.clone()).collect::<Vec<_>>(),
    "format": format,
  });
  if let Some(id) = &needs.id {
    reference["id"] = json!(id);
  }
  let error =
    serde_json::to_value(EngineError::pact_plugins_unavailable(&needs.plugins)).unwrap_or(Value::Null);
  Prepared {
    reference,
    transport: DEFAULT_TRANSPORT_KIND.to_string(),
    replays: vec![Replay::Broken {
      variant: LEGACY_VARIANT.to_string(),
      error,
    }],
  }
}

/// The sole variant of a converted interaction (contract-file spec §8.3).
const LEGACY_VARIANT: &str = "base";

/// A reply read the way design 3.5's plans address it: the generic slot captures, the body decoded
/// as the pact's content type ([`legacy_pact::response_types`]), plus one string per header name
/// ([`legacy_pact::header_captures`] documents why the two forms differ).
fn legacy_resolver(
  parts: &Parts,
  declared: &ContentTypes,
  content: Option<&dyn ContentComponent>,
) -> CapturedValues {
  legacy_pact::header_captures(parts_resolver(parts, declared, content), parts)
}

/// A produced message read the way a message's plan addresses it: the generic slot captures, the
/// contents decoded as the pact's content type, and the content type today's verifier takes the
/// produced one to have ([`legacy_pact::message_captures`]).
fn legacy_message_resolver(
  parts: &Parts,
  declared: &ContentTypes,
  content: Option<&dyn ContentComponent>,
) -> CapturedValues {
  legacy_pact::message_captures(parts_resolver(parts, declared, content), parts)
}

/// A v1–v4 body design 3.5 cannot compile, as the error a result carries: a body that claimed to be
/// JSON, or another type, and was not is a **document** problem, carrying a pointer into the pact its
/// author can act on. Every body kind is compiled since S1.23, so none is refused as a missing
/// component.
fn unsupported_body(err: &legacy_pact::Unsupported, index: usize, part: &str) -> Value {
  match err {
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
    // silent skip — a verifier that cannot speak an interaction's transport has not verified it. A
    // produced message needs no transport: a hook produces it.
    if target.is_none() && matches!(ready.answer, Answer::Driven) {
      let error = json!({
        "code": "component-unavailable",
        "component": format!("transport/{}", prepared.transport),
      });
      report_failure(ctx, tally, &reference, &ready.variant, error);
      continue;
    }

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
  let mut payload = json!({
    "interaction": reference, "variant": variant, "status": "failed", "error": error,
  });
  ctx.mark(&mut payload);
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
fn verify_exchange(
  exchange: &Exchange<'_>,
  target: Option<&Target>,
  ctx: &mut RunContext<'_>,
) -> (Value, Flow) {
  let mut scratch = ExchangeScratch::default();
  let base = Occurrence {
    interaction: Some(exchange.ready.interaction.clone()),
    variant: Some(json!({
      "id": exchange.ready.variant,
      "assignment": exchange.ready.assignment,
    })),
    exchange: Some(json!({ "id": exchange.id })),
    endpoint: target.map(|target| target.endpoint.clone()),
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
    let (status, aborted) = hook_point(ctx, "state-setup", &occurrence, None, &mut scratch);
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

  // 2. The exchange itself: before-request and a driven send, or produce-message (spec §4.1:
  //    whichever the interaction calls for, never both).
  let mut reply = None;
  let mut executed_plan = None;
  if result.is_none() {
    match (&exchange.ready.answer, target) {
      (Answer::Produced { part }, _) => match produce(exchange, part, &base, ctx, &mut scratch) {
        Produced::Scored(driven) => {
          result = Some(driven.payload);
          executed_plan = driven.executed;
        }
        Produced::Failed(payload) => result = Some(payload),
        Produced::Aborted(abort) => {
          result = Some(hook_failure_payload(
            exchange,
            "failed",
            &abort.hook,
            &abort.error,
          ));
          flow = Flow::Abort(abort);
        }
      },
      (Answer::Driven, Some(target)) => match drive_exchange(exchange, target, &base, ctx, &mut scratch) {
        Ok(driven) => {
          result = Some(driven.payload);
          reply = driven.reply;
          executed_plan = driven.executed;
        }
        Err(Stopped::Failed(payload)) => result = Some(payload),
        Err(Stopped::Aborted(abort)) => {
          result = Some(hook_failure_payload(
            exchange,
            "failed",
            &abort.hook,
            &abort.error,
          ));
          flow = Flow::Abort(abort);
        }
      },
      // Unreachable: `verify_prepared` reports a driven replay with no target before it gets here.
      (Answer::Driven, None) => {}
    }
  }

  // 3. after-response — observing only (spec §3.6): a hook that could rewrite the response before
  //    matching would be editing the evidence, and matching has already happened by here anyway.
  if let Some(inbound) = reply {
    let mut parts = inbound;
    let (_, aborted) = hook_point(ctx, "after-response", &base, Some(&mut parts), &mut scratch);
    if let Some(abort) = aborted {
      flow = Flow::Abort(abort);
    }
  }

  let mut payload = result.unwrap_or_else(
    || json!({ "interaction": exchange.reference, "variant": exchange.ready.variant, "status": "failed" }),
  );
  ctx.mark(&mut payload);

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
    let (_, aborted) = hook_point(ctx, "state-teardown", &occurrence, None, &mut scratch);
    if let Some(abort) = aborted {
      flow = Flow::Abort(abort);
    }
  }

  (payload, flow)
}

/// Why an exchange stopped before it had a result to score.
enum Stopped {
  /// The variant failed: its result payload.
  Failed(Value),
  /// A hook ended the run.
  Aborted(Abort),
}

/// A driven exchange (spec §4.1): the replay's request — the recorded one, cloned because
/// `before-request` may rewrite it and the recording must survive the run unchanged; or, for a pact
/// with request generators, generated now that the states have answered (ADR 0034), so a
/// `before-request` hook signs the request that is actually sent — then `before-request`, then the
/// send and the score.
fn drive_exchange(
  exchange: &Exchange<'_>,
  target: &Target,
  base: &Occurrence,
  ctx: &mut RunContext<'_>,
  scratch: &mut ExchangeScratch,
) -> Result<Driven, Stopped> {
  let names = target.component.exchange_parts();
  let request = match &exchange.ready.generate_from {
    None if !exchange.ready.fresh.is_empty() => fresh_request(exchange, scratch, ctx),
    None => exchange.ready.request.clone(),
    Some(request) => generated_request(exchange, request, scratch, ctx),
  };
  let Some(mut request) = sent_parts(&request, &names) else {
    return Err(Stopped::Failed(json!({
      "interaction": exchange.reference, "variant": exchange.ready.variant, "status": "failed",
      "error": { "code": "contract-invalid", "problems": [{
        "pointer": format!("/parts/{}", names.initiating),
        "message": format!("the recorded variant has no '{}' part to send", names.initiating),
      }] },
    })));
  };

  let (status, aborted) = hook_point(ctx, "before-request", base, Some(&mut request), scratch);
  if let Some(abort) = aborted {
    return Err(Stopped::Aborted(abort));
  }
  if let HookStatus::FailExchange { hook, error } | HookStatus::StateUnavailable { hook, error } = status {
    // A `before-request` hook that failed to sign a request would otherwise produce a 401 the
    // report would attribute to the provider; the hook's own error names the real cause.
    return Err(Stopped::Failed(hook_failure_payload(
      exchange, "failed", &hook, &error,
    )));
  }

  drive(
    exchange,
    request,
    target,
    ctx.content,
    ctx.actions,
    ctx.executed_plans,
  )
  .map_err(Stopped::Failed)
}

/// What a `produce-message` exchange came to.
enum Produced {
  /// The hooks produced the message, and it was scored.
  Scored(Driven),
  /// The variant failed before anything was scored: a hook failed, or none produced the message.
  Failed(Value),
  /// A hook ended the run.
  Aborted(Abort),
}

/// A produced exchange (lifecycle-hooks spec §3.4, staged plan S1.7): the `produce-message` hooks
/// are handed empty parts and produce the message, and `part` of what they produced is scored, as a
/// driven reply is. Where the parts came from is not a matching input (spike 1.5 finding 6): from
/// here on this is [`drive`]'s scoring.
fn produce(
  exchange: &Exchange<'_>,
  part: &str,
  base: &Occurrence,
  ctx: &mut RunContext<'_>,
  scratch: &mut ExchangeScratch,
) -> Produced {
  let mut parts = Parts::new();
  let (status, aborted) = hook_point(ctx, "produce-message", base, Some(&mut parts), scratch);
  if let Some(abort) = aborted {
    return Produced::Aborted(abort);
  }
  if let HookStatus::FailExchange { hook, error } | HookStatus::StateUnavailable { hook, error } = status {
    return Produced::Failed(hook_failure_payload(exchange, "failed", &hook, &error));
  }
  if !parts.contains_key(part) {
    // No hook ran, or none produced the part: the provider was never asked, so nothing about it
    // was verified, and the result says how to ask it.
    return Produced::Failed(json!({
      "interaction": exchange.reference, "variant": exchange.ready.variant, "status": "failed",
      "error": EngineError::message_not_produced(part),
    }));
  }
  Produced::Scored(score(
    exchange,
    parts,
    part,
    ctx.content,
    ctx.actions,
    ctx.executed_plans,
  ))
}

/// A pact's request with its generators applied (ADR 0034), in the wire vocabulary a transport
/// sends. The provider-state values are every state's `params` in order, then whatever the
/// `state-setup` hooks returned: today's precedence (lifecycle-hooks spec §3.2). A generator that
/// produced nothing leaves the recorded value and says so, before the result it qualifies.
fn generated_request(
  exchange: &Exchange<'_>,
  request: &RequestTemplate,
  scratch: &ExchangeScratch,
  ctx: &RunContext<'_>,
) -> Parts {
  let provider_state = provider_state(exchange, scratch);
  let (generated, failures) = request.generate(&provider_state, &PactGenerators);
  let failures = failures
    .into_iter()
    .map(|failure| fresh::Failure {
      target: failure.target,
      generator: failure.generator,
      message: failure.message,
    })
    .collect();
  warn_generator_failures(exchange, failures, ctx);
  generated
}

/// The provider-state values a generator reads (ADR 0034 decision 3): every state's `params` in
/// order, then whatever the `state-setup` hooks returned, a later one replacing an earlier one.
fn provider_state(exchange: &Exchange<'_>, scratch: &ExchangeScratch) -> Map<String, Value> {
  let mut provider_state = Map::new();
  for state in &exchange.ready.states {
    for (name, value) in state.params.iter().flatten() {
      provider_state.insert(name.clone(), value.clone());
    }
  }
  provider_state.extend(scratch.values.iter().map(|(k, v)| (k.clone(), v.clone())));
  provider_state
}

/// A contract's recorded request with every position its shapes give a generator filled with a fresh
/// value, on the provider's side (ADR 0048), the states having answered: the request a
/// `before-request` hook sees and the provider gets. Each slot is filled as the document its shape is
/// about: a slot carried as octets is decoded by its declared type, or the type it is labelled with,
/// and encoded back by the same one.
fn fresh_request(exchange: &Exchange<'_>, scratch: &ExchangeScratch, ctx: &RunContext<'_>) -> Parts {
  let provider_state = provider_state(exchange, scratch);
  let fresh = fresh::Fresh {
    generators: &PactGenerators,
    side: "provider",
    provider_state: &provider_state,
  };
  let mut request = exchange.ready.request.clone();
  let mut failures = Vec::new();
  for (part_name, slots) in request.iter_mut() {
    let Some(shapes) = exchange.ready.fresh.get(part_name) else {
      continue;
    };
    for (slot_name, slot) in slots.iter_mut() {
      let Some(shape) = shapes.get(slot_name).filter(|shape| fresh::has_generators(shape)) else {
        continue;
      };
      let at = format!("$.{part_name}.{slot_name}");
      let declared = crate::common::declared_type(Some(&exchange.ready.content_types), part_name, slot_name);
      fill_slot(slot, shape, declared, &at, &fresh, ctx.content, &mut failures);
    }
  }
  warn_generator_failures(exchange, failures, ctx);
  request
}

/// One slot, filled where its shape gives a generator ([`fresh_request`]). A slot carried as octets
/// that cannot be read as a document, or written back, is sent as recorded, and says so.
fn fill_slot(
  slot: &mut crate::component::SlotValue,
  shape: &Value,
  declared: Option<&str>,
  at: &str,
  fresh: &fresh::Fresh<'_>,
  content: Option<&dyn ContentComponent>,
  failures: &mut Vec<fresh::Failure>,
) {
  if matches!(slot.encoded.as_deref(), None | Some("json")) {
    fresh::fill(shape, &mut slot.content, at, fresh, failures);
    return;
  }
  let unread = |failures: &mut Vec<fresh::Failure>, message: String| {
    failures.push(fresh::Failure {
      target: at.to_string(),
      generator: "every generator in this slot".to_string(),
      message,
    })
  };
  let Some(content_type) = declared.or(slot.content_type.as_deref()).map(str::to_string) else {
    return unread(
      failures,
      "the slot's recorded octets have no content type to read them as a document by, so nothing in \
       them was generated; the recorded value was sent"
        .to_string(),
    );
  };
  let document = super::wire::decode_slot(slot, Some(&content_type), content);
  if matches!(
    document,
    plan::RuntimeValue::Absent | plan::RuntimeValue::Bytes(_)
  ) {
    return unread(
      failures,
      format!(
        "the slot's recorded octets could not be read as '{content_type}', so nothing in them was \
         generated; the recorded value was sent"
      ),
    );
  }
  let mut document = document.to_json();
  let before = failures.len();
  fresh::fill(shape, &mut document, at, fresh, failures);
  let mut encoded = encode_slot(&document, Some(&content_type), content);
  if !matches!(encoded.encoded.as_deref(), Some("base64" | "text")) {
    failures.truncate(before);
    return unread(
      failures,
      format!("the generated document could not be written as '{content_type}'; the recorded value was sent"),
    );
  }
  if encoded.content_type.is_none() {
    encoded.content_type = slot.content_type.clone();
  }
  *slot = encoded;
}

/// Each generator that produced nothing, as a `generator-failed` warning before the result it
/// qualifies (ADR 0034 decision 5).
fn warn_generator_failures(exchange: &Exchange<'_>, failures: Vec<fresh::Failure>, ctx: &RunContext<'_>) {
  for failure in failures {
    tracing::warn!(target = %failure.target, generator = %failure.generator, message = %failure.message, "a generator produced no value; the recorded one was sent");
    ctx.stream.emit(
      "verification/warning",
      json!({
        "code": "generator-failed",
        "interaction": exchange.reference, "variant": exchange.ready.variant,
        "target": failure.target, "generator": failure.generator, "message": failure.message,
      }),
    );
  }
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
  exchange: &mut ExchangeScratch,
) -> (HookStatus, Option<Abort>) {
  let stream = ctx.stream;
  let Some(runner) = ctx.hooks.as_mut() else {
    return (HookStatus::Continue, None);
  };
  let outcome = runner.run_point(point, occurrence, parts, exchange, &mut |payload| {
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

/// What one driven exchange produced: the result payload, the inbound parts (for
/// `after-response`), and the executed plan when the run was asked for one.
struct Driven {
  payload: Value,
  reply: Option<Parts>,
  executed: Option<Value>,
}

/// Send the request and score the reply against this replay's plan ([`score`]) — pinned to the
/// variant on the shape path, compiled from matching rules on the v1–v4 path, and by here just a
/// plan either way. `Err` carries a payload for the cases where nothing came back to match.
fn drive(
  exchange: &Exchange<'_>,
  request: Parts,
  target: &Target,
  content: Option<&dyn ContentComponent>,
  actions: &dyn ActionApplier,
  executed_plans: ExecutedPlans,
) -> Result<Driven, Value> {
  let reference = exchange.reference;
  let variant = &exchange.ready.variant;
  let names = target.component.exchange_parts();
  let Some(answering) = names.answering.as_deref() else {
    // A drive instance whose kind has no answer cannot be scored against a reply; saying so beats
    // a variant verified against nothing.
    let error = EngineError::component_failed(
      &format!("transport/{}", target.kind),
      &crate::component::ComponentError::synthesised(
        "no-answering-part",
        "the transport names no part that answers an exchange, so there is no reply to match",
      ),
    );
    return Err(json!({
      "interaction": reference, "variant": variant, "status": "failed", "error": error,
    }));
  };

  let request = encode_declared(
    request,
    &exchange.ready.content_types,
    &target.component.content_slots(),
    content,
  );
  let sent = target.component.send(SendRequest {
    instance: target.instance.clone(),
    parts: request,
    await_reply: true,
    // How long a provider may take is the drive instance's own configuration (ADR 0035
    // decision 5): the HTTP transport's request timeout, today's 5 s unless the host says
    // otherwise. A run still ends when a provider hangs: a transport that waited forever would be
    // stopped by the component host's per-call deadline (component-interfaces spec §3.4), and the
    // hang reported as this variant's failure.
    timeout_ms: None,
  });
  let reply = match sent {
    Ok(result) => result.reply,
    Err(error) => {
      // The result event reports it; a log line at the default level would say it twice.
      tracing::debug!(%variant, ?error, "the provider could not be driven for this variant");
      // The protocol's `component-failed` (spec §10.2), whose message carries the component's own,
      // so a host can say why a request got no answer without reading the component's document.
      let error = EngineError::component_failed(&format!("transport/{}", target.kind), &error);
      return Err(json!({
        "interaction": reference, "variant": variant, "status": "failed", "error": error,
      }));
    }
  };
  let Some(reply) = reply else {
    // Spec §5.2: "no reply expected" and "reply expected but absent" are different observations,
    // and this is the second one — a transport that answered a driven `send` with no reply cannot
    // be matched against a response shape.
    let error = EngineError::component_failed(
      &format!("transport/{}", target.kind),
      &crate::component::ComponentError::synthesised("no-reply", "the transport returned no reply to match"),
    );
    return Err(json!({
      "interaction": reference, "variant": variant, "status": "failed", "error": error,
    }));
  };

  Ok(score(
    exchange,
    reply,
    answering,
    content,
    actions,
    executed_plans,
  ))
}

/// Score a reply, or a produced message, against the replay's plan. Only the `answering` part is
/// scored. A request was replayed verbatim, so matching it against itself would assert nothing; the
/// provider answers for the response (variant-semantics §5.2). A shape-compiled plan carries every
/// part and the answering subtree is picked out of it, by the name the transport gives it; a v1–v4
/// plan is compiled from the response or the message alone, so its root already *is* that subtree
/// and the fallback is the whole of it.
fn score(
  exchange: &Exchange<'_>,
  reply: Parts,
  answering: &str,
  content: Option<&dyn ContentComponent>,
  actions: &dyn ActionApplier,
  executed_plans: ExecutedPlans,
) -> Driven {
  let reference = exchange.reference;
  let variant = &exchange.ready.variant;
  let resolver = (exchange.ready.resolve)(&reply, &exchange.ready.content_types, content);
  let executed = execute_with(&exchange.ready.plan, &resolver, None, Some(actions));
  let response = find_container(&executed, answering).unwrap_or(&executed);
  let (status, mismatches) = outcome(response);

  let matched = status == Status::Matched;
  let payload = match status {
    Status::Matched => {
      tracing::debug!(%variant, "variant verified");
      json!({ "interaction": reference, "variant": variant, "status": "verified" })
    }
    Status::Mismatched => {
      tracing::debug!(%variant, count = mismatches.len(), "variant failed");
      json!({
        "interaction": reference, "variant": variant, "status": "failed",
        "mismatches": mismatches.iter().map(mismatch_json).collect::<Vec<_>>(),
      })
    }
  };

  // The answering subtree that was scored, not the whole tree. The resolver holds only the reply,
  // so a shape-compiled plan's request half resolves every path to absence and would show the
  // request failing, when the verdict never looked at it. A v1–v4 plan is the response alone, so
  // this is also what makes the two paths read the same.
  let executed = executed_plans.wants(matched).then(|| {
    json!({
      "interaction": reference,
      "variant": variant,
      "status": if matched { "verified" } else { "failed" },
      "text": plan::render_executed(response),
      "plan": plan::plan_json(&exchange.ready.plan),
    })
  });

  Driven {
    payload,
    reply: Some(reply),
    executed,
  }
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
  if let Some(content_types) = &interaction.content_types {
    map.insert("content-types".to_string(), json!(content_types));
  }
  if let Some(requires) = &interaction.requires {
    map.insert("requires".to_string(), json!(requires));
  }
  interaction_spec::parse(&document).map_err(|err| err.problems)
}

/// A replayed request's declared content slots, turned back into octets (contract-file spec §5.5):
/// the contract records a declared slot as its document, tagged with its type, and the transport
/// carries octets. A slot already carried as octets or text is sent as it stands, and so is every
/// slot the transport does not treat as content (component-interfaces spec §5.5).
fn encode_declared(
  mut request: Parts,
  declared: &ContentTypes,
  content_slots: &crate::component::ContentSlots,
  content: Option<&dyn ContentComponent>,
) -> Parts {
  for (part_name, slots) in request.iter_mut() {
    let (Some(types), Some(names)) = (declared.get(part_name), content_slots.get(part_name)) else {
      continue;
    };
    for (slot_name, slot) in slots.iter_mut() {
      let as_document = matches!(slot.encoded.as_deref(), None | Some("json"));
      if let Some(content_type) = types.get(slot_name)
        && names.contains(slot_name)
        && as_document
      {
        *slot = encode_slot(&slot.content, Some(content_type), content);
      }
    }
  }
  request
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

/// The recorded variant's parts, in the wire form `transport/send` takes (variant-semantics spec
/// §5.2 step 2: "the provider sees the bytes the consumer actually sent"). No conversion happens
/// because none is needed — a contract's `SlotValue` *is* the component interface's `SlotValue`
/// (component-interfaces spec §4), which is the whole point of that reuse. Which of them is sent is
/// the transport's to say ([`sent_parts`]), so nothing here names one.
fn recorded_parts(variant: &contract::RecordedVariant) -> Parts {
  variant.parts.clone()
}

/// The parts whose shapes name a generator somewhere, which a replay fills (ADR 0048).
fn fresh_parts(parts: &BTreeMap<String, contract::ShapePart>) -> BTreeMap<String, contract::ShapePart> {
  parts
    .iter()
    .filter(|(_, slots)| slots.values().any(fresh::has_generators))
    .map(|(name, slots)| (name.clone(), slots.clone()))
    .collect()
}

/// The part a replay sends: the one the transport starts an exchange with (kernel-boundary review
/// finding 8). `None` when the replay has no such part, which is the document's problem.
fn sent_parts(parts: &Parts, names: &ExchangeParts) -> Option<Parts> {
  let part = parts.get(&names.initiating)?;
  Some(Parts::from([(names.initiating.clone(), part.clone())]))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn a_run_that_panics_still_ends_its_stream_with_the_panic_as_its_error() {
    let stream = Stream::new("s-1".to_string());
    guarded(&stream, json!({ "status": "failed", "contracts": 1 }), || {
      panic!("a bug in the run");
    });
    let events = stream.drain(10);
    assert_eq!(events.len(), 1);
    assert!(events[0].last);
    assert_eq!(events[0].kind, "verification/finished");
    assert_eq!(events[0].payload["status"], json!("failed"));
    assert_eq!(events[0].payload["contracts"], json!(1));
    assert_eq!(events[0].payload["error"]["code"], json!("internal"));
    assert!(
      events[0].payload["error"]["message"]
        .as_str()
        .is_some_and(|message| message.contains("a bug in the run")),
      "{:?}",
      events[0].payload
    );
  }

  #[test]
  fn a_run_that_finishes_keeps_the_summary_it_sent() {
    let stream = Stream::new("s-2".to_string());
    guarded(&stream, json!({ "status": "failed" }), || {
      stream.finish("verification/finished", json!({ "status": "verified" }));
    });
    let events = stream.drain(10);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].payload["status"], json!("verified"));
  }
}
