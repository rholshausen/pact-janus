//! `<component>:match-field` and `<component>:generate-field` (staged plan S5.2g, ADR 0113): a plugin's
//! field-level matching rule and generator, the `MATCHER` and `GENERATOR` catalogue entries a plugin of
//! interface version 2 registers (pact-plugins' proposal 006), called with `MatchField` and `GenerateField`.
//!
//! A field rule sees one value. For `match-field` the value under test is the value the rule applies to,
//! and the action has two arguments: the value the pact records there, and an object naming the `rule`
//! (its name in the pact, which is the catalogue key or a longer form of it), its `values` (the rule's
//! configuration in the pact, `brand` for the credit card rule), the `mismatch-type` (the part of the
//! interaction the value came from, as today's verifier names it: `body`, `header`, `query`, `metadata`,
//! `path`, `status`) and, for a value inside a part, that part's `root`, the plan path the value's own is
//! read under. The plugin is handed the value's path as a pact writes it, which is its plan path with the
//! root taken off (`$.response.body.card.number` under `$.response.body` is `$.card.number`), and no path
//! for the request path or the status, as today's verifier hands it none.
//!
//! For `generate-field` the value under test is the example the generated value replaces, and the one
//! argument names the `generator` (its `type` and `values`), the `path` it fills, and the `test-context`,
//! the exchange's provider-state values. The test mode is the provider's: a verifier fills the request it
//! sends. Both requests are built as pact-plugin-driver's `FieldMatcher` and `FieldGenerator` build them
//! for today's verifier, and both answers read as `pact_matching` reads them.

use crate::catalogue::Plugin;
use crate::compare::octets;
use crate::error::ComponentError;
use pact_plugin_driver::field::FieldValue;
use pact_plugin_driver::proto_v2::{self, generate_content_request::TestMode};
use pact_plugin_driver::utils::to_proto_struct;
use serde_json::{Map, Value, json};

/// Whether a rule or generator `name`, as a pact writes it, names the catalogue entry `key` of type
/// `kind` (`matcher` or `generator`) that `plugin` registered: its key, or a longer form of the catalogue
/// key ending in it, as pact-plugin-driver's `resolve_capability` matches one, whole components only.
pub fn names(name: &str, plugin: &str, kind: &str, key: &str) -> bool {
  name == key || name == format!("{kind}/{key}") || name == format!("plugin/{plugin}/{kind}/{key}")
}

/// What one `match-field` application compares, read from its arguments.
#[derive(Debug, Clone)]
pub struct MatchArguments {
  pub rule: String,
  values: Map<String, Value>,
  mismatch_type: String,
  root: Option<String>,
  expected: Value,
}

impl MatchArguments {
  /// Reads the arguments from an application's `config` (spec §7.2): the expected value, then an object.
  pub fn read(config: Option<&Value>) -> Result<MatchArguments, ComponentError> {
    let arguments = config
      .and_then(|config| config.get("arguments"))
      .and_then(Value::as_array)
      .filter(|arguments| arguments.len() == 2)
      .ok_or_else(|| {
        ComponentError::invalid_config(
          "match-field takes two arguments: the value the pact records, and an object naming the rule",
        )
      })?;
    let argument = arguments[1]
      .as_object()
      .ok_or_else(|| ComponentError::invalid_config("match-field's second argument must be an object"))?;
    let rule = argument
      .get("rule")
      .and_then(Value::as_str)
      .ok_or_else(|| ComponentError::invalid_config("match-field's argument names no 'rule'"))?
      .to_string();
    let values = object(argument, "values", "match-field")?;
    let mismatch_type = argument
      .get("mismatch-type")
      .and_then(Value::as_str)
      .unwrap_or("body")
      .to_string();
    let root = argument.get("root").and_then(Value::as_str).map(str::to_string);
    Ok(MatchArguments {
      rule,
      values,
      mismatch_type,
      root,
      expected: arguments[0].clone(),
    })
  }

  /// The value's path as a pact writes it: its plan path with the part's root taken off, or none for a
  /// value that is a whole part (the request path, the status).
  pub fn path(&self, value: &Value) -> String {
    let Some(root) = &self.root else {
      return String::new();
    };
    let path = value.get("path").and_then(Value::as_str).unwrap_or(root);
    match path.strip_prefix(root.as_str()) {
      Some(rest) if rest.is_empty() || rest.starts_with(['.', '[']) => format!("${rest}"),
      _ => path.to_string(),
    }
  }

