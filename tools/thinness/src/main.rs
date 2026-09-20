//! `thinness` — the SDK thinness audit (plan task 6.5).
//!
//! `thinness` prints the table. `thinness check` prints it and exits non-zero if a hand-written
//! layer is over its budget, or if any source file is unclassified — the form CI runs, so "SDKs are
//! thin" is a command's exit code rather than a number in a report that stopped being true.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use pact_janus_thinness::{Count, LAYERS, Measured, load_manifest, measure};

fn main() -> ExitCode {
  let arguments: Vec<String> = std::env::args().skip(1).collect();
  let checking = match arguments.first().map(String::as_str) {
    None => false,
    Some("check") => true,
    Some(other) => {
      eprintln!("thinness: unknown command '{other}'; usage: thinness [check]");
      return ExitCode::from(2);
    }
  };

  let repo_root = repo_root();
  let sdks = match load_manifest(&repo_root) {
    Ok(sdks) => sdks,
    Err(error) => {
      eprintln!("thinness: {error}");
      return ExitCode::from(2);
    }
  };

  let mut measured = Vec::new();
  for sdk in &sdks {
    match measure(&repo_root, sdk) {
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
        "✗ {}: {} belongs to no layer — add it to a layer in sdks/thinness.json",
        one.sdk.name,
        path.display()
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

/// The repository root: this crate sits at `tools/thinness`, so it is two levels up from the
/// manifest directory. Honours `CARGO_MANIFEST_DIR` so the tool works from any working directory.
fn repo_root() -> PathBuf {
  match std::env::var("CARGO_MANIFEST_DIR") {
    Ok(directory) => Path::new(&directory)
      .parent()
      .and_then(Path::parent)
      .map(Path::to_path_buf)
      .unwrap_or_else(|| PathBuf::from(".")),
    Err(_) => PathBuf::from("."),
  }
}
