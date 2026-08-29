//! Every ```json block in the contract-file specification and its worked
//! examples carries a marker saying what it is (`contract`, `findings`,
//! `shape`, `params`, `selection`, `sketch`); this test validates each against
//! the schema that owns it. `shape` and `params` blocks go to designs 2.2's and
//! 2.3's schemas, because a contract stores those documents and does not
//! define them. An unmarked block fails rather than silently skipping.
//!
//! The load-bearing check is the last one: a contract's `selection` member is
//! design 2.3's variant-selection document with this design's evidence added
//! (spec §5.2), so every one is validated against *both* schemas. If the two
//! ever disagree about that document, this is where it surfaces.

use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const MARKERS: [&str; 6] = ["contract", "findings", "shape", "params", "selection", "sketch"];

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
  for (design, version) in [
    ("contract-file", "v1"),
    ("shape-language", "v1"),
    ("variant-semantics", "v1"),
  ] {
    let dir = specs_dir().join(design).join("schemas").join(version);
    for entry in std::fs::read_dir(&dir).expect("schemas dir") {
      let path = entry.expect("entry").path();
      if path.to_string_lossy().ends_with(".schema.json") {
        let doc: Value = serde_json::from_str(&std::fs::read_to_string(&path).expect("read")).expect("parse");
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

fn fenced_blocks(markdown: &str, file: &str) -> Vec<(String, String)> {
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
        blocks.push((marker, buf));
      }
    } else if let Some(info) = line.strip_prefix("```") {
      let mut words = info.split_whitespace();
      let language = words.next().unwrap_or_default();
      let marker = words.next().unwrap_or_default();
      match language {
        "json" => {
          assert!(
            MARKERS.contains(&marker),
            "{file}: a ```json block is tagged '{marker}'; every block must carry one of {MARKERS:?}"
          );
          open = Some((marker.to_string(), String::new()));
        }
        _ => open = Some((String::new(), String::new())),
      }
    }
  }
  assert!(open.is_none(), "{file}: unterminated fenced block");
  blocks
}

