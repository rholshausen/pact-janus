//! `<component>:compare-contents` (ADR 0106 decision 4, ADR 0109): a plugin's content matcher, applied to
//! a whole slot's octets.
//!
//! The value under test is the actual body. The action's one argument is an object saying what a pact
//! says about that body: its `content-type` as the pact records it, parameters and all (which selects the
//! plugin's content matcher, and from which the protobuf plugin reads the message type), the `expected`
//! body, the pact's `rules` for it in the v4 pact form, whether unexpected keys are allowed, and the
//! plugin's own `plugin-configuration` for the interaction and the pact. Everything the plugin decides
//! is the plugin's: the bridge builds the request as the driver's `ContentMatcher::match_contents` builds
//! it for today's verifier, and reads every answer the plugin gives as that call reads it ([`mismatches`]).
//!
//! Where the bridge parts from that call is a plugin it cannot reach (ADR 0109 decision 6): the driver
//! reports that as one more mismatch, which would blame the provider for a dead plugin. The bridge reports
//! it as its own failure ([`unreachable`]), and leaves, so the engine starts it again.

use crate::error::ComponentError;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use pact_models::bodies::OptionalBody;
use pact_models::content_types::ContentType;
use pact_models::matchingrules::MatchingRuleCategory;
use pact_plugin_driver::plugin_models::PluginInteractionConfig;
use pact_plugin_driver::proto::{self, body::ContentTypeHint};
use pact_plugin_driver::utils::to_proto_struct;
use serde_json::{Map, Value, json};
use std::collections::HashMap;

/// What one `compare-contents` application compares against, read from its argument.
#[derive(Debug, Clone)]
pub struct Arguments {
  pub content_type: String,
  pub actual_content_type: Option<String>,
  pub expected: OptionalBody,
  pub rules: MatchingRuleCategory,
  pub allow_unexpected_keys: bool,
  pub plugin_configuration: Option<PluginInteractionConfig>,
}

impl Arguments {
  /// Reads the argument from an application's `config`, whose `arguments` carry the action node's
  /// children after the first (spec §7.2): for this action, one object.
  pub fn read(config: Option<&Value>) -> Result<Arguments, ComponentError> {
    let argument = config
      .and_then(|config| config.get("arguments"))
      .and_then(Value::as_array)
      .and_then(|arguments| arguments.first())
      .ok_or_else(|| {
        ComponentError::invalid_config(
          "compare-contents takes one argument, an object naming the expected body's content-type",
        )
      })?;
    let argument = argument
      .as_object()
      .ok_or_else(|| ComponentError::invalid_config("compare-contents' argument must be an object"))?;
    let content_type = string(argument, "content-type")?
      .ok_or_else(|| ComponentError::invalid_config("compare-contents' argument names no 'content-type'"))?;
    let parsed = parse_content_type(&content_type)?;
    let actual_content_type = string(argument, "actual-content-type")?;
    if let Some(actual) = &actual_content_type {
      parse_content_type(actual)?;
    }
    let expected = match argument.get("expected") {
      None | Some(Value::Null) => OptionalBody::Missing,
      Some(expected) => {
        OptionalBody::Present(octets(expected, "the expected body")?.into(), Some(parsed), None)
      }
    };
    let mut rules = MatchingRuleCategory::empty("body");
    match argument.get("rules") {
      None | Some(Value::Null) => {}
      Some(Value::Object(_)) => {
        rules.add_rules_from_json(&argument["rules"]).map_err(|err| {
          ComponentError::invalid_config("compare-contents' 'rules' are not a pact's body rules")
            .caused_by(&err)
        })?;
      }
      Some(_) => {
        return Err(ComponentError::invalid_config(
          "compare-contents' 'rules' must be an object of a pact's body rules, by path",
        ));
      }
    }
    let allow_unexpected_keys = match argument.get("allow-unexpected-keys") {
      None | Some(Value::Null) => false,
      Some(Value::Bool(allow)) => *allow,
      Some(_) => {
        return Err(ComponentError::invalid_config(
          "compare-contents' 'allow-unexpected-keys' must be a boolean",
        ));
      }
    };
    let plugin_configuration = match argument.get("plugin-configuration") {
      None | Some(Value::Null) => None,
      Some(Value::Object(configuration)) => Some(PluginInteractionConfig {
        interaction_configuration: members(configuration, "interaction")?,
        pact_configuration: members(configuration, "pact")?,
      }),
      Some(_) => {
        return Err(ComponentError::invalid_config(
          "compare-contents' 'plugin-configuration' must be an object with 'interaction' and 'pact'",
        ));
      }
    };
    Ok(Arguments {
      content_type,
      actual_content_type,
      expected,
      rules,
      allow_unexpected_keys,
      plugin_configuration,
    })
  }