  /// The plugin's `MatchField` request for the value under test, built as the driver's
  /// `FieldMatcher::match_field` builds it. Today's verifier hands it no plugin configuration, and an
  /// empty test context.
  pub fn request(&self, key: &str, value: &Value) -> Result<proto_v2::MatchFieldRequest, ComponentError> {
    Ok(proto_v2::MatchFieldRequest {
      key: key.to_string(),
      rule: Some(proto_v2::MatchingRule {
        r#type: self.rule.clone(),
        values: Some(structure(self.values.clone())),
      }),
      path: self.path(value),
      mismatch_type: self.mismatch_type.clone(),
      expected: Some(FieldValue::Json(self.expected.clone()).to_proto()),
      actual: Some(field_value(value, "the value under test")?.to_proto()),
      plugin_configuration: None,
      test_context: Some(structure(Map::new())),
    })
  }
}

/// What the plugin answered, as a plan result document (plan-grammar spec §2.3): `ok` when it found
/// nothing; an `error` whose message is the plugin's error, when it could not apply the rule; else an
/// `error` whose message joins every mismatch's, as `pact_matching` joins them for today's verifier. When
/// there are several, or one the plugin located somewhere other than the value, its `mismatches` keep
/// each, `at` where the plugin located it (ADR 0110).
pub fn match_result(path: &str, answer: Result<proto_v2::MatchFieldResponse, anyhow::Error>) -> Value {
  let response = match answer {
    Ok(response) => response,
    Err(err) => return json!({ "status": "error", "message": err.to_string() }),
  };
  if !response.error.is_empty() {
    return json!({ "status": "error", "message": response.error });
  }
  if response.mismatches.is_empty() {
    return json!({ "status": "ok" });
  }
  let message = response
    .mismatches
    .iter()
    .map(|mismatch| mismatch.mismatch.as_str())
    .collect::<Vec<_>>()
    .join(", ");
  let elsewhere = |mismatch: &proto_v2::ContentMismatch| !mismatch.path.is_empty() && mismatch.path != path;
  if response.mismatches.len() == 1 && !elsewhere(&response.mismatches[0]) {
    return json!({ "status": "error", "message": message });
  }
  let text =
    |bytes: &Option<Vec<u8>>| String::from_utf8_lossy(bytes.as_deref().unwrap_or_default()).to_string();
  let described: Vec<Value> = response
    .mismatches
    .iter()
    .map(|mismatch| {
      let mut described = Map::new();
      described.insert("message".to_string(), json!(mismatch.mismatch));
      if elsewhere(mismatch) {
        described.insert("at".to_string(), json!(mismatch.path));
      }
      for (key, value) in [
        ("expected", text(&mismatch.expected)),
        ("actual", text(&mismatch.actual)),
      ] {
        if !value.is_empty() {
          described.insert(key.to_string(), json!(value));
        }
      }
      Value::Object(described)
    })
    .collect();
  json!({ "status": "error", "message": message, "mismatches": described })
}

/// What one `generate-field` application fills, read from its argument.
#[derive(Debug, Clone)]
pub struct GenerateArguments {
  pub generator: String,
  values: Map<String, Value>,
  path: String,
  test_context: Map<String, Value>,
}

impl GenerateArguments {
  /// Reads the argument from an application's `config` (spec §7.2): for this action, one object.
  pub fn read(config: Option<&Value>) -> Result<GenerateArguments, ComponentError> {
    let argument = config
      .and_then(|config| config.get("arguments"))
      .and_then(Value::as_array)
      .and_then(|arguments| arguments.first())
      .and_then(Value::as_object)
      .ok_or_else(|| {
        ComponentError::invalid_config("generate-field takes one argument, an object naming the generator")
      })?;
    let generator = argument
      .get("generator")
      .and_then(Value::as_object)
      .ok_or_else(|| ComponentError::invalid_config("generate-field's argument names no 'generator'"))?;
    let name = generator
      .get("type")
      .and_then(Value::as_str)
      .ok_or_else(|| ComponentError::invalid_config("generate-field's 'generator' has no 'type'"))?
      .to_string();
    Ok(GenerateArguments {
      generator: name,
      values: object(generator, "values", "generate-field")?,
      path: argument
        .get("path")
        .and_then(Value::as_str)
        .unwrap_or("$")
        .to_string(),
      test_context: object(argument, "test-context", "generate-field")?,
    })
  }

  /// The plugin's `GenerateField` request, built as the driver's `FieldGenerator::generate_field` builds
  /// it, in the provider's test mode.
  pub fn request(&self, key: &str, value: &Value) -> Result<proto_v2::GenerateFieldRequest, ComponentError> {
    Ok(proto_v2::GenerateFieldRequest {
      key: key.to_string(),
      generator: Some(proto_v2::Generator {
        r#type: self.generator.clone(),
        values: Some(structure(self.values.clone())),
      }),
      path: self.path.clone(),
      example_value: Some(field_value(value, "the example")?.to_proto()),
      plugin_configuration: None,
      test_context: Some(structure(self.test_context.clone())),
      test_mode: TestMode::Provider as i32,
    })
  }
}

