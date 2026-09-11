//! The golden-corpus runner (plan task 3.7, plan-grammar spec §6): discovers every case under
//! `corpora/`, compiles and executes its `input` (and `also-compiled-from`, when present) against
//! its `values`, and reports two kinds of red that a reader must be able to tell apart (spec
//! §6.2):
//!
//! - **A `result` diff is a behaviour change** — never auto-fixed, in either mode.
//! - **A `plan.txt`/`executed.txt` diff is a snapshot** — in `accept` mode it is regenerated; in
//!   check mode (what `cargo test` and CI run) it is reported as a failure to review.
//!
//! [`run_case`] is the one entry point both `src/main.rs` (`cargo run -p pact_janus_corpus
//! [-- accept]`) and `tests/corpus.rs` (`cargo test -p pact_janus_corpus`) call, so the two never
//! drift into checking different things.

use pact_janus_kernel::interaction_spec;
use pact_janus_kernel::legacy_pact;
use pact_janus_kernel::plan::{self, Assignment, CapturedValues, Mismatch, Plan, Status};
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

// --- case.json (corpus-case.schema.json) ---

#[derive(Deserialize)]
struct CaseFile {
  description: String,
  input: Input,
  values: BTreeMap<String, Value>,
  result: ExpectedResult,
  #[serde(rename = "also-compiled-from")]
  also_compiled_from: Option<Input>,
}

#[derive(Deserialize)]
struct Input {
  kind: String,
  spec: Option<Value>,
  pact: Option<Value>,
  index: Option<usize>,
  variant: Option<String>,
}

#[derive(Deserialize)]
struct ExpectedResult {
  status: String,
  #[serde(default)]
  mismatches: Vec<ExpectedMismatch>,
}

#[derive(Deserialize, Clone, PartialEq, Debug)]
struct ExpectedMismatch {
  path: String,
  message: String,
  action: Option<String>,
}

// --- discovery ---

/// Every directory under `root` that holds a `case.json` — recursively, so cases may be grouped
/// into subdirectories (`corpora/shapes/`, `corpora/legacy/`, ...) for readability.
pub fn discover_cases(root: &Path) -> Vec<PathBuf> {
  let mut cases = Vec::new();
  discover_into(root, &mut cases);
  cases.sort();
  cases
}

fn discover_into(dir: &Path, out: &mut Vec<PathBuf>) {
  let Ok(entries) = std::fs::read_dir(dir) else {
    return;
  };
  let mut has_case = false;
  let mut subdirs = Vec::new();
  for entry in entries.flatten() {
    let path = entry.path();
    if path.is_dir() {
      subdirs.push(path);
    } else if path.file_name().and_then(|n| n.to_str()) == Some("case.json") {
      has_case = true;
    }
  }
  if has_case {
    out.push(dir.to_path_buf());
  }
  for subdir in subdirs {
    discover_into(&subdir, out);
  }
}

// --- running one case ---

pub struct CaseReport {
  pub dir: PathBuf,
  pub description: String,
  pub ok: bool,
  pub problems: Vec<String>,
}

/// Run one case. `accept: true` regenerates `plan.txt`/`executed.txt` on a snapshot diff instead
/// of reporting it; a `result` diff is always reported, in both modes (module docs).
pub fn run_case(dir: &Path, accept: bool) -> CaseReport {
  let mut problems = Vec::new();
  let case: CaseFile = match std::fs::read_to_string(dir.join("case.json"))
    .map_err(|e| e.to_string())
    .and_then(|text| serde_json::from_str(&text).map_err(|e| e.to_string()))
  {
    Ok(case) => case,
    Err(err) => {
      return CaseReport {
        dir: dir.to_path_buf(),
        description: String::new(),
        ok: false,
        problems: vec![format!("case.json: {err}")],
      };
    }
  };

  let plan = match compile_input(&case.input) {
    Ok((plan, _)) => Some(plan),
    Err(err) => {
      problems.push(format!("input: {err}"));
      None
    }
  };

  let resolver = CapturedValues::from_json(&case.values);
  if let Some(plan) = &plan {
    let executed = plan::execute(plan, &resolver);
    let (status, mismatches) = plan::outcome(&executed);
    if let Err(err) = check_result(status, &mismatches, &case.result) {
      problems.push(format!("result: {err}"));
    }
    check_snapshot(
      &dir.join("plan.txt"),
      &plan::render_pretty(plan),
      accept,
      &mut problems,
    );
    check_snapshot(
      &dir.join("executed.txt"),
      &plan::render_executed(&executed),
      accept,
      &mut problems,
    );
  }

  if let Some(other_input) = &case.also_compiled_from {
    match compile_input(other_input) {
      Ok((other_plan, _)) => {
        let other_executed = plan::execute(&other_plan, &resolver);
        let (other_status, other_mismatches) = plan::outcome(&other_executed);
        if let Err(err) = check_result(other_status, &other_mismatches, &case.result) {
          problems.push(format!("also-compiled-from result: {err}"));
        }
      }
      Err(err) => problems.push(format!("also-compiled-from input: {err}")),
    }
  }

  CaseReport {
    dir: dir.to_path_buf(),
    description: case.description,
    ok: problems.is_empty(),
    problems,
  }
}

