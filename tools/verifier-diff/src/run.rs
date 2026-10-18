//! Running a case: start its provider, run both verifiers against it, and read back a verdict for
//! every interaction and for the run.
//!
//! Each verifier is run as a user runs it — `pact_verifier_cli` with `--json`, `janus verify` with
//! `--output json` — and read only from what it reports. Today's verifier reports every interaction.
//! `janus verify`'s summary lists failures, so its passes are inferred: every interaction not in
//! `failures`, provided the summary's own counts account for all of them. (S1.3 needs every
//! variant's result in the summary anyway, to publish it; until then this is the inference.)
//!
//! A broker case ([`crate::broker`]) publishes its pacts to a Pact Broker first, under names of each
//! verifier's own, and has the verifiers fetch them from it and publish back. It is compared on what
//! the run published for each pact too, and an interaction the selectors did not fetch is `absent`.

use crate::broker::{self, Broker, RecordingProxy, Traffic};
use crate::fixture::{
  self, BrokerSpec, BrokerVerify, Fixture, ProviderSpec, Published, Route, RunVerdict, StateCallback,
};
use crate::mutate;
use crate::provider::{STATE_PATH, ScriptedProvider};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Where the two verifiers are, and the broker the broker cases run against.
pub struct Tools {
  pub janus: PathBuf,
  pub today: PathBuf,
  pub broker: Option<Broker>,
}

pub struct Case {
  pub id: String,
  /// The pacts, by file name, as written for the verifiers.
  pub pacts: Vec<(String, Value)>,
  pub provider: CaseProvider,
  pub expected: Option<RunVerdict>,
  pub broker: Option<BrokerSpec>,
}

pub enum CaseProvider {
  Scripted {
    routes: Vec<Route>,
    callback: StateCallback,
  },
  OrderService,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Verdict {
  Pass,
  Fail,
  /// The verifier could not say: it did not run, or its report does not cover the interaction.
  Error,
  /// In a broker case: an interaction the verifier did not fetch, or a pact it published no result
  /// for.
  Absent,
}

impl From<Published> for Verdict {
  fn from(published: Published) -> Self {
    match published {
      Published::Pass => Verdict::Pass,
      Published::Fail => Verdict::Fail,
      Published::Absent => Verdict::Absent,
    }
  }
}

impl From<RunVerdict> for Verdict {
  fn from(verdict: RunVerdict) -> Self {
    match verdict {
      RunVerdict::Pass => Verdict::Pass,
      RunVerdict::Fail => Verdict::Fail,
    }
  }
}

impl std::fmt::Display for Verdict {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str(match self {
      Verdict::Pass => "pass",
      Verdict::Fail => "fail",
      Verdict::Error => "error",
      Verdict::Absent => "absent",
    })
  }
}