  /// The value under test as the body a plugin compares: always present, so a plugin is never handed a
  /// body with no content. Its content type is `actual-content-type`, or the expected body's.
  pub fn actual(&self, value: &Value) -> Result<OptionalBody, ComponentError> {
    let content_type = self.actual_content_type.as_deref().unwrap_or(&self.content_type);
    let content_type = parse_content_type(content_type)?;
    Ok(OptionalBody::Present(
      octets(value, "the value under test")?.into(),
      Some(content_type),
      None,
    ))
  }

  /// The plugin's `CompareContents` request for one actual body, built as `match_contents` builds it.
  pub fn request(&self, actual: &OptionalBody) -> proto::CompareContentsRequest {
    let body = |body: &OptionalBody| proto::Body {
      content_type: body.content_type().unwrap_or_default().to_string(),
      content: body.value().map(|bytes| bytes.to_vec()),
      content_type_hint: ContentTypeHint::Default as i32,
    };
    proto::CompareContentsRequest {
      expected: Some(body(&self.expected)),
      actual: Some(body(actual)),
      allow_unexpected_keys: self.allow_unexpected_keys,
      rules: self
        .rules
        .rules
        .iter()
        .map(|(path, rules)| {
          (
            path.to_string(),
            proto::MatchingRules {
              rule: rules
                .rules
                .iter()
                .map(|rule| proto::MatchingRule {
                  r#type: rule.name(),
                  values: Some(to_proto_struct(&rule.value_map())),
                })
                .collect(),
            },
          )
        })
        .collect(),
      plugin_configuration: self.plugin_configuration.as_ref().map(|configuration| {
        proto::PluginConfiguration {
          interaction_configuration: Some(to_proto_struct(&configuration.interaction_configuration)),
          pact_configuration: Some(to_proto_struct(&configuration.pact_configuration)),
        }
      }),
    }
  }
}

/// One mismatch the plugin reported, as `match_contents` reads it.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Mismatch {
  pub path: String,
  pub message: String,
  pub expected: String,
  pub actual: String,
  pub diff: String,
  pub kind: String,
}

impl Mismatch {
  fn said(message: String) -> Mismatch {
    Mismatch {
      message,
      ..Mismatch::default()
    }
  }
}

/// What the plugin answered, as the mismatches `match_contents` makes of it for today's verifier, in the
/// same order of precedence: a content-type mismatch, then the plugin's own error, then its results, by
/// path. A call the plugin answered with a failure is a mismatch too, in the driver's words; only a plugin
/// that could not be reached is not, and the caller has already taken that out ([`unreachable`]).
pub fn mismatches(answer: Result<proto::CompareContentsResponse, anyhow::Error>) -> Vec<Mismatch> {
  let response = match answer {
    Ok(response) => response,
    Err(err) => {
      return vec![Mismatch::said(format!(
        "Call to compare contents handler failed = {err}"
      ))];
    }
  };
  if let Some(mismatch) = response.type_mismatch {
    return vec![Mismatch {
      message: format!(
        "Expected content type '{}' but got '{}'",
        mismatch.expected, mismatch.actual
      ),
      expected: mismatch.expected,
      actual: mismatch.actual,
      ..Mismatch::default()
    }];
  }
  if !response.error.is_empty() {
    return vec![Mismatch::said(response.error)];
  }
  let mut keys: Vec<&String> = response.results.keys().collect();
  keys.sort();
  let text = |bytes: &Option<Vec<u8>>| {
    bytes
      .as_deref()
      .and_then(|bytes| std::str::from_utf8(bytes).ok())
      .unwrap_or_default()
      .to_string()
  };
  keys
    .into_iter()
    .flat_map(|key| response.results[key].mismatches.iter())
    .map(|mismatch| Mismatch {
      path: mismatch.path.clone(),
      message: mismatch.mismatch.clone(),
      expected: text(&mismatch.expected),
      actual: text(&mismatch.actual),
      diff: mismatch.diff.clone(),
      kind: mismatch.mismatch_type.clone(),
    })
    .collect()
}

