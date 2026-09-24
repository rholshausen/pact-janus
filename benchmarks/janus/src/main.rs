//! Plan task 9.1: the 1.7 baseline's scenarios, run against Janus in every embedding it has.
//!
//! - **native**: the engine in the host's own process, wired as the `janus` CLI wires it. What
//!   the work costs with no boundary to cross.
//! - **subprocess**: `janus-engine` over its stdio framing (ADR 0003's artifact 3), the embedding
//!   both SDKs ship.
//! - **wasm**: the engine as a WASM component under wasmtime (ADR 0003's artifact 1, built for
//!   this task in `engine-wasm/`). It has no sockets and no threads, so it runs only what the
//!   kernel does without I/O. Every scenario that needs a mock server or a provider is absent from
//!   its results because the embedding cannot run it, not because it was skipped.
//!
//! Scenario names are the baseline's where the workload is the baseline's. Where Janus has to do
//! something different, the scenario has a new name and says why.
//!
//!   cd benchmarks/janus && cargo run --release [-- native subprocess wasm]

// The baseline's own file, in the baseline's own style: `cargo fmt` here must not reformat it.
#[allow(dead_code)]
#[rustfmt::skip]
#[path = "../../src/util.rs"]
mod util;

mod embed;
mod workloads;

use embed::{Client, Native, Pipe, Subprocess, Wasm, WasmRuntime};
use pact_janus_kernel::plan::{self, CapturedValues};
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};
use util::{HttpClient, ProviderStub, RunResults, Samples, order_doc};

#[derive(Clone, Copy, PartialEq)]
enum Kind {
  Native,
  Subprocess,
}

struct Env {
  engine_binary: PathBuf,
}

impl Env {
  fn client(&self, kind: Kind) -> Client {
    match kind {
      Kind::Native => Client::new(Box::new(Native::new())),
      Kind::Subprocess => Client::new(Box::new(Subprocess::spawn(&self.engine_binary))),
    }
  }
}

fn repo_root() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../..")
    .canonicalize()
    .unwrap()
}

fn run(dir: &Path, program: &str, args: &[&str]) {
  let status = Command::new(program)
    .args(args)
    .current_dir(dir)
    .status()
    .unwrap_or_else(|err| panic!("running {program}: {err}"));
  assert!(status.success(), "{program} {args:?} failed in {}", dir.display());
}

fn stdout_of(dir: &Path, program: &str, args: &[&str]) -> String {
  Command::new(program)
    .args(args)
    .current_dir(dir)
    .output()
    .ok()
    .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
    .filter(|s| !s.is_empty())
    .unwrap_or_else(|| "unknown".to_string())
}

fn round(value: f64, places: i32) -> f64 {
  let scale = 10f64.powi(places);
  (value * scale).round() / scale
}

fn median(mut values: Vec<f64>) -> f64 {
  values.sort_by(|a, b| a.partial_cmp(b).unwrap());
  values[values.len() / 2]
}

fn ms(duration: Duration) -> f64 {
  duration.as_secs_f64() * 1e3
}

// ------------------------------------------------------------------ consumer side: the mock

struct Mock {
  session: String,
  handle: String,
  variants: Vec<String>,
  port: u16,
}

fn start_mock(client: &mut Client, interaction: Value) -> Mock {
  let session = client.ok(
    "consumer-session/create",
    json!({ "config": { "consumer": { "name": "bench-consumer" }, "provider": { "name": "bench-provider" } } }),
  )["session"]
    .as_str()
    .unwrap()
    .to_string();
  let handle = client.ok(
    "consumer-session/add-interaction",
    json!({ "session": session, "interaction": interaction }),
  )["handle"]
    .as_str()
    .unwrap()
    .to_string();
  let variants = client.ok(
    "consumer-session/variants",
    json!({ "session": session, "handle": handle }),
  )["variants"]
    .as_array()
    .unwrap()
    .iter()
    .map(|v| v["id"].as_str().unwrap().to_string())
    .collect();
  let started = client.ok(
    "consumer-session/start-transport",
    json!({ "session": session, "transport": "http" }),
  );
  let port = started["endpoint"]["port"].as_u64().unwrap() as u16;
  Mock {
    session,
    handle,
    variants,
    port,
  }
}

