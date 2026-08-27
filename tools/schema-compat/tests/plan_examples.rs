//! Every ```json block in the plan-grammar specification and its worked
//! examples carries a marker saying what it is (`plan`, `corpus`, `shape`,
//! `sketch`); this test validates each against the schema that owns it. `shape`
//! blocks go to the *shape language's* schema, because a plan is compiled from
//! design 2.2's documents and this design only consumes them. An unmarked block
//! fails rather than silently skipping the check.
//!
//! The ```text blocks are the plan text forms (spec §3). They are not validated
//! structurally — there is no parser yet — but they are checked for the one
//! property a renderer must hold: balanced parentheses, so a truncated example
//! cannot sit in the specification looking complete.

use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const MARKERS: [&str; 4] = ["plan", "corpus", "shape", "sketch"];

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
  for (design, version) in [("plan-grammar", "v0"), ("shape-language", "v1")] {
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

/// Collects fenced blocks: `json` blocks with their required marker, and `text`
/// blocks (the plan forms) under the marker "text".
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
        "text" => open = Some(("text".to_string(), String::new())),
        _ => open = Some((String::new(), String::new())),
      }
    }
  }
  assert!(open.is_none(), "{file}: unterminated fenced block");
  blocks
}

fn markdown_files() -> Vec<PathBuf> {
  let dir = specs_dir().join("plan-grammar");
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

/// Parentheses outside quoted strings must balance. The text form nests them one
/// per node, so an unbalanced block is a truncated or hand-mangled plan.
fn parens_balance(text: &str) -> i32 {
  let mut depth = 0;
  let mut in_quote = false;
  let mut escaped = false;
  for ch in text.chars() {
    if escaped {
      escaped = false;
      continue;
    }
    match ch {
      '\\' => escaped = true,
      '\'' => in_quote = !in_quote,
      '(' if !in_quote => depth += 1,
      ')' if !in_quote => depth -= 1,
      _ => {}
    }
  }
  depth
}

#[test]
fn plan_examples_validate_against_the_schemas() {
  let schemas = load_schemas();
  let plan = validator_for("https://pact.io/janus/plan/v0/plan.schema.json", &schemas);
  let corpus = validator_for("https://pact.io/janus/plan/v0/corpus-case.schema.json", &schemas);
  let shape = validator_for("https://pact.io/janus/shape/v1/shape.schema.json", &schemas);

  let mut counts: HashMap<String, usize> = HashMap::new();
  for path in markdown_files() {
    let name = path.file_name().unwrap().to_string_lossy().into_owned();
    let markdown = std::fs::read_to_string(&path).expect("read");
    for (i, (marker, body)) in fenced_blocks(&markdown, &name).iter().enumerate() {
      let context = format!("{name} block {}", i + 1);
      if marker == "text" {
        // Whole plans start with a bare '(' on their own line; fragments and
        // diff reports are legitimately unbalanced and are not checked.
        if body.lines().next().map(str::trim) == Some("(") {
          assert_eq!(
            parens_balance(body),
            0,
            "{context}: plan text has unbalanced parentheses"
          );
        }
      } else {
        let value: Value = serde_json::from_str(body)
          .unwrap_or_else(|e| panic!("{context}: '{marker}' block is not valid JSON: {e}"));
        match marker.as_str() {
          "plan" => check(&plan, &value, &context),
          "corpus" => check(&corpus, &value, &context),
          "shape" => check(&shape, &value, &context),
          "sketch" => {}
          other => panic!("{context}: unknown marker '{other}'"),
        }
      }
      *counts.entry(marker.clone()).or_default() += 1;
    }
  }

  assert!(
    counts.get("corpus").copied().unwrap_or_default() >= 2,
    "expected the corpus-case documents, found {counts:?}"
  );
  assert!(
    counts.get("text").copied().unwrap_or_default() >= 5,
    "expected the plan text forms, found {counts:?}"
  );
}

/// Every action the specification names in its tables must appear in the action
/// vocabulary of §4, and every action the worked examples use must be one the
/// specification names. An example that invents an action is a spec gap.
#[test]
fn worked_examples_use_only_specified_actions() {
  let spec = std::fs::read_to_string(specs_dir().join("plan-grammar/spec.md")).expect("read");
  let named: Vec<String> = spec
    .match_indices('`')
    .filter_map(|(i, _)| {
      let rest = &spec[i + 1..];
      let end = rest.find('`')?;
      let token = &rest[..end];
      let ok = !token.is_empty()
        && token
          .chars()
          .all(|c| c.is_ascii_lowercase() || c == ':' || c == '-' || c == '*');
      ok.then(|| token.to_string())
    })
    .collect();

  for path in markdown_files() {
    let name = path.file_name().unwrap().to_string_lossy().into_owned();
    let markdown = std::fs::read_to_string(&path).expect("read");
    for (marker, body) in fenced_blocks(&markdown, &name) {
      if marker != "text" {
        continue;
      }
      for line in body.lines() {
        let Some(rest) = line.trim_start().strip_prefix('%') else {
          continue;
        };
        let action: String = rest
          .chars()
          .take_while(|c| c.is_ascii_lowercase() || *c == ':' || *c == '-')
          .collect();
        if action.is_empty() {
          continue;
        }
        // A family named with a wildcard covers its members.
        let family = action.split(':').next().unwrap_or_default();
        assert!(
          named.iter().any(|n| *n == action || n == &format!("{family}:*")),
          "{name}: action '%{action}' is used but never named in the specification"
        );
      }
    }
  }
}
