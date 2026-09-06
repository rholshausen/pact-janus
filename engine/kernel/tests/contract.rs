//! Plan task 3.1: the Janus contract model — canonical writing (contract-file spec §2.4, ADR 0018),
//! identification (contract-file spec §2.3), and a round trip validated against the shipped schema.

use pact_janus_kernel::contract::IdentifyMode;
use pact_janus_kernel::contract::{
  Contract, FORMAT, Interaction, Party, RecordedSelection, RecordedVariant, SlotValue, Transport, ValuePart,
  identify_strict, identify_tolerant, read as read_contract, write_canonical,
};
use serde_json::json;
use std::collections::BTreeMap;
use std::path::PathBuf;

fn sample_contract() -> Contract {
  let mut request_shapes = BTreeMap::new();
  request_shapes.insert(
    "method".to_string(),
    json!({ "shape": "equality", "example": "GET" }),
  );
  request_shapes.insert(
    "path".to_string(),
    json!({ "shape": "equality", "example": "/orders/42" }),
  );

  let mut response_shapes = BTreeMap::new();
  response_shapes.insert(
    "status".to_string(),
    json!({ "shape": "equality", "example": 200 }),
  );

  let mut parts = BTreeMap::new();
  parts.insert("request".to_string(), request_shapes);
  parts.insert("response".to_string(), response_shapes);

  let mut request_values: ValuePart = BTreeMap::new();
  request_values.insert(
    "method".to_string(),
    SlotValue {
      content: json!("GET"),
      encoded: None,
      content_type: None,
    },
  );
  request_values.insert(
    "path".to_string(),
    SlotValue {
      content: json!("/orders/42"),
      encoded: None,
      content_type: None,
    },
  );

  let mut response_values: ValuePart = BTreeMap::new();
  response_values.insert(
    "status".to_string(),
    SlotValue {
      content: json!(200),
      encoded: None,
      content_type: None,
    },
  );

  let mut variant_parts = BTreeMap::new();
  variant_parts.insert("request".to_string(), request_values);
  variant_parts.insert("response".to_string(), response_values);

  Contract {
    format: FORMAT.to_string(),
    schema: None,
    consumer: Party {
      name: "orders-ui".to_string(),
    },
    provider: Party {
      name: "orders-api".to_string(),
    },
    interactions: vec![Interaction {
      description: "get an order".to_string(),
      transport: Some(Transport {
        kind: "http".to_string(),
        mode: None,
      }),
      states: None,
      parts,
      requires: None,
      selection: RecordedSelection {
        variants: vec![RecordedVariant {
          id: "base".to_string(),
          assignment: vec![],
          states: None,
          parts: variant_parts,
        }],
        report: BTreeMap::new(),
      },
    }],
    metadata: None,
  }
}

#[test]
fn a_canonically_written_contract_begins_with_format() {
  let bytes = write_canonical(&sample_contract()).expect("writes");
  assert!(
    bytes.starts_with(b"{\"$format\":"),
    "expected $format first, got: {}",
    String::from_utf8_lossy(&bytes[..40.min(bytes.len())])
  );
}

#[test]
fn writing_the_same_contract_twice_produces_identical_bytes() {
  let contract = sample_contract();
  let first = write_canonical(&contract).expect("writes");
  let second = write_canonical(&contract).expect("writes");
  assert_eq!(
    first, second,
    "contract-file spec §2.4: the same content written twice MUST produce the same bytes"
  );
}

#[test]
fn a_contract_round_trips_through_canonical_bytes() {
  let contract = sample_contract();
  let bytes = write_canonical(&contract).expect("writes");
  let read_back = read_contract(&bytes, IdentifyMode::Strict).expect("reads its own output");
  assert_eq!(contract, read_back);

  let rewritten = write_canonical(&read_back).expect("writes");
  assert_eq!(
    bytes, rewritten,
    "packing what was just unpacked reproduces the canonical bytes"
  );
}

#[test]
fn canonical_bytes_end_with_exactly_one_trailing_newline() {
  let bytes = write_canonical(&sample_contract()).expect("writes");
  assert!(bytes.ends_with(b"\n"));
  assert!(!bytes.ends_with(b"\n\n"));
}