/// What the plugin generated, as a plan result document: the value as a `value`. Bytes are text where
/// they are UTF-8, as `pact_matching` takes them, since a generated value goes into a document. A plugin
/// that answers with a failure, or with nothing, is an `error`: the engine then sends the recorded value
/// and says so.
pub fn generate_result(
  generator: &str,
  answer: Result<proto_v2::GenerateFieldResponse, anyhow::Error>,
) -> Value {
  let response = match answer {
    Ok(response) => response,
    Err(err) => {
      return json!({ "status": "error", "message": format!("Generator '{generator}' failed: {err}") });
    }
  };
  if !response.error.is_empty() {
    return json!({ "status": "error", "message": format!("Generator '{generator}' failed: {}", response.error) });
  }
  let Some(value) = response.value else {
    return json!({ "status": "error", "message": format!("Generator '{generator}' returned no value") });
  };
  match FieldValue::from_proto(&value) {
    FieldValue::Json(value) => json!({ "status": "value", "value": { "value": value } }),
    FieldValue::Binary(bytes) => match std::str::from_utf8(&bytes) {
      Ok(text) => json!({ "status": "value", "value": { "value": text } }),
      Err(_) => json!({
        "status": "error",
        "message": format!(
          "The '{generator}' generator returned {} bytes of binary data, which can not be used as a value",
          bytes.len()
        )
      }),
    },
  }
}

/// The catalogue entry's key a plugin registered for `name`, among `keys` of `kind`.
pub fn entry_key<'a>(
  plugin: &Plugin,
  kind: &str,
  keys: impl IntoIterator<Item = &'a str>,
  name: &str,
) -> Option<&'a str> {
  keys.into_iter().find(|key| names(name, &plugin.name, kind, key))
}

/// A `MatchValue` as the field value the plugin is handed: octets when it is tagged `base64`, and its
/// JSON otherwise, whose whole numbers stay whole (proposal 006's value model).
fn field_value(value: &Value, what: &str) -> Result<FieldValue, ComponentError> {
  match value.get("encoded").and_then(Value::as_str) {
    Some("base64") => Ok(FieldValue::Binary(octets(value, what)?.into())),
    _ => Ok(FieldValue::Json(
      value.get("content").cloned().unwrap_or(Value::Null),
    )),
  }
}

fn object(
  argument: &Map<String, Value>,
  key: &str,
  action: &str,
) -> Result<Map<String, Value>, ComponentError> {
  match argument.get(key) {
    None | Some(Value::Null) => Ok(Map::new()),
    Some(Value::Object(members)) => Ok(members.clone()),
    Some(_) => Err(ComponentError::invalid_config(format!(
      "{action}'s '{key}' must be an object"
    ))),
  }
}

fn structure(members: Map<String, Value>) -> prost_types::Struct {
  to_proto_struct(&members.into_iter().collect())
}

#[cfg(test)]
mod tests {
  use super::*;
  use pact_plugin_driver::proto_v2::field_value;
  use pretty_assertions::assert_eq;
  use rstest::rstest;

  fn arguments(argument: Value) -> MatchArguments {
    MatchArguments::read(Some(&json!({ "arguments": ["4111111111111111", argument] }))).unwrap()
  }

  #[rstest]
  #[case("creditcard", true)]
  #[case("matcher/creditcard", true)]
  #[case("plugin/creditcard/matcher/creditcard", true)]
  #[case("generator/creditcard", false)]
  #[case("plugin/other/matcher/creditcard", false)]
  #[case("card", false)]
  fn a_name_names_an_entry_by_its_key_or_a_longer_form_of_it(#[case] name: &str, #[case] named: bool) {
    assert_eq!(names(name, "creditcard", "matcher", "creditcard"), named);
  }