fn serve(client: &mut Client, mock: &Mock, variant: &str) {
  let served = client.ok(
    "consumer-session/serve-variant",
    json!({ "session": mock.session, "handle": mock.handle, "variant": variant }),
  );
  debug_assert_eq!(served, json!({}));
}

fn finalise(client: &mut Client, mock: &Mock) -> Value {
  client.ok("consumer-session/finalise", json!({ "session": mock.session }))
}

/// Every variant's exchange status, from `finalise`'s results.
fn statuses(finalised: &Value) -> Vec<String> {
  finalised["results"]
    .as_array()
    .unwrap()
    .iter()
    .flat_map(|r| r["variants"].as_array().unwrap().iter())
    .map(|v| v["status"].as_str().unwrap().to_string())
    .collect()
}

/// Baseline: create pact + start mock + shut it down. Janus: create a session, add the
/// interaction, enumerate its variants, start the transport, finalise (which stops it).
fn mock_server_startup(results: &mut RunResults, env: &Env, kind: Kind) {
  let mut client = env.client(kind);
  let samples = Samples::collect(2, 20, || {
    let mock = start_mock(&mut client, workloads::small_request_interaction());
    finalise(&mut client, &mock);
  });
  results.record("mock-server-startup-cycle", samples.metrics());
}

/// A served variant answers exactly one request (engine/kernel/src/protocol/exchange.rs: one
/// armed exchange per transport instance, consumed by the arrival). So every Janus request is a
/// `serve-variant` frame and then the request, and a test pays for both. `arm_median_us` is the
/// frame's share.
#[allow(clippy::too_many_arguments)]
fn mock_latency(
  results: &mut RunResults,
  env: &Env,
  kind: Kind,
  name: &str,
  interaction: Value,
  body: &[u8],
  expected: (u16, &str),
  (warmup, iterations): (usize, usize),
) {
  let mut client = env.client(kind);
  let mock = start_mock(&mut client, interaction);
  let variant = mock.variants[0].clone();
  let mut http = HttpClient::new(mock.port);
  let mut arm = Vec::with_capacity(warmup + iterations);
  let samples = Samples::collect(warmup, iterations, || {
    let started = Instant::now();
    serve(&mut client, &mock, &variant);
    arm.push(started.elapsed().as_secs_f64() * 1e6);
    let (status, reply) = http.post("/orders", body);
    assert_eq!(
      status,
      expected.0,
      "{name}: {}",
      String::from_utf8_lossy(&reply[..reply.len().min(2000)])
    );
  });
  let finalised = finalise(&mut client, &mock);
  let statuses = statuses(&finalised);
  assert!(statuses.iter().all(|s| s == expected.1), "{name}: {statuses:?}");
  let mut metrics = samples.metrics();
  metrics["arm_median_us"] = json!(round(median(arm), 1));
  results.record(name, metrics);
}

/// Baseline: one mock server, four connections hammering it. Janus cannot run that: a transport
/// instance holds one armed exchange at a time, so one mock serves its requests strictly in
/// sequence with a frame between each. The parallel shape Janus has is four engines — four test
/// files, each with its own mock — so that is what this measures, under a new name.
fn mock_throughput(results: &mut RunResults, env: &Env, kind: Kind) {
  let threads = 4;
  let duration = Duration::from_secs(2);
  let total: u64 = std::thread::scope(|scope| {
    (0..threads)
      .map(|_| {
        scope.spawn(|| {
          let mut client = env.client(kind);
          let mock = start_mock(&mut client, workloads::small_request_interaction());
          let variant = mock.variants[0].clone();
          let mut http = HttpClient::new(mock.port);
          let body = br#"{"sku":"gadget-9","quantity":7}"#;
          let mut count = 0u64;
          let started = Instant::now();
          while started.elapsed() < duration {
            serve(&mut client, &mock, &variant);
            let (status, _) = http.post("/orders", body);
            assert_eq!(status, 201);
            count += 1;
          }
          finalise(&mut client, &mock);
          count
        })
      })
      .collect::<Vec<_>>()
      .into_iter()
      .map(|h| h.join().unwrap())
      .sum()
  });
  let rps = total as f64 / duration.as_secs_f64();
  results.record(
    "mock-server-throughput-4engines",
    json!({ "engines": threads, "seconds": duration.as_secs(), "requests": total, "rps": rps.round() }),
  );
}

