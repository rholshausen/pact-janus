//! `verifier-diff`: the differential verification harness (staged plan S1.1).
//!
//! It runs today's verifier (`pact_verifier_cli`) and `janus verify` on the same pacts against the
//! same provider, and compares their verdicts interaction by interaction, over the fixture corpus
//! in `corpora/verifier/`. Every disagreement is either in the ledger with its reason or a failure.
//!
//! ```sh
//! cargo build -p pact_janus_cli --bin janus
//! cargo run -p pact_janus_verifier_diff -- run --broker "$(.github/scripts/pact-broker.sh start)"
//! cargo run -p pact_janus_verifier_diff -- import-compatibility-suite ../pact-compatibility-suite
//! ```
//!
//! Exit status: 0 the disagreements are exactly the ledger's; 1 they are not; 2 it could not run.

mod broker;
mod fixture;
mod import;
mod ledger;
mod mutate;
mod provider;
mod run;

use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Parser)]
#[command(
  name = "verifier-diff",
  bin_name = "verifier-diff",
  about = "Today's verifier and janus verify, on the same pacts against the same provider, compared interaction by interaction"
)]
struct Cli {
  /// The repository root (default: the working directory)
  #[arg(long, global = true, value_name = "DIR", default_value = ".")]
  repo: PathBuf,

  #[command(subcommand)]
  command: Command,
}

#[derive(Subcommand)]
enum Command {
  /// Run the corpus and judge it against the ledger
  Run {
    /// The janus CLI (default: the `janus` built beside this binary)
    #[arg(long, value_name = "PATH", env = "JANUS_CLI")]
    janus: Option<PathBuf>,
    /// Today's verifier (default: pact_verifier_cli on PATH)
    #[arg(long, value_name = "PATH", env = "PACT_VERIFIER_CLI")]
    pact_verifier: Option<PathBuf>,
    /// The Pact Broker the broker cases run against (`.github/scripts/pact-broker.sh start` prints it)
    #[arg(long, value_name = "URL", env = "VERIFIER_DIFF_BROKER")]
    broker: Option<String>,
    /// Run a pact_verifier_cli, or a broker, other than the pinned version
    #[arg(long)]
    any_version: bool,
    /// Only the cases whose id contains this text
    #[arg(long, value_name = "TEXT")]
    filter: Option<String>,
    /// How many cases to run at once (default: the number of CPUs, at most 8)
    #[arg(long, value_name = "N")]
    jobs: Option<usize>,
    /// Also write every verdict, and the judgement, to this file
    #[arg(long, value_name = "FILE")]
    json: Option<PathBuf>,
    /// Print ledger entries for the unexpected disagreements, to paste in once each has a reason
    #[arg(long)]
    skeleton: bool,
    /// Keep each case's pacts, configuration and reports, and say where
    #[arg(long)]
    keep: bool,
  },
  /// Check the corpus and the ledger without running anything
  Check,
  /// Print the routes a scripted provider honouring these pacts answers with, as a fixture's
  /// `provider.routes` takes them: an HTTP interaction's request and response, or a message's
  /// request and the message
  Routes {
    /// v1-v4 pact files
    #[arg(value_name = "PACT", required = true)]
    pacts: Vec<PathBuf>,
  },
  /// Regenerate corpora/verifier/compatibility-suite from a pact-compatibility-suite checkout
  ImportCompatibilitySuite {
    /// The checkout
    #[arg(value_name = "SUITE_DIR")]
    suite: PathBuf,
  },
}

fn main() -> ExitCode {
  let cli = Cli::parse();
  let corpus = cli.repo.join("corpora").join("verifier");
  let outcome = match cli.command {
    Command::Run {
      janus,
      pact_verifier,
      broker,
      any_version,
      filter,
      jobs,
      json,
      skeleton,
      keep,
    } => run(
      &cli.repo,
      &corpus,
      janus,
      pact_verifier,
      broker,
      any_version,
      filter,
      jobs,
      json,
      skeleton,
      keep,
    ),
    Command::Check => check(&cli.repo, &corpus).map(|summary| {
      println!("{summary}");
      true
    }),
    Command::Routes { pacts } => routes(&pacts).map(|routes| {
      println!("{routes}");
      true
    }),
    Command::ImportCompatibilitySuite { suite } => {
      import::import(&suite, &corpus.join("compatibility-suite")).map(|summary| {
        println!("verifier-diff: {summary}");
        true
      })
    }
  };
  match outcome {
    Ok(true) => ExitCode::SUCCESS,
    Ok(false) => ExitCode::from(1),
    Err(err) => {
      eprintln!("verifier-diff: {err}");
      ExitCode::from(2)
    }
  }
}

