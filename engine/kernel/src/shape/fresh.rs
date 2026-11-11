//! Fresh values (shape-language spec §3.4, staged plan S1.19): a node's `generator` names a generator
//! component and its configuration, and where a value is produced — a verifier's request, a mock's
//! reply — the component produces it in place of the recorded one. The shape still governs what is
//! admitted; the recorded value is still the variant's evidence (ADR 0048).
//!
//! The walk is the shape's own: an `object`'s members, an `array`'s entries by position, every
//! element an `each-like` holds, every value an `each-entry` holds, the value an `optional` or
//! `nullable` wraps when there is one, and the `one-of` alternative whose discriminator the value
//! binds (ADR 0085). A `contains` entry claims some element, and which is not known until it is
//! matched, so nothing below one is filled. A node with a generator is filled, and nothing below it
//! is walked: the generated value replaced it. What a component is, or what its configuration means,
//! is not known here.
//!
//! [`check`] is the other half: when an interaction is added, every generator its shapes name is
//! asked for once, so a name no component provides, or a configuration it refuses, is the author's
//! error then, rather than a recorded value served in silence on every exchange (ADR 0085).

use crate::component::{Generate, GeneratorComponent};
use crate::error::{Problem, push_pointer_segment};
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
    ("one-of", value @ Value::Object(_)) => {
      if let Some(alternative) = bound_alternative(node, value) {
        fill(alternative, value, at, fresh, failures);
      }
    }
    _ => {}
  }
}

/// The `one-of` alternative `value` is, by the literal its discriminator member binds (shape spec
/// §5.4): each alternative is an `object`, perhaps wrapped in `nullable`, whose discriminator member
/// is that literal's node. `None` when no alternative binds the value's.
fn bound_alternative<'a>(node: &'a Map<String, Value>, value: &Value) -> Option<&'a Value> {
  let discriminator = node.get("discriminator").and_then(Value::as_str)?;
  let bound = value.get(discriminator)?;
  node
    .get("alternatives")
    .and_then(Value::as_object)?
    .values()
    .find(|alternative| {
      let object = match alternative.get("shape").and_then(Value::as_str) {
        Some("nullable") => alternative.get("of"),
        _ => Some(*alternative),
      };
      object
        .and_then(|object| object.get("members"))
        .and_then(|members| members.get(discriminator))
        .and_then(|literal| literal.get("example"))
        == Some(bound)
    })
}

/// The members of a shape node that hold shapes, by how many: a map of them, a list of them, or one.
const SHAPE_MAPS: [&str; 2] = ["members", "alternatives"];
const SHAPE_LISTS: [&str; 1] = ["entries"];
const SHAPE_ONES: [&str; 4] = ["items", "keys", "values", "of"];

/// Every generator `shape` names, anywhere in it, asked for once on the consumer's side with its
/// node's example (ADR 0085). One whose member is not `{ name, config }`, whose name no component
/// provides (`unknown-generator`), or whose configuration its component refuses (`invalid-config`)
/// is a problem at `pointer`'s node's `generator`. A generator that answers with an error of another
/// kind, one that needs a provider state, say, is the exchange's to report, as it is today. A
/// component operator's own members are its own, and are not walked.
pub fn check(shape: &Value, pointer: &str, generators: &dyn GeneratorComponent, problems: &mut Vec<Problem>) {
  let Value::Object(node) = shape else {
    return;
  };
  if let Some(generator) = node.get(GENERATOR) {
    let mut at = pointer.to_string();
    push_pointer_segment(&mut at, GENERATOR);
    if let Some(message) = refusal(generator, node.get("example"), generators) {
      problems.push(Problem { pointer: at, message });
    }
  }
  let operator = node.get("shape").and_then(Value::as_str).unwrap_or_default();
  if operator.contains(':') {
    return;
  }
  for member in SHAPE_MAPS {
    if let Some(Value::Object(shapes)) = node.get(member) {
      for (name, child) in shapes {
        let mut at = pointer.to_string();
        push_pointer_segment(&mut at, member);
        push_pointer_segment(&mut at, name);
        check(child, &at, generators, problems);
      }
    }
  }
  for member in SHAPE_LISTS {
    if let Some(Value::Array(shapes)) = node.get(member) {
      for (index, child) in shapes.iter().enumerate() {
        check(
          child,
          &format!("{pointer}/{member}/{index}"),
          generators,
          problems,
        );
      }
    }
  }
  for member in SHAPE_ONES {
    if let Some(child) = node.get(member) {
      check(child, &format!("{pointer}/{member}"), generators, problems);
    }
  }
}

