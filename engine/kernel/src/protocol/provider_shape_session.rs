//! The provider-shape session (plan task 7.2, engine-protocol spec §8.5): the recorder
//! ([`crate::subsumption::Recorder`]) as an engine mode, so that a provider's own test suite can
//! record a shape in whatever language it is written in.
//!
//! It is a third session kind rather than an extension of an existing one, which is what spec §7.3
//! requires ("new session kinds arrive as new operations plus capabilities, not as changes to
//! existing ones") and what the shape of the work wants anyway: a consumer session accumulates
//! *evidence for a contract* and a verification session *replays* one, while this accumulates
//! evidence about a provider and belongs to no consumer at all (design 2.8 §2.1).
//!
//! The engine makes no call of its own and holds no opinion about how the provider was called. An
//! observation is an arrival (ADR 0086 decision 3): the transport's own parts, each slot wrapped as
//! a contract wraps one, a content slot carrying its octets. They are read as a verification reads
//! what a provider answered, through the declarations the transport for the session's kind makes
//! (component-interfaces spec §5) and the session's content registry, so the kernel still does not
//! know what a response is. An observation names the operation it answered (decision 1), and the
//! transport's outcome slots keep one operation's outcomes apart (decision 4).
//!
//! A message has no transport yet (ADR 0067), so the kind `message` is read as `produce-message`
//! hooks produce one, and an observation names the kind it is of, so one session records a
//! provider's HTTP answers and its messages into one shape (ADR 0090).

use crate::component::{ContentComponent, ExchangeParts, OutcomeSlots, Parts, TransportComponent};
use crate::contract::Party;
use crate::error::Problem;
use crate::hooks::points::{PRODUCED_PART, PRODUCED_REQUEST_PART};
use crate::interaction_spec;
use crate::plan::{ActionApplier, Assignment, CapturedValues, Status, compile, execute_with, outcome};
use crate::protocol::EngineError;
use crate::subsumption::{EntryEvidence, ProviderShape, Recorder, RecordingPolicy, Selector};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;

/// The transport kind a session records for when `create` names none.
pub const DEFAULT_KIND: &str = "http";

#[derive(Debug, Deserialize)]
pub struct Create {
  pub provider: Party,
  /// The transport kind whose parts are observed (ADR 0086 decision 3): `http` unless given, and
  /// what an observation that names none is of (ADR 0090 decision 4).
  #[serde(default)]
  pub kind: Option<String>,
  /// Overrides the recorder's own judgements (design 2.8 §2.3's provenance is fixed at
  /// `recorded` — this is not that).
  #[serde(default)]
  pub policy: Option<Policy>,
}

/// The two recording judgements, as a host may set them (`Recorder`'s module docs). Both optional:
/// a host that passes neither gets the defaults, which is what a provider's test suite normally
/// wants.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Policy {
  #[serde(default)]
  pub max_options: Option<usize>,
  #[serde(default)]
  pub min_evidence: Option<u64>,
}

impl Policy {
  pub fn resolve(&self) -> RecordingPolicy {
    let defaults = RecordingPolicy::default();
    RecordingPolicy {
      max_options: self.max_options.unwrap_or(defaults.max_options),
      min_evidence: self.min_evidence.unwrap_or(defaults.min_evidence),
    }
  }
}

#[derive(Debug, Deserialize)]
pub struct Observe {
  pub session: String,
  /// The kind whose parts these are, when not the session's (ADR 0090 decision 4).
  #[serde(default)]
  pub kind: Option<String>,
  /// What a report calls the entry: the consumer's words when the test knows them, else the
  /// provider's own name for the operation (ADR 0086 decision 1).
  pub description: String,
  /// State *names* only (design 2.8 §2.2): a provider knows its own state vocabulary, never a
  /// given consumer's parameter values.
  #[serde(default)]
  pub states: Vec<String>,
  /// The operation answered, as shapes over the slots that identify it (ADR 0086 decision 1, ADR
  /// 0090 decision 1): the recorded entry's selector.
  #[serde(default)]
  pub operation: Option<Selector>,
  /// The transport's own parts (ADR 0086 decision 3). The answering part is recorded; every part
  /// `operation` names, when given, is checked against it.
  pub parts: Parts,
}

#[derive(Debug, Deserialize)]
pub struct Finalise {
  pub session: String,
  /// Where the engine's host writes the shape, and merges it (ADR 0086 decision 7).
  #[serde(default)]
  pub write: Option<Write>,
}

#[derive(Debug, Deserialize)]
pub struct Write {
  pub path: String,
  pub evidence: String,
  pub source: String,
}

/// What a transport declares that a recording reads (component-interfaces spec §5).
#[derive(Debug, Clone)]
pub struct Declarations {
  pub exchange_parts: ExchangeParts,
  pub outcome_slots: OutcomeSlots,
  pub credential_members: crate::redaction::CredentialMembers,
}

impl Declarations {
  pub fn of(transport: &dyn TransportComponent) -> Declarations {
    Declarations {
      exchange_parts: transport.exchange_parts(),
      outcome_slots: transport.outcome_slots(),
      credential_members: transport.credential_members(),
    }
  }

