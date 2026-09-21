//! Findings (design 2.8 §4) and the subsumption report (§6), and [`check`] — the top-level loop
//! that turns one (consumer contract, provider shape) pair into one.
//!
//! Field order follows the schemas, for the reason [`crate::contract::model`] gives: `serde_json`
//! serializes a struct's fields in declaration order, so a canonical member order falls out of the
//! type rather than a hand-rolled serializer.

use super::compare::{RecordedExclusion, Verdict, Walk};
use super::provider_shape::ProviderShape;
use crate::contract::{Contract, Party};
use crate::error::{Problem, push_pointer_segment};
use crate::shape::{self, ShapeNode, path};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The format token this module writes (spec §9).
pub const FORMAT: &str = "janus-subsumption-report/1";

/// What a finding does to a run, computed once from the verdict and stored so that §7's policy
/// dispatch never has to re-walk the tree (spec §4.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
  /// A decided incompatibility — a `no` verdict.
  Finding,
  /// An honest "a person must look at this" — an `unknown` verdict. Never silently escalated to
  /// `Finding`, and never quietly dropped.
  Review,
  /// A `yes` verdict reported only because it carries an `excluded-by` caveat (spec §5).
  /// Policy-inert by construction: §7.1 dispatches only on `Finding` and `Review`.
  Advisory,
}

/// One side's rendering of the node being compared (spec §4.4), generated deterministically from
/// the operator and its parameters so two checkers agree on the text.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Side {
  pub summary: String,
}

impl Side {
  pub fn new(summary: impl Into<String>) -> Side {
    Side {
      summary: summary.into(),
    }
  }
}

/// One place where `admits(provider)` is not decided to be a subset of `admits(consumer)`, or a
/// passing node worth a caveat (spec §4–§5).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Finding {
  /// Shape spec §6.2's dimension-path grammar, reused rather than reinvented (spec §4.4).
  pub path: String,
  pub verdict: Verdict,
  pub severity: Severity,
  /// An open vocabulary (spec §9): a component operator that declares a comparability class may
  /// contribute a kind outside the core seven.
  pub kind: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub provider: Option<Side>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub consumer: Option<Side>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub reason: Option<String>,
  #[serde(rename = "excluded-by", default, skip_serializing_if = "Vec::is_empty")]
  pub excluded_by: Vec<Value>,
}

impl Finding {
  fn new(
    path: &str,
    verdict: Verdict,
    kind: &str,
    provider: Side,
    consumer: Side,
    reason: impl Into<String>,
  ) -> Finding {
    Finding {
      path: path.to_string(),
      verdict,
      severity: match verdict {
        Verdict::No => Severity::Finding,
        Verdict::Unknown => Severity::Review,
        Verdict::Yes => Severity::Advisory,
      },
      kind: kind.to_string(),
      provider: Some(provider),
      consumer: Some(consumer),
      reason: Some(reason.into()),
      excluded_by: Vec::new(),
    }
  }

  /// Provider enum/union wider than the consumer's (spec §4.2).
  pub(super) fn wider_values(
    path: &str,
    provider: Side,
    consumer: Side,
    reason: impl Into<String>,
  ) -> Finding {
    Finding::new(path, Verdict::No, "wider-values", provider, consumer, reason)
  }

  /// Provider type broader — `number` where the consumer tested `integer` (spec §4.2).
  pub(super) fn broader_type(
    path: &str,
    provider: Side,
    consumer: Side,
    reason: impl Into<String>,
  ) -> Finding {
    Finding::new(path, Verdict::No, "broader-type", provider, consumer, reason)
  }

  /// Provider `nullable`/optional where the consumer tested only non-null/present (spec §4.2).
  pub(super) fn weaker_presence(
    path: &str,
    provider: Side,
    consumer: Side,
    reason: impl Into<String>,
  ) -> Finding {
    Finding::new(path, Verdict::No, "weaker-presence", provider, consumer, reason)
  }

