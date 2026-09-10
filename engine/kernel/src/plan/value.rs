//! The interpreter's runtime value (plan task 3.4): a superset of the plan document model's seven
//! kinds plus `entry` (plan-grammar spec §2.2), with one addition the static model does not need:
//! [`RuntimeValue::Absent`]. A compiled literal can never *be* absent — there is no way to write
//! `⊥` as a shape's `example` (shape-language spec §2.1) — but a value a resolver hands back at
//! run time certainly can be: that is exactly what "this member is not there" means, and
//! `check:exists` needs a way to tell it apart from an actual `null`.

use super::model::{DocumentKind, Literal};
use base64::Engine;
use serde_json::{Number, Value};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeValue {
  Absent,
  Null,
  Bool(bool),
  Number(Number),
  String(String),
  Array(Vec<RuntimeValue>),
  Object(BTreeMap<String, RuntimeValue>),
  Bytes(Vec<u8>),
  /// A key paired with a value (spec §2.2) — what `resolve-current`'s `.key`/`.value` suffixes
  /// address while iterating an `each-entry` (plan task 3.3's compiled form).
  Entry {
    key: String,
    value: Box<RuntimeValue>,
  },
}

impl RuntimeValue {
  pub fn from_json(value: &Value) -> RuntimeValue {
    match value {
      Value::Null => RuntimeValue::Null,
      Value::Bool(b) => RuntimeValue::Bool(*b),
      Value::Number(n) => RuntimeValue::Number(n.clone()),
      Value::String(s) => RuntimeValue::String(s.clone()),
      Value::Array(items) => RuntimeValue::Array(items.iter().map(RuntimeValue::from_json).collect()),
      Value::Object(members) => RuntimeValue::Object(
        members
          .iter()
          .map(|(k, v)| (k.clone(), RuntimeValue::from_json(v)))
          .collect(),
      ),
    }
  }

  /// From a compiled plan literal (spec §2.2), honouring its bytes tag.
  pub fn from_literal(literal: &Literal) -> RuntimeValue {
    match literal.of {
      DocumentKind::Bytes => match literal.encoded.as_deref() {
        Some("base64") => literal
          .value
          .as_str()
          .and_then(|s| base64::engine::general_purpose::STANDARD.decode(s).ok())
          .map(RuntimeValue::Bytes)
          .unwrap_or(RuntimeValue::Absent),
        _ => RuntimeValue::from_json(&literal.value),
      },
      DocumentKind::Entry => RuntimeValue::Entry {
        key: literal.key.clone().unwrap_or_default(),
        value: Box::new(RuntimeValue::from_json(&literal.value)),
      },
      _ => RuntimeValue::from_json(&literal.value),
    }
  }

  /// Back to the document model, for building readable mismatch messages. Bytes round-trip as
  /// their base64 text (there is no other JSON-safe rendering); this is display-only.
  pub fn to_json(&self) -> Value {
    match self {
      RuntimeValue::Absent | RuntimeValue::Null => Value::Null,
      RuntimeValue::Bool(b) => Value::Bool(*b),
      RuntimeValue::Number(n) => Value::Number(n.clone()),
      RuntimeValue::String(s) => Value::String(s.clone()),
      RuntimeValue::Array(items) => Value::Array(items.iter().map(RuntimeValue::to_json).collect()),
      RuntimeValue::Object(members) => {
        Value::Object(members.iter().map(|(k, v)| (k.clone(), v.to_json())).collect())
      }
      RuntimeValue::Bytes(bytes) => Value::String(base64::engine::general_purpose::STANDARD.encode(bytes)),
      RuntimeValue::Entry { key, value } => {
        let mut map = serde_json::Map::new();
        map.insert(key.clone(), value.to_json());
        Value::Object(map)
      }
    }
  }
}

/// Navigate `value` by a relative path suffix built from `.member` and `[index]` segments (e.g.
/// `.sku`, `[0]`, `.payment.dueDate`) — the syntax [`super::path`] and the compiler's `Cursor`
/// both use. An empty suffix returns `value` itself.
///
/// A member that does not exist, an index out of range, or descending into a value that is not
/// the addressed shape (an object for `.member`, an array for `[index]`) yields
/// [`RuntimeValue::Absent`] rather than an error: resolution is where absence is *discovered*,
/// and `check:exists` — not a resolve failure — is how a plan asks about it.
pub fn navigate(value: &RuntimeValue, suffix: &str) -> RuntimeValue {
  if suffix.is_empty() {
    return value.clone();
  }
  if let Some(after_dot) = suffix.strip_prefix('.') {
    let end = after_dot.find(['.', '[']).unwrap_or(after_dot.len());
    let (member, remaining) = after_dot.split_at(end);
    let next = match value {
      RuntimeValue::Object(members) => members.get(member).cloned().unwrap_or(RuntimeValue::Absent),
      RuntimeValue::Entry { key, value } => match member {
        "key" => RuntimeValue::String(key.clone()),
        "value" => (**value).clone(),
        _ => RuntimeValue::Absent,
      },
      _ => RuntimeValue::Absent,
    };
    navigate(&next, remaining)
  } else if let Some(after_bracket) = suffix.strip_prefix('[') {
    let end = after_bracket.find(']').unwrap_or(after_bracket.len());
    let (index_text, rest) = after_bracket.split_at(end);
    let remaining = rest.strip_prefix(']').unwrap_or(rest);
    let next = match index_text.parse::<usize>() {
      Ok(index) => match value {
        RuntimeValue::Array(items) => items.get(index).cloned().unwrap_or(RuntimeValue::Absent),
        _ => RuntimeValue::Absent,
      },
      Err(_) => RuntimeValue::Absent,
    };
    navigate(&next, remaining)
  } else {
    RuntimeValue::Absent
  }
}
