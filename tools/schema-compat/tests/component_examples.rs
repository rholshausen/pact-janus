//! Every ```json block in the component-interfaces specification and its worked
//! examples carries a marker naming what it is (`component-hello`, `decode`,
//! `matcher-apply-result`, …); this test validates each against the schema that
//! owns it. An unmarked block fails rather than silently skipping.
//!
//! Four checks do a job a schema cannot, and all four are the same job: this
//! design hands documents to other designs and must not describe them in terms
//! their owners would not recognise. A contributed plan fragment is design
//! 2.4's node, a component's mismatch is design 2.4's result, a contributed
//! dimension is design 2.2's dimension — each is validated against the owning
//! schema as well. The fifth check is design 2.6's own: every contributed name
//! is namespaced with the component's own name (spec §2.4), which is the rule
//! the engine enforces at load and the one a component author is most likely to
//! want to break.

use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const MARKERS: [&str; 26] = [
  "component-hello",
  "component-hello-result",
  "component-config",
  "component-error",
  "conformance-case",
  "capability",
  "engine-error",
  "parts",
  "start",
  "start-result",
  "stop",
  "send",
  "send-result",
  "poll-inbound",
  "poll-inbound-result",
  "reply",
  "dispose",
  "decode",
  "decode-result",
  "compile-result",
  "matcher-compile",
  "matcher-apply",
  "matcher-apply-result",
  "matcher-compare",
  "matcher-compare-result",
  "variant-space-result",
];

const COMPONENT: &str = "https://pact.io/janus/component/v1";
const PROTOCOL: &str = "https://pact.io/janus/protocol/v1";
const PLAN: &str = "https://pact.io/janus/plan/v0";
const SHAPE: &str = "https://pact.io/janus/shape/v1";

