//! A form body written as JSON with matchers (ADR 0066 decision 2): processed as a JSON body is,
//! then written as a query string, each number or string member one parameter and each list one
//! parameter per element.
//!
//! Ported from pact-reference's `pact_ffi` at `1f1c2e61` (`src/mock_server/form_urlencoded.rs`:
//! `process_form_urlencoded_json`, `convert_json_value_to_query_params`,
//! `handle_form_urlencoded_invalid_value`). A member a form cannot carry (an object, a boolean, a
//! null) is left out with its rules and generators, as the FFI leaves it out.

use super::integration::{Problems, process_json};
use pact_models::generators::{GeneratorCategory, Generators};
use pact_models::matchingrules::MatchingRuleCategory;
use pact_models::path_exp::DocPath;
use serde_json::Value;

/// The query string a form body written as JSON stands for.
pub fn process_form_urlencoded_json(
  body: &str,
  rules: &mut MatchingRuleCategory,
  generators: &mut Generators,
  problems: &mut Problems,
) -> String {
  let json = process_json(body, rules, generators, problems);
  let Ok(Value::Object(members)) = serde_json::from_str::<Value>(&json) else {
    return String::new();
  };
  let mut params: Vec<(String, String)> = vec![];
  for (key, value) in &members {
    match value {
      Value::Number(number) => params.push((key.clone(), number.to_string())),
      Value::String(text) => params.push((key.clone(), text.clone())),
      Value::Array(values) => {
        for (index, value) in values.iter().enumerate() {
          match value {
            Value::Number(number) => params.push((key.clone(), number.to_string())),
            Value::String(text) => params.push((key.clone(), text.clone())),
            _ => left_out(&DocPath::root().join(key).join_index(index), rules, generators),
          }
        }
      }
      _ => left_out(&DocPath::root().join(key), rules, generators),
    }
  }
  serde_urlencoded::to_string(params).unwrap_or_default()
}

/// A value a form cannot carry: every rule and generator at or below it goes with it.
fn left_out(path: &DocPath, rules: &mut MatchingRuleCategory, generators: &mut Generators) {
  let prefix = String::from(path);
  let keys: Vec<DocPath> = rules
    .rules
    .keys()
    .filter(|key| String::from(*key).contains(&prefix))
    .cloned()
    .collect();
  for key in keys {
    rules.rules.remove(&key);
    if let Some(body) = generators.categories.get_mut(&GeneratorCategory::BODY) {
      body.remove(&key);
    }
  }
}
