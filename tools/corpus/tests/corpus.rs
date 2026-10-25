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

/// A case named `upgrade-…` asserts that a pact and the contract `janus upgrade` converts it into
/// reach one verdict (plan-grammar spec §4.4). Its `also-compiled-from` is written out so a reader
/// sees the shapes, and this holds it to what the upgrade writes today: the interaction's
/// description, parts, content types and requirements.
#[test]
fn every_upgrade_case_compiles_what_the_upgrade_writes() {
  let root = corpora_dir().join("legacy");
  let mut checked = 0;
  let mut stale = Vec::new();
  for dir in pact_janus_corpus::discover_cases(&root) {
    let name = dir.file_name().unwrap().to_string_lossy().to_string();
    if !name.starts_with("upgrade-") {
      continue;
    }
    let case: serde_json::Value =
      serde_json::from_str(&std::fs::read_to_string(dir.join("case.json")).unwrap()).unwrap();
    let pact = &case["input"]["pact"];
    let upgraded = pact_janus_kernel::legacy::upgrade::pact("corpus case", pact).expect("the pact upgrades");
    let index = case["input"]["index"].as_u64().unwrap_or(0) as usize;
    let interaction = serde_json::to_value(&upgraded.contract.interactions[index]).unwrap();
    let mut expected = serde_json::Map::new();
    for member in ["description", "parts", "content-types", "requires"] {
      if let Some(value) = interaction.get(member) {
        expected.insert(member.to_string(), value.clone());
      }
    }
    let written = &case["also-compiled-from"]["spec"];
    if *written != serde_json::Value::Object(expected.clone()) {
      stale.push(format!(
        "corpora/legacy/{name}: also-compiled-from is not what the upgrade writes:\n{}",
        serde_json::to_string_pretty(&expected).unwrap()
      ));
    }
    checked += 1;
  }
  assert!(checked >= 10, "only {checked} upgrade cases");
  assert!(stale.is_empty(), "{}", stale.join("\n"));
}
