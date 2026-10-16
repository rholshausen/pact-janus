//! `thinness` — the SDK thinness audit (plan task 6.5). See `cli.rs` for the command.

use std::process::ExitCode;

fn main() -> ExitCode {
  let arguments: Vec<String> = std::env::args().skip(1).collect();
  pact_janus_thinness::cli::run(&arguments)
}
