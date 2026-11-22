//! The component conformance corpus (component-interfaces spec §9.4,
//! `schemas/v1/conformance-case.schema.json`, staged plan S5.10, ADR 0105): cases of `(op, body)` calls
//! with the results each must produce, run through every binding a component is reached by, which must
//! answer each call identically.
//!
//! A case is run against a [`Binding`]: a component's interfaces as some binding presents them. Every
//! call goes through [`wire::serve`], so a binding is compared by the documents it produces and nothing
//! else. The built-in components are run [`Binding::native`] and [`Binding::serialising`], the second
//! through frame bytes and the WASM binding's own client; an out-of-tree component is run through the
//! binding that loaded it (`janus component check`).

use super::wire::{self, InProcess, PipeClient};
use super::{ComponentError, ContentComponent, MatcherComponent};
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;

/// One case: a sequence of calls on one component, each with what it must produce.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Case {
  pub name: String,
  pub component: String,
  #[serde(default)]
  pub interface: Option<String>,
  /// The bindings the case runs through; absent means every one the component is reached by.
  #[serde(default)]
  pub bindings: Option<Vec<String>>,
  pub calls: Vec<Call>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Call {
  pub op: String,
  pub body: Value,
  #[serde(default)]
  pub expect: Option<Expected>,
}

/// What a call must produce: an `ok` document, matched member-wise, or an `error`, matched by its
/// `code` and whatever else it names but its `message`.
#[derive(Debug, Clone, Deserialize)]
pub struct Expected {
  #[serde(default)]
  pub ok: Option<Value>,
  #[serde(default)]
  pub error: Option<Value>,
}

impl Case {
  /// A case from its file's text.
  pub fn read(text: &str) -> Result<Case, String> {
    let case: Case = serde_json::from_str(text).map_err(|err| err.to_string())?;
    if case.calls.is_empty() {
      return Err("a case makes at least one call".to_string());
    }
    for (index, call) in case.calls.iter().enumerate() {
      if let Some(expect) = &call.expect
        && expect.ok.is_some() == expect.error.is_some()
      {
        return Err(format!(
          "calls[{index}]: 'expect' carries exactly one of 'ok' or 'error'"
        ));
      }
    }
    Ok(case)
  }
}

/// A component's interfaces as one binding presents them.
pub struct Binding {
  pub name: String,
  pub content: Option<Arc<dyn ContentComponent>>,
  pub matcher: Option<Arc<dyn MatcherComponent>>,
  /// Its `component/hello` result, when it has one to give. An in-tree component has none yet.
  pub hello: Option<Value>,
}

impl Binding {
  /// The native binding (spec §9.1): the component called with in-memory documents.
  pub fn native(
    content: Option<Arc<dyn ContentComponent>>,
    matcher: Option<Arc<dyn MatcherComponent>>,
  ) -> Self {
    Binding {
      name: "native".to_string(),
      content,
      matcher,
      hello: None,
    }
  }

  /// The serialising in-process binding (spec §9.4): the same component behind real frames, reached
  /// through the WASM binding's client ([`wire::PipeClient`] over [`wire::InProcess`]).
  pub fn serialising(
    component: &str,
    content: Option<Arc<dyn ContentComponent>>,
    matcher: Option<Arc<dyn MatcherComponent>>,
  ) -> Self {
    let (has_content, has_matcher) = (content.is_some(), matcher.is_some());
    let client = Arc::new(PipeClient::new(
      InProcess::new(component, content, matcher),
      Vec::new(),
    ));
    Binding {
      name: "serialising".to_string(),
      content: has_content.then(|| Arc::clone(&client) as Arc<dyn ContentComponent>),
      matcher: has_matcher.then_some(client as Arc<dyn MatcherComponent>),
      hello: None,
    }
  }

  /// One call, as its result document or the error the component answered with.
  pub fn call(&self, op: &str, body: &Value) -> Result<Value, ComponentError> {
    if op == "component/hello" {
      return self
        .hello
        .clone()
        .ok_or_else(|| ComponentError::operation_unsupported(op));
    }
    wire::serve(self.content.as_deref(), self.matcher.as_deref(), op, body)
  }
}

/// What running a case found: each problem a sentence naming the call and the binding. A case with
/// none passed.
#[derive(Debug, Clone, PartialEq)]
pub struct Report {
  pub case: String,
  /// The bindings it ran through.
  pub bindings: Vec<String>,
  pub problems: Vec<String>,
}

impl Report {
  pub fn passed(&self) -> bool {
    self.problems.is_empty()
  }
}

