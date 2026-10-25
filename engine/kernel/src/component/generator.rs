//! The matcher interface's generating half (component-interfaces spec §7.3): `generator/generate`,
//! which produces a value where one must be fresh rather than replayed, and the one generator
//! component every engine has built in, [`PactGenerators`] (ADR 0034).

use super::ComponentError;
use pact_models::generators::{
  GenerateValue, Generator, GeneratorTestMode, NoopVariantMatcher, VariantMatcher,
};
use serde_json::{Map, Value};
use std::collections::HashMap;

/// `generator/generate`'s request (`schemas/v1/matcher.schema.json`).
#[derive(Debug, Clone)]
pub struct Generate {
  /// Namespaced: `<component>:<name>`.
  pub generator: String,
  pub config: Option<Value>,
  /// An open document. Known members (spec §7.3): `example`, the value recorded where the generated
  /// one will go, and `provider-state`, the exchange's provider-state values.
  pub context: Option<Value>,
}

/// The generated value. The native binding carries the document-model value itself; the schema's
/// `MatchValue` wrapping is the pipe's business.
#[derive(Debug, Clone, PartialEq)]
pub struct GenerateResult {
  pub value: Value,
}

/// Native binding (spec §9.1), `Send + Sync` because a verification run owns a thread.
pub trait GeneratorComponent: Send + Sync {
  fn generate(&self, req: Generate) -> Result<GenerateResult, ComponentError>;
}

/// The v1–v4 generator vocabulary as a generator component, `pact` (ADR 0034). Values are produced
/// by `pact_models`, the code today's Rust verifier produces them with, so a generated value is
/// generated the way it is today. Only what a provider-side run applies is offered: `MockServerURL`
/// is the consumer side's, and a plugin's generator is the plugin's.
#[derive(Debug, Default, Clone, Copy)]
pub struct PactGenerators;

impl PactGenerators {
  /// The namespace every generator here is named under.
  pub const NAME: &'static str = "pact";

  /// The generators this component answers for, unqualified.
  pub const GENERATORS: &'static [&'static str] = &[
    "RandomInt",
    "RandomDecimal",
    "RandomHexadecimal",
    "RandomString",
    "Regex",
    "Uuid",
    "Date",
    "Time",
    "DateTime",
    "RandomBoolean",
    "ProviderState",
  ];

  /// `pact:<name>` for a v1–v4 generator type.
  pub fn qualified(name: &str) -> String {
    format!("{}:{name}", Self::NAME)
  }
}

impl GeneratorComponent for PactGenerators {
  fn generate(&self, req: Generate) -> Result<GenerateResult, ComponentError> {
    let name = req
      .generator
      .strip_prefix("pact:")
      .filter(|name| Self::GENERATORS.contains(name))
      .ok_or_else(|| ComponentError::unknown_generator(&req.generator))?;
    let config = match req.config {
      None | Some(Value::Null) => Map::new(),
      Some(Value::Object(config)) => config,
      Some(_) => {
        return Err(ComponentError::invalid_config(
          &req.generator,
          "a generator's config must be an object",
        ));
      }
    };
    let generator = Generator::from_map(name, &config).ok_or_else(|| {
      ComponentError::invalid_config(
        &req.generator,
        format!("'{name}' is missing a member it needs ({})", required(name)),
      )
    })?;

    let context = req.context.unwrap_or(Value::Null);
    let example = context.get("example").cloned().unwrap_or(Value::Null);
    // A generator runs on the side it is for, as today: `ProviderState` on the provider's, when a
    // verifier sends a request. Asked on the other side, it keeps the recorded value (ADR 0048).
    let side = match context.get("side").and_then(Value::as_str) {
      Some("consumer") => GeneratorTestMode::Consumer,
      _ => GeneratorTestMode::Provider,
    };
    if !generator.corresponds_to_mode(&side) {
      return Ok(GenerateResult { value: example });
    }
    let provider_state: HashMap<&str, Value> = context
      .get("provider-state")
      .and_then(Value::as_object)
      .map(|values| values.iter().map(|(k, v)| (k.as_str(), v.clone())).collect())
      .unwrap_or_default();

    let matcher: Box<dyn VariantMatcher + Send + Sync> = NoopVariantMatcher.boxed();
    generator
      .generate_value(&example, &provider_state, &matcher)
      .map(|value| GenerateResult { value })
      .map_err(|err| ComponentError::generate_failed(&req.generator, err.to_string()))
  }
}

/// What `Generator::from_map` refuses a generator without, for the error message.
fn required(name: &str) -> &'static str {
  match name {
    "Regex" => "regex",
    "ProviderState" => "expression",
    _ => "none is required, so this is a bug",
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use pretty_assertions::assert_eq;
  use serde_json::json;

  fn generate(generator: &str, config: Value, context: Value) -> Result<Value, ComponentError> {
    PactGenerators
      .generate(Generate {
        generator: generator.to_string(),
        config: Some(config),
        context: Some(context),
      })
      .map(|result| result.value)
  }

  #[test]
  fn a_provider_state_expression_reads_the_provider_state_values() {
    let value = generate(
      "pact:ProviderState",
      json!({ "expression": "/orders/${id}", "dataType": "STRING" }),
      json!({ "example": "/orders/1", "provider-state": { "id": 1234 } }),
    );
    assert_eq!(value.unwrap(), json!("/orders/1234"));
  }

  #[test]
  fn a_random_integer_keeps_the_type_of_the_example() {
    let text = generate(
      "pact:RandomInt",
      json!({ "min": 5, "max": 5 }),
      json!({ "example": "1" }),
    );
    assert_eq!(text.unwrap(), json!("5"));
    let number = generate(
      "pact:RandomInt",
      json!({ "min": 5, "max": 5 }),
      json!({ "example": 1 }),
    );
    assert_eq!(number.unwrap(), json!(5));
  }

  /// A mock asks on the consumer side, where today's mock server keeps a `ProviderState` value as
  /// recorded (ADR 0048).
  #[test]
  fn a_provider_state_expression_asked_on_the_consumer_side_keeps_the_example() {
    let value = generate(
      "pact:ProviderState",
      json!({ "expression": "/orders/${id}" }),
      json!({ "example": "/orders/1", "side": "consumer", "provider-state": { "id": 1234 } }),
    );
    assert_eq!(value.unwrap(), json!("/orders/1"));
    let value = generate(
      "pact:RandomInt",
      json!({ "min": 7, "max": 7 }),
      json!({ "example": 1, "side": "consumer" }),
    );
    assert_eq!(value.unwrap(), json!(7));
  }

  #[test]
  fn a_provider_state_value_nobody_provided_is_a_generate_failure() {
    let error = generate(
      "pact:ProviderState",
      json!({ "expression": "${id}" }),
      json!({ "example": "1", "provider-state": {} }),
    )
    .unwrap_err();
    assert_eq!(error.code, "generate-failed");
  }

  #[test]
  fn a_generator_this_component_does_not_provide_is_named() {
    for name in ["pact:MockServerURL", "pact:Teapot", "other:RandomInt"] {
      let error = generate(name, json!({}), json!({})).unwrap_err();
      assert_eq!(error.code, "unknown-generator", "{name}");
    }
  }

  #[test]
  fn a_regex_without_a_pattern_is_invalid_config() {
    let error = generate("pact:Regex", json!({}), json!({ "example": "x" })).unwrap_err();
    assert_eq!(error.code, "invalid-config");
  }
}
