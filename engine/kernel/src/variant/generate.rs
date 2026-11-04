//! Generators: the concrete payload one variant produces (plan task 4.3's fourth deliverable).
//!
//! This walks a shape tree exactly as [`crate::shape::variant_space`] does — same operators, same
//! path/dimension-id construction — but instead of recording *which axes vary*, it resolves each
//! one against a full [`Assignment`] and materialises the value that assignment describes. A
//! member's shape resolving to `None` means "omit this member" (an `optional` pinned to
//! `absent`, or `forbidden`), which is how a variant's width actually shows up in the produced
//! document.
//!
//! What this does not do: decide *which* variant to generate for (design 2.3's sampler,
//! [`super::select`]) or drive a transport with the result (no transport is bound to a consumer
//! session yet). It is pure: shape plus assignment in, a document-model value out.

use crate::interaction_spec::InteractionSpec;
use crate::plan::Assignment;
use crate::shape::variant_space::{canonical_point_name, cardinality_points};
use crate::shape::{CoreShape, Example, ShapeKind, ShapeNode, path};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde_json::Value;
use std::collections::BTreeMap;

/// What one slot of a payload is (staged plan S2.15): a document-model value, or octets, which a
/// slot is when the shape it resolves to has a `base64` example (shape spec §2.2). A transport is
/// handed octets as octets, and a contract records them so. Inside a document a byte-valued
/// example is its base64 text, since JSON has no bytes.
#[derive(Debug, Clone, PartialEq)]
pub enum Generated {
  Document(Value),
  Octets(Vec<u8>),
}

impl Generated {
  /// The document value, or `None` for octets.
  pub fn as_document(&self) -> Option<&Value> {
    match self {
      Generated::Document(value) => Some(value),
      Generated::Octets(_) => None,
    }
  }

  /// As JSON: octets as their base64 text, the one JSON-safe rendering.
  pub fn into_json(self) -> Value {
    match self {
      Generated::Document(value) => value,
      Generated::Octets(bytes) => Value::String(BASE64.encode(bytes)),
    }
  }
}

/// A node's example as it produces a value: octets when it is tagged `base64` and is base64, else
/// its value as it stands.
fn example(example: &Example) -> Generated {
  let octets = match example.encoded.as_deref() {
    Some("base64") => example.value.as_str().and_then(|text| BASE64.decode(text).ok()),
    _ => None,
  };
  match octets {
    Some(bytes) => Generated::Octets(bytes),
    None => Generated::Document(example.value.clone()),
  }
}

/// The concrete value one shape node produces under `assignment`, rooted at `at` (spec §6.2's
/// dimension-id path), as a position inside a document. `None` means the node contributes no member
/// at all under this variant.
pub fn value(shape: &ShapeNode, at: &str, assignment: &Assignment) -> Option<Value> {
  generated(shape, at, assignment).map(Generated::into_json)
}

