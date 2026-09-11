//! Plan task 3.7: `cargo test -p pact_janus_corpus` is the check-mode entry point CI runs (this
//! file), mirroring `cargo run -p pact_janus_corpus` exactly — both call `pact_janus_corpus::
//! run_case` in check mode, so there is exactly one place that decides what a corpus case asserts.

use std::path::PathBuf;

fn corpora_dir() -> PathBuf {
  PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../corpora")
}

#[test]
fn every_golden_corpus_case_passes() {
  let root = corpora_dir();
  let cases = pact_janus_corpus::discover_cases(&root);
  assert!(
    !cases.is_empty(),
    "expected at least one corpus case under {}",
    root.display()
  );

  let mut failures = Vec::new();
  for dir in &cases {
    let report = pact_janus_corpus::run_case(dir, false);
    if !report.ok {
      let relative = dir.strip_prefix(&root).unwrap_or(dir);
      failures.push(format!(
        "corpora/{}: {}",
        relative.display(),
        report.problems.join("; ")
      ));
    }
  }
  assert!(
    failures.is_empty(),
    "{} of {} corpus case(s) failed:\n{}",
    failures.len(),
    cases.len(),
    failures.join("\n")
  );
}
