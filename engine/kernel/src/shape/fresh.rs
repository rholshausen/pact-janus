//! Fresh values (shape-language spec §3.4, staged plan S1.19): a node's `generator` names a generator
//! component and its configuration, and where a value is produced — a verifier's request, a mock's
//! reply — the component produces it in place of the recorded one. The shape still governs what is
//! admitted; the recorded value is still the variant's evidence (ADR 0048).
//!
//! The walk is the shape's own: an `object`'s members, an `array`'s entries by position, every
//! element an `each-like` holds, every value an `each-entry` holds, and the value an `optional` or
//! `nullable` wraps when there is one. A `contains` entry claims some element, and which is not
//! known until it is matched, so nothing below one is filled. A node with a generator is filled, and nothing
//! below it is walked: the generated value replaced it. What a component is, or what its
//! configuration means, is not known here.

use crate::component::{Generate, GeneratorComponent};
use crate::plan::member_path;
use serde_json::{Map, Value, json};

/// The member that holds a node's generator.
pub const GENERATOR: &str = "generator";

/// What a generator is asked with, besides its own configuration: which side asks — `provider`, a
/// verifier's request, or `consumer`, a mock's reply — and the exchange's provider-state values
/// (component-interfaces spec §7.3).
pub struct Fresh<'a> {
  pub generators: &'a dyn GeneratorComponent,
  pub side: &'a str,
  pub provider_state: &'a Map<String, Value>,
}

/// A generator that produced no value, so the recorded one stayed (ADR 0034 decision 5).
#[derive(Debug, Clone, PartialEq)]
pub struct Failure {
  /// Where the value would have gone, as a plan path: `$.request.body.items[0].id`.
  pub target: String,
  /// The generator, as the node names it: `pact:RandomInt`.
  pub generator: String,
  pub message: String,
}

/// Whether `shape`, or any node below it, carries a generator.
pub fn has_generators(shape: &Value) -> bool {
  match shape {
    Value::Object(node) => node.contains_key(GENERATOR) || node.values().any(has_generators),
    Value::Array(items) => items.iter().any(has_generators),
    _ => false,
  }
}

/// Fill every position of `value` that `shape` gives a generator, reporting each that failed.
/// `at` is the value's plan path, for the report.
pub fn fill(shape: &Value, value: &mut Value, at: &str, fresh: &Fresh<'_>, failures: &mut Vec<Failure>) {
  let Value::Object(node) = shape else {
    return;
  };
  if let Some(generator) = node.get(GENERATOR) {
    let name = generator.get("name").and_then(Value::as_str).unwrap_or_default();
    let asked = fresh.generators.generate(Generate {
      generator: name.to_string(),
      config: generator.get("config").cloned(),
      context: Some(json!({
        "example": value.clone(),
        "side": fresh.side,
        "provider-state": fresh.provider_state,
      })),
    });
    match asked {
      Ok(result) => *value = result.value,
      Err(error) => failures.push(Failure {
        target: at.to_string(),
        generator: name.to_string(),
        message: error.message,
      }),
    }
    return;
  }
  let operator = node.get("shape").and_then(Value::as_str).unwrap_or_default();
  match (operator, value) {
    ("object", Value::Object(members)) => {
      let Some(Value::Object(shapes)) = node.get("members") else {
        return;
      };
      for (name, member) in shapes {
        if let Some(value) = members.get_mut(name) {
          fill(member, value, &member_path(at, name), fresh, failures);
        }
      }
    }
    ("array", Value::Array(items)) => {
      let entries = node
        .get("entries")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
      for (index, (entry, item)) in entries.iter().zip(items.iter_mut()).enumerate() {
        fill(entry, item, &format!("{at}[{index}]"), fresh, failures);
      }
    }
    ("each-like", Value::Array(items)) => {
      if let Some(template) = node.get("items") {
        for (index, item) in items.iter_mut().enumerate() {
          fill(template, item, &format!("{at}[{index}]"), fresh, failures);
        }
      }
    }
    ("each-entry", Value::Object(members)) => {
      if let Some(values) = node.get("values") {
        for (name, value) in members.iter_mut() {
          fill(values, value, &member_path(at, name), fresh, failures);
        }
      }
    }
    ("optional" | "nullable", value) if !value.is_null() => {
      if let Some(of) = node.get("of") {
        fill(of, value, at, fresh, failures);
      }
    }
    _ => {}
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::component::{ComponentError, GenerateResult};
  use pretty_assertions::assert_eq;

  /// Answers every generator with its name, and fails `test:fails`.
  struct Naming;

  impl GeneratorComponent for Naming {
    fn generate(&self, req: Generate) -> Result<GenerateResult, ComponentError> {
      if req.generator == "test:fails" {
        return Err(ComponentError::generate_failed(&req.generator, "no value"));
      }
      let context = req.context.unwrap_or_default();
      Ok(GenerateResult {
        value: json!(format!(
          "{} from {} on the {} side",
          req.generator, context["example"], context["side"]
        )),
      })
    }
  }

  #[test]
  fn every_position_a_generator_marks_is_filled_and_nothing_else() {
    let shape = json!({ "shape": "object", "members": {
      "id": { "shape": "integer", "example": 1, "generator": { "name": "test:id" } },
      "name": { "shape": "string", "example": "Ada" },
      "items": { "shape": "each-like", "items": { "shape": "object", "members": {
        "sku": { "shape": "string", "example": "a", "generator": { "name": "test:sku", "config": {} } } } } },
      "note": { "shape": "optional", "of": { "shape": "string", "example": "n", "generator": { "name": "test:fails" } } },
      "a.b": { "shape": "string", "example": "x", "generator": { "name": "test:dotted" } },
    } });
    let mut value =
      json!({ "id": 1, "name": "Ada", "items": [{ "sku": "a" }, { "sku": "b" }], "note": "n", "a.b": "x" });
    let mut failures = Vec::new();
    let state = Map::new();
    let fresh = Fresh {
      generators: &Naming,
      side: "consumer",
      provider_state: &state,
    };
    fill(&shape, &mut value, "$.response.body", &fresh, &mut failures);
    assert_eq!(
      value,
      json!({
        "id": "test:id from 1 on the \"consumer\" side",
        "name": "Ada",
        "items": [{ "sku": "test:sku from \"a\" on the \"consumer\" side" }, { "sku": "test:sku from \"b\" on the \"consumer\" side" }],
        "note": "n",
        "a.b": "test:dotted from \"x\" on the \"consumer\" side",
      })
    );
    assert_eq!(
      failures,
      vec![Failure {
        target: "$.response.body.note".to_string(),
        generator: "test:fails".to_string(),
        message: "no value".to_string(),
      }]
    );
    assert!(has_generators(&shape));
    assert!(!has_generators(
      &json!({ "shape": "object", "members": { "a": { "shape": "any" } } })
    ));
  }
}
