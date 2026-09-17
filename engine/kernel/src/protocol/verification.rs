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
//! What this task does not do, and 5.2 picks up: resolving `variant-params` provider-state
//! bindings (`whenVariant`) and the `state-unavailable` status that goes with them, hooks (5.3),
//! and v1–v4 pacts as a source (5.4 — a non-Janus document is refused by name here, never
//! misparsed, per ADR 0011).

use super::events::Stream;
use super::wire::{find_container, mismatch_json, parts_resolver};
use crate::component::{ContentComponent, Parts, Send as SendRequest, Stop, TransportComponent};
use crate::contract::{self, Contract};
use crate::error::Problem;
use crate::interaction_spec::{self, InteractionSpec};
use crate::plan::{self, Assignment, Status, execute, outcome};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::thread::{self, JoinHandle};

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

/// Read the inline contracts, identifying each before parsing it (contract-file spec §2.3): a v4
/// pact, a hand-written JSON blob or a truncated file becomes a named refusal, never a
/// half-verified run. v1–v4 pacts get their own path in plan task 5.4; until then the refusal says
/// which format was found so the message is actionable.
pub(crate) fn read_source(source: &ContractSource) -> Result<Vec<Contract>, VerifyError> {
  if source.kind != "inline" {
    return Err(VerifyError::SourceUnsupported(source.kind.clone()));
  }
  let mut contracts = Vec::with_capacity(source.contracts.len());
  for (index, doc) in source.contracts.iter().enumerate() {
    let format = doc.get("$format").and_then(Value::as_str);
    match format {
      Some(format) if format == contract::FORMAT => {}
      other => {
        return Err(VerifyError::NotAContract {
          index,
          found: other.map(str::to_string).or_else(|| legacy_hint(doc)),
        });
      }
    }
    let parsed: Contract = serde_path_to_error::deserialize(doc).map_err(|err| {
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
    })?;
    contracts.push(parsed);
  }
  Ok(contracts)
}

/// What a non-Janus document looks like, for the refusal's message: a pact file says so in its own
/// metadata, and naming "pactSpecification 3.0.0" is far more useful than "no `$format`".
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
/// `options` is accepted and carried no further yet: variant filtering and "a filtered run is
/// reported as filtered" are variant-semantics spec §5.1's, which plan task 5.2 implements. An
/// option a run ignores is logged rather than silently dropped, so a host is never left believing
/// it narrowed a run that in fact replayed everything.
pub(crate) fn start(
  contracts: Vec<Contract>,
  targets: Vec<Target>,
  content: Option<Arc<dyn ContentComponent>>,
  options: Option<Value>,
  stream: Arc<Stream>,
) -> Run {
  if let Some(options) = &options {
    tracing::warn!(options = %options, "verification options are not honoured yet (plan task 5.2)");
  }
  let thread_stream = Arc::clone(&stream);
  let thread = thread::spawn(move || run(contracts, targets, content, thread_stream));
  Run { stream, thread }
}

fn run(
  contracts: Vec<Contract>,
  targets: Vec<Target>,
  content: Option<Arc<dyn ContentComponent>>,
  stream: Arc<Stream>,
) {
  let interactions: usize = contracts.iter().map(|c| c.interactions.len()).sum();
  let variants: usize = contracts
    .iter()
    .flat_map(|c| &c.interactions)
    .map(|i| i.selection.variants.len())
    .sum();
  stream.emit(
    "verification/started",
    json!({
      "contracts": contracts.len(),
      "interactions": interactions,
      "variants": variants,
      "providers": contracts.iter().map(|c| c.provider.name.clone()).collect::<Vec<_>>(),
      "transports": targets.iter().map(|t| t.kind.clone()).collect::<Vec<_>>(),
    }),
  );

  let mut tally = Tally::default();
  for contract in &contracts {
    for (index, interaction) in contract.interactions.iter().enumerate() {
      verify_interaction(
        contract,
        index,
        interaction,
        &targets,
        content.as_deref(),
        &stream,
        &mut tally,
      );
    }
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
    tally.summary(contracts.len(), interactions),
  );
}

/// Per-run counters and the failure list the summary carries, so a host that only reads the
/// terminal event still knows what failed and where (the CLI, plan task 5.5, reads both).
#[derive(Default)]
struct Tally {
  verified: usize,
  failed: usize,
  failures: Vec<Value>,
}

impl Tally {
  fn record(&mut self, verified: bool, failure: Option<Value>) {
    if verified {
      self.verified += 1;
    } else {
      self.failed += 1;
      if let Some(failure) = failure {
        self.failures.push(failure);
      }
    }
  }

  fn summary(&self, contracts: usize, interactions: usize) -> Value {
    json!({
      "status": if self.failed == 0 { "verified" } else { "failed" },
      "contracts": contracts,
      "interactions": interactions,
      "variants": {
        "total": self.verified + self.failed,
        "verified": self.verified,
        "failed": self.failed,
      },
      "failures": self.failures,
    })
  }
}

