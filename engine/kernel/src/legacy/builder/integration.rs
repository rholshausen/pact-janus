//! The FFI's *integration JSON* (ADR 0066 decision 2): a value with matchers and generators
//! embedded in it as `{ "pact:matcher:type": …, "value": … }`, which a consumer library hands the
//! FFI and the FFI turns into the value, matching rules and generators a pact records.
//!
//! Ported from pact-reference's `pact_ffi` at `1f1c2e61`: `process_array`, `process_object`,
//! `process_matcher`, `add_each_value_rule_matchers` and `matchers_from_integration_json` from
//! `src/mock_server/bodies.rs`, and `from_integration_json` and `from_integration_json_v2` from
//! `src/mock_server/handles.rs`. The logic is the FFI's, step for step, with one difference: a
//! matcher the FFI cannot read is logged there and written as `null`, and here it is a problem the
//! step is refused with, since a consumer test that recorded `null` would pass against a contract
//! nobody wrote.

use either::Either;
use pact_models::generators::{Generator, GeneratorCategory, Generators};
use pact_models::json_utils::json_to_string;
use pact_models::matchingrules::expressions::{is_matcher_def, parse_matcher_def};
use pact_models::matchingrules::{Category, MatchingRule, MatchingRuleCategory, MatchingRules, RuleLogic};
use pact_models::path_exp::DocPath;
use serde_json::{Map, Value};

/// What could not be read, in the order it was found. The step that carried it is refused.
pub type Problems = Vec<String>;

/// An array with embedded matchers: each element's path is its index, or `[*]` under a type matcher.
pub fn process_array(
  array: &[Value],
  rules: &mut MatchingRuleCategory,
  generators: &mut Generators,
  path: DocPath,
  type_matcher: bool,
  skip_matchers: bool,
  problems: &mut Problems,
) -> Value {
  Value::Array(
    array
      .iter()
      .enumerate()
      .map(|(index, value)| {
        let mut item_path = path.clone();
        if type_matcher {
          item_path.push_star_index();
        } else {
          item_path.push_index(index);
        }
        match value {
          Value::Object(map) => process_object(map, rules, generators, item_path, skip_matchers, problems),
          Value::Array(array) => process_array(
            array,
            rules,
            generators,
            item_path,
            false,
            skip_matchers,
            problems,
          ),
          _ => value.clone(),
        }
      })
      .collect(),
  )
}

/// An object with embedded matchers: a matcher itself, or a map whose members are walked. A
/// member named `pact:…` is the FFI's and never part of the value.
pub fn process_object(
  object: &Map<String, Value>,
  rules: &mut MatchingRuleCategory,
  generators: &mut Generators,
  path: DocPath,
  type_matcher: bool,
  problems: &mut Problems,
) -> Value {
  if let Some(matcher_type) = object.get("pact:matcher:type") {
    return process_matcher(
      object,
      rules,
      generators,
      &path,
      type_matcher,
      matcher_type,
      problems,
    );
  }
  Value::Object(
    object
      .iter()
      .filter(|(key, _)| !key.starts_with("pact:"))
      .map(|(key, value)| {
        let item_path = if type_matcher {
          path.join("*")
        } else {
          path.join(key)
        };
        let value = match value {
          Value::Object(map) => process_object(map, rules, generators, item_path, false, problems),
          Value::Array(array) => process_array(array, rules, generators, item_path, false, false, problems),
          _ => value.clone(),
        };
        (key.clone(), value)
      })
      .collect(),
  )
}

