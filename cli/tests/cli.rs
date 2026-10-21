//! Plan task 5.5: the `janus` CLI, run as a binary — `verify`, `explain` and `upgrade` against a
//! real provider on a real socket — and plan task 7.4's `check`, which is the one command whose
//! subject is a *decision* over documents rather than a running provider. Plan task 8.2 adds
//! `component push|pull`, and a verification whose component comes from an OCI registry. Staged plan
//! S1.3 adds `publish`, and a verification from the Pact Broker it published to.
//!
//! These drive the *shipped executable*, not a library function, because the surface under test is
//! the command: its flags, its output and its exit code. A CI script depends on all three, and a
//! command whose exit code is only ever asserted from inside Rust is a command nobody has run.
//!
//! The cases here need something running first — a provider, a registry. Those that need nothing
//! (help, usage errors, offline commands) are trycmd cases under `tests/cmd/`. Where a whole output
//! is the CLI's own rendering it is compared whole, with snapbox; refresh those snapshots with
//! `SNAPSHOTS=overwrite cargo test -p pact_janus_cli --test cli` and read the diff.

#[path = "../../engine/component-host/tests/support/registry.rs"]
mod registry;

use pact_janus_sample_order_service::{Config, Provider, start};
use serde_json::{Value, json};
use snapbox::{assert_data_eq, str};
use std::path::PathBuf;
use std::process::{Command, Output};

/// `janus`, with no colour whatever the environment the tests run in asks for: what it prints is
/// compared as text. `explain_colours_the_plan_only_where_it_is_asked_for` asks for it.
fn janus_command() -> Command {
  let mut command = Command::new(env!("CARGO_BIN_EXE_janus"));
  command.env_remove("CLICOLOR_FORCE");
  command
}

