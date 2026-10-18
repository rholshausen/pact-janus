//! The command line, as clap reads it.
//!
//! Everything a user can type is declared here and nowhere else, so `janus --help` and the code
//! cannot disagree. The `///` comments on fields and variants are the help text clap prints: write
//! them for someone running the command, and put notes for maintainers in `//` comments instead.
//!
//! The flag spellings are a surface a CI script depends on, and stage 1a's flag-parity table
//! (staged plan S1.13) maps today's verifier options onto them, so a rename here is a breaking
//! change that needs a change file saying so. `--output text|json` is the spelling the Pact Broker
//! client already uses; it leaves `--json <file>` free for parity with `pact_verifier_cli`.

use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

// `bin_name` is pinned so help and usage errors say `janus` however the binary was invoked: clap
// otherwise takes the name from argv[0], which on Windows is `janus.exe`.
#[derive(Debug, Parser)]
#[command(
  name = "janus",
  bin_name = "janus",
  about = "Verify contracts and pacts against a provider, explain matching plans, upgrade pacts, \
           and decide whether a consumer can deploy",
  after_help = "Every command drives the same engine an SDK embeds. --log-level debug shows the \
                protocol frames it exchanges.",
  arg_required_else_help = true
)]
pub struct Cli {
  /// How much to log on standard error. Without it, RUST_LOG is read; the default is warn
  #[arg(long, global = true, value_enum, value_name = "LEVEL")]
  pub log_level: Option<LogLevel>,

  #[command(subcommand)]
  pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
  /// Replay Janus contracts or v1-v4 pacts at a running provider
  Verify(VerifyArgs),
  /// Decide whether a consumer can deploy against a provider, from documents that already exist
  Check(CheckArgs),
  /// Print the plan the engine will execute for one interaction
  Explain(ExplainArgs),
  /// Convert a v1-v4 pact into a Janus contract, and say what the conversion could not carry
  Upgrade(UpgradeArgs),
  /// Publish a WASM component to an OCI registry, or fetch one and say what it is
  #[command(subcommand)]
  Component(ComponentCommand),
  /// Print a shell completion script
  Completions(CompletionsArgs),
  /// Print the CLI, engine and protocol versions
  Version,
}

impl Command {
  /// The name an error message is prefixed with: `janus verify: ...`.
  pub fn name(&self) -> &'static str {
    match self {
      Command::Verify(_) => "verify",
      Command::Check(_) => "check",
      Command::Explain(_) => "explain",
      Command::Upgrade(_) => "upgrade",
      Command::Component(ComponentCommand::Push(_)) => "component push",
      Command::Component(ComponentCommand::Pull(_)) => "component pull",
      Command::Completions(_) => "completions",
      Command::Version => "version",
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum LogLevel {
  Error,
  Warn,
  Info,
  Debug,
  Trace,
}

impl LogLevel {
  pub fn as_str(self) -> &'static str {
    match self {
      LogLevel::Error => "error",
      LogLevel::Warn => "warn",
      LogLevel::Info => "info",
      LogLevel::Debug => "debug",
      LogLevel::Trace => "trace",
    }
  }
}

/// How a command prints its result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Output {
  Text,
  Json,
}

#[derive(Debug, Args)]
#[command(
  after_help = "Exit status: 0 verified; 1 the provider failed verification, or the run \
                        was aborted; 2 the command could not run."
)]
pub struct VerifyArgs {
  /// Janus contracts or v1-v4 pact files, or directories of them
  #[arg(required = true, value_name = "CONTRACT_OR_PACT")]
  pub sources: Vec<PathBuf>,

  /// The base URL of the provider under test
  #[arg(long, value_name = "URL")]
  pub provider_url: String,

  /// A verifier configuration (verifier.janus.yaml): the hooks that set up provider states or
  /// authenticate, and the components the contracts need
  #[arg(long, value_name = "FILE")]
  pub config: Option<PathBuf>,

  /// Replay only this variant; repeatable. The run is reported as filtered, because a partial run
  /// is not a pass
  #[arg(long = "variant", value_name = "ID")]
  pub variants: Vec<String>,

  /// The transport to bind to the provider
  #[arg(long, value_name = "KIND", default_value = "http")]
  pub transport: String,

  /// Print the executed plan for every variant that fails
  #[arg(long)]
  pub explain_failures: bool,

  /// text prints progress and a summary; json prints only the run summary document
  #[arg(long, value_enum, value_name = "FORMAT", default_value_t = Output::Text)]
  pub output: Output,
}

#[derive(Debug, Args)]
#[command(
  after_help = "Exit status: 0 the consumer can deploy, possibly with warnings; 1 \
                        something blocked; 2 the command could not run. Both severities default \
                        to warn.\n\nThis command reads documents and decides; it never contacts \
                        the provider. Verify first (janus verify ... --output json > result.json) \
                        and pass the result with --verification: a pair with no verification \
                        result is reported as unverified, which is not a pass."
)]
pub struct CheckArgs {
  /// Consumer contracts or v1-v4 pacts, or directories of them
  #[arg(required = true, value_name = "CONTRACT_OR_PACT")]
  pub sources: Vec<PathBuf>,

  /// A provider shape document, or a directory of them; repeatable
  #[arg(long = "provider-shape", value_name = "PATH")]
  pub provider_shapes: Vec<PathBuf>,

  /// A run summary written by `janus verify --output json`; repeatable
  #[arg(long = "verification", value_name = "FILE")]
  pub verifications: Vec<PathBuf>,