  #[test]
  fn the_request_is_what_the_driver_would_send() {
    let arguments = arguments(json!({
      "rule": "creditcard", "values": { "brand": "visa" }, "mismatch-type": "body", "root": "$.response.body"
    }));
    let request = arguments
      .request(
        "creditcard",
        &json!({ "content": "4111111111111112", "path": "$.response.body.card.number" }),
      )
      .unwrap();
    assert_eq!(request.key, "creditcard");
    assert_eq!(request.path, "$.card.number");
    assert_eq!(request.mismatch_type, "body");
    let rule = request.rule.unwrap();
    assert_eq!(rule.r#type, "creditcard");
    assert!(rule.values.unwrap().fields.contains_key("brand"));
    assert_eq!(
      request.actual.unwrap().value,
      Some(field_value::Value::StringValue("4111111111111112".into()))
    );
    assert_eq!(
      request.expected.unwrap().value,
      Some(field_value::Value::StringValue("4111111111111111".into()))
    );
    assert!(request.plugin_configuration.is_none());
  }

  #[rstest]
  #[case(Some("$.response.body"), "$.response.body", "$")]
  #[case(
    Some("$.response.body"),
    "$.response.body.cards[1].number",
    "$.cards[1].number"
  )]
  #[case(Some("$.response.headers"), "$.response.headers.x-card", "$.x-card")]
  #[case(None, "$.response.status", "")]
  fn the_plugin_is_handed_the_path_a_pact_writes(
    #[case] root: Option<&str>,
    #[case] path: &str,
    #[case] expected: &str,
  ) {
    let arguments = arguments(json!({ "rule": "creditcard", "root": root }));
    assert_eq!(arguments.path(&json!({ "content": 1, "path": path })), expected);
  }

  #[test]
  fn a_whole_number_stays_whole_and_octets_stay_octets() {
    let integer = field_value(&json!({ "content": 100 }), "v").unwrap().to_proto();
    assert_eq!(integer.value, Some(field_value::Value::IntegerValue(100)));
    let decimal = field_value(&json!({ "content": 100.5 }), "v").unwrap().to_proto();
    assert_eq!(decimal.value, Some(field_value::Value::DecimalValue(100.5)));
    let bytes = field_value(&json!({ "content": "AAE=", "encoded": "base64" }), "v")
      .unwrap()
      .to_proto();
    assert_eq!(bytes.value, Some(field_value::Value::BinaryValue(vec![0, 1])));
  }

  fn mismatch(message: &str, path: &str) -> proto_v2::ContentMismatch {
    proto_v2::ContentMismatch {
      mismatch: message.to_string(),
      path: path.to_string(),
      ..proto_v2::ContentMismatch::default()
    }
  }

  #[test]
  fn an_answer_is_ok_or_one_error_naming_what_the_plugin_found() {
    let ok = match_result("$.n", Ok(proto_v2::MatchFieldResponse::default()));
    assert_eq!(ok, json!({ "status": "ok" }));
    let one = match_result(
      "$.n",
      Ok(proto_v2::MatchFieldResponse {
        mismatches: vec![mismatch("fails the Luhn check", "$.n")],
        ..Default::default()
      }),
    );
    assert_eq!(
      one,
      json!({ "status": "error", "message": "fails the Luhn check" })
    );
    let two = match_result(
      "$.n",
      Ok(proto_v2::MatchFieldResponse {
        mismatches: vec![mismatch("first", ""), mismatch("second", "$.n.x")],
        ..Default::default()
      }),
    );
    assert_eq!(
      two,
      json!({
        "status": "error", "message": "first, second",
        "mismatches": [{ "message": "first" }, { "message": "second", "at": "$.n.x" }]
      })
    );
    let misconfigured = match_result(
      "$.n",
      Ok(proto_v2::MatchFieldResponse {
        error: "'nope' is not a credit card brand".into(),
        ..Default::default()
      }),
    );
    assert_eq!(misconfigured["status"], "error");
    assert_eq!(misconfigured["message"], "'nope' is not a credit card brand");
  }

  #[test]
  fn a_generated_value_is_a_value_and_a_failure_an_error() {
    let request = GenerateArguments::read(Some(&json!({ "arguments": [{
      "generator": { "type": "creditcard", "values": { "brand": "visa" } },
      "path": "$.card.number",
      "test-context": { "id": 7 }
    }] })))
    .unwrap()
    .request("creditcard", &json!({ "content": "4111111111111111" }))
    .unwrap();
    assert_eq!(request.generator.unwrap().r#type, "creditcard");
    assert_eq!(request.path, "$.card.number");
    assert_eq!(request.test_mode, TestMode::Provider as i32);
    assert!(request.test_context.unwrap().fields.contains_key("id"));

    let generated = generate_result(
      "creditcard",
      Ok(proto_v2::GenerateFieldResponse {
        value: Some(FieldValue::Json(json!("4012888888881881")).to_proto()),
        ..Default::default()
      }),
    );
    assert_eq!(
      generated,
      json!({ "status": "value", "value": { "value": "4012888888881881" } })
    );
    let failed = generate_result(
      "creditcard",
      Ok(proto_v2::GenerateFieldResponse {
        error: "unknown brand".into(),
        ..Default::default()
      }),
    );
    assert_eq!(failed["status"], "error");
  }
}
