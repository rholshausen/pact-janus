//! The `janus` CLI. Subcommands (verify, upgrade, check) arrive with their phases; `explain` is
//! plan task 3.6's own — a thin invocation of the kernel's compile/execute/render operations
//! (plan task 3.6's actual work), not a hardened command surface. That hardening (real flags, a
//! parser, `--executed` after a *failure* rather than a second file) is plan task 5.5's job.

use pact_janus_kernel::interaction_spec::parse;
use pact_janus_kernel::plan::{Assignment, CapturedValues, compile, execute, render_executed, render_pretty};
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
  eprintln!("usage: janus explain <interaction-spec.json> [--executed <values.json>]");
}

/// `janus explain <spec.json> [--executed <values.json>]` (plan task 3.6): compiles the unpinned
/// shape (design 3.2's document model) and prints the pretty form (plan-grammar spec §3.1); with
/// `--executed`, resolves against a captured-value map (the same shape a golden-corpus case's
/// `values` carries, plan-grammar spec §6.1) and prints the executed form (spec §3.2) instead.
fn explain(args: &[String]) -> ExitCode {
  let Some(spec_path) = args.first() else {
    eprintln!("janus explain: missing <interaction-spec.json>");
    usage();
    return ExitCode::FAILURE;
  };
  let values_path = match args.get(1).map(String::as_str) {
    Some("--executed") => match args.get(2) {
      Some(path) => Some(path.as_str()),
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
    None => None,
  };

  let spec_json = match read_json(spec_path) {
    Ok(v) => v,
    Err(err) => {
      eprintln!("janus explain: reading {spec_path}: {err}");
      return ExitCode::FAILURE;
    }
  };
  let spec = match parse(&spec_json) {
    Ok(spec) => spec,
    Err(err) => {
      eprintln!("janus explain: {spec_path} is not a well-formed interaction specification:");
      for problem in &err.problems {
        eprintln!("  {}: {}", problem.pointer, problem.message);
      }
      return ExitCode::FAILURE;
    }
  };
  let plan = compile(&spec, &Assignment::new(), None);

  match values_path {
    None => {
      println!("{}", render_pretty(&plan));
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
      let executed = execute(&plan, &resolver);
      println!("{}", render_executed(&executed));
    }
  }
  ExitCode::SUCCESS
}

fn read_json(path: &str) -> Result<Value, String> {
  let text = std::fs::read_to_string(path).map_err(|err| err.to_string())?;
  serde_json::from_str(&text).map_err(|err| err.to_string())
}
