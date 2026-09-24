//! Recording a provider shape from the provider's own tests (plan task 7.2, design 2.8 §2.3's
//! `recorded` provenance): "the union of response shapes produced by the provider's own tests".
//!
//! **What a recorded shape claims, exactly.** It is evidence, not inference about the provider's
//! code: it says *these are the shapes my tests produced*, which is why §2.3 ranks `recorded`
//! highest for fidelity and why a checker must not read more into it. A provider whose tests never
//! produce `CANCELLED` records a shape without it, and the subsumption walk will then decide `yes`
//! where a complete shape would decide `no`. That gap is the provider's test coverage, not the
//! recorder's bug, and it is the one thing a reader of a `recorded` shape has to keep in mind.
//!
//! **Why a profile rather than a fold of shapes.** The obvious implementation — infer a shape per
//! observation and union the shapes pairwise — cannot make the one decision that matters here:
//! whether a scalar position is a closed set of values or an open domain. `{"PENDING"}` unioned
//! with `{"SHIPPED"}` and `{"DELIVERED"}` is a three-option enum; three timestamps unioned the same
//! way is a three-option enum too, and only one of those is right. Telling them apart needs the
//! counts — how many observations, how many distinct values, how often they repeated — so this
//! module accumulates a [`Profile`] tree of evidence and emits a shape once, at the end, when the
//! counts are all in. The profile *is* the union; §2.1's "union of response shapes" is its
//! `to_shape`.
//!
//! **The two judgements, and they are judgements.** [`RecordingPolicy`] holds them both, because a
//! recorder cannot avoid them and pretending otherwise would bury them:
//!
//! - `max_options` — beyond this many distinct values a position is an open domain, whatever the
//!   repetition says. Nothing deep: an enum with more cases than this exists, and recording it as
//!   a literal set would be technically right and practically unreadable.
//! - `min_evidence` — below this many observations, no amount of not-repeating is evidence of
//!   anything. A position that showed three values in three observations has demonstrated nothing
//!   about being closed, and a position that showed three values in twelve observations has.
//!
//! Getting these wrong is visible in one direction and invisible in the other, which is worth
//! saying plainly: widening too eagerly produces `broader-type` findings a team can see and argue
//! with, while keeping a literal set too long produces a narrow provider shape that makes the
//! checker answer `yes`. The defaults therefore lean towards widening.

use super::provider_shape::{FORMAT, ProviderInteraction, ProviderShape, ShapePart, StateRef};
use crate::contract::Party;
use serde_json::{Map, Value};
use std::collections::BTreeMap;

/// The two judgements a recorder cannot avoid (module docs).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecordingPolicy {
  /// Beyond this many distinct values, a scalar position is an open domain rather than a set.
  pub max_options: usize,
  /// Below this many observations, "every value was new" is not evidence of an open domain.
  pub min_evidence: u64,
}

impl Default for RecordingPolicy {
  fn default() -> Self {
    RecordingPolicy {
      max_options: 8,
      min_evidence: 3,
    }
  }
}

/// One interaction's identity, as design 2.8 §2.2 matches it: description plus state *names*.
type Key = (String, Vec<String>);

/// Accumulates observations and emits one [`ProviderShape`].
#[derive(Debug)]
pub struct Recorder {
  provider: String,
  provenance: String,
  policy: RecordingPolicy,
  interactions: BTreeMap<Key, Observed>,
}

#[derive(Debug, Default)]
struct Observed {
  /// Part name -> slot name -> the evidence accumulated at that slot's root.
  parts: BTreeMap<String, BTreeMap<String, Profile>>,
  observations: u64,
}

impl Recorder {
  pub fn new(provider: impl Into<String>) -> Recorder {
    Recorder::with_policy(provider, RecordingPolicy::default())
  }

  pub fn with_policy(provider: impl Into<String>, policy: RecordingPolicy) -> Recorder {
    Recorder {
      provider: provider.into(),
      provenance: "recorded".to_string(),
      policy,
      interactions: BTreeMap::new(),
    }
  }

  /// How many observations one interaction has contributed, for a host that wants to report it.
  pub fn observations(&self, description: &str, states: &[String]) -> u64 {
    self
      .interactions
      .get(&(description.to_string(), states.to_vec()))
      .map(|observed| observed.observations)
      .unwrap_or(0)
  }

  pub fn interaction_count(&self) -> usize {
    self.interactions.len()
  }

  /// Record one response the provider produced: the already-decoded value of each slot, keyed by
  /// part and slot exactly as a contract's `parts` are (contract spec §5.1).
  ///
  /// A slot missing from `parts` is not "absent" — it is *not observed*, and contributes nothing.
  /// Absence is a fact about a member inside a value, which is where the enclosing object's own
  /// observation count makes it decidable (spec §2.1's "only the slots it actually produces").
  pub fn observe(
    &mut self,
    description: &str,
    states: &[String],
    parts: &BTreeMap<String, BTreeMap<String, Value>>,
  ) {
    let observed = self
      .interactions
      .entry((description.to_string(), states.to_vec()))
      .or_default();
    observed.observations += 1;
    for (part, slots) in parts {
      let recorded_part = observed.parts.entry(part.clone()).or_default();
      for (slot, value) in slots {
        recorded_part.entry(slot.clone()).or_default().observe(value);
      }
    }
  }

