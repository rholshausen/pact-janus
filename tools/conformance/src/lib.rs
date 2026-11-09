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
//!   later by two SDKs quietly disagreeing. It holds the compatibility facades' cases to their
//!   migration notes too (ADR 0076 decision 3): every row a note gives the verdict `kept` is named by
//!   a case of its facade, and every row a case names is in the note. And it holds the framework
//!   cases to `conformance/frameworks.json` (ADR 0078): every framework the list names has a case.
//! - [`check`] reads the reports SDK runs produce and decides whether each language passed: every
//!   case in the corpus that is the language's to run accounted for, and passed: every case that is
//!   not a facade's or a framework's, every case of each facade the language ships (ADR 0076
//!   decision 2), and every framework case with a subject in the language (ADR 0078 decision 5).
//!   That is what makes "this SDK is conformant" a
//!   claim a build makes rather than a maintainer asserts. Across the reports, a live case whose
//!   contract two SDKs wrote with different bytes fails too: neither SDK writes a contract, so
//!   they cannot differ (ADR 0063 decision 6).

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

pub mod cli;

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

/// A compatibility facade (ADR 0076 decision 2), as `conformance/facades.json` gives it: the
/// language whose SDK ships it and runs its cases, and the section of each migration note that gives
/// its spellings' verdicts.
pub struct Facade {
  pub language: String,
  /// Note path, relative to the repository -> the heading of the facade's section in it.
  pub notes: BTreeMap<String, String>,
}

/// Every facade the suite has cases for, by name.
pub fn load_facades(suite: &Path) -> Result<BTreeMap<String, Facade>, String> {
  let path = suite.join("facades.json");
  let text = fs::read_to_string(&path).map_err(|e| format!("reading {}: {e}", path.display()))?;
  let document: Value =
    serde_json::from_str(&text).map_err(|e| format!("{}: not JSON: {e}", path.display()))?;
  if document["$format"].as_str() != Some("janus-conformance-facades/1") {
    return Err(format!(
      "{}: not a facade list ('$format' is {})",
      path.display(),
      document["$format"]
    ));
  }
  let mut facades = BTreeMap::new();
  for (name, facade) in document["facades"]
    .as_object()
    .ok_or_else(|| format!("{}: no 'facades' object", path.display()))?
  {
    let language = facade["language"]
      .as_str()
      .ok_or_else(|| format!("{}: facade '{name}' names no 'language'", path.display()))?;
    let notes = facade["notes"]
      .as_object()
      .into_iter()
      .flatten()
      .filter_map(|(note, section)| section.as_str().map(|s| (note.clone(), s.to_string())))
      .collect();
    facades.insert(
      name.clone(),
      Facade {
        language: language.to_string(),
        notes,
      },
    );
  }
  Ok(facades)
}

/// A test framework an SDK integrates with (ADR 0078 decision 1), as `conformance/frameworks.json`
/// gives it: the language whose SDK runs its subjects, the suffix a subject for it has, and the style
/// guide that describes the integration, with the sections of it that do.
pub struct Framework {
  pub language: String,
  pub subject: String,
  /// The style guide's path, relative to the repository.
  pub guide: String,
  pub sections: Vec<String>,
}

/// Every framework the suite's framework cases run, by name.
pub fn load_frameworks(suite: &Path) -> Result<BTreeMap<String, Framework>, String> {
  let path = suite.join("frameworks.json");
  let text = fs::read_to_string(&path).map_err(|e| format!("reading {}: {e}", path.display()))?;
  let document: Value =
    serde_json::from_str(&text).map_err(|e| format!("{}: not JSON: {e}", path.display()))?;
  if document["$format"].as_str() != Some("janus-conformance-frameworks/1") {
    return Err(format!(
      "{}: not a framework list ('$format' is {})",
      path.display(),
      document["$format"]
    ));
  }
  let mut frameworks = BTreeMap::new();
  for (name, framework) in document["frameworks"]
    .as_object()
    .ok_or_else(|| format!("{}: no 'frameworks' object", path.display()))?
  {
    let member = |key: &str| {
      framework[key]
        .as_str()
        .map(String::from)
        .ok_or_else(|| format!("{}: framework '{name}' names no '{key}'", path.display()))
    };
    frameworks.insert(
      name.clone(),
      Framework {
        language: member("language")?,
        subject: member("subject")?,
        guide: framework["style"]["guide"]
          .as_str()
          .map(String::from)
          .ok_or_else(|| format!("{}: framework '{name}' names no style guide", path.display()))?,
        sections: framework["style"]["sections"]
          .as_array()
          .into_iter()
          .flatten()
          .filter_map(|section| section.as_str().map(String::from))
          .collect(),
      },
    );
  }
  Ok(frameworks)
}

