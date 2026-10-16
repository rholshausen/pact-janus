//! `thinness [check] [<thinness.json or SDK directory>...]` — the audit as a command, shared by this
//! crate's own binary and `janus-sdk-gates thinness` (staged plan S0.11).
//!
//! With no manifest named it measures every `sdks/*/thinness.json` under the working directory —
//! this repository, run from its root — or else the `thinness.json` in it, which is an SDK's own
//! checkout. `check` exits non-zero if a hand-written layer is over its budget, or if any source file
//! is unclassified — the form CI runs, so "SDKs are thin" is a command's exit code rather than a
//! number in a report that stopped being true.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crate::{Count, LAYERS, MANIFEST, Measured, load_manifest, measure};

const USAGE: &str = "usage: thinness [check] [<thinness.json or SDK directory>...]";

/// Runs the command on its arguments (the program name excluded).
pub fn run(arguments: &[String]) -> ExitCode {
  let (checking, named) = match arguments.split_first() {
    Some((first, rest)) if first == "check" => (true, rest),
    _ => (false, arguments),
  };
  if let Some(flag) = named.iter().find(|a| a.starts_with('-')) {
    eprintln!("thinness: unknown option '{flag}'; {USAGE}");
    return ExitCode::from(2);
  }
  let manifests = match manifests(named) {
    Ok(manifests) => manifests,
    Err(error) => {
      eprintln!("thinness: {error}");
      return ExitCode::from(2);
    }
  };

  let mut measured = Vec::new();
  for manifest in &manifests {
    match load_manifest(manifest).and_then(|sdk| measure(&sdk)) {
      Ok(one) => measured.push(one),
      Err(error) => {
        eprintln!("thinness: {error}");
        return ExitCode::from(2);
      }
    }
  }

  for one in &measured {
    report(one);
  }
  comparison(&measured);

  if !checking {
    return ExitCode::SUCCESS;
  }

  let mut failed = false;
  for one in &measured {
    for path in &one.unclassified {
      println!(
        "✗ {}: {} belongs to no layer — add it to a layer in {}",
        one.sdk.name,
        path.display(),
        one.sdk.root.join(MANIFEST).display()
      );
      failed = true;
    }
    for (layer, actual, budget) in one.over_budget() {
      println!(
        "✗ {}: {layer} is {actual} code lines, over its budget of {budget}. Either the layer grew \
         something that belongs in the engine or the protocol, or the budget is wrong now — say \
         which in the commit that changes it.",
        one.sdk.name
      );
      failed = true;
    }
  }

  if failed {
    ExitCode::FAILURE
  } else {
    println!("\n✓ thinness: every source file is classified, and every budgeted layer is inside its budget");
    ExitCode::SUCCESS
  }
}

/// The manifests named — a directory means the `thinness.json` in it — or, with none named, every
/// `sdks/*/thinness.json` under the working directory, or else its own `thinness.json`.
fn manifests(named: &[String]) -> Result<Vec<PathBuf>, String> {
  if !named.is_empty() {
    return Ok(
      named
        .iter()
        .map(PathBuf::from)
        .map(|path| if path.is_dir() { path.join(MANIFEST) } else { path })
        .collect(),
    );
  }
  let mut found = Vec::new();
  if let Ok(entries) = std::fs::read_dir("sdks") {
    for entry in entries.flatten() {
      let manifest = entry.path().join(MANIFEST);
      if manifest.is_file() {
        found.push(manifest);
      }
    }
  }
  found.sort();
  if found.is_empty() && Path::new(MANIFEST).is_file() {
    found.push(PathBuf::from(MANIFEST));
  }
  if found.is_empty() {
    return Err(format!(
      "no sdks/*/{MANIFEST} and no {MANIFEST} in the working directory; name one. {USAGE}"
    ));
  }
  Ok(found)
}

fn report(one: &Measured) {
  println!("\n{} ({})", one.sdk.name, one.sdk.language);
  println!(
    "  {:<14} {:>6} {:>8} {:>8}   budget",
    "layer", "files", "lines", "code"
  );
  let mut total = Count::default();
  for layer in LAYERS {
    let count = one.by_layer.get(layer).copied().unwrap_or_default();
    total.add(count);
    let budget = match one.sdk.budgets.get(layer) {
      Some(budget) => format!("{} code lines", budget),
      None => "—".to_string(),
    };
    println!(
      "  {:<14} {:>6} {:>8} {:>8}   {}",
      layer, count.files, count.lines, count.code, budget
    );
  }
  println!(
    "  {:<14} {:>6} {:>8} {:>8}",
    "total", total.files, total.lines, total.code
  );

  let hand = one.hand_written();
  let shipped = total.code - one.by_layer.get("tests").copied().unwrap_or_default().code;
  if shipped > 0 {
    println!(
      "  hand-written: {} of {} shipped code lines ({:.0}%) — the rest is generated",
      hand.code,
      shipped,
      100.0 * hand.code as f64 / shipped as f64
    );
  }
}

/// The comparison the audit exists for: how much of what ships is hand-written, per language.
fn comparison(measured: &[Measured]) {
  println!("\nthinness, side by side (shipped code lines — 'tests' excluded)");
  println!(
    "  {:<24} {:>10} {:>12} {:>14}",
    "sdk", "generated", "hand-written", "hand-written %"
  );
  for one in measured {
    let generated = one.by_layer.get("generated").copied().unwrap_or_default().code;
    let hand = one.hand_written().code;
    let shipped = generated + hand;
    let percent = if shipped == 0 {
      0.0
    } else {
      100.0 * hand as f64 / shipped as f64
    };
    println!(
      "  {:<24} {:>10} {:>12} {:>13.0}%",
      one.sdk.name, generated, hand, percent
    );
  }
}