#[test]
fn canonical_bytes_have_no_insignificant_whitespace() {
  // ADR 0018: compact, not pretty-printed. The sample contract has spaces *inside* a string value
  // ("get an order"), so this checks for the separator patterns a pretty-printer inserts —
  // ": " and ", " between syntax elements — rather than for the byte 0x20 anywhere at all.
  let bytes = write_canonical(&sample_contract()).expect("writes");
  let body = &bytes[..bytes.len() - 1]; // strip the one permitted trailing newline
  let text = std::str::from_utf8(body).expect("valid UTF-8");
  assert!(!text.contains('\n'), "no newlines before the trailing one");

  let mut in_string = false;
  let mut escaped = false;
  let bytes_iter: Vec<char> = text.chars().collect();
  for i in 0..bytes_iter.len() {
    let c = bytes_iter[i];
    if in_string {
      if escaped {
        escaped = false;
      } else if c == '\\' {
        escaped = true;
      } else if c == '"' {
        in_string = false;
      }
      continue;
    }
    match c {
      '"' => in_string = true,
      ':' | ',' => {
        assert_ne!(
          bytes_iter.get(i + 1),
          Some(&' '),
          "no space after ':' or ',' outside a string"
        );
      }
      _ => {}
    }
  }
}

#[test]
fn reading_does_not_assume_canonical_formatting() {
  // A contract that reached us pretty-printed — round-tripped through a formatter, a broker, or a
  // hand edit — is exactly as valid as this writer's own compact output (ADR 0018).
  let contract = sample_contract();
  let compact = write_canonical(&contract).expect("writes");
  let value: serde_json::Value = serde_json::from_slice(&compact).expect("valid JSON");
  let reformatted = serde_json::to_vec_pretty(&value).expect("re-serializes");

  assert!(
    !identify_strict(&reformatted),
    "pretty-printing moves $format off the strict prefix"
  );
  assert!(identify_tolerant(&reformatted));

  let read_back =
    read_contract(&reformatted, IdentifyMode::Tolerant).expect("reads a differently-formatted contract");
  assert_eq!(contract, read_back);
}

#[test]
fn strict_identification_accepts_canonical_bytes() {
  let bytes = write_canonical(&sample_contract()).expect("writes");
  assert!(identify_strict(&bytes));
  assert!(identify_tolerant(&bytes));
}

#[test]
fn strict_identification_rejects_format_that_is_present_but_not_first() {
  let reordered =
    br#"{"consumer":{"name":"c"},"provider":{"name":"p"},"$format":"janus-contract/1","interactions":[]}"#;
  assert!(!identify_strict(reordered));
  assert!(
    identify_tolerant(reordered),
    "tolerant mode scans for $format anywhere in the window"
  );
}

#[test]
fn both_modes_reject_a_document_with_no_format_member() {
  let legacy_shaped = br#"{"consumer":{"name":"c"},"provider":{"name":"p"},"interactions":[]}"#;
  assert!(!identify_strict(legacy_shaped));
  assert!(!identify_tolerant(legacy_shaped));
}

#[test]
fn tolerant_identification_has_an_8kb_window() {
  const WINDOW: usize = 8 * 1024;
  let token = b"\"$format\"";
  // A buffer well past the window, with the token placed so its last byte lands exactly on the
  // boundary (found) or one byte past it (not found) — `identify_tolerant` only ever looks at
  // the first WINDOW bytes, so a token straddling the edge must not match.
  let build = |start: usize| -> Vec<u8> {
    let mut buf = vec![b' '; WINDOW + 100];
    buf[0] = b'{';
    buf[start..start + token.len()].copy_from_slice(token);
    buf
  };

  let just_inside = build(WINDOW - token.len());
  assert!(identify_tolerant(&just_inside));

  let just_outside = build(WINDOW - token.len() + 1);
  assert!(!identify_tolerant(&just_outside));
}

#[test]
fn reading_rejects_a_document_that_never_identifies_as_a_contract() {
  let legacy_shaped = br#"{"consumer":{"name":"c"},"provider":{"name":"p"},"interactions":[]}"#;
  let err = read_contract(legacy_shaped, IdentifyMode::Tolerant).expect_err("no $format member");
  assert_eq!(err.code(), "contract-invalid");
}

#[test]
fn a_round_tripped_contract_validates_against_the_shipped_schema() {
  let bytes = write_canonical(&sample_contract()).expect("writes");
  let instance: serde_json::Value = serde_json::from_slice(&bytes).expect("valid JSON");

  let schema_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    .join("../../Documentation/specs/contract-file/schemas/v1/contract.schema.json");
  let schema_bytes =
    std::fs::read(&schema_path).unwrap_or_else(|err| panic!("reading {schema_path:?}: {err}"));
  let schema: serde_json::Value = serde_json::from_slice(&schema_bytes).expect("valid schema JSON");

  jsonschema::validate(&schema, &instance)
    .unwrap_or_else(|err| panic!("round-tripped contract does not validate: {err}"));
}
