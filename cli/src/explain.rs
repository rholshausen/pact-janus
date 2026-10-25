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
//! for when there is no provider to run at all. The engine runs it too, in the same call that
//! compiles the plan (staged plan S1.12, ADR 0050), so nothing here steps outside the protocol.
//!
//! `--config` names the verifier configuration whose `components` a run would load, so a plan a
//! declared component contributes a fragment to is explained as a run compiles it.

use crate::Outcome;
use crate::args::ExplainArgs;
use crate::engine;
use crate::error::Error;
use crate::highlight;
use crate::io;
use serde_json::json;

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

  let mut request = json!({ "interaction": subject, "options": { "plan": args.plan } });
  if let Some(path) = &args.config {
    let config = pact_janus_hooks_host::loader::load(path)
      .map_err(|err| format!("reading {}: {err}", path.display()))?;
    if let Some(components) = config.get("components") {
      request["components"] = components.clone();
    }
  }
  if let Some(path) = &args.executed {
    let values = io::read_json(path)?;
    if !values.is_object() {
      return Err(Error::Failed(format!(
        "{} must be a JSON object of path -> value",
        path.display()
      )));
    }
    request["values"] = values;
  }

  let mut engine = engine::start()?;
  let result = engine::call(&mut engine, "verification/explain", request).map_err(|err| {
    // A document with no `$format` that is also not a pact reaches the engine as a
    // contract-interaction and is refused there. Saying so beats a bare parse error. A missing
    // component is not a parse error: the document was read.
    if args.spec || err.starts_with("component-unavailable") {
      err
    } else {
      format!(
        "{err}\n  (if {} is an interaction specification, pass --spec)",
        args.document.display()
      )
    }
  })?;

  match &args.executed {
    Some(_) => {
      let executed = &result["executed"];
      outln_styled!(
        "{}",
        highlight::plan(executed["text"].as_str().unwrap_or_default())
      );
      Ok(match executed["status"].as_str() {
        Some("matched") => Outcome::Done,
        _ => Outcome::SubjectFailed,
      })
    }
    None if args.plan => {
      outln!("{}", crate::output::pretty(&result["plan"]));
      Ok(Outcome::Done)
    }
    None => {
      outln_styled!("{}", highlight::plan(result["text"].as_str().unwrap_or_default()));
      Ok(Outcome::Done)
    }
  }
}
