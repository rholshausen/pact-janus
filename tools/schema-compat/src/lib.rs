//! Checks Engine Protocol JSON Schemas against the open-world authoring rules
//! (spec §2.2) and, between two versions of the same schema, the
//! additive-evolution rules (spec §11.2). The spec is the authority; every
//! check here cites the rule it enforces.

use serde_json::Value;
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
  /// JSON-pointer-ish location inside the schema document.
  pub path: String,
  pub message: String,
}

impl std::fmt::Display for Violation {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}: {}", self.path, self.message)
  }
}

/// Keywords holding a single subschema.
const SINGLE_SUBSCHEMA: &[&str] = &[
  "items",
  "additionalProperties",
  "propertyNames",
  "if",
  "then",
  "else",
  "not",
];

/// Keywords holding an array of subschemas.
const SUBSCHEMA_ARRAYS: &[&str] = &["allOf", "anyOf", "oneOf", "prefixItems"];

/// Keywords holding a map of name -> subschema.
const SUBSCHEMA_MAPS: &[&str] = &["properties", "$defs"];

/// Constraint keywords frozen within a protocol version (§11.2: neither
/// narrowing nor widening — old validators must keep accepting new frames and
/// new validators old frames).
const FROZEN_KEYWORDS: &[&str] = &[
  "type",
  "const",
  "pattern",
  "format",
  "minLength",
  "maxLength",
  "minimum",
  "maximum",
  "exclusiveMinimum",
  "exclusiveMaximum",
  "minItems",
  "maxItems",
  "multipleOf",
  "uniqueItems",
  "default",
  "$ref",
  // §2.4: flipping a member between text and bytes is breaking in both
  // directions, and no validator catches it — contentEncoding is
  // annotation-only in draft 2020-12.
  "contentEncoding",
];

fn is_schema_object(v: &Value) -> bool {
  v.is_object()
}

fn subschemas<'a>(node: &'a Value, path: &str) -> Vec<(String, &'a Value)> {
  let mut out = Vec::new();
  let Some(obj) = node.as_object() else {
    return out;
  };
  for key in SUBSCHEMA_MAPS {
    if let Some(map) = obj.get(*key).and_then(Value::as_object) {
      for (name, sub) in map {
        if is_schema_object(sub) {
          out.push((format!("{path}/{key}/{name}"), sub));
        }
      }
    }
  }
  for key in SINGLE_SUBSCHEMA {
    if let Some(sub) = obj.get(*key)
      && is_schema_object(sub)
    {
      out.push((format!("{path}/{key}"), sub));
    }
  }
  for key in SUBSCHEMA_ARRAYS {
    if let Some(arr) = obj.get(*key).and_then(Value::as_array) {
      for (i, sub) in arr.iter().enumerate() {
        if is_schema_object(sub) {
          out.push((format!("{path}/{key}/{i}"), sub));
        }
      }
    }
  }
  out
}

fn title_is_type_shaped(title: &str) -> bool {
  let mut chars = title.chars();
  matches!(chars.next(), Some(c) if c.is_ascii_uppercase())
    && title.len() <= 40
    && chars.all(|c| c.is_ascii_alphanumeric())
}

