//! Plan task 5.5: the `janus` CLI, run as a binary — `verify`, `explain` and `upgrade` against a
//! real provider on a real socket.
//!
//! These drive the *shipped executable*, not a library function, because the surface under test is
//! the command: its flags, its output and its exit code. A CI script depends on all three, and a
//! command whose exit code is only ever asserted from inside Rust is a command nobody has run.

use pact_janus_sample_order_service::{Config, Provider, start};
use serde_json::{Value, json};
use std::path::PathBuf;
use std::process::{Command, Output};

fn janus(args: &[&str]) -> Output {
  Command::new(env!("CARGO_BIN_EXE_janus"))
    .args(args)
    .output()
    .expect("the janus binary runs")
}

fn stdout(output: &Output) -> String {
  String::from_utf8_lossy(&output.stdout).to_string()
}

fn stderr(output: &Output) -> String {
  String::from_utf8_lossy(&output.stderr).to_string()
}

fn code(output: &Output) -> i32 {
  output.status.code().unwrap_or(-1)
}

fn repo(relative: &str) -> String {
  PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    .join("..")
    .join(relative)
    .canonicalize()
    .unwrap_or_else(|err| panic!("{relative}: {err}"))
    .display()
    .to_string()
}

/// The v3 pact a consumer published for the sample provider (plan task 5.4).
fn order_pact() -> String {
  repo("samples/order-service/pacts/web-app-order-service.json")
}

fn temp(name: &str) -> String {
  let mut path = std::env::temp_dir();
  path.push(format!("janus-cli-{}-{name}", std::process::id()));
  path.display().to_string()
}

/// The provider, with auth off: what a run does to earn a credential is plan task 5.3's story and
/// `hooks.rs` tells it. Here the subject is the command.
fn provider() -> Provider {
  start(Config {
    token: None,
    ..Config::default()
  })
  .expect("the sample provider binds")
}

/// The one hook every verification of this provider needs: its own v3 state endpoint. Named after
/// the provider's own port, because these tests run in parallel and each has its own provider —
/// one shared file would point every run at whichever one wrote it last.
fn state_config(provider: &Provider) -> String {
  let port = provider.base_url().rsplit(':').next().unwrap_or("0").to_string();
  let path = temp(&format!("verifier-{port}.yaml"));
  std::fs::write(
    &path,
    format!(
      "version: 1\nhooks:\n  state-setup:\n    - name: fixtures\n      run:\n        kind: http\n        url: \"{}/_pact/provider-states\"\n        format: pact-state-change\n",
      provider.base_url()
    ),
  )
  .expect("writing the config");
  path
}

fn broken_pact(name: &str) -> String {
  let path = temp(name);
  let mut document: Value =
    serde_json::from_str(&std::fs::read_to_string(order_pact()).expect("the pact")).expect("json");
  document["interactions"][0]["response"]["body"]["status"] = json!("CANCELLED");
  std::fs::write(&path, document.to_string()).expect("writing the pact");
  path
}

// ---------------------------------------------------------------------------------------------
// verify
// ---------------------------------------------------------------------------------------------

#[test]
fn verify_replays_a_v3_pact_at_a_running_provider_and_exits_zero() {
  let provider = provider();
  let config = state_config(&provider);
  let output = janus(&[
    "verify",
    &order_pact(),
    "--provider-url",
    provider.base_url(),
    "--config",
    &config,
  ]);
  let text = stdout(&output);
  assert_eq!(code(&output), 0, "stdout: {text}\nstderr: {}", stderr(&output));
  assert!(
    text.contains("pact/3.0.0"),
    "the run says what it verified: {text}"
  );
  assert!(text.contains("VERIFIED: 2 verified, 0 failed"), "{text}");
}

/// A verification that ran and found mismatches is a *successful operation* whose subject failed
/// (protocol spec §10.2) — so it is exit 1, not exit 2, and the mismatch is on stdout where a
/// reader is already looking.
#[test]
fn verify_exits_one_and_names_the_position_when_the_provider_disagrees() {
  let provider = provider();
  let config = state_config(&provider);
  let pact = broken_pact("broken.json");
  let output = janus(&[
    "verify",
    &pact,
    "--provider-url",
    provider.base_url(),
    "--config",
    &config,
  ]);
  let text = stdout(&output);
  assert_eq!(code(&output), 1, "{text}");
  assert!(text.contains("FAILED  a request for an order"), "{text}");
  assert!(
    text.contains("$.response.body.status: Expected 'SHIPPED' to equal 'CANCELLED'"),
    "the mismatch names the position and both values: {text}"
  );
  assert!(
    !text.contains("executed plan"),
    "the executed plan is opt-in: {text}"
  );
}

