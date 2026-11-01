//! A contract or a v1–v4 pact handed to the engine as its text (ADR 0063 decision 3). A host reads
//! the file and sends what it read, so no host parses a document it then writes back changed. A
//! contract and a pact are always objects, so a string where one is expected is never ambiguous.

use super::Problem;
use serde_json::Value;
use std::borrow::Cow;

/// `value` as a document: itself, or the document its text holds. The text is read as a file is,
/// whatever its layout (contract-file spec §2.4). Text that is not JSON is a problem at `pointer`.
pub fn document<'a>(value: &'a Value, pointer: &str) -> Result<Cow<'a, Value>, Problem> {
  match value {
    Value::String(text) => serde_json::from_str(text).map(Cow::Owned).map_err(|err| Problem {
      pointer: pointer.to_string(),
      message: format!("the text is not a JSON document: {err}"),
    }),
    other => Ok(Cow::Borrowed(other)),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use pretty_assertions::assert_eq;
  use serde_json::json;

  #[test]
  fn a_document_is_itself() {
    let value = json!({ "$format": "janus-contract/1" });
    assert!(matches!(document(&value, "/c").unwrap(), Cow::Borrowed(_)));
  }

  #[test]
  fn text_is_read_whatever_its_layout_and_keeps_its_numbers() {
    let value = json!("{\n  \"b\": 9007199254740993,\n  \"a\": 1.0\n}\n");
    let read = document(&value, "/c").unwrap().into_owned();
    assert_eq!(
      serde_json::to_string(&read).unwrap(),
      r#"{"b":9007199254740993,"a":1.0}"#
    );
  }

  #[test]
  fn text_that_is_not_json_is_a_problem_at_the_pointer() {
    let problem = document(&json!("{ not json"), "/source/contracts/2").unwrap_err();
    assert_eq!(problem.pointer, "/source/contracts/2");
    assert!(
      problem.message.starts_with("the text is not a JSON document: "),
      "{}",
      problem.message
    );
  }
}
