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
  #[arg(long, short = 'l', global = true, value_enum, value_name = "LEVEL")]
  pub log_level: Option<LogLevel>,

  #[command(subcommand)]
  pub command: Command,
}

// One `Command` is parsed per process, so `verify`'s many options cost nothing worth boxing for.
#[allow(clippy::large_enum_variant)]
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
  /// Publish Janus contracts or v1-v4 pacts to a Pact Broker, for a version of their consumer
  Publish(PublishArgs),
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
      Command::Publish(_) => "publish",
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

/// Which HTTP versions `verify` speaks to the provider (the HTTP client options' `http-version`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum HttpVersion {
  #[value(name = "1.1")]
  Http1,
  #[value(name = "2")]
  Http2,
  Negotiate,
}

impl HttpVersion {
  pub fn as_str(self) -> &'static str {
    match self {
      HttpVersion::Http1 => "1.1",
      HttpVersion::Http2 => "2",
      HttpVersion::Negotiate => "negotiate",
    }
  }
}

/// `--header KEY=VALUE`, as today's verifier reads it: split at the first `=`, the name and value
/// as HTTP allows them. The value is not repeated in an error, since it is usually a credential.
fn header(text: &str) -> Result<(String, String), String> {
  let (name, value) = text
    .split_once('=')
    .ok_or_else(|| "a header is KEY=VALUE".to_string())?;
  let name = name.trim();
  let valid_name = !name.is_empty()
    && name
      .bytes()
      .all(|byte| byte.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&byte));
  if !valid_name {
    return Err(format!("'{name}' is not a header name"));
  }
  if value.bytes().any(|byte| byte.is_ascii_control() && byte != b'\t') {
    return Err(format!("the value of '{name}' has a control character in it"));
  }
  Ok((name.to_string(), value.to_string()))
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
                        was aborted; 2 the command could not run.",
  arg_required_else_help = true
)]
pub struct VerifyArgs {
  /// Janus contracts or v1-v4 pact files, or directories of them. Not with --broker-url
  #[arg(
    required_unless_present = "broker_url",
    conflicts_with = "broker_url",
    value_name = "CONTRACT_OR_PACT"
  )]
  pub sources: Vec<PathBuf>,

  /// The base URL of the provider under test
  #[arg(long, short = 'u', value_name = "URL")]
  pub provider_url: String,

  /// A verifier configuration (verifier.janus.yaml): the hooks that set up provider states or
  /// authenticate, and the components the contracts need
  #[arg(long, value_name = "FILE")]
  pub config: Option<PathBuf>,

  /// Replay only this variant; repeatable. An id no document records is refused, and a run that
  /// left a variant out is reported as filtered, never as verified, and is not published
  #[arg(long = "variant", value_name = "ID")]
  pub variants: Vec<String>,

  /// The transport to bind to the provider
  #[arg(long, value_name = "KIND", default_value = "http")]
  pub transport: String,

  /// Print the executed plan for every variant that fails
  #[arg(long)]
  pub explain_failures: bool,

  /// text prints progress and a summary; json prints only the run summary document
  #[arg(long, short = 'o', value_enum, value_name = "FORMAT", default_value_t = Output::Text)]
  pub output: Output,

  // How the provider is reached (staged plan S1.5, ADR 0035). Today's verifier's options keep its
  // spellings and environment variables (Documentation/verifier-migration.md records each); the
  // rest do what it cannot. They configure the provider connection only, as today's do.
  /// A path to put in front of every request's path
  #[arg(
    long,
    value_name = "PATH",
    env = "PACT_PROVIDER_BASE_PATH",
    help_heading = "Provider connection"
  )]
  pub base_path: Option<String>,

  /// A header for every request to the provider, replacing the interaction's own of that name;
  /// repeatable
  #[arg(
    long = "header",
    short = 'H',
    value_name = "KEY=VALUE",
    value_parser = header,
    help_heading = "Provider connection"
  )]
  pub headers: Vec<(String, String)>,

  /// How long a request to the provider may take, in milliseconds [default: 5000]
  #[arg(
    long,
    value_name = "MS",
    env = "PACT_PROVIDER_REQUEST_TIMEOUT",
    value_parser = clap::value_parser!(u64).range(1..),
    help_heading = "Provider connection"
  )]
  pub request_timeout: Option<u64>,

  /// How long connecting to the provider may take, in milliseconds
  #[arg(
    long,
    value_name = "MS",
    value_parser = clap::value_parser!(u64).range(1..),
    help_heading = "Provider connection"
  )]
  pub connect_timeout: Option<u64>,

  /// Accept any certificate the provider presents
  #[arg(long, help_heading = "Provider connection")]
  pub disable_ssl_verification: bool,

  /// A PEM file of CA certificates to trust beside the system's; repeatable
  #[arg(long = "ca-cert", value_name = "FILE", help_heading = "Provider connection")]
  pub ca_certs: Vec<PathBuf>,

  /// A PEM certificate (or chain) to present when the provider asks for one
  #[arg(
    long,
    value_name = "FILE",
    requires = "client_key",
    help_heading = "Provider connection"
  )]
  pub client_cert: Option<PathBuf>,

  /// The PEM private key of --client-cert
  #[arg(
    long,
    value_name = "FILE",
    requires = "client_cert",
    help_heading = "Provider connection"
  )]
  pub client_key: Option<PathBuf>,

  /// Reach the provider through this proxy. Without it, HTTPS_PROXY, HTTP_PROXY, ALL_PROXY and
  /// NO_PROXY are read
  #[arg(long, value_name = "URL", help_heading = "Provider connection")]
  pub proxy: Option<String>,

  /// Hosts to reach directly rather than through --proxy, comma-separated, as NO_PROXY
  #[arg(
    long,
    value_name = "HOSTS",
    value_delimiter = ',',
    requires = "proxy",
    help_heading = "Provider connection"
  )]
  pub no_proxy: Vec<String>,

  /// The HTTP versions to speak to the provider: 1.1, as today's verifier; 2 only; or negotiate,
  /// offering both over TLS [default: 1.1]
  #[arg(
    long,
    value_enum,
    value_name = "VERSION",
    help_heading = "Provider connection"
  )]
  pub http_version: Option<HttpVersion>,

  // The Pact Broker options are spelled as today's verifier (pact_verifier_cli) spells them, so a
  // provider's CI script moves by changing the command's name; staged plan S1.13's table records
  // each. The credentials are read from today's environment variables too, and never logged.
  /// Fetch the pacts to verify from this Pact Broker instead of from files
  #[arg(
    long,
    value_name = "URL",
    requires = "provider_name",
    help_heading = "Pact Broker"
  )]
  pub broker_url: Option<String>,

  /// The provider whose pacts to fetch
  #[arg(
    long,
    value_name = "NAME",
    requires = "broker_url",
    help_heading = "Pact Broker"
  )]
  pub provider_name: Option<String>,

  /// A consumer version selector, as the broker reads it, in JSON; repeatable
  /// (https://docs.pact.io/pact_broker/advanced_topics/consumer_version_selectors)
  #[arg(
    long = "consumer-version-selectors",
    value_name = "JSON",
    requires = "broker_url",
    help_heading = "Pact Broker"
  )]
  pub consumer_version_selectors: Vec<String>,

  /// Ask the broker which pacts are pending: their failures are reported, and do not fail the run.
  /// Only with --consumer-version-selectors, as today's verifier
  #[arg(long, requires = "broker_url", help_heading = "Pact Broker")]
  pub enable_pending: bool,

  /// Also verify the work-in-progress pacts published since this date (YYYY-MM-DD); they are
  /// pending. Only with --consumer-version-selectors, as today's verifier
  #[arg(
    long,
    value_name = "DATE",
    requires = "broker_url",
    help_heading = "Pact Broker"
  )]
  pub include_wip_pacts_since: Option<String>,

  /// The provider version being verified, which --publish publishes results for
  #[arg(long, value_name = "VERSION", help_heading = "Pact Broker")]
  pub provider_version: Option<String>,

  /// The provider version's branch, which the broker decides pending status from
  #[arg(long, value_name = "BRANCH", help_heading = "Pact Broker")]
  pub provider_branch: Option<String>,

  /// The provider version's tags, comma-separated
  #[arg(
    long,
    value_name = "TAGS",
    value_delimiter = ',',
    help_heading = "Pact Broker"
  )]
  pub provider_tags: Vec<String>,

  /// Publish each pact's result back to the broker
  #[arg(long, requires_all = ["broker_url", "provider_version"], help_heading = "Pact Broker")]
  pub publish: bool,

  /// The build that ran this verification, recorded with each published result
  #[arg(long, value_name = "URL", help_heading = "Pact Broker")]
  pub build_url: Option<String>,

  /// The broker's username, for basic authentication
  #[arg(
    long,
    visible_alias = "broker-user",
    value_name = "USER",
    env = "PACT_BROKER_USERNAME",
    hide_env_values = true,
    help_heading = "Pact Broker"
  )]
  pub user: Option<String>,

  /// The broker's password, for basic authentication
  #[arg(
    long,
    visible_alias = "broker-password",
    value_name = "PASSWORD",
    env = "PACT_BROKER_PASSWORD",
    hide_env_values = true,
    help_heading = "Pact Broker"
  )]
  pub password: Option<String>,

  /// A bearer token for the broker; used instead of --user when both are given
  #[arg(
    long,
    visible_alias = "broker-token",
    value_name = "TOKEN",
    env = "PACT_BROKER_TOKEN",
    hide_env_values = true,
    help_heading = "Pact Broker"
  )]
  pub token: Option<String>,
}