/// Marker -> the schema (or `$defs` fragment) that owns that document.
fn schema_for(marker: &str) -> Option<String> {
  let id = match marker {
    "component-hello" => format!("{COMPONENT}/handshake.schema.json#/$defs/ComponentHello"),
    "component-hello-result" => format!("{COMPONENT}/handshake.schema.json#/$defs/ComponentHelloResult"),
    "component-config" => format!("{COMPONENT}/component-config.schema.json"),
    "component-error" => format!("{COMPONENT}/component-error.schema.json"),
    "conformance-case" => format!("{COMPONENT}/conformance-case.schema.json"),
    "parts" => format!("{COMPONENT}/parts.schema.json"),
    "start" => format!("{COMPONENT}/transport.schema.json#/$defs/Start"),
    "start-result" => format!("{COMPONENT}/transport.schema.json#/$defs/StartResult"),
    "stop" => format!("{COMPONENT}/transport.schema.json#/$defs/Stop"),
    "send" => format!("{COMPONENT}/transport.schema.json#/$defs/Send"),
    "send-result" => format!("{COMPONENT}/transport.schema.json#/$defs/SendResult"),
    "poll-inbound" => format!("{COMPONENT}/transport.schema.json#/$defs/PollInbound"),
    "poll-inbound-result" => format!("{COMPONENT}/transport.schema.json#/$defs/PollInboundResult"),
    "reply" => format!("{COMPONENT}/transport.schema.json#/$defs/Reply"),
    "dispose" => format!("{COMPONENT}/transport.schema.json#/$defs/Dispose"),
    "decode" => format!("{COMPONENT}/content.schema.json#/$defs/Decode"),
    "decode-result" => format!("{COMPONENT}/content.schema.json#/$defs/DecodeResult"),
    "compile-result" => format!("{COMPONENT}/content.schema.json#/$defs/CompileResult"),
    "matcher-compile" => format!("{COMPONENT}/matcher.schema.json#/$defs/Compile"),
    "matcher-apply" => format!("{COMPONENT}/matcher.schema.json#/$defs/Apply"),
    "matcher-apply-result" => format!("{COMPONENT}/matcher.schema.json#/$defs/ApplyResult"),
    "matcher-compare" => format!("{COMPONENT}/matcher.schema.json#/$defs/Compare"),
    "matcher-compare-result" => format!("{COMPONENT}/matcher.schema.json#/$defs/CompareResult"),
    "variant-space-result" => format!("{COMPONENT}/matcher.schema.json#/$defs/VariantSpaceResult"),
    // Documents this design carries but does not own.
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
    ("component-interfaces", "v1"),
    ("engine-protocol", "v1"),
    ("plan-grammar", "v0"),
    ("shape-language", "v1"),
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
  let dir = specs_dir().join("component-interfaces");
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

#[test]
fn component_examples_validate_against_the_schemas() {
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

  for required in ["component-hello-result", "decode-result", "conformance-case"] {
    assert!(
      counts.get(required).copied().unwrap_or_default() >= 1,
      "expected a '{required}' block, found {counts:?}"
    );
  }
}

/// Spec §2.4: a component's contributed operators, actions and generators are
/// namespaced by its own name. The engine checks this at load rather than at
/// first use, which is what makes the namespace a partition and not a
/// convention; a worked example that broke it would document the opposite.
#[test]
fn contributed_names_are_namespaced_with_the_components_own_name() {
  let mut checked = 0;
  for (context, marker, value) in blocks() {
    if marker != "component-hello-result" {
      continue;
    }
    let name = value["component"]["name"].as_str().expect("component name");
    let contributes = &value["contributes"];
    for member in ["operators", "actions", "generators"] {
      let Some(entries) = contributes.get(member).and_then(Value::as_array) else {
        continue;
      };
      for entry in entries {
        let contributed = entry["name"].as_str().expect("contribution name");
        let (namespace, _) = contributed
          .split_once(':')
          .unwrap_or_else(|| panic!("{context}: '{contributed}' is not namespaced"));
        assert_eq!(
          namespace, name,
          "{context}: component '{name}' contributes '{contributed}', which is not in its namespace"
        );
        checked += 1;
      }
    }
  }
  assert!(
    checked >= 4,
    "expected contributed names to check, found {checked}"
  );
}

/// Spec §6.3 and §12.3: a contributed plan fragment is a plan document in
/// design 2.4's grammar — not a look-alike. If the two designs ever disagree
/// about what a node is, this is where it surfaces.
#[test]
fn contributed_fragments_are_plan_nodes() {
  let schemas = load_schemas();
  let node = validator_for(&format!("{PLAN}/plan.schema.json#/$defs/Node"), &schemas);
  let mut checked = 0;
  for (context, marker, value) in blocks() {
    if marker != "compile-result" {
      continue;
    }
    if let Some(fragment) = value.get("fragment") {
      check(&node, fragment, &format!("{context} fragment"));
      checked += 1;
    }
  }
  assert!(checked >= 1, "expected a contributed fragment, found {checked}");
}

/// Spec §7.2: a component's mismatch is a mismatch in the plan grammar's own
/// result form, which is why `matcher.schema.json` deliberately does not
/// restate it. This is the check that keeps the deliberate omission honest.
#[test]
fn component_results_are_plan_results() {
  let schemas = load_schemas();
  let result = validator_for(&format!("{PLAN}/plan.schema.json#/$defs/Result"), &schemas);
  let mut checked = 0;
  for (context, marker, value) in blocks() {
    if marker != "matcher-apply-result" {
      continue;
    }
    for (i, entry) in value["results"].as_array().expect("results").iter().enumerate() {
      check(&result, entry, &format!("{context} result {i}"));
      checked += 1;
    }
  }
  assert!(checked >= 2, "expected component results, found {checked}");
}

/// Spec §7.4: a contributed dimension joins the same variant space every other
/// dimension does, so it is design 2.2's document — including the ordered
/// points that variant semantics §3.1 reads as the facet's extremes.
#[test]
fn contributed_dimensions_are_variant_space_dimensions() {
  let schemas = load_schemas();
  let dimension = validator_for(
    &format!("{SHAPE}/variant-space.schema.json#/$defs/Dimension"),
    &schemas,
  );
  let mut checked = 0;
  for (context, marker, value) in blocks() {
    if marker != "variant-space-result" {
      continue;
    }
    for (i, entry) in value["dimensions"]
      .as_array()
      .expect("dimensions")
      .iter()
      .enumerate()
    {
      check(&dimension, entry, &format!("{context} dimension {i}"));
      assert!(
        entry["points"].as_array().expect("points").len() >= 2,
        "{context}: a dimension with fewer than two points is not a dimension"
      );
      checked += 1;
    }
  }
  assert!(checked >= 1, "expected a contributed dimension, found {checked}");
}

/// Spec §9.4: a conformance case may only call operations this specification
/// defines. An example that invented one would document an interface that does
/// not exist — the same failure mode `plan_examples.rs` guards for actions.
#[test]
fn conformance_cases_call_only_specified_operations() {
  let schemas = load_schemas();
  let specified: Vec<String> = schemas[&format!("{COMPONENT}/conformance-case.schema.json")]["$defs"]["Call"]
    ["properties"]["op"]["x-known-values"]
    .as_array()
    .expect("op vocabulary")
    .iter()
    .map(|v| v.as_str().expect("op name").to_string())
    .collect();
  let mut checked = 0;
  for (context, marker, value) in blocks() {
    if marker != "conformance-case" {
      continue;
    }
    for call in value["calls"].as_array().expect("calls") {
      let op = call["op"].as_str().expect("op").to_string();
      assert!(
        specified.contains(&op),
        "{context}: calls '{op}', which this specification does not define"
      );
      checked += 1;
    }
  }
  assert!(checked >= 3, "expected conformance calls, found {checked}");
}
