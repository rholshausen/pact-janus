//! Which provider-shape entry a consumer interaction is checked against (design 2.8 §2.2, ADR
//! 0025): by `description` plus state names first, then by an entry's `selector`.
//!
//! A selector is shapes over slots — for HTTP, a `method` and a `path` pattern — and an entry
//! selects an interaction when its plan admits the values of **every** variant the contract
//! recorded. That keeps the kernel out of HTTP: it never parses a path template, it runs a plan
//! against recorded examples, the one thing it already does for every interaction. Recorded
//! examples rather than the consumer's shapes, because an example is what the consumer actually
//! sent, and "is this request an instance of that operation" is a question about a request.
//!
//! Running a plan costs microseconds, and an interaction would run every entry's (ADR 0096). So each
//! entry also has constraints its selector fixes, an `equality`'s value and a prefix a component
//! operator declares, which rule an entry out with a comparison. A constraint rules out only what
//! the plan would certainly refuse, so the plans still decide which entry is selected.

use super::provider_shape::{ProviderInteraction, ProviderShape};
use crate::contract::Interaction;
use crate::error::Problem;
use crate::interaction_spec;
use crate::plan::{
  ActionApplier, Assignment, CapturedValues, Plan, RuntimeValue, Status, compile, execute_with, outcome,
};
use serde_json::{Value, json};
use std::cell::{OnceCell, RefCell};
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;

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

/// One entry's selector, compiled: its plan, and the constraints it fixes.
struct Compiled {
  plan: Plan,
  constraints: Vec<Constraint>,
}

/// A value a selector fixes at a slot, or at a member under it: what a recorded value must be for
/// the plan to have any chance of admitting it.
struct Constraint {
  part: String,
  slot: String,
  members: Vec<String>,
  test: Test,
}

enum Test {
  /// An `equality`'s scalar example.
  Equal(Value),
  /// Text every admitted string begins with, as the operator's component declares (ADR 0096).
  Prefix(String),
}

impl Test {
  /// Whether the plan would certainly refuse `value`. Anything this cannot tell is left to the plan.
  fn refuses(&self, value: &RuntimeValue) -> bool {
    match (self, value) {
      (Test::Equal(Value::String(expected)), RuntimeValue::String(actual)) => expected != actual,
      (Test::Equal(Value::Bool(expected)), RuntimeValue::Bool(actual)) => expected != actual,
      // Equality holds 100 and 100.0 to be one value (ADR 0072), so numbers are compared as numbers.
      (Test::Equal(Value::Number(expected)), RuntimeValue::Number(actual)) => {
        match (expected.as_f64(), actual.as_f64()) {
          (Some(expected), Some(actual)) => expected != actual,
          _ => false,
        }
      }
      (Test::Prefix(prefix), RuntimeValue::String(actual)) => !actual.starts_with(prefix.as_str()),
      _ => false,
    }
  }
}

/// The constraints `node` fixes at `members` under a slot: an `equality`'s scalar example, an
/// object's members' own, and the prefix a component operator declares. Nothing for any other shape.
fn constraints(
  node: &Value,
  members: &[String],
  components: Option<&dyn super::OperatorComparer>,
  found: &mut Vec<(Vec<String>, Test)>,
) {
  let Some(shape) = node.get("shape").and_then(Value::as_str) else {
    return;
  };
  match shape {
    "equality" => {
      if let Some(example @ (Value::String(_) | Value::Bool(_) | Value::Number(_))) = node.get("example") {
        found.push((members.to_vec(), Test::Equal(example.clone())));
      }
    }
    "object" => {
      for (name, member) in node
        .get("members")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
      {
        let mut path = members.to_vec();
        path.push(name.clone());
        constraints(member, &path, components, found);
      }
    }
    operator if operator.contains(':') => {
      if let Some(prefix) = components
        .and_then(|components| components.prefix(operator, node))
        .filter(|prefix| !prefix.is_empty())
      {
        found.push((members.to_vec(), Test::Prefix(prefix)));
      }
    }
    _ => {}
  }
}

/// Each entry's selector, compiled the first time an interaction needs it and kept for the rest of
/// the check (staged plan S4.5). Without it, every interaction compiled every entry's selector again.
pub struct Selectors {
  compiled: Vec<OnceCell<Option<Compiled>>>,
}

impl Selectors {
  /// None compiled yet, for `provider_shape`'s entries.
  pub fn new(provider_shape: &ProviderShape) -> Selectors {
    Selectors {
      compiled: provider_shape
        .interactions
        .iter()
        .map(|_| OnceCell::new())
        .collect(),
    }
  }

  /// Entry `index`'s selector compiled, or `None` when it is not a valid shape document, which is a
  /// problem at `/interactions/<index>/selector`, reported the first time it is read.
  fn compiled(
    &self,
    index: usize,
    selector: &BTreeMap<String, BTreeMap<String, Value>>,
    components: Option<&dyn super::OperatorComparer>,
    problems: &mut Vec<Problem>,
  ) -> Option<&Compiled> {
    self.compiled[index]
      .get_or_init(|| {
        let document = json!({ "description": "selector", "parts": selector });
        match interaction_spec::parse(&document) {
          Ok(spec) => {
            let mut found = Vec::new();
            let mut constraints_of = Vec::new();
            for (part, slots) in selector {
              for (slot, node) in slots {
                found.clear();
                constraints(node, &[], components, &mut found);
                constraints_of.extend(found.drain(..).map(|(members, test)| Constraint {
                  part: part.clone(),
                  slot: slot.clone(),
                  members,
                  test,
                }));
              }
            }
            Some(Compiled {
              plan: compile(&spec, &Assignment::new(), None),
              constraints: constraints_of,
            })
          }
          Err(err) => {
            let prefix = format!("/interactions/{index}/selector");
            problems.extend(err.problems.into_iter().map(|problem| Problem {
              pointer: format!("{prefix}{}", problem.pointer.trim_start_matches("/parts")),
              message: problem.message,
            }));
            None
          }
        }
      })
      .as_ref()
  }
}

