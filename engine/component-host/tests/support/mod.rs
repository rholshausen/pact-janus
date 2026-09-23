//! What more than one of this crate's test binaries needs.

#![allow(dead_code)]

pub mod registry;

use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

/// Build `third-party/janus-csv` for `wasm32-wasip2` and return the component's path — once per
/// test binary, so no test runs against a stale `.wasm`: the same rule the TypeScript SDK's tests
/// follow for `janus-engine`.
pub fn csv_wasm() -> &'static PathBuf {
  static WASM: OnceLock<PathBuf> = OnceLock::new();
  WASM.get_or_init(|| {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../third-party/janus-csv");
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
      "building third-party/janus-csv for wasm32-wasip2 failed"
    );
    let wasm = crate_dir.join("target/wasm32-wasip2/release/janus_csv.wasm");
    wasm.canonicalize().expect("the component was built")
  })
}
