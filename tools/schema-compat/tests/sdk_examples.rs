//! Every ```json block in the SDK specification and its worked examples
//! carries a marker naming what it is — only `primitive` exists in this
//! design — and validates against the schema's `Primitive` definition. An
//! unmarked block fails rather than silently skipping the check.
//!
//! Two checks do a job a schema cannot. Spec §3.2 fixes 'produces[].kind'
//! against which sibling member is set (the schema's own if/then already
//! enforces presence; this test additionally forbids a *stray* sibling from
//! a different kind, which if/then alone does not). Spec §6.1 fixes that a
//! 'kept' facade mapping still requires 'reason' — the schema already
//! requires it structurally, but this test pins the stronger prose rule that
//! 'dropped' and 'adapted' entries must also name what a user does next, not
//! just why the old behaviour doesn't fit.

use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const MARKERS: [&str; 1] = ["primitive"];

fn specs_dir() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Documentation/specs")
}

struct InMemory(HashMap<String, Value>);

impl jsonschema::Retrieve for InMemory {
  fn retrieve(
    &self,
    uri: &jsonschema::Uri<String>,
  ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    self
      .0
      .get(uri.as_str())
      .cloned()
      .ok_or_else(|| format!("schema not registered: {uri}").into())
  }
}

fn load_schemas() -> HashMap<String, Value> {
  let mut out = HashMap::new();
  let dir = specs_dir().join("sdk-specification/schemas/v1");
  for entry in std::fs::read_dir(&dir).expect("schemas dir") {
    let path = entry.expect("entry").path();
    if path.to_string_lossy().ends_with(".schema.json") {
      let doc: Value =
        serde_json::from_str(&std::fs::read_to_string(&path).expect("read")).expect("parse schema");
      let id = doc["$id"].as_str().expect("$id").to_string();
      out.insert(id, doc);
    }
  }
  out
}

fn validator_for(id: &str, schemas: &HashMap<String, Value>) -> jsonschema::Validator {
  let base = id.split('#').next().expect("base id");
  assert!(schemas.contains_key(base), "no such schema: {base}");
  jsonschema::options()
    .with_retriever(InMemory(schemas.clone()))
    .build(&serde_json::json!({ "$ref": id }))
    .unwrap_or_else(|e| panic!("compiling {id}: {e}"))
}

fn check(validator: &jsonschema::Validator, instance: &Value, context: &str) {
  let problems: Vec<String> = validator
    .iter_errors(instance)
    .map(|e| format!("  {}: {}", e.instance_path(), e))
    .collect();
  assert!(problems.is_empty(), "{context}:\n{}", problems.join("\n"));
}

/// Fences are read line by line; only `json`-tagged blocks are collected, with
/// the marker word that must follow the language.
fn marked_json_blocks(markdown: &str, file: &str) -> Vec<(String, Value)> {
  let mut blocks = Vec::new();
  let mut open: Option<(String, String)> = None;
  for line in markdown.lines() {
    if let Some((_, buf)) = open.as_mut() {
      if !line.starts_with("```") {
        buf.push_str(line);
        buf.push('\n');
        continue;
      }
      let (marker, buf) = open.take().expect("open block");
      if !marker.is_empty() {
        let value = serde_json::from_str(&buf)
          .unwrap_or_else(|e| panic!("{file}: '{marker}' block is not valid JSON: {e}"));
        blocks.push((marker, value));
      }
    } else if let Some(info) = line.strip_prefix("```") {
      let mut words = info.split_whitespace();
      let language = words.next().unwrap_or_default();
      let marker = words.next().unwrap_or_default();
      if language == "json" {
        assert!(
          MARKERS.contains(&marker),
          "{file}: a ```json block is tagged '{marker}'; every block must carry one of {MARKERS:?}"
        );
        open = Some((marker.to_string(), String::new()));
      } else {
        open = Some((String::new(), String::new()));
      }
    }
  }
  assert!(open.is_none(), "{file}: unterminated fenced block");
  blocks
}

fn markdown_files() -> Vec<PathBuf> {
  let dir = specs_dir().join("sdk-specification");
  let mut files = vec![dir.join("spec.md")];
  for entry in std::fs::read_dir(dir.join("examples")).expect("examples dir") {
    let path = entry.expect("entry").path();
    if path.extension().is_some_and(|e| e == "md") {
      files.push(path);
    }
  }
  files.sort();
  files
}

fn blocks() -> Vec<(String, String, Value)> {
  let mut out = Vec::new();
  for path in markdown_files() {
    let name = path.file_name().unwrap().to_string_lossy().into_owned();
    let markdown = std::fs::read_to_string(&path).expect("read");
    for (i, (marker, value)) in marked_json_blocks(&markdown, &name).into_iter().enumerate() {
      out.push((format!("{name} block {}", i + 1), marker, value));
    }
  }
  out
}

#[test]
fn sdk_examples_validate_against_the_schema() {
  let schemas = load_schemas();
  let validator = validator_for(
    "https://pact.io/janus/sdk/v1/behavioural-spec.schema.json#/$defs/Primitive",
    &schemas,
  );
  let mut count = 0;
  for (context, marker, value) in blocks() {
    assert_eq!(marker, "primitive", "{context}: unexpected marker '{marker}'");
    check(&validator, &value, &context);
    count += 1;
  }
  assert!(
    count >= 10,
    "expected primitive examples across the spec and worked examples, found {count}"
  );
}

/// Spec §3.2: 'produces[].kind' selects which sibling applies. The schema's
/// if/then already requires the matching sibling to be present; this test
/// additionally forbids a sibling from a *different* kind being set at the
/// same time, which if/then alone does not rule out.
#[test]
fn produces_entries_carry_only_their_own_kinds_sibling() {
  let siblings = [
    ("shape-operator", "operator"),
    ("protocol-operation", "operation"),
    ("spec-member", "path"),
  ];
  let mut checked = 0;
  for (context, _marker, value) in blocks() {
    for produces in value["produces"].as_array().unwrap_or(&Vec::new()) {
      let kind = produces["kind"].as_str().expect("kind");
      for (other_kind, other_field) in siblings {
        if other_kind != kind {
          assert!(
            produces.get(other_field).is_none(),
            "{context}: a '{kind}' produces entry also sets '{other_field}', which belongs to '{other_kind}'"
          );
        }
      }
      checked += 1;
    }
  }
  assert!(
    checked >= 10,
    "expected produces entries to check, found {checked}"
  );
}

/// Spec §6.1: 'adapted' names its supplied default and 'dropped' names a
/// migration path — a reason that only restates *why* the old behaviour
/// doesn't fit, without saying what a user does about it, has not actually
/// given the guidance §6.1 promises.
#[test]
fn non_kept_facade_mappings_name_a_way_forward() {
  let mut checked = 0;
  for (context, _marker, value) in blocks() {
    for facade in value["facade"].as_array().unwrap_or(&Vec::new()) {
      let status = facade["status"].as_str().expect("status");
      let reason = facade["reason"].as_str().expect("reason");
      if status == "adapted" || status == "dropped" {
        let has_way_forward = reason.contains("Migrat")
          || reason.contains("migrat")
          || reason.contains("adapts to")
          || reason.contains("Adapts to");
        assert!(
          has_way_forward,
          "{context}: a '{status}' facade mapping's reason does not name a migration or a supplied default: {reason:?}"
        );
        checked += 1;
      }
    }
  }
  assert!(
    checked >= 2,
    "expected 'adapted'/'dropped' facade mappings to check, found {checked}"
  );
}
