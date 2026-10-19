//! The `janus` CLI: `verify`, `check`, `explain`, `upgrade`, `publish` and `component`, over the
//! *same engine* every SDK speaks to.
//!
//! Nothing here reaches into the kernel's Rust API to do its work. Each command builds protocol
//! frames and reads frames back ([`engine`]), which is what makes this CLI evidence rather than a
//! convenience: if a command needs something the protocol cannot express, that is a finding about
//! the protocol and it shows up here first. `janus-engine`, the sibling binary, carries the
//! identical frames over stdio for embeddings that want a subprocess (ADR 0023: every SDK's).
//!
//! `component` is the one command that does not speak the protocol, and says why in its own module:
//! publishing a component is not an engine operation.
//!
//! `check` is the one command whose subject is not a provider but a *decision*: it reads documents
//! that already exist — contracts, provider shapes, the summaries `verify` wrote — and answers the
//! RFC's `can-i-deploy` question over them. Same rule as the rest: the walk, the policy and the page
//! all live behind `subsumption/check` and `subsumption/decide` (engine-protocol spec §8.6), because
//! a broker or a GitHub Action asking the same question has to get the same answer.
//!
//! Exit codes are part of the surface (see [`Outcome`] and [`error::Error`]): `0` the command did
//! what it was asked, `1` the *subject* failed — an answer, not an error — and `2` the command could
//! not run, which includes every usage error clap reports.

#[macro_use]
mod output;

mod args;
mod check;
mod component;
mod date;
mod engine;
mod error;
mod explain;
pub mod framing;
mod io;
pub mod logging;
mod publish;
pub mod register;
mod upgrade;
mod verify;

use args::{Cli, Command, LogLevel};
use clap::{CommandFactory, FromArgMatches};
use error::Error;
use std::ffi::OsString;
use std::process::ExitCode;
use std::sync::LazyLock;

/// What a command that ran concluded about its subject.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
  /// The command did what it was asked, and the answer was yes: exit 0.
  Done,
  /// The command ran, and its subject failed — a verification found mismatches, a check blocked, an
  /// artifact is not what it claims: exit 1. Protocol spec §10.2 draws the same line: "a
  /// verification that ran and found mismatches is a successful operation".
  SubjectFailed,
}

pub const SUBJECT_FAILED: u8 = 1;
pub const COMMAND_FAILED: u8 = 2;

/// `0.1.0 (engine protocol v1)`: what `janus version` and `janus --version` both print after the
/// binary's name. The release workflow's smoke test reads it, so its shape is fixed.
pub static VERSION: LazyLock<String> = LazyLock::new(|| {
  format!(
    "{} (engine protocol v{})",
    pact_janus_kernel::ENGINE_VERSION,
    pact_janus_kernel::PROTOCOL_VERSION
  )
});

/// The whole CLI: parse `argv` (program name first), run the command, and say how it went.
pub fn run<I, T>(argv: I) -> ExitCode
where
  I: IntoIterator<Item = T>,
  T: Into<OsString> + Clone,
{
  let matches = match Cli::command()
    .version(VERSION.as_str())
    .try_get_matches_from(argv)
  {
    Ok(matches) => matches,
    Err(err) => return clap_exit(&err),
  };
  let cli = match Cli::from_arg_matches(&matches) {
    Ok(cli) => cli,
    Err(err) => return clap_exit(&err),
  };
  init_tracing(cli.log_level);

  let name = cli.command.name();
  match dispatch(cli.command) {
    Ok(Outcome::Done) => ExitCode::SUCCESS,
    Ok(Outcome::SubjectFailed) => ExitCode::from(SUBJECT_FAILED),
    Err(Error::Usage(err)) => clap_exit(&err),
    Err(Error::Failed(message)) => {
      errln!("janus {name}: {message}");
      ExitCode::from(COMMAND_FAILED)
    }
    Err(Error::Refused(message)) => {
      errln!("janus {name}: {message}");
      ExitCode::from(SUBJECT_FAILED)
    }
  }
}

fn dispatch(command: Command) -> Result<Outcome, Error> {
  match command {
    Command::Verify(args) => verify::run(&args),
    Command::Check(args) => check::run(&args),
    Command::Explain(args) => explain::run(&args),
    Command::Upgrade(args) => upgrade::run(&args),
    Command::Publish(args) => publish::run(&args),
    Command::Component(command) => component::run(&command),
    Command::Completions(args) => {
      let mut command = Cli::command();
      let mut script = Vec::new();
      clap_complete::generate(args.shell, &mut command, "janus", &mut script);
      output::out_bytes(&script);
      Ok(Outcome::Done)
    }
    Command::Version => {
      outln!("janus {}", VERSION.as_str());
      Ok(Outcome::Done)
    }
  }
}

/// clap's own report — help and `--version` on stdout with exit 0, a usage error on stderr with
/// exit 2 — printed the way clap prints it, and never a panic if the stream is gone.
fn clap_exit(err: &clap::Error) -> ExitCode {
  let _ = err.print();
  ExitCode::from(u8::try_from(err.exit_code()).unwrap_or(COMMAND_FAILED))
}

/// `--log-level` wins; without it `RUST_LOG` is read, so the finer directives a maintainer reaches
/// for still work; the default is `warn`.
fn init_tracing(level: Option<LogLevel>) {
  use tracing_subscriber::EnvFilter;
  logging::to_stderr(match level {
    Some(level) => EnvFilter::new(level.as_str()),
    None => EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn")),
  });
}
