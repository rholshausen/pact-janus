//! `config/load` (engine-protocol spec §8.8, ADR 0055): a host that verifies inside a test reads
//! the project's configuration file through the engine's host, so every SDK resolves it as
//! `janus verify --config` does.
//!
//! The loader is where the file system and the environment are (lifecycle-hooks spec §7.1), which a
//! kernel that builds for `wasm32-wasip2` does not have. So it arrives as a [`ConfigLoader`] the
//! embedding registers, as the broker client does (ADR 0032). With none, `config/load` is
//! `component-unavailable` naming `config-loader`, and `engine/hello` declares no `config-loading`.
//!
//! This is not a run: `verification/verify` still receives a closed document and reads nothing.

use crate::protocol::EngineError;
use serde_json::Value;

/// What the operation is called in `component-unavailable`'s `component`.
pub const LOADER: &str = "config-loader";

/// Loads a project configuration file into the resolved document: `version`, `hooks` and
/// `components`. An error is the loader's own: `hook-unresolved` or `hook-config-invalid`.
pub trait ConfigLoader: Send + Sync {
  /// `path` is absolute; the caller has checked.
  fn load(&self, path: &str) -> Result<Value, EngineError>;
}