/// `--explain-failures` is the hardening plan task 5.5 asks for: the executed plan for the variant
/// that actually failed, against the response the provider actually sent, printed where the
/// failure is — rather than a second command run against a values file written by hand.
#[test]
fn explain_failures_prints_the_executed_plan_after_the_failure_it_explains() {
  let provider = provider();
  let config = state_config(&provider);
  let pact = broken_pact("broken-explained.json");
  let output = janus(&[
    "verify",
    &pact,
    "--provider-url",
    provider.base_url(),
    "--config",
    &config,
    "--explain-failures",
  ]);
  let text = stdout(&output);
  assert_eq!(code(&output), 1, "{text}");
  let failure = text
    .find("FAILED  a request for an order")
    .expect("the failure line");
  let plan = text.find("--- executed plan").expect("the executed plan");
  assert!(plan > failure, "the evidence follows the verdict:\n{text}");
  assert!(
    text.contains("ERROR(Expected 'SHIPPED' to equal 'CANCELLED')"),
    "the executed tree carries the result at the node that produced it: {text}"
  );
  assert!(
    text.contains("$.response.body.status => 'SHIPPED'"),
    "and the value the provider actually sent: {text}"
  );
}

/// A narrowed run must be reported as narrowed (variant-semantics spec §5.1): an unreplayed
/// variant is not a passing one, and a summary that did not say so would read like one.
#[test]
fn a_filtered_run_says_so_in_its_summary() {
  let provider = provider();
  let config = state_config(&provider);
  let output = janus(&[
    "verify",
    &order_pact(),
    "--provider-url",
    provider.base_url(),
    "--config",
    &config,
    "--variant",
    "nothing-is-called-this",
  ]);
  let text = stdout(&output);
  assert!(text.contains("filtered run"), "{text}");
  assert!(text.contains("2 skipped"), "{text}");
}

