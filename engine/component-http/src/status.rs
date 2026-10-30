//! The HTTP component's matcher half (component-interfaces spec §7.1, ADR 0057): `http:status-class`,
//! the operator a v4 `statusCode` matcher becomes (contract-file spec §8.2). The kernel knows nothing
//! of statuses (shape spec §3.5), so the component that speaks HTTP is the one that says what a
//! status class is.
//!
//! The operator's members are `class`, one of seven classes by name, or `codes`, the statuses
//! allowed. Each class is the range today's verifier checks (`pact_matching`'s `match_status_code`).
//! A plan passes them as the action's second argument, which arrives as `config.arguments[0]`.

use pact_janus_kernel::component::{Apply, ApplyResult, ComponentError, MatcherComponent};
use serde_json::{Value, json};

/// The component's name, which namespaces what it contributes (spec §2.4).
pub const NAME: &str = "http";

/// The one action it contributes.
pub const STATUS_CLASS: &str = "http:status-class";

/// Every action this component runs, as an embedding registers it.
pub const ACTIONS: &[&str] = &[STATUS_CLASS];

/// Runs `http:status-class`. Holds nothing: a status and its class are all it reads.
#[derive(Debug, Default)]
pub struct HttpMatcher;

impl HttpMatcher {
  pub fn new() -> Self {
    HttpMatcher
  }
}

/// What a status is held to.
#[derive(Debug, PartialEq)]
enum Allowed {
  Range {
    from: u16,
    to: u16,
    phrase: &'static str,
  },
  Codes(Vec<u16>),
}

impl Allowed {
  fn read(config: Option<&Value>) -> Result<Allowed, ComponentError> {
    let config = config
      .and_then(|config| config.pointer("/arguments/0"))
      .ok_or_else(|| invalid("names neither a 'class' nor 'codes'"))?;
    if let Some(codes) = config.get("codes") {
      let codes = codes
        .as_array()
        .filter(|codes| !codes.is_empty())
        .and_then(|codes| codes.iter().map(as_status).collect::<Option<Vec<_>>>())
        .ok_or_else(|| invalid(format!("'codes' must be a list of statuses, and is {codes}")))?;
      return Ok(Allowed::Codes(codes));
    }
    let class = config
      .get("class")
      .and_then(Value::as_str)
      .ok_or_else(|| invalid("names neither a 'class' nor 'codes'"))?;
    let (from, to, phrase) = match class {
      "information" => (100, 199, "an informational status (100–199)"),
      "success" => (200, 299, "a success status (200–299)"),
      "redirect" => (300, 399, "a redirect status (300–399)"),
      "client-error" => (400, 499, "a client error status (400–499)"),
      "server-error" => (500, 599, "a server error status (500–599)"),
      "non-error" => (0, 399, "a status below 400"),
      "error" => (400, u16::MAX, "a status of 400 or above"),
      other => {
        return Err(invalid(format!(
          "'{other}' is not a status class: information, success, redirect, client-error, \
           server-error, non-error or error"
        )));
      }
    };
    Ok(Allowed::Range { from, to, phrase })
  }

  fn admits(&self, status: u16) -> bool {
    match self {
      Allowed::Range { from, to, .. } => (*from..=*to).contains(&status),
      Allowed::Codes(codes) => codes.contains(&status),
    }
  }

  fn phrase(&self) -> String {
    match self {
      Allowed::Range { phrase, .. } => phrase.to_string(),
      Allowed::Codes(codes) if codes.len() == 1 => format!("status {}", codes[0]),
      Allowed::Codes(codes) => format!(
        "one of the statuses {}",
        codes.iter().map(u16::to_string).collect::<Vec<_>>().join(", ")
      ),
    }
  }
}

fn invalid(message: impl Into<String>) -> ComponentError {
  ComponentError::invalid_config(STATUS_CLASS, message)
}

/// A status as the HTTP transport writes the slot, a number, or as text that is one.
fn as_status(value: &Value) -> Option<u16> {
  match value {
    Value::Number(number) => number.as_u64().and_then(|n| u16::try_from(n).ok()),
    Value::String(text) => text.trim().parse().ok(),
    _ => None,
  }
}

