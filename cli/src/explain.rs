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

use crate::Outcome;
use crate::args::ExplainArgs;
use crate::engine;
use crate::error::Error;
use crate::io;
use pact_janus_kernel::plan::{self, CapturedValues};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::Path;

pub fn run(args: &ExplainArgs) -> Result<Outcome, Error> {
  let document = io::read_json(&args.document)?;

  // The document says what it is (ADR 0011, plan task 5.5): an interaction specification is the
  // one that cannot, so it is the one that takes a flag.
  let subject = if args.spec {
    json!({ "kind": "spec", "spec": document })
  } else {
    let mut subject = json!({ "kind": "contract-interaction", "contract": document });
    if let Some(index) = args.index {
      subject["index"] = json!(index);
    }
    if let Some(variant) = &args.variant {
      subject["variant"] = json!(variant);
    }
    subject
  };

  let mut engine = engine::start()?;
  let want_plan = args.plan || args.executed.is_some();
  let result = engine::call(
    &mut engine,
    "verification/explain",
    json!({ "interaction": subject, "options": { "plan": want_plan } }),
  )
  .map_err(|err| {
    // A document with no `$format` that is also not a pact reaches the engine as a
    // contract-interaction and is refused there. Saying so beats a bare parse error.
    if args.spec {
      err
    } else {
      format!(
        "{err}\n  (if {} is an interaction specification, pass --spec)",
        args.document.display()
      )
    }
  })?;

  match &args.executed {
    Some(values) => executed(&result["plan"], values),
    None if args.plan => {
      outln!("{}", crate::output::pretty(&result["plan"]));
      Ok(Outcome::Done)
    }
    None => {
      outln!("{}", result["text"].as_str().unwrap_or_default());
      Ok(Outcome::Done)
    }
  }
}

/// The offline executed form: the plan the engine just compiled, run against captured values.
///
/// The plan is re-read from the engine's own structured document rather than recompiled here —
/// same compiler, same plan, one source of truth — and executed by the kernel's interpreter.
fn executed(document: &Value, values_path: &Path) -> Result<Outcome, Error> {
  let Value::Object(map) = io::read_json(values_path)? else {
    return Err(Error::Failed(format!(
      "{} must be a JSON object of path -> value",
      values_path.display()
    )));
  };
  let values: BTreeMap<String, Value> = map.into_iter().collect();
  let plan = plan::from_json(document).map_err(|err| format!("the engine's plan document: {err}"))?;
  let executed = plan::execute(&plan, &CapturedValues::from_json(&values));
  outln!("{}", plan::render_executed(&executed));
  Ok(match plan::outcome(&executed).0 {
    plan::Status::Matched => Outcome::Done,
    plan::Status::Mismatched => Outcome::SubjectFailed,
  })
}