fn mock_scenarios(results: &mut RunResults, env: &Env, kind: Kind) {
  mock_server_startup(results, env, kind);
  mock_latency(
    results,
    env,
    kind,
    "mock-server-latency-small",
    workloads::small_request_interaction(),
    br#"{"sku":"gadget-9","quantity":7}"#,
    (201, "verified"),
    (500, 5000),
  );
  let document = order_doc(100 * 1024);
  let large = serde_json::to_vec(&document).unwrap();
  mock_latency(
    results,
    env,
    kind,
    "mock-server-latency-100k",
    workloads::large_request_interaction(&document),
    &large,
    (201, "verified"),
    (50, 500),
  );
  mock_latency(
    results,
    env,
    kind,
    "mock-server-latency-mismatch",
    workloads::small_request_interaction(),
    br#"{"sku":123,"quantity":"two"}"#,
    (500, "failed"),
    (50, 500),
  );
  mock_throughput(results, env, kind);
}

// ------------------------------------------------------------------ provider side: verification

/// Starts a run and polls its stream to the terminal event; returns every event.
fn verify(client: &mut Client, contracts: Vec<Value>, target: Value, options: Value) -> Vec<Value> {
  let started = client.ok(
    "verification/verify",
    json!({ "source": { "kind": "inline", "contracts": contracts }, "target": target, "options": options }),
  );
  let stream = started["stream"].as_str().unwrap().to_string();
  let mut events = Vec::new();
  loop {
    let polled = client.ok("events/poll", json!({ "streams": [stream], "wait-ms": 10_000 }));
    let batch = polled["events"].as_array().unwrap().clone();
    assert!(!batch.is_empty(), "the run stopped producing events");
    let done = batch.iter().any(|e| e["last"] == json!(true));
    events.extend(batch);
    if done {
      return events;
    }
  }
}

fn interaction_results(events: &[Value]) -> Vec<String> {
  events
    .iter()
    .filter(|e| e["kind"] == json!("verification/interaction-result"))
    .map(|e| e["payload"]["status"].as_str().unwrap_or("?").to_string())
    .collect()
}

fn http_target(port: u16) -> Value {
  json!({ "transports": [ { "transport": "http", "options": { "base-url": format!("http://127.0.0.1:{port}") } } ] })
}

/// Writes the baseline's corpus (`pacts` v3 pacts of `interactions` each) as files, and the stub's
/// route table.
fn write_corpus(
  dir: &Path,
  pacts: usize,
  interactions: usize,
  body: &Value,
) -> (Vec<PathBuf>, HashMap<String, Vec<u8>>) {
  std::fs::create_dir_all(dir).unwrap();
  let mut files = Vec::new();
  let mut routes = HashMap::new();
  for p in 0..pacts {
    let consumer = format!("bench-consumer-{p}");
    let path = dir.join(format!("{consumer}-bench-provider.json"));
    std::fs::write(
      &path,
      workloads::v3_pact(&consumer, p, interactions, body).to_string(),
    )
    .unwrap();
    files.push(path);
    for i in 0..interactions {
      routes.insert(format!("/orders/{p}/{i}"), serde_json::to_vec(body).unwrap());
    }
  }
  (files, routes)
}

