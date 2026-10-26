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
  #[serde(default)]
  pub capabilities: Value,
}

impl Hello {
  /// Whether the host asked for display text's segments (spec §2.7, ADR 0054): it declared the
  /// `segments` capability.
  pub fn wants_segments(&self) -> bool {
    self.capabilities.get("segments").is_some_and(Value::is_object)
  }
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

/// Result body of a successful `engine/hello` (spec §5.1).
///
/// `encoding` and `push-events` stay undeclared: absence means `json` encoding and no push
/// delivery, both conformant (spec's own worked example, `examples/error-and-negotiation.md`,
/// "Capability negotiation that cannot fail").
///
/// `provider-shape-recording` is declared, and is what spec §7.3 means by "new session kinds
/// arrive as new operations plus capabilities": a host that wants to record a provider shape has
/// to know before it builds a run whether this engine can, and §5.3's rule is that it MUST NOT
/// rely on operations behind a capability the engine did not declare. `subsumption-check` is the
/// other half of that loop (spec §8.6) and is declared for the same reason: a CLI or a broker
/// decides whether to *offer* a compatibility check before it has any documents to check.
///
/// `components` is always declared (component-interfaces spec §10.1, ADR 0013): a host learns which
/// loaders this embedding has from the handshake, not from a failure at interaction 40. An engine
/// with no loader registered still says `["in-tree"]`.
///
/// `broker` is declared only by an engine with a broker client registered (spec §8.7, ADR 0032):
/// the native embeddings, never the offline WASM artifact.
///
/// `segments` is always declared (spec §2.7, ADR 0054): this engine writes display text's segments
/// for a host that declared the capability too.
pub fn result(loaders: &[&str], broker: bool) -> Value {
  let mut result = json!({
    "protocol-version": crate::PROTOCOL_VERSION,
    "engine": { "name": "janus-engine", "version": crate::ENGINE_VERSION },
    "capabilities": {
      "provider-shape-recording": {},
      "subsumption-check": {},
      "components": { "loaders": loaders },
      "segments": {},
    }
  });
  if broker {
    result["capabilities"]["broker"] = json!({});
  }
  result
}