fn markdown_files() -> Vec<PathBuf> {
  let dir = specs_dir().join("contract-file");
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

fn contracts() -> Vec<(String, Value)> {
  let mut out = Vec::new();
  for path in markdown_files() {
    let name = path.file_name().unwrap().to_string_lossy().into_owned();
    let markdown = std::fs::read_to_string(&path).expect("read");
    for (i, (marker, body)) in fenced_blocks(&markdown, &name).iter().enumerate() {
      if marker == "contract" {
        let value: Value = serde_json::from_str(body).expect("contract block is not valid JSON");
        out.push((format!("{name} block {}", i + 1), value));
      }
    }
  }
  out
}

#[test]
fn contract_examples_validate_against_the_schemas() {
  let schemas = load_schemas();
  let contract = validator_for("https://pact.io/janus/contract/v1/contract.schema.json", &schemas);
  let findings = validator_for(
    "https://pact.io/janus/contract/v1/upgrade-findings.schema.json",
    &schemas,
  );
  let shape = validator_for("https://pact.io/janus/shape/v1/shape.schema.json", &schemas);
  let params = validator_for(
    "https://pact.io/janus/variant/v1/variant-params.schema.json",
    &schemas,
  );
  let selection = validator_for(
    "https://pact.io/janus/variant/v1/variant-selection.schema.json",
    &schemas,
  );

  let mut counts: HashMap<String, usize> = HashMap::new();
  for path in markdown_files() {
    let name = path.file_name().unwrap().to_string_lossy().into_owned();
    let markdown = std::fs::read_to_string(&path).expect("read");
    for (i, (marker, body)) in fenced_blocks(&markdown, &name).iter().enumerate() {
      let context = format!("{name} block {}", i + 1);
      let value: Value = serde_json::from_str(body)
        .unwrap_or_else(|e| panic!("{context}: '{marker}' block is not valid JSON: {e}"));
      match marker.as_str() {
        "contract" => check(&contract, &value, &context),
        "findings" => check(&findings, &value, &context),
        "shape" => check(&shape, &value, &context),
        "params" => check(&params, &value, &context),
        "selection" => check(&selection, &value, &context),
        "sketch" => {}
        other => panic!("{context}: unknown marker '{other}'"),
      }
      *counts.entry(marker.clone()).or_default() += 1;
    }
  }

  assert!(
    counts.get("contract").copied().unwrap_or_default() >= 2,
    "expected the worked contracts, found {counts:?}"
  );
  assert!(
    counts.get("findings").copied().unwrap_or_default() >= 1,
    "expected the upgrade findings, found {counts:?}"
  );
}

/// Spec §5.2: a contract's `selection` is design 2.3's document plus this
/// design's evidence. Both schemas must accept every one.
#[test]
fn recorded_selections_are_also_variant_selection_documents() {
  let schemas = load_schemas();
  let selection = validator_for(
    "https://pact.io/janus/variant/v1/variant-selection.schema.json",
    &schemas,
  );
  let mut checked = 0;
  for (context, doc) in contracts() {
    for (i, interaction) in doc["interactions"]
      .as_array()
      .expect("interactions")
      .iter()
      .enumerate()
    {
      check(
        &selection,
        &interaction["selection"],
        &format!("{context}, interaction {i} selection"),
      );
      checked += 1;
    }
  }
  assert!(checked >= 2, "expected recorded selections, found {checked}");
}

/// Spec §5.2: labels are never recorded. Design 2.3 renders them for humans and
/// they change meaning when the shape grows a dimension, so a contract carrying
/// one is a contract that will mislead later.
#[test]
fn recorded_variants_carry_no_labels() {
  for (context, doc) in contracts() {
    for interaction in doc["interactions"].as_array().expect("interactions") {
      for variant in interaction["selection"]["variants"].as_array().expect("variants") {
        assert!(
          variant.get("label").is_none(),
          "{context}: variant '{}' records a label",
          variant["id"]
        );
      }
    }
  }
}

/// Spec §5.2 and §5.1: the interaction's shapes and each variant's evidence are
/// keyed identically, so a slot can never have a shape with no evidence or
/// evidence with no shape.
#[test]
fn evidence_is_keyed_like_the_shapes() {
  for (context, doc) in contracts() {
    for interaction in doc["interactions"].as_array().expect("interactions") {
      let parts = interaction["parts"].as_object().expect("parts");
      for variant in interaction["selection"]["variants"].as_array().expect("variants") {
        let recorded = variant["parts"].as_object().expect("variant parts");
        let mut shape_keys: Vec<&String> = parts.keys().collect();
        let mut value_keys: Vec<&String> = recorded.keys().collect();
        shape_keys.sort();
        value_keys.sort();
        assert_eq!(
          shape_keys, value_keys,
          "{context}: variant '{}' records parts the interaction does not shape, or vice versa",
          variant["id"]
        );
        for (part, slots) in parts {
          let mut shaped: Vec<&String> = slots.as_object().expect("slots").keys().collect();
          let mut valued: Vec<&String> = recorded[part].as_object().expect("slots").keys().collect();
          shaped.sort();
          valued.sort();
          assert_eq!(
            shaped, valued,
            "{context}: variant '{}' part '{part}' slots do not line up with its shapes",
            variant["id"]
          );
        }
      }
    }
  }
}

/// Spec §4.2: identity is description plus states, and it must be unique.
#[test]
fn interaction_identities_are_unique() {
  for (context, doc) in contracts() {
    let mut seen: Vec<(String, String)> = Vec::new();
    for interaction in doc["interactions"].as_array().expect("interactions") {
      let identity = (
        interaction["description"]
          .as_str()
          .expect("description")
          .to_string(),
        interaction
          .get("states")
          .map(ToString::to_string)
          .unwrap_or_default(),
      );
      assert!(
        !seen.contains(&identity),
        "{context}: two interactions share description '{}' and the same states",
        identity.0
      );
      seen.push(identity);
    }
  }
}

/// Spec §3.2 and ADR 0011: a contract conforms to no pact specification
/// version, so it must not claim one.
#[test]
fn contracts_claim_no_pact_specification_version() {
  for (context, doc) in contracts() {
    assert!(
      doc["metadata"].get("pactSpecification").is_none(),
      "{context}: metadata claims a pactSpecification version"
    );
  }
}

/// Spec §2.3: a writer emits `$format` first, so a conformant file begins with
/// the byte prefix `{"$format":`. JSON Schema cannot express member order; this
/// is the check that does.
#[test]
fn contract_blocks_lead_with_the_format_member() {
  for path in markdown_files() {
    let name = path.file_name().unwrap().to_string_lossy().into_owned();
    let markdown = std::fs::read_to_string(&path).expect("read");
    for (i, (marker, body)) in fenced_blocks(&markdown, &name).iter().enumerate() {
      if marker != "contract" {
        continue;
      }
      let head: String = body.chars().filter(|c| !c.is_whitespace()).take(11).collect();
      assert_eq!(
        head,
        "{\"$format\":",
        "{name} block {}: a contract must begin with the $format member",
        i + 1
      );
    }
  }
}
