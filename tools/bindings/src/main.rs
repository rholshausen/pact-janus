//! `janus-bindings` — the binding-generation pipeline (plan task 6.1). See `cli.rs` for the command.

use std::process::ExitCode;

fn main() -> ExitCode {
  let arguments: Vec<String> = std::env::args().skip(1).collect();
  pact_janus_bindings::cli::run(&arguments)
}