/// One thing both verifiers gave a verdict on: an interaction (`"<pact>/<interaction>"`, counting
/// from 0) or the whole run (`"run"`).
#[derive(Debug, Clone, Serialize)]
pub struct Compared {
  pub key: String,
  pub description: String,
  pub today: Verdict,
  pub janus: Verdict,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub today_detail: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub janus_detail: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CaseResult {
  pub id: String,
  /// How long the case took, both verifiers included.
  pub millis: u128,
  pub compared: Vec<Compared>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub expected: Option<RunVerdict>,
  /// What the fixture's source says a broker case publishes, by the key it is compared under.
  #[serde(skip_serializing_if = "BTreeMap::is_empty")]
  pub expected_published: BTreeMap<String, Verdict>,
  /// What each verifier asked of the broker, in a broker case.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub broker: Option<Traffic>,
}

/// Every case the corpus holds: each fixture as written, and each of its mutations.
pub fn expand(id: &str, dir: &Path, fixture: &Fixture) -> Result<Vec<Case>, String> {
  let mut pacts = Vec::new();
  for name in &fixture.pacts {
    let path = dir.join(name);
    let text = std::fs::read_to_string(&path).map_err(|err| format!("{}: {err}", path.display()))?;
    let document: Value = serde_json::from_str(&text).map_err(|err| format!("{}: {err}", path.display()))?;
    let file = Path::new(name)
      .file_name()
      .map(|file| file.to_string_lossy().into_owned())
      .unwrap_or_else(|| name.clone());
    pacts.push((file, document));
  }
  let provider = match &fixture.provider {
    ProviderSpec::Scripted {
      state_callback,
      routes,
    } => CaseProvider::Scripted {
      routes: routes.clone(),
      callback: *state_callback,
    },
    ProviderSpec::OrderService => CaseProvider::OrderService,
  };

  if let Some(spec) = &fixture.broker {
    check_broker_fixture(spec, &pacts, fixture.mutations)?;
  }

  let mut cases = Vec::new();
  if fixture.mutations {
    for (p, (file, document)) in pacts.iter().enumerate() {
      for (i, interaction) in fixture::http_interactions(document, file)?.iter().enumerate() {
        let route = Route::from_interaction(interaction);
        let mut single = document.clone();
        single["interactions"] = json!([document["interactions"][i].clone()]);
        for mutation in mutate::mutations(&route.response) {
          cases.push(Case {
            id: format!("{id}/mutations/{p}/{i}/{}", mutation.id),
            pacts: vec![(file.clone(), single.clone())],
            provider: CaseProvider::Scripted {
              routes: vec![Route {
                response: mutation.response,
                ..route.clone()
              }],
              callback: StateCallback::Ok,
            },
            expected: None,
            broker: None,
          });
        }
      }
    }
  }
  cases.insert(
    0,
    Case {
      id: id.to_string(),
      pacts,
      provider,
      expected: fixture.expected,
      broker: fixture.broker.clone(),
    },
  );
  Ok(cases)
}

/// What a broker fixture must be for its run to mean anything: one provider, no mutations (each
/// would publish to the broker again), and names that are the file names of the fixture's pacts.
fn check_broker_fixture(spec: &BrokerSpec, pacts: &[(String, Value)], mutations: bool) -> Result<(), String> {
  if mutations {
    return Err("a broker fixture cannot also be mutated".to_string());
  }
  let providers: std::collections::BTreeSet<&str> = pacts
    .iter()
    .map(|(_, document)| document["provider"]["name"].as_str().unwrap_or_default())
    .collect();
  if providers.len() != 1 {
    return Err(format!(
      "a broker fixture's pacts must have one provider, not {providers:?}"
    ));
  }
  // Today's report does not always say which consumer a result is for, so a description shared
  // by two pacts could be read as the other's, in whatever order the broker returned them.
  let mut descriptions = std::collections::BTreeSet::new();
  for (file, document) in pacts {
    for interaction in document["interactions"].as_array().into_iter().flatten() {
      let description = interaction["description"].as_str().unwrap_or_default();
      if !descriptions.insert(description.to_string()) {
        return Err(format!(
          "'{description}' in {file}: a broker fixture's interaction descriptions must be unique across its pacts"
        ));
      }
    }
  }
  let files: Vec<&str> = pacts.iter().map(|(file, _)| file.as_str()).collect();
  let named = spec
    .publish
    .keys()
    .chain(spec.expected_published.keys())
    .chain(spec.results.iter().map(|result| &result.pact));
  for name in named {
    if !files.contains(&name.as_str()) {
      return Err(format!(
        "the broker section names '{name}', which is not the file name of one of the fixture's pacts"
      ));
    }
  }
  Ok(())
}

/// What every interaction is, in the order both verifiers meet them: consumer, description, and the
/// key the comparison uses.
struct Expected {
  key: String,
  consumer: String,
  description: String,
}

/// Every interaction of these pacts, keyed by its position.
fn expected_of(pacts: &[(String, Value)]) -> Vec<Expected> {
  let mut expected = Vec::new();
  for (p, (_, document)) in pacts.iter().enumerate() {
    let consumer = document["consumer"]["name"]
      .as_str()
      .unwrap_or_default()
      .to_string();
    for (i, interaction) in document["interactions"]
      .as_array()
      .into_iter()
      .flatten()
      .enumerate()
    {
      expected.push(Expected {
        key: format!("{p}/{i}"),
        consumer: consumer.clone(),
        description: interaction["description"]
          .as_str()
          .unwrap_or_default()
          .to_string(),
      });
    }
  }
  expected
}

/// Where a verifier reads its pacts from.
enum Source<'a> {
  Files(&'a [PathBuf]),
  Broker {
    url: String,
    provider: String,
    verify: &'a BrokerVerify,
  },
}

pub fn run_case(case: &Case, tools: &Tools, work: &Path) -> Result<CaseResult, String> {
  if let Some(spec) = &case.broker {
    return run_broker_case(case, spec, tools, work);
  }
  let started = std::time::Instant::now();
  std::fs::create_dir_all(work).map_err(|err| format!("{}: {err}", work.display()))?;
  let mut files = Vec::new();
  for (p, (file, document)) in case.pacts.iter().enumerate() {
    let path = work.join(format!("{p}-{file}"));
    std::fs::write(&path, serde_json::to_vec_pretty(document).unwrap_or_default())
      .map_err(|err| format!("{}: {err}", path.display()))?;
    files.push(path);
  }
  let expected = expected_of(&case.pacts);

  // Each verifier gets a provider of its own, freshly started: a provider remembers things — the
  // states set up, a callback that has already failed once, the orders a state created — and what
  // one verifier did to it must not decide what the other sees.
  let (today_run, today) = {
    let provider = Started::start(&case.provider)?;
    run_today(
      tools,
      &Source::Files(&files),
      provider.port,
      provider.state_url.as_deref(),
      work,
      &expected,
    )
  };
  let (janus_run, janus) = {
    let provider = Started::start(&case.provider)?;
    run_janus(
      tools,
      &Source::Files(&files),
      &provider.base_url,
      provider.state_url.as_deref(),
      work,
      &expected,
    )
  };

  let mut compared = vec![Compared {
    key: "run".to_string(),
    description: "the whole run".to_string(),
    today: today_run.0,
    janus: janus_run.0,
    today_detail: today_run.1,
    janus_detail: janus_run.1,
  }];
  for ((entry, today), janus) in expected.iter().zip(today).zip(janus) {
    compared.push(Compared {
      key: entry.key.clone(),
      description: entry.description.clone(),
      today: today.0,
      janus: janus.0,
      today_detail: today.1,
      janus_detail: janus.1,
    });
  }
  Ok(CaseResult {
    id: case.id.clone(),
    millis: started.elapsed().as_millis(),
    compared,
    expected: case.expected,
    expected_published: BTreeMap::new(),
    broker: None,
  })
}

/// One broker case at a time: the broker is shared, and its SQLite database is not one to race.
static BROKER_CASES: Mutex<()> = Mutex::new(());
static BROKER_RUNS: AtomicUsize = AtomicUsize::new(0);

/// What one verifier did in a broker case.
struct BrokerSide {
  run: Judged,
  interactions: Vec<Judged>,
  published: Vec<Judged>,
  traffic: Vec<broker::Exchange>,
}

fn run_broker_case(case: &Case, spec: &BrokerSpec, tools: &Tools, work: &Path) -> Result<CaseResult, String> {
  let broker = tools
    .broker
    .as_ref()
    .ok_or("a broker case needs a broker: start one with `.github/scripts/pact-broker.sh start` and name it with --broker")?;
  let _one_at_a_time = BROKER_CASES
    .lock()
    .unwrap_or_else(|poisoned| poisoned.into_inner());
  let started = std::time::Instant::now();
  std::fs::create_dir_all(work).map_err(|err| format!("{}: {err}", work.display()))?;
  // A run of its own on a broker that outlives it: the process, the time and the case keep every
  // run's pacticipants apart from every other's.
  let seconds = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .map_or(0, |elapsed| elapsed.as_secs());
  let run = format!(
    "vd{}-{seconds}-{}",
    std::process::id(),
    BROKER_RUNS.fetch_add(1, Ordering::Relaxed)
  );
  let expected = expected_of(&case.pacts);
  let today = broker_side(
    case,
    spec,
    tools,
    broker,
    &format!("{run}-today-"),
    work,
    Side::Today,
  )?;
  let janus = broker_side(
    case,
    spec,
    tools,
    broker,
    &format!("{run}-janus-"),
    work,
    Side::Janus,
  )?;

  let mut compared = vec![Compared {
    key: "run".to_string(),
    description: "the whole run".to_string(),
    today: today.run.0,
    janus: janus.run.0,
    today_detail: today.run.1,
    janus_detail: janus.run.1,
  }];
  for ((entry, today), janus) in expected.iter().zip(today.interactions).zip(janus.interactions) {
    compared.push(Compared {
      key: entry.key.clone(),
      description: entry.description.clone(),
      today: today.0,
      janus: janus.0,
      today_detail: today.1,
      janus_detail: janus.1,
    });
  }
  let mut expected_published = BTreeMap::new();
  for (p, ((file, _), (today, janus))) in case
    .pacts
    .iter()
    .zip(today.published.into_iter().zip(janus.published))
    .enumerate()
  {
    let key = format!("{p}/published");
    if let Some(published) = spec.expected_published.get(file) {
      expected_published.insert(key.clone(), Verdict::from(*published));
    }
    compared.push(Compared {
      key,
      description: format!("what the run published for {file}"),
      today: today.0,
      janus: janus.0,
      today_detail: today.1,
      janus_detail: janus.1,
    });
  }
  Ok(CaseResult {
    id: case.id.clone(),
    millis: started.elapsed().as_millis(),
    compared,
    expected: case.expected,
    expected_published,
    broker: Some(Traffic {
      today: today.traffic,
      janus: janus.traffic,
    }),
  })
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
  Today,
  Janus,
}

/// Publish the case's pacts under `prefix`, start a fresh provider and a recording proxy, run one
/// verifier against them, and read back what it published.
#[allow(clippy::too_many_arguments)]
fn broker_side(
  case: &Case,
  spec: &BrokerSpec,
  tools: &Tools,
  broker: &Broker,
  prefix: &str,
  work: &Path,
  side: Side,
) -> Result<BrokerSide, String> {
  let renamed: Vec<(String, Value)> = case
    .pacts
    .iter()
    .map(|(file, document)| {
      let mut document = document.clone();
      for role in ["consumer", "provider"] {
        let name = document[role]["name"].as_str().unwrap_or_default().to_string();
        document[role]["name"] = json!(format!("{prefix}{name}"));
      }
      (file.clone(), document)
    })
    .collect();
  let pacts = broker.seed(spec, &renamed)?;
  let provider_name = pacts
    .first()
    .map(|pact| pact.provider.clone())
    .unwrap_or_default();
  let expected = expected_of(&renamed);

  let provider = Started::start(&case.provider)?;
  let proxy = RecordingProxy::start(broker.base_url())?;
  let source = Source::Broker {
    url: proxy.base_url(),
    provider: provider_name,
    verify: &spec.verify,
  };
  let side_work = work.join(match side {
    Side::Today => "today",
    Side::Janus => "janus",
  });
  std::fs::create_dir_all(&side_work).map_err(|err| format!("{}: {err}", side_work.display()))?;
  let (run, interactions) = match side {
    Side::Today => run_today(
      tools,
      &source,
      provider.port,
      provider.state_url.as_deref(),
      &side_work,
      &expected,
    ),
    Side::Janus => run_janus(
      tools,
      &source,
      &provider.base_url,
      provider.state_url.as_deref(),
      &side_work,
      &expected,
    ),
  };
  let traffic = broker::normalised(proxy.exchanges(), prefix, &proxy.base_url());
  drop(proxy);
  let _ = std::fs::write(
    side_work.join("broker-traffic.json"),
    serde_json::to_vec_pretty(&traffic).unwrap_or_default(),
  );
  let published = pacts
    .iter()
    .map(
      |pact| match broker.published(pact, &spec.verify.provider_version) {
        Ok(published) => (Verdict::from(published), None),
        Err(err) => (Verdict::Error, Some(err)),
      },
    )
    .collect();
  Ok(BrokerSide {
    run,
    interactions,
    published,
    traffic,
  })
}

/// A case's provider, running until dropped.
struct Started {
  base_url: String,
  port: u16,
  state_url: Option<String>,
  _scripted: Option<ScriptedProvider>,
  _order_service: Option<pact_janus_sample_order_service::Provider>,
}

impl Started {
  fn start(provider: &CaseProvider) -> Result<Started, String> {
    match provider {
      CaseProvider::Scripted { routes, callback } => {
        let provider = ScriptedProvider::start(routes.clone(), *callback)?;
        Ok(Started {
          base_url: provider.base_url(),
          port: provider.port(),
          state_url: (*callback != StateCallback::None)
            .then(|| format!("{}{STATE_PATH}", provider.base_url())),
          _scripted: Some(provider),
          _order_service: None,
        })
      }
      CaseProvider::OrderService => {
        let provider = pact_janus_sample_order_service::start(pact_janus_sample_order_service::Config {
          token: None,
          ..Default::default()
        })
        .map_err(|err| format!("starting the order-service: {err}"))?;
        let base_url = provider.base_url().to_string();
        let port = base_url
          .rsplit(':')
          .next()
          .and_then(|port| port.parse().ok())
          .unwrap_or(0);
        Ok(Started {
          state_url: Some(format!("{base_url}/_pact/provider-states")),
          base_url,
          port,
          _scripted: None,
          _order_service: Some(provider),
        })
      }
    }
  }
}

type Judged = (Verdict, Option<String>);

/// The broker options, in today's verifier's spelling. `janus verify` is given the same spelling:
/// S1.13 maps today's options onto janus's, and until a janus option exists its absence is the
/// disagreement the ledger records.
fn broker_args(command: &mut Command, url: &str, provider: &str, verify: &BrokerVerify) {
  command.args(["--broker-url", url, "--provider-name", provider]);
  for selector in &verify.selectors {
    command
      .arg("--consumer-version-selectors")
      .arg(selector.to_string());
  }
  if verify.enable_pending {
    command.arg("--enable-pending");
  }
  if let Some(since) = &verify.include_wip_pacts_since {
    command.args(["--include-wip-pacts-since", since]);
  }
  command.args(["--provider-version", &verify.provider_version]);
  if let Some(branch) = &verify.provider_branch {
    command.args(["--provider-branch", branch]);
  }
  if verify.publish {
    command.arg("--publish");
  }
}

fn run_today(
  tools: &Tools,
  source: &Source,
  port: u16,
  state_url: Option<&str>,
  work: &Path,
  expected: &[Expected],
) -> (Judged, Vec<Judged>) {
  let report_path = work.join("today.json");
  let mut command = Command::new(&tools.today);
  let from_broker = match source {
    Source::Files(files) => {
      for file in *files {
        command.arg("--file").arg(file);
      }
      false
    }
    Source::Broker {
      url,
      provider,
      verify,
    } => {
      broker_args(&mut command, url, provider, verify);
      true
    }
  };
  command
    .args([
      "--hostname",
      "127.0.0.1",
      "--port",
      &port.to_string(),
      "--loglevel",
      "error",
      "--no-colour",
    ])
    .arg("--json")
    .arg(&report_path)
    .env("PACT_DO_NOT_TRACK", "true");
  if let Some(url) = state_url {
    command.args(["--state-change-url", url]);
  }
  let output = match command.output() {
    Ok(output) => output,
    Err(err) => {
      return everything_errs(
        expected,
        format!("could not run {}: {err}", tools.today.display()),
      );
    }
  };
  let report: Value = match std::fs::read(&report_path)
    .ok()
    .and_then(|bytes| serde_json::from_slice(&bytes).ok())
  {
    Some(report) => report,
    None => {
      let said = first_line(&output.stderr).or_else(|| first_line(&output.stdout));
      return everything_errs(expected, format!("wrote no report: {}", said.unwrap_or_default()));
    }
  };

  let run = match report["result"].as_bool() {
    Some(true) => (Verdict::Pass, None),
    Some(false) => (Verdict::Fail, None),
    None => (Verdict::Error, Some("the report has no result".to_string())),
  };
  let results = report["interactionResults"]
    .as_array()
    .cloned()
    .unwrap_or_default();
  let mut used = vec![false; results.len()];
  let judged = expected
    .iter()
    .map(|entry| {
      // In order, matched by consumer and description: the report says which interaction each is.
      let found = results.iter().enumerate().find(|(index, result)| {
        !used[*index]
          && result["description"].as_str() == Some(entry.description.as_str())
          && result["consumer"]
            .as_str()
            .is_none_or(|consumer| consumer == entry.consumer)
      });
      match found {
        Some((index, result)) => {
          used[index] = true;
          let mut pending = result["pending"] == json!(true);
          match result["result"].as_str() {
            Some(ok) if ok.eq_ignore_ascii_case("ok") => {
              (Verdict::Pass, pending.then(|| "pending".to_string()))
            }
            Some(_) => {
              // Newer releases put the mismatch on the result; older ones only under `errors`, or
              // under `pendingErrors` when the pact is pending.
              let mismatch = if result["mismatch"].is_null() {
                let (mismatch, was_pending) = errors_for(&report, &entry.consumer, &entry.description);
                pending |= was_pending;
                mismatch
              } else {
                result["mismatch"].clone()
              };
              let detail = mismatch_text(&mismatch);
              let detail = if pending {
                format!("pending: {detail}")
              } else {
                detail
              };
              (Verdict::Fail, Some(detail))
            }
            None => (Verdict::Error, Some("no result".to_string())),
          }
        }
        // From a broker, an interaction the report does not have is one the selectors did not fetch.
        None if from_broker => (Verdict::Absent, Some("not fetched".to_string())),
        None => (Verdict::Error, Some("not in the report".to_string())),
      }
    })
    .collect();
  (run, judged)
}

/// The `errors` or `pendingErrors` entry for an interaction, and whether it was pending: its
/// `interaction` text ends with ` - <description>`, and names the consumer when there is one.
fn errors_for(report: &Value, consumer: &str, description: &str) -> (Value, bool) {
  let suffix = format!(" - {description}");
  let between = format!("between {consumer} and ");
  for (list, pending) in [("errors", false), ("pendingErrors", true)] {
    let found = report[list].as_array().into_iter().flatten().find(|error| {
      error["interaction"]
        .as_str()
        .is_some_and(|text| text.ends_with(&suffix) && (consumer.is_empty() || text.contains(&between)))
    });
    if let Some(error) = found {
      return (error["mismatch"].clone(), pending);
    }
  }
  (Value::Null, false)
}

fn mismatch_text(mismatch: &Value) -> String {
  let first = mismatch["mismatches"].as_array().and_then(|all| all.first());
  match first {
    Some(first) => first["mismatch"]
      .as_str()
      .or(first["description"].as_str())
      .unwrap_or("mismatch")
      .to_string(),
    None => mismatch["message"]
      .as_str()
      .or(mismatch["type"].as_str())
      .unwrap_or("failed")
      .to_string(),
  }
}

fn run_janus(
  tools: &Tools,
  source: &Source,
  base_url: &str,
  state_url: Option<&str>,
  work: &Path,
  expected: &[Expected],
) -> (Judged, Vec<Judged>) {
  let mut command = Command::new(&tools.janus);
  command.arg("verify");
  match source {
    Source::Files(files) => {
      command.args(*files);
    }
    Source::Broker {
      url,
      provider,
      verify,
    } => broker_args(&mut command, url, provider, verify),
  }
  command.args(["--provider-url", base_url, "--output", "json"]);
  command.env("RUST_LOG", "off");
  if let Some(url) = state_url {
    let config = work.join("verifier.janus.yaml");
    let text = format!(
      "version: 1\nhooks:\n  state-setup:\n    - name: provider-states\n      run:\n        kind: http\n        url: \"{url}\"\n        format: pact-state-change\n"
    );
    if let Err(err) = std::fs::write(&config, text) {
      return everything_errs(expected, format!("{}: {err}", config.display()));
    }
    command.arg("--config").arg(&config);
  }
  let output = match command.output() {
    Ok(output) => output,
    Err(err) => {
      return everything_errs(
        expected,
        format!("could not run {}: {err}", tools.janus.display()),
      );
    }
  };
  if output.status.code() == Some(2) {
    let said = first_line(&output.stderr).unwrap_or_default();
    return everything_errs(expected, said);
  }
  let summary: Value = match serde_json::from_slice(&output.stdout) {
    Ok(summary) => summary,
    Err(err) => return everything_errs(expected, format!("the summary is not JSON: {err}")),
  };
  let run = match output.status.code() {
    Some(0) => (Verdict::Pass, None),
    Some(1) => (
      Verdict::Fail,
      summary["aborted"]["hook"]
        .as_str()
        .map(|hook| format!("aborted by the '{hook}' hook")),
    ),
    other => (Verdict::Error, Some(format!("exit status {other:?}"))),
  };

  let failures = summary["failures"].as_array().cloned().unwrap_or_default();
  let variants = &summary["variants"];
  let counted = ["verified", "failed", "state-unavailable"]
    .iter()
    .map(|name| variants[name].as_u64().unwrap_or(0))
    .sum::<u64>();
  let accounted = counted == expected.len() as u64 && variants["skipped"].as_u64().unwrap_or(0) == 0;

  let judged = expected
    .iter()
    .map(|entry| {
      let index: usize = entry
        .key
        .rsplit('/')
        .next()
        .and_then(|i| i.parse().ok())
        .unwrap_or(0);
      let failure = failures.iter().find(|failure| {
        let interaction = &failure["interaction"];
        interaction["index"].as_u64() == Some(index as u64)
          && interaction["contract"]["consumer"].as_str() == Some(entry.consumer.as_str())
      });
      match failure {
        Some(failure) => {
          let detail = failure["mismatches"]
            .as_array()
            .and_then(|all| all.first())
            .map(|first| {
              format!(
                "{}: {}",
                first["path"].as_str().unwrap_or(""),
                first["message"].as_str().unwrap_or("")
              )
            })
            .or_else(|| failure["error"]["code"].as_str().map(str::to_string))
            .or_else(|| failure["status"].as_str().map(str::to_string));
          (Verdict::Fail, detail)
        }
        None if accounted => (Verdict::Pass, None),
        None => (
          Verdict::Error,
          Some("the summary does not account for every interaction".to_string()),
        ),
      }
    })
    .collect();
  (run, judged)
}

fn everything_errs(expected: &[Expected], why: String) -> (Judged, Vec<Judged>) {
  let judged = expected
    .iter()
    .map(|_| (Verdict::Error, Some(why.clone())))
    .collect();
  ((Verdict::Error, Some(why)), judged)
}

fn first_line(bytes: &[u8]) -> Option<String> {
  String::from_utf8_lossy(bytes)
    .lines()
    .map(str::trim)
    .find(|line| !line.is_empty())
    .map(str::to_string)
}

#[cfg(test)]
mod tests {
  use super::*;
  use rstest::rstest;