  /// What `produce-message` hooks produce, for the kind `message` when no transport serves it (ADR
  /// 0090 decision 3): the `message` part answers a synchronous message's `request` (ADR 0067
  /// decision 2), and nothing tells a message's outcomes apart or is a credential.
  pub fn produced_message() -> Declarations {
    Declarations {
      exchange_parts: ExchangeParts {
        initiating: PRODUCED_REQUEST_PART.to_string(),
        answering: Some(PRODUCED_PART.to_string()),
      },
      outcome_slots: OutcomeSlots::new(),
      credential_members: crate::redaction::CredentialMembers::new(),
    }
  }
}

/// One recording session: a [`Recorder`], the kind an observation is of unless it says, and the
/// counter a host sees back from `observe`.
#[derive(Debug)]
pub struct Session {
  recorder: Recorder,
  kind: String,
  policy: RecordingPolicy,
  observations: u64,
}

impl Session {
  pub fn new(provider: Party, policy: RecordingPolicy, kind: String) -> Session {
    Session {
      recorder: Recorder::with_policy(provider.name, policy),
      kind,
      policy,
      observations: 0,
    }
  }

  /// The kind `request` is of: its own, else the session's.
  pub fn kind_of<'a>(&'a self, request: &'a Observe) -> &'a str {
    request.kind.as_deref().unwrap_or(&self.kind)
  }

  /// Record one observation, read through what its kind declares, answering the warnings it raised.
  pub fn observe(
    &mut self,
    request: &Observe,
    declarations: &Declarations,
    content: Option<&dyn ContentComponent>,
    actions: &dyn ActionApplier,
  ) -> Result<Vec<String>, EngineError> {
    let initiating = &declarations.exchange_parts.initiating;
    let answering = declarations.exchange_parts.answering.as_deref();
    // A kind whose answer is the produced message is read as a verification reads one; HTTP's
    // `request` is not a message's.
    let observed = if answering == Some(PRODUCED_PART) {
      labelled(&request.parts, content)
    } else {
      request.parts.clone()
    };

    if let Some(operation) = &request.operation {
      admits_observed(operation, &observed, initiating, content, actions)?;
    }
    self.recorder.add_credentials(&declarations.credential_members);

    let mut warnings = Vec::new();
    let mut parts = BTreeMap::new();
    for (part, slots) in &observed {
      // A provider shape carries what the provider produces (design 2.8 §2.1): the answering part,
      // or for a kind with none, everything but what started the exchange.
      let produced = match answering {
        Some(answering) => part == answering,
        None => part != initiating,
      };
      if !produced {
        continue;
      }
      let mut values = BTreeMap::new();
      for (slot, value) in slots {
        let (document, warning) = super::wire::recorded_slot(value, content);
        warnings.extend(warning.map(|warning| format!("{part}.{slot}: {warning}")));
        values.insert(slot.clone(), document.to_json());
      }
      parts.insert(part.clone(), values);
    }

    // The outcome goes into the selector, so a consumer interaction that recorded a 200 selects the
    // entry the 200s were recorded in (ADR 0086 decision 4). Only for an observation that names its
    // operation: one named by the consumer's words is about one interaction already.
    let selector = request.operation.as_ref().map(|operation| {
      let mut selector = operation.clone();
      for (part, slots) in &declarations.outcome_slots {
        for slot in slots {
          if let Some(value) = parts.get(part).and_then(|values| values.get(slot)) {
            selector
              .entry(part.clone())
              .or_default()
              .insert(slot.clone(), json!({ "shape": "equality", "example": value }));
          }
        }
      }
      selector
    });

    self
      .recorder
      .observe_operation(&request.description, &request.states, selector.as_ref(), &parts);
    self.observations += 1;
    Ok(warnings)
  }

  /// What this session has accumulated so far, for the `observe` result: enough for a host to
  /// report "recorded 24 responses across 6 operations" without holding its own counters.
  pub fn progress(&self) -> (u64, usize) {
    (self.observations, self.recorder.interaction_count())
  }

  pub fn provider(&self) -> &str {
    self.recorder.provider()
  }

  pub fn policy(&self) -> RecordingPolicy {
    self.policy
  }

  pub fn evidence(&self) -> Vec<EntryEvidence> {
    self.recorder.evidence()
  }

  pub fn finish(self) -> ProviderShape {
    self.recorder.finish()
  }
}

/// The observed parts, a produced message's unlabelled body labelled with the type a verification
/// reads it as (ADR 0090 decision 3): its metadata's `contentType`, else what its octets are
/// detected as. So a recording decodes the body a verification would, and a selector over one of
/// its members is matched against the document.
fn labelled(parts: &Parts, content: Option<&dyn ContentComponent>) -> Parts {
  let mut parts = parts.clone();
  for (name, part) in parts.iter_mut() {
    if name != PRODUCED_PART && name != PRODUCED_REQUEST_PART {
      continue;
    }
    let unlabelled = part.get("body").is_some_and(|body| body.content_type.is_none());
    if !unlabelled {
      continue;
    }
    if let Some(content_type) = super::wire::produced_message_type(part, content)
      && let Some(body) = part.get_mut("body")
    {
      body.content_type = Some(content_type);
    }
  }
  parts
}