fn janus(args: &[&str]) -> Output {
  janus_command()
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

/// A v3 pact with one interaction and no provider state, for a provider these tests stand up
/// themselves.
fn secure_orders_pact() -> String {
  let path = temp("secure-orders-pact.json");
  std::fs::write(
    &path,
    json!({
      "consumer": { "name": "web-app" },
      "provider": { "name": "secure-orders" },
      "interactions": [{
        "description": "a request for an order",
        "request": { "method": "GET", "path": "/orders/1" },
        "response": {
          "status": 200,
          "headers": { "Content-Type": "application/json" },
          "body": { "id": 1, "status": "NEW" }
        }
      }],
      "metadata": { "pactSpecification": { "version": "3.0.0" } }
    })
    .to_string(),
  )
  .expect("writing the pact");
  path
}

/// An HTTPS provider answering [`secure_orders_pact`] at `/api`, to a request that carries the
/// token, demanding a client certificate from `clients` when there is one.
fn secure_orders(
  authority: &pact_janus_test_servers::Authority,
  clients: Option<&pact_janus_test_servers::Authority>,
  token: &str,
) -> pact_janus_test_servers::TlsProvider {
  use pact_janus_test_servers::Response;
  let expected = format!("Bearer {token}");
  pact_janus_test_servers::TlsProvider::start(
    &authority.server(&["127.0.0.1", "localhost"]),
    clients,
    move |request| match (request.method.as_str(), request.target.as_str()) {
      ("GET", "/api/orders/1") if request.header("authorization") == Some(expected.as_str()) => {
        Response::json(200, r#"{"id":1,"status":"NEW"}"#)
      }
      ("GET", "/api/orders/1") => Response::json(401, "{}"),
      _ => Response::json(404, "{}"),
    },
  )
}

fn pem_file(name: &str, pem: &str) -> String {
  let path = temp(name);
  std::fs::write(&path, pem).expect("writing a PEM file");
  path
}

/// Staged plan S1.5's exit: a verification against a TLS provider that demands a client
/// certificate, reached through a proxy, with today's `--base-path` and `--header` beside it.
/// It runs with every log on: the client's private key and the header's token appear in nothing
/// it prints (ADR 0035).
#[test]
fn verify_reaches_a_tls_provider_that_demands_a_client_certificate_through_a_proxy() {
  let authority = pact_janus_test_servers::Authority::new("Provider CA");
  let token = unique("t0ken");
  let provider = secure_orders(&authority, Some(&authority), &token);
  let proxy = pact_janus_test_servers::Proxy::start();
  let client = authority.client("verifier");
  let ca = pem_file("provider-ca.pem", authority.certificate());
  let certificate = pem_file("verifier.pem", &client.certificate);
  let key = pem_file("verifier-key.pem", &client.key);
  let header = format!("Authorization=Bearer {token}");

  let output = janus(&[
    "verify",
    &secure_orders_pact(),
    "--provider-url",
    &provider.base_url,
    "--base-path",
    "/api",
    "-H",
    &header,
    "--ca-cert",
    &ca,
    "--client-cert",
    &certificate,
    "--client-key",
    &key,
    "--proxy",
    &proxy.url,
    "--log-level",
    "trace",
  ]);
  assert_eq!(code(&output), 0, "{}\n{}", stdout(&output), stderr(&output));
  assert!(
    stdout(&output).contains("VERIFIED: 1 verified, 0 failed"),
    "{}",
    stdout(&output)
  );

  let authority_of = provider.base_url.trim_start_matches("https://");
  assert_eq!(proxy.carried(), vec![format!("CONNECT {authority_of} HTTP/1.1")]);
  assert!(
    provider
      .handshakes()
      .iter()
      .all(|handshake| handshake.client_certificate),
    "{:?}",
    provider.handshakes()
  );
  let everything = format!("{}{}", stdout(&output), stderr(&output));
  let key_body = client.key.lines().nth(1).expect("a PEM key has a body");
  assert!(!everything.contains(key_body), "the client key was printed");
  assert!(!everything.contains(&token), "the header's token was printed");
}

/// A certificate nothing trusts fails each request, by name; `--disable-ssl-verification` accepts
/// it, as today's verifier's does.
#[test]
fn verify_trusts_a_provider_certificate_only_when_told_to() {
  let authority = pact_janus_test_servers::Authority::new("Untrusted CA");
  let token = unique("t0ken");
  let provider = secure_orders(&authority, None, &token);
  let header = format!("Authorization=Bearer {token}");
  let args = [
    "verify",
    &secure_orders_pact(),
    "--provider-url",
    &provider.base_url,
    "--base-path",
    "api/",
    "--header",
    &header,
  ];

  let refused = janus(&args);
  assert_eq!(code(&refused), 1, "{}\n{}", stdout(&refused), stderr(&refused));
  assert!(
    stdout(&refused).contains("could not connect") && stdout(&refused).contains("certificate"),
    "{}",
    stdout(&refused)
  );

  let mut insecure = args.to_vec();
  insecure.push("--disable-ssl-verification");
  let accepted = janus(&insecure);
  assert_eq!(code(&accepted), 0, "{}\n{}", stdout(&accepted), stderr(&accepted));
}

/// A provider that speaks only HTTP/1.1, asked for HTTP/2: each request fails, and says why on
/// one line, with the component's own code and no JSON, and nothing is logged at the default level
/// that the result does not already say. (The sample provider never answers HTTP/2's preface, so
/// the request times out; a provider that answers it with HTTP/1.1 fails on a bad frame instead.)
#[test]
fn a_request_the_provider_cannot_answer_is_reported_on_one_line() {
  let provider = provider();
  let config = state_config(&provider);
  let output = janus(&[
    "verify",
    &order_pact(),
    "--provider-url",
    provider.base_url(),
    "--config",
    &config,
    "--http-version",
    "2",
    "--request-timeout",
    "500",
  ]);
  assert_eq!(code(&output), 1, "{}\n{}", stdout(&output), stderr(&output));
  let text = stdout(&output);
  let failures: Vec<&str> = text
    .lines()
    .filter(|line| line.contains("component-failed"))
    .collect();
  assert_eq!(failures.len(), 2, "{text}");
  assert!(
    failures
      .iter()
      .all(|line| line.contains("no answer within 500 ms") && line.ends_with("[transport-failed]")),
    "{text}"
  );
  assert!(!text.contains(r#"{"component""#), "{text}");
  assert!(!stderr(&output).contains("WARN"), "{}", stderr(&output));
}

/// Without `--proxy`, the environment's proxy is used, as today's verifier and curl use it.
#[test]
fn verify_reaches_the_provider_through_the_environments_proxy() {
  let authority = pact_janus_test_servers::Authority::new("Provider CA");
  let token = unique("t0ken");
  let provider = secure_orders(&authority, None, &token);
  let proxy = pact_janus_test_servers::Proxy::start();
  let ca = pem_file("environment-ca.pem", authority.certificate());
  let header = format!("Authorization=Bearer {token}");
  let output = janus_command()
    .args([
      "verify",
      &secure_orders_pact(),
      "--provider-url",
      &provider.base_url,
      "--base-path",
      "/api",
      "-H",
      &header,
      "--ca-cert",
      &ca,
    ])
    .env("HTTPS_PROXY", &proxy.url)
    .env_remove("NO_PROXY")
    .env_remove("no_proxy")
    .output()
    .expect("the janus binary runs");
  assert_eq!(code(&output), 0, "{}\n{}", stdout(&output), stderr(&output));
  assert_eq!(proxy.carried().len(), 1, "{:?}", proxy.carried());
}

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
  assert_eq!(code(&output), 0, "stderr: {}", stderr(&output));
  // The run says what it verified, one line per variant, and the verdict.
  assert_data_eq!(
    stdout(&output),
    str![[r#"
Verifying 2 interaction(s) in 1 document(s) [pact/3.0.0] against order-service

  ok      a request for an order [base]
  ok      a request for an order that does not exist [base]

VERIFIED: 2 verified, 0 failed, 0 state-unavailable, 0 skipped (of 2 variant(s) across 2 interaction(s))

"#]]
  );
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
  assert_eq!(code(&output), 1, "stderr: {}", stderr(&output));
  // The mismatch names the position and both values; the executed plan is opt-in, so absent.
  assert_data_eq!(
    stdout(&output),
    str![[r#"
Verifying 2 interaction(s) in 1 document(s) [pact/3.0.0] against order-service

  FAILED  a request for an order [base]
          $.response.body.status: Expected 'SHIPPED' to equal 'CANCELLED'
  ok      a request for an order that does not exist [base]

FAILED: 1 verified, 1 failed, 0 state-unavailable, 0 skipped (of 2 variant(s) across 2 interaction(s))

"#]]
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

/// A contract's plan is compiled from both parts, but only the response is scored, and only the
/// reply is there to resolve against. The executed plan is the scored response, as a pact's is,
/// and not a request half whose every check fails on a value it was never given.
#[test]
fn explain_failures_shows_only_the_response_a_contract_was_scored_on() {
  let provider = provider();
  let config = state_config(&provider);
  let pact = broken_pact("broken-upgraded.json");
  let contract = temp("broken-upgraded.janus.json");
  let upgraded = janus(&["upgrade", &pact, "--write", &contract, "--quiet"]);
  assert_eq!(code(&upgraded), 0, "stderr: {}", stderr(&upgraded));
  let output = janus(&[
    "verify",
    &contract,
    "--provider-url",
    provider.base_url(),
    "--config",
    &config,
    "--explain-failures",
  ]);
  let text = stdout(&output);
  assert_eq!(code(&output), 1, "{text}");
  assert!(
    text.contains("$.response.body.status => 'SHIPPED'"),
    "the executed plan carries the response: {text}"
  );
  assert!(
    !text.contains(":request") && !text.contains("<absent>"),
    "and no request half resolved against nothing: {text}"
  );
}

/// A narrowed run must be reported as narrowed (variant-semantics spec §5.1): an unreplayed
/// variant is not a passing one, and a summary that did not say so would read like one.
///
/// The snapshot records today's gap: a filter that matches nothing still reports `VERIFIED` and
/// exits 0. Staged plan S1.17 changes both, and this snapshot with them.
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
  assert_data_eq!(
    stdout(&output),
    str![[r#"
Verifying 2 interaction(s) in 1 document(s) [pact/3.0.0] against order-service
  (filtered run: not every recorded variant will be replayed)


VERIFIED: 0 verified, 0 failed, 0 state-unavailable, 2 skipped (of 2 variant(s) across 2 interaction(s))
This was a filtered run: unreplayed variants are not passing ones.

"#]]
  );
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
    "--output",
    "json",
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
  assert_eq!(code(&output), 1, "stderr: {}", stderr(&output));
  // The hook's line carries its error, the summary names the cause, and what the abort left undone
  // is counted, not dropped.
  assert_data_eq!(
    stdout(&output),
    str![[r#"
Verifying 2 interaction(s) in 1 document(s) [pact/3.0.0] against order-service

  hook    auth at before-verification: failed
          token-endpoint-unreachable: http://127.0.0.1:[..]/oauth/token: [..]

FAILED: 0 verified, 0 failed, 0 state-unavailable, 0 skipped, 2 not run (of 2 variant(s) across 2 interaction(s))
The run was aborted by the 'auth' hook at before-verification:
  token-endpoint-unreachable: http://127.0.0.1:[..]/oauth/token: [..]

"#]]
  );
}

/// `janus ... | head` closes stdout early. The command still runs to its end and exits with its own
/// answer, rather than panicking on the write that found the pipe gone.
#[test]
fn a_reader_that_stops_reading_is_not_a_crash() {
  let mut child = janus_command()
    .args(["explain", &order_pact(), "--plan"])
    .stdout(std::process::Stdio::piped())
    .stderr(std::process::Stdio::piped())
    .spawn()
    .expect("the janus binary runs");
  drop(child.stdout.take());
  let output = child.wait_with_output().expect("janus exits");
  assert_eq!(code(&output), 0, "{}", stderr(&output));
  assert!(!stderr(&output).contains("panicked"), "{}", stderr(&output));
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

/// Plan-grammar spec §3.3: the plan is coloured where colour is wanted, and the colour is all that
/// differs. A test has no terminal, so it asks with `CLICOLOR_FORCE`, as a CI log does; `NO_COLOR`
/// still wins.
#[test]
fn explain_colours_the_plan_only_where_it_is_asked_for() {
  let explain = |env: &[(&str, &str)]| {
    let mut command = janus_command();
    command.args(["explain", &order_pact()]).env_remove("NO_COLOR");
    for (key, value) in env {
      command.env(key, value);
    }
    stdout(&command.output().expect("the janus binary runs"))
  };
  let plain = explain(&[]);
  let coloured = explain(&[("CLICOLOR_FORCE", "1")]);
  assert!(!plain.contains('\u{1b}'), "{plain}");
  assert!(
    coloured.contains("\u{1b}[1m\u{1b}[36m%match:equality\u{1b}[0m"),
    "{coloured:?}"
  );
  assert_eq!(without_styling(&coloured), plain);
  assert_eq!(explain(&[("CLICOLOR_FORCE", "1"), ("NO_COLOR", "1")]), plain);
}

/// `text` without its ANSI SGR sequences, the only ones the CLI writes.
fn without_styling(text: &str) -> String {
  let mut out = String::new();
  let mut rest = text;
  while let Some(at) = rest.find("\u{1b}[") {
    out.push_str(&rest[..at]);
    let after = &rest[at + 2..];
    let end = after.find('m').map_or(after.len(), |end| end + 1);
    rest = &after[end..];
  }
  out.push_str(rest);
  out
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
  assert_eq!(plan["grammar"], json!("v1"));
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

  // Findings are grouped by what they mean, not listed flat. The one thing this pact loses is what
  // every HTTP request loses — its closed query, which a shape cannot close (ADR 0007).
  let findings = stderr(&output);
  assert!(findings.contains("LOSSY"), "{findings}");
  assert!(findings.contains("request-query-opened"), "{findings}");
  assert!(
    !findings.contains("generator-dropped"),
    "this pact has no generators to lose: {findings}"
  );
  assert!(findings.contains("JUDGEMENT"), "{findings}");
  assert!(findings.contains("rule-narrowed"), "{findings}");
  assert!(findings.contains("NOTE"), "{findings}");

  // One that loses more says so under the same heading: this pact carries generators, and the
  // contract format names a generator *component* nothing implements yet.
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
  let output = janus(&["upgrade", &order_pact(), "--write", &out, "--quiet"]);
  assert_eq!(code(&output), 0, "{}", stderr(&output));
  let written = std::fs::read_to_string(&out).expect("the contract was written");
  assert!(written.starts_with("{\"$format\":"));

  let json = janus(&["upgrade", &order_pact(), "--output", "json"]);
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

  let converted = janus(&["upgrade", &order_pact(), "--write", &upgraded, "--quiet"]);
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
// publish, and verify from a broker (staged plan S1.3)
// ---------------------------------------------------------------------------------------------

/// The Pact Broker `JANUS_PACT_BROKER` names: CI's `verifier-diff` job starts the pinned one
/// (`.github/scripts/pact-broker.sh start`). With none, the tests that need it say so and pass.
fn pact_broker() -> Option<String> {
  std::env::var("JANUS_PACT_BROKER")
    .ok()
    .filter(|url| !url.is_empty())
}

/// A name no earlier run left in the broker, which keeps what it already holds.
fn unique(name: &str) -> String {
  let nanos = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .map(|elapsed| elapsed.as_nanos())
    .unwrap_or_default();
  format!("{name}-{}-{nanos}", std::process::id())
}

/// The loop a consumer and a provider run between them through a broker: the consumer publishes a
/// v3 pact and the Janus contract it upgrades into, and the provider verifies both, fetched back.
/// What the broker serves is not what was published (it adds its links, and an `_id` to every
/// interaction), so this is the test that a Janus contract survives the round trip.
#[test]
fn a_pact_and_a_contract_published_to_a_broker_verify_when_fetched_back() {
  let Some(broker) = pact_broker() else {
    eprintln!("skipped: JANUS_PACT_BROKER names no Pact Broker");
    return;
  };
  let provider = provider();
  let config = state_config(&provider);
  let provider_name = unique("order-service");
  let pact_consumer = unique("web-app");
  let contract_consumer = unique("web-app-janus");

  let mut pact: Value =
    serde_json::from_str(&std::fs::read_to_string(order_pact()).expect("the pact")).expect("json");
  pact["consumer"]["name"] = json!(pact_consumer);
  pact["provider"]["name"] = json!(provider_name);
  let pact_file = temp("published-pact.json");
  std::fs::write(&pact_file, pact.to_string()).expect("writing the pact");
  let contract_file = temp("published-contract.json");
  let converted = janus(&["upgrade", &pact_file, "--write", &contract_file, "--quiet"]);
  assert_eq!(code(&converted), 0, "{}", stderr(&converted));
  let mut contract: Value =
    serde_json::from_str(&std::fs::read_to_string(&contract_file).expect("the contract")).expect("json");
  contract["consumer"]["name"] = json!(contract_consumer);
  std::fs::write(&contract_file, contract.to_string()).expect("writing the contract");

  let published = janus(&[
    "publish",
    &pact_file,
    &contract_file,
    "--consumer-app-version",
    "1.0.0",
    "--branch",
    "main",
    "--broker-base-url",
    &broker,
  ]);
  assert_eq!(
    code(&published),
    0,
    "{}\n{}",
    stdout(&published),
    stderr(&published)
  );
  for consumer in [&pact_consumer, &contract_consumer] {
    assert!(
      stdout(&published).contains(&format!("published {consumer} -> {provider_name}")),
      "{}",
      stdout(&published)
    );
  }

  let verified = janus(&[
    "verify",
    "--broker-url",
    &broker,
    "--provider-name",
    &provider_name,
    "--consumer-version-selectors",
    r#"{"branch":"main"}"#,
    "--provider-url",
    provider.base_url(),
    "--config",
    &config,
    "--output",
    "json",
  ]);
  let summary: Value = serde_json::from_str(&stdout(&verified))
    .unwrap_or_else(|err| panic!("{err}: {}\n{}", stdout(&verified), stderr(&verified)));
  assert_eq!(code(&verified), 0, "{summary:#}\n{}", stderr(&verified));
  let mut consumers: Vec<&str> = summary["pairs"]
    .as_array()
    .expect("pairs")
    .iter()
    .map(|pair| {
      assert_eq!(pair["status"], "verified", "{pair}");
      pair["consumer"].as_str().unwrap()
    })
    .collect();
  consumers.sort();
  assert_eq!(
    consumers,
    vec![pact_consumer.as_str(), contract_consumer.as_str()]
  );
}

/// The broker behind a door: a proxy in front of `broker` that forwards a request only when it
/// carries `authorization`, and without it, and answers 401 to anything else. The OSS broker has
/// basic authentication and no bearer tokens, so this is how one broker is put behind both.
fn behind_a_door(broker: &str, authorization: String) -> String {
  let server = tiny_http::Server::http("127.0.0.1:0").expect("the door binds");
  let url = format!("http://{}", server.server_addr().to_ip().expect("an IP address"));
  let broker = broker.trim_end_matches('/').to_string();
  std::thread::spawn(move || {
    let agent: ureq::Agent = ureq::Agent::config_builder()
      .http_status_as_error(false)
      .build()
      .into();
    for mut request in server.incoming_requests() {
      let header = |name: &str| {
        request
          .headers()
          .iter()
          .find(|header| header.field.as_str().as_str().eq_ignore_ascii_case(name))
          .map(|header| header.value.as_str().to_string())
      };
      if header("authorization").as_deref() != Some(authorization.as_str()) {
        let _ = request
          .respond(tiny_http::Response::from_string(r#"{"error":"Unauthorized"}"#).with_status_code(401));
        continue;
      }
      let mut forwarded = ureq::http::Request::builder()
        .method(request.method().as_str())
        .uri(format!("{broker}{}", request.url()));
      for name in ["accept", "content-type"] {
        if let Some(value) = header(name) {
          forwarded = forwarded.header(name, value);
        }
      }
      let mut body = Vec::new();
      let _ = request.as_reader().read_to_end(&mut body);
      let (status, content_type, text) = match agent.run(forwarded.body(body).expect("a request")) {
        Ok(mut response) => (
          response.status().as_u16(),
          response
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok())
            .unwrap_or("application/json")
            .to_string(),
          response.body_mut().read_to_string().unwrap_or_default(),
        ),
        Err(err) => (502, "text/plain".to_string(), err.to_string()),
      };
      let _ = request.respond(
        tiny_http::Response::from_string(text)
          .with_status_code(status)
          .with_header(tiny_http::Header::from_bytes("Content-Type", content_type).expect("a header")),
      );
    }
  });
  url
}

/// ADR 0032 decision 5, end to end: a broker credential reaches the broker and appears nowhere
/// else. Each command runs with every log on, so what it prints is every event, the summary,
/// every error, every frame the engine saw and every log line; none may carry the secret, nor the
/// `Authorization` header it became. Basic credentials arrive by flag; the token arrives by
/// today's environment variable.
#[rstest::rstest]
#[case::basic("basic")]
#[case::bearer("bearer")]
fn a_broker_credential_reaches_the_broker_and_appears_nowhere_else(#[case] kind: &str) {
  use base64::Engine as _;
  let Some(broker) = pact_broker() else {
    eprintln!("skipped: JANUS_PACT_BROKER names no Pact Broker");
    return;
  };
  let user = "janus-user";
  let secret = unique("s3cret");
  let wrong = unique("not-the-s3cret");
  let header = |secret: &str| match kind {
    "basic" => format!(
      "Basic {}",
      base64::engine::general_purpose::STANDARD.encode(format!("{user}:{secret}"))
    ),
    _ => format!("Bearer {secret}"),
  };
  let door = behind_a_door(&broker, header(&secret));
  let run = |args: &[&str], secret: &str| -> Output {
    let mut command = janus_command();
    command
      .args(args)
      .args(["--log-level", "trace"])
      .env_remove("PACT_BROKER_TOKEN")
      .env_remove("PACT_BROKER_USERNAME")
      .env_remove("PACT_BROKER_PASSWORD");
    let publishing = args.first() == Some(&"publish");
    match (kind, publishing) {
      ("basic", true) => command.args(["--broker-username", user, "--broker-password", secret]),
      ("basic", false) => command.args(["--user", user, "--password", secret]),
      _ => command.env("PACT_BROKER_TOKEN", secret),
    };
    let output = command.output().expect("the janus binary runs");
    let everything = format!("{}{}", stdout(&output), stderr(&output));
    for leaked in [secret.to_string(), header(secret)] {
      assert!(
        !everything.contains(&leaked),
        "janus {} printed a credential:
{everything}",
        args[0]
      );
    }
    output
  };

  let provider = provider();
  let config = state_config(&provider);
  let provider_name = unique("order-service");
  let mut pact: Value =
    serde_json::from_str(&std::fs::read_to_string(order_pact()).expect("the pact")).expect("json");
  pact["consumer"]["name"] = json!(unique("web-app"));
  pact["provider"]["name"] = json!(provider_name);
  let pact_file = temp(&format!("{kind}-door-pact.json"));
  std::fs::write(&pact_file, pact.to_string()).expect("writing the pact");

  let published = run(
    &[
      "publish",
      &pact_file,
      "--consumer-app-version",
      "1.0.0",
      "--branch",
      "main",
      "--broker-base-url",
      &door,
    ],
    &secret,
  );
  assert_eq!(
    code(&published),
    0,
    "{}\n{}",
    stdout(&published),
    stderr(&published)
  );

  let verify = |output: &str| {
    vec![
      "verify".to_string(),
      "--broker-url".to_string(),
      door.clone(),
      "--provider-name".to_string(),
      provider_name.clone(),
      "--consumer-version-selectors".to_string(),
      r#"{"branch":"main"}"#.to_string(),
      "--provider-url".to_string(),
      provider.base_url().to_string(),
      "--config".to_string(),
      config.clone(),
      "--provider-version".to_string(),
      "2.0.0".to_string(),
      "--publish".to_string(),
      "--output".to_string(),
      output.to_string(),
    ]
  };
  for output in ["text", "json"] {
    let args = verify(output);
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let verified = run(&args, &secret);
    assert_eq!(code(&verified), 0, "{}\n{}", stdout(&verified), stderr(&verified));
    assert!(stdout(&verified).contains("published"), "{}", stdout(&verified));
  }

  // The wrong credential is refused, and is not repeated either.
  let args = verify("text");
  let args: Vec<&str> = args.iter().map(String::as_str).collect();
  let refused = run(&args, &wrong);
  assert_eq!(code(&refused), 1, "{}\n{}", stdout(&refused), stderr(&refused));
  assert!(stdout(&refused).contains("with 401"), "{}", stdout(&refused));
}

#[test]
fn publishing_to_a_broker_nobody_answers_for_fails_and_names_it() {
  let output = janus(&[
    "publish",
    &order_pact(),
    "--consumer-app-version",
    "1.0.0",
    "--broker-base-url",
    "http://u:hunter2@127.0.0.1:1",
  ]);
  assert_eq!(code(&output), 2, "{}", stdout(&output));
  assert!(
    stdout(&output).contains("FAILED    web-app -> order-service"),
    "{}",
    stdout(&output)
  );
  assert!(
    stdout(&output).contains("broker-unreachable"),
    "{}",
    stdout(&output)
  );
  assert!(
    stderr(&output).contains("janus publish: the broker did not accept web-app's contracts"),
    "{}",
    stderr(&output)
  );
  assert!(
    !stdout(&output).contains("hunter2") && !stderr(&output).contains("hunter2"),
    "a credential in the output"
  );
}

// ---------------------------------------------------------------------------------------------
// check (plan task 7.4)
// ---------------------------------------------------------------------------------------------

/// The provider shape the sample provider's own tests recorded (plan task 7.2), checked in beside
/// its pacts and asserted against the recorder in `engine/kernel/tests/provider_shape_record.rs`.
fn order_shapes() -> String {
  repo("samples/order-service/shapes")
}

/// The consumer side of M5 as the RFC writes it: a contract that declares two statuses, so the
/// provider's third shows up as the RFC's own two-line finding. The pact beside the provider
/// froze one example as `equality` instead, which is a true finding about a narrower claim but
/// not the sketch.
///
/// Written once per test process: the tests that read it run in parallel, and a rewrite truncates
/// the file under a `janus check` another test has just started on it.
fn order_consumer_contract() -> String {
  static CONTRACT: std::sync::OnceLock<String> = std::sync::OnceLock::new();
  CONTRACT.get_or_init(write_order_consumer_contract).clone()
}

fn write_order_consumer_contract() -> String {
  let path = temp("web-app.janus.json");
  let document = json!({ "$format": "janus-contract/1",
    "consumer": { "name": "web-app" },
    "provider": { "name": "order-service" },
    "interactions": [
      { "description": "a request for an order",
        "states": [ { "name": "an order exists" } ],
        "parts": { "response": { "body": {
          "shape": "object",
          "members": {
            "id": { "shape": "string", "example": "66" },
            "status": { "shape": "any-of", "options": ["PENDING", "SHIPPED"], "example": "PENDING" },
            "items": { "shape": "each-like", "min": 0,
                       "items": { "shape": "object", "members": {
                         "sku": { "shape": "string", "example": "sku-0" },
                         "quantity": { "shape": "integer", "example": 1 } } } } } } } },
        "selection": { "variants": [], "report": {} } } ] });
  std::fs::write(&path, document.to_string()).expect("writing the contract");
  path
}

fn policy_file(name: &str, body: &str) -> String {
  let path = temp(name);
  std::fs::write(&path, body).expect("writing the policy");
  path
}

/// M5, end to end and in one command: the shape the provider recorded, the contract the consumer
/// declared, and the RFC's report — reported exactly as the RFC sketches it.
#[test]
fn check_reports_the_undeclared_variance_the_way_the_rfc_sketches_it() {
  let output = janus(&[
    "check",
    &order_consumer_contract(),
    "--provider-shape",
    &order_shapes(),
  ]);
  let text = stdout(&output);
  for line in [
    "✗ web-app is not compatible with order-service",
    "  interaction 'a request for an order', response body $.status:",
    "    provider may produce: 'CANCELLED' | 'PENDING' | 'SHIPPED'",
    "    consumer has only tested: 'PENDING' | 'SHIPPED'",
  ] {
    assert!(text.contains(line), "missing {line:?} in:\n{text}");
  }
  assert_eq!(code(&output), 0, "a finding warns by default (ADR 0016): {text}");
  assert!(
    text.contains("no verification result was supplied"),
    "and a pair nobody replayed is not a passing one: {text}"
  );
}

/// The whole loop as CI would run it: verify, keep the summary, then decide with it. The policy
/// blocks on findings, so the answer is no — and the exit code says so without the command having
/// failed.
#[test]
fn check_combines_a_verification_result_with_the_findings_and_exits_one_when_blocked() {
  let provider = provider();
  let config = state_config(&provider);
  let verification = temp("verification.json");
  let verified = janus(&[
    "verify",
    &order_pact(),
    "--provider-url",
    provider.base_url(),
    "--config",
    &config,
    "--output",
    "json",
  ]);
  assert_eq!(code(&verified), 0, "{}", stderr(&verified));
  std::fs::write(&verification, stdout(&verified)).expect("writing the summary");

  let policy = policy_file("policy-block.yaml", "on-finding: block\non-review: warn\n");
  let output = janus(&[
    "check",
    &order_consumer_contract(),
    "--provider-shape",
    &order_shapes(),
    "--verification",
    &verification,
    "--policy",
    &policy,
    "--as-of",
    "2026-09-22",
  ]);
  let text = stdout(&output);
  assert_eq!(code(&output), 1, "{text}\n{}", stderr(&output));
  assert!(
    text.contains("verification: verified (2 of 2 variant(s))"),
    "the verification half of the answer is on the page: {text}"
  );
  assert!(text.contains("=> BLOCK: web-app -> order-service"), "{text}");
  assert!(
    text.contains("BLOCK: 1 pair(s) blocked"),
    "and the run's own verdict: {text}"
  );
}

/// An exemption with a reason turns the same documents into a deploy — and says what it silenced
/// and until when, which is the record design 2.8 §7.2 requires.
#[test]
fn an_exemption_lets_the_same_documents_deploy_and_says_what_it_silenced() {
  let policy = policy_file(
    "policy-exempt.yaml",
    "on-finding: block\nexemptions:\n  - path: response.body.status\n    reason: \"CANCELLED ships next sprint; ORD-451\"\n    expires: \"2026-12-01\"\n",
  );
  let args = [
    "check".to_string(),
    order_consumer_contract(),
    "--provider-shape".to_string(),
    order_shapes(),
    "--policy".to_string(),
    policy,
    "--as-of".to_string(),
    "2026-09-22".to_string(),
  ];
  let borrowed: Vec<&str> = args.iter().map(String::as_str).collect();
  let output = janus(&borrowed);
  let text = stdout(&output);
  assert_eq!(code(&output), 0, "{text}");
  assert!(
    text.contains("~ interaction 'a request for an order', response body $.status:"),
    "the exempted finding is still printed, marked: {text}"
  );
  assert!(
    text.contains("exempted until 2026-12-01: CANCELLED ships next sprint; ORD-451"),
    "{text}"
  );

  // The same policy, a year later: the exemption has lapsed and the finding is decided again.
  let mut lapsed = borrowed.clone();
  let index = lapsed.len() - 1;
  lapsed[index] = "2027-09-22";
  let output = janus(&lapsed);
  let text = stdout(&output);
  assert_eq!(code(&output), 1, "{text}");
  assert!(text.contains("lapsed exemption (2026-12-01)"), "{text}");
}

#[test]
fn check_json_prints_the_compatibility_report_a_script_can_read() {
  let output = janus(&[
    "check",
    &order_consumer_contract(),
    "--provider-shape",
    &order_shapes(),
    "--output",
    "json",
  ]);
  assert_eq!(code(&output), 0);
  let report: Value = serde_json::from_str(&stdout(&output)).expect("one JSON document");
  assert_eq!(report["$format"], json!("janus-compatibility-report/1"));
  assert_eq!(report["decision"], json!("warn"));
  assert_eq!(report["summary"]["pairs"], json!(1));
  assert_eq!(report["pairs"][0]["subsumption"]["verdict"], json!("no"));
  assert_eq!(report["pairs"][0]["format"], json!("janus-contract/1"));
}

/// A provider that published nothing gets today's semantics — and the command says so on stderr,
/// because "no shape was supplied for this provider" is the CLI's own half of the answer: the
/// engine was never handed a file to miss.
#[test]
fn check_without_a_provider_shape_says_which_provider_published_nothing() {
  let verification = temp("verification-noshape.json");
  let provider = provider();
  let config = state_config(&provider);
  let verified = janus(&[
    "verify",
    &order_pact(),
    "--provider-url",
    provider.base_url(),
    "--config",
    &config,
    "--output",
    "json",
  ]);
  std::fs::write(&verification, stdout(&verified)).expect("writing the summary");

  let output = janus(&[
    "check",
    &order_pact(),
    "--verification",
    &verification,
    "--on-finding",
    "block",
  ]);
  let text = stdout(&output);
  assert_eq!(code(&output), 0, "replay-only semantics is a pass: {text}");
  assert!(
    text.contains("no published shape matched any interaction"),
    "{text}"
  );
  assert!(
    stderr(&output).contains("no provider shape supplied for 'order-service'"),
    "{}",
    stderr(&output)
  );
}

// ---------------------------------------------------------------------------------------------
// components (plan task 8.1)
// ---------------------------------------------------------------------------------------------

/// The out-of-tree CSV component, built for `wasm32-wasip2` by the test itself so it never runs a
/// stale one. Its crate depends on nothing in this workspace.
fn csv_component() -> String {
  static WASM: std::sync::OnceLock<String> = std::sync::OnceLock::new();
  WASM
    .get_or_init(|| {
      let dir = PathBuf::from(repo("third-party/janus-csv"));
      let status = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string()))
        .args(["build", "--release", "--target", "wasm32-wasip2"])
        .current_dir(&dir)
        .env_remove("CARGO_TARGET_DIR")
        .status()
        .expect("cargo runs");
      assert!(status.success(), "building the CSV component");
      dir
        .join("target/wasm32-wasip2/release/janus_csv.wasm")
        .display()
        .to_string()
    })
    .clone()
}

/// A consumer's contract for the orders export: one interaction, one recorded variant, a body the
/// interaction declares as `text/csv` (contract spec §5.5) and records as the document it decoded.
fn csv_contract() -> String {
  let path = temp("reporting.janus.json");
  let row = json!({ "id": "66", "status": "PENDING", "items": "1" });
  let document = json!({ "$format": "janus-contract/1",
    "consumer": { "name": "reporting" },
    "provider": { "name": "order-service" },
    "interactions": [
      { "description": "the orders export",
        "transport": { "kind": "http", "mode": "passive" },
        "states": [ { "name": "an order exists", "params": { "id": "66" } } ],
        "parts": {
          "request": { "method": { "shape": "equality", "example": "GET" },
                       "path": { "shape": "equality", "example": "/orders.csv" } },
          "response": { "status": { "shape": "equality", "example": 200 },
                        "body": { "shape": "each-like", "min": 1, "max": 1,
                                  "items": { "shape": "object", "members": {
                                    "id": { "shape": "string", "example": "66" },
                                    "status": { "shape": "string", "example": "PENDING" },
                                    "items": { "shape": "regex", "pattern": "^[0-9]+$", "example": "1" } } } } } },
        "content-types": { "response": { "body": "text/csv" } },
        "requires": [ { "component": "content/csv", "min-version": 1 } ],
        "selection": {
          "variants": [ { "id": "base", "origin": "base", "assignment": [],
            "states": [ { "name": "an order exists", "params": { "id": "66" } } ],
            "parts": {
              "request": { "method": { "content": "GET" }, "path": { "content": "/orders.csv" } },
              "response": { "status": { "content": 200 },
                            "body": { "content": [row], "content-type": "text/csv" } } } } ],
          "report": {} } } ] });
  std::fs::write(&path, document.to_string()).expect("writing the contract");
  path
}

/// The sample's state hook, and the component — one file, as a project writes it (lifecycle-hooks
/// spec §6.1). The component's path is relative to the file, which is how the loader resolves it.
fn csv_config(provider: &Provider, component: &str) -> String {
  let port = provider.base_url().rsplit(':').next().unwrap_or("0").to_string();
  let path = temp(&format!("verifier-csv-{port}.yaml"));
  // Single-quoted in the YAML below: a Windows path in a double-quoted scalar is a run of escapes.
  let reference = pathdiff(component, &std::env::temp_dir());
  std::fs::write(
    &path,
    format!(
      "version: 1\ncomponents:\n  - name: csv\n    source: {{ kind: file, reference: '{reference}' }}\nhooks:\n  state-setup:\n    - name: fixtures\n      run:\n        kind: http\n        url: \"{}/_pact/provider-states\"\n        format: pact-state-change\n",
      provider.base_url()
    ),
  )
  .expect("writing the config");
  path
}

/// `target` relative to `base`, the way a project would write a path in a file that lives in `base`.
fn pathdiff(target: &str, base: &std::path::Path) -> String {
  let target = PathBuf::from(target);
  let base = base.canonicalize().expect("the temp dir exists");
  let common = target
    .components()
    .zip(base.components())
    .take_while(|(a, b)| a == b)
    .count();
  let ups = base.components().count() - common;
  let rest: PathBuf = target.components().skip(common).collect();
  let mut relative = PathBuf::new();
  for _ in 0..ups {
    relative.push("..");
  }
  relative.join(rest).display().to_string()
}

#[test]
fn verify_loads_a_declared_component_and_verifies_a_csv_contract() {
  let provider = provider();
  let config = csv_config(&provider, &csv_component());
  let output = janus(&[
    "verify",
    &csv_contract(),
    "--provider-url",
    provider.base_url(),
    "--config",
    &config,
  ]);
  let text = stdout(&output);
  assert_eq!(code(&output), 0, "stdout: {text}\nstderr: {}", stderr(&output));
  assert!(text.contains("VERIFIED: 1 verified, 0 failed"), "{text}");
}

#[test]
fn verify_without_the_component_fails_before_the_run_naming_what_is_missing() {
  let provider = provider();
  let config = state_config(&provider);
  let output = janus(&[
    "verify",
    &csv_contract(),
    "--provider-url",
    provider.base_url(),
    "--config",
    &config,
  ]);
  assert_eq!(code(&output), 2, "the command could not run: {}", stderr(&output));
  let text = stderr(&output);
  assert!(text.contains("component-unavailable"), "{text}");
  assert!(text.contains("content/csv"), "{text}");
}

// ---------------------------------------------------------------------------------------------
// OCI distribution (plan task 8.2)
// ---------------------------------------------------------------------------------------------

/// `janus`, with its own component cache: what is or is not already cached is the subject here.
fn janus_cached(cache: &str, args: &[&str]) -> Output {
  janus_command()
    .args(args)
    .env("JANUS_COMPONENT_CACHE", cache)
    .output()
    .expect("the janus binary runs")
}

/// The sample's state hook, and the component by OCI reference and digest.
fn oci_config(provider: &Provider, reference: &str, digest: &str) -> String {
  let port = provider.base_url().rsplit(':').next().unwrap_or("0").to_string();
  let path = temp(&format!("verifier-oci-{port}.yaml"));
  std::fs::write(
    &path,
    format!(
      "version: 1\ncomponents:\n  - name: csv\n    source: {{ kind: oci, reference: \"{reference}\", digest: \"{digest}\" }}\nhooks:\n  state-setup:\n    - name: fixtures\n      run:\n        kind: http\n        url: \"{}/_pact/provider-states\"\n        format: pact-state-change\n",
      provider.base_url()
    ),
  )
  .expect("writing the config");
  path
}

#[test]
fn a_pushed_component_verifies_a_csv_contract_pinned_by_digest_and_then_offline() {
  let registry = registry::Registry::start();
  let reference = format!("{}/janus-csv:1.0.0", registry.host);
  let pushed = janus_cached(
    &temp("oci-push-cache"),
    &[
      "component",
      "push",
      &csv_component(),
      &reference,
      "--output",
      "json",
    ],
  );
  assert_eq!(code(&pushed), 0, "{}", stderr(&pushed));
  let pushed: Value = serde_json::from_slice(&pushed.stdout).unwrap();
  assert_eq!(pushed["component"], json!({ "name": "csv", "version": "1.0.0" }));
  let digest = pushed["digest"].as_str().unwrap().to_string();

  let provider = provider();
  let config = oci_config(&provider, &reference, &digest);
  let cache = temp("oci-verify-cache");
  let _ = std::fs::remove_dir_all(&cache);
  let verify = || {
    janus_cached(
      &cache,
      &[
        "verify",
        &csv_contract(),
        "--provider-url",
        provider.base_url(),
        "--config",
        &config,
      ],
    )
  };
  registry.take_log();
  let first = verify();
  assert_eq!(
    code(&first),
    0,
    "stdout: {}\nstderr: {}",
    stdout(&first),
    stderr(&first)
  );
  assert!(
    stdout(&first).contains("VERIFIED: 1 verified, 0 failed"),
    "{}",
    stdout(&first)
  );
  assert_eq!(
    registry.take_log().len(),
    3,
    "a manifest by digest, its config, its layer"
  );

  // The pin is in the cache: the next run needs no registry at all.
  drop(registry);
  let offline = verify();
  assert_eq!(code(&offline), 0, "stderr: {}", stderr(&offline));
  assert!(stdout(&offline).contains("VERIFIED: 1 verified, 0 failed"));
}

#[test]
fn component_pull_says_what_to_pin_and_refuses_what_is_not_what_it_claims() {
  let registry = registry::Registry::start();
  let reference = format!("{}/janus-csv:1.0.0", registry.host);
  let pushed = janus_cached(
    &temp("oci-pull-push-cache"),
    &["component", "push", &csv_component(), &reference],
  );
  assert_eq!(code(&pushed), 0, "{}", stderr(&pushed));
  assert!(
    stdout(&pushed).contains("pushed csv 1.0.0 (content, matcher)"),
    "{}",
    stdout(&pushed)
  );

  let cache = temp("oci-pull-cache");
  let _ = std::fs::remove_dir_all(&cache);
  let pulled = janus_cached(&cache, &["component", "pull", &reference]);
  let text = stdout(&pulled);
  assert_eq!(code(&pulled), 0, "{}", stderr(&pulled));
  assert!(text.contains("csv 1.0.0 (content, matcher)"), "{text}");
  assert!(text.contains("content types: text/csv"), "{text}");
  assert!(
    text.contains(&format!(
      "source: {{ kind: oci, reference: \"{reference}\", digest: \"sha256:"
    )),
    "{text}"
  );

  // A registry serving other bytes: the artifact is not what it claims — the subject failed.
  registry.tamper(registry::Tamper::Blobs);
  let tampered_cache = temp("oci-pull-tampered-cache");
  let _ = std::fs::remove_dir_all(&tampered_cache);
  let tampered = janus_cached(&tampered_cache, &["component", "pull", &reference]);
  assert_eq!(code(&tampered), 1, "{}", stderr(&tampered));
  assert!(
    stderr(&tampered).contains("digest-mismatch"),
    "{}",
    stderr(&tampered)
  );

  // And no registry at all is the command failing to run.
  drop(registry);
  let unreachable = janus_cached(&tampered_cache, &["component", "pull", &reference]);
  assert_eq!(code(&unreachable), 2, "{}", stderr(&unreachable));
}
