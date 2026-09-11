//! The `engine/hello` handshake (engine-protocol spec §5). The first request on every pipe;
//! everything else before a successful handshake is `handshake-required`.

use serde::Deserialize;
use serde_json::{Value, json};

/// Request body of `engine/hello`. `host`/`capabilities` are read loosely (`Value`): 4.1
/// declares no capabilities of its own (v1 defines `push-events` and `encoding`, neither
/// applicable to the native pipe this kernel dispatches for today), and host identification is
/// diagnostics-only by spec (§5.1) — semantics never depend on it.
#[derive(Debug, Deserialize)]
pub struct Hello {
  #[serde(rename = "protocol-versions")]
  pub protocol_versions: Vec<u32>,
}

/// Negotiate a protocol version: the first of the host's offered versions this engine also
/// speaks (today, only [`crate::PROTOCOL_VERSION`]). `None` means no match — spec §5.1 requires
/// `protocol-version-unsupported`, and the pipe stays usable so the host can report a good error.
pub fn negotiate(hello: &Hello) -> Option<u32> {
  hello
    .protocol_versions
    .iter()
    .copied()
    .find(|v| *v == crate::PROTOCOL_VERSION)
}

/// Result body of a successful `engine/hello` (spec §5.1). No capabilities declared: absence
/// means `json` encoding and no push delivery, both conformant (spec's own worked example,
/// `examples/error-and-negotiation.md`, "Capability negotiation that cannot fail").
pub fn result() -> Value {
  json!({
    "protocol-version": crate::PROTOCOL_VERSION,
    "engine": { "name": "janus-engine", "version": crate::ENGINE_VERSION },
    "capabilities": {}
  })
}
