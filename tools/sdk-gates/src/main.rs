//! `janus-sdk-gates` — the gates an SDK's CI runs, as one binary every release ships for each target
//! triple (staged plan S0.11, ADR 0030), so an SDK runs them from another repository, against the
//! engine version it pins, with no Rust toolchain:
//!
//! - `janus-sdk-gates conformance [--spec <dir>] check <report.json>...` — did the SDK's run of the
//!   conformance suite account for every case, and pass it (plan task 6.4, ADR 0017);
//! - `janus-sdk-gates thinness [check] [<thinness.json or SDK directory>...]` — is every source file
//!   in a layer, and is every hand-written layer inside its budget (plan task 6.5);
//! - `janus-sdk-gates bindings [--spec <dir>] generate --only <language> --sdk <dir>` — regenerate
//!   the SDK's protocol bindings from the schemas, for CI to diff (plan task 6.1).
//!
//! `--spec` is an unpacked `janus-spec.tar.gz` from the same release, or this repository's root. Each
//! subcommand is the same code as the `tools/` crate it names.

use std::process::ExitCode;

const USAGE: &str = "usage: janus-sdk-gates <conformance | thinness | bindings | version> [arguments...]
  conformance [--spec <dir>] [lint | check <report.json>...]
  thinness [check] [<thinness.json or SDK directory>...]
  bindings [--spec <dir>] generate [--only typescript|jvm [--sdk <dir>]]";

fn main() -> ExitCode {
  let arguments: Vec<String> = std::env::args().skip(1).collect();
  let Some((gate, rest)) = arguments.split_first() else {
    eprintln!("{USAGE}");
    return ExitCode::from(2);
  };
  match gate.as_str() {
    "conformance" => pact_janus_conformance::cli::run(rest),
    "thinness" => pact_janus_thinness::cli::run(rest),
    "bindings" => pact_janus_bindings::cli::run(rest),
    "version" | "--version" => {
      println!("janus-sdk-gates {}", env!("CARGO_PKG_VERSION"));
      ExitCode::SUCCESS
    }
    "help" | "--help" | "-h" => {
      println!("{USAGE}");
      ExitCode::SUCCESS
    }
    other => {
      eprintln!("janus-sdk-gates: unknown gate '{other}'\n{USAGE}");
      ExitCode::from(2)
    }
  }
}