fn lint_node(path: &str, node: &Value, out: &mut Vec<Violation>) {
  let Some(obj) = node.as_object() else { return };

  // §2.2 rule 1: open vocabularies are strings, never `enum`.
  if obj.contains_key("enum") {
    out.push(Violation {
      path: path.into(),
      message: "'enum' closes a vocabulary; use a string with 'x-known-values' (spec §2.2 rule 1)".into(),
    });
  }
  // §2.2 rule 2: unknown members are ignored, never rejected.
  if obj.get("additionalProperties") == Some(&Value::Bool(false)) {
    out.push(Violation {
      path: path.into(),
      message: "'additionalProperties: false' rejects unknown members (spec §2.2 rule 2)".into(),
    });
  }
  // x-known-values is advisory metadata on an open string vocabulary.
  if let Some(known) = obj.get("x-known-values") {
    let strings: Option<Vec<&str>> = known
      .as_array()
      .map(|a| a.iter().filter_map(Value::as_str).collect());
    let all_strings = matches!((&strings, known.as_array()), (Some(s), Some(a)) if s.len() == a.len());
    let unique = strings
      .as_ref()
      .is_some_and(|s| s.iter().collect::<BTreeSet<_>>().len() == s.len());
    if !all_strings || !unique {
      out.push(Violation {
        path: path.into(),
        message: "'x-known-values' must be an array of unique strings".into(),
      });
    }
    if obj.get("type").and_then(Value::as_str) != Some("string") {
      out.push(Violation {
        path: path.into(),
        message: "'x-known-values' belongs on a schema with type 'string' (spec §2.2 rule 1)".into(),
      });
    }
  }
  // §2.4: bytes members are declared exactly one way, so that every reader
  // decodes them the same way.
  if let Some(encoding) = obj.get("contentEncoding") {
    if encoding.as_str() != Some("base64") {
      out.push(Violation {
        path: path.into(),
        message: format!(
          "contentEncoding {encoding} is not the specified bytes projection 'base64' (spec §2.4)"
        ),
      });
    }
    if obj.get("type").and_then(Value::as_str) != Some("string") {
      out.push(Violation {
        path: path.into(),
        message: "a bytes member is a 'string' carrying base64 (spec §2.4)".into(),
      });
    }
  }
  // §2.2 rule 5: titles are short and type-shaped.
  if let Some(title) = obj.get("title") {
    match title.as_str() {
      Some(t) if title_is_type_shaped(t) => {}
      _ => out.push(Violation {
        path: path.into(),
        message: format!("title {title} is not short and type-shaped (spec §2.2 rule 5)"),
      }),
    }
  }
  // §2.2 rule 6: schemas are self-contained.
  if let Some(r) = obj.get("$ref").and_then(Value::as_str)
    && r.contains("://")
  {
    out.push(Violation {
      path: path.into(),
      message: format!("remote $ref '{r}' (spec §2.2 rule 6)"),
    });
  }

  for (sub_path, sub) in subschemas(node, path) {
    lint_node(&sub_path, sub, out);
  }
}

/// Lint one schema document against the authoring rules (spec §2.2).
pub fn lint_document(doc: &Value) -> Vec<Violation> {
  let mut out = Vec::new();
  let root = doc.as_object();
  if root.is_none_or(|o| !o.contains_key("$id")) {
    out.push(Violation {
      path: "#".into(),
      message: "root '$id' is required".into(),
    });
  }
  if root.is_none_or(|o| !o.contains_key("title")) {
    out.push(Violation {
      path: "#".into(),
      message: "root 'title' is required (spec §2.2 rule 5)".into(),
    });
  }
  if let Some(defs) = root.and_then(|o| o.get("$defs")).and_then(Value::as_object) {
    for (name, def) in defs {
      if def.as_object().is_none_or(|d| !d.contains_key("title")) {
        out.push(Violation {
          path: format!("#/$defs/{name}"),
          message: "every $def needs a 'title' (spec §2.2 rule 5)".into(),
        });
      }
    }
  }
  lint_node("#", doc, &mut out);
  out
}

fn string_set(v: Option<&Value>) -> BTreeSet<String> {
  v.and_then(Value::as_array)
    .map(|a| a.iter().filter_map(Value::as_str).map(String::from).collect())
    .unwrap_or_default()
}