/// What one interaction recorded at a slot, in each variant, decoded once however many entries read
/// it. A content slot is the document its declared type decodes it to, as a verification reads it,
/// so a selector may name a member of a message's body (ADR 0090 decision 2).
struct Recorded<'i> {
  interaction: &'i Interaction,
  content: Option<&'i dyn crate::component::ContentComponent>,
  slots: RefCell<HashMap<(String, String), Variants>>,
}

/// A slot's value in each variant, `None` where a variant recorded none.
type Variants = Rc<Vec<Option<RuntimeValue>>>;

impl<'i> Recorded<'i> {
  fn slot(&self, part: &str, slot: &str) -> Variants {
    let key = (part.to_string(), slot.to_string());
    if let Some(values) = self.slots.borrow().get(&key) {
      return Rc::clone(values);
    }
    let declared = crate::common::declared_type(self.interaction.content_types.as_ref(), part, slot);
    let values: Variants = Rc::new(
      self
        .interaction
        .selection
        .variants
        .iter()
        .map(|variant| {
          variant
            .parts
            .get(part)
            .and_then(|recorded| recorded.get(slot))
            .map(|value| crate::protocol::wire::decode_slot(value, declared, self.content))
        })
        .collect(),
    );
    self.slots.borrow_mut().insert(key, Rc::clone(&values));
    values
  }
}

/// The member at `members` under `value`, if it is there.
fn member<'v>(value: &'v RuntimeValue, members: &[String]) -> Option<&'v RuntimeValue> {
  members.iter().try_fold(value, |value, name| match value {
    RuntimeValue::Object(members) => members.get(name),
    _ => None,
  })
}

/// Find the entry `interaction` is checked against. A selector that is not a valid shape
/// document is a problem at `/interactions/<n>/selector`, reported as `interaction-invalid` with
/// everything else the walk finds (spec §8). `actions` runs a component operator a selector uses,
/// such as `http:path` (ADR 0086 decision 2). `selectors` holds the selectors compiled so far.
pub fn select<'a>(
  provider_shape: &'a ProviderShape,
  selectors: &Selectors,
  interaction: &Interaction,
  states: &[String],
  components: Option<&dyn super::OperatorComparer>,
  problems: &mut Vec<Problem>,
) -> Selection<'a> {
  if let Some(entry) = provider_shape.find(&interaction.description, states) {
    return Selection::Description(entry);
  }
  // A contract always records at least its base variant; one that recorded none has no example to
  // select by, and is selected by nothing.
  let recorded = Recorded {
    interaction,
    content: components.and_then(|components| components.content()),
    slots: RefCell::new(HashMap::new()),
  };

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
    let Some(compiled) = selectors.compiled(index, selector, components, problems) else {
      continue;
    };
    if admits_every_example(compiled, selector, &recorded, components) {
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
  compiled: &Compiled,
  selector: &BTreeMap<String, BTreeMap<String, Value>>,
  recorded: &Recorded<'_>,
  components: Option<&dyn super::OperatorComparer>,
) -> bool {
  let variants = recorded.interaction.selection.variants.len();
  if variants == 0 {
    return false;
  }
  // What the selector fixes rules the entry out before its plan runs.
  for constraint in &compiled.constraints {
    let values = recorded.slot(&constraint.part, &constraint.slot);
    let refused = values
      .iter()
      .flatten()
      .any(|value| member(value, &constraint.members).is_some_and(|value| constraint.test.refuses(value)));
    if refused {
      return false;
    }
  }

  let actions = components.map(|components| components as &dyn ActionApplier);
  (0..variants).all(|variant| {
    let mut resolver = CapturedValues::new();
    for (part, slots) in selector {
      for slot in slots.keys() {
        if let Some(value) = &recorded.slot(part, slot)[variant] {
          resolver = resolver.capture(format!("$.{part}.{slot}"), value.clone());
        }
      }
    }
    outcome(&execute_with(&compiled.plan, &resolver, None, actions)).0 == Status::Matched
  })
}

#[cfg(test)]
mod tests {
  use super::{RuntimeValue, Test};
  use serde_json::json;

  fn value(json: serde_json::Value) -> RuntimeValue {
    RuntimeValue::from_json(&json)
  }

  /// A constraint rules an entry out only where the plan would certainly refuse the value.
  #[test]
  fn a_constraint_refuses_only_what_the_plan_would() {
    let equal = Test::Equal(json!("GET"));
    assert!(equal.refuses(&value(json!("POST"))));
    assert!(!equal.refuses(&value(json!("GET"))));
    // Not a string: left to the plan.
    assert!(!equal.refuses(&value(json!(1))));

    // Equality holds 100 and 100.0 to be one value (ADR 0072).
    let status = Test::Equal(json!(200));
    assert!(!status.refuses(&value(json!(200.0))));
    assert!(status.refuses(&value(json!(404))));

    let prefix = Test::Prefix("/orders/".to_string());
    assert!(!prefix.refuses(&value(json!("/orders/66"))));
    assert!(prefix.refuses(&value(json!("/order-lines/1"))));
    assert!(prefix.refuses(&value(json!("/orders"))));
    assert!(!prefix.refuses(&value(json!(66))));
  }
}
