//! What this component says a value operator means for CSV (component-interfaces spec §6.3, §7.2):
//! the substitutions its handshake declares, and the actions they name, run through `matcher/apply`.
//!
//! **Why substitutions at all.** CSV has no types, so a CSV body decodes to strings — `items` is
//! `"1"`, never `1`. A consumer who writes the shape they mean, `items: integer`, would get the
//! engine's `match:integer`, and a string is not an integer: every CSV provider would fail. So this
//! component says what `integer` means *for this content type*, text that spells an integer, and the
//! engine runs `csv:integer` there instead. Everything else in the plan, every structure and every
//! variant's pinning, is the engine's, and every operator not named here is checked as the engine
//! checks it.
//!
//! Written against the component interfaces' handshake (`substitutions`, spec §3.3) and `apply`'s
//! arguments (spec §7.2): the value under test, then the core action's arguments in order.

use serde_json::{Value, json};

/// The actions this component contributes, as its handshake declares them.
pub const ACTIONS: &[&str] = &["csv:integer", "csv:number", "csv:boolean", "csv:equality"];

/// The `substitutions` of its `text/csv` entry: the value operators whose meaning CSV changes.
pub fn substitutions() -> Value {
  json!({
    "integer": "csv:integer",
    "number": "csv:number",
    "decimal": "csv:number",
    "boolean": "csv:boolean",
    "equality": "csv:equality",
  })
}

/// `matcher/apply` (spec §7.2): one result per value, in plan-grammar spec §2.3's result form.
/// `arguments` are the core action's: `equality`'s example, and none for the rest.
pub fn apply(action: &str, value: &Value, arguments: &[Value]) -> Value {
  let text = value.as_str();
  let (holds, what) = match action {
    "csv:integer" => (
      text.is_some_and(|t| {
        let digits = t.strip_prefix(['-', '+']).unwrap_or(t);
        !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit())
      }),
      "text that spells an integer".to_string(),
    ),
    "csv:number" => (
      text
        .is_some_and(|t| !t.trim().is_empty() && t.trim() == t && t.parse::<f64>().is_ok_and(f64::is_finite)),
      "text that spells a number".to_string(),
    ),
    "csv:boolean" => (
      text.is_some_and(|t| t == "true" || t == "false"),
      "text that spells a boolean".to_string(),
    ),
    "csv:equality" => {
      // The example as the text a CSV field would hold: `1` is `"1"` here.
      let Some(example) = arguments.first().and_then(as_field) else {
        return json!({ "status": "error",
                       "message": format!("a CSV field cannot hold {}", arguments.first().unwrap_or(&Value::Null)) });
      };
      (text == Some(example.as_str()), format!("'{example}'"))
    }
    _ => {
      return json!({ "status": "error", "message": format!("'{action}' is not an action of this component") });
    }
  };
  if holds {
    json!({ "status": "ok" })
  } else {
    json!({ "status": "error", "message": format!("expected {what}, got {value}") })
  }
}

/// A scalar as the text a CSV field holds, or `None` for what no field can (an array, an object,
/// null: an empty field is the empty string).
fn as_field(value: &Value) -> Option<String> {
  match value {
    Value::String(text) => Some(text.clone()),
    Value::Number(number) => Some(number.to_string()),
    Value::Bool(flag) => Some(flag.to_string()),
    _ => None,
  }
}