/// Every route of these pacts, in order, as pretty JSON.
fn routes(pacts: &[PathBuf]) -> Result<String, String> {
  let mut routes = Vec::new();
  for path in pacts {
    let text = std::fs::read_to_string(path).map_err(|err| format!("{}: {err}", path.display()))?;
    let document: serde_json::Value =
      serde_json::from_str(&text).map_err(|err| format!("{}: {err}", path.display()))?;
    for interaction in fixture::routes_of(&document, &path.display().to_string())? {
      routes.push(interaction.route);
    }
  }
  serde_json::to_string_pretty(&routes).map_err(|err| err.to_string())
}

/// The pinned version of today's verifier (`tools/verifier-diff/pact-verifier.json`).
fn pinned_version(repo: &Path) -> Result<String, String> {
  let path = repo
    .join("tools")
    .join("verifier-diff")
    .join("pact-verifier.json");
  let text = std::fs::read_to_string(&path).map_err(|err| format!("{}: {err}", path.display()))?;
  let pin: serde_json::Value =
    serde_json::from_str(&text).map_err(|err| format!("{}: {err}", path.display()))?;
  pin["version"]
    .as_str()
    .map(str::to_string)
    .ok_or_else(|| format!("{}: no version", path.display()))
}

fn load_ledger(corpus: &Path) -> Result<ledger::Ledger, String> {
  let path = corpus.join("disagreements.json");
  let text = std::fs::read_to_string(&path).map_err(|err| format!("{}: {err}", path.display()))?;
  let ledger: ledger::Ledger =
    serde_json::from_str(&text).map_err(|err| format!("{}: {err}", path.display()))?;
  ledger
    .check()
    .map_err(|problems| format!("{}:\n{problems}", path.display()))?;
  Ok(ledger)
}

fn load_upgrade_ledger(corpus: &Path) -> Result<ledger::UpgradeLedger, String> {
  let path = corpus.join("upgrade-disagreements.json");
  let text = std::fs::read_to_string(&path).map_err(|err| format!("{}: {err}", path.display()))?;
  let ledger: ledger::UpgradeLedger =
    serde_json::from_str(&text).map_err(|err| format!("{}: {err}", path.display()))?;
  ledger
    .check()
    .map_err(|problems| format!("{}:\n{problems}", path.display()))?;
  Ok(ledger)
}

/// Every case, and the ledger, read and checked: what CI can verify with no verifier installed.
fn cases(corpus: &Path) -> Result<(Vec<run::Case>, ledger::Ledger), String> {
  let ledger = load_ledger(corpus)?;
  let upgrades = load_upgrade_ledger(corpus)?;
  let mut cases = Vec::new();
  for (id, dir, fixture) in fixture::load_corpus(corpus)? {
    cases.extend(run::expand(&id, &dir, &fixture)?);
  }
  let ids: std::collections::BTreeSet<&str> = cases.iter().map(|case| case.id.as_str()).collect();
  let unknown: Vec<String> = ledger
    .disagreements
    .iter()
    .map(|entry| (&entry.case, &entry.key))
    .chain(
      upgrades
        .disagreements
        .iter()
        .map(|entry| (&entry.case, &entry.key)),
    )
    .filter(|(case, _)| !ids.contains(case.as_str()))
    .map(|(case, key)| format!("  {case} {key}"))
    .collect();
  if !unknown.is_empty() {
    return Err(format!(
      "the ledger lists cases the corpus does not have:\n{}",
      unknown.join("\n")
    ));
  }
  Ok((cases, ledger))
}

fn check(_repo: &Path, corpus: &Path) -> Result<String, String> {
  let (cases, ledger) = cases(corpus)?;
  Ok(format!(
    "verifier-diff: {} case(s), {} known disagreement(s), all readable",
    cases.len(),
    ledger.disagreements.len()
  ))
}