/// Baseline: new verifier handle, add the pact files, execute. Janus: read the files, one
/// `verification/verify` with them inline (the only source kind the engine reads), poll to the end.
/// The engine is started once per scenario; its start is `cold-start`'s business.
#[allow(clippy::too_many_arguments)]
fn verify_corpus(
  results: &mut RunResults,
  env: &Env,
  kind: Kind,
  name: &str,
  pacts: usize,
  interactions: usize,
  body: &Value,
  runs: usize,
  upgraded: bool,
) {
  let dir = std::env::temp_dir().join(format!("pact-bench-janus-{name}-{}", std::process::id()));
  let (files, routes) = write_corpus(&dir, pacts, interactions, body);
  let provider = ProviderStub::start(routes);
  let mut client = env.client(kind);
  if upgraded {
    // The same corpus, upgraded to Janus contracts first (outside the timing), so the run takes
    // the contract path instead of the v1–v4 one.
    for file in &files {
      let pact: Value = serde_json::from_slice(&std::fs::read(file).unwrap()).unwrap();
      let upgraded = client.ok("upgrade/pact", json!({ "pact": pact }));
      std::fs::write(file, upgraded["contract"].to_string()).unwrap();
    }
  }
  let mut times = Vec::with_capacity(runs);
  for _ in 0..runs + 1 {
    let started = Instant::now();
    let contracts: Vec<Value> = files
      .iter()
      .map(|f| serde_json::from_slice(&std::fs::read(f).unwrap()).unwrap())
      .collect();
    let events = verify(&mut client, contracts, http_target(provider.port), json!({}));
    times.push(ms(started.elapsed()));
    let statuses = interaction_results(&events);
    assert_eq!(
      statuses.len(),
      pacts * interactions,
      "{name}: one result per interaction"
    );
    assert!(statuses.iter().all(|s| s == "verified"), "{name}: {statuses:?}");
  }
  times.remove(0); // the first run is the warm-up
  let median_ms = median(times);
  results.record(
    name,
    json!({
      "pacts": pacts,
      "interactions": pacts * interactions,
      "runs": runs,
      "median_total_ms": round(median_ms, 1),
      "median_ms_per_interaction": round(median_ms / (pacts * interactions) as f64, 2),
    }),
  );
  let _ = std::fs::remove_dir_all(&dir);
}

fn small_order() -> Value {
  json!({
    "id": "ORD-1", "status": "PENDING",
    "customer": { "id": 42, "name": "Customer 7", "email": "customer7@example.com" },
    "lines": [ { "line": 1, "sku": "widget-1", "quantity": 2, "price": 12.5 } ],
    "created": "2026-08-23T10:00:00Z",
    "tags": ["widget", "flange"],
  })
}

fn verify_scenarios(results: &mut RunResults, env: &Env, kind: Kind) {
  verify_corpus(
    results,
    env,
    kind,
    "verify-corpus-small",
    10,
    5,
    &small_order(),
    5,
    false,
  );
  verify_corpus(
    results,
    env,
    kind,
    "verify-corpus-100k",
    1,
    5,
    &order_doc(100 * 1024),
    3,
    false,
  );
  verify_corpus(
    results,
    env,
    kind,
    "verify-corpus-small-upgraded",
    10,
    5,
    &small_order(),
    5,
    true,
  );
}

// ------------------------------------------------------------------ variants

