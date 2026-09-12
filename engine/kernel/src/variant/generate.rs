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
use crate::shape::{CoreShape, ShapeKind, ShapeNode, path};
use serde_json::Value;
use std::collections::BTreeMap;

/// The concrete value one shape node produces under `assignment`, rooted at `at` (spec §6.2's
/// dimension-id path). `None` means the node contributes no member at all under this variant.
pub fn value(shape: &ShapeNode, at: &str, assignment: &Assignment) -> Option<Value> {
  let ShapeKind::Core(core) = &shape.kind else {
    // A component operator's interior is opaque to the kernel (shape spec §3.5); its own
    // component (design 2.6) is what would generate a value for it. The best this walk can do
    // without guessing is fall back to the node's own declared example, if it named one.
    return shape.example.as_ref().map(|e| e.value.clone());
  };
  match core {
    CoreShape::Optional { of } => {
      let id = path::dimension_id(at, "presence");
      let present = assignment.get(&id).map(String::as_str).unwrap_or("present") != "absent";
      present.then(|| value(of, at, assignment)).flatten()
    }
    CoreShape::Forbidden => None,
    CoreShape::Nullable { of } => {
      let id = path::dimension_id(at, "nullability");
      if assignment.get(&id).map(String::as_str) == Some("null") {
        Some(Value::Null)
      } else {
        value(of, at, assignment)
      }
    }
    CoreShape::AnyOf { options } => {
      let id = path::dimension_id(at, "value");
      let chosen = assignment
        .get(&id)
        .and_then(|point| options.iter().find(|o| &canonical_point_name(o) == point));
      chosen
        .or(shape.example.as_ref().map(|e| &e.value))
        .or(options.first())
        .cloned()
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
      value(alt, &path::alternative(at, &chosen_name), assignment)
    }
    CoreShape::EachLike { items, min, max } => {
      let size = cardinality_size(at, *min, *max, assignment);
      let item_path = path::each_like_item(at);
      let item_value = value(items, &item_path, assignment).unwrap_or(Value::Null);
      Some(Value::Array((0..size).map(|_| item_value.clone()).collect()))
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
      Some(Value::Object(map))
    }
    CoreShape::Object { members } => {
      let mut map = serde_json::Map::new();
      for (name, member) in members {
        if let Some(v) = value(member, &path::member(at, name), assignment) {
          map.insert(name.clone(), v);
        }
      }
      Some(Value::Object(map))
    }
    CoreShape::Array { entries } => Some(Value::Array(
      entries
        .iter()
        .enumerate()
        .map(|(index, entry)| value(entry, &path::array_index(at, index), assignment).unwrap_or(Value::Null))
        .collect(),
    )),
    CoreShape::Contains { entries } => Some(Value::Array(
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
        .map(|e| e.value.clone())
        .unwrap_or(Value::Null),
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
) -> BTreeMap<String, BTreeMap<String, Value>> {
  let mut parts = BTreeMap::new();
  for (part_name, part) in &spec.parts {
    let mut slots = BTreeMap::new();
    for (slot_name, shape) in part {
      let at = path::root(part_name, slot_name);
      if let Some(v) = value(shape, &at, assignment) {
        slots.insert(slot_name.clone(), v);
      }
    }
    parts.insert(part_name.clone(), slots);
  }
  parts
}