/// Run `case` through each of `bindings` it names (every one, when it names none): each call must
/// produce what it expects, and every binding must answer each call as the first did.
pub fn run(case: &Case, bindings: &[Binding]) -> Report {
  let selected: Vec<&Binding> = bindings
    .iter()
    .filter(|binding| {
      case
        .bindings
        .as_ref()
        .is_none_or(|names| names.contains(&binding.name))
    })
    .collect();
  let mut report = Report {
    case: case.name.clone(),
    bindings: selected.iter().map(|binding| binding.name.clone()).collect(),
    problems: Vec::new(),
  };
  if selected.is_empty() {
    report
      .problems
      .push("no binding it names is one this component is reached by".to_string());
    return report;
  }
  let answers: Vec<Vec<Result<Value, Value>>> = selected
    .iter()
    .map(|binding| {
      case
        .calls
        .iter()
        .map(|call| {
          binding
            .call(&call.op, &call.body)
            .map_err(|error| serde_json::to_value(error).unwrap_or(Value::Null))
        })
        .collect()
    })
    .collect();
  for (binding, answers) in selected.iter().zip(&answers) {
    for (index, (call, answer)) in case.calls.iter().zip(answers).enumerate() {
      if let Some(expect) = &call.expect {
        for problem in unmet(expect, answer) {
          report.problems.push(format!(
            "calls[{index}] ({}) through {}: {problem}",
            call.op, binding.name
          ));
        }
      }
    }
  }
  let (first, first_answers) = (&selected[0].name, &answers[0]);
  for (binding, answers) in selected.iter().zip(&answers).skip(1) {
    for (index, (call, (theirs, ours))) in case
      .calls
      .iter()
      .zip(first_answers.iter().zip(answers))
      .enumerate()
    {
      if theirs != ours {
        report.problems.push(format!(
          "calls[{index}] ({}): {first} answered {} and {} answered {}",
          call.op,
          described(theirs),
          binding.name,
          described(ours)
        ));
      }
    }
  }
  report
}

fn described(answer: &Result<Value, Value>) -> String {
  match answer {
    Ok(ok) => format!("ok {ok}"),
    Err(error) => format!("error {error}"),
  }
}

/// How `answer` falls short of `expect`.
fn unmet(expect: &Expected, answer: &Result<Value, Value>) -> Vec<String> {
  match (&expect.ok, &expect.error, answer) {
    (Some(ok), _, Ok(actual)) => differences(ok, actual, ""),
    (Some(_), _, Err(error)) => vec![format!(
      "expected a result, and the component answered {}: {}",
      error["code"].as_str().unwrap_or("?"),
      error["message"].as_str().unwrap_or_default()
    )],
    (None, Some(expected), Err(error)) => {
      let mut expected = expected.clone();
      // `message` is never asserted (the schema's `Expected`): it is for people.
      if let Some(members) = expected.as_object_mut() {
        members.shift_remove("message");
      }
      differences(&expected, error, "/error")
    }
    (None, Some(expected), Ok(actual)) => vec![format!(
      "expected the error {}, and the component answered {actual}",
      expected["code"]
    )],
    (None, None, _) => Vec::new(),
  }
}

