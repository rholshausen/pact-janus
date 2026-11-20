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

/// The actions this component contributes, as its handshake declares them. `csv:column` is also an
/// operator ([`OPERATOR`]), and runs as its own action.
pub const ACTIONS: &[&str] = &["csv:integer", "csv:number", "csv:boolean", "csv:equality", OPERATOR];

/// The one operator this component contributes: `{ "shape": "csv:column", "type": "integer" }`, a
/// field that spells a value of `type` — `text` (the default), `integer`, `number` or `boolean`. It
/// says in one operator what the substitutions say operator by operator, and its handshake declares it
/// `exact`, so a subsumption check decides two of them ([`compare`]).
pub const OPERATOR: &str = "csv:column";

/// What a `csv:column` admits, by its `type`: each admits the ones below it, and `text` every field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Column {
  Text,
  Number,
  Integer,
  Boolean,
}

impl Column {
  fn of(config: &Value) -> Result<Column, String> {
    match config.get("type").and_then(Value::as_str).unwrap_or("text") {
      "text" => Ok(Column::Text),
      "number" => Ok(Column::Number),
      "integer" => Ok(Column::Integer),
      "boolean" => Ok(Column::Boolean),
      other => Err(format!(
        "a csv:column's type is text, integer, number or boolean, not '{other}'"
      )),
    }
  }

  fn name(self) -> &'static str {
    match self {
      Column::Text => "text",
      Column::Number => "number",
      Column::Integer => "integer",
      Column::Boolean => "boolean",
    }
  }

  /// Whether every field this one admits, `wider` admits too.
  fn within(self, wider: Column) -> bool {
    self == wider || wider == Column::Text || (self == Column::Integer && wider == Column::Number)
  }

  /// The action that checks a field is this kind's spelling, if it needs one.
  fn action(self) -> Option<&'static str> {
    match self {
      Column::Text => None,
      Column::Number => Some("csv:number"),
      Column::Integer => Some("csv:integer"),
      Column::Boolean => Some("csv:boolean"),
    }
  }
}

/// `matcher/compare` (spec §7.5): whether every field the provider's shape admits, the consumer's
/// does, one of them being a `csv:column`. Two columns are decided by their types. A column against a
/// core operator is decided only where the answer holds whatever the slot's content type: every column
/// is a string, and a `string` or `any` admits it; a column narrower than `text` does not admit every
/// string. Anything else is `unknown`: `integer` means a number in a JSON body and text that spells one
/// in a CSV body, and this component is not told which.
pub fn compare(provider: &Value, consumer: &Value) -> Value {
  let operator = |shape: &Value| shape.get("shape").and_then(Value::as_str).unwrap_or("").to_string();
  let (p, c) = (operator(provider), operator(consumer));
  let answer = |verdict: &str, reason: String| json!({ "verdict": verdict, "reason": reason });
  match (p.as_str(), c.as_str()) {
    (OPERATOR, OPERATOR) => match (Column::of(provider), Column::of(consumer)) {
      (Ok(p), Ok(c)) if p.within(c) => answer(
        "yes",
        format!("every field that spells {} spells {}", p.name(), c.name()),
      ),
      (Ok(p), Ok(c)) => answer(
        "no",
        format!("a field that spells {} need not spell {}", p.name(), c.name()),
      ),
      (Err(reason), _) | (_, Err(reason)) => answer("unknown", reason),
    },
    (OPERATOR, "string" | "any") => answer("yes", "every CSV field is a string".to_string()),
    ("string", OPERATOR) => match Column::of(consumer) {
      Ok(Column::Text) => answer("yes", "a text column admits every string".to_string()),
      Ok(c) => answer("no", format!("a string need not spell {}", c.name())),
      Err(reason) => answer("unknown", reason),
    },
    _ => answer(
      "unknown",
      format!("'{p}' against '{c}' depends on the content type of the slot, which compare is not told"),
    ),
  }
}

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
    OPERATOR => {
      let config = arguments.first().unwrap_or(&Value::Null);
      let column = match Column::of(config) {
        Ok(column) => column,
        Err(message) => return json!({ "status": "error", "message": message }),
      };
      match column.action() {
        None if text.is_some() => return json!({ "status": "ok" }),
        None => (false, "a CSV field".to_string()),
        Some(action) => return apply(action, value, &[]),
      }
    }
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