#[allow(clippy::too_many_arguments)]
fn run(
  repo: &Path,
  corpus: &Path,
  janus: Option<PathBuf>,
  pact_verifier: Option<PathBuf>,
  broker: Option<String>,
  any_version: bool,
  filter: Option<String>,
  jobs: Option<usize>,
  json: Option<PathBuf>,
  skeleton: bool,
  keep: bool,
) -> Result<bool, String> {
  let (mut cases, ledger) = cases(corpus)?;
  if let Some(filter) = &filter {
    cases.retain(|case| case.id.contains(filter.as_str()));
  }
  let broker = match (broker, cases.iter().any(|case| case.broker.is_some())) {
    (Some(url), true) => {
      let broker = broker::Broker::new(&url);
      let version = broker.version()?;
      let pinned = broker::pinned_version(repo)?;
      if version != pinned && !any_version {
        return Err(format!(
          "the broker at {url} is version {version}, and the ledger is kept against {pinned}: start the pinned one with .github/scripts/pact-broker.sh, or pass --any-version"
        ));
      }
      Some(broker)
    }
    (None, true) => {
      return Err(
        "the corpus has broker cases, and no broker was named: start one with `.github/scripts/pact-broker.sh start` and pass its URL with --broker (or VERIFIER_DIFF_BROKER), or --filter them out"
          .to_string(),
      );
    }
    (_, false) => None,
  };
  let tools = run::Tools {
    janus: janus.unwrap_or_else(|| beside_this_binary("janus")),
    today: pact_verifier.unwrap_or_else(|| PathBuf::from("pact_verifier_cli")),
    broker,
  };
  if !tools.janus.exists() {
    return Err(format!(
      "no janus at {}: build it with `cargo build -p pact_janus_cli --bin janus`, or name one with --janus",
      tools.janus.display()
    ));
  }
  let version = today_version(&tools.today)?;
  let pinned = pinned_version(repo)?;
  if version != pinned && !any_version {
    return Err(format!(
      "{} is pact_verifier_cli {version}, and the ledger is kept against {pinned}: install {pinned}, or pass --any-version",
      tools.today.display()
    ));
  }

  let work = std::env::temp_dir().join(format!("verifier-diff-{}", std::process::id()));
  let jobs = jobs
    .unwrap_or_else(|| std::thread::available_parallelism().map_or(1, |n| n.get()).min(8))
    .max(1);
  let next = AtomicUsize::new(0);
  let results = Mutex::new(Vec::new());
  let failures = Mutex::new(Vec::new());
  std::thread::scope(|scope| {
    for _ in 0..jobs {
      scope.spawn(|| {
        loop {
          let index = next.fetch_add(1, Ordering::Relaxed);
          let Some(case) = cases.get(index) else { break };
          match run::run_case(case, &tools, &work.join(index.to_string())) {
            Ok(result) => results.lock().expect("results").push((index, result)),
            Err(err) => failures
              .lock()
              .expect("failures")
              .push(format!("{}: {err}", case.id)),
          }
        }
      });
    }
  });
  if keep {
    println!(
      "verifier-diff: each case's files are in {}, by its position in the run",
      work.display()
    );
  } else {
    let _ = std::fs::remove_dir_all(&work);
  }
  let failures = failures.into_inner().expect("failures");
  if !failures.is_empty() {
    return Err(format!("cases that could not run:\n  {}", failures.join("\n  ")));
  }
  let mut results = results.into_inner().expect("results");
  results.sort_by_key(|(index, _)| *index);
  let results: Vec<run::CaseResult> = results.into_iter().map(|(_, result)| result).collect();

  let judgement = ledger::judge(&results, &ledger);
  report(&judgement, &version, filter.is_some());
  let upgrades = ledger::judge_upgrades(&results, &load_upgrade_ledger(corpus)?);
  report_upgrades(&upgrades);
  if skeleton && !judgement.unexpected.is_empty() {
    let entries: Vec<ledger::Entry> = judgement
      .unexpected
      .iter()
      .map(|entry| ledger::Entry {
        reason: format!("S?.?: TODO ({})", entry.reason),
        ..entry.clone()
      })
      .collect();
    println!("\n{}", serde_json::to_string_pretty(&entries).unwrap_or_default());
  }
  if let Some(path) = json {
    let document = serde_json::json!({
      "pact-verifier": version, "judgement": judgement, "upgrades": upgrades, "results": results,
    });
    std::fs::write(&path, serde_json::to_string_pretty(&document).unwrap_or_default())
      .map_err(|err| format!("{}: {err}", path.display()))?;
  }
  Ok(judgement.passed() && upgrades.passed())
}

