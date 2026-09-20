//! `cargo run -p pact_janus_conformance -- lint` checks the conformance corpus itself; `-- check
//! <report.json>...` checks what an SDK's run of it reported. See `src/lib.rs` for what each one
//! holds the suite to.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn main() -> ExitCode {
  let mut args = std::env::args().skip(1);
  let command = args.next().unwrap_or_else(|| "lint".to_string());
  let rest: Vec<PathBuf> = args.map(PathBuf::from).collect();
  let root = repo_root();
  let suite = root.join("conformance");
  let behavioural_spec = root.join("Documentation/specs/sdk-specification/behavioural-spec.json");

  let report = match command.as_str() {
    "lint" => pact_janus_conformance::lint(&suite, &behavioural_spec),
    "check" => pact_janus_conformance::check(&suite, &rest),
    other => {
      eprintln!("usage: conformance [lint | check <report.json>...]\n  (unknown command '{other}')");
      return ExitCode::from(2);
    }
  };

  for note in &report.summary {
    println!("  {note}");
  }
  for problem in &report.problems {
    println!("✗ {problem}");
  }
  if report.ok() {
    println!("✓ conformance {command}");
    ExitCode::SUCCESS
  } else {
    println!("{} problems", report.problems.len());
    ExitCode::FAILURE
  }
}

/// The repository root: this crate lives at `tools/conformance`.
fn repo_root() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../..")
    .canonicalize()
    .expect("repo root")
}
