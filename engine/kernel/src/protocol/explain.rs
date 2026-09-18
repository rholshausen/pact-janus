//! `verification/explain` (engine-protocol spec §8.3, plan task 5.5): compiling *one* interaction
//! to a plan, from whichever of the three documents a plan is ever compiled from.
//!
//! **It is a kernel operation precisely so no SDK builds its own** (the RFC). The point of
//! `explain` is that a user can see what the engine will actually do, and a renderer that
//! reconstructed the plan from a DSL — or from a second, "display" compiler — would be showing
//! them something else. Every subject here therefore goes through the same compiler the matching
//! path uses: design 3.3's for shapes, design 3.5's for v1–v4 matching rules.
//!
//! The three subjects are not three formats this module invented. They are the three shapes a plan
//! can come from: a specification the consumer is *about* to run (`spec`), a contract's recorded
//! interaction, and a v1–v4 pact's interaction (both `contract-interaction`, told apart the way
//! ADR 0011 says to — by asking the document what it is, never by guessing at its structure).
//!
//! Explaining an *executed* plan is deliberately not here: an execution belongs to a run, so it
//! arrives on that run's event stream ([`super::events`], spec §9.6's `verification/executed-plan`)
//! rather than being re-derived from a values file the caller had to write by hand.

use crate::contract::{self, Contract};
use crate::error::Problem;
use crate::interaction_spec;
use crate::legacy_pact;
use crate::plan::{self, Assignment, Plan};
use serde::Deserialize;
use serde_json::Value;

/// `verification/explain`'s request body.
#[derive(Debug, Deserialize)]
pub struct Explain {
  pub interaction: Subject,
  #[serde(default)]
  pub options: Option<Value>,
}

/// What to compile. An open vocabulary like every other `kind` in the protocol (spec §2.2): a
/// value this engine does not know is refused by name, never guessed at.
#[derive(Debug, Deserialize)]
pub struct Subject {
  pub kind: String,
  /// For `spec`: the interaction specification.
  #[serde(default)]
  pub spec: Option<Value>,
  /// For `contract-interaction`: the contract document — a Janus contract or a v1–v4 pact.
  #[serde(default)]
  pub contract: Option<Value>,
  /// For `contract-interaction`: which interaction. Defaults to the first.
  #[serde(default)]
  pub index: Option<usize>,
  /// For `contract-interaction` on a Janus contract: which recorded variant to pin the plan to
  /// (shape spec §7.1). Absent means the unpinned plan — what the interaction admits in general,
  /// rather than what one variant demonstrated.
  #[serde(default)]
  pub variant: Option<String>,
}

#[derive(Debug)]
pub enum ExplainError {
  KindUnsupported(String),
  Missing(&'static str),
  NoSuchInteraction { index: usize, count: usize },
  Invalid(Vec<Problem>),
  NotReadable(String),
}

/// Compile the named subject.
pub fn compile(subject: &Subject) -> Result<Plan, ExplainError> {
  match subject.kind.as_str() {
    "spec" => {
      let document = subject.spec.as_ref().ok_or(ExplainError::Missing("spec"))?;
      let spec = interaction_spec::parse(document).map_err(|err| ExplainError::Invalid(err.problems))?;
      Ok(plan::compile(&spec, &Assignment::new(), None))
    }
    "contract-interaction" => {
      let document = subject
        .contract
        .as_ref()
        .ok_or(ExplainError::Missing("contract"))?;
      let index = subject.index.unwrap_or(0);
      // ADR 0011: the document says what it is. A pact is never mistaken for a contract whose
      // shapes happen to be missing, and a contract is never fed to the v1–v4 compiler.
      match document.get("$format").and_then(Value::as_str) {
        Some(format) if format == contract::FORMAT => janus(document, index, subject.variant.as_deref()),
        Some(other) => Err(ExplainError::NotReadable(format!(
          "'{other}' is not a format this engine reads (expected '{}')",
          contract::FORMAT
        ))),
        None => legacy(document, index),
      }
    }
    other => Err(ExplainError::KindUnsupported(other.to_string())),
  }
}

/// A Janus contract's interaction. A contract interaction *is* an interaction specification plus
/// evidence, so this rebuilds the specification and compiles it — the same move the verifier makes
/// (`super::verification::interaction_spec`), which is what makes an explained plan the plan a run
/// would actually execute rather than a lookalike.
fn janus(document: &Value, index: usize, variant: Option<&str>) -> Result<Plan, ExplainError> {
  let contract: Contract =
    serde_json::from_value(document.clone()).map_err(|err| ExplainError::NotReadable(err.to_string()))?;
  let interaction = contract
    .interactions
    .get(index)
    .ok_or(ExplainError::NoSuchInteraction {
      index,
      count: contract.interactions.len(),
    })?;

  let mut spec_document = serde_json::json!({
    "description": interaction.description,
    "parts": interaction.parts,
  });
  let map = spec_document.as_object_mut().expect("a json! object literal");
  if let Some(transport) = &interaction.transport {
    map.insert("transport".to_string(), serde_json::json!(transport));
  }
  if let Some(states) = &interaction.states {
    map.insert("states".to_string(), serde_json::json!(states));
  }
  if let Some(requires) = &interaction.requires {
    map.insert("requires".to_string(), serde_json::json!(requires));
  }
  let spec = interaction_spec::parse(&spec_document).map_err(|err| ExplainError::Invalid(err.problems))?;

  let Some(wanted) = variant else {
    return Ok(plan::compile(&spec, &Assignment::new(), None));
  };
  let recorded = interaction
    .selection
    .variants
    .iter()
    .find(|v| v.id == wanted)
    .ok_or_else(|| ExplainError::NotReadable(format!("the interaction records no variant '{wanted}'")))?;
  let mut assignment = Assignment::new();
  for entry in &recorded.assignment {
    if let (Some(dimension), Some(point)) = (
      entry.get("dimension").and_then(Value::as_str),
      entry.get("point").and_then(Value::as_str),
    ) {
      assignment.insert(dimension.to_string(), point.to_string());
    }
  }
  Ok(plan::compile(&spec, &assignment, Some(&recorded.id)))
}

/// A v1–v4 pact's interaction, through design 3.5's compiler: request and response together, which
/// is the whole interaction's verdict in one renderable plan.
fn legacy(document: &Value, index: usize) -> Result<Plan, ExplainError> {
  let pact = legacy_pact::read("explain", document).map_err(|err| match err {
    contract::ContractError::Invalid { problems } => ExplainError::Invalid(problems),
    other => ExplainError::NotReadable(other.to_string()),
  })?;
  let interactions = legacy_pact::http_interactions(pact.as_ref());
  let interaction = interactions.get(index).ok_or(ExplainError::NoSuchInteraction {
    index,
    count: interactions.len(),
  })?;
  let request = legacy_pact::legacy_request(&interaction.request)
    .map_err(|err| ExplainError::NotReadable(err.to_string()))?;
  let response = legacy_pact::legacy_response(&interaction.response)
    .map_err(|err| ExplainError::NotReadable(err.to_string()))?;
  Ok(plan::compile_legacy_interaction(
    &interaction.description,
    &request,
    &response,
  ))
}
