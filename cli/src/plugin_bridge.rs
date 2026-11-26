//! Where `janus-plugin-bridge` is, for `janus verify` and `janus explain` to tell the engine (ADR 0110
//! decision 6): the engine declares one for each pact plugin a pact names, as `options.plugin-bridge`
//! says to start it. Without one, an interaction that needs a plugin is refused, saying where the bridge
//! was looked for.

use serde_json::{Value, json};

/// The variable that names janus-plugin-bridge, for a bridge installed anywhere but beside `janus`.
pub const PLUGIN_BRIDGE_VARIABLE: &str = "JANUS_PLUGIN_BRIDGE";

/// How the engine starts janus-plugin-bridge (ADR 0110 decision 6): the program `JANUS_PLUGIN_BRIDGE`
/// names, else the one beside this executable, granted the variables pact-plugin-driver reads and a
/// deadline a plugin can start within. `None` when there is neither.
pub fn option() -> Option<Value> {
  let program = std::env::var_os(PLUGIN_BRIDGE_VARIABLE)
    .filter(|named| !named.is_empty())
    .map(std::path::PathBuf::from)
    // A declaration with no configuration file is read relative to the engine (ADR 0108), so the
    // program is named absolutely.
    .map(|named| std::path::absolute(&named).unwrap_or(named))
    .or_else(|| {
      let beside = std::env::current_exe()
        .ok()?
        .with_file_name(format!("janus-plugin-bridge{}", std::env::consts::EXE_SUFFIX));
      beside.is_file().then_some(beside)
    })?;
  Some(json!({
    "source": { "kind": "subprocess", "reference": program.display().to_string() },
    "grants": { "env": ["PACT_PLUGIN_DIR", "HOME", "USERPROFILE", "PACT_DO_NOT_TRACK", "RUST_LOG"] },
    "limits": { "deadline-ms": 30_000 },
  }))
}
