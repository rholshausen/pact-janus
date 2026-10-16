//! `conformance` — the SDK conformance suite's checker (plan task 6.4). See `cli.rs` for the command.

use std::process::ExitCode;

fn main() -> ExitCode {
  let arguments: Vec<String> = std::env::args().skip(1).collect();
  pact_janus_conformance::cli::run(&arguments)
}
