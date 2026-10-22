//! How the opaque members of a recorded document are written (contract-file spec §2.4, ADR 0037).
//!
//! The typed records in [`super::model`] get their member order from their declarations. The
//! members this crate keeps as `Value` need a rule of their own, because `serde_json` keeps an
//! object's members in the order they arrived:
//!
//! - a **record** the engine or a specification defines — a selection report, an assignment, a
//!   variant-param binding's own members — is written by name, all the way down, so its bytes never
//!   depend on the order the code that built it, or an SDK, happened to use;
//! - a **shape** is written by [`crate::shape::canonical`]: its own members by name, an `object`
//!   shape's members in the author's order;
//! - a **document** — a recorded slot's content, a state's parameter values, a shape's example —
//!   is written as it arrived, since its order is the author's, or the order the author's shape
//!   produced.
//!
//! These are `serialize_with` functions, so every route that writes a contract or a provider shape
//! (a `finalise` frame, `janus upgrade`, a document read and written again) writes the same bytes.

use crate::shape;
use serde::{Serialize, Serializer};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

/// `value` as a record: every object's members by name, at every depth.
pub fn sorted(value: &Value) -> Value {
  match value {
    Value::Object(members) => {
      let mut names: Vec<&String> = members.keys().collect();
      names.sort();
      Value::Object(
        names
          .into_iter()
          .map(|name| (name.clone(), sorted(&members[name.as_str()])))
          .collect(),
      )
    }
    Value::Array(items) => Value::Array(items.iter().map(sorted).collect()),
    other => other.clone(),
  }
}

/// Part name -> slot name -> shape, each shape written canonically.
pub(crate) fn shape_parts<S: Serializer>(
  parts: &BTreeMap<String, BTreeMap<String, Value>>,
  serializer: S,
) -> Result<S::Ok, S::Error> {
  canonical_parts(parts).serialize(serializer)
}

/// [`shape_parts`] for an optional member, which `skip_serializing_if` leaves out when absent.
pub(crate) fn optional_shape_parts<S: Serializer>(
  parts: &Option<BTreeMap<String, BTreeMap<String, Value>>>,
  serializer: S,
) -> Result<S::Ok, S::Error> {
  parts.as_ref().map(canonical_parts).serialize(serializer)
}

fn canonical_parts(
  parts: &BTreeMap<String, BTreeMap<String, Value>>,
) -> BTreeMap<&String, BTreeMap<&String, Value>> {
  parts
    .iter()
    .map(|(part, slots)| {
      (
        part,
        slots
          .iter()
          .map(|(slot, node)| (slot, shape::canonical(node)))
          .collect(),
      )
    })
    .collect()
}

/// A record kept as a map of `Value`s, written by name at every depth.
pub(crate) fn record_map<S: Serializer>(
  record: &BTreeMap<String, Value>,
  serializer: S,
) -> Result<S::Ok, S::Error> {
  record
    .iter()
    .map(|(name, value)| (name, sorted(value)))
    .collect::<BTreeMap<_, _>>()
    .serialize(serializer)
}

/// [`record_map`] for an optional member.
pub(crate) fn optional_record_map<S: Serializer>(
  record: &Option<BTreeMap<String, Value>>,
  serializer: S,
) -> Result<S::Ok, S::Error> {
  record
    .as_ref()
    .map(|record| {
      record
        .iter()
        .map(|(name, value)| (name, sorted(value)))
        .collect::<BTreeMap<_, _>>()
    })
    .serialize(serializer)
}

/// A list of records, each written by name at every depth.
#[allow(clippy::ptr_arg)] // `serialize_with` hands over the field itself: `&Vec<Value>`.
pub(crate) fn record_list<S: Serializer>(records: &Vec<Value>, serializer: S) -> Result<S::Ok, S::Error> {
  records
    .iter()
    .map(sorted)
    .collect::<Vec<_>>()
    .serialize(serializer)
}

/// A state's variant-param bindings (variant-semantics spec §6.2): each binding and each of its
/// cases by name, and the values a parameter takes, `value` and `default`, as they arrived — they
/// are state parameters, a document.
pub(crate) fn bindings<S: Serializer>(
  bindings: &Option<Vec<Value>>,
  serializer: S,
) -> Result<S::Ok, S::Error> {
  bindings
    .as_ref()
    .map(|bindings| bindings.iter().map(binding).collect::<Vec<_>>())
    .serialize(serializer)
}

fn binding(binding: &Value) -> Value {
  let Value::Object(members) = binding else {
    return binding.clone();
  };
  let mut names: Vec<&String> = members.keys().collect();
  names.sort();
  let mut out = Map::new();
  for name in names {
    let member = &members[name.as_str()];
    let written = match (name.as_str(), member) {
      ("default", _) => member.clone(),
      ("cases", Value::Array(cases)) => Value::Array(cases.iter().map(case).collect()),
      _ => sorted(member),
    };
    out.insert(name.clone(), written);
  }
  Value::Object(out)
}

fn case(case: &Value) -> Value {
  let Value::Object(members) = case else {
    return case.clone();
  };
  let mut names: Vec<&String> = members.keys().collect();
  names.sort();
  Value::Object(
    names
      .into_iter()
      .map(|name| {
        let member = &members[name.as_str()];
        let written = if name == "value" {
          member.clone()
        } else {
          sorted(member)
        };
        (name.clone(), written)
      })
      .collect(),
  )
}