  /// The provider shape the evidence adds up to (design 2.8 §2.4).
  pub fn finish(self) -> ProviderShape {
    let policy = self.policy;
    let interactions = self
      .interactions
      .into_iter()
      .map(|((description, states), observed)| ProviderInteraction {
        description,
        states: (!states.is_empty()).then(|| {
          states
            .into_iter()
            .map(|name| StateRef { name })
            .collect::<Vec<_>>()
        }),
        selector: None,
        provenance: None,
        source: Some(BTreeMap::from([(
          "observations".to_string(),
          Value::from(observed.observations),
        )])),
        parts: observed
          .parts
          .into_iter()
          .map(|(part, slots)| {
            let recorded: ShapePart = slots
              .into_iter()
              .filter_map(|(slot, profile)| Some((slot, profile.to_shape(&policy)?)))
              .collect();
            (part, recorded)
          })
          .filter(|(_, slots)| !slots.is_empty())
          .collect(),
      })
      .collect();

    ProviderShape {
      format: FORMAT.to_string(),
      schema: None,
      provider: Party { name: self.provider },
      provenance: Some(self.provenance),
      metadata: None,
      interactions,
    }
  }
}

// -----------------------------------------------------------------------------------------------
// The evidence
// -----------------------------------------------------------------------------------------------

/// What one position in a value tree has been seen to hold.
///
/// Every counter here exists to answer one question the emitted shape needs: how often the
/// position was looked at (presence), how often it held `null` (nullability), which JSON kinds it
/// took (the value operator), and which distinct literals (a set, or an open domain).
#[derive(Debug, Default, Clone)]
struct Profile {
  /// How many observations reached this position with a value of any kind, `null` included.
  seen: u64,
  /// ...of which the value was `null`.
  nulls: u64,
  /// Distinct scalar literals, keyed by canonical JSON text so the set is deterministic and
  /// `1` and `1.0` do not become two entries under two spellings.
  literals: BTreeMap<String, Value>,
  /// How many observations held a scalar, which is the denominator the repetition rule divides
  /// the distinct count into.
  scalars: u64,
  strings: bool,
  booleans: bool,
  integers: bool,
  decimals: bool,
  object: Option<Box<ObjectProfile>>,
  array: Option<Box<ArrayProfile>>,
}

#[derive(Debug, Default, Clone)]
struct ObjectProfile {
  /// How many objects were seen here — the denominator that makes a member's absence decidable.
  count: u64,
  members: BTreeMap<String, Profile>,
}

#[derive(Debug, Default, Clone)]
struct ArrayProfile {
  count: u64,
  min: u64,
  max: u64,
  /// Every element of every array seen here, accumulated into one profile — `each-like` says
  /// "every element is like this" (shape spec §6.5), so the elements share one shape.
  items: Profile,
}

impl Profile {
  fn observe(&mut self, value: &Value) {
    self.seen += 1;
    match value {
      Value::Null => self.nulls += 1,
      Value::Object(members) => {
        let object = self.object.get_or_insert_with(Box::default);
        object.count += 1;
        for (name, member) in members {
          object.members.entry(name.clone()).or_default().observe(member);
        }
      }
      Value::Array(items) => {
        let array = self.array.get_or_insert_with(|| {
          Box::new(ArrayProfile {
            count: 0,
            min: u64::MAX,
            max: 0,
            items: Profile::default(),
          })
        });
        array.count += 1;
        let length = items.len() as u64;
        array.min = array.min.min(length);
        array.max = array.max.max(length);
        for item in items {
          array.items.observe(item);
        }
      }
      scalar => {
        self.scalars += 1;
        match scalar {
          Value::String(_) => self.strings = true,
          Value::Bool(_) => self.booleans = true,
          Value::Number(number) => {
            if number.as_f64().is_some_and(|n| n.fract() == 0.0) {
              self.integers = true;
            } else {
              self.decimals = true;
            }
          }
          _ => {}
        }
        self
          .literals
          .entry(canonical(scalar))
          .or_insert_with(|| scalar.clone());
      }
    }
  }

  /// The shape this evidence adds up to, or `None` when the position was never reached.
  fn to_shape(&self, policy: &RecordingPolicy) -> Option<Value> {
    if self.seen == 0 {
      return None;
    }
    let non_null = self.seen - self.nulls;
    if non_null == 0 {
      // Every observation was `null`. `null` is a kind, not a modifier, when it is all there is.
      return Some(shape("null"));
    }
    let value_shape = self.value_shape(policy)?;
    Some(if self.nulls > 0 {
      // Seen with and without a value: `nullable` is exactly that union (shape spec §4.4).
      wrap("nullable", value_shape)
    } else {
      value_shape
    })
  }

