//! `conformance [--spec <dir>] lint` checks the conformance corpus itself; `conformance [--spec <dir>]
//! check <report.json>...` checks what an SDK's run of it reported. Shared by this crate's own binary
//! and `janus-sdk-gates conformance` (staged plan S0.11). See `lib.rs` for what each one holds the
//! suite to.
//!
//! `--spec` is where `conformance/` and `Documentation/specs/` are: this repository's root, or an
//! unpacked `janus-spec.tar.gz` from a release, whose paths are the same. It defaults to the working
//! directory. `check` needs only the cases; `lint` reads the behavioural specification too.

use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "usage: conformance [--spec <dir>] [lint | check <report.json>...]";

/// Where the behavioural specification is, under `--spec`.
pub const BEHAVIOURAL_SPEC: &str = "Documentation/specs/sdk-specification/behavioural-spec.json";

/// Runs the command on its arguments (the program name excluded).
pub fn run(arguments: &[String]) -> ExitCode {
  let mut spec = PathBuf::from(".");
  let mut positional = Vec::new();
  let mut arguments = arguments.iter();
  while let Some(argument) = arguments.next() {
    match argument.as_str() {
      "--spec" => match arguments.next() {
        Some(dir) => spec = PathBuf::from(dir),
        None => return usage("--spec takes a directory"),
      },
      flag if flag.starts_with('-') => return usage(&format!("unknown option '{flag}'")),
      _ => positional.push(argument.clone()),
    }
  }
  let command = positional.first().cloned().unwrap_or_else(|| "lint".to_string());
  let rest: Vec<PathBuf> = positional.iter().skip(1).map(PathBuf::from).collect();
  let suite = spec.join("conformance");

  let report = match command.as_str() {
    "lint" if rest.is_empty() => crate::lint(&suite, &spec.join(BEHAVIOURAL_SPEC)),
    "check" => crate::check(&suite, &rest),
    other => return usage(&format!("unknown command '{other}'")),
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

fn usage(problem: &str) -> ExitCode {
  eprintln!("conformance: {problem}\n{USAGE}");
  ExitCode::from(2)
}