/// One row of a migration note's tables: its first cell, verbatim, and its verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
  pub cell: String,
  pub verdict: String,
}

/// The verdicts a migration note gives (SDK spec §6.1).
const VERDICTS: [&str; 3] = ["kept", "adapted", "dropped"];

/// The rows of every table under the `## <section>` heading of a migration note, up to the next
/// `## ` heading: each row whose last cell is a verdict. `None` when the note has no such section.
pub fn note_rows(text: &str, section: &str) -> Option<Vec<Row>> {
  let mut rows = Vec::new();
  let mut inside = false;
  let mut found = false;
  for line in text.lines() {
    if let Some(heading) = line.strip_prefix("## ") {
      inside = heading.trim() == section;
      found |= inside;
      continue;
    }
    if !inside || !line.starts_with('|') {
      continue;
    }
    let cells = table_cells(line);
    if let (Some(first), Some(last)) = (cells.first(), cells.last())
      && cells.len() > 1
      && VERDICTS.contains(&last.as_str())
    {
      rows.push(Row {
        cell: first.clone(),
        verdict: last.clone(),
      });
    }
  }
  found.then_some(rows)
}

/// A Markdown table row's cells, trimmed. A `|` inside a code span does not end a cell.
fn table_cells(line: &str) -> Vec<String> {
  let mut cells = Vec::new();
  let mut cell = String::new();
  let mut code = false;
  for c in line.trim().trim_start_matches('|').chars() {
    match c {
      '`' => {
        code = !code;
        cell.push(c);
      }
      '|' if !code => cells.push(std::mem::take(&mut cell).trim().to_string()),
      _ => cell.push(c),
    }
  }
  if !cell.trim().is_empty() {
    cells.push(cell.trim().to_string());
  }
  cells
}

