//! Mutations: an interaction's response, changed one way at a time, for both verifiers to judge.
//!
//! Nothing here says which verdict a mutation should get — under a type matcher a changed value
//! passes, under equality it fails, and which applies is the verifiers' business. The harness only
//! asks that both verifiers give the same one. That is where disagreements hide: the verdicts at
//! the edges of a matching rule, not the example everyone already agrees on.
//!
//! Every mutation has a stable id (`body/$.items[0].sku/same-type`), so a disagreement in the
//! ledger names exactly which change exposed it. The set is bounded per interaction, and the
//! same pact always yields the same mutations in the same order.

use crate::fixture::{Body, ResponseSpec};
use serde_json::Value;

/// At most this many body leaves are changed, in document order: enough to reach every matching
/// rule a real response carries near its top, without a large body multiplying the run.
const MAX_LEAVES: usize = 12;
const MAX_REMOVALS: usize = 6;
const MAX_ARRAYS: usize = 4;

pub struct Mutation {
  pub id: String,
  pub response: ResponseSpec,
}

pub fn mutations(response: &ResponseSpec) -> Vec<Mutation> {
  let mut out = Vec::new();
  let mut push = |id: String, change: &dyn Fn(&mut ResponseSpec)| {
    let mut mutated = response.clone();
    change(&mut mutated);
    if &mutated != response {
      out.push(Mutation {
        id,
        response: mutated,
      });
    }
  };

  push("status/next".to_string(), &|r| {
    r.status = if r.status >= 599 { 200 } else { r.status + 1 }
  });

  for name in response.headers.keys() {
    push(format!("header/{name}/changed"), &|r| {
      if let Some(values) = r.headers.get_mut(name) {
        for value in values.iter_mut() {
          value.push_str("-changed");
        }
      }
    });
    push(format!("header/{name}/missing"), &|r| {
      r.headers.remove(name);
    });
  }

  match &response.body {
    None => push("body/added".to_string(), &|r| {
      r.body = Some(Body::Json(serde_json::json!({ "unexpected": true })));
    }),
    Some(Body::Json(document)) => {
      for (id, changed) in json_mutations(document) {
        push(format!("body/{id}"), &|r| {
          r.body = Some(Body::Json(changed.clone()))
        });
      }
    }
    Some(Body::Text(text)) => {
      push("body/text-changed".to_string(), &|r| {
        r.body = Some(Body::Text(format!("{text}!")))
      });
      push("body/empty".to_string(), &|r| {
        r.body = Some(Body::Text(String::new()))
      });
    }
    Some(body @ Body::Base64(_)) => {
      push("body/bytes-changed".to_string(), &|r| {
        let mut bytes = body.bytes();
        if let Some(last) = bytes.last_mut() {
          *last ^= 0xff;
        }
        r.body = Some(Body::from_bytes(&bytes, false));
      });
    }
  }
  out
}

/// A JSON document's mutations, as (id, changed document).
fn json_mutations(document: &Value) -> Vec<(String, Value)> {
  let mut leaves = Vec::new();
  let mut members = Vec::new();
  let mut arrays = Vec::new();
  walk(document, "$", &mut leaves, &mut members, &mut arrays);

  let mut out = Vec::new();
  for (path, pointer, value) in leaves.into_iter().take(MAX_LEAVES) {
    if let Some(same) = same_type(value) {
      out.push((format!("{path}/same-type"), replaced(document, &pointer, same)));
    }
    out.push((
      format!("{path}/other-type"),
      replaced(document, &pointer, other_type(value)),
    ));
  }
  for (path, pointer) in members.into_iter().take(MAX_REMOVALS) {
    out.push((format!("{path}/missing"), removed(document, &pointer)));
  }
  for (path, pointer, items) in arrays.into_iter().take(MAX_ARRAYS) {
    out.push((
      format!("{path}/empty"),
      replaced(document, &pointer, Value::Array(vec![])),
    ));
    if let Some(last) = items.last() {
      let mut longer = items.clone();
      longer.push(last.clone());
      out.push((
        format!("{path}/one-more"),
        replaced(document, &pointer, Value::Array(longer)),
      ));
    }
  }
  if let Value::Object(map) = document {
    let mut extended = map.clone();
    extended.insert("janusUnexpectedMember".to_string(), Value::Bool(true));
    out.push(("$/extra-member".to_string(), Value::Object(extended)));
  }
  out
}

type Leaf<'a> = (String, String, &'a Value);

fn walk<'a>(
  value: &'a Value,
  path: &str,
  leaves: &mut Vec<Leaf<'a>>,
  members: &mut Vec<(String, String)>,
  arrays: &mut Vec<(String, String, Vec<Value>)>,
) {
  walk_at(value, path, "", leaves, members, arrays);
}

