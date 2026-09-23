//! What this component contributes to a plan (component-interfaces spec §6.3 and §7.1): a fragment
//! for a CSV body, and the three actions it uses.
//!
//! **Why a fragment at all.** CSV has no types, so a CSV body decodes to strings — `items` is `"1"`,
//! never `1`. A consumer who writes the shape they mean, `items: integer`, gets a generic
//! `match:integer` from the engine, and a string is not an integer: every CSV provider fails. The
//! fragment says what `integer` means *for this content type* — text that spells an integer — with
//! `csv:integer`, and leaves every operator CSV does not change to the core action the engine would
//! have used anyway.
//!
//! **What it covers.** The body shape a CSV document actually has: an `each-like` of `object`s whose
//! members are value operators. Anything else — an `optional` member, a nested
//! object, a `one-of` — gets no fragment, and the engine's generic plan, which is correct for
//! everything except the typed operators. Declining is always safe; guessing is not.
//!
//! Written against plan-grammar spec §2 (the node kinds), §4 (the action names) and §5.2 (what the
//! engine emits for each operator, which this mirrors so `explain` reads the same either way), and
//! the plan document schema. Plan grammar `v0`, and only when the engine says it reads it.

use serde_json::{Value, json};

/// The grammar every fragment here is written against.
pub const GRAMMAR: &str = "v0";

/// The actions this component contributes, as its handshake declares them.
pub const ACTIONS: &[&str] = &["csv:integer", "csv:number", "csv:boolean"];

/// `content/compile`: a fragment for `shape` at `path`, or `None` for the engine's generic plan.
pub fn compile(shape: &Value, path: &str) -> Option<Value> {
  let (items, min, max) = match shape.get("shape")?.as_str()? {
    "each-like" => (
      shape.get("items")?,
      shape.get("min").cloned().unwrap_or(json!(1)),
      shape.get("max").cloned().unwrap_or(Value::Null),
    ),
    _ => return None,
  };
  let row = format!("{path}[*]");
  let mut members = vec![action("expect:object", vec![current("~>")])];
  if items.get("shape")?.as_str()? != "object" {
    return None;
  }
  for (name, member) in items.get("members")?.as_object()? {
    let segment = crate::member_path(name);
    let check = value_check(member, &format!("~>{segment}"))?;
    members.push(container(&format!("{row}{segment}"), vec![check]));
  }
  Some(container(
    "text/csv",
    vec![
      json!({ "kind": "annotation",
              "text": "CSV has no types: integer, number, decimal and boolean are checked as the text that spells them" }),
      action("expect:array", vec![resolve(path)]),
      action("expect:size", vec![resolve(path), value(min), value(max)]),
      action(
        "for-each",
        vec![
          json!({ "kind": "splat", "children": [resolve(path)] }),
          container(&row, members),
        ],
      ),
    ],
  ))
}

/// One member's check: the core action for an operator CSV does not change, a `csv:` one for an
/// operator it does, and `None` — no fragment at all — for anything else.
fn value_check(shape: &Value, at: &str) -> Option<Value> {
  let target = current(at);
  Some(match shape.get("shape")?.as_str()? {
    "string" => action("match:string", vec![target]),
    "any" => action("match:any", vec![target]),
    "regex" => action("match:regex", vec![target, value(shape.get("pattern")?.clone())]),
    "equality" => {
      // The example as the text a CSV field would hold: `1` is `"1"` here.
      let example = match shape.get("example")? {
        Value::String(text) => text.clone(),
        Value::Number(number) => number.to_string(),
        Value::Bool(flag) => flag.to_string(),
        _ => return None,
      };
      action("match:equality", vec![target, value(json!(example))])
    }
    "integer" => action("csv:integer", vec![target]),
    "number" | "decimal" => action("csv:number", vec![target]),
    "boolean" => action("csv:boolean", vec![target]),
    _ => return None,
  })
}

/// `matcher/apply` (spec §7.1): one result per value, in plan-grammar spec §2.3's result form.
pub fn apply(action: &str, value: &Value) -> Value {
  let text = value.as_str();
  let (holds, what) = match action {
    "csv:integer" => (
      text.is_some_and(|t| {
        let digits = t.strip_prefix(['-', '+']).unwrap_or(t);
        !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit())
      }),
      "an integer",
    ),
    "csv:number" => (
      text
        .is_some_and(|t| !t.trim().is_empty() && t.trim() == t && t.parse::<f64>().is_ok_and(f64::is_finite)),
      "a number",
    ),
    "csv:boolean" => (text.is_some_and(|t| t == "true" || t == "false"), "a boolean"),
    _ => {
      return json!({ "status": "error", "message": format!("'{action}' is not an action of this component") });
    }
  };
  if holds {
    json!({ "status": "ok" })
  } else {
    json!({ "status": "error", "message": format!("expected text that spells {what}, got {value}") })
  }
}

fn container(label: &str, children: Vec<Value>) -> Value {
  json!({ "kind": "container", "label": label, "children": children })
}

fn action(name: &str, children: Vec<Value>) -> Value {
  json!({ "kind": "action", "name": name, "children": children })
}

fn resolve(path: &str) -> Value {
  json!({ "kind": "resolve", "path": path })
}

fn current(path: &str) -> Value {
  json!({ "kind": "resolve-current", "path": path })
}

/// A `value` node: the literal tagged with its document kind (plan schema `Value`).
fn value(literal: Value) -> Value {
  let of = match &literal {
    Value::Null => "null",
    Value::Bool(_) => "boolean",
    Value::Number(_) => "number",
    Value::String(_) => "string",
    Value::Array(_) => "array",
    Value::Object(_) => "object",
  };
  json!({ "kind": "value", "value": { "of": of, "value": literal } })
}