/// The consumer side of the variant matrix: the RFC's order payload, every selected variant served
/// and requested, against the same interaction with only its base variant. A fresh session per
/// run, as a test file would have. The difference is what the variants cost a consumer test.
fn variants_consumer(results: &mut RunResults, env: &Env, kind: Kind) {
  let mut client = env.client(kind);
  let mut finalising = Vec::new();
  let mut run_once = |all: bool| -> (usize, f64) {
    let started = Instant::now();
    let mock = start_mock(&mut client, workloads::rfc_order_interaction());
    let mut http = HttpClient::new(mock.port);
    let chosen: Vec<String> = if all {
      mock.variants.clone()
    } else {
      mock.variants[..1].to_vec()
    };
    for variant in &chosen {
      serve(&mut client, &mock, variant);
      let (status, _) = http.request("GET", "/orders/66", b"");
      assert_eq!(status, 200);
    }
    let finalise_started = Instant::now();
    let finalised = finalise(&mut client, &mock);
    finalising.push(ms(finalise_started.elapsed()));
    let elapsed = ms(started.elapsed());
    assert_eq!(
      statuses(&finalised).iter().filter(|s| *s == "verified").count(),
      chosen.len()
    );
    (mock.variants.len(), elapsed)
  };
  run_once(true);
  let runs = 20;
  let all: Vec<f64> = (0..runs).map(|_| run_once(true).1).collect();
  let one: Vec<f64> = (0..runs).map(|_| run_once(false).1).collect();
  let variants = run_once(true).0;
  let (all, one) = (median(all), median(one));
  results.record(
    "variants-consumer-rfc-order",
    json!({
      "variants": variants,
      "runs": runs,
      // `finalise` stops the transport, and the HTTP transport's `stop` waits for the lock its
      // exchange loop holds through each 200 ms `poll-inbound` (engine/component-http): a fixed
      // cost per session, whatever the variant count (Phase 9 finding 27).
      "median_ms_finalise": round(median(finalising), 2),
      "median_ms_all_variants": round(all, 2),
      "median_ms_base_only": round(one, 2),
      "median_ms_per_extra_variant": round((all - one) / (variants - 1) as f64, 3),
    }),
  );
}

/// The provider side: a contract recording every variant of an interaction the sample provider
/// can satisfy through its own v3 state endpoint, verified whole and filtered to one variant.
/// Needs the `http` hook implementation, which only the native embedding registers — `janus-engine`
/// registers no hook invokers (cli/src/bin/janus_engine.rs), so a state-bound verification cannot
/// run through the subprocess at all.
fn variants_provider(results: &mut RunResults, env: &Env) {
  let mut client = env.client(Kind::Native);
  let mock = start_mock(&mut client, workloads::bound_order_interaction());
  let mut http = HttpClient::new(mock.port);
  for variant in &mock.variants {
    serve(&mut client, &mock, variant);
    let (status, _) = http.request("GET", "/orders/66", b"");
    assert_eq!(status, 200);
  }
  let contract = finalise(&mut client, &mock)["contract"].clone();
  assert!(
    contract.is_object(),
    "every variant verified on the consumer side"
  );

  let provider = pact_janus_sample_order_service::start(pact_janus_sample_order_service::Config {
    token: None,
    ..Default::default()
  })
  .unwrap();
  let base_url = provider.base_url();
  let state_hook = json!([{ "name": "fixtures",
    "run": { "kind": "http", "url": format!("{base_url}/_pact/provider-states"), "format": "pact-state-change" } }]);
  let mut target = json!({ "transports": [ { "transport": "http", "options": { "base-url": base_url } } ] });
  target["hooks"] =
    json!({ "version": 1, "hooks": { "state-setup": state_hook, "state-teardown": state_hook } });

  let mut run_once = |options: Value, expected: usize| -> f64 {
    let started = Instant::now();
    let events = verify(&mut client, vec![contract.clone()], target.clone(), options);
    let elapsed = ms(started.elapsed());
    let statuses = interaction_results(&events);
    assert_eq!(statuses.len(), expected, "{statuses:?}");
    assert!(statuses.iter().all(|s| s == "verified"), "{statuses:?}");
    elapsed
  };
  let variants = mock.variants.len();
  run_once(json!({}), variants);
  let runs = 10;
  let all: Vec<f64> = (0..runs).map(|_| run_once(json!({}), variants)).collect();
  let one: Vec<f64> = (0..runs)
    .map(|_| run_once(json!({ "variants": [mock.variants[0]] }), 1))
    .collect();
  let (all, one) = (median(all), median(one));
  results.record(
    "variants-provider-order-service",
    json!({
      "variants": variants,
      "runs": runs,
      "median_ms_all_variants": round(all, 2),
      "median_ms_one_variant": round(one, 2),
      "median_ms_per_extra_variant": round((all - one) / (variants - 1) as f64, 3),
    }),
  );
}

// ------------------------------------------------------------------ offline: what WASM can run too

