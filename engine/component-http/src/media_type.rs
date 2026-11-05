//! `http:media-type` (ADR 0071): a `Content-Type` compared as a media type, as today's mocks and
//! verifiers compare one. The kernel knows nothing of media types (shape spec §3.5), so the component
//! that speaks HTTP says what one is.
//!
//! The operator's one member is `type`, the media type expected, which a plan passes as the action's
//! second argument and so arrives as `config.arguments[0]`.

use pact_janus_kernel::component::{ApplyResult, ComponentError};
use serde_json::{Value, json};

/// The action.
pub const MEDIA_TYPE: &str = "http:media-type";

/// A media type: its type and subtype, lower-cased, and its parameters, their names lower-cased.
#[derive(Debug, PartialEq)]
struct MediaType {
  essence: String,
  parameters: Vec<(String, String)>,
}

impl MediaType {
  /// `type/subtype; name=value; ...`, with whitespace around each part and quotes around a value
  /// ignored. `None` for a value with no `/` in its type.
  fn parse(text: &str) -> Option<MediaType> {
    let mut segments = text.split(';').map(str::trim);
    let essence = segments.next()?.to_ascii_lowercase();
    let (main, sub) = essence.split_once('/')?;
    if main.trim().is_empty() || sub.trim().is_empty() {
      return None;
    }
    let parameters = segments
      .filter_map(|segment| segment.split_once('='))
      .map(|(name, value)| {
        (
          name.trim().to_ascii_lowercase(),
          value.trim().trim_matches('"').to_string(),
        )
      })
      .collect();
    Some(MediaType { essence, parameters })
  }

  /// Whether `actual` is this type: the same type and subtype, and each of this type's parameters
  /// with an equal value, compared case-insensitively. Other parameters, in any order, are allowed.
  fn admits(&self, actual: &MediaType) -> bool {
    self.essence == actual.essence
      && self.parameters.iter().all(|(name, value)| {
        actual
          .parameters
          .iter()
          .any(|(n, v)| n == name && v.eq_ignore_ascii_case(value))
      })
  }
}

fn invalid(message: impl Into<String>) -> ComponentError {
  ComponentError::invalid_config(MEDIA_TYPE, message)
}

/// Runs `http:media-type` over each value.
pub(crate) fn apply(config: Option<&Value>, values: &[Value]) -> Result<ApplyResult, ComponentError> {
  let expected_text = config
    .and_then(|config| config.pointer("/arguments/0/type"))
    .and_then(Value::as_str)
    .ok_or_else(|| invalid("names no 'type'"))?;
  let expected = MediaType::parse(expected_text)
    .ok_or_else(|| invalid(format!("'{expected_text}' is not a media type")))?;
  let results = values
    .iter()
    .map(|value| {
      let actual = value.get("content").unwrap_or(&Value::Null);
      match actual.as_str().and_then(MediaType::parse) {
        Some(found) if expected.admits(&found) => json!({ "status": "ok" }),
        Some(_) => json!({
          "status": "error",
          "message": format!("Expected a media type matching '{expected_text}' but was {actual}"),
        }),
        None => json!({
          "status": "error",
          "message": format!(
            "Expected a media type matching '{expected_text}' but was {actual}, which is not a media type"
          ),
        }),
      }
    })
    .collect();
  Ok(ApplyResult { results })
}

#[cfg(test)]
mod tests {
  use super::*;
  use pretty_assertions::assert_eq;
  use rstest::rstest;

  fn verdict(expected: &str, actual: Value) -> bool {
    let config = json!({ "arguments": [{ "type": expected }] });
    let result = apply(Some(&config), &[json!({ "content": actual })]).expect("applied");
    result.results[0]["status"] == "ok"
  }

  #[rstest]
  #[case::the_same("application/json", "application/json")]
  #[case::a_charset_the_type_does_not_name("application/json", "application/json; charset=UTF-8")]
  #[case::the_type_in_another_case("application/json", "Application/JSON")]
  #[case::its_parameter_in_another_case("application/json; charset=UTF-8", "application/json;charset=utf-8")]
  #[case::its_parameters_in_another_order("text/plain; a=1; b=2", "text/plain; b=2; a=1; c=3")]
  #[case::a_quoted_value("text/plain; charset=utf-8", "text/plain; charset=\"utf-8\"")]
  fn admits(#[case] expected: &str, #[case] actual: &str) {
    assert!(verdict(expected, json!(actual)), "{expected} admits {actual}");
  }

  #[rstest]
  #[case::another_subtype("application/json", "application/hal+json")]
  #[case::another_type("application/json", "text/json")]
  #[case::a_parameter_it_names_missing("application/json; charset=UTF-8", "application/json")]
  #[case::a_parameter_it_names_different(
    "application/json; charset=UTF-8",
    "application/json; charset=ISO-8859-1"
  )]
  #[case::not_a_media_type("application/json", "json")]
  fn refuses(#[case] expected: &str, #[case] actual: &str) {
    assert!(!verdict(expected, json!(actual)), "{expected} refuses {actual}");
  }

  #[test]
  fn a_value_that_is_not_text_is_not_a_media_type() {
    assert!(!verdict("application/json", json!(1)));
  }

  #[test]
  fn a_refusal_says_what_was_expected() {
    let config = json!({ "arguments": [{ "type": "application/json" }] });
    let result = apply(Some(&config), &[json!({ "content": "text/plain" })]).unwrap();
    assert_eq!(
      result.results[0],
      json!({ "status": "error", "message": "Expected a media type matching 'application/json' but was \"text/plain\"" })
    );
  }

  #[test]
  fn a_type_that_is_not_a_media_type_is_invalid_config() {
    let config = json!({ "arguments": [{ "type": "json" }] });
    assert!(apply(Some(&config), &[]).is_err());
    assert!(apply(None, &[]).is_err());
  }
}