/// Refuse an observation that its own operation does not admit: a test that named the wrong
/// template would otherwise record an entry no consumer interaction ever selects. Every part the
/// operation names and the observation carries is checked: an HTTP request, a synchronous
/// message's request, or an asynchronous message itself (ADR 0090 decision 1).
fn admits_observed(
  operation: &Selector,
  parts: &Parts,
  initiating: &str,
  content: Option<&dyn ContentComponent>,
  actions: &dyn ActionApplier,
) -> Result<(), EngineError> {
  let document = json!({ "description": "operation", "parts": operation });
  let spec = interaction_spec::parse(&document).map_err(|err| {
    let problems: Vec<Problem> = err
      .problems
      .into_iter()
      .map(|problem| Problem {
        pointer: format!("/operation{}", problem.pointer.trim_start_matches("/parts")),
        message: problem.message,
      })
      .collect();
    EngineError::interaction_invalid(&problems)
  })?;
  let mut resolver = CapturedValues::new();
  let mut named = Vec::new();
  for part in operation.keys() {
    let Some(observed) = parts.get(part) else {
      continue;
    };
    named.push(if part == initiating {
      format!("the {part} the provider answered")
    } else {
      format!("the {part} the provider produced")
    });
    for (slot, value) in observed {
      let document = super::wire::decode_slot(value, None, content);
      resolver = resolver.capture(format!("$.{part}.{slot}"), document);
    }
  }
  if named.is_empty() {
    return Ok(());
  }
  let plan = compile(&spec, &Assignment::new(), None);
  let executed = execute_with(&plan, &resolver, None, Some(actions));
  let (status, mismatches) = outcome(&executed);
  if status == Status::Matched {
    return Ok(());
  }
  let problems: Vec<Problem> = mismatches
    .iter()
    .map(|mismatch| Problem {
      pointer: "/operation".to_string(),
      message: format!(
        "the operation does not admit {}: {}",
        named.join(" and "),
        mismatch.message
      ),
    })
    .collect();
  Err(EngineError::interaction_invalid(&problems))
}

/// An evidence file (ADR 0086 decision 7): each source's evidence, keyed by source, for one
/// provider. Written beside nothing a team publishes, and read back by the next session to finish.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Evidence {
  #[serde(rename = "$format")]
  pub format: String,
  pub provider: Party,
  pub sources: BTreeMap<String, Vec<EntryEvidence>>,
}

impl Evidence {
  pub fn new(provider: &str) -> Evidence {
    Evidence {
      format: crate::persisted::PROVIDER_SHAPE_EVIDENCE_FORMAT
        .writes()
        .to_string(),
      provider: Party {
        name: provider.to_string(),
      },
      sources: BTreeMap::new(),
    }
  }

  /// Read what an evidence file holds: nothing, for one that is new or empty.
  pub fn read(path: &str, bytes: &[u8], provider: &str) -> Result<Evidence, EngineError> {
    if bytes.iter().all(u8::is_ascii_whitespace) {
      return Ok(Evidence::new(provider));
    }
    let unreadable = |message: String| {
      let mut error = EngineError::document_invalid(&[Problem {
        pointer: String::new(),
        message,
      }]);
      error.message = format!("the file at '{path}' holds no provider shape evidence this engine can read");
      error.details = Some(Box::new(json!({ "path": path, "problems": [] })));
      error
    };
    let value: Value = serde_json::from_slice(bytes).map_err(|err| unreadable(err.to_string()))?;
    let format = value.get("$format").and_then(Value::as_str).unwrap_or_default();
    if !crate::persisted::PROVIDER_SHAPE_EVIDENCE_FORMAT.reads(format) {
      return Err(unreadable(format!(
        "'$format' is '{format}', not provider shape evidence"
      )));
    }
    let evidence: Evidence = serde_json::from_value(value).map_err(|err| unreadable(err.to_string()))?;
    if evidence.provider.name != provider {
      let mut error = EngineError::document_invalid(&[]);
      error.code = "contract-conflict".to_string();
      error.message = format!(
        "the file at '{path}' holds evidence about '{}', not '{provider}'",
        evidence.provider.name
      );
      error.details = Some(Box::new(
        json!({ "path": path, "provider": evidence.provider.name }),
      ));
      return Err(error);
    }
    Ok(evidence)
  }

  /// This source's evidence in place of whatever it recorded before, and the shape all of it adds
  /// up to.
  pub fn replace(
    &mut self,
    source: &str,
    entries: Vec<EntryEvidence>,
    policy: RecordingPolicy,
  ) -> ProviderShape {
    self.sources.insert(source.to_string(), entries);
    let mut recorder = Recorder::with_policy(self.provider.name.clone(), policy);
    for entries in self.sources.values() {
      recorder.absorb(entries);
    }
    recorder.finish()
  }
}
