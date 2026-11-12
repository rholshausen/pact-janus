//! The HTTP component's answers to the subsumption walk (component-interfaces spec §7.5, ADR 0087):
//! the comparability class of each operator it contributes, and `matcher/compare` for a pair one of
//! them is in. A provider that admits a finite set of values never reaches here — the walk asks the
//! consumer's operator about each value (shape spec §8) — so these are the pairs where at least one
//! side admits more values than can be listed.
//!
//! Every answer is either a decision the operator's own semantics settle, or `unknown`. A wrong `yes`
//! is how a checker loses its users (spec §7.5), so nothing here is a heuristic.

use crate::{list, media_type, path, status};
use pact_janus_kernel::component::{Comparability, Compare, CompareResult};
use serde_json::{Value, json};

/// Each operator's class: the three whose admits are decided by their members are `exact`; a path
/// template's containment of another template is not decided here, so `http:path` is `conservative`.
pub fn comparability(operator: &str) -> Comparability {
  match operator {
    status::STATUS_CLASS | media_type::MEDIA_TYPE | list::LIST => Comparability::Exact,
    path::PATH => Comparability::Conservative,
    _ => Comparability::Opaque,
  }
}

/// `matcher/compare`: whether `admits(provider) ⊆ admits(consumer)`.
pub fn compare(req: &Compare) -> CompareResult {
  let (p, c) = (&req.provider, &req.consumer);
  let (p_op, c_op) = (operator(p), operator(c));
  // Wider than any of these: every value they admit is a string, or a status number.
  if c_op == "any" {
    return yes();
  }
  match (p_op, c_op) {
    (status::STATUS_CLASS, status::STATUS_CLASS) => match status::admitted(c) {
      Some(consumer) => statuses(p, consumer),
      None => unknown("the consumer's status class is not one"),
    },
    (status::STATUS_CLASS, "equality" | "any-of") => {
      let codes = literals(c);
      statuses(p, |s| codes.iter().any(|v| v.as_u64() == Some(u64::from(s))))
    }
    ("integer" | "number", status::STATUS_CLASS) => {
      no("the provider may produce any number, not only a status")
    }
    (media_type::MEDIA_TYPE, media_type::MEDIA_TYPE) => {
      // The consumer admits every value the provider does exactly when it admits the provider's own
      // type: the provider's values are that type with parameters added, which the consumer allows.
      let declared = p.get("type").cloned().unwrap_or(Value::Null);
      if admits(c, &declared) {
        yes()
      } else {
        no(format!("the consumer's media type does not admit {declared}"))
      }
    }
    (media_type::MEDIA_TYPE | path::PATH, "string") => yes(),
    (media_type::MEDIA_TYPE, "equality" | "any-of") => {
      no("the provider's media type admits it with parameters added")
    }
    ("string", media_type::MEDIA_TYPE | path::PATH) => no("the provider may produce any string"),
    (list::LIST, list::LIST) => {
      if p.get("elements") == c.get("elements") {
        yes()
      } else {
        no(format!(
          "the provider's list is {}, the consumer's {}",
          p.get("elements").unwrap_or(&Value::Null),
          c.get("elements").unwrap_or(&Value::Null)
        ))
      }
    }
    (list::LIST, "equality" | "any-of") => {
      no("the provider's list may arrive in field lines framed another way")
    }
    (path::PATH, _) => paths(p, c),
    _ => unknown(format!(
      "'{p_op}' against '{c_op}' is not one the HTTP component decides"
    )),
  }
}

/// A path template against another shape. One with no parameter admits one path, which the consumer
/// is asked about; one with a parameter admits paths without end, which only an equal template, a
/// string or `any` contains, and no set of values does.
fn paths(p: &Value, c: &Value) -> CompareResult {
  let template = p.get("template").and_then(Value::as_str).unwrap_or_default();
  match path::Template::parse(template) {
    Ok(parsed) if !parsed.has_parameters() => {
      if admits(c, &json!(template)) {
        yes()
      } else {
        no(format!("the consumer does not admit the path '{template}'"))
      }
    }
    Ok(_) if matches!(operator(c), "equality" | "any-of") => {
      no(format!("the template '{template}' admits paths without end"))
    }
    _ => unknown(format!(
      "whether the template '{template}' is contained in the consumer's shape is not decided"
    )),
  }
}

/// Every status the provider's class admits, against `consumer`.
fn statuses(p: &Value, consumer: impl Fn(u16) -> bool) -> CompareResult {
  let Some(provider) = status::admitted(p) else {
    return unknown("the provider's status class is not one");
  };
  // The statuses HTTP defines first (RFC 9110 §15), so a reason names one a reader recognises.
  let outside: Vec<u16> = (100..=599)
    .chain(0..100)
    .chain(600..=u16::MAX)
    .filter(|s| provider(*s) && !consumer(*s))
    .take(3)
    .collect();
  if outside.is_empty() {
    yes()
  } else {
    no(format!(
      "the provider may produce {}{}",
      outside.iter().map(u16::to_string).collect::<Vec<_>>().join(", "),
      if outside.len() == 3 { ", …" } else { "" }
    ))
  }
}

fn operator(node: &Value) -> &str {
  node.get("shape").and_then(Value::as_str).unwrap_or_default()
}

/// The values an `equality` or `any-of` admits.
fn literals(node: &Value) -> Vec<Value> {
  match node.get("options").and_then(Value::as_array) {
    Some(options) => options.clone(),
    None => node.get("example").into_iter().cloned().collect(),
  }
}

/// Whether `node`, one of this component's operators or `equality`/`any-of`, admits `value`: the
/// operator's own matcher, given the node's members as its configuration.
fn admits(node: &Value, value: &Value) -> bool {
  let config = json!({ "arguments": [node] });
  let values = [json!({ "content": value })];
  let applied = match operator(node) {
    status::STATUS_CLASS => status::apply(Some(&config), &values),
    media_type::MEDIA_TYPE => media_type::apply(Some(&config), &values),
    list::LIST => list::apply(Some(&config), &values),
    path::PATH => path::apply(Some(&config), &values),
    "equality" | "any-of" => return literals(node).contains(value),
    _ => return false,
  };
  applied.is_ok_and(|result| result.results.first().and_then(|r| r.get("status")) == Some(&json!("ok")))
}

fn yes() -> CompareResult {
  CompareResult {
    verdict: "yes".to_string(),
    reason: None,
  }
}

fn no(reason: impl Into<String>) -> CompareResult {
  CompareResult {
    verdict: "no".to_string(),
    reason: Some(reason.into()),
  }
}

fn unknown(reason: impl Into<String>) -> CompareResult {
  CompareResult {
    verdict: "unknown".to_string(),
    reason: Some(reason.into()),
  }
}
