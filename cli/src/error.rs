//! Why a command stopped short of its answer: it could not run (exit 2), or it ran and refused its
//! subject with a reason to print (exit 1).

use crate::args::Cli;
use clap::CommandFactory;
use clap::error::ErrorKind;

#[derive(Debug)]
pub enum Error {
  /// The command line asks for something that cannot be done. clap prints it, with the usage of
  /// the (sub)command it is about.
  Usage(clap::Error),
  /// The command could not do its work: a file that cannot be read, an engine that refused. Printed
  /// as `janus <command>: <message>`.
  Failed(String),
  /// The command ran and its subject failed, for a reason worth a line of its own — an artifact
  /// that is not what it claims. Printed like `Failed`, but exits 1: an answer, not an error.
  Refused(String),
}

impl From<String> for Error {
  fn from(message: String) -> Self {
    Error::Failed(message)
  }
}

/// A usage error clap cannot see — two flags it allows separately but not together, depending on a
/// value — reported exactly as clap reports its own, for the subcommand at `path`.
pub fn usage(path: &[&str], kind: ErrorKind, message: impl std::fmt::Display) -> Error {
  let mut command = Cli::command();
  command.build();
  let mut subcommand = &mut command;
  for name in path {
    subcommand = subcommand
      .find_subcommand_mut(name)
      .unwrap_or_else(|| panic!("no subcommand '{name}' in the CLI definition"));
  }
  Error::Usage(subcommand.error(kind, message))
}
