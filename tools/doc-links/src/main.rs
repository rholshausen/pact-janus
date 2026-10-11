//! `doc-links` — the documentation link check (staged plan S0.12).
//!
//! Checks every Markdown file in the repository, or under the directory given, and exits non-zero
//! if a relative link names a file that is not there or a heading the file does not have — the form
//! CI runs, so "every link resolves" is a command's exit code rather than something a reviewer
//! notices.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use pact_janus_doc_links::check;

fn main() -> ExitCode {
  let arguments: Vec<String> = std::env::args().skip(1).collect();
  let root = match arguments.as_slice() {
    [] => repo_root(),
    [root] if !root.starts_with('-') => PathBuf::from(root),
    _ => {
      eprintln!("doc-links: usage: doc-links [root]");
      return ExitCode::from(2);
    }
  };

  let report = match check(&root) {
    Ok(report) => report,
    Err(error) => {
      eprintln!("doc-links: {}: {error}", root.display());
      return ExitCode::from(2);
    }
  };

  for broken in &report.broken {
    println!("✗ {broken}");
  }
  if report.broken.is_empty() {
    println!(
      "✓ doc-links: {} relative links in {} Markdown files, and every one resolves",
      report.links, report.files
    );
    ExitCode::SUCCESS
  } else {
    println!(
      "\n{} of {} relative links in {} Markdown files do not resolve",
      report.broken.len(),
      report.links,
      report.files
    );
    ExitCode::FAILURE
  }
}

/// The repository root: this crate sits at `tools/doc-links`, two levels below it.
fn repo_root() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .parent()
    .and_then(Path::parent)
    .map(Path::to_path_buf)
    .unwrap_or_else(|| PathBuf::from("."))
}