/// One matcher: its rules and generator at `path`, and its example as the value.
fn process_matcher(
  object: &Map<String, Value>,
  rules: &mut MatchingRuleCategory,
  generators: &mut Generators,
  path: &DocPath,
  skip_matchers: bool,
  matcher_type: &Value,
  problems: &mut Problems,
) -> Value {
  let is_array_contains =
    matches!(matcher_type, Value::String(s) if s == "arrayContains" || s == "array-contains");

  let result = if is_array_contains {
    match object.get("variants") {
      Some(Value::Array(variants)) => {
        let mut examples = vec![];
        let variants = variants
          .iter()
          .enumerate()
          .map(|(index, variant)| {
            let mut category = MatchingRuleCategory::empty("body");
            let mut variant_generators = Generators::default();
            let value = match variant {
              Value::Object(map) => process_object(
                map,
                &mut category,
                &mut variant_generators,
                DocPath::root(),
                false,
                problems,
              ),
              Value::Array(array) => process_array(
                array,
                &mut category,
                &mut variant_generators,
                DocPath::root(),
                false,
                false,
                problems,
              ),
              _ => variant.clone(),
            };
            examples.push(value);
            let body = variant_generators
              .categories
              .get(&GeneratorCategory::BODY)
              .cloned()
              .unwrap_or_default();
            (index, category, body)
          })
          .collect();
        Ok((
          vec![MatchingRule::ArrayContains(variants)],
          Value::Array(examples),
        ))
      }
      _ => Err("an arrayContains matcher's 'variants' is missing or not an array".to_string()),
    }
  } else {
    matchers_from_integration_json(object)
      .map(|(matched, generator)| {
        let has_values_matcher = matched.iter().any(MatchingRule::is_values_matcher);
        let value = match object.get("value") {
          Some(Value::Object(map)) => {
            process_object(map, rules, generators, path.clone(), has_values_matcher, problems)
          }
          Some(Value::Array(array)) => process_array(
            array,
            rules,
            generators,
            path.clone(),
            true,
            skip_matchers,
            problems,
          ),
          Some(inner) => inner.clone(),
          None => Value::Null,
        };
        if let Some(generator) = generator {
          let category = generator_category(rules);
          generators.add_generator_with_subcategory(category, path.clone(), generator);
        }
        if matched
          .iter()
          .any(|rule| matches!(rule, MatchingRule::EachValue(_)))
        {
          add_each_value_rule_matchers(object, rules, path, problems);
        }
        (matched, value)
      })
      .map_err(|err| err.to_string())
  };

  if let Some(generator_type) = object.get("pact:generator:type")
    && let Some(generator) = Generator::from_map(&json_to_string(generator_type), object)
  {
    let category = generator_category(rules);
    generators.add_generator_with_subcategory(category, path.clone(), generator);
  }

  match result {
    Ok((matched, value)) => {
      for rule in matched {
        rules.add_rule(path.clone(), rule, RuleLogic::And);
      }
      value
    }
    Err(message) => {
      problems.push(format!("the matcher at '{path}' cannot be read: {message}"));
      Value::Null
    }
  }
}

/// The rules an `eachValue` matcher gives every value only hold the rule itself, so the matchers
/// nested in their examples are added for the values, at `path.*`.
fn add_each_value_rule_matchers(
  object: &Map<String, Value>,
  rules: &mut MatchingRuleCategory,
  path: &DocPath,
  problems: &mut Problems,
) {
  let Some(Value::Array(definitions)) = object.get("rules") else {
    return;
  };
  let values_path = path.join("*");
  let mut generators = Generators::default();
  for rule in definitions.iter().filter_map(Value::as_object) {
    let rule_type = rule.get("pact:matcher:type").and_then(Value::as_str);
    let is_each_value = matches!(rule_type, Some("eachValue" | "each-value"));
    match rule.get("value") {
      Some(Value::Object(map)) => {
        let is_values_matcher = is_each_value || rule_type == Some("values");
        process_object(
          map,
          rules,
          &mut generators,
          values_path.clone(),
          is_values_matcher,
          problems,
        );
      }
      Some(Value::Array(array)) => {
        process_array(
          array,
          rules,
          &mut generators,
          values_path.clone(),
          true,
          false,
          problems,
        );
      }
      _ => {}
    }
    if is_each_value {
      add_each_value_rule_matchers(rule, rules, &values_path, problems);
    }
  }
}

