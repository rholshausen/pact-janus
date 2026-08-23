//! Toy engine logic for spike 1.2: JSON frames in, JSON frames out.
//!
//! The frame vocabulary follows the 1.1 draft G1 recommendation: an open `op`
//! vocabulary, must-ignore unknown members, errors as values (`ok`/`err`
//! envelope), and a handshake that names capabilities.

use serde_json::{json, Map, Value};

pub const PROTOCOL_VERSION: u64 = 1;
pub const CAPABILITIES: &[&str] = &["echo", "match-type"];

/// Entry point shared by every embedding: one request frame in, one response
/// frame out, both JSON bytes. Never panics across the boundary.
pub fn handle_frame(request: &[u8]) -> Vec<u8> {
    let response = match serde_json::from_slice::<Value>(request) {
        Ok(frame) => dispatch(&frame),
        Err(e) => err_frame("malformed-frame", json!({ "detail": e.to_string() })),
    };
    serde_json::to_vec(&response).expect("response frames are always serializable")
}

fn dispatch(frame: &Value) -> Value {
    let op = frame.get("op").and_then(Value::as_str);
    match op {
        Some("handshake") => handshake(frame),
        Some("echo") => ok_frame(json!({ "payload": frame.get("payload").cloned().unwrap_or(Value::Null) })),
        Some("match-type") => match_type_op(frame),
        Some(other) => err_frame("unsupported-operation", json!({ "op": other })),
        None => err_frame("malformed-frame", json!({ "detail": "missing 'op' member" })),
    }
}

fn handshake(frame: &Value) -> Value {
    let requested = frame
        .get("protocol-versions")
        .and_then(Value::as_array)
        .map(|vs| vs.iter().filter_map(Value::as_u64).collect::<Vec<_>>())
        .unwrap_or_default();
    if requested.contains(&PROTOCOL_VERSION) {
        ok_frame(json!({
            "protocol-version": PROTOCOL_VERSION,
            "capabilities": CAPABILITIES,
        }))
    } else {
        err_frame("protocol-version-unsupported", json!({ "supported": [PROTOCOL_VERSION] }))
    }
}

fn match_type_op(frame: &Value) -> Value {
    let (Some(expected), Some(actual)) = (frame.get("expected"), frame.get("actual")) else {
        return err_frame(
            "malformed-frame",
            json!({ "detail": "match-type requires 'expected' and 'actual' members" }),
        );
    };
    let mut mismatches = Vec::new();
    match_type(expected, actual, "$", &mut mismatches);
    ok_frame(json!({ "matched": mismatches.is_empty(), "mismatches": mismatches }))
}

/// Structural type match in the spirit of Pact's `type` matcher: values must
/// agree on JSON type recursively; objects are matched per expected key
/// (extra actual keys are ignored); each actual array element is matched
/// against the first expected element.
fn match_type(expected: &Value, actual: &Value, path: &str, mismatches: &mut Vec<Value>) {
    match (expected, actual) {
        (Value::Object(exp), Value::Object(act)) => {
            for (key, exp_child) in exp {
                let child_path = format!("{path}.{key}");
                match act.get(key) {
                    Some(act_child) => match_type(exp_child, act_child, &child_path, mismatches),
                    None => mismatches.push(mismatch(&child_path, "present", "missing")),
                }
            }
        }
        (Value::Array(exp), Value::Array(act)) => {
            if let Some(template) = exp.first() {
                for (i, act_child) in act.iter().enumerate() {
                    match_type(template, act_child, &format!("{path}[{i}]"), mismatches);
                }
            }
        }
        _ => {
            if type_name(expected) != type_name(actual) {
                mismatches.push(mismatch(path, type_name(expected), type_name(actual)));
            }
        }
    }
}

fn type_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn mismatch(path: &str, expected: &str, actual: &str) -> Value {
    json!({ "path": path, "expected": expected, "actual": actual })
}

fn ok_frame(body: Value) -> Value {
    Value::Object(Map::from_iter([("ok".to_string(), body)]))
}

fn err_frame(code: &str, mut detail: Value) -> Value {
    if let Value::Object(m) = &mut detail {
        m.insert("code".to_string(), json!(code));
    }
    Value::Object(Map::from_iter([("err".to_string(), detail)]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(frame: Value) -> Value {
        serde_json::from_slice(&handle_frame(&serde_json::to_vec(&frame).unwrap())).unwrap()
    }

    #[test]
    fn handshake_negotiates() {
        let resp = call(json!({ "op": "handshake", "protocol-versions": [1] }));
        assert_eq!(resp["ok"]["protocol-version"], 1);
        assert!(resp["ok"]["capabilities"].as_array().unwrap().len() >= 2);
    }

    #[test]
    fn echo_round_trips() {
        let resp = call(json!({ "op": "echo", "payload": { "a": [1, 2, 3] } }));
        assert_eq!(resp["ok"]["payload"], json!({ "a": [1, 2, 3] }));
    }

    #[test]
    fn match_type_finds_mismatches() {
        let resp = call(json!({
            "op": "match-type",
            "expected": { "id": 1, "tags": ["a"], "name": "x" },
            "actual": { "id": "oops", "tags": ["b", 3], "name": "y", "extra": true }
        }));
        assert_eq!(resp["ok"]["matched"], false);
        let mismatches = resp["ok"]["mismatches"].as_array().unwrap();
        assert_eq!(mismatches.len(), 2); // $.id and $.tags[1]
    }

    #[test]
    fn unknown_op_is_a_value_not_a_panic() {
        let resp = call(json!({ "op": "explain" }));
        assert_eq!(resp["err"]["code"], "unsupported-operation");
    }

    #[test]
    fn garbage_bytes_are_a_value_too() {
        let resp: Value = serde_json::from_slice(&handle_frame(b"not json")).unwrap();
        assert_eq!(resp["err"]["code"], "malformed-frame");
    }
}
