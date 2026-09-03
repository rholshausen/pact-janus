//! Every ```json block in the subsumption-check specification and its worked
//! examples carries a marker naming what it is (`provider-shape`, `report`,
//! `policy`, `finding`); this test validates each against the schema that
//! owns it. An unmarked block fails rather than silently skipping the check.
//! `sketch` blocks are owned by other designs (2.2's shapes, 2.3's
//! exclusions) and are parsed but not validated here.
//!
//! Two checks do a job a schema cannot. Spec §4.3 fixes 'severity' as a
//! function of 'verdict' (finding/no, review/unknown, advisory/yes); an
//! example that disagreed with its own rule would document a checker that
//! computes severity some other way. Spec §6.2 fixes an interaction's
//! aggregate 'verdict' as the Kleene conjunction of its findings' verdicts;
//! this pins the worked report's arithmetic so a change to the findings list
//! has to change the aggregate too.

use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const MARKERS: [&str; 4] = ["provider-shape", "report", "policy", "finding"];

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
  let dir = specs_dir().join("subsumption-check/schemas/v1");
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

fn schema_for(marker: &str) -> Option<&'static str> {
  match marker {
    "provider-shape" => Some("https://pact.io/janus/subsumption/v1/provider-shape.schema.json"),
    "report" => Some("https://pact.io/janus/subsumption/v1/subsumption-report.schema.json"),
    "policy" => Some("https://pact.io/janus/subsumption/v1/subsumption-policy.schema.json"),
    "finding" => Some("https://pact.io/janus/subsumption/v1/finding.schema.json"),
    "sketch" => None,
    other => panic!("unknown marker '{other}'"),
  }
}

fn validator_for(id: &str, schemas: &HashMap<String, Value>) -> jsonschema::Validator {
  assert!(schemas.contains_key(id), "no such schema: {id}");
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
          MARKERS.contains(&marker) || marker == "sketch",
          "{file}: a ```json block is tagged '{marker}'; every block must carry one of {MARKERS:?} or 'sketch'"
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
  let dir = specs_dir().join("subsumption-check");
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
fn subsumption_examples_validate_against_the_schemas() {
  let schemas = load_schemas();
  let mut validators: HashMap<&str, jsonschema::Validator> = HashMap::new();
  let mut counts: HashMap<String, usize> = HashMap::new();

  for (context, marker, value) in blocks() {
    if let Some(id) = schema_for(&marker) {
      let validator = validators
        .entry(id)
        .or_insert_with(|| validator_for(id, &schemas));
      check(validator, &value, &context);
    }
    *counts.entry(marker).or_default() += 1;
  }

  for (marker, min) in [
    ("provider-shape", 1),
    ("report", 1),
    ("policy", 1),
    ("finding", 3),
  ] {
    assert!(
      counts.get(marker).copied().unwrap_or_default() >= min,
      "expected at least {min} '{marker}' block(s), found {counts:?}"
    );
  }
}

/// Spec §4.3: severity is a pure function of verdict. An example that got
/// this wrong would document a checker inventing a fourth relationship the
/// specification does not define.
#[test]
fn finding_severity_agrees_with_verdict() {
  let mut checked = 0;
  for (context, marker, value) in blocks() {
    if marker != "finding" {
      continue;
    }
    let verdict = value["verdict"].as_str().expect("verdict");
    let severity = value["severity"].as_str().expect("severity");
    let expected = match verdict {
      "no" => "finding",
      "unknown" => "review",
      "yes" => "advisory",
      other => panic!("{context}: unknown verdict '{other}'"),
    };
    assert_eq!(
      severity, expected,
      "{context}: verdict '{verdict}' must carry severity '{expected}' (spec §4.3), found '{severity}'"
    );
    if verdict == "yes" {
      assert!(
        value["excluded-by"].as_array().is_some_and(|a| !a.is_empty()),
        "{context}: an 'advisory' finding must carry a non-empty 'excluded-by' (spec §5) — a bare 'yes' is not a finding at all"
      );
    }
    checked += 1;
  }
  assert!(
    checked >= 3,
    "expected standalone finding examples, found {checked}"
  );
}

/// Spec §6.2: an interaction's aggregate verdict is the Kleene conjunction of
/// its findings' verdicts (no dominates unknown dominates yes), ignoring
/// 'advisory' entries, which by construction carry verdict 'yes' and change
/// nothing (spec §4.3). This is the worked report's own arithmetic, checked
/// so the table in the prose cannot drift from the JSON it describes.
#[test]
fn report_verdicts_are_the_kleene_conjunction_of_their_findings() {
  let mut checked = 0;
  for (context, marker, value) in blocks() {
    if marker != "report" {
      continue;
    }
    for interaction in value["interactions"].as_array().expect("interactions") {
      let findings = interaction["findings"].as_array().expect("findings");
      let verdicts: Vec<&str> = findings
        .iter()
        .map(|f| f["verdict"].as_str().expect("verdict"))
        .collect();
      let expected = if verdicts.contains(&"no") {
        "no"
      } else if verdicts.contains(&"unknown") {
        "unknown"
      } else {
        "yes"
      };
      let actual = interaction["verdict"].as_str().expect("verdict");
      assert_eq!(
        actual,
        expected,
        "{context}: interaction '{}' verdict '{actual}' disagrees with its findings' Kleene conjunction '{expected}'",
        interaction["description"].as_str().unwrap_or("?")
      );
      checked += 1;
    }
  }
  assert!(
    checked >= 1,
    "expected at least one report interaction, found {checked}"
  );
}
