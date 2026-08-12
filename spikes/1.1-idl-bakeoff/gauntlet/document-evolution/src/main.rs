//! Document-schema gauntlet leg: E1–E5 as JSON documents governed by versioned schemas
//! with authored open-world rules. The "old plugin" is a v1-schema validator plus v1 serde
//! structs (typed view with pass-through); the "new engine" writes v2 documents.
//!
//! What this leg must demonstrate (not assert):
//!   1. E1–E5 pass by construction under the open rules;
//!   2. every unknown arrives *classifiable, with its name* (vs prost's silent None);
//!   3. pass-through preserves unknown content (where prost lost it);
//!   4. the failure form is a per-field authoring choice (strict counterexample rejects
//!      loudly with JSON paths).
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

/// v1 typed view of EngineError: open enum as string + pass-through for unknown members.
#[derive(Serialize, Deserialize, Debug)]
struct EngineErrorV1 {
  code: String,
  message: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  details: Option<Value>,
  #[serde(flatten)]
  extra: Map<String, Value>,
}

const V1_KNOWN_CODES: &[&str] =
  &["invalid-spec", "unsupported-spec-version", "session-not-found", "internal"];
const V1_KNOWN_EVENTS: &[&str] = &["started", "interaction-result", "finished"];
const V1_KNOWN_OPS: &[&str] = &["add-interaction", "finalise"];
const V1_KNOWN_PART_KINDS: &[&str] = &["request", "response"];

fn validator(path: &str) -> Result<jsonschema::Validator> {
  let schema: Value = serde_json::from_str(&std::fs::read_to_string(path)?)?;
  Ok(jsonschema::validator_for(&schema)?)
}

fn main() -> Result<()> {
  let v1_error = validator("schema/v1/engine-error.schema.json")?;
  let v1_event = validator("schema/v1/verify-event.schema.json")?;
  let v1_request = validator("schema/v1/request.schema.json")?;
  let v1_strict_error = validator("schema/v1-strict/engine-error.schema.json")?;
  let v2_error = validator("schema/v2/engine-error.schema.json")?;

  // The new engine's frame: E1 (new code value) + E2 (new field) together.
  let new_error = json!({
    "code": "component-unavailable",
    "message": "component x unavailable",
    "retryable": true
  });

  // ── E1: new enum value, D-a ──────────────────────────────────────────────────────
  println!("E1 D-a: v1 schema validates new frame: {}", v1_error.is_valid(&new_error));
  let typed: EngineErrorV1 = serde_json::from_value(new_error.clone())?;
  let known = V1_KNOWN_CODES.contains(&typed.code.as_str());
  println!(
    "E1 D-a: typed view sees code = {:?} verbatim; known-values check -> known: {known} \
     — unknown value arrives NAMED and classifiable (policy: ignore/warn/reject)",
    typed.code
  );

  // ── E2: new field, D-a + pass-through round-trip ─────────────────────────────────
  println!(
    "E2 D-a: unknown members captured in pass-through: {:?}",
    typed.extra.keys().collect::<Vec<_>>()
  );
  let round_tripped = serde_json::to_value(&typed)?;
  println!(
    "E2 pass-through: v2 view of old-plugin round-trip still has retryable = {} — \
     unknown content PRESERVED (prost dropped it); still valid under v2: {}",
    round_tripped["retryable"], v2_error.is_valid(&round_tripped)
  );

  // ── E4: new event kind, D-a ──────────────────────────────────────────────────────
  let new_event = json!({ "type": "hook-invoked", "payload": { "hook": "before-request" } });
  let known = V1_KNOWN_EVENTS.contains(&new_event["type"].as_str().unwrap());
  println!(
    "E4 D-a: v1 envelope validates: {}; event kind {:?} known: {known} — old consumer can \
     log 'skipping unknown event hook-invoked', not a bare None",
    v1_event.is_valid(&new_event), new_event["type"]
  );

  // ── E5: new union alternative — same open-discriminator mechanism as E4 ──────────
  let new_part = json!({ "kind": "message", "content": {} });
  let known = V1_KNOWN_PART_KINDS.contains(&new_part["kind"].as_str().unwrap());
  println!(
    "E5 D-a: part kind {:?} known: {known} — identical mechanics to E4 by construction",
    new_part["kind"]
  );

  // ── E3: new operation, D-a ───────────────────────────────────────────────────────
  let new_request = json!({ "op": "explain", "body": { "session": 1, "spec": {} } });
  let known = V1_KNOWN_OPS.contains(&new_request["op"].as_str().unwrap());
  println!(
    "E3 D-a: v1 envelope validates: {}; op {:?} known: {known} — old callee answers \
     {{\"error\": \"unsupported operation 'explain'\"}} with the op NAMED",
    v1_request.is_valid(&new_request), new_request["op"]
  );

  // ── D-b: old frames under the new view ───────────────────────────────────────────
  let old_error = json!({ "code": "internal", "message": "boom" });
  println!(
    "D-b: old frame valid under v2 schema: {}; retryable absent -> caller default",
    v2_error.is_valid(&old_error)
  );

  // ── Counterexample: the same evolution against a DELIBERATELY closed v1 schema ───
  println!("strict counterexample: v1-strict validates new frame: {}", v1_strict_error.is_valid(&new_error));
  for err in v1_strict_error.iter_errors(&new_error) {
    println!("  strict rejection at '{}': {err}", err.instance_path);
  }
  println!("  -> failure form is AUTHORED per field: open (classifiable) vs closed (loud reject)");

  println!("RESULT: OK");
  Ok(())
}
