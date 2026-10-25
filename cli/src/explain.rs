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
use crate::error::{self, Error};
use crate::highlight;
use crate::io;
use clap::error::ErrorKind;
use serde_json::{Value, json};

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

  if let Some(index) = args.index {
    check_index(&document, index)?;
  }

  let mut engine = engine::start()?;
  let result = engine::call(&mut engine, "verification/explain", request).map_err(|err| {
    // A document with no `$format` that is also not a pact reaches the engine as a
    // contract-interaction and is refused there, as one it cannot identify. Saying which flag reads
    // it as a specification beats the refusal alone; any other error is about a document that was
    // identified, and the hint would mislead.
    let unidentified = ["contract-invalid", "contract-version-unsupported"]
      .iter()
      .any(|code| err.starts_with(&format!("{code}:")));
    if args.spec || !unidentified {
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

/// `--index` past the document's last interaction is the command line asking for something that is
/// not there: a usage error naming the flag, before the engine is asked. A contract's interactions,
/// a v4 pact's, and a v3 message pact's messages are each counted where the document keeps them; a
/// document with neither is left for the engine to refuse by what it is.
fn check_index(document: &Value, index: usize) -> Result<(), Error> {
  let Some(count) = ["interactions", "messages"]
    .iter()
    .find_map(|member| document[member].as_array().map(Vec::len))
  else {
    return Ok(());
  };
  if index < count {
    return Ok(());
  }
  let has = match count {
    0 => "the document has no interactions".to_string(),
    1 => "the document has one interaction, at 0".to_string(),
    _ => format!("the document has {count} interactions, at 0 to {}", count - 1),
  };
  Err(error::usage(
    &["explain"],
    ErrorKind::ValueValidation,
    format!("invalid value '{index}' for '--index <N>': {has}"),
  ))
}