/// Why `generator` cannot be asked, if it cannot.
fn refusal(
  generator: &Value,
  example: Option<&Value>,
  generators: &dyn GeneratorComponent,
) -> Option<String> {
  let Some(name) = generator.get("name").and_then(Value::as_str) else {
    return Some(
      "a generator is { name, config }, and 'name' is the generator's, '<component>:<generator>'".into(),
    );
  };
  let config = generator.get("config");
  if config.is_some_and(|config| !config.is_object()) {
    return Some(format!(
      "generator '{name}': 'config' is an object, the generator's own configuration"
    ));
  }
  let asked = generators.generate(Generate {
    generator: name.to_string(),
    config: config.cloned(),
    context: Some(json!({
      "example": example.cloned().unwrap_or(Value::Null),
      "side": "consumer",
      "provider-state": {},
    })),
  });
  match asked {
    Err(error) if error.code == "unknown-generator" => Some(error.message),
    Err(error) if error.code == "invalid-config" => Some(format!("generator '{name}': {}", error.message)),
    _ => None,
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

  #[test]
  fn a_one_of_fills_the_alternative_its_discriminator_binds() {
    let shape = json!({ "shape": "one-of", "discriminator": "type", "alternatives": {
      "card": { "shape": "object", "members": {
        "type": { "shape": "equality", "example": "card" },
        "token": { "shape": "string", "example": "t", "generator": { "name": "test:card" } } } },
      "invoice": { "shape": "nullable", "of": { "shape": "object", "members": {
        "type": { "shape": "equality", "example": "invoice" },
        "number": { "shape": "string", "example": "n", "generator": { "name": "test:invoice" } } } } },
    } });
    let state = Map::new();
    let fresh = Fresh {
      generators: &Naming,
      side: "consumer",
      provider_state: &state,
    };
    let mut failures = Vec::new();

    let mut card = json!({ "type": "card", "token": "t" });
    fill(&shape, &mut card, "$.body", &fresh, &mut failures);
    assert_eq!(
      card["token"],
      json!("test:card from \"t\" on the \"consumer\" side")
    );

    let mut invoice = json!({ "type": "invoice", "number": "n" });
    fill(&shape, &mut invoice, "$.body", &fresh, &mut failures);
    assert_eq!(
      invoice["number"],
      json!("test:invoice from \"n\" on the \"consumer\" side")
    );

    let mut neither = json!({ "type": "cash", "token": "t" });
    fill(&shape, &mut neither, "$.body", &fresh, &mut failures);
    assert_eq!(neither, json!({ "type": "cash", "token": "t" }));
    assert_eq!(failures, vec![]);
  }

  #[test]
  fn every_generator_a_shape_names_is_asked_for_when_it_is_checked() {
    use crate::component::PactGenerators;
    let shape = json!({ "shape": "object", "members": {
      "id": { "shape": "string", "example": "x", "generator": { "name": "pact:Uuid" } },
      "typo": { "shape": "string", "example": "x", "generator": { "name": "pact:Uiud" } },
      "a/b": { "shape": "string", "example": "x", "generator": { "name": "pact:Regex" } },
      "items": { "shape": "each-like", "items": { "shape": "integer", "example": 1,
        "generator": { "name": "pact:RandomInt", "config": [1, 2] } } },
      "state": { "shape": "string", "example": "x",
        "generator": { "name": "pact:ProviderState", "config": { "expression": "${id}" } } },
      "pay": { "shape": "one-of", "discriminator": "t", "alternatives": { "a": { "shape": "object", "members": {
        "t": { "shape": "equality", "example": "a" },
        "n": { "shape": "string", "example": "n", "generator": { "nom": "pact:Uuid" } } } } } },
      "list": { "shape": "http:list", "elements": ["a"], "example": ["a"],
        "of": { "generator": { "name": "pact:Uiud" } } },
    } });
    let mut problems = Vec::new();
    check(&shape, "/parts/response/body", &PactGenerators, &mut problems);
    assert_eq!(
      problems,
      vec![
        Problem {
          pointer: "/parts/response/body/members/typo/generator".to_string(),
          message: "no generator 'pact:Uiud'".to_string(),
        },
        Problem {
          pointer: "/parts/response/body/members/a~1b/generator".to_string(),
          message: "generator 'pact:Regex': 'Regex' is missing a member it needs (regex)".to_string(),
        },
        Problem {
          pointer: "/parts/response/body/members/items/items/generator".to_string(),
          message: "generator 'pact:RandomInt': 'config' is an object, the generator's own configuration"
            .to_string(),
        },
        Problem {
          pointer: "/parts/response/body/members/pay/alternatives/a/members/n/generator".to_string(),
          message:
            "a generator is { name, config }, and 'name' is the generator's, '<component>:<generator>'"
              .to_string(),
        },
      ]
    );
  }
}