fn diff_node(path: &str, base: &Value, head: &Value, out: &mut Vec<Violation>) {
  let (Some(base_obj), Some(head_obj)) = (base.as_object(), head.as_object()) else {
    if base != head {
      out.push(Violation {
        path: path.into(),
        message: "schema node changed shape (spec §11.2)".into(),
      });
    }
    return;
  };

  // Frozen constraint keywords: any change, including adding or dropping
  // one, changes what a well-formed frame means (§11.2).
  for key in FROZEN_KEYWORDS {
    if base_obj.get(*key) != head_obj.get(*key) {
      out.push(Violation {
        path: format!("{path}/{key}"),
        message: format!("'{key}' changed; constraints are frozen within a version (spec §11.2)"),
      });
    }
  }

  // Required member sets are frozen (§2.2 rule 4, §11.2).
  if string_set(base_obj.get("required")) != string_set(head_obj.get("required")) {
    out.push(Violation {
      path: format!("{path}/required"),
      message: "'required' set changed (spec §11.2)".into(),
    });
  }

  // Open vocabularies only grow (§11.2).
  let base_known = string_set(base_obj.get("x-known-values"));
  let head_known = string_set(head_obj.get("x-known-values"));
  let removed: Vec<_> = base_known.difference(&head_known).collect();
  if !removed.is_empty() {
    out.push(Violation {
      path: format!("{path}/x-known-values"),
      message: format!("known values removed: {removed:?} (spec §11.2)"),
    });
  }

  // Named subschemas: removal is breaking, addition is the designed-for
  // case, common members are diffed recursively.
  for key in SUBSCHEMA_MAPS {
    let base_map = base_obj.get(*key).and_then(Value::as_object);
    let head_map = head_obj.get(*key).and_then(Value::as_object);
    for (name, base_sub) in base_map.into_iter().flatten() {
      match head_map.and_then(|m| m.get(name)) {
        None => out.push(Violation {
          path: format!("{path}/{key}/{name}"),
          message: "member removed (spec §11.2: deprecate, never remove)".into(),
        }),
        Some(head_sub) => {
          diff_node(&format!("{path}/{key}/{name}"), base_sub, head_sub, out);
        }
      }
    }
  }

  // Single-subschema keywords: presence is frozen, contents are diffed.
  for key in SINGLE_SUBSCHEMA {
    match (base_obj.get(*key), head_obj.get(*key)) {
      (None, None) => {}
      (Some(b), Some(h)) => diff_node(&format!("{path}/{key}"), b, h, out),
      _ => out.push(Violation {
        path: format!("{path}/{key}"),
        message: format!("'{key}' added or removed; constraints are frozen (spec §11.2)"),
      }),
    }
  }

  // Combinator arrays: existing alternatives are frozen in place; new ones
  // may be appended (a new frame type or branch, capability-gated).
  for key in SUBSCHEMA_ARRAYS {
    let base_arr = base_obj.get(*key).and_then(Value::as_array);
    let head_arr = head_obj.get(*key).and_then(Value::as_array);
    match (base_arr, head_arr) {
      (None, _) => {}
      (Some(_), None) => out.push(Violation {
        path: format!("{path}/{key}"),
        message: format!("'{key}' removed (spec §11.2)"),
      }),
      (Some(b), Some(h)) if h.len() < b.len() => out.push(Violation {
        path: format!("{path}/{key}"),
        message: "alternatives removed (spec §11.2)".into(),
      }),
      (Some(b), Some(h)) => {
        for (i, (bs, hs)) in b.iter().zip(h).enumerate() {
          diff_node(&format!("{path}/{key}/{i}"), bs, hs, out);
        }
      }
    }
  }
}

/// Diff two versions of the same schema document under the additive-evolution
/// rules (spec §11.2). `base` is the published version, `head` the proposed one.
pub fn diff_documents(base: &Value, head: &Value) -> Vec<Violation> {
  let mut out = Vec::new();
  diff_node("#", base, head, &mut out);
  out
}

#[cfg(test)]
mod tests {
  use super::*;
  use pretty_assertions::assert_eq;
  use serde_json::json;

  fn messages(v: &[Violation]) -> Vec<String> {
    v.iter().map(ToString::to_string).collect()
  }

  #[test]
  fn clean_schema_lints_clean() {
    let doc = json!({
        "$id": "https://pact.io/janus/protocol/v1/example",
        "title": "Example",
        "type": "object",
        "required": ["kind"],
        "properties": {
            "kind": { "type": "string", "x-known-values": ["a", "b"] }
        },
        "$defs": {
            "Sub": { "title": "Sub", "type": "object" }
        }
    });
    assert_eq!(lint_document(&doc), vec![]);
  }