  /// A subsumption policy (YAML or JSON): when to warn and when to block, and any exemptions
  #[arg(long, value_name = "FILE")]
  pub policy: Option<PathBuf>,

  /// Override the policy for findings the check decided: warn reports them, block also exits 1
  #[arg(long, value_enum, value_name = "SEVERITY")]
  pub on_finding: Option<Severity>,

  /// Override the policy for comparisons the check could not decide: warn or block, as above
  #[arg(long, value_enum, value_name = "SEVERITY")]
  pub on_review: Option<Severity>,

  /// Judge exemption expiry against this date instead of today (UTC)
  #[arg(long, value_name = "YYYY-MM-DD", value_parser = crate::date::parse)]
  pub as_of: Option<String>,

  /// text prints the report as prose; json prints the compatibility report document
  #[arg(long, value_enum, value_name = "FORMAT", default_value_t = Output::Text)]
  pub output: Output,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Severity {
  Warn,
  Block,
}

impl Severity {
  pub fn as_str(self) -> &'static str {
    match self {
      Severity::Warn => "warn",
      Severity::Block => "block",
    }
  }
}

#[derive(Debug, Args)]
#[command(after_help = "To see why a live verification failed, use janus verify \
                        --explain-failures instead: it prints the executed plan against what the \
                        provider actually sent.")]
pub struct ExplainArgs {
  /// An interaction specification, a Janus contract, or a v1-v4 pact
  #[arg(value_name = "DOCUMENT")]
  pub document: PathBuf,

  /// Which interaction of a contract or pact, counting from 0
  #[arg(long, value_name = "N", conflicts_with = "spec")]
  pub index: Option<usize>,

  /// Compile for this recorded variant of a Janus contract
  #[arg(long, value_name = "ID", conflicts_with = "spec")]
  pub variant: Option<String>,

  /// Read the document as an interaction specification, which cannot identify itself
  #[arg(long)]
  pub spec: bool,

  /// Print the structured plan document (JSON) instead of the text form
  #[arg(long, conflicts_with = "executed")]
  pub plan: bool,

  /// Run the plan against a JSON object of captured values (path to value) and print the executed
  /// form. Exits 1 if the values do not match
  #[arg(long, value_name = "VALUES")]
  pub executed: Option<PathBuf>,
}

#[derive(Debug, Args)]
#[command(
  after_help = "A converted pact has one variant per interaction, because one example is \
                        one variant; growing it is the consumer test suite's job. Verifying a pact \
                        needs no conversion: janus verify reads v1-v4 pacts directly."
)]
pub struct UpgradeArgs {
  /// The v1-v4 pact to convert
  #[arg(value_name = "PACT")]
  pub pact: PathBuf,

  /// Write the contract to this file instead of standard output
  #[arg(long, value_name = "FILE")]
  pub write: Option<PathBuf>,

  /// Do not print the findings
  #[arg(long)]
  pub quiet: bool,

  /// text prints the contract, and the findings on standard error; json prints one document
  /// holding both
  #[arg(long, value_enum, value_name = "FORMAT", default_value_t = Output::Text)]
  pub output: Output,
}

#[derive(Debug, Subcommand)]
#[command(
  after_help = "Loopback registries (localhost, 127.0.0.1) are plain HTTP; everything \
                        else is HTTPS. Credentials, when a registry wants them: JANUS_OCI_USERNAME \
                        and JANUS_OCI_PASSWORD. The cache is JANUS_COMPONENT_CACHE, or \
                        pact-janus/components in the user cache directory.\n\nExit status: 0 \
                        done; 1 the artifact is not what it claims (a digest that does not match, \
                        not a Janus component, a config that disagrees with the component); 2 the \
                        command could not run."
)]
pub enum ComponentCommand {
  /// Publish a WASM component as an OCI artifact, described by the component's own handshake
  Push(PushArgs),
  /// Fetch a component into the cache, check every byte, and print how to declare it, pinned
  Pull(PullArgs),
}

#[derive(Debug, Args)]
pub struct PushArgs {
  /// The component to publish
  #[arg(value_name = "COMPONENT_WASM")]
  pub component: PathBuf,

  /// Where to publish it: registry/repository:tag
  #[arg(value_name = "REFERENCE")]
  pub reference: String,

  /// text prints a summary and the declaration to paste; json prints one document
  #[arg(long, value_enum, value_name = "FORMAT", default_value_t = Output::Text)]
  pub output: Output,
}

#[derive(Debug, Args)]
pub struct PullArgs {
  /// What to fetch: registry/repository:tag, or registry/repository@sha256:...
  #[arg(value_name = "REFERENCE")]
  pub reference: String,

  /// The manifest digest the artifact must match
  #[arg(long, value_name = "sha256:...")]
  pub digest: Option<String>,

  /// text prints a summary and the declaration to paste; json prints one document
  #[arg(long, value_enum, value_name = "FORMAT", default_value_t = Output::Text)]
  pub output: Output,
}

#[derive(Debug, Args)]
#[command(
  after_help = "For example, for bash: janus completions bash > ~/.local/share/bash-completion/completions/janus"
)]
pub struct CompletionsArgs {
  /// The shell to generate the script for
  #[arg(value_enum)]
  pub shell: clap_complete::Shell,
}

#[cfg(test)]
mod tests {
  use super::Cli;
  use clap::CommandFactory;

  #[test]
  fn the_command_line_is_well_formed() {
    Cli::command().debug_assert();
  }
}
