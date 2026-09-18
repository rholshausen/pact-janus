//! `janus explain` (plan tasks 3.6 and 5.5): what the engine will actually do, before it does it.
//!
//! The compiling is `verification/explain`'s — a kernel operation precisely so no SDK builds its
//! own (the RFC), and so what this prints is the plan a run would really execute rather than a
//! second, display-only rendering of the same document.
//!
//! `--executed` has two forms, and the difference is the point of plan task 5.5's hardening. The
//! one that matters is `janus verify --explain-failures`, which prints the executed plan for each
//! variant that actually failed, against the response the provider actually sent. The offline form
//! here — `--executed <values.json>` — resolves a plan against a hand-written map of captured
//! values, which is what a golden-corpus case carries (plan-grammar spec §6.1) and what you reach
//! for when there is no provider to run at all.

use crate::args::{Args, Spec};
use crate::engine;
use crate::io;
use pact_janus_kernel::plan::{self, CapturedValues};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::process::ExitCode;

pub const SPEC: Spec = Spec {
  values: &["index", "variant", "executed"],
  flags: &["spec", "plan"],
};

pub const USAGE: &str = "\
usage: janus explain <document.json> [options]

  <document.json>            an interaction specification, a Janus contract, or a v1-v4 pact
  --index <n>                which interaction of a contract or pact (default: 0)
  --variant <id>             compile pinned to this recorded variant of a Janus contract
  --spec                     read the document as an interaction specification
  --plan                     print the structured plan document instead of the text form
  --executed <values.json>   resolve the plan against a map of captured values and print the
                             executed form. For a real failure, prefer:
                                 janus verify ... --explain-failures";

pub fn run(args: &Args) -> ExitCode {
  let Some(path) = args.positionals().first() else {
    return io::usage("explain", "missing <document.json>", USAGE);
  };
  let document = match io::read_json(path) {
    Ok(document) => document,
    Err(err) => return io::fail(&format!("janus explain: {err}")),
  };

  // The document says what it is (ADR 0011, plan task 5.5): an interaction specification is the
  // one that cannot, so it is the one that takes a flag.
  let subject = if args.flag("spec") {
    json!({ "kind": "spec", "spec": document })
  } else {
    let mut subject = json!({ "kind": "contract-interaction", "contract": document });
    if let Some(index) = args.value("index") {
      match index.parse::<usize>() {
        Ok(index) => subject["index"] = json!(index),
        Err(_) => {
          return io::usage(
            "explain",
            &format!("--index needs an integer, got '{index}'"),
            USAGE,
          );
        }
      }
    }
    if let Some(variant) = args.value("variant") {
      subject["variant"] = json!(variant);
    }
    subject
  };

  let mut engine = match engine::start() {
    Ok(engine) => engine,
    Err(err) => return io::fail(&format!("janus explain: {err}")),
  };
  let want_plan = args.flag("plan") || args.value("executed").is_some();
  let result = engine::call(
    &mut engine,
    "verification/explain",
    json!({ "interaction": subject, "options": { "plan": want_plan } }),
  );
  let result = match result {
    Ok(result) => result,
    Err(err) => {
      // A document with no `$format` that is also not a pact reaches the engine as a
      // contract-interaction and is refused there. Saying so beats a bare parse error.
      eprintln!("janus explain: {err}");
      if !args.flag("spec") {
        eprintln!("  (if {path} is an interaction specification, pass --spec)");
      }
      return ExitCode::from(io::COMMAND_FAILED);
    }
  };

  match args.value("executed") {
    None if args.flag("plan") => {
      println!(
        "{}",
        serde_json::to_string_pretty(&result["plan"]).unwrap_or_default()
      );
    }
    None => println!("{}", result["text"].as_str().unwrap_or_default()),
    Some(values_path) => return executed(&result["plan"], values_path),
  }
  ExitCode::SUCCESS
}

/// The offline executed form: the plan the engine just compiled, run against captured values.
///
/// The plan is re-read from the engine's own structured document rather than recompiled here —
/// same compiler, same plan, one source of truth — and executed by the kernel's interpreter.
fn executed(document: &Value, values_path: &str) -> ExitCode {
  let values_json = match io::read_json(values_path) {
    Ok(values) => values,
    Err(err) => return io::fail(&format!("janus explain: {err}")),
  };
  let Value::Object(map) = values_json else {
    return io::fail(&format!(
      "janus explain: {values_path} must be a JSON object of path -> value"
    ));
  };
  let values: BTreeMap<String, Value> = map.into_iter().collect();
  let plan = match plan::from_json(document) {
    Ok(plan) => plan,
    Err(err) => return io::fail(&format!("janus explain: the engine's plan document: {err}")),
  };
  let executed = plan::execute(&plan, &CapturedValues::from_json(&values));
  println!("{}", plan::render_executed(&executed));
  match plan::outcome(&executed).0 {
    plan::Status::Matched => ExitCode::SUCCESS,
    plan::Status::Mismatched => ExitCode::from(io::SUBJECT_FAILED),
  }
}