/// Compiles `input`, returning the plan and a description (the pact interaction's own, for
/// `kind: pact-interaction`; the case doesn't otherwise have one to offer for `kind: spec`, whose
/// description already labels the compiled root).
fn compile_input(input: &Input) -> Result<(Plan, String), String> {
  match input.kind.as_str() {
    "spec" => {
      let spec_json = input.spec.as_ref().ok_or("kind 'spec' requires 'spec'")?;
      let spec = interaction_spec::parse(spec_json).map_err(|err| format!("{:?}", err.problems))?;
      let assignment = input.variant.as_deref().map(parse_variant).unwrap_or_default();
      let plan = plan::compile(&spec, &assignment, input.variant.as_deref());
      Ok((plan, spec.description.clone()))
    }
    "pact-interaction" => {
      let pact_json = input
        .pact
        .as_ref()
        .ok_or("kind 'pact-interaction' requires 'pact'")?;
      let pact = legacy_pact::read("corpus case", pact_json).map_err(|err| err.to_string())?;
      let interactions = legacy_pact::http_interactions(pact.as_ref());
      let index = input.index.unwrap_or(0);
      let (description, request, response) = interactions.get(index).ok_or_else(|| {
        format!(
          "no HTTP interaction at index {index} (pact has {})",
          interactions.len()
        )
      })?;
      let legacy_request = legacy_pact::legacy_request(request)?;
      let legacy_response = legacy_pact::legacy_response(response)?;
      let plan = plan::compile_legacy_interaction(description, &legacy_request, &legacy_response);
      Ok((plan, description.clone()))
    }
    other => Err(format!("unknown input kind '{other}'")),
  }
}

/// `"dim.id#facet=point;dim.id2#facet2=point2"` (design 2.3's variant id) to an [`Assignment`].
fn parse_variant(variant: &str) -> Assignment {
  variant
    .split(';')
    .filter_map(|pair| pair.split_once('='))
    .map(|(dim, point)| (dim.to_string(), point.to_string()))
    .collect()
}

fn check_result(status: Status, mismatches: &[Mismatch], expected: &ExpectedResult) -> Result<(), String> {
  let actual_status = match status {
    Status::Matched => "matched",
    Status::Mismatched => "mismatched",
  };
  if actual_status != expected.status {
    return Err(format!(
      "expected status '{}', got '{actual_status}' ({} mismatch(es): {mismatches:?})",
      expected.status,
      mismatches.len()
    ));
  }
  let actual: Vec<ExpectedMismatch> = mismatches
    .iter()
    .map(|m| ExpectedMismatch {
      path: m.path.clone().unwrap_or_default(),
      message: m.message.clone(),
      action: m.action.clone(),
    })
    .collect();
  if actual != expected.mismatches {
    return Err(format!(
      "mismatches differ from case.json:\n  expected: {:?}\n  actual:   {actual:?}",
      expected.mismatches
    ));
  }
  Ok(())
}

fn check_snapshot(path: &Path, actual: &str, accept: bool, problems: &mut Vec<String>) {
  let existing = std::fs::read_to_string(path).ok();
  if existing.as_deref() == Some(actual) {
    return;
  }
  if accept {
    std::fs::write(path, actual).unwrap_or_else(|err| panic!("writing {}: {err}", path.display()));
    return;
  }
  match existing {
    Some(_) => problems.push(format!(
      "{} differs from its checked-in snapshot — regenerate with `cargo run -p pact_janus_corpus -- accept` and review the diff",
      path.display()
    )),
    None => problems.push(format!(
      "{} is missing — generate it with `cargo run -p pact_janus_corpus -- accept`",
      path.display()
    )),
  }
}