fn walk_at<'a>(
  value: &'a Value,
  path: &str,
  pointer: &str,
  leaves: &mut Vec<Leaf<'a>>,
  members: &mut Vec<(String, String)>,
  arrays: &mut Vec<(String, String, Vec<Value>)>,
) {
  match value {
    Value::Object(map) => {
      for (name, member) in map {
        let child_path = if name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
          format!("{path}.{name}")
        } else {
          format!("{path}['{name}']")
        };
        let child_pointer = format!("{pointer}/{}", name.replace('~', "~0").replace('/', "~1"));
        members.push((child_path.clone(), child_pointer.clone()));
        walk_at(member, &child_path, &child_pointer, leaves, members, arrays);
      }
    }
    Value::Array(items) => {
      arrays.push((path.to_string(), pointer.to_string(), items.clone()));
      for (index, item) in items.iter().enumerate() {
        walk_at(
          item,
          &format!("{path}[{index}]"),
          &format!("{pointer}/{index}"),
          leaves,
          members,
          arrays,
        );
      }
    }
    leaf => leaves.push((path.to_string(), pointer.to_string(), leaf)),
  }
}

/// A different value of the same JSON type, or `None` for `null`, which has no other.
fn same_type(value: &Value) -> Option<Value> {
  match value {
    Value::String(text) => Some(Value::String(format!("{text}-changed"))),
    Value::Number(number) if number.is_i64() || number.is_u64() => {
      Some(Value::from(number.as_i64().map_or(0, |n| n.wrapping_add(1))))
    }
    Value::Number(number) => Some(Value::from(number.as_f64().unwrap_or(0.0) + 0.5)),
    Value::Bool(flag) => Some(Value::Bool(!flag)),
    Value::Null | Value::Object(_) | Value::Array(_) => None,
  }
}

/// The same value spelled as another JSON type.
fn other_type(value: &Value) -> Value {
  match value {
    Value::String(_) => Value::from(1),
    Value::Number(number) => Value::String(number.to_string()),
    Value::Bool(flag) => Value::String(flag.to_string()),
    Value::Null => Value::String("null".to_string()),
    Value::Object(_) | Value::Array(_) => Value::Null,
  }
}

fn replaced(document: &Value, pointer: &str, with: Value) -> Value {
  let mut copy = document.clone();
  match copy.pointer_mut(pointer) {
    Some(slot) => *slot = with,
    None if pointer.is_empty() => copy = with,
    None => {}
  }
  copy
}

fn removed(document: &Value, pointer: &str) -> Value {
  let mut copy = document.clone();
  if let Some((parent, name)) = pointer.rsplit_once('/') {
    let name = name.replace("~1", "/").replace("~0", "~");
    let parent = if parent.is_empty() {
      Some(&mut copy)
    } else {
      copy.pointer_mut(parent)
    };
    if let Some(Value::Object(map)) = parent {
      let _: Option<Value> = map.remove(&name);
    }
  }
  copy
}

#[cfg(test)]
mod tests {
  use super::*;
  use pretty_assertions::assert_eq;
  use serde_json::json;
  use std::collections::BTreeMap;

  fn response(body: Option<Body>) -> ResponseSpec {
    ResponseSpec {
      status: 200,
      delay_ms: None,
      headers: BTreeMap::from([("Content-Type".to_string(), vec!["application/json".to_string()])]),
      body,
    }
  }

  #[test]
  fn every_mutation_has_a_stable_id_and_changes_something() {
    let original = response(Some(Body::Json(
      json!({ "id": "66", "items": [{ "qty": 1 }], "ok": true }),
    )));
    let ids: Vec<String> = mutations(&original).into_iter().map(|m| m.id).collect();
    assert_eq!(
      ids,
      vec![
        "status/next",
        "header/Content-Type/changed",
        "header/Content-Type/missing",
        "body/$.id/same-type",
        "body/$.id/other-type",
        "body/$.items[0].qty/same-type",
        "body/$.items[0].qty/other-type",
        "body/$.ok/same-type",
        "body/$.ok/other-type",
        "body/$.id/missing",
        "body/$.items/missing",
        "body/$.items[0].qty/missing",
        "body/$.ok/missing",
        "body/$.items/empty",
        "body/$.items/one-more",
        "body/$/extra-member",
      ]
    );
  }

  #[test]
  fn a_change_lands_where_its_id_says() {
    let original = response(Some(Body::Json(json!({ "items": [{ "qty": 1 }] }))));
    let mutated = mutations(&original);
    let find = |id: &str| mutated.iter().find(|m| m.id == id).unwrap().response.body.clone();
    assert_eq!(
      find("body/$.items[0].qty/same-type"),
      Some(Body::Json(json!({ "items": [{ "qty": 2 }] })))
    );
    assert_eq!(
      find("body/$.items[0].qty/missing"),
      Some(Body::Json(json!({ "items": [{}] })))
    );
    assert_eq!(
      find("body/$.items/one-more"),
      Some(Body::Json(json!({ "items": [{ "qty": 1 }, { "qty": 1 }] })))
    );
  }

  #[test]
  fn a_body_that_is_not_json_is_changed_as_text_or_bytes() {
    let text: Vec<String> = mutations(&response(Some(Body::Text("a=1".to_string()))))
      .into_iter()
      .map(|m| m.id)
      .collect();
    assert!(text.contains(&"body/text-changed".to_string()), "{text:?}");
    let bytes = mutations(&response(Some(Body::Base64("/wA=".to_string()))));
    let changed = bytes.iter().find(|m| m.id == "body/bytes-changed").unwrap();
    assert_eq!(changed.response.body.as_ref().unwrap().bytes(), vec![0xff, 0xff]);
  }
}
