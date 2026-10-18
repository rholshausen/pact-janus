//! `janus`: the command line. Everything it does is in the library (`src/lib.rs`).

fn main() -> std::process::ExitCode {
  pact_janus_cli::run(std::env::args_os())
}
