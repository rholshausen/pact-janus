//! `schema-compat lint <dir>` — authoring rules (spec §2.2) on every
//! `*.schema.json` under `<dir>`, recursively.
//!
//! `schema-compat diff <base-dir> <head-dir>` — additive-evolution rules
//! (spec §11.2) between two checkouts of the same schema version directory.
//! A file present in base but missing in head is breaking; new files pass.
//!
//! Exit code 1 with one violation per line on stderr; silent 0 when clean.

use pact_janus_schema_compat::{Violation, diff_documents, lint_document};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn schema_files(dir: &Path) -> Result<Vec<PathBuf>, String> {
  let mut out = Vec::new();
  let entries = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
  for entry in entries {
    let path = entry.map_err(|e| e.to_string())?.path();
    if path.is_dir() {
      out.extend(schema_files(&path)?);
    } else if path
      .file_name()
      .is_some_and(|n| n.to_string_lossy().ends_with(".schema.json"))
    {
      out.push(path);
    }
  }
  out.sort();
  Ok(out)
}

fn load(path: &Path) -> Result<Value, String> {
  let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
  serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
}

fn report(file: &Path, violations: &[Violation]) {
  for v in violations {
    eprintln!("{}: {v}", file.display());
  }
}

fn lint(dir: &Path) -> Result<usize, String> {
  let files = schema_files(dir)?;
  if files.is_empty() {
    return Err(format!("no *.schema.json files under {}", dir.display()));
  }
  let mut count = 0;
  for file in files {
    let violations = lint_document(&load(&file)?);
    report(&file, &violations);
    count += violations.len();
  }
  Ok(count)
}

fn diff(base_dir: &Path, head_dir: &Path) -> Result<usize, String> {
  let mut count = 0;
  for base_file in schema_files(base_dir)? {
    let rel = base_file.strip_prefix(base_dir).expect("under base dir");
    let head_file = head_dir.join(rel);
    if head_file.is_file() {
      let violations = diff_documents(&load(&base_file)?, &load(&head_file)?);
      report(&head_file, &violations);
      count += violations.len();
    } else {
      eprintln!(
        "{}: schema file removed (spec §11.2: deprecate, never remove)",
        rel.display()
      );
      count += 1;
    }
  }
  Ok(count)
}

fn run() -> Result<usize, String> {
  let args: Vec<String> = std::env::args().collect();
  match args.iter().map(String::as_str).collect::<Vec<_>>()[1..] {
    ["lint", dir] => lint(Path::new(dir)),
    ["diff", base, head] => diff(Path::new(base), Path::new(head)),
    _ => Err("usage: schema-compat lint <dir> | schema-compat diff <base-dir> <head-dir>".into()),
  }
}

fn main() -> ExitCode {
  match run() {
    Ok(0) => ExitCode::SUCCESS,
    Ok(n) => {
      eprintln!("{n} violation(s)");
      ExitCode::FAILURE
    }
    Err(e) => {
      eprintln!("schema-compat: {e}");
      ExitCode::FAILURE
    }
  }
}