/// The upgraded contracts' verdicts against the pacts' (staged plan S1.19).
fn report_upgrades(judgement: &ledger::UpgradeJudgement) {
  println!(
    "verifier-diff: janus verify on each pact and on the contract it upgrades into, {} verdict(s)",
    judgement.compared
  );
  println!(
    "  {} agree, {} known disagreement(s), {} unexpected, {} stale ledger entr{}",
    judgement.agreed,
    judgement.known.len(),
    judgement.unexpected.len(),
    judgement.stale.len(),
    if judgement.stale.len() == 1 { "y" } else { "ies" },
  );
  for entry in &judgement.unexpected {
    println!(
      "UNEXPECTED  {} [{}]  pact: {}  contract: {}",
      entry.case, entry.key, entry.pact, entry.contract
    );
    println!("            {}", entry.reason);
  }
  for entry in &judgement.stale {
    println!(
      "STALE       {} [{}]  listed as pact: {}  contract: {} ({})",
      entry.case, entry.key, entry.pact, entry.contract, entry.reason
    );
  }
  if !judgement.unexpected.is_empty() {
    let entries: Vec<ledger::UpgradeEntry> = judgement
      .unexpected
      .iter()
      .map(|entry| ledger::UpgradeEntry {
        reason: format!("S?.?: TODO ({})", entry.reason),
        ..entry.clone()
      })
      .collect();
    println!("\n{}", serde_json::to_string_pretty(&entries).unwrap_or_default());
  }
}

fn report(judgement: &ledger::Judgement, version: &str, filtered: bool) {
  println!(
    "verifier-diff: pact_verifier_cli {version} and janus verify, {} verdict(s) over {} case(s){}",
    judgement.compared,
    judgement.cases,
    if filtered { " (filtered)" } else { "" }
  );
  println!(
    "  {} agree, {} known disagreement(s), {} unexpected, {} stale ledger entr{}, {} unjudged, {} fixture problem(s)",
    judgement.agreed,
    judgement.known.len(),
    judgement.unexpected.len(),
    judgement.stale.len(),
    if judgement.stale.len() == 1 { "y" } else { "ies" },
    judgement.unjudged.len(),
    judgement.fixture_problems.len(),
  );
  for entry in &judgement.unexpected {
    println!(
      "UNEXPECTED  {} [{}]  today: {}  janus: {}",
      entry.case, entry.key, entry.today, entry.janus
    );
    println!("            {}", entry.reason);
  }
  for entry in &judgement.stale {
    println!(
      "STALE       {} [{}]  listed as today: {}  janus: {} ({})",
      entry.case, entry.key, entry.today, entry.janus, entry.reason
    );
  }
  for problem in &judgement.unjudged {
    println!("UNJUDGED    {problem}");
  }
  for problem in &judgement.fixture_problems {
    println!("FIXTURE     {problem}");
  }
}

/// `pact verifier version   : v1.3.5` as `1.3.5`.
fn today_version(today: &Path) -> Result<String, String> {
  let output = std::process::Command::new(today)
    .arg("--version")
    .output()
    .map_err(|err| {
      format!(
        "could not run {}: {err} (set PACT_VERIFIER_CLI, or pass --pact-verifier)",
        today.display()
      )
    })?;
  let text = String::from_utf8_lossy(&output.stdout);
  text
    .lines()
    .find_map(|line| line.strip_prefix("pact verifier version"))
    .and_then(|rest| rest.split(':').nth(1))
    .map(|version| version.trim().trim_start_matches('v').to_string())
    .ok_or_else(|| format!("{} did not say its version: {text}", today.display()))
}

fn beside_this_binary(name: &str) -> PathBuf {
  let file = format!("{name}{}", std::env::consts::EXE_SUFFIX);
  std::env::current_exe()
    .ok()
    .and_then(|exe| exe.parent().map(|dir| dir.join(&file)))
    .unwrap_or_else(|| PathBuf::from(file))
}

#[cfg(test)]
mod tests {
  /// The corpus and the ledger as checked in: every fixture reads, every pact is an HTTP pact the
  /// models can read, every mutation expands, and every ledger entry names a case and a reason. The
  /// run itself needs today's verifier and is CI's verifier-diff job.
  #[test]
  fn the_checked_in_corpus_and_ledger_are_readable() {
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let summary = super::check(&repo, &repo.join("corpora").join("verifier")).unwrap();
    assert!(summary.contains("case(s)"), "{summary}");
  }
}
