//! `http:path` (ADR 0086 decision 2): a request path compared against a path template, as OpenAPI and
//! every router write one. The kernel knows nothing of paths (shape spec §3.5), so the component that
//! speaks HTTP says what a template admits.
//!
//! The operator's one member is `template`. Each `{name}` in it matches one or more characters that
//! are not `/`, so one non-empty segment or a non-empty run of one, and every other character matches
//! itself. A path is admitted when it matches the whole template. A plan passes the shape's members
//! as the action's second argument, which arrives as `config.arguments[0]`.

use pact_janus_kernel::component::{ApplyResult, ComponentError};
use serde_json::{Value, json};

/// The action.
pub const PATH: &str = "http:path";

/// One piece of a template: text matched as written, or a parameter.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Piece {
  Literal(String),
  Parameter,
}

/// A template, read into its pieces. Two parameters with nothing between them could split a segment
/// any way at all, so a template that has them is refused rather than matched by a guess.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Template {
  pieces: Vec<Piece>,
}

impl Template {
  pub fn parse(template: &str) -> Result<Template, String> {
    let mut pieces = Vec::new();
    let mut literal = String::new();
    let mut chars = template.chars();
    while let Some(c) = chars.next() {
      match c {
        '{' => {
          let mut name = String::new();
          let mut closed = false;
          for c in chars.by_ref() {
            match c {
              '}' => {
                closed = true;
                break;
              }
              '{' | '/' => return Err(format!("'{template}' has a '{{' that no '}}' closes")),
              c => name.push(c),
            }
          }
          if !closed {
            return Err(format!("'{template}' has a '{{' that no '}}' closes"));
          }
          if name.is_empty() {
            return Err(format!("'{template}' has a parameter with no name"));
          }
          if literal.is_empty() && matches!(pieces.last(), Some(Piece::Parameter)) {
            return Err(format!(
              "'{template}' has two parameters with nothing between them, which no path can be split by"
            ));
          }
          if !literal.is_empty() {
            pieces.push(Piece::Literal(std::mem::take(&mut literal)));
          }
          pieces.push(Piece::Parameter);
        }
        '}' => return Err(format!("'{template}' has a '}}' that no '{{' opens")),
        c => literal.push(c),
      }
    }
    if !literal.is_empty() {
      pieces.push(Piece::Literal(literal));
    }
    Ok(Template { pieces })
  }

  /// Whether the template has a parameter, and so admits more than one path.
  pub fn has_parameters(&self) -> bool {
    self.pieces.contains(&Piece::Parameter)
  }

  /// Whether `path` matches the whole template.
  pub fn admits(&self, path: &str) -> bool {
    matches(&self.pieces, path)
  }
}

fn matches(pieces: &[Piece], path: &str) -> bool {
  match pieces.split_first() {
    None => path.is_empty(),
    Some((Piece::Literal(text), rest)) => path
      .strip_prefix(text.as_str())
      .is_some_and(|remaining| matches(rest, remaining)),
    Some((Piece::Parameter, rest)) => {
      // One or more characters of one segment, then the rest of the template: tried shortest first.
      let segment = path.find('/').unwrap_or(path.len());
      path[..segment]
        .char_indices()
        .skip(1)
        .map(|(at, _)| at)
        .chain(std::iter::once(segment))
        .filter(|at| *at > 0)
        .any(|at| matches(rest, &path[at..]))
    }
  }
}

fn invalid(message: impl Into<String>) -> ComponentError {
  ComponentError::invalid_config(PATH, message)
}

fn template(config: Option<&Value>) -> Result<(String, Template), ComponentError> {
  let template = config
    .and_then(|config| config.pointer("/arguments/0/template"))
    .ok_or_else(|| invalid("names no 'template'"))?;
  let text = template
    .as_str()
    .ok_or_else(|| invalid(format!("'template' must be a path template, and is {template}")))?;
  Template::parse(text)
    .map(|parsed| (text.to_string(), parsed))
    .map_err(invalid)
}

/// Runs `http:path` over each value.
pub(crate) fn apply(config: Option<&Value>, values: &[Value]) -> Result<ApplyResult, ComponentError> {
  let (text, template) = template(config)?;
  let results = values
    .iter()
    .map(|value| match value.get("content").unwrap_or(&Value::Null) {
      Value::String(path) if template.admits(path) => json!({ "status": "ok" }),
      Value::String(path) => json!({
        "status": "error",
        "message": format!("Expected a path matching the template '{text}' but was '{path}'"),
      }),
      Value::Null => json!({
        "status": "error",
        "message": format!("Expected a path matching the template '{text}' but the path was absent"),
      }),
      other => json!({
        "status": "error",
        "message": format!("Expected a path matching the template '{text}' but was {other}, which is not a path"),
      }),
    })
    .collect();
  Ok(ApplyResult { results })
}

#[cfg(test)]
mod tests {
  use super::*;
  use pretty_assertions::assert_eq;
  use rstest::rstest;

  fn verdict(template: &str, actual: Value) -> Value {
    let config = json!({ "arguments": [{ "template": template }] });
    apply(Some(&config), &[json!({ "content": actual })])
      .expect("applied")
      .results
      .remove(0)
  }

  #[rstest]
  #[case::a_literal_path("/orders", "/orders")]
  #[case::one_parameter("/orders/{id}", "/orders/66")]
  #[case::two_parameters("/orders/{id}/items/{item}", "/orders/66/items/sku-1")]
  #[case::inside_a_segment("/files/{name}.{ext}", "/files/report.final.pdf")]
  #[case::a_parameter_last_in_a_segment("/v{version}/orders", "/v2/orders")]
  #[case::the_root("/", "/")]
  fn admits_a_path_that_matches_the_whole_template(#[case] template: &str, #[case] path: &str) {
    assert_eq!(verdict(template, json!(path)), json!({ "status": "ok" }));
  }

  #[rstest]
  #[case::an_empty_segment("/orders/{id}", "/orders/")]
  #[case::two_segments_for_one("/orders/{id}", "/orders/66/items")]
  #[case::a_trailing_slash("/orders/{id}", "/orders/66/")]
  #[case::another_literal("/orders/{id}", "/order/66")]
  #[case::another_case("/orders/{id}", "/Orders/66")]
  #[case::missing_the_rest("/orders/{id}/items", "/orders/66")]
  fn refuses_one_that_does_not(#[case] template: &str, #[case] path: &str) {
    assert_eq!(verdict(template, json!(path))["status"], "error");
  }

  #[test]
  fn a_refusal_says_what_was_expected() {
    assert_eq!(
      verdict("/orders/{id}", json!("/customers/1")),
      json!({ "status": "error",
              "message": "Expected a path matching the template '/orders/{id}' but was '/customers/1'" })
    );
    assert_eq!(
      verdict("/orders/{id}", Value::Null)["message"],
      "Expected a path matching the template '/orders/{id}' but the path was absent"
    );
  }

  #[rstest]
  #[case::no_template(json!({}))]
  #[case::not_text(json!({ "template": 1 }))]
  #[case::unclosed(json!({ "template": "/orders/{id" }))]
  #[case::unopened(json!({ "template": "/orders/id}" }))]
  #[case::unnamed(json!({ "template": "/orders/{}" }))]
  #[case::adjacent(json!({ "template": "/files/{name}{ext}" }))]
  #[case::across_segments(json!({ "template": "/orders/{a/b}" }))]
  fn a_template_that_cannot_match_is_invalid_config(#[case] members: Value) {
    let config = json!({ "arguments": [members] });
    assert!(apply(Some(&config), &[]).is_err());
  }
}
