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

/// ADR 0063 decision 6: two SDKs' reports for one live case carry the SHA-256 of the contract each
/// wrote, and `check` fails when they differ.
#[test]
fn check_fails_when_two_sdks_wrote_different_contracts() {
  let root = repo_root();
  let suite = root.join("conformance");
  let cases = pact_janus_conformance::load_cases(&suite).expect("cases");
  let dir = std::env::temp_dir().join(format!("janus-conformance-digests-{}", std::process::id()));
  std::fs::create_dir_all(&dir).unwrap();
  let report = |name: &str, digest: &str| {
    let entries: Vec<serde_json::Value> = cases
      .iter()
      .map(|case| {
        let mut entry = serde_json::json!({ "id": case.id, "status": "passed", "covers": case.covers });
        if case.id == "live/rfc-order-contract" {
          entry["contract-sha256"] = serde_json::json!(digest);
        }
        entry
      })
      .collect();
    let path = dir.join(format!("{name}.json"));
    let document = serde_json::json!({
      "$format": "janus-conformance-report/1",
      "sdk": { "name": name },
      "cases": entries,
    });
    std::fs::write(&path, serde_json::to_vec(&document).unwrap()).unwrap();
    path
  };

  let same = [report("one", "aa"), report("two", "aa")];
  let checked = pact_janus_conformance::check(&suite, &same);
  assert!(checked.ok(), "{:?}", checked.problems);

  let different = [report("one", "aa"), report("two", "bb")];
  let checked = pact_janus_conformance::check(&suite, &different);
  assert_eq!(checked.problems.len(), 1, "{:?}", checked.problems);
  assert!(
    checked.problems[0].starts_with("live/rfc-order-contract: the SDKs wrote different contracts"),
    "{}",
    checked.problems[0]
  );
  let _ = std::fs::remove_dir_all(&dir);
}
