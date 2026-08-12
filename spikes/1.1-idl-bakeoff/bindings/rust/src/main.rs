//! Bindings round, Rust leg: are typify-generated types (from the gauntlet's JSON Schemas)
//! good enough to give back compile-time safety — and do they preserve the document model's
//! evolution properties, especially pass-through?
mod engine_error_v1;
mod engine_error_v2;

use anyhow::Result;
use engine_error_v1::EngineErrorV1OpenWorld as V1Error;
use serde_json::json;

const V1_KNOWN_CODES: &[&str] =
  &["invalid-spec", "unsupported-spec-version", "session-not-found", "internal"];

fn main() -> Result<()> {
  let new_frame = json!({
    "code": "component-unavailable",
    "message": "component x unavailable",
    "retryable": true
  });

  let typed: V1Error = serde_json::from_value(new_frame)?;
  println!(
    "E1: generated type decodes new frame; code = {:?} verbatim; known: {}",
    typed.code,
    V1_KNOWN_CODES.contains(&typed.code.as_str())
  );

  let round_tripped = serde_json::to_value(&typed)?;
  println!(
    "E2 pass-through: retryable after round-trip through generated v1 type = {:?}",
    round_tripped.get("retryable")
  );
  println!(
    "  -> typify emits NO flatten/extra map: unknown members are {} by generated Rust types",
    if round_tripped.get("retryable").is_some() { "PRESERVED" } else { "DROPPED" }
  );

  println!("RESULT: OK");
  Ok(())
}
