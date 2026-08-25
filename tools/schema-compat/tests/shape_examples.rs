//! Every ```json block in the shape-language specification and its worked
//! examples carries a marker saying what it is (`shape`, `variants`, `value`,
//! `sketch`); this test validates the shape and variant-space documents against
//! the v1 shape schemas, and fails on an unmarked block — so the examples
//! cannot drift from the specified surface, and a new block cannot quietly opt
//! out of the check.

use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const BASE: &str = "https://pact.io/janus/shape/v1/";
const MARKERS: [&str; 4] = ["shape", "variants", "value", "sketch"];

fn shape_language_dir() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Documentation/specs/shape-language")
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
  let dir = shape_language_dir().join("schemas/v1");
  let mut out = HashMap::new();
  for entry in std::fs::read_dir(dir).expect("schemas dir") {
    let path = entry.expect("entry").path();
    if path.to_string_lossy().ends_with(".schema.json") {
      let doc: Value =
        serde_json::from_str(&std::fs::read_to_string(&path).expect("read")).expect("parse schema");
      let id = doc["$id"].as_str().expect("$id").to_string();
      out.insert(id, doc);
    }
  }
  assert_eq!(out.len(), 2, "expected the v1 shape schema set");
  out
}

fn validator_for(reference: &str, schemas: &HashMap<String, Value>) -> jsonschema::Validator {
  jsonschema::options()
    .with_retriever(InMemory(schemas.clone()))
    .build(&json!({ "$ref": format!("{BASE}{reference}") }))
    .unwrap_or_else(|e| panic!("compiling {reference}: {e}"))
}

fn check(validator: &jsonschema::Validator, instance: &Value, context: &str) {
  let problems: Vec<String> = validator
    .iter_errors(instance)
    .map(|e| format!("  {}: {}", e.instance_path(), e))
    .collect();
  assert!(problems.is_empty(), "{context}:\n{}", problems.join("\n"));
}

/// Markdown files here are read line by line: a fence opens with ``` plus an
/// info string, and only `json`-tagged blocks are collected — with the marker
/// word that must follow the language.
fn marked_json_blocks(markdown: &str, file: &str) -> Vec<(String, Value)> {
  let mut blocks = Vec::new();
  // Some(marker, body); an empty marker means a fenced block in some other
  // language, collected and discarded so its contents never open a fence.
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
  let dir = shape_language_dir();
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

#[test]
fn shape_examples_validate_against_the_schemas() {
  let schemas = load_schemas();
  let shape = validator_for("shape.schema.json", &schemas);
  let variant_space = validator_for("variant-space.schema.json", &schemas);

  let mut counts: HashMap<String, usize> = HashMap::new();
  for path in markdown_files() {
    let name = path.file_name().unwrap().to_string_lossy().into_owned();
    let markdown = std::fs::read_to_string(&path).expect("read");
    for (i, (marker, block)) in marked_json_blocks(&markdown, &name).iter().enumerate() {
      let context = format!("{name} block {}", i + 1);
      match marker.as_str() {
        "shape" => check(&shape, block, &context),
        "variants" => check(&variant_space, block, &context),
        // Payloads and other designs' documents: parsed, not validated —
        // their shapes are not this specification's to govern.
        "value" | "sketch" => {}
        other => panic!("{context}: unknown marker '{other}'"),
      }
      *counts.entry(marker.clone()).or_default() += 1;
    }
  }

  assert!(
    counts.get("shape").copied().unwrap_or_default() >= 15,
    "expected a real set of shape documents, found {counts:?}"
  );
  assert!(
    counts.get("variants").copied().unwrap_or_default() >= 4,
    "expected the variant-space documents, found {counts:?}"
  );
}
