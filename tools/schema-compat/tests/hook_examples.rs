//! Every ```json block in the lifecycle-hooks specification and its worked
//! examples carries a marker naming what it is (`hook-config`, `hook-context`,
//! `hook-result`, …); this test validates each against the schema that owns it.
//! An unmarked block fails rather than silently skipping.
//!
//! Five checks do a job a schema cannot, and each of them is a claim the
//! specification makes in prose. §2.3 says every implementation answers with
//! design 2.6's result document — so results are validated against *that*
//! schema, and a divergence between the two designs is a CI failure rather
//! than a Phase 5 discovery. §4.3 says a change is declared before it is
//! applied, and §3 says v1's mutable sets are parts-rooted. ADR 0014 says the
//! resolved configuration carries no template and no path — the property that
//! keeps the file system out of the engine. And `hook-api.d.ts` is the script's
//! view of the context document, which is only true while its members are the
//! context schema's members.

use serde_json::{Value, json};
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

const MARKERS: [&str; 8] = [
  "project-config",
  "hook-config",
  "hook-context",
  "hook-result",
  "hook-invocation",
  "hook-report",
  "capability",
  "engine-error",
];

const HOOKS: &str = "https://pact.io/janus/hooks/v1";
const COMPONENT: &str = "https://pact.io/janus/component/v1";
const PROTOCOL: &str = "https://pact.io/janus/protocol/v1";

/// Marker -> the schema (or `$defs` fragment) that owns that document.
fn schema_for(marker: &str) -> Option<String> {
  let id = match marker {
    "project-config" => format!("{HOOKS}/project-config.schema.json"),
    "hook-config" => format!("{HOOKS}/hook-config.schema.json"),
    "hook-context" => format!("{HOOKS}/hook-context.schema.json"),
    "hook-invocation" => format!("{HOOKS}/hook-report.schema.json#/$defs/HookInvocation"),
    "hook-report" => format!("{HOOKS}/hook-report.schema.json"),
    // Documents this design carries but does not own. A hook result is design
    // 2.6's InvokeResult — the same document a component answers with, which is
    // the whole of the "one interface, four implementations" claim (spec §2.3).
    "hook-result" => format!("{COMPONENT}/hook.schema.json#/$defs/InvokeResult"),
    "capability" => format!("{PROTOCOL}/hello.schema.json#/$defs/Capabilities"),
    "engine-error" => format!("{PROTOCOL}/engine-error.schema.json"),
    "sketch" => return None,
    other => panic!("unknown marker '{other}'"),
  };
  Some(id)
}

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
    ("lifecycle-hooks", "v1"),
    ("component-interfaces", "v1"),
    ("engine-protocol", "v1"),
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
  let base = id.split('#').next().expect("base id");
  assert!(schemas.contains_key(base), "no such schema: {base}");
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
            MARKERS.contains(&marker) || marker == "sketch",
            "{file}: a ```json block is tagged '{marker}'; every block must carry a marker naming what it is"
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
  let dir = specs_dir().join("lifecycle-hooks");
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
    for (i, (marker, body)) in fenced_blocks(&markdown, &name).iter().enumerate() {
      let context = format!("{name} block {}", i + 1);
      let value: Value = serde_json::from_str(body)
        .unwrap_or_else(|e| panic!("{context}: '{marker}' block is not valid JSON: {e}"));
      out.push((context, marker.clone(), value));
    }
  }
  out
}

fn entries(config: &Value) -> impl Iterator<Item = (&String, &Value)> {
  config["hooks"]
    .as_object()
    .expect("hooks")
    .iter()
    .flat_map(|(point, list)| {
      list
        .as_array()
        .expect("hook list")
        .iter()
        .map(move |entry| (point, entry))
    })
}

#[test]
fn hook_examples_validate_against_the_schemas() {
  let schemas = load_schemas();
  let mut validators: HashMap<String, jsonschema::Validator> = HashMap::new();
  let mut counts: HashMap<String, usize> = HashMap::new();

  for (context, marker, value) in blocks() {
    if let Some(id) = schema_for(&marker) {
      let validator = validators
        .entry(id.clone())
        .or_insert_with(|| validator_for(&id, &schemas));
      check(validator, &value, &context);
    }
    *counts.entry(marker).or_default() += 1;
  }

  for required in [
    "project-config",
    "hook-config",
    "hook-context",
    "hook-result",
    "hook-report",
  ] {
    assert!(
      counts.get(required).copied().unwrap_or_default() >= 1,
      "expected a '{required}' block, found {counts:?}"
    );
  }
}

/// Spec §2.3: every implementation answers with design 2.6's InvokeResult.
/// The marker map already validates results against that schema; this makes
/// the count explicit, so the claim cannot be quietly satisfied by an example
/// that has no results in it.
#[test]
fn results_are_the_component_interfaces_result_document() {
  let schemas = load_schemas();
  let result = validator_for(
    &format!("{COMPONENT}/hook.schema.json#/$defs/InvokeResult"),
    &schemas,
  );
  let mut checked = 0;
  for (context, marker, value) in blocks() {
    if marker == "hook-result" {
      check(&result, &value, &context);
      checked += 1;
    }
  }
  assert!(
    checked >= 6,
    "expected hook results across the examples, found {checked}"
  );
}

