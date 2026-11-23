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

/// `node` on this machine's `PATH`, for the subprocess binding's tests (ADR 0108): `None` where there is
/// none, so a contributor without Node skips them, unless `CI` is set, where every runner has Node and a
/// missing one is a broken runner.
pub fn node() -> Option<PathBuf> {
  let names: &[&str] = if cfg!(windows) {
    &["node.exe", "node.cmd"]
  } else {
    &["node"]
  };
  let found = std::env::var_os("PATH").and_then(|paths| {
    std::env::split_paths(&paths).find_map(|dir| {
      names
        .iter()
        .map(|name| dir.join(name))
        .find(|candidate| candidate.is_file())
    })
  });
  if found.is_none() {
    assert!(
      std::env::var_os("CI").is_none(),
      "CI is set and node is not on PATH: the subprocess binding's tests need it"
    );
    eprintln!("node is not on PATH: skipping a test of the subprocess binding");
  }
  found
}

/// A file in this repository, by its path from the root.
pub fn repo(path: &str) -> PathBuf {
  PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..").join(path)
}

/// Whether a process with this id is still running.
pub fn alive(pid: u32) -> bool {
  if cfg!(windows) {
    let output = Command::new("tasklist")
      .args(["/FI", &format!("PID eq {pid}"), "/NH"])
      .output()
      .expect("tasklist runs");
    String::from_utf8_lossy(&output.stdout).contains(&pid.to_string())
  } else {
    Command::new("kill")
      .args(["-0", &pid.to_string()])
      .stderr(std::process::Stdio::null())
      .status()
      .expect("kill runs")
      .success()
  }
}

/// Wait up to `within` for a process to be gone; whether it went.
pub fn gone_within(pid: u32, within: std::time::Duration) -> bool {
  let started = std::time::Instant::now();
  while started.elapsed() < within {
    if !alive(pid) {
      return true;
    }
    std::thread::sleep(std::time::Duration::from_millis(20));
  }
  !alive(pid)
}