#[derive(Debug, Args)]
#[command(
  after_help = "Exit status: 0 the consumer can deploy, possibly with warnings; 1 \
                        something blocked; 2 the command could not run. Both severities default \
                        to warn.\n\nThis command reads documents and decides; it never contacts \
                        the provider. Verify first (janus verify ... --output json > result.json) \
                        and pass the result with --verification: a pair with no verification \
                        result is reported as unverified, which is not a pass.",
  arg_required_else_help = true
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
#[command(
  after_help = "To see why a live verification failed, use janus verify \
                        --explain-failures instead: it prints the executed plan against what the \
                        provider actually sent.",
  arg_required_else_help = true
)]
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
                        needs no conversion: janus verify reads v1-v4 pacts directly.",
  arg_required_else_help = true
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

// The options are spelled as today's publisher (`pact-broker publish`, the Pact Broker client)
// spells them, short forms included, so a consumer's CI script moves by changing the command's
// name. Its `-h` for --branch is clap's help here, so --branch has no short form. Not carried:
// --auto-detect-version-properties and --tag-with-git-branch, which read git and CI variables,
// and --merge, which the protocol has no member for.
#[derive(Debug, Args)]
#[command(
  after_help = "Every contract is published as the broker's 'pact' specification, which every \
                        Pact Broker accepts. Contracts from several consumers are published one \
                        consumer at a time, each for the same version.\n\nExit status: 0 \
                        published; 2 something was not published (the broker refused it or could \
                        not be reached), or the command could not run.",
  arg_required_else_help = true
)]
pub struct PublishArgs {
  /// Janus contracts or v1-v4 pact files, or directories of them
  #[arg(required = true, value_name = "CONTRACT_OR_PACT")]
  pub sources: Vec<PathBuf>,

