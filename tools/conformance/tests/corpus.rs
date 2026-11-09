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
      .filter(|case| case.document.get("facade").is_none() && case.category != "framework")
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

/// A report as an SDK of `language` writes it, every case it names passed, at a scratch path.
fn passing_report(dir: &Path, language: &str, ids: &[&str]) -> PathBuf {
  let entries: Vec<serde_json::Value> = ids
    .iter()
    .map(|id| serde_json::json!({ "id": id, "status": "passed", "covers": [] }))
    .collect();
  let path = dir.join(format!("{language}.json"));
  let document = serde_json::json!({
    "$format": "janus-conformance-report/1",
    "sdk": { "name": format!("pact-janus-{language}"), "language": language },
    "cases": entries,
  });
  std::fs::write(&path, serde_json::to_vec(&document).unwrap()).unwrap();
  path
}

/// The languages whose SDKs run a case, as ADR 0076 decision 2 and ADR 0078 decision 5 say: `None`
/// for every SDK.
fn languages_of(suite: &Path, case: &pact_janus_conformance::Case) -> Option<Vec<String>> {
  let facades = pact_janus_conformance::load_facades(suite).expect("facades");
  let frameworks = pact_janus_conformance::load_frameworks(suite).expect("frameworks");
  if let Some(name) = case.document["facade"].as_str() {
    return Some(vec![facades[name].language.clone()]);
  }
  (case.category == "framework").then(|| {
    let mut languages: Vec<String> = case.document["subjects"]
      .as_object()
      .into_iter()
      .flatten()
      .map(|(key, _)| frameworks[key].language.clone())
      .collect();
    languages.sort();
    languages.dedup();
    languages
  })
}

/// ADR 0076 decision 2: an SDK runs every case that is not a facade's, and every case of each facade
/// its language ships. Leaving one of its facade's cases out fails it, and so does reporting another
/// language's.
#[test]
fn a_report_runs_its_own_languages_facade_cases_and_no_others() {
  let root = repo_root();
  let suite = root.join("conformance");
  let cases = pact_janus_conformance::load_cases(&suite).expect("cases");
  let dir = std::env::temp_dir().join(format!("janus-conformance-facades-{}", std::process::id()));
  std::fs::create_dir_all(&dir).unwrap();
  let runs_in_jvm = |case: &pact_janus_conformance::Case| {
    languages_of(&suite, case).is_none_or(|l| l.contains(&"jvm".to_string()))
  };
  let own: Vec<&str> = cases
    .iter()
    .filter(|case| runs_in_jvm(case))
    .map(|case| case.id.as_str())
    .collect();
  let facade_case = cases
    .iter()
    .find(|case| case.category == "facade" && runs_in_jvm(case))
    .expect("a pact-jvm case");
  let other = cases
    .iter()
    .find(|case| case.category == "facade" && !runs_in_jvm(case))
    .expect("a pact-js case");

  let checked = pact_janus_conformance::check(&suite, &[passing_report(&dir, "jvm", &own)]);
  assert!(checked.ok(), "{:?}", checked.problems);

  let without: Vec<&str> = own.iter().copied().filter(|id| *id != facade_case.id).collect();
  let checked = pact_janus_conformance::check(&suite, &[passing_report(&dir, "jvm", &without)]);
  assert_eq!(checked.problems.len(), 1, "{:?}", checked.problems);
  assert!(checked.problems[0].contains(&format!("did not run '{}'", facade_case.id)));

  let with_other: Vec<&str> = own.iter().copied().chain([other.id.as_str()]).collect();
  let checked = pact_janus_conformance::check(&suite, &[passing_report(&dir, "jvm", &with_other)]);
  assert_eq!(checked.problems.len(), 1, "{:?}", checked.problems);
  assert!(
    checked.problems[0].contains("a case for typescript, and this report's language is 'jvm'"),
    "{}",
    checked.problems[0]
  );
  let _ = std::fs::remove_dir_all(&dir);
}