/// [`value`], keeping octets: what a slot's shape produces.
fn generated(shape: &ShapeNode, at: &str, assignment: &Assignment) -> Option<Generated> {
  let ShapeKind::Core(core) = &shape.kind else {
    // A component operator's interior is opaque to the kernel (shape spec §3.5); its own
    // component (design 2.6) is what would generate a value for it. The best this walk can do
    // without guessing is fall back to the node's own declared example, if it named one.
    return shape.example.as_ref().map(example);
  };
  let document = |value: Value| Some(Generated::Document(value));
  match core {
    CoreShape::Optional { of } => {
      let id = path::dimension_id(at, "presence");
      let present = assignment.get(&id).map(String::as_str).unwrap_or("present") != "absent";
      present.then(|| generated(of, at, assignment)).flatten()
    }
    CoreShape::Forbidden => None,
    CoreShape::Nullable { of } => {
      let id = path::dimension_id(at, "nullability");
      if assignment.get(&id).map(String::as_str) == Some("null") {
        document(Value::Null)
      } else {
        generated(of, at, assignment)
      }
    }
    CoreShape::AnyOf { options } => {
      let id = path::dimension_id(at, "value");
      let chosen = assignment
        .get(&id)
        .and_then(|point| options.iter().find(|o| &canonical_point_name(o) == point));
      match (chosen, &shape.example) {
        (Some(option), _) => document(option.clone()),
        (None, Some(declared)) => Some(example(declared)),
        (None, None) => options.first().cloned().map(Generated::Document),
      }
    }
    CoreShape::OneOf {
      alternatives,
      default,
      ..
    } => {
      let id = path::dimension_id(at, "alternative");
      let chosen_name = assignment
        .get(&id)
        .cloned()
        .or_else(|| default.clone())
        .or_else(|| alternatives.keys().next().cloned())?;
      let alt = alternatives.get(&chosen_name)?;
      generated(alt, &path::alternative(at, &chosen_name), assignment)
    }
    CoreShape::EachLike { items, min, max } => {
      let size = cardinality_size(at, *min, *max, assignment);
      let item_path = path::each_like_item(at);
      let item_value = value(items, &item_path, assignment).unwrap_or(Value::Null);
      document(Value::Array((0..size).map(|_| item_value.clone()).collect()))
    }
    CoreShape::EachEntry {
      keys,
      values,
      min,
      max,
    } => {
      let size = cardinality_size(at, *min, *max, assignment);
      let entry_value = value(values, &path::each_entry_value(at), assignment).unwrap_or(Value::Null);
      let key_path = path::each_entry_key(at);
      let mut map = serde_json::Map::new();
      for index in 0..size {
        let key = keys
          .as_ref()
          .and_then(|k| value(k, &key_path, assignment))
          .and_then(|v| v.as_str().map(str::to_string))
          .unwrap_or_else(|| format!("key{index}"));
        map.insert(key, entry_value.clone());
      }
      document(Value::Object(map))
    }
    CoreShape::Object { members } => {
      // In the order the author declared them, so the octets `content/encode` writes follow it: a
      // CSV's columns, say (ADR 0037).
      let mut map = serde_json::Map::new();
      for (name, member) in members {
        if let Some(v) = value(member, &path::member(at, name), assignment) {
          map.insert(name.clone(), v);
        }
      }
      document(Value::Object(map))
    }
    CoreShape::Array { entries } => document(Value::Array(
      entries
        .iter()
        .enumerate()
        .map(|(index, entry)| value(entry, &path::array_index(at, index), assignment).unwrap_or(Value::Null))
        .collect(),
    )),
    CoreShape::Contains { entries } => document(Value::Array(
      entries
        .iter()
        .enumerate()
        .filter_map(|(index, entry)| value(entry, &path::array_index(at, index), assignment))
        .collect(),
    )),
    // Leaves: any, equality, type, kind, not-empty, regex, temporal, include, content-type,
    // semver — none of them vary by variant, so the node's own example is the value.
    _ => Some(
      shape
        .example
        .as_ref()
        .map(example)
        .unwrap_or(Generated::Document(Value::Null)),
    ),
  }
}

fn cardinality_size(at: &str, min: u64, max: Option<u64>, assignment: &Assignment) -> u64 {
  let id = path::dimension_id(at, "cardinality");
  match assignment.get(&id) {
    Some(point) => cardinality_points(min, max)
      .into_iter()
      .find(|p| &p.name == point)
      .map(|p| p.size)
      .unwrap_or(min),
    None => min,
  }
}

/// The concrete payload a whole interaction produces under one variant: part name -> slot name ->
/// value, omitting any slot whose shape resolved to nothing (spec §4.4's "concrete values it
/// produced" — the per-variant form [`crate::contract::model::RecordedVariant::parts`] records).
pub fn interaction(
  spec: &InteractionSpec,
  assignment: &Assignment,
) -> BTreeMap<String, BTreeMap<String, Generated>> {
  let mut parts = BTreeMap::new();
  for (part_name, part) in &spec.parts {
    let mut slots = BTreeMap::new();
    for (slot_name, shape) in part {
      let at = path::root(part_name, slot_name);
      if let Some(v) = generated(shape, &at, assignment) {
        slots.insert(slot_name.clone(), v);
      }
    }
    parts.insert(part_name.clone(), slots);
  }
  parts
}
