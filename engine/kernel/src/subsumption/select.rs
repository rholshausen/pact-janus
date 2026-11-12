//! Which provider-shape entry a consumer interaction is checked against (design 2.8 §2.2, ADR
//! 0025): by `description` plus state names first, then by an entry's `selector`.
//!
//! A selector is shapes over slots — for HTTP, a `method` and a `path` pattern — and an entry
//! selects an interaction when its plan admits the values of **every** variant the contract
//! recorded. That keeps the kernel out of HTTP: it never parses a path template, it runs a plan
//! against recorded examples, the one thing it already does for every interaction. Recorded
//! examples rather than the consumer's shapes, because an example is what the consumer actually
//! sent, and "is this request an instance of that operation" is a question about a request.

use super::provider_shape::{ProviderInteraction, ProviderShape};
use crate::contract::Interaction;
use crate::error::Problem;
use crate::interaction_spec;
use crate::plan::{ActionApplier, Assignment, CapturedValues, Status, compile, execute_with, outcome};
use serde_json::{Value, json};
use std::collections::BTreeMap;

/// How a consumer interaction found its provider-shape entry, or why it found none.
#[derive(Debug)]
pub enum Selection<'a> {
  /// Contract spec §4.2's identity: `description` plus state names.
  Description(&'a ProviderInteraction),
  /// One entry's selector admitted every recorded example (ADR 0025).
  Selector(&'a ProviderInteraction),
  /// More than one entry's selector did. The checker does not guess between them (shape spec
  /// §8's discipline, one level up): no check runs, and the report names the candidates.
  Ambiguous(Vec<&'a ProviderInteraction>),
  None,
}

/// The `matched-by` value a report records for each way of matching (spec §6.1).
pub const BY_DESCRIPTION: &str = "description";
pub const BY_SELECTOR: &str = "selector";

/// Find the entry `interaction` is checked against. A selector that is not a valid shape
/// document is a problem at `/interactions/<n>/selector`, reported as `interaction-invalid` with
/// everything else the walk finds (spec §8). `actions` runs a component operator a selector uses,
/// such as `http:path` (ADR 0086 decision 2).
pub fn select<'a>(
  provider_shape: &'a ProviderShape,
  interaction: &Interaction,
  states: &[String],
  actions: Option<&dyn ActionApplier>,
  problems: &mut Vec<Problem>,
) -> Selection<'a> {
  if let Some(entry) = provider_shape.find(&interaction.description, states) {
    return Selection::Description(entry);
  }

  let mut selected = Vec::new();
  for (index, entry) in provider_shape.interactions.iter().enumerate() {
    let Some(selector) = &entry.selector else {
      continue;
    };
    // An entry that names states is about those states; one that names none (every derived
    // shape — an OpenAPI document has no provider states) is about the operation alone.
    if entry.states.is_some() && entry.state_names() != states {
      continue;
    }
    if admits_every_example(selector, interaction, index, actions, problems) {
      selected.push(entry);
    }
  }
  // An entry about the interaction's own states is more specific than one about the operation in
  // any state, and is used over it (ADR 0086 decision 5). Two alike in that still tie.
  if selected.iter().any(|entry| entry.states.is_some()) {
    selected.retain(|entry| entry.states.is_some());
  }
  match selected.len() {
    0 => Selection::None,
    1 => Selection::Selector(selected[0]),
    _ => Selection::Ambiguous(selected),
  }
}

fn admits_every_example(
  selector: &BTreeMap<String, BTreeMap<String, Value>>,
  interaction: &Interaction,
  index: usize,
  actions: Option<&dyn ActionApplier>,
  problems: &mut Vec<Problem>,
) -> bool {
  let document = json!({ "description": "selector", "parts": selector });
  let spec = match interaction_spec::parse(&document) {
    Ok(spec) => spec,
    Err(err) => {
      let prefix = format!("/interactions/{index}/selector");
      problems.extend(err.problems.into_iter().map(|problem| Problem {
        pointer: format!("{prefix}{}", problem.pointer.trim_start_matches("/parts")),
        message: problem.message,
      }));
      return false;
    }
  };
  let plan = compile(&spec, &Assignment::new(), None);

  // A contract always records at least its base variant; one that recorded none has no example
  // to select by, and is selected by nothing.
  !interaction.selection.variants.is_empty()
    && interaction.selection.variants.iter().all(|variant| {
      let mut values = BTreeMap::new();
      for (part, slots) in selector {
        for slot in slots.keys() {
          if let Some(value) = variant.parts.get(part).and_then(|recorded| recorded.get(slot)) {
            values.insert(format!("$.{part}.{slot}"), value.content.clone());
          }
        }
      }
      let resolver = CapturedValues::from_json(&values);
      outcome(&execute_with(&plan, &resolver, None, actions)).0 == Status::Matched
    })
}