#[test]
fn verify_reads_a_directory_and_skips_what_is_not_a_contract_or_a_pact() {
  let provider = provider();
  let config = state_config(&provider);
  let dir = temp("pacts");
  std::fs::create_dir_all(&dir).expect("a temp dir");
  std::fs::copy(order_pact(), format!("{dir}/web-app-order-service.json")).expect("copying the pact");
  // Filtered by identification, not by name: this one is neither, and handing it to the engine
  // would fail a run that has nothing wrong with it.
  std::fs::write(format!("{dir}/notes.json"), r#"{"hello":"world"}"#).expect("writing a stray file");

  let output = janus(&[
    "verify",
    &dir,
    "--provider-url",
    provider.base_url(),
    "--config",
    &config,
  ]);
  assert_eq!(code(&output), 0, "{}\n{}", stdout(&output), stderr(&output));
  assert!(stdout(&output).contains("VERIFIED"), "{}", stdout(&output));
}

#[test]
fn verify_json_prints_the_summary_document_a_script_can_read() {
  let provider = provider();
  let config = state_config(&provider);
  let output = janus(&[
    "verify",
    &order_pact(),
    "--provider-url",
    provider.base_url(),
    "--config",
    &config,
    "--json",
  ]);
  let summary: Value = serde_json::from_str(&stdout(&output)).expect("the summary is JSON");
  assert_eq!(summary["status"], json!("verified"));
  assert_eq!(summary["variants"]["verified"], json!(2));
}

/// A hook that aborts the run is the run's headline, so the text says *why* — here, the token
/// endpoint of a provider nobody started — rather than only that the hook failed.
#[test]
fn a_run_aborted_by_a_hook_says_what_the_hook_hit() {
  let port = std::net::TcpListener::bind("127.0.0.1:0")
    .and_then(|listener| listener.local_addr())
    .expect("a free port")
    .port();
  let url = format!("http://127.0.0.1:{port}");
  let config = temp("verifier-unreachable.yaml");
  std::fs::write(
    &config,
    format!(
      "version: 1\nhooks:\n  before-verification:\n    - name: auth\n      run: {{ kind: component, component: oauth2 }}\n      config:\n        token-url: \"{url}/oauth/token\"\n        client-id: janus-demo\n        client-secret: janus-demo-secret\n"
    ),
  )
  .expect("writing the config");
  let output = janus(&[
    "verify",
    &order_pact(),
    "--provider-url",
    &url,
    "--config",
    &config,
  ]);
  let text = stdout(&output);
  assert_eq!(code(&output), 1, "{text}\nstderr: {}", stderr(&output));
  assert!(
    text.contains("hook    auth at before-verification: failed\n          token-endpoint-unreachable:"),
    "the hook's line carries its error: {text}"
  );
  assert!(
    text.contains("aborted by the 'auth' hook at before-verification:\n  token-endpoint-unreachable:"),
    "the summary names the cause: {text}"
  );
  assert!(
    text.contains("0 skipped, 2 not run (of 2 variant(s) across 2 interaction(s))"),
    "what the abort left undone is counted, not dropped: {text}"
  );
}

// ---------------------------------------------------------------------------------------------
// explain
// ---------------------------------------------------------------------------------------------

/// The document says what it is (ADR 0011). A pact needs no flag, and the plan printed is design
/// 3.5's — the one a verification run would really execute.
#[test]
fn explain_compiles_a_pact_interaction_without_being_told_what_it_is() {
  let output = janus(&["explain", &order_pact()]);
  let text = stdout(&output);
  assert_eq!(code(&output), 0, "{}", stderr(&output));
  assert!(text.contains(":\"a request for an order\""), "{text}");
  assert!(text.contains("%match:equality"), "{text}");
  assert!(text.contains("$.response.body.status"), "{text}");
}

#[test]
fn explain_takes_the_interaction_at_an_index() {
  let output = janus(&["explain", &order_pact(), "--index", "1"]);
  assert!(
    stdout(&output).contains("a request for an order that does not exist"),
    "{}",
    stdout(&output)
  );
}

/// An interaction specification is the one document that cannot identify itself, so it is the one
/// that takes a flag — and the error for forgetting says so.
#[test]
fn explain_needs_a_flag_only_for_an_interaction_specification() {
  let spec = temp("spec.json");
  std::fs::write(
    &spec,
    json!({
      "description": "a request for an order",
      "parts": { "response": { "status": { "shape": "equality", "example": 200 } } }
    })
    .to_string(),
  )
  .unwrap();

  let refused = janus(&["explain", &spec]);
  assert_eq!(code(&refused), 2);
  assert!(
    stderr(&refused).contains("--spec"),
    "the error says what to do next: {}",
    stderr(&refused)
  );

  let output = janus(&["explain", &spec, "--spec"]);
  assert_eq!(code(&output), 0, "{}", stderr(&output));
  assert!(
    stdout(&output).contains("$.response.status"),
    "{}",
    stdout(&output)
  );
}

#[test]
fn explain_prints_the_structured_plan_document_on_request() {
  let output = janus(&["explain", &order_pact(), "--plan"]);
  let plan: Value = serde_json::from_str(&stdout(&output)).expect("the plan is JSON");
  assert_eq!(plan["grammar"], json!("v0"));
  assert_eq!(plan["root"]["kind"], json!("container"));
}

/// The offline executed form: a plan resolved against captured values, which is what a
/// golden-corpus case carries. Exit 1 when the values do not satisfy it — the same "the subject
/// failed" code `verify` uses.
#[test]
fn explain_executed_resolves_a_plan_against_captured_values() {
  let values = temp("values.json");
  std::fs::write(
    &values,
    json!({
      "$.request.method": "GET",
      "$.request.path": "/orders/66",
      "$.request.query": {},
      "$.response.status": 200,
      "$.response.headers.content-type": "application/json",
      "$.response.body": {
        "id": "66", "status": "SHIPPED", "shippedAt": "2026-07-30T09:00:00Z",
        "items": [{ "sku": "sku-0", "quantity": 1 }]
      }
    })
    .to_string(),
  )
  .unwrap();

  let output = janus(&["explain", &order_pact(), "--executed", &values]);
  let text = stdout(&output);
  assert_eq!(code(&output), 0, "{text}\n{}", stderr(&output));
  assert!(
    text.contains("=> BOOL(true)"),
    "the executed tree carries results: {text}"
  );

  let mut wrong: Value = serde_json::from_str(&std::fs::read_to_string(&values).unwrap()).unwrap();
  wrong["$.response.body"]["status"] = json!("CANCELLED");
  std::fs::write(&values, wrong.to_string()).unwrap();
  let failed = janus(&["explain", &order_pact(), "--executed", &values]);
  assert_eq!(code(&failed), 1, "{}", stdout(&failed));
  assert!(stdout(&failed).contains("ERROR("), "{}", stdout(&failed));
}

// ---------------------------------------------------------------------------------------------
// upgrade
// ---------------------------------------------------------------------------------------------

/// Canonical bytes (ADR 0018): compact, `$format` first, a trailing newline. Deterministic bytes
/// are what keep broker dedup and git diffs honest, so the command never pretty-prints.
#[test]
fn upgrade_writes_canonical_bytes_and_reports_its_findings() {
  let output = janus(&["upgrade", &order_pact()]);
  assert_eq!(code(&output), 0, "{}", stderr(&output));
  let contract = stdout(&output);
  assert!(
    contract.starts_with("{\"$format\":"),
    "the identification prefix falls out of ordinary serialization: {}",
    &contract[..contract.len().min(60)]
  );
  assert!(contract.ends_with('\n'));

  // Findings are grouped by what they mean, not listed flat. This pact loses nothing, so there is
  // no LOSSY group at all — the absence is the claim.
  let findings = stderr(&output);
  assert!(findings.contains("JUDGEMENT"), "{findings}");
  assert!(findings.contains("rule-narrowed"), "{findings}");
  assert!(findings.contains("NOTE"), "{findings}");
  assert!(
    !findings.contains("LOSSY"),
    "nothing was lost converting this one: {findings}"
  );

  // One that does lose something says so under its own heading: this pact carries generators, and
  // the contract format names a generator *component* nothing implements yet.
  let lossy = janus(&[
    "upgrade",
    &repo("engine/kernel/tests/fixtures/legacy-pacts/V3Consumer-ProviderStateService.json"),
  ]);
  let findings = stderr(&lossy);
  assert!(findings.contains("LOSSY"), "{findings}");
  assert!(findings.contains("generator-dropped"), "{findings}");
}

#[test]
fn upgrade_writes_to_a_file_and_can_print_one_json_document_instead() {
  let out = temp("upgraded.json");
  let output = janus(&["upgrade", &order_pact(), "--out", &out, "--quiet"]);
  assert_eq!(code(&output), 0, "{}", stderr(&output));
  let written = std::fs::read_to_string(&out).expect("the contract was written");
  assert!(written.starts_with("{\"$format\":"));

  let json = janus(&["upgrade", &order_pact(), "--json"]);
  let document: Value = serde_json::from_str(&stdout(&json)).expect("one JSON document");
  assert!(document["contract"]["interactions"].is_array());
  assert!(document["findings"].is_array());
}

/// **The migration claim, end to end through the CLI.** The same provider verifies the pact where
/// it stands (plan task 5.4) and the contract that pact upgrades into (contract-file spec §8) —
/// plan-grammar spec §4.4's two-path agreement, observed rather than asserted.
#[test]
fn a_pact_and_the_contract_it_upgrades_into_verify_the_same_provider() {
  let provider = provider();
  let config = state_config(&provider);
  let upgraded = temp("agreement.json");

  let converted = janus(&["upgrade", &order_pact(), "--out", &upgraded, "--quiet"]);
  assert_eq!(code(&converted), 0, "{}", stderr(&converted));

  for source in [order_pact(), upgraded.clone()] {
    let output = janus(&[
      "verify",
      &source,
      "--provider-url",
      provider.base_url(),
      "--config",
      &config,
    ]);
    assert_eq!(
      code(&output),
      0,
      "{source} did not verify:\n{}\n{}",
      stdout(&output),
      stderr(&output)
    );
    assert!(
      stdout(&output).contains("2 verified, 0 failed"),
      "{}",
      stdout(&output)
    );
  }
}

// ---------------------------------------------------------------------------------------------
// The command surface itself
// ---------------------------------------------------------------------------------------------

#[test]
fn an_unknown_command_or_option_is_a_command_failure_with_the_usage() {
  let unknown = janus(&["frobnicate"]);
  assert_eq!(code(&unknown), 2);
  assert!(stderr(&unknown).contains("unknown command 'frobnicate'"));
  assert!(stderr(&unknown).contains("verify"), "the usage follows");

  let bad_option = janus(&["verify", "x.json", "--nonsense"]);
  assert_eq!(code(&bad_option), 2);
  assert!(stderr(&bad_option).contains("unknown option '--nonsense'"));
}

#[test]
fn every_command_explains_itself() {
  for command in ["verify", "explain", "upgrade"] {
    let output = janus(&[command, "--help"]);
    assert_eq!(code(&output), 0);
    assert!(
      stdout(&output).starts_with(&format!("usage: janus {command}")),
      "{command}: {}",
      stdout(&output)
    );
  }
  assert!(stdout(&janus(&["version"])).contains("engine protocol v"));
}

#[test]
fn verify_without_a_provider_url_is_a_usage_error_not_a_run() {
  let output = janus(&["verify", &order_pact()]);
  assert_eq!(code(&output), 2);
  assert!(
    stderr(&output).contains("missing --provider-url"),
    "{}",
    stderr(&output)
  );
}