/// Where `actual` differs from `expected`, member-wise: a member `expected` does not mention is not
/// constrained, so a result that grows a member additively still passes. An array's elements are
/// matched in order, and it must be as long. Each difference is located by a JSON pointer.
pub fn differences(expected: &Value, actual: &Value, at: &str) -> Vec<String> {
  match (expected, actual) {
    (Value::Object(expected), Value::Object(actual)) => expected
      .iter()
      .flat_map(|(name, expected)| {
        let at = format!("{at}/{}", name.replace('~', "~0").replace('/', "~1"));
        match actual.get(name) {
          Some(actual) => differences(expected, actual, &at),
          None => vec![format!(
            "at {at}: expected {expected}, and there is nothing there"
          )],
        }
      })
      .collect(),
    (Value::Array(expected_items), Value::Array(actual_items)) => {
      if expected_items.len() != actual_items.len() {
        return vec![format!(
          "at {}: expected {} element(s), and there are {}: {actual}",
          if at.is_empty() { "/" } else { at },
          expected_items.len(),
          actual_items.len()
        )];
      }
      expected_items
        .iter()
        .zip(actual_items)
        .enumerate()
        .flat_map(|(index, (expected, actual))| differences(expected, actual, &format!("{at}/{index}")))
        .collect()
    }
    (expected, actual) if expected == actual => Vec::new(),
    (expected, actual) => vec![format!(
      "at {}: expected {expected}, got {actual}",
      if at.is_empty() { "/" } else { at }
    )],
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::component::{
    Compile, CompileResult, Decode, DecodeResult, Detect, DetectResult, Encode, EncodeResult,
  };
  use crate::plan::RuntimeValue;
  use pretty_assertions::assert_eq;
  use serde_json::json;

  /// Decodes a slot's content as a JSON string document; refuses every other operation.
  struct Strings;

  impl ContentComponent for Strings {
    fn handles(&self, _content_type: &str) -> bool {
      true
    }
    fn decode(&self, req: Decode) -> Result<DecodeResult, ComponentError> {
      Ok(DecodeResult::of(RuntimeValue::from_json(&req.value.content)))
    }
    fn encode(&self, req: Encode) -> Result<EncodeResult, ComponentError> {
      Err(ComponentError::encode_failed(&req.content_type, "no"))
    }
    fn compile(&self, _req: Compile) -> Result<CompileResult, ComponentError> {
      Ok(CompileResult {})
    }
    fn detect(&self, _req: Detect) -> Result<DetectResult, ComponentError> {
      Err(ComponentError::operation_unsupported("content/detect"))
    }
  }

  fn bindings() -> Vec<Binding> {
    let content: Arc<dyn ContentComponent> = Arc::new(Strings);
    vec![
      Binding::native(Some(Arc::clone(&content)), None),
      Binding::serialising("strings", Some(content), None),
    ]
  }

  fn case(calls: Value) -> Case {
    Case::read(&json!({ "name": "c", "component": "strings", "calls": calls }).to_string()).unwrap()
  }

  #[test]
  fn a_case_whose_calls_answer_as_expected_passes_through_both_bindings() {
    let report = run(
      &case(json!([
        { "op": "content/decode",
          "body": { "content-type": "text/plain", "value": { "content": { "a": 1, "b": 2 } } },
          "expect": { "ok": { "document": { "a": 1 } } } },
        { "op": "content/encode", "body": { "content-type": "text/plain", "document": "x" },
          "expect": { "error": { "code": "encode-failed", "message": "never asserted" } } }
      ])),
      &bindings(),
    );
    assert_eq!(report.bindings, vec!["native", "serialising"]);
    assert_eq!(report.problems, Vec::<String>::new());
  }

  #[test]
  fn each_unmet_expectation_is_a_problem_naming_the_call_and_the_binding() {
    let report = run(
      &case(json!([
        { "op": "content/decode",
          "body": { "content-type": "text/plain", "value": { "content": [1, 2] } },
          "expect": { "ok": { "document": [1, 3] } } },
        { "op": "content/detect", "body": { "value": { "content": "x" } },
          "expect": { "ok": {} } }
      ])),
      &bindings(),
    );
    assert_eq!(
      report.problems,
      vec![
        "calls[0] (content/decode) through native: at /document/1: expected 3, got 2",
        "calls[1] (content/detect) through native: expected a result, and the component answered operation-unsupported: unsupported operation 'content/detect'",
        "calls[0] (content/decode) through serialising: at /document/1: expected 3, got 2",
        "calls[1] (content/detect) through serialising: expected a result, and the component answered operation-unsupported: unsupported operation 'content/detect'",
      ]
    );
  }

  #[test]
  fn bindings_that_answer_a_call_differently_fail_the_case() {
    let mut bindings = bindings();
    bindings[1].content = None;
    let report = run(
      &case(json!([
        { "op": "content/compile", "body": { "content-type": "text/plain", "shape": {}, "path": "$" } }
      ])),
      &bindings,
    );
    assert_eq!(report.problems.len(), 1, "{:?}", report.problems);
    assert!(
      report.problems[0]
        .starts_with("calls[0] (content/compile): native answered ok {} and serialising answered error"),
      "{}",
      report.problems[0]
    );
  }

  #[test]
  fn a_case_names_the_bindings_it_runs_through() {
    let mut named = case(json!([{ "op": "component/hello", "body": {} }]));
    named.bindings = Some(vec!["wasm".to_string()]);
    assert_eq!(
      run(&named, &bindings()).problems,
      vec!["no binding it names is one this component is reached by"]
    );
  }

  #[test]
  fn an_expectation_carries_exactly_one_of_ok_or_error() {
    let text = json!({ "name": "c", "component": "x",
                       "calls": [ { "op": "content/decode", "body": {}, "expect": {} } ] })
    .to_string();
    assert_eq!(
      Case::read(&text).unwrap_err(),
      "calls[0]: 'expect' carries exactly one of 'ok' or 'error'"
    );
  }

  #[test]
  fn differences_are_member_wise_and_arrays_match_in_order_and_length() {
    assert_eq!(
      differences(
        &json!({ "a": { "b": 1 } }),
        &json!({ "a": { "b": 1, "c": 2 }, "d": 3 }),
        ""
      ),
      Vec::<String>::new()
    );
    assert_eq!(
      differences(&json!({ "a/b": [1] }), &json!({ "a/b": [1, 2] }), ""),
      vec!["at /a~1b: expected 1 element(s), and there are 2: [1,2]"]
    );
    assert_eq!(
      differences(&json!({ "a": 1 }), &json!({}), ""),
      vec!["at /a: expected 1, and there is nothing there"]
    );
    assert_eq!(
      differences(&json!(1), &json!(1.0), ""),
      vec!["at /: expected 1, got 1.0"],
      "a number's kind is how it is written (ADR 0072)"
    );
  }
}