/// Compile, enumerate, upgrade: the kernel's work with no I/O, through the protocol, in any
/// embedding.
fn offline_scenarios(results: &mut RunResults, client: &mut Client) {
  let small = workloads::v3_pact("bench-consumer", 0, 5, &small_order());
  let large = workloads::v3_pact("bench-consumer", 0, 5, &order_doc(100 * 1024));

  for (name, pact, iterations) in [
    ("explain-pact-small", &small, 2000),
    ("explain-pact-100k", &large, 200),
  ] {
    let body = json!({ "interaction": { "kind": "contract-interaction", "contract": pact }, "options": { "plan": true } });
    let samples = Samples::collect(iterations / 10, iterations, || {
      client.ok("verification/explain", body.clone());
    });
    results.record(name, samples.metrics());
  }

  let upgrade = json!({ "pact": large });
  let samples = Samples::collect(20, 200, || {
    client.ok("upgrade/pact", upgrade.clone());
  });
  results.record("upgrade-pact-100k", samples.metrics());

  let session = client.ok(
    "consumer-session/create",
    json!({ "config": { "consumer": { "name": "c" }, "provider": { "name": "p" } } }),
  )["session"]
    .as_str()
    .unwrap()
    .to_string();
  let mut count = 0;
  let samples = Samples::collect(20, 200, || {
    let handle = client.ok(
      "consumer-session/add-interaction",
      json!({ "session": session, "interaction": workloads::rfc_order_interaction() }),
    )["handle"]
      .as_str()
      .unwrap()
      .to_string();
    count = client.ok(
      "consumer-session/variants",
      json!({ "session": session, "handle": handle }),
    )["variants"]
      .as_array()
      .unwrap()
      .len();
  });
  let mut metrics = samples.metrics();
  metrics["variants"] = json!(count);
  results.record("variant-space-rfc-order", metrics);
}

/// The plan `verification/explain` compiles for a pact's first interaction, and the captured
/// values that match it: the response the pact itself records.
fn plan_and_values(client: &mut Client, pact: &Value) -> (Value, BTreeMap<String, Value>) {
  let explained = client.ok(
    "verification/explain",
    json!({ "interaction": { "kind": "contract-interaction", "contract": pact }, "options": { "plan": true } }),
  );
  let request = &pact["interactions"][0]["request"];
  let response = &pact["interactions"][0]["response"];
  let mut values = BTreeMap::new();
  values.insert("$.request.method".to_string(), request["method"].clone());
  values.insert("$.request.path".to_string(), request["path"].clone());
  values.insert("$.request.query".to_string(), json!({}));
  values.insert("$.response.status".to_string(), response["status"].clone());
  values.insert("$.response.body".to_string(), response["body"].clone());
  values.insert(
    "$.response.headers".to_string(),
    json!({ "content-type": "application/json" }),
  );
  (explained["plan"].clone(), values)
}

/// Matching captured values against a compiled plan: the interpreter's own cost, which no protocol
/// operation exposes (the CLI's `explain --executed` runs it beside the protocol). Native calls the
/// kernel; WASM calls the guest's bench-only export. Each iteration parses the values, as a real
/// arrival would be parsed.
enum Matcher<'a> {
  Native(plan::Plan),
  Wasm(&'a mut Wasm),
}

fn match_scenario(
  results: &mut RunResults,
  name: &str,
  matcher: &mut Matcher,
  values: &[u8],
  iterations: usize,
) {
  let samples = Samples::collect(iterations / 10, iterations, || {
    let status = match matcher {
      Matcher::Native(compiled) => {
        let values: BTreeMap<String, Value> = serde_json::from_slice(values).unwrap();
        let executed = plan::execute(compiled, &CapturedValues::from_json(&values));
        match plan::outcome(&executed).0 {
          plan::Status::Matched => "matched".to_string(),
          plan::Status::Mismatched => "mismatched".to_string(),
        }
      }
      Matcher::Wasm(wasm) => wasm.execute(values),
    };
    assert_eq!(status, "matched");
  });
  results.record(name, samples.metrics());
}

