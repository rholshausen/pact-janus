//! `http:list` (ADR 0082): a header compared as the elements of a list, however its field lines frame
//! them. The kernel knows nothing of headers (shape spec §3.5), so the component that speaks HTTP says
//! what a list-valued one is.
//!
//! The operator's one member is `elements`, the elements expected, in order. A plan passes the shape's
//! members as the action's second argument, which arrives as `config.arguments[0]`. The value is the
//! header's field lines, as the transport presents them.

use pact_janus_kernel::component::{ApplyResult, ComponentError};
use serde_json::{Value, json};

/// The action.
pub const LIST: &str = "http:list";

/// The elements of a list-valued field (RFC 9110 §5.6.1): its field lines joined with commas, split
/// at each comma outside a quoted string (§5.5, §5.6.4), whitespace around each element dropped, and
/// empty elements ignored.
pub fn elements<'a>(lines: impl IntoIterator<Item = &'a str>) -> Vec<String> {
  let mut elements = Vec::new();
  for line in lines {
    let mut current = String::new();
    let mut quoted = false;
    let mut escaped = false;
    for c in line.chars() {
      if escaped {
        escaped = false;
      } else if quoted && c == '\\' {
        escaped = true;
      } else if c == '"' {
        quoted = !quoted;
      } else if c == ',' && !quoted {
        push_element(&mut elements, &current);
        current.clear();
        continue;
      }
      current.push(c);
    }
    push_element(&mut elements, &current);
  }
  elements
}

fn push_element(elements: &mut Vec<String>, text: &str) {
  let element = text.trim_matches([' ', '\t']);
  if !element.is_empty() {
    elements.push(element.to_string());
  }
}

fn invalid(message: impl Into<String>) -> ComponentError {
  ComponentError::invalid_config(LIST, message)
}

/// The elements a shape expects. Each must be one element as [`elements`] reads one, or no header
/// could ever match it.
fn expected(config: Option<&Value>) -> Result<Vec<String>, ComponentError> {
  let items = config
    .and_then(|config| config.pointer("/arguments/0/elements"))
    .ok_or_else(|| invalid("names no 'elements'"))?;
  let items = items
    .as_array()
    .ok_or_else(|| invalid(format!("'elements' must be a list of elements, and is {items}")))?;
  items
    .iter()
    .map(|item| match item.as_str() {
      Some(text) if elements([text]) == [text] => Ok(text.to_string()),
      Some(text) => Err(invalid(format!(
        "'{text}' is not one element of a list: an element has no comma outside quotes, and no \
         whitespace around it"
      ))),
      None => Err(invalid(format!("an element must be a string, and {item} is not"))),
    })
    .collect()
}

/// The field lines in a value: a list of them, as the transport presents a header, or one line.
fn lines(value: &Value) -> Option<Vec<&str>> {
  match value {
    Value::Array(lines) => lines.iter().map(Value::as_str).collect(),
    Value::String(line) => Some(vec![line.as_str()]),
    _ => None,
  }
}

/// Runs `http:list` over each value.
pub(crate) fn apply(config: Option<&Value>, values: &[Value]) -> Result<ApplyResult, ComponentError> {
  let expected = expected(config)?;
  let shown = expected.join(", ");
  let results = values
    .iter()
    .map(|value| {
      let actual = value.get("content").unwrap_or(&Value::Null);
      match lines(actual) {
        Some(lines) if elements(lines.iter().copied()) == expected => json!({ "status": "ok" }),
        Some(lines) => json!({
          "status": "error",
          "message": format!(
            "Expected the list '{shown}' but was '{}'",
            elements(lines.iter().copied()).join(", ")
          ),
        }),
        None if actual.is_null() => json!({
          "status": "error",
          "message": format!("Expected the list '{shown}' but the header was absent"),
        }),
        None => json!({
          "status": "error",
          "message": format!("Expected the list '{shown}' but was {actual}, which is not a header's text"),
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

  fn verdict(elements: Value, actual: Value) -> Value {
    let config = json!({ "arguments": [{ "elements": elements }] });
    apply(Some(&config), &[json!({ "content": actual })])
      .expect("applied")
      .results
      .remove(0)
  }

  #[rstest]
  #[case::one_line(json!(["a, b"]))]
  #[case::no_space(json!(["a,b"]))]
  #[case::two_lines(json!(["a", "b"]))]
  #[case::tabs_and_spaces(json!([" a\t,\tb "]))]
  #[case::an_empty_element(json!(["a, , b,"]))]
  #[case::one_line_as_text(json!("a, b"))]
  fn admits_any_framing(#[case] actual: Value) {
    assert_eq!(verdict(json!(["a", "b"]), actual), json!({ "status": "ok" }));
  }

  #[rstest]
  #[case::another_order(json!(["b, a"]))]
  #[case::one_missing(json!(["a"]))]
  #[case::one_more(json!(["a, b, c"]))]
  #[case::another_case(json!(["A, b"]))]
  fn refuses_other_elements(#[case] actual: Value) {
    assert_eq!(verdict(json!(["a", "b"]), actual)["status"], "error");
  }

  #[test]
  fn a_comma_in_a_quoted_string_is_not_a_delimiter() {
    // RFC 9110 §5.5's own example.
    let line = r#""http://example.com/a.html,foo", "http://without-commas.example.com/""#;
    assert_eq!(
      elements([line]),
      vec![
        r#""http://example.com/a.html,foo""#.to_string(),
        r#""http://without-commas.example.com/""#.to_string(),
      ]
    );
    assert_eq!(
      elements([r#""a\",b", c"#]),
      vec![r#""a\",b""#.to_string(), "c".to_string()]
    );
  }

  #[test]
  fn no_elements_is_an_empty_list() {
    assert_eq!(verdict(json!([]), json!([""])), json!({ "status": "ok" }));
    assert_eq!(verdict(json!([]), json!([])), json!({ "status": "ok" }));
  }

  #[test]
  fn a_refusal_says_what_was_expected() {
    assert_eq!(
      verdict(json!(["a", "b"]), json!(["a", "c"])),
      json!({ "status": "error", "message": "Expected the list 'a, b' but was 'a, c'" })
    );
    assert_eq!(
      verdict(json!(["a"]), Value::Null),
      json!({ "status": "error", "message": "Expected the list 'a' but the header was absent" })
    );
  }

  #[rstest]
  #[case::no_elements(json!({}))]
  #[case::not_a_list(json!({ "elements": "a, b" }))]
  #[case::two_elements_in_one(json!({ "elements": ["a, b"] }))]
  #[case::whitespace_around(json!({ "elements": [" a"] }))]
  #[case::empty(json!({ "elements": [""] }))]
  #[case::not_text(json!({ "elements": [1] }))]
  fn elements_that_cannot_match_are_invalid_config(#[case] members: Value) {
    let config = json!({ "arguments": [members] });
    assert!(apply(Some(&config), &[]).is_err());
  }
}
