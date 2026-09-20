//! The corpus is checked by `cargo test`, like the golden corpora are: a case that does not match
//! its schema, or a conformance id with no case, fails the Rust build — not later, in whichever
//! SDK happens to run next.

use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../..")
    .canonicalize()
    .expect("repo root")
}

#[test]
fn the_corpus_lints() {
  let root = repo_root();
  let report = pact_janus_conformance::lint(
    &root.join("conformance"),
    &root.join("Documentation/specs/sdk-specification/behavioural-spec.json"),
  );
  assert!(
    report.ok(),
    "conformance corpus:\n  {}",
    report.problems.join("\n  ")
  );
}

#[test]
fn every_case_says_which_specification_sentence_it_pins() {
  let root = repo_root();
  let cases = pact_janus_conformance::load_cases(&root.join("conformance")).expect("cases");
  let missing: Vec<&str> = cases
    .iter()
    .filter(|case| {
      case.document["why"]
        .as_str()
        .unwrap_or_default()
        .trim()
        .is_empty()
    })
    .map(|case| case.id.as_str())
    .collect();
  assert!(
    missing.is_empty(),
    "a case is a scenario for a sentence of the specification, and says which: {missing:?}"
  );
}
