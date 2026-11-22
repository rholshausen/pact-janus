//! The component conformance corpus (component-interfaces spec §9.4, staged plan S5.10, ADR 0105):
//! `cargo test -p pact_janus_corpus` runs every case under `corpora/components/` through each built-in
//! component's native and serialising bindings, as `cargo run -p pact_janus_corpus` does.

use pact_janus_corpus::components::{built_ins, discover, run_file};
use serde_json::Value;
use std::path::{Path, PathBuf};

fn components_dir() -> PathBuf {
  PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../corpora/components")
}

fn schema() -> Value {
  let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    .join("../../Documentation/specs/component-interfaces/schemas/v1/conformance-case.schema.json");
  serde_json::from_str(&std::fs::read_to_string(path).expect("the case schema")).expect("JSON")
}

fn relative(path: &Path) -> String {
  path
    .strip_prefix(components_dir())
    .unwrap_or(path)
    .display()
    .to_string()
}

#[test]
fn every_component_case_passes_through_both_bindings() {
  let files = discover(&components_dir());
  let built_ins = built_ins();
  let mut failures = Vec::new();
  for file in &files {
    match run_file(file, &built_ins) {
      Ok(report) => {
        assert_eq!(
          report.bindings,
          vec!["native", "serialising"],
          "{}",
          relative(file)
        );
        failures.extend(
          report
            .problems
            .iter()
            .map(|problem| format!("{}: {problem}", relative(file))),
        );
      }
      Err(problem) => failures.push(format!("{}: {problem}", relative(file))),
    }
  }
  assert!(
    failures.is_empty(),
    "{} problem(s) in {} component case(s):\n{}",
    failures.len(),
    files.len(),
    failures.join("\n")
  );
}

#[test]
fn every_component_case_is_a_conformance_case() {
  let validator = jsonschema::validator_for(&schema()).expect("a valid schema");
  let mut invalid = Vec::new();
  for file in discover(&components_dir()) {
    let case: Value = serde_json::from_str(&std::fs::read_to_string(&file).expect("readable")).expect("JSON");
    for error in validator.iter_errors(&case) {
      invalid.push(format!(
        "{}: {} at {}",
        relative(&file),
        error,
        error.instance_path()
      ));
    }
  }
  assert!(invalid.is_empty(), "{}", invalid.join("\n"));
}

/// Spec §9.4: every in-tree component is held to the corpus, so a built-in with no case is one nothing
/// holds to its documents.
#[test]
fn every_built_in_component_has_cases() {
  let files = discover(&components_dir());
  let missing: Vec<&str> = built_ins()
    .iter()
    .map(|built_in| built_in.name)
    .filter(|name| {
      !files.iter().any(|file| {
        file
          .parent()
          .and_then(Path::file_name)
          .is_some_and(|dir| dir == *name)
      })
    })
    .collect();
  assert!(missing.is_empty(), "no cases for {missing:?}");
}