  /// Wider cardinality — `min: 0` where the consumer declared `min: 1` (spec §4.2).
  pub(super) fn wider_cardinality(
    path: &str,
    provider: Side,
    consumer: Side,
    reason: impl Into<String>,
  ) -> Finding {
    Finding::new(path, Verdict::No, "wider-cardinality", provider, consumer, reason)
  }

  /// A member the provider's object shape does not name, that the consumer's does (spec §3.2's
  /// Rule 2). There is no provider node to summarise — that is what makes the finding.
  pub(super) fn undeclared_member(path: &str, consumer: Side) -> Finding {
    Finding::new(
      path,
      Verdict::No,
      "undeclared-member",
      Side::new(super::phrases::UNCONSTRAINED),
      consumer,
      "the provider's shape does not name this member; an unnamed member is the widest possible \
       claim, not a narrow one (spec §3.2 Rule 2)",
    )
  }

  /// A comparison shape spec §8's conservative or opaque class does not decide (spec §4.2).
  pub(super) fn unreviewable(
    path: &str,
    provider: Side,
    consumer: Side,
    reason: impl Into<String>,
  ) -> Finding {
    Finding::new(path, Verdict::Unknown, "unreviewable", provider, consumer, reason)
  }

  /// A `yes` that sits over a combination the consumer's own sampler excluded (spec §5).
  pub(super) fn advisory(path: &str, provider: Side, consumer: Side, reason: impl Into<String>) -> Finding {
    Finding::new(
      path,
      Verdict::Yes,
      "excluded-combination",
      provider,
      consumer,
      reason,
    )
  }
}

/// One interaction's outcome (spec §6.1–§6.3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InteractionResult {
  pub description: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub states: Option<Vec<super::StateRef>>,
  /// Whether a provider-shape entry was found (spec §2.2). `false` means the interaction was
  /// never checked, which is a different fact from a checked-and-passing one.
  pub matched: bool,
  /// The Kleene conjunction of every part/slot root comparison (spec §6.2), or `not-published`
  /// when `matched` is false (spec §6.3) — a report-level state, not one of the walk's own three.
  pub verdict: String,
  pub findings: Vec<Finding>,
}

/// The report-level state for an interaction the provider published nothing for (spec §6.3).
pub const NOT_PUBLISHED: &str = "not-published";

/// Counts across the whole report, so a reader sees coverage without counting the list (spec
/// §6.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Summary {
  pub interactions: usize,
  pub matched: usize,
  pub findings: usize,
  pub reviews: usize,
}

/// The result of walking one (consumer, provider) pair (spec §6).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubsumptionReport {
  #[serde(rename = "$format")]
  pub format: String,
  #[serde(rename = "$schema", skip_serializing_if = "Option::is_none")]
  pub schema: Option<String>,
  pub consumer: Party,
  pub provider: Party,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub metadata: Option<Metadata>,
  pub interactions: Vec<InteractionResult>,
  pub summary: Summary,
}

/// How the report came to exist (spec §6.1's schema), outside anything a policy reads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Metadata {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub writer: Option<std::collections::BTreeMap<String, String>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub created: Option<String>,
}

/// A document the walk could not read (spec §8).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckError {
  /// A part/slot of either document does not parse as a shape, naming the path (spec §8).
  InteractionInvalid { problems: Vec<Problem> },
}

impl CheckError {
  /// The engine-error code (engine-protocol spec §10.2).
  pub fn code(&self) -> &'static str {
    match self {
      CheckError::InteractionInvalid { .. } => "interaction-invalid",
    }
  }
}

impl std::fmt::Display for CheckError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      CheckError::InteractionInvalid { problems } => {
        write!(f, "invalid interaction ({} problem(s))", problems.len())
      }
    }
  }
}

impl std::error::Error for CheckError {}