  /// The version of the consumer these contracts belong to
  #[arg(short = 'a', long, value_name = "VERSION")]
  pub consumer_app_version: String,

  /// The consumer version's branch
  #[arg(long, value_name = "BRANCH")]
  pub branch: Option<String>,

  /// A tag for the consumer version; repeatable, or comma-separated
  #[arg(short = 't', long = "tag", value_name = "TAG", value_delimiter = ',')]
  pub tags: Vec<String>,

  /// The build that produced the contracts
  #[arg(long, value_name = "URL")]
  pub build_url: Option<String>,

  /// text prints what was published and the broker's notices; json prints one document
  #[arg(long, value_enum, value_name = "FORMAT", default_value_t = Output::Text)]
  pub output: Output,

  /// The Pact Broker's base URL
  #[arg(
    short = 'b',
    long,
    value_name = "URL",
    env = "PACT_BROKER_BASE_URL",
    hide_env_values = true,
    help_heading = "Pact Broker"
  )]
  pub broker_base_url: String,

  /// The broker's username, for basic authentication
  #[arg(
    short = 'u',
    long,
    value_name = "USER",
    env = "PACT_BROKER_USERNAME",
    hide_env_values = true,
    help_heading = "Pact Broker"
  )]
  pub broker_username: Option<String>,

  /// The broker's password, for basic authentication
  #[arg(
    short = 'p',
    long,
    value_name = "PASSWORD",
    env = "PACT_BROKER_PASSWORD",
    hide_env_values = true,
    help_heading = "Pact Broker"
  )]
  pub broker_password: Option<String>,

  /// A bearer token for the broker; used instead of --broker-username when both are given
  #[arg(
    short = 'k',
    long,
    value_name = "TOKEN",
    env = "PACT_BROKER_TOKEN",
    hide_env_values = true,
    help_heading = "Pact Broker"
  )]
  pub broker_token: Option<String>,
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
#[command(arg_required_else_help = true)]
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
#[command(arg_required_else_help = true)]
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
  after_help = "For example, for bash: janus completions bash > ~/.local/share/bash-completion/completions/janus",
  arg_required_else_help = true
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