fn verify_interaction(
  contract: &Contract,
  index: usize,
  interaction: &contract::Interaction,
  targets: &[Target],
  content: Option<&dyn ContentComponent>,
  stream: &Stream,
  tally: &mut Tally,
) {
  let reference = interaction_ref(contract, index, interaction);

  // A shape the engine cannot parse is the contract's problem, not the provider's, and it is
  // reported against every variant rather than as one interaction-level event: the unit of a
  // verification result is the interaction × variant (spec §9.6), and collapsing that here would
  // make a summary's variant counts stop adding up.
  let spec = match interaction_spec(interaction) {
    Ok(spec) => spec,
    Err(problems) => {
      for variant in &interaction.selection.variants {
        let payload = json!({
          "interaction": reference, "variant": variant.id, "status": "failed",
          "error": { "code": "contract-invalid", "problems": problems },
        });
        stream.emit("verification/interaction-result", payload.clone());
        tally.record(false, Some(payload));
      }
      return;
    }
  };

  let kind = interaction
    .transport
    .as_ref()
    .map(|t| t.kind.as_str())
    .unwrap_or(DEFAULT_TRANSPORT_KIND);
  let Some(target) = targets.iter().find(|target| target.kind == kind) else {
    // Contract-file spec §4.3: an unknown kind is `component-unavailable` naming it, never a
    // silent skip — a verifier that cannot speak an interaction's transport has not verified it.
    for variant in &interaction.selection.variants {
      let payload = json!({
        "interaction": reference, "variant": variant.id, "status": "failed",
        "error": { "code": "component-unavailable", "component": format!("transport/{kind}") },
      });
      stream.emit("verification/interaction-result", payload.clone());
      tally.record(false, Some(payload));
    }
    return;
  };

  for variant in &interaction.selection.variants {
    stream.emit(
      "verification/interaction-started",
      json!({ "interaction": reference, "variant": variant.id, "origin": variant.origin }),
    );
    let payload = verify_variant(&spec, &reference, variant, target, content);
    let verified = payload["status"] == "verified";
    stream.emit("verification/interaction-result", payload.clone());
    tally.record(verified, (!verified).then_some(payload));
  }
}

/// One variant: replay the recorded request, match the reply against the shape pinned to that
/// variant. Returns the `verification/interaction-result` payload either way — a provider that
/// cannot be reached is a failed variant with the component's own error attached, not a hole in
/// the run.
fn verify_variant(
  spec: &InteractionSpec,
  reference: &Value,
  variant: &contract::RecordedVariant,
  target: &Target,
  content: Option<&dyn ContentComponent>,
) -> Value {
  let assignment = assignment_of(variant);
  // Pinned to this variant (shape spec §7.1, variant-semantics spec §5.2 step 3): an `optional`
  // pinned to `absent` admits only absence, an `any-of` pinned to `SHIPPED` admits only that.
  // Matching against the unpinned shape would accept `PENDING` where the consumer demonstrated
  // `SHIPPED`, and the variant would have proved nothing.
  let plan = plan::compile(spec, &assignment, Some(&variant.id));

  let request = match recorded_request(variant) {
    Some(parts) => parts,
    None => {
      return json!({
        "interaction": reference, "variant": variant.id, "status": "failed",
        "error": { "code": "contract-invalid", "problems": [{
          "pointer": "/parts/request",
          "message": "the recorded variant has no request part to replay",
        }] },
      });
    }
  };

  let sent = target.component.send(SendRequest {
    instance: target.instance.clone(),
    parts: request,
    await_reply: true,
    timeout_ms: Some(SEND_TIMEOUT_MS),
  });
  let reply = match sent {
    Ok(result) => result.reply,
    Err(error) => {
      tracing::warn!(variant = %variant.id, ?error, "the provider could not be driven for this variant");
      return json!({
        "interaction": reference, "variant": variant.id, "status": "failed",
        "error": { "code": "component-failed", "component": format!("transport/{}", target.kind), "error": error },
      });
    }
  };
  let Some(reply) = reply else {
    // Spec §5.2: "no reply expected" and "reply expected but absent" are different observations,
    // and this is the second one — a transport that answered a driven `send` with no reply cannot
    // be matched against a response shape.
    return json!({
      "interaction": reference, "variant": variant.id, "status": "failed",
      "error": { "code": "component-failed", "component": format!("transport/{}", target.kind),
                 "error": { "code": "no-reply", "message": "the transport returned no reply to match" } },
    });
  };

  let resolver = parts_resolver(&reply, content);
  let executed = execute(&plan, &resolver);
  // Only the response half is scored. The request was replayed verbatim, so matching it against
  // itself would assert nothing; the provider answers for the response (variant-semantics §5.2).
  let response = find_container(&executed, "response").unwrap_or(&executed);
  let (status, mismatches) = outcome(response);

  match status {
    Status::Matched => {
      tracing::debug!(variant = %variant.id, "variant verified");
      json!({ "interaction": reference, "variant": variant.id, "status": "verified" })
    }
    Status::Mismatched => {
      tracing::warn!(variant = %variant.id, count = mismatches.len(), "variant failed");
      json!({
        "interaction": reference, "variant": variant.id, "status": "failed",
        "mismatches": mismatches.iter().map(mismatch_json).collect::<Vec<_>>(),
      })
    }
  }
}

/// How an interaction is named in every event of the run: the contract it came from, its index,
/// and its identity (description plus states — contract-file spec §4.2). Enough for a host to
/// point a user at one line of one file without the engine assuming anything about file layout.
fn interaction_ref(contract: &Contract, index: usize, interaction: &contract::Interaction) -> Value {
  json!({
    "contract": { "consumer": contract.consumer.name, "provider": contract.provider.name },
    "index": index,
    "description": interaction.description,
    "states": interaction.states.as_ref().map(|states| {
      states.iter().map(|state| state.name.clone()).collect::<Vec<_>>()
    }),
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