fn match_workloads(client: &mut Client) -> Vec<(&'static str, Value, Vec<u8>, usize)> {
  [
    ("match-small", small_order(), 5000),
    ("match-100k", order_doc(100 * 1024), 500),
  ]
  .into_iter()
  .map(|(name, body, iterations)| {
    let pact = workloads::v3_pact("bench-consumer", 0, 1, &body);
    let (plan, values) = plan_and_values(client, &pact);
    (name, plan, serde_json::to_vec(&values).unwrap(), iterations)
  })
  .collect()
}

// ------------------------------------------------------------------ cold start

/// Per test file, a host starts an engine and handshakes before its first real operation.
fn cold_start(results: &mut RunResults, env: &Env, kind: Kind) {
  let samples = Samples::collect(2, 20, || match kind {
    Kind::Native => drop(Client::new(Box::new(Native::new()))),
    Kind::Subprocess => {
      let mut subprocess = Subprocess::spawn(&env.engine_binary);
      let hello = json!({ "type": "request", "id": "r-1", "op": "engine/hello",
        "body": { "protocol-versions": [1], "host": { "name": "b", "version": "0" }, "capabilities": {} } });
      subprocess.call(&serde_json::to_vec(&hello).unwrap());
      subprocess.close();
    }
  });
  results.record("cold-start", samples.metrics());
}

// ------------------------------------------------------------------ the three stacks

/// `cargo run --release -- probe`: where the time goes in one mock exchange, as the request
/// document grows. How the positional 100 KB shape's cost curve was found.
fn probe() {
  for kib in [1usize, 2, 4, 8, 16] {
    let document = order_doc(kib * 1024);
    let body = serde_json::to_vec(&document).unwrap();
    let mut client = Client::new(Box::new(Native::new()));
    let mut lap = Instant::now();
    let mut step = |name: &str| {
      let elapsed = ms(lap.elapsed());
      lap = Instant::now();
      format!("{name} {elapsed:.1}ms")
    };
    let session = client.ok(
      "consumer-session/create",
      json!({ "config": { "consumer": { "name": "c" }, "provider": { "name": "p" } } }),
    )["session"]
      .clone();
    let handle = client.ok(
      "consumer-session/add-interaction",
      json!({ "session": session, "interaction": workloads::large_request_interaction(&document) }),
    )["handle"]
      .clone();
    let added = step("add-interaction");
    let variants = client.ok(
      "consumer-session/variants",
      json!({ "session": session, "handle": handle }),
    )["variants"]
      .clone();
    let listed = step("variants");
    let port = client.ok(
      "consumer-session/start-transport",
      json!({ "session": session, "transport": "http" }),
    )["endpoint"]["port"]
      .as_u64()
      .unwrap() as u16;
    let started = step("start-transport");
    client.ok(
      "consumer-session/serve-variant",
      json!({ "session": session, "handle": handle, "variant": variants[0]["id"] }),
    );
    let served = step("serve-variant");
    let (status, _) = HttpClient::new(port).post("/orders", &body);
    let requested = step("request");
    println!(
      "{kib:>3} KiB ({} bytes, {status}): {added}, {listed}, {started}, {served}, {requested}",
      body.len()
    );
  }
}