/// Whether a failed call never reached the plugin (ADR 0109 decision 6): the driver could not connect,
/// or gRPC says the plugin is unavailable, the call was cancelled, or its deadline passed. That is the
/// bridge failing, not the provider's body, and it is answered as `unavailable`. Anything else the plugin
/// answered with, a gRPC status of its own (the CSV plugin's `Aborted` for a body it cannot read) or a
/// Lua plugin's script error, is the plugin's answer, and stays a mismatch as it is for today's verifier.
pub fn unreachable(err: &anyhow::Error) -> bool {
  use tonic::Code;
  err.chain().any(|cause| {
    cause.downcast_ref::<tonic::transport::Error>().is_some()
      || cause.downcast_ref::<tonic::Status>().is_some_and(|status| {
        matches!(
          status.code(),
          Code::Unavailable | Code::Cancelled | Code::DeadlineExceeded
        )
      })
  })
}

/// A `MatchValue`'s content as octets (spec §7.2): `base64` decoded, `text` as UTF-8, and a `json`
/// value as the text of a string, or the JSON text of anything else. A JSON `null` is no octets.
pub fn octets(value: &Value, what: &str) -> Result<Vec<u8>, ComponentError> {
  let content = value
    .get("content")
    .ok_or_else(|| ComponentError::invalid_config(format!("{what} has no 'content'")))?;
  match value.get("encoded").and_then(Value::as_str).unwrap_or("json") {
    "base64" => {
      let text = content.as_str().ok_or_else(|| {
        ComponentError::invalid_config(format!(
          "{what} is encoded 'base64' and its content is not a string"
        ))
      })?;
      BASE64
        .decode(text)
        .map_err(|err| ComponentError::invalid_config(format!("{what} is not base64: {err}")))
    }
    "text" => content
      .as_str()
      .map(|text| text.as_bytes().to_vec())
      .ok_or_else(|| {
        ComponentError::invalid_config(format!(
          "{what} is encoded 'text' and its content is not a string"
        ))
      }),
    "json" => Ok(match content {
      Value::Null => Vec::new(),
      Value::String(text) => text.as_bytes().to_vec(),
      other => other.to_string().into_bytes(),
    }),
    other => Err(ComponentError::invalid_config(format!(
      "{what} is encoded '{other}', which is not 'json', 'text' or 'base64'"
    ))),
  }
}