/// The rules (and the generator) one matcher declares: one type, a list of matchers, or a
/// matching-rule expression such as `matching(type, 'x')`.
pub fn matchers_from_integration_json(
  matcher: &Map<String, Value>,
) -> anyhow::Result<(Vec<MatchingRule>, Option<Generator>)> {
  let Some(value) = matcher.get("pact:matcher:type") else {
    return Ok((vec![], None));
  };
  if let Value::Array(list) = value {
    let mut rules = vec![];
    for entry in list {
      let Some(rule_type) = entry.get("pact:matcher:type") else {
        anyhow::bail!("an entry of a list of matchers has no 'pact:matcher:type'");
      };
      rules.push(MatchingRule::create_from_integration_json(
        json_to_string(rule_type).as_str(),
        entry,
      )?);
    }
    return Ok((rules, None));
  }
  let rule_type = json_to_string(value);
  if !matches!(
    rule_type.as_str(),
    "eachKey" | "eachValue" | "notEmpty" | "arrayContains"
  ) && is_matcher_def(rule_type.as_str())
  {
    let definition = parse_matcher_def(rule_type.as_str())?;
    let mut rules = vec![];
    for rule in definition.rules {
      match rule {
        Either::Left(rule) => rules.push(rule),
        Either::Right(reference) => {
          if matcher.contains_key(reference.name.as_str()) {
            rules.push(MatchingRule::Type);
          } else {
            anyhow::bail!(
              "the matching rule expression refers to '{}', which it does not define",
              reference.name
            );
          }
        }
      }
    }
    return Ok((rules, definition.generator));
  }
  Ok((
    vec![MatchingRule::create_from_integration_json(
      rule_type.as_str(),
      &Value::Object(matcher.clone()),
    )?],
    None,
  ))
}

/// A JSON body's text with its matchers processed into `rules` and `generators`; text that is not
/// JSON, or JSON that is neither an object nor an array, is the body as given.
pub fn process_json(
  body: &str,
  rules: &mut MatchingRuleCategory,
  generators: &mut Generators,
  problems: &mut Problems,
) -> String {
  match serde_json::from_str::<Value>(body) {
    Ok(Value::Object(map)) => {
      process_object(&map, rules, generators, DocPath::root(), false, problems).to_string()
    }
    Ok(Value::Array(array)) => {
      process_array(&array, rules, generators, DocPath::root(), false, false, problems).to_string()
    }
    _ => body.to_string(),
  }
}

/// A query value as `pactffi_with_query_parameter` reads one (`from_integration_json`): an object is
/// processed as a body is, anything else is the text as given.
pub fn from_integration_json(
  rules: &mut MatchingRules,
  generators: &mut Generators,
  value: &str,
  path: DocPath,
  category: &str,
  problems: &mut Problems,
) -> String {
  let category = rules.add_category(category);
  match serde_json::from_str::<Value>(value) {
    // These are simple JSON primitives, so they are unescaped.
    Ok(Value::Object(map)) => {
      json_to_string(&process_object(&map, category, generators, path, false, problems))
    }
    _ => value.to_string(),
  }
}

/// A path, header or status value as the FFI's `_v2` calls read one (`from_integration_json_v2`):
/// a matcher puts its rules and generator at `path` (at its parent for a path or a status, whose
/// rules sit at the category's root) and stands for its example, which may be a list of values.
pub fn from_integration_json_v2(
  rules: &mut MatchingRules,
  generators: &mut Generators,
  value: &str,
  path: DocPath,
  category: &str,
  index: usize,
  problems: &mut Problems,
) -> Either<String, Vec<String>> {
  let rules = rules.add_category(category);
  let path_or_status = [Category::PATH, Category::STATUS].contains(&rules.name);
  let query_or_header = [Category::QUERY, Category::HEADER].contains(&rules.name);

  let Ok(Value::Object(map)) = serde_json::from_str::<Value>(value) else {
    return Either::Left(value.to_string());
  };
  let result = if map.contains_key("pact:matcher:type") {
    let matched = matchers_from_integration_json(&map);
    let example = match map.get("value") {
      Some(Value::Array(array)) => {
        process_array(array, rules, generators, path.clone(), true, false, problems)
      }
      Some(value) => value.clone(),
      None => Value::Null,
    };
    let at = if path_or_status {
      path.parent().unwrap_or(DocPath::root())
    } else {
      path.clone()
    };
    match matched {
      Ok((matched, generator)) => {
        if query_or_header && index > 0 {
          // A later value of a name re-keys the name's first rules under index 0.
          let mut parent = at.parent().unwrap_or(DocPath::root());
          if let Some(list) = rules.rules.remove(&parent) {
            rules.rules.insert(parent.push_index(0).clone(), list);
          }
        }
        for rule in matched {
          rules.add_rule(at.clone(), rule, RuleLogic::And);
        }
        if let Some(generator) = generator {
          let category = generator_category(rules);
          generators.add_generator_with_subcategory(category, at.clone(), generator);
        }
      }
      Err(err) => problems.push(format!("the matcher at '{path}' cannot be read: {err}")),
    }
    if let Some(generator_type) = map.get("pact:generator:type")
      && let Some(generator) = Generator::from_map(&json_to_string(generator_type), &map)
    {
      let category = generator_category(rules);
      generators.add_generator_with_subcategory(category, at, generator);
    }
    example
  } else {
    map.get("value").cloned().unwrap_or_default()
  };
  match result {
    Value::Array(values) => Either::Right(values.iter().map(json_to_string).collect()),
    // These are simple JSON primitives, so they are unescaped.
    other => Either::Left(json_to_string(&other)),
  }
}

