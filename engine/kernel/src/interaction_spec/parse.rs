//! Parser/validator for the interaction-specification document (plan task 3.2).
//!
//! Every violation found is collected and reported together, per node, as
//! [`Problem`]s — the `interaction-invalid` quality this task exists to deliver (engine protocol
//! spec §8.2, §10.2).

use super::error::InteractionSpecError;
use super::model::{InteractionSpec, Part};
use crate::common::{Requirement, State, Transport};
use crate::error::{Problem, json_pointer, push_pointer_segment};
use crate::shape;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value};
use std::collections::BTreeMap;

/// Parse and validate one interaction specification.
pub fn parse(value: &Value) -> Result<InteractionSpec, InteractionSpecError> {
  let mut problems = Vec::new();

  let Some(obj) = value.as_object() else {
    problems.push(Problem {
      pointer: String::new(),
      message: "an interaction specification must be an object".to_string(),
    });
    return Err(InteractionSpecError { problems });
  };

  let description = parse_description(obj, &mut problems);
  let transport = parse_field::<Transport>(obj, "transport", &mut problems);
  let states = parse_field::<Vec<State>>(obj, "states", &mut problems);
  let requires = parse_field::<Vec<Requirement>>(obj, "requires", &mut problems);
  let parts = parse_parts(obj, &mut problems);

  match (description, parts) {
    (Some(description), Some(parts)) if problems.is_empty() => Ok(InteractionSpec {
      description,
      transport,
      states,
      parts,
      requires,
    }),
    _ => Err(InteractionSpecError { problems }),
  }
}

fn parse_description(obj: &Map<String, Value>, problems: &mut Vec<Problem>) -> Option<String> {
  match obj.get("description") {
    None => {
      problems.push(Problem {
        pointer: "/description".to_string(),
        message: "'description' is required".to_string(),
      });
      None
    }
    Some(Value::String(description)) if !description.is_empty() => Some(description.clone()),
    Some(Value::String(_)) => {
      problems.push(Problem {
        pointer: "/description".to_string(),
        message: "'description' must not be empty".to_string(),
      });
      None
    }
    Some(_) => {
      problems.push(Problem {
        pointer: "/description".to_string(),
        message: "'description' must be a string".to_string(),
      });
      None
    }
  }
}

/// Deserialize an optional top-level field, translating a `serde` failure into a [`Problem`]
/// pointing at the field (and, for a nested failure, the member inside it).
fn parse_field<T: DeserializeOwned>(
  obj: &Map<String, Value>,
  field: &str,
  problems: &mut Vec<Problem>,
) -> Option<T> {
  let value = obj.get(field)?;
  match serde_path_to_error::deserialize(value) {
    Ok(parsed) => Some(parsed),
    Err(err) => {
      let mut pointer = String::new();
      push_pointer_segment(&mut pointer, field);
      pointer.push_str(&json_pointer(err.path()));
      problems.push(Problem {
        pointer,
        message: err.inner().to_string(),
      });
      None
    }
  }
}

/// `parts`: part name -> slot name -> shape (contract-file spec §5.1, shape-language spec §3.6).
/// Required; every slot's shape is parsed and validated via [`shape::parse`].
fn parse_parts(obj: &Map<String, Value>, problems: &mut Vec<Problem>) -> Option<BTreeMap<String, Part>> {
  let Some(parts_value) = obj.get("parts") else {
    problems.push(Problem {
      pointer: "/parts".to_string(),
      message: "'parts' is required".to_string(),
    });
    return None;
  };
  let Some(parts_obj) = parts_value.as_object() else {
    problems.push(Problem {
      pointer: "/parts".to_string(),
      message: "'parts' must be an object".to_string(),
    });
    return None;
  };

  let mut parts = BTreeMap::new();
  for (part_name, slots_value) in parts_obj {
    let mut part_path = String::from("/parts");
    push_pointer_segment(&mut part_path, part_name);

    let Some(slots_obj) = slots_value.as_object() else {
      problems.push(Problem {
        pointer: part_path,
        message: format!("part '{part_name}' must be an object of slot name -> shape"),
      });
      continue;
    };

    let mut slots = BTreeMap::new();
    for (slot_name, shape_value) in slots_obj {
      let mut slot_path = part_path.clone();
      push_pointer_segment(&mut slot_path, slot_name);
      match shape::parse(shape_value, &slot_path) {
        Ok(node) => {
          slots.insert(slot_name.clone(), node);
        }
        Err(mut shape_problems) => problems.append(&mut shape_problems),
      }
    }
    parts.insert(part_name.clone(), slots);
  }
  Some(parts)
}
