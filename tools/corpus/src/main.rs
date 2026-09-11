//! `cargo run -p pact_janus_corpus [-- accept]` (plan task 3.7): checks every case under
//! `corpora/` against its snapshots and its `result` (default), or regenerates the snapshots on a
//! diff (`accept`) — see `src/lib.rs`'s module docs for what each mode does and does not fix.

use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
  let accept = std::env::args().nth(1).as_deref() == Some("accept");
  let root = corpora_dir();
  let cases = pact_janus_corpus::discover_cases(&root);
  if cases.is_empty() {
    eprintln!("no corpus cases found under {}", root.display());
    return ExitCode::FAILURE;
  }

  let mut failed = 0usize;
  for dir in &cases {
    let report = pact_janus_corpus::run_case(dir, accept);
    let relative = report.dir.strip_prefix(&root).unwrap_or(&report.dir);
    if report.ok {
      println!("✓ corpora/{}", relative.display());
    } else {
      failed += 1;
      println!("✗ corpora/{} ({})", relative.display(), report.description);
      for problem in &report.problems {
        println!("    {problem}");
      }
    }
  }

  let passed = cases.len() - failed;
  println!("{passed}/{} cases passed", cases.len());
  if failed > 0 {
    ExitCode::FAILURE
  } else {
    ExitCode::SUCCESS
  }
}

fn corpora_dir() -> PathBuf {
  PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../corpora")
}