/// The generator category a rule category's generators go in.
pub fn generator_category(rules: &MatchingRuleCategory) -> &'static GeneratorCategory {
  match rules.name {
    Category::HEADER => &GeneratorCategory::HEADER,
    Category::PATH => &GeneratorCategory::PATH,
    Category::QUERY => &GeneratorCategory::QUERY,
    Category::METADATA => &GeneratorCategory::METADATA,
    Category::STATUS => &GeneratorCategory::STATUS,
    _ => &GeneratorCategory::BODY,
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use pact_models::matchingrules_list;
  use pretty_assertions::assert_eq;
  use serde_json::json;

  // pact_ffi's `process_json_with_nested_rules` (handles.rs).
  #[test]
  fn nested_matchers_put_their_rules_at_their_paths() {
    let body = json!({
      "pact:matcher:type": "type",
      "value": {
        "name": { "pact:matcher:type": "regex", "regex": "\\w+", "value": "Fred" },
        "ids": { "pact:matcher:type": "type", "min": 1, "value": [ { "pact:matcher:type": "integer", "value": 1 } ] }
      }
    })
    .to_string();
    let mut rules = MatchingRuleCategory::empty("body");
    let mut generators = Generators::default();
    let mut problems = Problems::new();

    let processed = process_json(&body, &mut rules, &mut generators, &mut problems);

    assert!(problems.is_empty());
    assert_eq!(
      serde_json::from_str::<Value>(&processed).unwrap(),
      json!({ "name": "Fred", "ids": [1] })
    );
    assert_eq!(
      rules,
      matchingrules_list! {
        "body";
        "$" => [ MatchingRule::Type ],
        "$.name" => [ MatchingRule::Regex("\\w+".to_string()) ],
        "$.ids" => [ MatchingRule::MinType(1) ],
        "$.ids[*]" => [ MatchingRule::Integer ]
      }
    );
  }

  #[test]
  fn a_matcher_that_cannot_be_read_is_a_problem() {
    // A regex matcher names its pattern; one with none cannot be read.
    let body = json!({ "id": { "pact:matcher:type": "regex", "value": "1" } }).to_string();
    let mut rules = MatchingRuleCategory::empty("body");
    let mut problems = Problems::new();
    process_json(&body, &mut rules, &mut Generators::default(), &mut problems);
    assert_eq!(problems.len(), 1);
    assert!(problems[0].contains("$.id"));
  }

  // pact_ffi's `path_with_matcher` and `status_with_matcher` (handles.rs): rules at the root.
  #[test]
  fn a_path_or_status_matcher_puts_its_rules_at_the_root() {
    let mut rules = MatchingRules::default();
    let mut generators = Generators::default();
    let mut problems = Problems::new();
    let path =
      json!({ "pact:matcher:type": "regex", "regex": "/users/\\d+", "value": "/users/1" }).to_string();

    let value = from_integration_json_v2(
      &mut rules,
      &mut generators,
      &path,
      DocPath::empty(),
      "path",
      0,
      &mut problems,
    );

    assert_eq!(value, Either::Left("/users/1".to_string()));
    assert_eq!(
      rules.rules_for_category("path").unwrap(),
      matchingrules_list! {
        "path"; "$" => [ MatchingRule::Regex("/users/\\d+".to_string()) ]
      }
    );
  }

  #[test]
  fn plain_text_and_json_scalars_are_the_text_as_given() {
    let mut rules = MatchingRules::default();
    let mut problems = Problems::new();
    for text in ["application/json", "123", "\"quoted\""] {
      let value = from_integration_json_v2(
        &mut rules,
        &mut Generators::default(),
        text,
        DocPath::root().join("Accept"),
        "header",
        0,
        &mut problems,
      );
      assert_eq!(value, Either::Left(text.to_string()));
    }
  }
}