  #[test]
  fn lint_rejects_closed_world_keywords() {
    let doc = json!({
        "$id": "x", "title": "Example",
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "kind": { "type": "string", "enum": ["a"] }
        }
    });
    let out = messages(&lint_document(&doc));
    assert!(out.iter().any(|m| m.contains("additionalProperties")), "{out:?}");
    assert!(out.iter().any(|m| m.contains("'enum'")), "{out:?}");
  }

  #[test]
  fn lint_rejects_prose_titles_remote_refs_and_missing_ids() {
    let doc = json!({
        "title": "Request envelope (v1). Operations are open.",
        "properties": {
            "body": { "$ref": "https://example.com/other.json" }
        }
    });
    let out = messages(&lint_document(&doc));
    assert!(out.iter().any(|m| m.contains("$id")), "{out:?}");
    assert!(out.iter().any(|m| m.contains("type-shaped")), "{out:?}");
    assert!(out.iter().any(|m| m.contains("remote $ref")), "{out:?}");
  }

  #[test]
  fn lint_requires_string_type_under_known_values() {
    let doc = json!({
        "$id": "x", "title": "Example",
        "properties": { "n": { "type": "integer", "x-known-values": ["1"] } }
    });
    let out = messages(&lint_document(&doc));
    assert!(out.iter().any(|m| m.contains("type 'string'")), "{out:?}");
  }

  #[test]
  fn lint_accepts_a_well_formed_bytes_member() {
    let doc = json!({
        "$id": "x", "title": "Example",
        "properties": { "payload": { "type": "string", "contentEncoding": "base64" } }
    });
    assert_eq!(lint_document(&doc), vec![]);
  }

  #[test]
  fn lint_rejects_other_encodings_and_non_string_bytes() {
    let doc = json!({
        "$id": "x", "title": "Example",
        "properties": {
            "a": { "type": "string", "contentEncoding": "base64url" },
            "b": { "type": "array", "contentEncoding": "base64" }
        }
    });
    let out = messages(&lint_document(&doc));
    assert!(out.iter().any(|m| m.contains("base64url")), "{out:?}");
    assert!(
      out.iter().any(|m| m.contains("'string' carrying base64")),
      "{out:?}"
    );
  }

  #[test]
  fn diff_rejects_the_text_bytes_flip() {
    let base = json!({ "properties": { "body": { "type": "string" } } });
    let head = json!({
        "properties": { "body": { "type": "string", "contentEncoding": "base64" } }
    });
    let out = messages(&diff_documents(&base, &head));
    assert!(out.iter().any(|m| m.contains("/body/contentEncoding")), "{out:?}");
    // …and back the other way.
    let out = messages(&diff_documents(&head, &base));
    assert!(out.iter().any(|m| m.contains("/body/contentEncoding")), "{out:?}");
  }

  #[test]
  fn shipped_v1_schemas_lint_clean() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
      .join("../../Documentation/specs/engine-protocol/schemas/v1");
    let mut checked = 0;
    for entry in std::fs::read_dir(dir).expect("schemas dir") {
      let path = entry.expect("entry").path();
      if path.extension().is_some_and(|e| e == "json") {
        let doc: Value = serde_json::from_str(&std::fs::read_to_string(&path).expect("read")).expect("parse");
        assert_eq!(lint_document(&doc), vec![], "in {}", path.display());
        checked += 1;
      }
    }
    assert!(checked >= 7, "expected the v1 schema set, found {checked}");
  }

  #[test]
  fn additive_changes_pass_diff() {
    let base = json!({
        "type": "object",
        "required": ["code"],
        "properties": {
            "code": { "type": "string", "x-known-values": ["a"] }
        }
    });
    let head = json!({
        "type": "object",
        "required": ["code"],
        "properties": {
            "code": { "type": "string", "x-known-values": ["a", "b"] },
            "retryable": { "type": "boolean" }
        },
        "$defs": { "New": { "title": "New", "type": "object" } }
    });
    assert_eq!(diff_documents(&base, &head), vec![]);
  }

  #[test]
  fn diff_rejects_removals_and_narrowing() {
    let base = json!({
        "type": "object",
        "required": ["code"],
        "properties": {
            "code": { "type": "string", "x-known-values": ["a", "b"] },
            "message": { "type": "string" }
        }
    });
    let head = json!({
        "type": "object",
        "required": ["code", "message"],
        "properties": {
            "code": { "type": "integer", "x-known-values": ["a"] },
            "message": { "type": "string", "minLength": 1 }
        }
    });
    let out = messages(&diff_documents(&base, &head));
    assert!(
      out.iter().any(|m| m.contains("'required' set changed")),
      "{out:?}"
    );
    assert!(out.iter().any(|m| m.contains("known values removed")), "{out:?}");
    assert!(out.iter().any(|m| m.contains("/code/type")), "{out:?}");
    assert!(out.iter().any(|m| m.contains("/message/minLength")), "{out:?}");
  }

  #[test]
  fn diff_rejects_removed_property_and_oneof_alternative() {
    let base = json!({
        "properties": { "old": { "type": "string" } },
        "oneOf": [ { "required": ["ok"] }, { "required": ["error"] } ]
    });
    let head = json!({
        "properties": {},
        "oneOf": [ { "required": ["ok"] } ]
    });
    let out = messages(&diff_documents(&base, &head));
    assert!(out.iter().any(|m| m.contains("/properties/old")), "{out:?}");
    assert!(out.iter().any(|m| m.contains("alternatives removed")), "{out:?}");
  }

  #[test]
  fn diff_allows_appended_alternatives_and_frozen_prefix() {
    let base = json!({ "allOf": [ { "if": { "properties": { "type": { "const": "request" } } }, "then": { "title": "A" } } ] });
    let mut head = base.clone();
    head["allOf"]
      .as_array_mut()
      .unwrap()
      .push(json!({ "if": { "properties": { "type": { "const": "event" } } }, "then": { "title": "B" } }));
    assert_eq!(diff_documents(&base, &head), vec![]);
  }
}
