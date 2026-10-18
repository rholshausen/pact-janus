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

use crate::Outcome;
use crate::args::{Output, UpgradeArgs};
use crate::engine;
use crate::error::{self, Error};
use crate::io;
use clap::error::ErrorKind;
use serde_json::{Value, json};

pub fn run(args: &UpgradeArgs) -> Result<Outcome, Error> {
  // `--output json` puts the contract and its findings in one document on stdout, so there is no
  // separate contract to write and no findings to silence.
  if args.output == Output::Json {
    for (given, flag) in [(args.write.is_some(), "--write <FILE>"), (args.quiet, "--quiet")] {
      if given {
        return Err(error::usage(
          &["upgrade"],
          ErrorKind::ArgumentConflict,
          format!("the argument '--output json' cannot be used with '{flag}'"),
        ));
      }
    }
  }

  let pact = io::read_json(&args.pact)?;
  let mut engine = engine::start()?;
  let result = engine::call(&mut engine, "upgrade/pact", json!({ "pact": pact }))
    .map_err(|err| format!("{}: {err}", args.pact.display()))?;

  if args.output == Output::Json {
    outln!("{}", crate::output::pretty(&result));
    return Ok(Outcome::Done);
  }

  // Canonical bytes, whichever way they leave: a contract's identity is its bytes, and a writer
  // that pretty-printed to a terminal and compacted to a file would be two writers.
  let contract = canonical(&result["contract"])?;
  match &args.write {
    Some(path) => {
      std::fs::write(path, &contract).map_err(|err| format!("writing {}: {err}", path.display()))?;
      errln!("janus upgrade: wrote {}", path.display());
    }
    None => crate::output::out_bytes(&contract),
  }

  if !args.quiet {
    report(&result["findings"]);
  }
  Ok(Outcome::Done)
}

fn canonical(contract: &Value) -> Result<Vec<u8>, Error> {
  let contract: pact_janus_kernel::contract::Contract =
    serde_json::from_value(contract.clone()).map_err(|err| format!("the engine's contract: {err}"))?;
  pact_janus_kernel::contract::write_canonical(&contract)
    .map_err(|err| Error::Failed(format!("writing the contract: {err}")))
}

/// Findings, grouped by what they mean rather than listed flat: `lossy` is the contract now saying
/// less than the pact did, `judgement` is the converter choosing between readings the pact did not
/// distinguish, and `note` is neither. Three things that would otherwise be read as one severity.
fn report(findings: &Value) {
  let findings = findings.as_array().cloned().unwrap_or_default();
  if findings.is_empty() {
    errln!();
    errln!("No findings: this conversion was exact.");
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
    errln!();
    errln!("{heading}");
    for finding in group {
      errln!(
        "  {} at {}",
        finding["code"].as_str().unwrap_or("?"),
        finding["path"].as_str().unwrap_or("?")
      );
      errln!("      {}", finding["message"].as_str().unwrap_or(""));
      if let Some(target) = finding["target"].as_str() {
        errln!("      -> {target} in the contract");
      }
    }
  }
}