impl MatcherComponent for HttpMatcher {
  fn apply(&self, req: Apply) -> Result<ApplyResult, ComponentError> {
    if req.action != STATUS_CLASS {
      return Err(ComponentError::operation_unsupported(&req.action));
    }
    let allowed = Allowed::read(req.config.as_ref())?;
    let results = req
      .values
      .iter()
      .map(|value| {
        let actual = value.get("content").unwrap_or(&Value::Null);
        match as_status(actual) {
          Some(status) if allowed.admits(status) => json!({ "status": "ok" }),
          Some(status) => json!({
            "status": "error",
            "message": format!("Expected {} but was {status}", allowed.phrase()),
          }),
          None => json!({
            "status": "error",
            "message": format!("Expected {} but was {actual}, which is not a status", allowed.phrase()),
          }),
        }
      })
      .collect();
    Ok(ApplyResult { results })
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use pretty_assertions::assert_eq;

  fn apply(config: Value, status: Value) -> Value {
    HttpMatcher::new()
      .apply(Apply {
        action: STATUS_CLASS.to_string(),
        config: Some(json!({ "arguments": [config] })),
        values: vec![json!({ "content": status })],
      })
      .expect("applied")
      .results
      .remove(0)
  }

  fn ok() -> Value {
    json!({ "status": "ok" })
  }

  #[test]
  fn each_class_admits_its_range_and_nothing_either_side() {
    for (class, first, last) in [
      ("information", 100, 199),
      ("success", 200, 299),
      ("redirect", 300, 399),
      ("client-error", 400, 499),
      ("server-error", 500, 599),
      ("non-error", 100, 399),
      ("error", 400, 599),
    ] {
      let config = json!({ "class": class });
      assert_eq!(
        apply(config.clone(), json!(first)),
        ok(),
        "{class} admits {first}"
      );
      assert_eq!(apply(config.clone(), json!(last)), ok(), "{class} admits {last}");
      if first > 100 {
        assert_ne!(
          apply(config.clone(), json!(first - 1)),
          ok(),
          "{class} refuses {}",
          first - 1
        );
      }
      if last < 599 {
        assert_ne!(
          apply(config, json!(last + 1)),
          ok(),
          "{class} refuses {}",
          last + 1
        );
      }
    }
  }

  #[test]
  fn a_status_outside_its_class_says_what_was_expected() {
    assert_eq!(
      apply(json!({ "class": "success" }), json!(404)),
      json!({ "status": "error", "message": "Expected a success status (200–299) but was 404" })
    );
  }

  #[test]
  fn codes_admit_only_themselves() {
    let config = json!({ "codes": [200, 204] });
    assert_eq!(apply(config.clone(), json!(204)), ok());
    assert_eq!(
      apply(config, json!(201)),
      json!({ "status": "error", "message": "Expected one of the statuses 200, 204 but was 201" })
    );
    assert_eq!(
      apply(json!({ "codes": [204] }), json!(200)),
      json!({ "status": "error", "message": "Expected status 204 but was 200" })
    );
  }

  #[test]
  fn a_status_written_as_text_is_read() {
    assert_eq!(apply(json!({ "class": "success" }), json!("201")), ok());
  }

  #[test]
  fn a_value_that_is_not_a_status_fails_the_node() {
    assert_eq!(
      apply(json!({ "class": "success" }), json!(null)),
      json!({ "status": "error",
              "message": "Expected a success status (200–299) but was null, which is not a status" })
    );
  }

  #[test]
  fn a_class_it_does_not_know_is_invalid_configuration() {
    let error = HttpMatcher::new()
      .apply(Apply {
        action: STATUS_CLASS.to_string(),
        config: Some(json!({ "arguments": [{ "class": "teapot" }] })),
        values: vec![json!({ "content": 418 })],
      })
      .expect_err("refused");
    assert!(
      error.message.contains("'teapot' is not a status class"),
      "{}",
      error.message
    );
  }

  #[test]
  fn any_other_action_is_unsupported() {
    let error = HttpMatcher::new()
      .apply(Apply {
        action: "http:other".to_string(),
        config: None,
        values: vec![],
      })
      .expect_err("refused");
    assert_eq!(error.code, "operation-unsupported");
  }
}
