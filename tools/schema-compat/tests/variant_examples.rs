//! Every ```json block in the variant-semantics specification and its worked
//! examples carries a marker saying what it is (`selection`, `policy`,
//! `params`, `variants`, `sketch`); this test validates each against the
//! schema that owns it — the v1 variant schemas for the first three, and the
//! *shape language's* variant-space schema for `variants`, because a variant
//! space is design 2.2's document and design 2.3 only consumes it. An unmarked
//! block fails rather than silently skipping the check.

use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const MARKERS: [&str; 5] = ["selection", "policy", "params", "variants", "sketch"];

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

/// Both schema sets: 2.3 owns selections and policies, 2.2 owns variant spaces.
fn load_schemas() -> HashMap<String, Value> {
  let mut out = HashMap::new();
  for design in ["variant-semantics", "shape-language"] {
    let dir = specs_dir().join(design).join("schemas/v1");
    for entry in std::fs::read_dir(&dir).expect("schemas dir") {
      let path = entry.expect("entry").path();
      if path.to_string_lossy().ends_with(".schema.json") {
        let doc: Value =
          serde_json::from_str(&std::fs::read_to_string(&path).expect("read")).expect("parse schema");
        let id = doc["$id"].as_str().expect("$id").to_string();
        out.insert(id, doc);
      }
    }
  }
  out
}

fn validator_for(id: &str, schemas: &HashMap<String, Value>) -> jsonschema::Validator {
  assert!(schemas.contains_key(id), "no such schema: {id}");
  jsonschema::options()
    .with_retriever(InMemory(schemas.clone()))
    .build(&json!({ "$ref": id }))
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
  let dir = specs_dir().join("variant-semantics");
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
fn variant_examples_validate_against_the_schemas() {
  let schemas = load_schemas();
  let selection = validator_for(
    "https://pact.io/janus/variant/v1/variant-selection.schema.json",
    &schemas,
  );
  let policy = validator_for(
    "https://pact.io/janus/variant/v1/sampling-policy.schema.json",
    &schemas,
  );
  let params = validator_for(
    "https://pact.io/janus/variant/v1/variant-params.schema.json",
    &schemas,
  );
  let space = validator_for(
    "https://pact.io/janus/shape/v1/variant-space.schema.json",
    &schemas,
  );

  let mut counts: HashMap<String, usize> = HashMap::new();
  for path in markdown_files() {
    let name = path.file_name().unwrap().to_string_lossy().into_owned();
    let markdown = std::fs::read_to_string(&path).expect("read");
    for (i, (marker, block)) in marked_json_blocks(&markdown, &name).iter().enumerate() {
      let context = format!("{name} block {}", i + 1);
      match marker.as_str() {
        "selection" => check(&selection, block, &context),
        "policy" => check(&policy, block, &context),
        "params" => check(&params, block, &context),
        "variants" => check(&space, block, &context),
        // Documents owned by other designs (2.5's pact records, 2.7's hook
        // results, protocol frames): parsed, not validated here.
        "sketch" => {}
        other => panic!("{context}: unknown marker '{other}'"),
      }
      *counts.entry(marker.clone()).or_default() += 1;
    }
  }

  assert!(
    counts.get("selection").copied().unwrap_or_default() >= 2,
    "expected the selection documents, found {counts:?}"
  );
  assert!(
    counts.get("policy").copied().unwrap_or_default() >= 3,
    "expected the policy documents, found {counts:?}"
  );
  assert!(
    counts.get("params").copied().unwrap_or_default() >= 1,
    "expected the variant-params documents, found {counts:?}"
  );
}

/// The selections in the worked example are the algorithm's output, not prose:
/// this pins the arithmetic the spec's tables quote (§3.2, §3.9), so a change
/// to either has to change the other.
#[test]
fn worked_example_selections_are_internally_consistent() {
  let path = specs_dir().join("variant-semantics/examples/order-payload-sampling.md");
  let markdown = std::fs::read_to_string(&path).expect("read");
  let blocks = marked_json_blocks(&markdown, "order-payload-sampling.md");
  let selections: Vec<&Value> = blocks
    .iter()
    .filter(|(m, _)| m == "selection")
    .map(|(_, v)| v)
    .collect();
  assert!(!selections.is_empty(), "no selection documents");

  for selection in selections {
    let report = &selection["report"];
    let variants = selection["variants"].as_array().expect("variants");
    assert_eq!(
      report["selected"].as_u64().expect("selected") as usize,
      variants.len(),
      "report.selected disagrees with the list it describes"
    );
    assert_eq!(
      variants[0]["id"], "base",
      "the base variant is unconditional and first (spec §3.1)"
    );
    let coverage = &report["coverage"];
    assert_eq!(
      coverage["covered"], coverage["targets"],
      "the worked example covers every reachable target"
    );
    // Every id is derivable from its assignment (spec §2.2).
    for variant in variants {
      let derived: Vec<String> = variant["assignment"]
        .as_array()
        .expect("assignment")
        .iter()
        .filter_map(|p| {
          let dimension = p["dimension"].as_str()?;
          let point = p["point"].as_str()?;
          Some(format!("{dimension}={point}"))
        })
        .collect();
      let id = variant["id"].as_str().expect("id");
      if id == "base" {
        continue;
      }
      for part in id.split(';') {
        assert!(
          derived.contains(&part.to_string()),
          "id part '{part}' is not in the assignment it names"
        );
      }
    }
  }
}