/// The facade a case belongs to, if it is a facade's.
fn facade_of(case: &Case) -> Option<&str> {
  case.document["facade"].as_str()
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

  for case in &cases {
    for (language, file) in case.document["subjects"].as_object().into_iter().flatten() {
      let file = file.as_str().unwrap_or_default();
      if !case.path.with_file_name(file).is_file() {
        report.problem(format!(
          "{}: its '{language}' subject '{file}' is not beside it",
          relative(&case.path, suite)
        ));
      }
    }
  }
  lint_facades(suite, &cases, &mut report);
  lint_frameworks(suite, &cases, &mut report);

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

/// The facades' cases against `facades.json` and the migration notes (ADR 0076 decisions 2 and 3):
/// each case names a facade the list has, a subject in its language alone, and rows its notes have;
/// and every row a note gives the verdict `kept` is named by a case of its facade. The notes are read
/// from the repository `suite` is in.
fn lint_facades(suite: &Path, cases: &[Case], report: &mut Report) {
  let facades = match load_facades(suite) {
    Ok(facades) => facades,
    Err(problem) => {
      report.problem(problem);
      return;
    }
  };
  let root = suite.parent().unwrap_or(suite);
  // (facade, note) -> its rows, read once.
  let mut rows: BTreeMap<(&str, &str), Vec<Row>> = BTreeMap::new();
  for (name, facade) in &facades {
    for (note, section) in &facade.notes {
      let path = root.join(note);
      let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(problem) => {
          report.problem(format!("facade '{name}': reading {note}: {problem}"));
          continue;
        }
      };
      match note_rows(&text, section) {
        Some(found) => {
          let mut seen = BTreeSet::new();
          for row in &found {
            if !seen.insert(row.cell.as_str()) {
              report.problem(format!(
                "{note}, '## {section}': two rows are '{}', and a case names a row by its first cell",
                row.cell
              ));
            }
          }
          rows.insert((name.as_str(), note.as_str()), found);
        }
        None => report.problem(format!("facade '{name}': {note} has no '## {section}' section")),
      }
    }
  }

  let mut covered: BTreeSet<(&str, &str, &str)> = BTreeSet::new();
  let mut facade_cases = 0usize;
  for case in cases {
    let relative_path = relative(&case.path, suite);
    let Some(name) = facade_of(case) else {
      continue;
    };
    facade_cases += 1;
    let Some(facade) = facades.get(name) else {
      report.problem(format!("{relative_path}: facade '{name}' is not in facades.json"));
      continue;
    };
    let languages: Vec<&String> = case.document["subjects"]
      .as_object()
      .map(|subjects| subjects.keys().collect())
      .unwrap_or_default();
    if languages != [&facade.language] {
      report.problem(format!(
        "{relative_path}: a '{name}' case has one subject, in {}, the language that ships it; it has {languages:?}",
        facade.language
      ));
    }
    for entry in case.document["rows"].as_array().into_iter().flatten() {
      let note = entry["note"].as_str().unwrap_or_default();
      let cell = entry["row"].as_str().unwrap_or_default();
      let Some(note_rows) = rows.get(&(name, note)) else {
        report.problem(format!(
          "{relative_path}: names a row of {note}, which facades.json does not give '{name}'"
        ));
        continue;
      };
      match note_rows.iter().find(|row| row.cell == cell) {
        Some(row) => {
          covered.insert((name, note, row.cell.as_str()));
        }
        None => report.problem(format!(
          "{relative_path}: names the row '{cell}', which {note}'s '## {}' section does not have",
          facade.notes[note]
        )),
      }
    }
  }

  let mut kept = 0usize;
  for ((name, note), found) in &rows {
    for row in found.iter().filter(|row| row.verdict == "kept") {
      kept += 1;
      if !covered.contains(&(*name, *note, row.cell.as_str())) {
        report.problem(format!(
          "{note}, '## {}': the row '{}' is kept, and no '{name}' case names it: a kept spelling no build checks is a claim (ADR 0076)",
          facades[*name].notes[*note], row.cell
        ));
      }
    }
  }
  let reached = covered
    .iter()
    .filter(|(name, note, cell)| {
      rows[&(*name, *note)]
        .iter()
        .any(|row| row.cell == *cell && row.verdict == "kept")
    })
    .count();
  report.note(format!(
    "{facade_cases} facade cases cover {reached} of the migration notes' {kept} kept rows, for {} facades",
    facades.len()
  ));
}

/// The framework cases against `frameworks.json` (ADR 0078 decision 5): each subject is keyed by a
/// framework the list has, and has its suffix; every framework is named by a case; and each style
/// guide the list names has its sections, when the guide is in the tree (it leaves with its SDK, S2.9).
fn lint_frameworks(suite: &Path, cases: &[Case], report: &mut Report) {
  let frameworks = match load_frameworks(suite) {
    Ok(frameworks) => frameworks,
    Err(problem) => {
      report.problem(problem);
      return;
    }
  };
  let root = suite.parent().unwrap_or(suite);
  for (name, framework) in &frameworks {
    let Ok(text) = fs::read_to_string(root.join(&framework.guide)) else {
      continue;
    };
    for section in &framework.sections {
      if !text
        .lines()
        .any(|line| line.strip_prefix("## ").is_some_and(|h| h.trim() == section))
      {
        report.problem(format!(
          "framework '{name}': {} has no '## {section}' section",
          framework.guide
        ));
      }
    }
  }

  let mut named: BTreeSet<&str> = BTreeSet::new();
  let mut framework_cases = 0usize;
  let mut subjects = 0usize;
  for case in cases.iter().filter(|case| case.category == "framework") {
    framework_cases += 1;
    let relative_path = relative(&case.path, suite);
    for (key, file) in case.document["subjects"].as_object().into_iter().flatten() {
      subjects += 1;
      let file = file.as_str().unwrap_or_default();
      let Some(framework) = frameworks.get(key) else {
        report.problem(format!(
          "{relative_path}: a framework case's subjects are keyed by framework, and '{key}' is not in frameworks.json"
        ));
        continue;
      };
      named.insert(key);
      if !file.ends_with(&framework.subject) {
        report.problem(format!(
          "{relative_path}: its '{key}' subject '{file}' is not a {} file",
          framework.subject
        ));
      }
    }
    if !case.document["expect"]["engine-log"].is_null() {
      report.problem(format!(
        "{relative_path}: a framework subject runs in a process of its own, and its driver reads no engine log ('engine-log')"
      ));
    }
  }
  for name in frameworks.keys() {
    if !named.contains(name.as_str()) {
      report.problem(format!(
        "framework '{name}' has no case: an integration no case runs is a claim (ADR 0078)"
      ));
    }
  }
  report.note(format!(
    "{framework_cases} framework cases run {subjects} subjects, for {} frameworks",
    frameworks.len()
  ));
}

