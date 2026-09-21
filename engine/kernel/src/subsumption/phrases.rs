//! The phrase table (design 2.8 §4.4, §6.5): how a shape node is described to a human.
//!
//! §4.4's requirement is determinism, not prose quality — "two checkers comparing the same two
//! shapes MUST produce the same summary text, so a diff between two report runs is a diff of
//! substance, not of phrasing". Everything here is therefore a total function of the node's
//! operator and its parameters, with no configuration and no locale, the same discipline
//! plan-grammar spec §3.1 applies to `explain`'s pretty form.
//!
//! Two phrasings exist on purpose and are not interchangeable: [`describe`] is the `summary` a
//! finding records (§4.4), and [`super::render`] has its own per-kind templates for the text block
//! §6.4 fixes; spec §6.5 tabulates both. The worked examples show both for the same finding — a
//! cardinality summary reads "0 to unbounded elements" while the rendered line reads "provider may
//! produce an empty list" — so the renderer's table is separate rather than derived from this one.

use crate::shape::{CoreShape, Example, ShapeKind, ShapeNode, TemporalKind, ValueKind};
use serde_json::Value;

/// One node, as a finding's `summary` (§4.4).
pub fn describe(node: &ShapeNode) -> String {
  let core = match &node.kind {
    ShapeKind::Component { operator, .. } => return format!("component operator '{operator}'"),
    ShapeKind::Core(core) => core,
  };
  match core {
    CoreShape::Any => "any value".to_string(),
    CoreShape::Equality => match &node.example {
      Some(example) => format!("exactly {}", literal(example)),
      // `equality` without an example is ill-formed (shape spec §4.2); a checker reading a
      // document someone else wrote still has to say something rather than panic.
      None => "exactly one value".to_string(),
    },
    CoreShape::Type => match node.example.as_ref().map(class_phrase) {
      Some(phrase) => format!("any {phrase}"),
      None => "any value of one kind".to_string(),
    },
    CoreShape::Kind(kind) => kind_phrase(*kind).to_string(),
    CoreShape::NotEmpty => "any non-empty value".to_string(),
    CoreShape::Regex { pattern } => format!("strings matching '{pattern}'"),
    CoreShape::Temporal { kind, .. } => match kind {
      TemporalKind::DateTime => "a datetime".to_string(),
      TemporalKind::Date => "a date".to_string(),
      TemporalKind::Time => "a time".to_string(),
    },
    CoreShape::Include { substring } => format!("strings containing '{substring}'"),
    CoreShape::ContentType { content_type } => format!("octets detected as {content_type}"),
    CoreShape::Semver => "a semantic version".to_string(),
    CoreShape::Object { .. } => "an object".to_string(),
    CoreShape::Array { entries } => format!("an array of exactly {} elements", entries.len()),
    CoreShape::EachLike { min, max, .. } => cardinality(*min, *max),
    CoreShape::EachEntry { min, max, .. } => cardinality_entries(*min, *max),
    CoreShape::Contains { entries } => {
      format!("an array containing {} matched elements", entries.len())
    }
    CoreShape::Optional { of } => format!("{}, or absent", describe(of)),
    CoreShape::Forbidden => "absent".to_string(),
    CoreShape::Nullable { of } => format!("{}, or null", describe(of)),
    CoreShape::AnyOf { options } => format!("one of {}", literals(options)),
    CoreShape::OneOf { alternatives, .. } => format!(
      "one of the alternatives {}",
      join(alternatives.keys().map(|name| format!("'{name}'"))),
    ),
  }
}

/// A node that must be present, as the consumer side of a `weaker-presence` finding reads it
/// (§4.2's "a datetime, always present").
pub fn describe_present(node: &ShapeNode) -> String {
  format!("{}, always present", describe(node))
}

/// The provider side of an `undeclared-member` finding (§3.2's Rule 2): there is no node to
/// describe, and what makes the finding is precisely that.
pub const UNCONSTRAINED: &str = "unconstrained: any value, or absent";

/// An `each-like` interval, which is also the grammar [`super::render`] reads the bounds back
/// from: `"<min> to <max|unbounded> elements"`.
pub fn cardinality(min: u64, max: Option<u64>) -> String {
  match max {
    Some(max) => format!("{min} to {max} elements"),
    None => format!("{min} to unbounded elements"),
  }
}

/// The same, for `each-entry`, whose members are entries rather than elements.
pub fn cardinality_entries(min: u64, max: Option<u64>) -> String {
  match max {
    Some(max) => format!("{min} to {max} entries"),
    None => format!("{min} to unbounded entries"),
  }
}

/// A literal, as the examples render one: a string in single quotes, anything else as its JSON
/// text (the same rule shape spec §6.4 gives for `any-of` point names).
pub fn literal(example: &Example) -> String {
  if example.encoded.is_some() {
    return "an octet sequence".to_string();
  }
  value(&example.value)
}

pub fn value(value: &Value) -> String {
  match value {
    Value::String(text) => format!("'{text}'"),
    other => other.to_string(),
  }
}

pub fn literals(options: &[Value]) -> String {
  join(options.iter().map(value))
}

fn join(parts: impl Iterator<Item = String>) -> String {
  parts.collect::<Vec<_>>().join(" | ")
}

fn kind_phrase(kind: ValueKind) -> &'static str {
  match kind {
    ValueKind::String => "any string",
    ValueKind::Number => "any number",
    ValueKind::Integer => "a whole number",
    ValueKind::Decimal => "a number with a fractional part",
    ValueKind::Boolean => "any boolean",
    ValueKind::Null => "null",
  }
}

/// The kind of a `type` operator's example (shape spec §4.2's kinds).
fn class_phrase(example: &Example) -> &'static str {
  if example.encoded.is_some() {
    return "octet sequence";
  }
  match &example.value {
    Value::Null => "null",
    Value::Bool(_) => "boolean",
    Value::Number(_) => "number",
    Value::String(_) => "string",
    Value::Array(_) => "array",
    Value::Object(_) => "object",
  }
}
