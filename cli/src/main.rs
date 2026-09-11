//! The `janus` CLI. Subcommands (verify, upgrade, check) arrive with their phases; `explain` is
//! plan task 3.6's own — a thin invocation of the kernel's compile/execute/render operations
//! (plan task 3.6's actual work), not a hardened command surface. That hardening (real flags, a
//! parser, `--executed` after a *failure* rather than a second file) is plan task 5.5's job.
//!
//! `explain` reads either document `janus explain` is asked to compile: a new-style interaction
//! spec (design 3.2) or a v1–v4 pact (design 3.1, compiled via design 3.5's legacy compiler) —
//! milestone M1's own bar ("CLI compiles and `explain`s plans for both a v4 pact and a new-style
//! interaction spec"). Detection is a top-level `interactions` key, the one member every pact has
//! and no interaction spec does; a real format identifier (ADR 0011's `$format`, ADR-equivalent for
//! pacts) is a task 5.5 concern, not this skeleton's.

use pact_janus_kernel::interaction_spec;
use pact_janus_kernel::legacy_pact;
use pact_janus_kernel::plan::{self, Assignment, CapturedValues};
use serde_json::Value;
use std::collections::BTreeMap;
use std::process::ExitCode;

fn main() -> ExitCode {
  let args: Vec<String> = std::env::args().skip(1).collect();
  match args.first().map(String::as_str) {
    Some("explain") => explain(&args[1..]),
    Some(other) => {
      eprintln!("janus: unknown command '{other}'");
      usage();
      ExitCode::FAILURE
    }
    None => {
      println!(
        "janus {} (engine protocol v{})",
        pact_janus_kernel::ENGINE_VERSION,
        pact_janus_kernel::PROTOCOL_VERSION
      );
      ExitCode::SUCCESS
    }
  }
}

fn usage() {
  eprintln!(
    "usage: janus explain <interaction-spec.json | pact.json> [--index N] [--executed <values.json>]"
  );
}

/// `janus explain <doc.json> [--index N] [--executed <values.json>]` (plan task 3.6): compiles the
/// document — unpinned, for a shape-based interaction spec; the interaction at `--index` (default
/// 0), for a v1–v4 pact — and prints the pretty form (plan-grammar spec §3.1). With `--executed`,
/// resolves against a captured-value map (the same shape a golden-corpus case's `values` carries,
/// plan-grammar spec §6.1) and prints the executed form (spec §3.2) instead.
fn explain(args: &[String]) -> ExitCode {
  let Some(doc_path) = args.first() else {
    eprintln!("janus explain: missing <interaction-spec.json | pact.json>");
    usage();
    return ExitCode::FAILURE;
  };

  let mut index = 0usize;
  let mut values_path: Option<&str> = None;
  let mut rest = &args[1..];
  loop {
    match rest.first().map(String::as_str) {
      Some("--index") => match rest.get(1) {
        Some(n) => match n.parse() {
          Ok(n) => {
            index = n;
            rest = &rest[2..];
          }
          Err(_) => {
            eprintln!("janus explain: --index needs an integer, got '{n}'");
            usage();
            return ExitCode::FAILURE;
          }
        },
        None => {
          eprintln!("janus explain: --index needs a value");
          usage();
          return ExitCode::FAILURE;
        }
      },
      Some("--executed") => match rest.get(1) {
        Some(path) => {
          values_path = Some(path.as_str());
          rest = &rest[2..];
        }
        None => {
          eprintln!("janus explain: --executed needs a <values.json> path");
          usage();
          return ExitCode::FAILURE;
        }
      },
      Some(other) => {
        eprintln!("janus explain: unknown option '{other}'");
        usage();
        return ExitCode::FAILURE;
      }
      None => break,
    }
  }

  let doc_json = match read_json(doc_path) {
    Ok(v) => v,
    Err(err) => {
      eprintln!("janus explain: reading {doc_path}: {err}");
      return ExitCode::FAILURE;
    }
  };

  let plan = if doc_json.get("interactions").is_some() {
    match compile_pact(doc_path, &doc_json, index) {
      Ok(plan) => plan,
      Err(err) => {
        eprintln!("janus explain: {err}");
        return ExitCode::FAILURE;
      }
    }
  } else {
    let spec = match interaction_spec::parse(&doc_json) {
      Ok(spec) => spec,
      Err(err) => {
        eprintln!("janus explain: {doc_path} is not a well-formed interaction specification:");
        for problem in &err.problems {
          eprintln!("  {}: {}", problem.pointer, problem.message);
        }
        return ExitCode::FAILURE;
      }
    };
    plan::compile(&spec, &Assignment::new(), None)
  };

  match values_path {
    None => {
      println!("{}", plan::render_pretty(&plan));
    }
    Some(values_path) => {
      let values_json = match read_json(values_path) {
        Ok(v) => v,
        Err(err) => {
          eprintln!("janus explain: reading {values_path}: {err}");
          return ExitCode::FAILURE;
        }
      };
      let values: BTreeMap<String, Value> = match values_json {
        Value::Object(map) => map.into_iter().collect(),
        other => {
          eprintln!("janus explain: {values_path} must be a JSON object of path -> value, got {other}");
          return ExitCode::FAILURE;
        }
      };
      let resolver = CapturedValues::from_json(&values);
      let executed = plan::execute(&plan, &resolver);
      println!("{}", plan::render_executed(&executed));
    }
  }
  ExitCode::SUCCESS
}

/// A v1–v4 pact to a plan (design 3.1's reading, design 3.5's compiler): the interaction at
/// `index`, request and response together (`plan::compile_legacy_interaction`).
fn compile_pact(doc_path: &str, doc_json: &Value, index: usize) -> Result<plan::Plan, String> {
  let pact = legacy_pact::read(doc_path, doc_json).map_err(|err| err.to_string())?;
  let interactions = legacy_pact::http_interactions(pact.as_ref());
  let (description, request, response) = interactions.get(index).ok_or_else(|| {
    format!(
      "no HTTP interaction at index {index} ({} has {})",
      doc_path,
      interactions.len()
    )
  })?;
  let legacy_request = legacy_pact::legacy_request(request)?;
  let legacy_response = legacy_pact::legacy_response(response)?;
  Ok(plan::compile_legacy_interaction(
    description,
    &legacy_request,
    &legacy_response,
  ))
}

fn read_json(path: &str) -> Result<Value, String> {
  let text = std::fs::read_to_string(path).map_err(|err| err.to_string())?;
  serde_json::from_str(&text).map_err(|err| err.to_string())
}
