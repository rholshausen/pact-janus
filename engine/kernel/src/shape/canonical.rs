//! How a shape is written into a recorded document, a contract or a provider shape (contract-file
//! spec §2.4, ADR 0037).
//!
//! A shape node mixes two kinds of member. Its own members (`shape`, `of`, `min`, `pattern`, ...)
//! are names this specification defines, so they are written by name: the same shape is the same
//! bytes whichever SDK wrote it, and a contract's bytes do not change because an SDK changed the
//! order it builds a node in. An `object` shape's `members` are the author's declaration, and their
//! order is what a generated document has (a CSV's columns, say), so they keep it. The documents a
//! node carries (`example`, `options`) are written as they arrived, and so is anything the kernel
//! does not interpret: the value of a component operator's own members (shape spec §3.5), and a
//! `generator`. A component operator's members are still written by name, since a component reads
//! them by name and nothing is decided by their order (ADR 0037), so two SDKs that build the node in
//! different orders write the same bytes (ADR 0082).

use super::model::CORE_OPERATORS;
use serde_json::{Map, Value};

/// `shape` written canonically: see the module docs. Anything that is not a shape node — which a
/// recorded document never holds, since it was validated before it was recorded — is returned
/// unchanged.
pub fn canonical(shape: &Value) -> Value {
  let Value::Object(node) = shape else {
    return shape.clone();
  };
  let core = node
    .get("shape")
    .and_then(Value::as_str)
    .is_some_and(|operator| CORE_OPERATORS.contains(&operator));
  if !core {
    // A component operator's node is the component's to read (shape spec §3.5): its members by
    // name, and each value as it arrived.
    let mut names: Vec<&String> = node.keys().collect();
    names.sort();
    return Value::Object(
      names
        .into_iter()
        .map(|name| (name.clone(), node[name.as_str()].clone()))
        .collect(),
    );
  }
  let mut names: Vec<&String> = node.keys().collect();
  names.sort();
  let mut out = Map::new();
  for name in names {
    let member = &node[name.as_str()];
    let written = match name.as_str() {
      // The author's declaration, in the author's order.
      "members" => each_member(member, false),
      // Names, but not an order anything reads: a `one-of`'s default is the first by name.
      "alternatives" => each_member(member, true),
      "of" | "items" | "keys" | "values" => canonical(member),
      "entries" => match member {
        Value::Array(entries) => Value::Array(entries.iter().map(canonical).collect()),
        other => other.clone(),
      },
      _ => member.clone(),
    };
    out.insert(name.clone(), written);
  }
  Value::Object(out)
}

/// A map of names to shapes, each written canonically, in the order given or by name.
fn each_member(members: &Value, by_name: bool) -> Value {
  let Value::Object(members) = members else {
    return members.clone();
  };
  let mut entries: Vec<(&String, &Value)> = members.iter().collect();
  if by_name {
    entries.sort_by_key(|(name, _)| *name);
  }
  Value::Object(
    entries
      .into_iter()
      .map(|(name, shape)| (name.clone(), canonical(shape)))
      .collect(),
  )
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json::json;

  /// ADR 0082: two SDKs build `http:list` in different orders, and the engine writes one set of bytes.
  /// A member's value is written as it arrived, so the elements keep their order.
  #[test]
  fn a_component_operators_members_are_written_by_name_and_their_values_as_they_arrived() {
    let typescript: Value =
      serde_json::from_str(r#"{ "shape": "http:list", "elements": ["b", "a"], "example": ["b, a"] }"#)
        .unwrap();
    let java: Value =
      serde_json::from_str(r#"{ "shape": "http:list", "example": ["b, a"], "elements": ["b", "a"] }"#)
        .unwrap();
    let written = serde_json::to_string(&canonical(&typescript)).unwrap();
    assert_eq!(written, serde_json::to_string(&canonical(&java)).unwrap());
    assert_eq!(
      written,
      r#"{"elements":["b","a"],"example":["b, a"],"shape":"http:list"}"#
    );
    assert_eq!(
      canonical(&json!({ "shape": "http:list" })),
      json!({ "shape": "http:list" })
    );
  }
}
