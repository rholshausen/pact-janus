//! Running a case: start its provider, run both verifiers against it, and read back a verdict for
//! every interaction and for the run.
//!
//! Each verifier is run as a user runs it — `pact_verifier_cli` with `--json`, `janus verify` with
//! `--output json` — and read only from what it reports. Today's verifier reports every interaction.
//! `janus verify`'s summary lists failures, so its passes are inferred: every interaction not in
//! `failures`, provided the summary's own counts account for all of them. (S1.3 needs every
//! variant's result in the summary anyway, to publish it; until then this is the inference.)

use crate::fixture::{self, Fixture, ProviderSpec, Route, RunVerdict, StateCallback};
use crate::mutate;
use crate::provider::{STATE_PATH, ScriptedProvider};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Where the two verifiers are.
pub struct Tools {
  pub janus: PathBuf,
  pub today: PathBuf,
}

pub struct Case {
  pub id: String,
  /// The pacts, by file name, as written for the verifiers.
  pub pacts: Vec<(String, Value)>,
  pub provider: CaseProvider,
  pub expected: Option<RunVerdict>,
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
    },
  );
  Ok(cases)
}

/// What every interaction is, in the order both verifiers meet them: consumer, description, and the
/// key the comparison uses.
struct Expected {
  key: String,
  consumer: String,
  description: String,
}

pub fn run_case(case: &Case, tools: &Tools, work: &Path) -> Result<CaseResult, String> {
  let started = std::time::Instant::now();
  std::fs::create_dir_all(work).map_err(|err| format!("{}: {err}", work.display()))?;
  let mut files = Vec::new();
  let mut expected = Vec::new();
  for (p, (file, document)) in case.pacts.iter().enumerate() {
    let path = work.join(format!("{p}-{file}"));
    std::fs::write(&path, serde_json::to_vec_pretty(document).unwrap_or_default())
      .map_err(|err| format!("{}: {err}", path.display()))?;
    files.push(path);
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

  // The provider lives until both verifiers are done with it.
  let (base_url, port, state_url, _scripted, _order_service) = match &case.provider {
    CaseProvider::Scripted { routes, callback } => {
      let provider = ScriptedProvider::start(routes.clone(), *callback)?;
      let state = (*callback != StateCallback::None).then(|| format!("{}{STATE_PATH}", provider.base_url()));
      (provider.base_url(), provider.port(), state, Some(provider), None)
    }
    CaseProvider::OrderService => {
      let provider = pact_janus_sample_order_service::start(pact_janus_sample_order_service::Config {
        token: None,
        ..Default::default()
      })
      .map_err(|err| format!("starting the order-service: {err}"))?;
      let base = provider.base_url().to_string();
      let port = base
        .rsplit(':')
        .next()
        .and_then(|port| port.parse().ok())
        .unwrap_or(0);
      let state = format!("{base}/_pact/provider-states");
      (base, port, Some(state), None, Some(provider))
    }
  };

  let (today_run, today) = run_today(tools, &files, port, state_url.as_deref(), work, &expected);
  let (janus_run, janus) = run_janus(tools, &files, &base_url, state_url.as_deref(), work, &expected);

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
  })
}

type Judged = (Verdict, Option<String>);

fn run_today(
  tools: &Tools,
  files: &[PathBuf],
  port: u16,
  state_url: Option<&str>,
  work: &Path,
  expected: &[Expected],
) -> (Judged, Vec<Judged>) {
  let report_path = work.join("today.json");
  let mut command = Command::new(&tools.today);
  for file in files {
    command.arg("--file").arg(file);
  }
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
          let pending = result["pending"] == json!(true);
          match result["result"].as_str() {
            Some(ok) if ok.eq_ignore_ascii_case("ok") => {
              (Verdict::Pass, pending.then(|| "pending".to_string()))
            }
            Some(_) => {
              // Newer releases put the mismatch on the result; older ones only under `errors`.
              let mismatch = if result["mismatch"].is_null() {
                errors_for(&report, &entry.description)
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
        None => (Verdict::Error, Some("not in the report".to_string())),
      }
    })
    .collect();
  (run, judged)
}

/// The `errors` entry for an interaction: its `interaction` text ends with ` - <description>`.
fn errors_for(report: &Value, description: &str) -> Value {
  let suffix = format!(" - {description}");
  report["errors"]
    .as_array()
    .into_iter()
    .flatten()
    .find(|error| {
      error["interaction"]
        .as_str()
        .is_some_and(|text| text.ends_with(&suffix))
    })
    .map(|error| error["mismatch"].clone())
    .unwrap_or(Value::Null)
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
  files: &[PathBuf],
  base_url: &str,
  state_url: Option<&str>,
  work: &Path,
  expected: &[Expected],
) -> (Judged, Vec<Judged>) {
  let mut command = Command::new(&tools.janus);
  command
    .arg("verify")
    .args(files)
    .args(["--provider-url", base_url, "--output", "json"]);
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