/// Walk one consumer contract against one provider shape (spec §3.1's top-level loop).
///
/// Every interaction of the *contract* appears in the report, including the ones the provider
/// published nothing for: "a provider that has published nothing produces a report that says so
/// plainly, rather than a report full of silent `yes`es a reader could mistake for coverage"
/// (spec §6.3).
pub fn check(contract: &Contract, provider_shape: &ProviderShape) -> Result<SubsumptionReport, CheckError> {
  let mut problems = Vec::new();
  let mut interactions = Vec::new();

  for (index, interaction) in contract.interactions.iter().enumerate() {
    let states: Vec<String> = interaction
      .states
      .iter()
      .flatten()
      .map(|state| state.name.clone())
      .collect();
    let recorded_states = (!states.is_empty()).then(|| {
      states
        .iter()
        .map(|name| super::StateRef { name: name.clone() })
        .collect()
    });

    let Some(published) = provider_shape.find(&interaction.description, &states) else {
      interactions.push(InteractionResult {
        description: interaction.description.clone(),
        states: recorded_states,
        matched: false,
        verdict: NOT_PUBLISHED.to_string(),
        findings: Vec::new(),
      });
      continue;
    };

    // §5's cross-reference reads the consumer's own recorded exclusions; a contract whose
    // selection report carries none contributes an empty set, and the walk then skips the
    // caveat pass entirely.
    let exclusions = RecordedExclusion::from_selection_report(&Value::Object(
      interaction
        .selection
        .report
        .iter()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect(),
    ));
    let mut walk = Walk::new(&exclusions);

    let mut verdict = Verdict::Yes;
    let mut findings = Vec::new();
    // Only the part/slot pairs present in *both* documents are compared (spec §2.1): a slot
    // present in one and absent from the other has nothing to compare against, and a verdict the
    // checker cannot support would be exactly the guess §8's `unknown` discipline forbids.
    for (part_name, consumer_part) in &interaction.parts {
      let Some(provider_part) = published.parts.get(part_name) else {
        continue;
      };
      for (slot_name, consumer_shape) in consumer_part {
        let Some(provider_shape_value) = provider_part.get(slot_name) else {
          continue;
        };
        let at = path::root(part_name, slot_name);
        let consumer_node = parse_slot(
          consumer_shape,
          &pointer(&["interactions", &index.to_string(), "parts", part_name, slot_name]),
          &mut problems,
        );
        let provider_node = parse_slot(
          provider_shape_value,
          &pointer(&["parts", part_name, slot_name]),
          &mut problems,
        );
        let (Some(consumer_node), Some(provider_node)) = (consumer_node, provider_node) else {
          continue;
        };
        let out = walk.node(&provider_node, &consumer_node, &at);
        verdict = verdict.and(out.verdict);
        findings.extend(out.findings);
      }
    }

    interactions.push(InteractionResult {
      description: interaction.description.clone(),
      states: recorded_states,
      matched: true,
      verdict: verdict.as_str().to_string(),
      findings,
    });
  }

  if !problems.is_empty() {
    return Err(CheckError::InteractionInvalid { problems });
  }

  let summary = summarise(&interactions);
  Ok(SubsumptionReport {
    format: FORMAT.to_string(),
    schema: None,
    consumer: contract.consumer.clone(),
    provider: contract.provider.clone(),
    metadata: None,
    interactions,
    summary,
  })
}

fn summarise(interactions: &[InteractionResult]) -> Summary {
  let mut summary = Summary {
    interactions: interactions.len(),
    matched: 0,
    findings: 0,
    reviews: 0,
  };
  for interaction in interactions {
    if interaction.matched {
      summary.matched += 1;
    }
    for finding in &interaction.findings {
      match finding.severity {
        Severity::Finding => summary.findings += 1,
        Severity::Review => summary.reviews += 1,
        Severity::Advisory => {}
      }
    }
  }
  summary
}

fn parse_slot(value: &Value, pointer: &str, problems: &mut Vec<Problem>) -> Option<ShapeNode> {
  match shape::parse(value, pointer) {
    Ok(node) => Some(node),
    Err(found) => {
      problems.extend(found);
      None
    }
  }
}

fn pointer(segments: &[&str]) -> String {
  let mut pointer = String::new();
  for segment in segments {
    push_pointer_segment(&mut pointer, segment);
  }
  pointer
}
