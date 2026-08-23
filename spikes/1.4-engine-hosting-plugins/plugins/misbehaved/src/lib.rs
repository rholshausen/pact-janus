//! Hostile-ish plugin: misbehaves on demand so the host's containment story
//! can be tested. Built with std the way a careless third party would.

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
            "name": "misbehaved",
            "actions": [],
        }}),
        // Sandbox probe: try to read a host file.
        Some("read-file") => {
            let path = frame.get("path").and_then(Value::as_str).unwrap_or("/etc/passwd");
            match std::fs::read_to_string(path) {
                Ok(content) => json!({ "ok": { "leaked": true, "bytes": content.len() } }),
                Err(e) => json!({ "err": { "code": "io-denied", "detail": e.to_string() } }),
            }
        }
        // Sandbox probe: what does the environment look like from in here?
        Some("get-env") => {
            let vars: Vec<(String, String)> = std::env::vars().collect();
            json!({ "ok": { "var-count": vars.len(), "vars": vars } })
        }
        // Fault containment probe: a plain Rust panic.
        Some("panic") => panic!("plugin panicked on request"),
        // Runaway probe: never returns (black_box keeps LLVM from folding
        // the loop into a closed form).
        Some("spin") => {
            let mut x: u64 = 1;
            while std::hint::black_box(x) != 0 {
                x = std::hint::black_box(x).wrapping_add(1);
            }
            json!({ "ok": { "x": x } })
        }
        Some(other) => json!({ "err": { "code": "unsupported-operation", "op": other } }),
        None => json!({ "err": { "code": "malformed-frame", "detail": "missing 'op'" } }),
    }
}

bindings::export!(Plugin with_types_in bindings);