/// Spec §4.3 and §3: a change is *declared* before it is applied, and v1's
/// mutable sets are parts-rooted. An example that changed something no entry
/// declared would document the opposite of the rule the engine enforces.
#[test]
fn changes_are_declared_and_parts_rooted() {
  let mut declared: BTreeSet<String> = BTreeSet::new();
  let mut changed: Vec<(String, String)> = Vec::new();
  for (context, marker, value) in blocks() {
    match marker.as_str() {
      "hook-config" | "project-config" => {
        for (_point, entry) in entries(&value) {
          for path in entry["changes"].as_array().unwrap_or(&Vec::new()) {
            declared.insert(path.as_str().expect("change path").to_string());
          }
        }
      }
      "hook-result" => {
        if let Some(changes) = value.get("changes").and_then(Value::as_object) {
          changed.extend(changes.keys().map(|k| (context.clone(), k.clone())));
        }
      }
      "hook-context" => {
        for path in value["mutable"].as_array().unwrap_or(&Vec::new()) {
          let path = path.as_str().expect("mutable path");
          assert!(
            path == "parts" || path.starts_with("parts."),
            "{context}: mutable path '{path}' is not parts-rooted; v1 permits no other (spec §3)"
          );
        }
      }
      _ => {}
    }
  }
  assert!(!changed.is_empty(), "expected an example that changes something");
  for (context, path) in &changed {
    assert!(
      declared.contains(path),
      "{context}: changes '{path}', which no configuration entry declares (spec §4.3)"
    );
  }
}

/// ADR 0014: the resolved configuration carries no template and no path. This
/// is the property that keeps the environment and the file system out of the
/// engine, and it is checkable precisely because the authored and resolved
/// forms are two schemas rather than one.
#[test]
fn resolved_configuration_has_no_templates_and_no_paths() {
  let mut checked = 0;
  for (context, marker, value) in blocks() {
    if marker != "hook-config" {
      continue;
    }
    let text = serde_json::to_string(&value).expect("serialise");
    assert!(
      !text.contains("${"),
      "{context}: a resolved hook configuration still carries a '${{…}}' template (ADR 0014)"
    );
    for (_point, entry) in entries(&value) {
      let run = &entry["run"];
      assert!(
        run.get("path").is_none(),
        "{context}: a resolved 'run' names a path; the loader inlines scripts (ADR 0014)"
      );
      if run["kind"] == json!("script") {
        assert!(
          run
            .get("source")
            .and_then(Value::as_str)
            .is_some_and(|s| !s.is_empty()),
          "{context}: a resolved script hook must carry its source inline"
        );
      }
      checked += 1;
    }
  }
  assert!(checked >= 5, "expected resolved hook entries, found {checked}");
}

/// Spec §3 and §12.2: the point vocabulary is one vocabulary. Every point an
/// example uses — in a configuration, a context or a report — is one the
/// schemas know about; an example that invented one would document a point no
/// engine implements.
#[test]
fn examples_use_only_the_specified_points() {
  let schemas = load_schemas();
  let known: BTreeSet<String> = schemas[&format!("{HOOKS}/hook-context.schema.json")]["properties"]["point"]
    ["x-known-values"]
    .as_array()
    .expect("point vocabulary")
    .iter()
    .map(|v| v.as_str().expect("point").to_string())
    .collect();
  let mut checked = 0;
  let mut used: BTreeSet<String> = BTreeSet::new();
  for (context, marker, value) in blocks() {
    let mut points: Vec<String> = Vec::new();
    match marker.as_str() {
      "hook-config" | "project-config" => {
        points.extend(value["hooks"].as_object().expect("hooks").keys().cloned())
      }
      "hook-context" | "hook-invocation" => points.push(value["point"].as_str().expect("point").into()),
      "hook-report" => points.extend(
        value["invocations"]
          .as_array()
          .expect("invocations")
          .iter()
          .map(|i| i["point"].as_str().expect("point").to_string()),
      ),
      _ => {}
    }
    for point in points {
      assert!(
        known.contains(&point),
        "{context}: uses point '{point}', which the schemas do not define"
      );
      used.insert(point);
      checked += 1;
    }
  }
  assert!(checked >= 15, "expected points to check, found {checked}");
  assert!(
    used.len() >= 6,
    "the examples exercise only {} of the eight points: {used:?}",
    used.len()
  );
}

/// Spec §9.3: `hook-api.d.ts` is the script's view of the context document, so
/// its `HookContext` members are the context schema's members — no more (a
/// member scripts would look for and never receive) and no fewer (a member the
/// engine sends and the types deny).
#[test]
fn the_script_api_declares_exactly_the_context_members() {
  let schemas = load_schemas();
  let schema_members: BTreeSet<String> = schemas[&format!("{HOOKS}/hook-context.schema.json")]["properties"]
    .as_object()
    .expect("context properties")
    .keys()
    .cloned()
    .collect();

  let ts = std::fs::read_to_string(specs_dir().join("lifecycle-hooks/hook-api.d.ts")).expect("read d.ts");
  let body = ts
    .split_once("export interface HookContext {")
    .expect("HookContext interface")
    .1
    .split_once('}')
    .expect("interface body")
    .0;
  let declared: BTreeSet<String> = body
    .lines()
    .map(str::trim)
    .filter(|l| !l.is_empty() && !l.starts_with("/*") && !l.starts_with("*") && !l.starts_with("//"))
    .filter_map(|l| l.split(&[':', '?'][..]).next())
    .map(|name| name.trim().trim_matches('"').to_string())
    .filter(|name| !name.is_empty())
    .collect();

  assert_eq!(
    declared, schema_members,
    "hook-api.d.ts and hook-context.schema.json disagree about what a hook is handed"
  );
}
