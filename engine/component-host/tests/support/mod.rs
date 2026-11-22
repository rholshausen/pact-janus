//! What more than one of this crate's test binaries needs.

#![allow(dead_code)]

pub mod registry;
pub mod sigstore;

use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

/// Build `third-party/janus-csv` for `wasm32-wasip2` and return the component's path — once per
/// test binary, so no test runs against a stale `.wasm`: the same rule the TypeScript SDK's tests
/// follow for `janus-engine`.
pub fn csv_wasm() -> &'static PathBuf {
  static WASM: OnceLock<PathBuf> = OnceLock::new();
  WASM.get_or_init(|| third_party_wasm("janus-csv", "janus_csv.wasm"))
}

/// `third-party/janus-ndjson`, the component written from the component author guide alone (S5.1d,
/// ADR 0104), built as [`csv_wasm`] is.
pub fn ndjson_wasm() -> &'static PathBuf {
  static WASM: OnceLock<PathBuf> = OnceLock::new();
  WASM.get_or_init(|| third_party_wasm("janus-ndjson", "janus_ndjson.wasm"))
}

/// Build `third-party/<dir>` for `wasm32-wasip2` and return the path of `<file>` it builds.
fn third_party_wasm(dir: &str, file: &str) -> PathBuf {
  let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    .join("../../third-party")
    .join(dir);
  let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
  let status = Command::new(cargo)
    .args(["build", "--release", "--target", "wasm32-wasip2"])
    .current_dir(&crate_dir)
    // The workspace's own build settings must not leak into an out-of-tree build.
    .env_remove("CARGO_TARGET_DIR")
    .env_remove("RUSTFLAGS")
    .status()
    .expect("cargo runs");
  assert!(
    status.success(),
    "building third-party/{dir} for wasm32-wasip2 failed"
  );
  let wasm = crate_dir.join("target/wasm32-wasip2/release").join(file);
  wasm.canonicalize().expect("the component was built")
}
