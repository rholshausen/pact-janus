//! Every ```json block in the spec's worked examples is one protocol frame;
//! this test validates each against the v1 schemas — the frame envelope, then
//! the operation's request/result schema — so the transcripts cannot rot.

use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const BASE: &str = "https://pact.io/janus/protocol/v1/";

fn protocol_dir() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Documentation/specs/engine-protocol")
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
  let dir = protocol_dir().join("schemas/v1");
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
  assert!(out.len() >= 7, "expected the v1 schema set, found {}", out.len());
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

fn json_blocks(markdown: &str) -> Vec<Value> {
  let mut blocks = Vec::new();
  let mut current: Option<String> = None;
  for line in markdown.lines() {
    match &mut current {
      None if line.trim() == "```json" => current = Some(String::new()),
      None => {}
      Some(buf) => {
        if line.trim() == "```" {
          blocks.push(serde_json::from_str(buf).expect("example block is valid JSON"));
          current = None;
        } else {
          buf.push_str(line);
          buf.push('\n');
        }
      }
    }
  }
  assert!(current.is_none(), "unterminated ```json block");
  blocks
}

#[test]
fn example_transcripts_validate_against_the_schemas() {
  let schemas = load_schemas();
  let frame = validator_for("frame.schema.json", &schemas);

  // op name -> (request body schema, result body schema); None = plain object
  // with no further constraints (engine/shutdown's empty documents).
  let ops: HashMap<&str, (Option<&str>, Option<&str>)> = HashMap::from([
    (
      "engine/hello",
      (
        Some("hello.schema.json"),
        Some("hello.schema.json#/$defs/HelloResult"),
      ),
    ),
    ("engine/shutdown", (None, None)),
    (
      "consumer-session/create",
      (
        Some("consumer-session.schema.json#/$defs/Create"),
        Some("consumer-session.schema.json#/$defs/CreateResult"),
      ),
    ),
    (
      "consumer-session/add-interaction",
      (
        Some("consumer-session.schema.json#/$defs/AddInteraction"),
        Some("consumer-session.schema.json#/$defs/AddInteractionResult"),
      ),
    ),
    (
      "consumer-session/variants",
      (
        Some("consumer-session.schema.json#/$defs/Variants"),
        Some("consumer-session.schema.json#/$defs/VariantsResult"),
      ),
    ),
    (
      "consumer-session/start-transport",
      (
        Some("consumer-session.schema.json#/$defs/StartTransport"),
        Some("consumer-session.schema.json#/$defs/StartTransportResult"),
      ),
    ),
    (
      "consumer-session/serve-variant",
      (
        Some("consumer-session.schema.json#/$defs/ServeVariant"),
        Some("consumer-session.schema.json#/$defs/ServeVariantResult"),
      ),
    ),
    (
      "consumer-session/finalise",
      (
        Some("consumer-session.schema.json#/$defs/Finalise"),
        Some("consumer-session.schema.json#/$defs/FinaliseResult"),
      ),
    ),
    (
      "verification/verify",
      (
        Some("verification.schema.json#/$defs/Verify"),
        Some("verification.schema.json#/$defs/VerifyResult"),
      ),
    ),
    (
      "verification/explain",
      (
        Some("verification.schema.json#/$defs/Explain"),
        Some("verification.schema.json#/$defs/ExplainResult"),
      ),
    ),
    (
      "upgrade/pact",
      (
        Some("upgrade.schema.json#/$defs/Pact"),
        Some("upgrade.schema.json#/$defs/PactResult"),
      ),
    ),
    (
      "events/poll",
      (
        Some("events.schema.json#/$defs/Poll"),
        Some("events.schema.json#/$defs/PollResult"),
      ),
    ),
  ]);
  let mut validators: HashMap<&str, jsonschema::Validator> = HashMap::new();
  for (req, ok) in ops.values() {
    for reference in [req, ok].into_iter().flatten() {
      validators
        .entry(reference)
        .or_insert_with(|| validator_for(reference, &schemas));
    }
  }

  let examples = protocol_dir().join("examples");
  let mut total_frames = 0;
  for entry in std::fs::read_dir(&examples).expect("examples dir") {
    let path = entry.expect("entry").path();
    if path.extension().is_none_or(|e| e != "md") {
      continue;
    }
    let name = path.file_name().unwrap().to_string_lossy().into_owned();
    // None = an op outside the table, allowed only for transcripts that show
    // it being rejected (the response must be an error, never 'ok').
    let mut op_by_id: HashMap<String, Option<&str>> = HashMap::new();

    for (i, block) in json_blocks(&std::fs::read_to_string(&path).expect("read"))
      .iter()
      .enumerate()
    {
      let context = format!("{name} block {}", i + 1);
      check(&frame, block, &context);
      total_frames += 1;

      match block["type"].as_str() {
        Some("request") => {
          let op = block["op"].as_str().expect("op");
          let known = ops.get_key_value(op).map(|(k, _)| *k);
          op_by_id.insert(block["id"].as_str().expect("id").into(), known);
          if let Some((Some(reference), _)) = known.map(|k| ops[k]) {
            check(
              &validators[reference],
              &block["body"],
              &format!("{context} ({op} body)"),
            );
          }
        }
        Some("response") => {
          if let Some(ok) = block.get("ok") {
            let id = block["id"].as_str().expect("id");
            let op = op_by_id
              .get(id)
              .unwrap_or_else(|| panic!("{context}: response to unknown request id '{id}'"))
              .unwrap_or_else(|| panic!("{context}: 'ok' response to an op outside the test's op table"));
            if let (_, Some(reference)) = ops[op] {
              check(&validators[reference], ok, &format!("{context} ({op} result)"));
            }
          }
          // error responses: the frame validator already checked the
          // EngineError via the envelope's $ref.
        }
        other => panic!("{context}: unexpected frame type {other:?} in a transcript"),
      }
    }
  }
  assert!(
    total_frames >= 20,
    "expected a real set of example frames, found {total_frames}"
  );
}
