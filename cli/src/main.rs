//! The `janus` CLI (plan task 5.5): `verify`, `explain` and `upgrade`, over the *same engine* every
//! SDK will speak to.
//!
//! Nothing here reaches into the kernel's Rust API to do its work. Each command builds protocol
//! frames and reads frames back ([`engine`]), which is what makes this CLI evidence rather than a
//! convenience: if a command needs something the protocol cannot express, that is a finding about
//! the protocol and it shows up here first. `janus-engine`, the sibling binary, carries the
//! identical frames over stdio for embeddings that want a subprocess (ADR 0003).
//!
//! `check` — the subsumption command (design 2.8) — belongs to Phase 7 and is deliberately absent
//! rather than stubbed: a command that accepted arguments and did nothing would be worse than one
//! that is not there.

mod args;
mod engine;
mod explain;
mod io;
mod upgrade;
mod verify;

use args::{Args, Spec};
use std::process::ExitCode;

const USAGE: &str = "\
usage: janus <command> [options]

  verify    replay a contract or a v1-v4 pact at a running provider
  explain   print the plan the engine will execute for one interaction
  upgrade   convert a v1-v4 pact into a Janus contract
  version   print the engine and protocol versions

`janus <command> --help` explains one of them.

Every command speaks the engine protocol to the same engine an SDK embeds — set
RUST_LOG=debug to watch it.";

fn main() -> ExitCode {
  tracing_subscriber::fmt()
    .with_env_filter(
      tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
    )
    .with_writer(std::io::stderr)
    .init();

  let argv: Vec<String> = std::env::args().skip(1).collect();
  let Some(command) = argv.first().map(String::as_str) else {
    println!("{USAGE}");
    return ExitCode::SUCCESS;
  };

  let (spec, usage): (&Spec, &str) = match command {
    "verify" => (&verify::SPEC, verify::USAGE),
    "explain" => (&explain::SPEC, explain::USAGE),
    "upgrade" => (&upgrade::SPEC, upgrade::USAGE),
    "version" | "--version" => {
      println!(
        "janus {} (engine protocol v{})",
        pact_janus_kernel::ENGINE_VERSION,
        pact_janus_kernel::PROTOCOL_VERSION
      );
      return ExitCode::SUCCESS;
    }
    "help" | "--help" | "-h" => {
      println!("{USAGE}");
      return ExitCode::SUCCESS;
    }
    other => {
      eprintln!("janus: unknown command '{other}'");
      eprintln!();
      eprintln!("{USAGE}");
      return ExitCode::from(io::COMMAND_FAILED);
    }
  };

  let rest = &argv[1..];
  if rest.iter().any(|arg| arg == "--help" || arg == "-h") {
    println!("{usage}");
    return ExitCode::SUCCESS;
  }
  let args = match Args::parse(rest, spec) {
    Ok(args) => args,
    Err(problem) => return io::usage(command, &problem, usage),
  };

  match command {
    "verify" => verify::run(&args),
    "explain" => explain::run(&args),
    "upgrade" => upgrade::run(&args),
    _ => unreachable!("the match above covers every command"),
  }
}