  /// The shape of the non-`null` evidence.
  fn value_shape(&self, policy: &RecordingPolicy) -> Option<Value> {
    let shapes = self.structural_kinds();
    match shapes {
      // One kind of thing, which is the ordinary case and the only one that can be described
      // more precisely than "any value".
      1 => {
        if let Some(object) = &self.object {
          return Some(self.object_shape(object, policy));
        }
        if let Some(array) = &self.array {
          return Some(self.array_shape(array, policy));
        }
        Some(self.scalar_shape(policy))
      }
      // A position that held an object in one run and a string in another is not describable by
      // any narrower core operator, and guessing which one "really" belongs there is exactly what
      // a recorder must not do.
      _ => Some(shape("any")),
    }
  }

  /// How many *structurally different* things this position held: an object, a list, a scalar.
  fn structural_kinds(&self) -> usize {
    usize::from(self.object.is_some()) + usize::from(self.array.is_some()) + usize::from(self.scalars > 0)
  }

  fn object_shape(&self, object: &ObjectProfile, policy: &RecordingPolicy) -> Value {
    let mut members = Map::new();
    for (name, member) in &object.members {
      let Some(member_shape) = member.to_shape(policy) else {
        continue;
      };
      // A member that was not in every object is optional — and this is the one place absence is
      // decidable, because the enclosing object's own count is the denominator.
      members.insert(
        name.clone(),
        if member.seen < object.count {
          wrap("optional", member_shape)
        } else {
          member_shape
        },
      );
    }
    Value::Object(Map::from_iter([
      ("shape".to_string(), Value::from("object")),
      ("members".to_string(), Value::Object(members)),
    ]))
  }

  fn array_shape(&self, array: &ArrayProfile, policy: &RecordingPolicy) -> Value {
    // A list only ever seen empty has no element evidence at all; `any` is the honest item shape,
    // and the cardinality carries what was actually observed.
    let items = array.items.to_shape(policy).unwrap_or_else(|| shape("any"));
    Value::Object(Map::from_iter([
      ("shape".to_string(), Value::from("each-like")),
      ("min".to_string(), Value::from(array.min)),
      ("max".to_string(), Value::from(array.max)),
      ("items".to_string(), items),
    ]))
  }

  /// A scalar position: a literal set while the evidence supports one, the kind predicate
  /// otherwise (module docs).
  fn scalar_shape(&self, policy: &RecordingPolicy) -> Value {
    let distinct = self.literals.len();
    let values: Vec<&Value> = self.literals.values().collect();

    // Two booleans are `boolean`. Recording them as `any-of[true, false]` would be the same set
    // written the long way, and the kind predicate is what a reader expects.
    if self.booleans && !self.strings && !self.integers && !self.decimals && distinct == 2 {
      return shape("boolean");
    }

    if self.keeps_literals(policy) {
      return if distinct == 1 {
        with_example(shape("equality"), values[0].clone())
      } else {
        let mut node = Map::from_iter([
          ("shape".to_string(), Value::from("any-of")),
          (
            "options".to_string(),
            Value::Array(values.iter().map(|value| (*value).clone()).collect()),
          ),
        ]);
        node.insert("example".to_string(), values[0].clone());
        Value::Object(node)
      };
    }

    let kind = match (self.strings, self.booleans, self.integers, self.decimals) {
      (true, false, false, false) => "string",
      (false, true, false, false) => "boolean",
      (false, false, true, false) => "integer",
      (false, false, false, true) => "decimal",
      (false, false, true, true) => "number",
      // Scalars of genuinely different kinds at one position.
      _ => return shape("any"),
    };
    with_example(shape(kind), values[0].clone())
  }

  /// Whether the distinct values seen here are a set the provider chooses from, or samples of an
  /// open domain. The two rules are [`RecordingPolicy`]'s, and this is the only place they apply.
  fn keeps_literals(&self, policy: &RecordingPolicy) -> bool {
    let distinct = self.literals.len();
    if distinct > policy.max_options {
      return false;
    }
    // "Every observation was a new value" is evidence of an open domain — but only once there
    // have been enough observations for it to be evidence of anything.
    let never_repeated = distinct as u64 == self.scalars;
    !(never_repeated && self.scalars >= policy.min_evidence)
  }
}

/// A scalar's canonical text, which is also how two spellings of one number (`1`, `1.0`) end up as
/// one entry (shape spec §4.2's numeric rule).
fn canonical(value: &Value) -> String {
  match value {
    Value::Number(number) => number
      .as_f64()
      .map(|n| format!("n:{n}"))
      .unwrap_or_else(|| format!("n:{number}")),
    other => other.to_string(),
  }
}

fn shape(operator: &str) -> Value {
  Value::Object(Map::from_iter([("shape".to_string(), Value::from(operator))]))
}

fn with_example(mut node: Value, example: Value) -> Value {
  if let Some(members) = node.as_object_mut() {
    members.insert("example".to_string(), example);
  }
  node
}

fn wrap(operator: &str, of: Value) -> Value {
  Value::Object(Map::from_iter([
    ("shape".to_string(), Value::from(operator)),
    ("of".to_string(), of),
  ]))
}
