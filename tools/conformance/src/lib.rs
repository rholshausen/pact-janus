//! The SDK conformance suite's checker (plan task 6.4, SDK spec §7, ADR 0017).
//!
//! Two jobs, and neither of them is running a case — a case runs in a language, through that
//! language's SDK, which is the whole point:
//!
//! - [`lint`] holds the corpus to its own rules: every case valid against
//!   `conformance/schemas/v1/conformance-case.schema.json`, every case id unique and matching its
//!   path, every conformance id a case claims to cover actually named by a primitive in the
//!   behavioural specification, and — the load-bearing one — every conformance id in the
//!   behavioural specification covered by at least one case. An id with no case is a hole in what
//!   "conformant" guarantees (ADR 0017's "Harder"), so it fails here rather than being discovered
//!   later by two SDKs quietly disagreeing.
//! - [`check`] reads the reports SDK runs produce and decides whether each language passed: every
//!   case in the corpus accounted for, and passed. That is what makes "this SDK is conformant" a
//!   claim a build makes rather than a maintainer asserts.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

/// What a run of the checker found. Empty `problems` is a pass.
#[derive(Debug, Default)]
pub struct Report {
  pub problems: Vec<String>,
  pub summary: Vec<String>,
}

impl Report {
  fn problem(&mut self, text: impl Into<String>) {
    self.problems.push(text.into());
  }

  fn note(&mut self, text: impl Into<String>) {
    self.summary.push(text.into());
  }

  pub fn ok(&self) -> bool {
    self.problems.is_empty()
  }
}

/// One case, as the checker needs it: where it came from, and what it says about itself.
pub struct Case {
  pub path: PathBuf,
  pub id: String,
  pub category: String,
  pub covers: Vec<String>,
  pub document: Value,
}

/// Every case under `<suite>/cases`, in id order.
pub fn load_cases(suite: &Path) -> Result<Vec<Case>, String> {
  let root = suite.join("cases");
  let mut cases = Vec::new();
  let mut categories: Vec<PathBuf> = fs::read_dir(&root)
    .map_err(|e| format!("reading {}: {e}", root.display()))?
    .filter_map(|entry| entry.ok().map(|e| e.path()))
    .filter(|path| path.is_dir())
    .collect();
  categories.sort();
  for category in categories {
    let mut files: Vec<PathBuf> = fs::read_dir(&category)
      .map_err(|e| format!("reading {}: {e}", category.display()))?
      .filter_map(|entry| entry.ok().map(|e| e.path()))
      .filter(|path| path.extension().is_some_and(|e| e == "json"))
      .collect();
    files.sort();
    for path in files {
      let text = fs::read_to_string(&path).map_err(|e| format!("reading {}: {e}", path.display()))?;
      let document: Value =
        serde_json::from_str(&text).map_err(|e| format!("{}: not JSON: {e}", path.display()))?;
      cases.push(Case {
        id: document["id"].as_str().unwrap_or_default().to_string(),
        category: document["category"].as_str().unwrap_or_default().to_string(),
        covers: document["covers"]
          .as_array()
          .map(|ids| {
            ids
              .iter()
              .filter_map(|id| id.as_str().map(String::from))
              .collect()
          })
          .unwrap_or_default(),
        path,
        document,
      });
    }
  }
  cases.sort_by(|a, b| a.id.cmp(&b.id));
  Ok(cases)
}

/// Conformance id -> the primitive that names it, from the canonical behavioural specification.
pub fn behavioural_ids(spec: &Path) -> Result<BTreeMap<String, String>, String> {
  let text = fs::read_to_string(spec).map_err(|e| format!("reading {}: {e}", spec.display()))?;
  let document: Value =
    serde_json::from_str(&text).map_err(|e| format!("{}: not JSON: {e}", spec.display()))?;
  let mut ids = BTreeMap::new();
  for primitive in document["primitives"].as_array().ok_or("no 'primitives' array")? {
    let name = primitive["id"].as_str().unwrap_or_default().to_string();
    for id in primitive["conformance"].as_array().into_iter().flatten() {
      if let Some(id) = id.as_str() {
        ids.insert(id.to_string(), name.clone());
      }
    }
  }
  Ok(ids)
}