fn main() {
  if std::env::args().nth(1).as_deref() == Some("probe") {
    return probe();
  }
  let root = repo_root();
  let wanted: Vec<String> = std::env::args().skip(1).collect();
  let wants = |stack: &str| wanted.is_empty() || wanted.iter().any(|w| w == stack);
  let date = stdout_of(&root, "date", &["+%F"]);
  let commit = stdout_of(
    &root,
    "git",
    &["describe", "--always", "--dirty", "--exclude", "*"],
  );
  let results_dir = root.join("benchmarks/results");

  println!("building janus-engine and the engine component (release)");
  let engine_binary = match std::env::var_os("JANUS_ENGINE") {
    Some(path) => PathBuf::from(path),
    None => {
      run(
        &root,
        "cargo",
        &[
          "build",
          "--release",
          "-p",
          "pact_janus_cli",
          "--bin",
          "janus-engine",
        ],
      );
      root.join("target/release/janus-engine")
    }
  };
  let guest_dir = root.join("benchmarks/janus/engine-wasm");
  run(
    &guest_dir,
    "cargo",
    &["build", "--release", "--target", "wasm32-wasip2"],
  );
  let guest = guest_dir.join("target/wasm32-wasip2/release/janus_engine_wasm.wasm");
  let env = Env { engine_binary };

  for (kind, stack) in [(Kind::Native, "native"), (Kind::Subprocess, "subprocess")] {
    if !wants(stack) {
      continue;
    }
    let stack = format!("janus-{stack}-{commit}");
    println!("pact-bench-janus — stack {stack} — {date}");
    let mut results = RunResults::new(&stack);
    cold_start(&mut results, &env, kind);
    println!("mock server scenarios:");
    mock_scenarios(&mut results, &env, kind);
    println!("verification scenarios:");
    verify_scenarios(&mut results, &env, kind);
    println!("variant scenarios:");
    variants_consumer(&mut results, &env, kind);
    if kind == Kind::Native {
      variants_provider(&mut results, &env);
    }
    println!("offline scenarios:");
    let mut client = env.client(kind);
    offline_scenarios(&mut results, &mut client);
    if kind == Kind::Native {
      for (name, plan_document, values, iterations) in match_workloads(&mut client) {
        let mut matcher = Matcher::Native(plan::from_json(&plan_document).unwrap());
        match_scenario(&mut results, name, &mut matcher, &values, iterations);
      }
    }
    println!(
      "results written to {}",
      results.write_in(&results_dir, &date).unwrap()
    );
  }

  if wants("wasm") {
    let stack = format!("janus-wasm-{commit}");
    println!("pact-bench-janus — stack {stack} — {date}");
    let mut results = RunResults::new(&stack);
    let bytes = std::fs::read(&guest).unwrap();
    let mut compile = Vec::new();
    for _ in 0..5 {
      let started = Instant::now();
      drop(WasmRuntime::compile(&bytes));
      compile.push(ms(started.elapsed()));
    }
    let runtime = WasmRuntime::compile(&bytes);
    let samples = Samples::collect(2, 20, || drop(Client::new(Box::new(runtime.instantiate()))));
    let mut metrics = samples.metrics();
    metrics["compile_median_ms"] = json!(round(median(compile), 1));
    metrics["component_bytes"] = json!(bytes.len());
    results.record("cold-start", metrics);

    println!("offline scenarios:");
    let mut client = Client::new(Box::new(runtime.instantiate()));
    offline_scenarios(&mut results, &mut client);
    let mut planner = Client::new(Box::new(Native::new()));
    for (name, plan_document, values, iterations) in match_workloads(&mut planner) {
      let mut wasm = runtime.instantiate();
      wasm.set_plan(&serde_json::to_vec(&plan_document).unwrap());
      let mut matcher = Matcher::Wasm(&mut wasm);
      match_scenario(&mut results, name, &mut matcher, &values, iterations);
    }

    // What the embedding cannot do, said by the engine rather than assumed by the harness.
    let session = client.ok(
      "consumer-session/create",
      json!({ "config": { "consumer": { "name": "c" }, "provider": { "name": "p" } } }),
    )["session"]
      .clone();
    client.ok(
      "consumer-session/add-interaction",
      json!({ "session": session, "interaction": workloads::small_request_interaction() }),
    );
    let refused = client.call(
      "consumer-session/start-transport",
      json!({ "session": session, "transport": "http" }),
    );
    let verify = client.call(
      "verification/verify",
      json!({ "source": { "kind": "inline", "contracts": [] }, "target": http_target(1) }),
    );
    results.record(
      "unsupported",
      json!({ "consumer-session/start-transport": refused.err(), "verification/verify": verify.err() }),
    );
    println!(
      "results written to {}",
      results.write_in(&results_dir, &date).unwrap()
    );
  }
}
