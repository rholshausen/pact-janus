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
//! **It compiles what a run would compile** (staged plan S1.12, ADR 0050): the request may declare
//! `components`, resolved exactly as `verify` resolves `target.components`, and a contract's or a
//! specification's requirements are checked against them, so a component's plan fragment is in the
//! plan it prints. And it may carry `values`, captured values it runs the plan against, as a golden
//! corpus case does, with the run's content detection and contributed actions: matching captured
//! values offline is an engine operation, not something a host does with the kernel's own API. A run's
//! executed plans still arrive on its event stream (spec §9.6's `verification/executed-plan`).

use super::scope::Scope;
use crate::component::{ContentDetection, Unavailable};
use crate::contract::{self, Contract};
use crate::error::Problem;
use crate::interaction_spec;
use crate::legacy::compile::compile_interaction;
use crate::legacy::pact as legacy_pact;
use crate::plan::{self, Assignment, CapturedValues, Executed, Plan, RuntimeValue};
use serde::Deserialize;
use serde_json::{Map, Value};
use std::collections::BTreeMap;

/// `verification/explain`'s request body.
#[derive(Debug, Deserialize)]
pub struct Explain {
  pub interaction: Subject,
  /// The project's declared components, resolved as `verify`'s `target.components` are.
  #[serde(default)]
  pub components: Vec<Value>,
  /// Captured values to run the plan against: path to value, as a golden corpus case gives them.
  #[serde(default)]
  pub values: Option<Map<String, Value>>,
  /// The representation of each value in `values`, by path: `base64` for octets.
  #[serde(default)]
  pub encoded: Option<Map<String, Value>>,
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
  NoSuchInteraction {
    index: usize,
    count: usize,
  },
  Invalid(Vec<Problem>),
  NotReadable(String),
  /// A v1–v4 interaction that needs these pact plugins (staged plan S1.8): `verify` refuses it, so
  /// there is no plan to explain.
  NeedsPlugins(Vec<String>),
  /// A component the interaction needs, or a fragment one contributes, that the scope cannot
  /// provide: `verify` would refuse it before anything ran.
  Unavailable(Unavailable),
  /// A value `encoded` tags as a representation this engine does not read, or that is not one.
  Values(String),
}

/// Compile the named subject, with the scope's components, as a run would.
pub fn compile(subject: &Subject, scope: &Scope) -> Result<Plan, ExplainError> {
  match subject.kind.as_str() {
    "spec" => {
      let document = subject.spec.as_ref().ok_or(ExplainError::Missing("spec"))?;
      let spec = interaction_spec::parse(document).map_err(|err| ExplainError::Invalid(err.problems))?;
      let parts = super::session::raw_parts(document);
      checked(scope, &spec, &parts)?;
      Ok(
        scope
          .contributions()
          .compile(&spec, &parts, &Assignment::new(), None),
      )
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
        Some(format) if crate::persisted::CONTRACT_FORMAT.reads(format) => {
          janus(document, index, subject.variant.as_deref(), scope)
        }
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
fn janus(document: &Value, index: usize, variant: Option<&str>, scope: &Scope) -> Result<Plan, ExplainError> {
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
  if let Some(content_types) = &interaction.content_types {
    map.insert("content-types".to_string(), serde_json::json!(content_types));
  }
  let spec = interaction_spec::parse(&spec_document).map_err(|err| ExplainError::Invalid(err.problems))?;
  checked(scope, &spec, &interaction.parts)?;
  let contributions = scope.contributions();

  let Some(wanted) = variant else {
    return Ok(contributions.compile(&spec, &interaction.parts, &Assignment::new(), None));
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
  Ok(contributions.compile(&spec, &interaction.parts, &assignment, Some(&recorded.id)))
}

/// What `verify` checks before a run starts (component-interfaces spec §2.3): every requirement, and
/// every declared content type and the fragment its component contributes.
fn checked(
  scope: &Scope,
  spec: &interaction_spec::InteractionSpec,
  parts: &BTreeMap<String, contract::ShapePart>,
) -> Result<(), ExplainError> {
  scope
    .check(spec.requires.iter().flatten(), spec.content_types.as_ref(), parts)
    .map_err(ExplainError::Unavailable)
}

/// `plan` run against captured values, as a run's reply would be: with the scope's content
/// detection and the actions its components contribute (spec §8.3, ADR 0050).
pub fn execute(
  plan: &Plan,
  values: &Map<String, Value>,
  encoded: Option<&Map<String, Value>>,
  scope: &Scope,
) -> Result<Executed, ExplainError> {
  use base64::Engine as _;
  let mut resolver = CapturedValues::new();
  for (path, value) in values {
    let value = match encoded
      .and_then(|encoded| encoded.get(path))
      .and_then(Value::as_str)
    {
      None | Some("json") => RuntimeValue::from_json(value),
      Some("base64") => value
        .as_str()
        .and_then(|text| base64::engine::general_purpose::STANDARD.decode(text).ok())
        .map(RuntimeValue::Bytes)
        .ok_or_else(|| ExplainError::Values(format!("'{path}' is tagged base64 and is not base64")))?,
      Some(other) => {
        return Err(ExplainError::Values(format!(
          "'{path}' is tagged '{other}', which this engine does not read"
        )));
      }
    };
    resolver = resolver.capture(path.clone(), value);
  }
  let content = scope.content();
  let detection = content.as_deref().map(ContentDetection::of);
  let contributions = scope.contributions();
  Ok(plan::execute_with(
    plan,
    &resolver,
    detection
      .as_ref()
      .map(|detection| detection as &dyn plan::ContentDetector),
    Some(contributions.as_ref()),
  ))
}

/// A v1–v4 pact's interaction, through design 3.5's compiler: request and response together, which
/// is the whole interaction's verdict in one renderable plan.
fn legacy(document: &Value, index: usize) -> Result<Plan, ExplainError> {
  let pact = legacy_pact::read("explain", document).map_err(|err| match err {
    contract::ContractError::Invalid { problems } => ExplainError::Invalid(problems),
    other => ExplainError::NotReadable(other.to_string()),
  })?;
  let interactions = legacy_pact::interactions(pact.as_ref());
  let interaction = match interactions.get(index) {
    Some(legacy_pact::PactInteraction::Http(interaction)) => interaction,
    // A message's plan is the produced message's (staged plan S1.7): there is no request to show,
    // since what asks the provider for it is a hook, not a part of the interaction.
    Some(legacy_pact::PactInteraction::Message(message)) => {
      return legacy_pact::message_plan(message).map_err(|err| ExplainError::NotReadable(err.to_string()));
    }
    Some(legacy_pact::PactInteraction::NeedsPlugins(needs)) => {
      return Err(ExplainError::NeedsPlugins(needs.plugins.clone()));
    }
    None => {
      return Err(ExplainError::NoSuchInteraction {
        index,
        count: interactions.len(),
      });
    }
  };
  let request = legacy_pact::legacy_request(&interaction.request)
    .map_err(|err| ExplainError::NotReadable(err.to_string()))?;
  let response = legacy_pact::legacy_response(&interaction.response)
    .map_err(|err| ExplainError::NotReadable(err.to_string()))?;
  Ok(compile_interaction(&interaction.description, &request, &response))
}