/// The corpus against its schema, its own id rules, and the behavioural specification's ids.
pub fn lint(suite: &Path, behavioural_spec: &Path) -> Report {
  let mut report = Report::default();
  let cases = match load_cases(suite) {
    Ok(cases) => cases,
    Err(problem) => {
      report.problem(problem);
      return report;
    }
  };
  if cases.is_empty() {
    report.problem(format!("no cases under {}", suite.join("cases").display()));
    return report;
  }

  let schema_path = suite.join("schemas/v1/conformance-case.schema.json");
  match compile_schema(&schema_path) {
    Ok(validator) => {
      for case in &cases {
        for error in validator.iter_errors(&case.document) {
          report.problem(format!(
            "{}: does not match the case schema at {}: {error}",
            relative(&case.path, suite),
            error.instance_path()
          ));
        }
      }
    }
    Err(problem) => report.problem(problem),
  }

  let mut seen: BTreeSet<&str> = BTreeSet::new();
  for case in &cases {
    let relative_path = relative(&case.path, suite);
    if !seen.insert(&case.id) {
      report.problem(format!(
        "{relative_path}: a second case claims the id '{}'",
        case.id
      ));
    }
    let expected = format!(
      "cases/{}/{}.json",
      case.category,
      case.id.rsplit('/').next().unwrap_or_default()
    );
    if relative_path != expected {
      report.problem(format!(
        "{relative_path}: a case with id '{}' belongs at {expected}",
        case.id
      ));
    }
  }

  let ids = match behavioural_ids(behavioural_spec) {
    Ok(ids) => ids,
    Err(problem) => {
      report.problem(problem);
      return report;
    }
  };
  let mut covered: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
  for case in &cases {
    for id in &case.covers {
      if !ids.contains_key(id) {
        report.problem(format!(
          "{}: covers '{id}', which no primitive in the behavioural specification names",
          relative(&case.path, suite)
        ));
      }
      covered.entry(id.as_str()).or_default().push(&case.id);
    }
  }
  for (id, primitive) in &ids {
    if !covered.contains_key(id.as_str()) {
      report.problem(format!(
        "'{id}' ({primitive}) has no case: an id with no scenario is a hole in what 'conformant' guarantees (ADR 0017)"
      ));
    }
  }

  let mut by_category: BTreeMap<&str, usize> = BTreeMap::new();
  for case in &cases {
    *by_category.entry(case.category.as_str()).or_default() += 1;
  }
  let reached = ids.keys().filter(|id| covered.contains_key(id.as_str())).count();
  report.note(format!(
    "{} cases ({}) cover {reached} of the behavioural specification's {} conformance ids",
    cases.len(),
    by_category
      .iter()
      .map(|(category, count)| format!("{count} {category}"))
      .collect::<Vec<_>>()
      .join(", "),
    ids.len()
  ));
  report
}

/// An SDK's run report against the corpus: every case accounted for, and passed.
pub fn check(suite: &Path, reports: &[PathBuf]) -> Report {
  let mut report = Report::default();
  let cases = match load_cases(suite) {
    Ok(cases) => cases,
    Err(problem) => {
      report.problem(problem);
      return report;
    }
  };
  let expected: BTreeSet<&str> = cases.iter().map(|case| case.id.as_str()).collect();
  if reports.is_empty() {
    report.problem("no reports given: `conformance check <report.json>...`".to_string());
    return report;
  }

  for path in reports {
    let text = match fs::read_to_string(path) {
      Ok(text) => text,
      Err(problem) => {
        report.problem(format!("reading {}: {problem}", path.display()));
        continue;
      }
    };
    let document: Value = match serde_json::from_str(&text) {
      Ok(document) => document,
      Err(problem) => {
        report.problem(format!("{}: not JSON: {problem}", path.display()));
        continue;
      }
    };
    let sdk = document["sdk"]["name"]
      .as_str()
      .unwrap_or("(unnamed SDK)")
      .to_string();
    if document["$format"].as_str() != Some("janus-conformance-report/1") {
      report.problem(format!(
        "{}: not a conformance report ('$format' is {})",
        path.display(),
        document["$format"]
      ));
      continue;
    }
    let mut reported: BTreeSet<&str> = BTreeSet::new();
    let mut passed = 0usize;
    for entry in document["cases"].as_array().into_iter().flatten() {
      let Some(id) = entry["id"].as_str() else {
        report.problem(format!("{sdk}: a report entry has no 'id'"));
        continue;
      };
      reported.insert(id);
      if !expected.contains(id) {
        report.problem(format!(
          "{sdk}: reported '{id}', which is not a case in this corpus"
        ));
      }
      match entry["status"].as_str() {
        Some("passed") => passed += 1,
        Some(status) => report.problem(format!(
          "{sdk}: {id} {status}{}",
          entry["detail"]
            .as_str()
            .map(|d| format!("\n    {}", d.replace('\n', "\n    ")))
            .unwrap_or_default()
        )),
        None => report.problem(format!("{sdk}: {id} has no 'status'")),
      }
    }
    for id in expected.difference(&reported) {
      report.problem(format!(
        "{sdk}: did not run '{id}' — every case in the corpus runs in every language, or the claim is not conformance"
      ));
    }
    report.note(format!("{sdk}: {passed} of {} cases passed", expected.len()));
  }
  report
}

fn compile_schema(path: &Path) -> Result<jsonschema::Validator, String> {
  let text = fs::read_to_string(path).map_err(|e| format!("reading {}: {e}", path.display()))?;
  let schema: Value =
    serde_json::from_str(&text).map_err(|e| format!("{}: not JSON: {e}", path.display()))?;
  jsonschema::options()
    .build(&schema)
    .map_err(|e| format!("compiling {}: {e}", path.display()))
}

fn relative(path: &Path, root: &Path) -> String {
  path.strip_prefix(root).unwrap_or(path).display().to_string()
}
