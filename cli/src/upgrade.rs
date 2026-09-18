//! `janus upgrade` (plan task 5.5, contract-file spec §8): a v1–v4 pact in, a Janus contract out —
//! and the findings that say what the conversion could not carry.
//!
//! **The findings are half the output, not a diagnostic afterthought.** Conversion is not required
//! to be lossless (spec §8.1); it is required to be honest. So this command prints them by default
//! and says plainly when there are none, because "no findings" is a claim worth making explicitly:
//! the conversion was exact.
//!
//! The contract is written in its canonical bytes (ADR 0018): compact, `$format` first. That is
//! not a display choice — deterministic bytes are what keep broker deduplication and git diffs
//! honest. Anything that wants it readable can pipe it through `jq`.

use crate::args::{Args, Spec};
use crate::engine;
use crate::io;
use serde_json::{Value, json};
use std::process::ExitCode;

pub const SPEC: Spec = Spec {
  values: &["out"],
  flags: &["quiet", "json"],
};

pub const USAGE: &str = "\
usage: janus upgrade <pact.json> [options]

  <pact.json>                a v1-v4 pact to convert
  --out <file>               write the contract here (default: stdout)
  --json                     print {contract, findings} as one JSON document
  --quiet                    do not print findings

A converted pact is honestly under-covered: it has one variant, because one example is
one variant. Growing it is the consumer suite's job — and the reason to upgrade at all.
Verifying a pact needs no conversion: `janus verify` reads v1-v4 pacts directly.";

pub fn run(args: &Args) -> ExitCode {
  let Some(path) = args.positionals().first() else {
    return io::usage("upgrade", "missing <pact.json>", USAGE);
  };
  let pact = match io::read_json(path) {
    Ok(pact) => pact,
    Err(err) => return io::fail(&format!("janus upgrade: {err}")),
  };

  let mut engine = match engine::start() {
    Ok(engine) => engine,
    Err(err) => return io::fail(&format!("janus upgrade: {err}")),
  };
  let result = match engine::call(&mut engine, "upgrade/pact", json!({ "pact": pact })) {
    Ok(result) => result,
    Err(err) => return io::fail(&format!("janus upgrade: {path}: {err}")),
  };

  if args.flag("json") {
    println!("{}", serde_json::to_string_pretty(&result).unwrap_or_default());
    return ExitCode::SUCCESS;
  }

  // Canonical bytes, whichever way they leave: a contract's identity is its bytes, and a writer
  // that pretty-printed to a terminal and compacted to a file would be two writers.
  let contract = match canonical(&result["contract"]) {
    Ok(bytes) => bytes,
    Err(err) => return io::fail(&format!("janus upgrade: {err}")),
  };
  match args.value("out") {
    Some(out) => {
      if let Err(err) = std::fs::write(out, &contract) {
        return io::fail(&format!("janus upgrade: writing {out}: {err}"));
      }
      eprintln!("janus upgrade: wrote {out}");
    }
    None => {
      use std::io::Write;
      let _ = std::io::stdout().write_all(&contract);
    }
  }

  if !args.flag("quiet") {
    report(&result["findings"]);
  }
  ExitCode::SUCCESS
}

fn canonical(contract: &Value) -> Result<Vec<u8>, String> {
  let contract: pact_janus_kernel::contract::Contract =
    serde_json::from_value(contract.clone()).map_err(|err| err.to_string())?;
  pact_janus_kernel::contract::write_canonical(&contract).map_err(|err| err.to_string())
}

/// Findings, grouped by what they mean rather than listed flat: `lossy` is the contract now saying
/// less than the pact did, `judgement` is the converter choosing between readings the pact did not
/// distinguish, and `note` is neither. Three things that would otherwise be read as one severity.
fn report(findings: &Value) {
  let findings = findings.as_array().cloned().unwrap_or_default();
  if findings.is_empty() {
    eprintln!();
    eprintln!("No findings: this conversion was exact.");
    return;
  }
  for (kind, heading) in [
    ("lossy", "LOSSY — the contract now says less than the pact did"),
    (
      "judgement",
      "JUDGEMENT — the converter chose between readings the pact did not distinguish",
    ),
    ("note", "NOTE — worth reading"),
  ] {
    let group: Vec<&Value> = findings.iter().filter(|f| f["kind"] == json!(kind)).collect();
    if group.is_empty() {
      continue;
    }
    eprintln!();
    eprintln!("{heading}");
    for finding in group {
      eprintln!(
        "  {} at {}",
        finding["code"].as_str().unwrap_or("?"),
        finding["path"].as_str().unwrap_or("?")
      );
      eprintln!("      {}", finding["message"].as_str().unwrap_or(""));
      if let Some(target) = finding["target"].as_str() {
        eprintln!("      -> {target} in the contract");
      }
    }
  }
}