/// The mismatches as a plan result document (plan-grammar spec §2.3). None is `ok`. Mismatches are one
/// `error`, whose message names each, prefixed by where the plugin located it, and whose `mismatches`
/// keep each one whole, in the order of the plugin's keys, located `at` where the plugin said. The engine
/// reports each as a mismatch of its own (ADR 0110).
pub fn result(mismatches: &[Mismatch]) -> Value {
  if mismatches.is_empty() {
    return json!({ "status": "ok" });
  }
  let message = mismatches
    .iter()
    .map(|mismatch| {
      if mismatch.path.is_empty() {
        mismatch.message.clone()
      } else {
        format!("{}: {}", mismatch.path, mismatch.message)
      }
    })
    .collect::<Vec<_>>()
    .join("; ");
  let described: Vec<Value> = mismatches
    .iter()
    .map(|mismatch| {
      // Located `at`, in the plugin's words: a result's `path` is an absolute path (ADR 0110 decision 2).
      let mut described = Map::new();
      described.insert("message".to_string(), json!(mismatch.message));
      for (key, value) in [
        ("at", &mismatch.path),
        ("expected", &mismatch.expected),
        ("actual", &mismatch.actual),
        ("diff", &mismatch.diff),
        ("type", &mismatch.kind),
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

fn string(argument: &Map<String, Value>, key: &str) -> Result<Option<String>, ComponentError> {
  match argument.get(key) {
    None | Some(Value::Null) => Ok(None),
    Some(Value::String(text)) => Ok(Some(text.clone())),
    Some(_) => Err(ComponentError::invalid_config(format!(
      "compare-contents' '{key}' must be a string"
    ))),
  }
}

fn members(configuration: &Map<String, Value>, key: &str) -> Result<HashMap<String, Value>, ComponentError> {
  match configuration.get(key) {
    None | Some(Value::Null) => Ok(HashMap::new()),
    Some(Value::Object(members)) => Ok(members.clone().into_iter().collect()),
    Some(_) => Err(ComponentError::invalid_config(format!(
      "compare-contents' 'plugin-configuration.{key}' must be an object"
    ))),
  }
}

fn parse_content_type(text: &str) -> Result<ContentType, ComponentError> {
  ContentType::parse(text).map_err(|err| {
    ComponentError::invalid_config(format!("'{text}' is not a content type: {err}"))
      .with("content-type", text)
  })
}

#[cfg(test)]
mod tests {
  use super::*;
  use pretty_assertions::assert_eq;

  fn read(argument: Value) -> Result<Arguments, ComponentError> {
    Arguments::read(Some(&json!({ "arguments": [argument] })))
  }

  fn reported(path: &str, message: &str) -> proto::ContentMismatch {
    proto::ContentMismatch {
      expected: Some(b"1".to_vec()),
      actual: None,
      mismatch: message.to_string(),
      path: path.to_string(),
      diff: String::new(),
      mismatch_type: "body".to_string(),
    }
  }

  #[test]
  fn the_argument_says_what_the_pact_says_about_the_body() {
    let arguments = read(json!({
      "content-type": "text/csv;charset=utf-8",
      "expected": { "content": "Name,100\n", "encoded": "text" },
      "rules": { "column:2": { "combine": "AND", "matchers": [{ "match": "number" }] } },
      "allow-unexpected-keys": true,
      "plugin-configuration": { "interaction": { "csvHeaders": false }, "pact": {} }
    }))
    .unwrap();
    assert_eq!(arguments.content_type, "text/csv;charset=utf-8");
    assert_eq!(arguments.expected.value().unwrap().as_ref(), b"Name,100\n");
    assert!(arguments.allow_unexpected_keys);
    // The rule's path is handed to the plugin as the pact wrote it.
    let paths: Vec<String> = arguments
      .rules
      .rules
      .keys()
      .map(|path| path.to_string())
      .collect();
    assert_eq!(paths, vec!["column:2".to_string()]);
    let configuration = arguments.plugin_configuration.unwrap();
    assert_eq!(
      configuration.interaction_configuration["csvHeaders"],
      json!(false)
    );
  }

  #[test]
  fn with_nothing_else_said_there_is_no_expected_body_no_rule_and_no_configuration() {
    let arguments = read(json!({ "content-type": "application/protobuf" })).unwrap();
    assert_eq!(arguments.expected, OptionalBody::Missing);
    assert!(arguments.rules.is_empty());
    assert!(!arguments.allow_unexpected_keys);
    assert!(arguments.plugin_configuration.is_none());
  }

  #[test]
  fn an_argument_that_cannot_be_read_is_invalid_config() {
    for argument in [
      json!("text/csv"),
      json!({}),
      json!({ "content-type": 1 }),
      json!({ "content-type": "text/csv", "expected": { "content": "!", "encoded": "base64" } }),
      json!({ "content-type": "text/csv", "rules": [] }),
      json!({ "content-type": "text/csv", "allow-unexpected-keys": "yes" }),
      json!({ "content-type": "text/csv", "plugin-configuration": { "pact": [] } }),
    ] {
      let err = read(argument.clone()).expect_err(&argument.to_string());
      assert_eq!(err.code, "invalid-config", "{argument}");
    }
    assert_eq!(Arguments::read(None).unwrap_err().code, "invalid-config");
  }

  #[test]
  fn a_match_value_is_octets_however_it_is_encoded() {
    assert_eq!(
      octets(&json!({ "content": "AAE=", "encoded": "base64" }), "v").unwrap(),
      vec![0, 1]
    );
    assert_eq!(
      octets(&json!({ "content": "a,b", "encoded": "text" }), "v").unwrap(),
      b"a,b"
    );
    assert_eq!(octets(&json!({ "content": "a,b" }), "v").unwrap(), b"a,b");
    assert_eq!(
      octets(&json!({ "content": { "a": 1 } }), "v").unwrap(),
      br#"{"a":1}"#
    );
    assert_eq!(octets(&json!({ "content": null }), "v").unwrap(), b"");
  }

  #[test]
  fn the_actual_body_takes_the_expected_content_type_unless_told_another() {
    let arguments = read(json!({ "content-type": "text/csv" })).unwrap();
    let actual = arguments.actual(&json!({ "content": "" })).unwrap();
    assert_eq!(actual.content_type().unwrap().to_string(), "text/csv");
    assert!(
      actual.value().is_some(),
      "an empty body is still handed to the plugin as present"
    );
    let arguments = read(json!({ "content-type": "text/csv", "actual-content-type": "text/plain" })).unwrap();
    let actual = arguments.actual(&json!({ "content": "x" })).unwrap();
    assert_eq!(actual.content_type().unwrap().to_string(), "text/plain");
  }

  #[test]
  fn the_request_is_what_match_contents_would_send() {
    let arguments = read(json!({
      "content-type": "text/csv",
      "expected": { "content": "a\n", "encoded": "text" },
      "rules": { "column:1": { "combine": "AND", "matchers": [{ "match": "regex", "regex": "\\d+" }] } },
      "allow-unexpected-keys": true,
      "plugin-configuration": { "interaction": { "csvHeaders": false } }
    }))
    .unwrap();
    let request = arguments.request(&arguments.actual(&json!({ "content": "1\n" })).unwrap());
    assert_eq!(request.expected.as_ref().unwrap().content_type, "text/csv");
    assert_eq!(
      request.expected.as_ref().unwrap().content.as_deref(),
      Some(&b"a\n"[..])
    );
    assert_eq!(
      request.actual.as_ref().unwrap().content.as_deref(),
      Some(&b"1\n"[..])
    );
    assert!(request.allow_unexpected_keys);
    let rule = &request.rules["column:1"].rule[0];
    assert_eq!(rule.r#type, "regex");
    let configuration = request.plugin_configuration.unwrap();
    assert!(
      configuration
        .interaction_configuration
        .unwrap()
        .fields
        .contains_key("csvHeaders")
    );
    assert!(configuration.pact_configuration.unwrap().fields.is_empty());

    let missing = read(json!({ "content-type": "text/csv" })).unwrap();
    let request = missing.request(&missing.actual(&json!({ "content": "" })).unwrap());
    let expected = request.expected.unwrap();
    // As the driver sends a pact's missing body: no content, and a default content type (`*/*`).
    assert_eq!(expected.content, None);
    assert_eq!(
      expected.content_type,
      OptionalBody::Missing
        .content_type()
        .unwrap_or_default()
        .to_string()
    );
    assert!(request.plugin_configuration.is_none());
  }

  #[test]
  fn every_answer_the_plugin_gives_is_read_as_match_contents_reads_it() {
    let response = |response: proto::CompareContentsResponse| mismatches(Ok(response));
    assert_eq!(response(proto::CompareContentsResponse::default()), vec![]);
    assert_eq!(
      response(proto::CompareContentsResponse {
        type_mismatch: Some(proto::ContentTypeMismatch {
          expected: "text/csv".into(),
          actual: "text/plain".into()
        }),
        error: "ignored, as the driver ignores it".into(),
        ..Default::default()
      }),
      vec![Mismatch {
        message: "Expected content type 'text/csv' but got 'text/plain'".into(),
        expected: "text/csv".into(),
        actual: "text/plain".into(),
        ..Default::default()
      }]
    );
    assert_eq!(
      response(proto::CompareContentsResponse {
        error: "no descriptor".into(),
        ..Default::default()
      }),
      vec![Mismatch::said("no descriptor".into())]
    );
    let answered = mismatches(Err(anyhow::Error::from(tonic::Status::aborted(
      "Failed to compare CSV contents",
    ))));
    assert_eq!(answered.len(), 1);
    assert!(
      answered[0]
        .message
        .starts_with("Call to compare contents handler failed = "),
      "{answered:?}"
    );
  }

  #[test]
  fn only_a_plugin_that_could_not_be_reached_is_unreachable() {
    for code in [
      tonic::Code::Unavailable,
      tonic::Code::Cancelled,
      tonic::Code::DeadlineExceeded,
    ] {
      let err = anyhow::Error::from(tonic::Status::new(code, "gone"));
      assert!(unreachable(&err), "{code:?}");
      assert!(
        unreachable(&err.context("calling the plugin")),
        "{code:?} beneath a context"
      );
    }
    for code in [
      tonic::Code::Aborted,
      tonic::Code::Internal,
      tonic::Code::InvalidArgument,
      tonic::Code::Unknown,
    ] {
      assert!(
        !unreachable(&anyhow::Error::from(tonic::Status::new(code, "answered"))),
        "{code:?}"
      );
    }
    assert!(!unreachable(&anyhow::anyhow!(
      "Lua match_contents() function failed - boom"
    )));
  }

  #[test]
  fn no_mismatch_is_ok() {
    assert_eq!(result(&[]), json!({ "status": "ok" }));
  }

  #[test]
  fn mismatches_are_one_error_naming_each_in_path_order() {
    let found = mismatches(Ok(proto::CompareContentsResponse {
      results: HashMap::from([
        (
          "$.b".to_string(),
          proto::ContentMismatches {
            mismatches: vec![reported("$.b", "b differs")],
          },
        ),
        (
          "$.a".to_string(),
          proto::ContentMismatches {
            mismatches: vec![reported("$.a", "a differs"), reported("", "and so does the type")],
          },
        ),
      ]),
      ..Default::default()
    }));
    assert_eq!(
      result(&found),
      json!({
        "status": "error",
        "message": "$.a: a differs; and so does the type; $.b: b differs",
        "mismatches": [
          { "message": "a differs", "at": "$.a", "expected": "1", "type": "body" },
          { "message": "and so does the type", "expected": "1", "type": "body" },
          { "message": "b differs", "at": "$.b", "expected": "1", "type": "body" }
        ]
      })
    );
  }
}