/// The languages whose SDKs run a case: `None` for every SDK; a facade's language for its case; and
/// for a framework case, the languages of the frameworks it has subjects for (ADR 0078 decision 5).
/// A name neither list has is a language no report has, so the case is run by none and the lint says why.
fn runs_in(
  case: &Case,
  facades: &BTreeMap<String, Facade>,
  frameworks: &BTreeMap<String, Framework>,
) -> Option<BTreeSet<String>> {
  if let Some(name) = facade_of(case) {
    return Some(
      [facades.get(name).map_or_else(
        || format!("(no language: '{name}' is not in facades.json)"),
        |f| f.language.clone(),
      )]
      .into(),
    );
  }
  (case.category == "framework").then(|| {
    case.document["subjects"]
      .as_object()
      .into_iter()
      .flatten()
      .map(|(key, _)| {
        frameworks.get(key).map_or_else(
          || format!("(no language: '{key}' is not in frameworks.json)"),
          |f| f.language.clone(),
        )
      })
      .collect()
  })
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
  let facades = match load_facades(suite) {
    Ok(facades) => facades,
    Err(problem) => {
      report.problem(problem);
      return report;
    }
  };
  let frameworks = match load_frameworks(suite) {
    Ok(frameworks) => frameworks,
    Err(problem) => {
      report.problem(problem);
      return report;
    }
  };
  let corpus: BTreeMap<&str, Option<BTreeSet<String>>> = cases
    .iter()
    .map(|case| (case.id.as_str(), runs_in(case, &facades, &frameworks)))
    .collect();
  // Each case's written contract, by SDK, as the SHA-256 its report gives.
  let mut digests: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
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
    let language = document["sdk"]["language"].as_str().unwrap_or_default();
    // Every case every SDK runs, every case of each facade this language ships, and every framework
    // case with a subject for one of this language's frameworks.
    let expected: BTreeSet<&str> = corpus
      .iter()
      .filter(|(_, by)| by.as_ref().is_none_or(|by| by.contains(language)))
      .map(|(id, _)| *id)
      .collect();
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
      if let Some(digest) = entry["contract-sha256"].as_str() {
        digests
          .entry(id.to_string())
          .or_default()
          .push((sdk.clone(), digest.to_string()));
      }
      match corpus.get(id) {
        None => report.problem(format!(
          "{sdk}: reported '{id}', which is not a case in this corpus"
        )),
        Some(Some(by)) if !by.contains(language) => report.problem(format!(
          "{sdk}: reported '{id}', a case for {}, and this report's language is '{language}'",
          by.iter().cloned().collect::<Vec<_>>().join(" and ")
        )),
        _ => {}
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
        "{sdk}: did not run '{id}' — every case in the corpus runs in every language whose SDK it is for, or the claim is not conformance"
      ));
    }
    report.note(format!("{sdk}: {passed} of {} cases passed", expected.len()));
  }
  for (id, written) in &digests {
    let distinct: BTreeSet<&str> = written.iter().map(|(_, digest)| digest.as_str()).collect();
    if distinct.len() > 1 {
      let each: Vec<String> = written
        .iter()
        .map(|(sdk, digest)| format!("{sdk} {digest}"))
        .collect();
      report.problem(format!(
        "{id}: the SDKs wrote different contracts, where the engine writes the bytes and neither SDK changes them (ADR 0063): {}",
        each.join(", ")
      ));
    }
  }
  if !digests.is_empty() {
    report.note(format!(
      "{} contract(s) compared byte for byte across the reports",
      digests.len()
    ));
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

/// A path inside the suite, spelled the way a case's id spells it: with `/`, on every platform. A
/// case's id says where its file belongs, and on Windows the file's own path would never agree.
fn relative(path: &Path, root: &Path) -> String {
  path
    .strip_prefix(root)
    .unwrap_or(path)
    .components()
    .map(|part| part.as_os_str().to_string_lossy())
    .collect::<Vec<_>>()
    .join("/")
}
