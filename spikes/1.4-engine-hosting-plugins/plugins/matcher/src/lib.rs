//! Well-behaved plugin: contributes a Luhn-checksum matcher (an action the
//! core engine does not have) plus an echo op for marshalling benchmarks.

#[allow(warnings)]
mod bindings;

use bindings::exports::pact::plugin::pipe::Guest;
use serde_json::{json, Value};

struct Plugin;

impl Guest for Plugin {
    fn call(request: Vec<u8>) -> Vec<u8> {
        let response = match serde_json::from_slice::<Value>(&request) {
            Ok(frame) => dispatch(&frame),
            Err(e) => json!({ "err": { "code": "malformed-frame", "detail": e.to_string() } }),
        };
        serde_json::to_vec(&response).expect("serializable")
    }
}

fn dispatch(frame: &Value) -> Value {
    match frame.get("op").and_then(Value::as_str) {
        Some("handshake") => json!({ "ok": {
            "protocol-version": 1,
            "name": "toy-luhn-matcher",
            "actions": ["match:luhn"],
        }}),
        Some("echo") => json!({ "ok": { "payload": frame.get("payload").cloned().unwrap_or(Value::Null) } }),
        Some("invoke") => invoke(frame),
        Some(other) => json!({ "err": { "code": "unsupported-operation", "op": other } }),
        None => json!({ "err": { "code": "malformed-frame", "detail": "missing 'op'" } }),
    }
}

fn invoke(frame: &Value) -> Value {
    let action = frame.get("action").and_then(Value::as_str).unwrap_or("");
    match action {
        "match:luhn" => {
            let actual = frame.get("actual").and_then(Value::as_str).unwrap_or("");
            let mismatches = if luhn_valid(actual) {
                vec![]
            } else {
                vec![json!({ "path": "$", "expected": "a Luhn-valid digit string", "actual": actual })]
            };
            json!({ "ok": { "matched": mismatches.is_empty(), "mismatches": mismatches } })
        }
        other => json!({ "err": { "code": "unknown-action", "action": other } }),
    }
}

fn luhn_valid(s: &str) -> bool {
    if s.is_empty() || !s.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }
    let sum: u32 = s
        .chars()
        .rev()
        .filter_map(|c| c.to_digit(10))
        .enumerate()
        .map(|(i, d)| if i % 2 == 1 { if d * 2 > 9 { d * 2 - 9 } else { d * 2 } } else { d })
        .sum();
    sum % 10 == 0
}

bindings::export!(Plugin with_types_in bindings);