/// ADR 0078 decision 5: a framework case is run by every SDK it has a subject for, and by no other.
/// A case for both languages is in both reports; one for a single framework is in its language's alone.
#[test]
fn a_framework_case_is_run_by_each_language_it_has_a_subject_for() {
  let root = repo_root();
  let suite = root.join("conformance");
  let cases = pact_janus_conformance::load_cases(&suite).expect("cases");
  let dir = std::env::temp_dir().join(format!("janus-conformance-frameworks-{}", std::process::id()));
  std::fs::create_dir_all(&dir).unwrap();
  let both = cases
    .iter()
    .find(|case| languages_of(&suite, case).is_some_and(|l| l.len() == 2))
    .expect("a framework case for both languages");
  let jvm_only = cases
    .iter()
    .find(|case| languages_of(&suite, case).is_some_and(|l| l == ["jvm"]))
    .expect("a framework case for the JVM alone");
  let own = |language: &str| -> Vec<&str> {
    cases
      .iter()
      .filter(|case| languages_of(&suite, case).is_none_or(|l| l.contains(&language.to_string())))
      .map(|case| case.id.as_str())
      .collect()
  };
  let (jvm, typescript) = (own("jvm"), own("typescript"));
  assert!(jvm.contains(&both.id.as_str()) && typescript.contains(&both.id.as_str()));
  assert!(!typescript.contains(&jvm_only.id.as_str()));

  let checked = pact_janus_conformance::check(
    &suite,
    &[
      passing_report(&dir, "jvm", &jvm),
      passing_report(&dir, "typescript", &typescript),
    ],
  );
  assert!(checked.ok(), "{:?}", checked.problems);

  let without: Vec<&str> = typescript.iter().copied().filter(|id| *id != both.id).collect();
  let checked = pact_janus_conformance::check(&suite, &[passing_report(&dir, "typescript", &without)]);
  assert_eq!(checked.problems.len(), 1, "{:?}", checked.problems);
  assert!(checked.problems[0].contains(&format!("did not run '{}'", both.id)));

  let with_jvm_only: Vec<&str> = typescript.iter().copied().chain([jvm_only.id.as_str()]).collect();
  let checked = pact_janus_conformance::check(&suite, &[passing_report(&dir, "typescript", &with_jvm_only)]);
  assert_eq!(checked.problems.len(), 1, "{:?}", checked.problems);
  assert!(checked.problems[0].contains("a case for jvm, and this report's language is 'typescript'"));
  let _ = std::fs::remove_dir_all(&dir);
}

/// ADR 0076 decision 3: a note's rows are its tables' rows whose last cell is a verdict, under the
/// facade's own section, each by its first cell, verbatim. A `|` inside a code span is not a cell's end.
#[test]
fn a_notes_rows_are_read_from_the_facades_section() {
  let note = "\
# Moving

| a | b | Verdict |
|---|---|---|
| `outside` | x | kept |

## pact-js

| pact-js | Janus | Verdict |
|---|---|---|
| `like(v)` on a value | `type` | kept |
| `fn | { setup }` | a handler | adapted |
| `config` | Janus's own | — |

## pact-jvm (JUnit 5)

| `inside jvm` | x | dropped |
";
  use pact_janus_conformance::{Row, note_rows};
  let row = |cell: &str, verdict: &str| Row {
    cell: cell.to_string(),
    verdict: verdict.to_string(),
  };
  pretty_assertions::assert_eq!(
    note_rows(note, "pact-js"),
    Some(vec![
      row("`like(v)` on a value", "kept"),
      row("`fn | { setup }`", "adapted")
    ])
  );
  pretty_assertions::assert_eq!(
    note_rows(note, "pact-jvm (JUnit 5)"),
    Some(vec![row("`inside jvm`", "dropped")])
  );
  pretty_assertions::assert_eq!(note_rows(note, "pact-python"), None);
}