  fn pact(consumer: &str, provider: &str, description: &str) -> Value {
    json!({
      "consumer": { "name": consumer }, "provider": { "name": provider },
      "interactions": [{ "description": description, "request": { "method": "GET", "path": "/" },
                         "response": { "status": 200 } }]
    })
  }

  #[rstest]
  #[case::one_provider(vec![("a.json", "c1", "p", "x"), ("b.json", "c2", "p", "y")], false, None)]
  #[case::two_providers(vec![("a.json", "c1", "p", "x"), ("b.json", "c2", "q", "y")], false, Some("one provider"))]
  #[case::shared_description(vec![("a.json", "c1", "p", "x"), ("b.json", "c2", "p", "x")], false, Some("unique"))]
  #[case::mutated(vec![("a.json", "c1", "p", "x")], true, Some("mutated"))]
  fn a_broker_fixture_is_one_whose_run_means_something(
    #[case] pacts: Vec<(&str, &str, &str, &str)>,
    #[case] mutations: bool,
    #[case] refused: Option<&str>,
  ) {
    let pacts: Vec<(String, Value)> = pacts
      .into_iter()
      .map(|(file, consumer, provider, description)| {
        (file.to_string(), pact(consumer, provider, description))
      })
      .collect();
    match (
      check_broker_fixture(&BrokerSpec::default(), &pacts, mutations),
      refused,
    ) {
      (Ok(()), None) => {}
      (Err(err), Some(why)) => assert!(err.contains(why), "{err}"),
      (outcome, _) => panic!("{outcome:?}"),
    }
  }

  #[test]
  fn a_broker_section_names_only_the_fixtures_pacts() {
    let spec = BrokerSpec {
      expected_published: BTreeMap::from([("missing.json".to_string(), Published::Pass)]),
      ..BrokerSpec::default()
    };
    let err = check_broker_fixture(&spec, &[("a.json".to_string(), pact("c", "p", "x"))], false).unwrap_err();
    assert!(err.contains("missing.json"), "{err}");
  }

  #[test]
  fn a_pending_failure_is_found_under_pending_errors_for_its_own_consumer() {
    let report = json!({
      "errors": [],
      "pendingErrors": [
        { "interaction": "Verifying a pact between web and p - read", "mismatch": { "message": "web's" } },
        { "interaction": "Verifying a pact between admin and p - read", "mismatch": { "message": "admin's" } }
      ]
    });
    let (mismatch, pending) = errors_for(&report, "admin", "read");
    assert!(pending);
    assert_eq!(mismatch["message"], json!("admin's"));
  }
}
