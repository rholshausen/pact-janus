//! Plan task 5.3: lifecycle hooks end to end — `state-setup` over HTTP in the v3 provider-state
//! format, `before-request` as a command, the failure policies, the change rules and the hook
//! report, all against the sample provider (5.6) rather than a stub.
//!
//! The two headline scenarios are B5's migration claim and the RFC's request filter:
//!
//! - a provider whose only integration is the state endpoint it **already had** is verified across
//!   several variants, each put into the state its `whenVariant` binding resolved to;
//! - a provider that requires a bearer token is verified because a `before-request` hook adds one —
//!   the exact failure `verification.rs` pins in `the_sample_providers_auth_is_visible_as_a_failed_
//!   variant_until_a_hook_supplies_it`, now removed by configuration rather than by code.

use pact_janus_hooks_host::{ExecHooks, HttpHooks, loader};
use pact_janus_kernel::component::TransportComponent;
use pact_janus_kernel::hooks::HookInvoker;
use pact_janus_kernel::protocol::Engine;
use pact_janus_sample_order_service::{
  Config as ProviderConfig, DEFAULT_TOKEN, Provider, start as start_provider,
};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

// ---------------------------------------------------------------------------------------------
// Harness
// ---------------------------------------------------------------------------------------------

fn send(engine: &mut Engine, id: &str, op: &str, body: Value) -> Value {
  let request = json!({ "type": "request", "id": id, "op": op, "body": body });
  let bytes = engine.dispatch(&serde_json::to_vec(&request).expect("a json! literal serializes"));
  serde_json::from_slice(&bytes).expect("dispatch always returns valid JSON")
}

/// An engine with the real components *and* the two hook implementations a native host can run
/// (lifecycle-hooks spec §8.5). An engine built without them refuses a configuration naming them.
fn engine_with_hooks() -> Engine {
  let mut transports: HashMap<String, Arc<dyn TransportComponent>> = HashMap::new();
  transports.insert(
    "http".to_string(),
    Arc::new(pact_janus_component_http::HttpTransport::new()),
  );
  let mut engine = Engine::with_components(
    transports,
    Some(Arc::new(pact_janus_component_json::JsonContent::new())),
  );
  engine.register_hook_invoker("exec", Arc::new(ExecHooks::new()) as Arc<dyn HookInvoker>);
  engine.register_hook_invoker("http", Arc::new(HttpHooks::new()) as Arc<dyn HookInvoker>);
  hello(&mut engine);
  engine
}

/// The same, plus the built-in oauth2 hook component for `run: { kind: component }`.
fn engine_with_the_oauth2_component() -> Engine {
  let mut engine = engine_with_hooks();
  engine.register_hook_component("oauth2", Arc::new(pact_janus_component_oauth2::Oauth2Hook::new()));
  engine
}

fn engine_without_hooks() -> Engine {
  let mut transports: HashMap<String, Arc<dyn TransportComponent>> = HashMap::new();
  transports.insert(
    "http".to_string(),
    Arc::new(pact_janus_component_http::HttpTransport::new()),
  );
  let mut engine = Engine::with_components(
    transports,
    Some(Arc::new(pact_janus_component_json::JsonContent::new())),
  );
  hello(&mut engine);
  engine
}

fn hello(engine: &mut Engine) {
  send(
    engine,
    "r-1",
    "engine/hello",
    json!({ "protocol-versions": [1], "host": { "name": "janus-test", "version": "0.0.0" }, "capabilities": {} }),
  );
}

fn fixture(name: &str) -> String {
  PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    .join("tests/fixtures/hooks")
    .join(name)
    .display()
    .to_string()
}

fn drain_run(engine: &mut Engine, stream: &str) -> Vec<Value> {
  let mut events: Vec<Value> = Vec::new();
  for poll in 0..200 {
    let response = send(
      engine,
      &format!("p-{poll}"),
      "events/poll",
      json!({ "streams": [stream], "wait-ms": 5_000 }),
    );
    events.extend(
      response["ok"]["events"]
        .as_array()
        .unwrap_or_else(|| panic!("events/poll failed: {response}"))
        .clone(),
    );
    if events.last().is_some_and(|e| e["last"] == json!(true)) {
      break;
    }
  }
  assert!(
    events.last().is_some_and(|e| e["last"] == json!(true)),
    "the run terminated"
  );
  events
}

fn results(events: &[Value]) -> Vec<&Value> {
  payloads(events, "verification/interaction-result")
}

fn hook_events(events: &[Value]) -> Vec<&Value> {
  payloads(events, "verification/hook")
}

fn payloads<'a>(events: &'a [Value], kind: &str) -> Vec<&'a Value> {
  events
    .iter()
    .filter(|e| e["kind"] == json!(kind))
    .map(|e| &e["payload"])
    .collect()
}

fn summary(events: &[Value]) -> &Value {
  &events.last().expect("a terminal event")["payload"]
}

fn verify(engine: &mut Engine, contract: Value, base_url: &str, hooks: Option<Value>) -> Vec<Value> {
  let mut target = json!({
    "transports": [ { "transport": "http", "options": { "base-url": base_url } } ]
  });
  if let Some(hooks) = hooks {
    target["hooks"] = hooks;
  }
  let started = send(
    engine,
    "v-1",
    "verification/verify",
    json!({ "source": { "kind": "inline", "contracts": [contract] }, "target": target }),
  );
  let stream = started["ok"]["stream"]
    .as_str()
    .unwrap_or_else(|| panic!("verify failed: {started}"))
    .to_string();
  drain_run(engine, &stream)
}

// ---------------------------------------------------------------------------------------------
// Contracts
// ---------------------------------------------------------------------------------------------

/// The order interaction against the sample provider, with `shippedAt` optional and a `whenVariant`
/// binding on it — the RFC's `given('an order exists', { shipped: whenVariant('shippedAt', …) })`.
/// Recorded by a consumer session so the variants, their assignments and their resolved states are
/// this engine's own, not a hand-written guess at them.
fn contract_with_two_variants(engine: &mut Engine) -> Value {
  let interaction = json!({
    "description": "a request for an order",
    "transport": { "kind": "http", "mode": "passive" },
    "states": [{
      "name": "an order exists",
      "params": { "id": "66" },
      "variant-params": [{
        "name": "shipped",
        "dimension": "shippedAt",
        "cases": [ { "point": "present", "value": true }, { "point": "absent", "value": false } ]
      }]
    }],
    "parts": {
      "request": { "method": { "shape": "equality", "example": "GET" },
                   "path": { "shape": "equality", "example": "/orders/66" } },
      "response": { "status": { "shape": "equality", "example": 200 },
                    "body": { "shape": "object", "members": {
                      "id": { "shape": "equality", "example": "66" },
                      "shippedAt": { "shape": "optional",
                                     "of": { "shape": "string", "example": "2026-07-30T09:00:00Z" } } } } }
    }
  });
  record(engine, interaction)
}

/// Run an interaction through a consumer session against the engine's own mock and return the
/// contract it wrote.
fn record(engine: &mut Engine, interaction: Value) -> Value {
  let create = send(
    engine,
    "c-1",
    "consumer-session/create",
    json!({ "config": { "consumer": { "name": "web-app" }, "provider": { "name": "order-service" } } }),
  );
  let session = create["ok"]["session"].as_str().unwrap().to_string();
  let added = send(
    engine,
    "c-2",
    "consumer-session/add-interaction",
    json!({ "session": session, "interaction": interaction }),
  );
  let handle = added["ok"]["handle"]
    .as_str()
    .unwrap_or_else(|| panic!("add-interaction failed: {added}"))
    .to_string();
  let variants = send(
    engine,
    "c-3",
    "consumer-session/variants",
    json!({ "session": session, "handle": handle }),
  );
  let ids: Vec<String> = variants["ok"]["variants"]
    .as_array()
    .expect("a selection")
    .iter()
    .map(|v| v["id"].as_str().unwrap().to_string())
    .collect();
  let started = send(
    engine,
    "c-4",
    "consumer-session/start-transport",
    json!({ "session": session, "transport": "http" }),
  );
  let endpoint = &started["ok"]["endpoint"];
  let addr = format!(
    "{}:{}",
    endpoint["host"].as_str().unwrap(),
    endpoint["port"].as_u64().unwrap()
  );
  for (i, id) in ids.iter().enumerate() {
    send(
      engine,
      &format!("c-1{i}"),
      "consumer-session/serve-variant",
      json!({ "session": session, "handle": handle, "variant": id }),
    );
    assert_eq!(http_get(&addr, "/orders/66"), 200, "the mock served '{id}'");
  }
  let finalised = send(
    engine,
    "c-99",
    "consumer-session/finalise",
    json!({ "session": session }),
  );
  let contract = finalised["ok"]["contract"].clone();
  assert!(contract.is_object(), "every variant verified: {finalised}");
  contract
}

fn http_get(addr: &str, path: &str) -> u16 {
  let mut stream = TcpStream::connect(addr).expect("the mock is listening");
  stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
  let request = format!("GET {path} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n");
  stream.write_all(request.as_bytes()).unwrap();
  let mut response = Vec::new();
  let _ = stream.read_to_end(&mut response);
  let head = String::from_utf8_lossy(&response);
  head
    .lines()
    .next()
    .and_then(|line| line.split_whitespace().nth(1))
    .and_then(|code| code.parse().ok())
    .unwrap_or(0)
}

fn provider_without_auth() -> Provider {
  start_provider(ProviderConfig {
    token: None,
    ..ProviderConfig::default()
  })
  .expect("the sample provider binds")
}

// ---------------------------------------------------------------------------------------------
// The two headline scenarios
// ---------------------------------------------------------------------------------------------

#[test]
fn a_v3_state_endpoint_puts_the_provider_into_each_variants_state_unchanged() {
  let provider = provider_without_auth();
  let mut engine = engine_with_hooks();
  let contract = contract_with_two_variants(&mut engine);

  // The only integration: naming the endpoint the provider already had. `format:
  // pact-state-change` is what makes that true — the endpoint receives `{state, params, action}`,
  // exactly what it received before Janus existed.
  let hooks = json!({
    "version": 1,
    "hooks": {
      "state-setup": [
        { "name": "fixtures",
          "run": { "kind": "http", "url": format!("{}/_pact/provider-states", provider.base_url()),
                   "format": "pact-state-change" } }
      ]
    }
  });

  let events = verify(&mut engine, contract, provider.base_url(), Some(hooks));
  let results = results(&events);
  assert_eq!(results.len(), 2);
  for result in &results {
    assert_eq!(
      result["status"],
      json!("verified"),
      "each variant was put into the state its binding resolved to: {result}"
    );
  }

  // Setup ran once per variant, with the parameter the binding resolved — including the literal
  // `id` alongside the bound `shipped`.
  let log = provider.store().lock().unwrap();
  let setups: Vec<&Value> = log
    .state_log
    .iter()
    .filter(|entry| entry["action"] == json!("setup"))
    .collect();
  assert_eq!(setups.len(), 2, "once per variant, never collapsed: {setups:?}");
  let shipped: Vec<&Value> = setups.iter().map(|entry| &entry["params"]["shipped"]).collect();
  assert!(
    shipped.contains(&&json!(true)) && shipped.contains(&&json!(false)),
    "the two variants asked for two different states: {shipped:?}"
  );
  assert_eq!(setups[0]["params"]["id"], json!("66"));
  assert_eq!(summary(&events)["status"], json!("verified"));
}

#[test]
fn a_before_request_hook_supplies_the_credential_the_provider_demands() {
  // Auth on: without a hook this run reports failed variants (see `verification.rs`).
  let provider = start_provider(ProviderConfig::default()).expect("the sample provider binds");
  let mut engine = engine_with_hooks();
  let contract = contract_with_two_variants(&mut engine);

  let hooks = json!({
    "version": 1,
    "hooks": {
      "state-setup": [
        { "name": "fixtures",
          "run": { "kind": "http", "url": format!("{}/_pact/provider-states", provider.base_url()),
                   "format": "pact-state-change" } }
      ],
      "before-request": [
        { "name": "add-token",
          // The child's environment is exactly this: deny-by-default, so even PATH is asked for.
          "run": { "kind": "exec", "command": "/bin/sh", "args": [fixture("add-token.sh")],
                   "env": { "TOKEN": DEFAULT_TOKEN, "PATH": "/usr/bin:/bin" } },
          "changes": ["parts.request.headers"] }
      ]
    }
  });

  let events = verify(&mut engine, contract, provider.base_url(), Some(hooks));
  for result in results(&events) {
    assert_eq!(
      result["status"],
      json!("verified"),
      "the hook signed the request the contract never mentioned: {result}"
    );
  }

  // The report answers "which hook set this header" by path, and never carries the token.
  let report = &summary(&events)["hooks"];
  let signed: Vec<&Value> = report["invocations"]
    .as_array()
    .expect("invocations")
    .iter()
    .filter(|i| i["hook"] == json!("add-token"))
    .collect();
  assert_eq!(signed.len(), 2, "once per exchange");
  assert_eq!(signed[0]["changed"], json!(["parts.request.headers"]));
  assert_eq!(signed[0]["implementation"], json!("exec"));
  let rendered = serde_json::to_string(report).unwrap();
  assert!(
    !rendered.contains(DEFAULT_TOKEN),
    "the path is what a reader needs; the value is what an attacker needs: {rendered}"
  );
}

#[test]
fn the_same_run_from_a_verifier_janus_yaml_file() {
  // The loader's whole job, end to end: a file with `${VAR}` references becomes the document the
  // engine receives, with no templates and no paths left in it (ADR 0014).
  let provider = provider_without_auth();
  let dir = std::env::temp_dir().join(format!(
    "janus-hooks-{}-{:?}",
    std::process::id(),
    std::thread::current().id()
  ));
  std::fs::create_dir_all(&dir).unwrap();
  let config = r#"
version: 1
hooks:
  state-setup:
    - name: fixtures
      run:
        kind: http
        url: "${PROVIDER_URL}/_pact/provider-states"
        format: pact-state-change
  after-response:
    - name: observe
      run: { kind: exec, command: /bin/sh, args: ["${OBSERVE_SH}"], env: { PATH: "/usr/bin:/bin" } }
"#;
  std::fs::write(dir.join("verifier.janus.yaml"), config).unwrap();

  let env: HashMap<String, String> = [
    ("PROVIDER_URL".to_string(), provider.base_url().to_string()),
    ("OBSERVE_SH".to_string(), fixture("observe.sh")),
  ]
  .into_iter()
  .collect();
  let text = std::fs::read_to_string(dir.join("verifier.janus.yaml")).unwrap();
  let resolved = loader::load_document(&text, &dir, &env).expect("the configuration resolves");
  assert!(
    !serde_json::to_string(&resolved).unwrap().contains("${"),
    "no templates cross the boundary: {resolved}"
  );

  let mut engine = engine_with_hooks();
  let contract = contract_with_two_variants(&mut engine);
  let events = verify(&mut engine, contract, provider.base_url(), Some(resolved));
  assert_eq!(summary(&events)["status"], json!("verified"));
  assert!(
    hook_events(&events)
      .iter()
      .any(|e| e["hook"] == json!("observe") && e["outcome"] == json!("ok")),
    "a command that succeeded with nothing to say is ok"
  );
  std::fs::remove_dir_all(&dir).ok();
}

// ---------------------------------------------------------------------------------------------
// Failure semantics
// ---------------------------------------------------------------------------------------------

#[test]
fn a_state_the_provider_cannot_reach_is_state_unavailable_not_failed() {
  let provider = provider_without_auth();
  let mut engine = engine_with_hooks();
  let mut contract = contract_with_two_variants(&mut engine);

  // Ask for the one state this provider genuinely cannot produce: SHIPPED with no shippedAt. The
  // endpoint answers `unsupported`, and that is a fact about the contract, not about the code.
  for variant in contract["interactions"][0]["selection"]["variants"]
    .as_array_mut()
    .unwrap()
  {
    variant["states"][0]["params"]["status"] = json!("SHIPPED");
    variant["states"][0]["params"]["shipped"] = json!(false);
  }
  // The interaction's binding would re-resolve `shipped`, so drop it: this run is about what the
  // state handler answers, not about resolution.
  contract["interactions"][0]["states"][0]
    .as_object_mut()
    .unwrap()
    .remove("variant-params");
  contract["interactions"][0]["states"][0]["params"] =
    json!({ "id": "66", "status": "SHIPPED", "shipped": false });

  let hooks = json!({
    "hooks": { "state-setup": [
      { "name": "fixtures",
        "run": { "kind": "http", "url": format!("{}/_pact/provider-states", provider.base_url()),
                 "format": "pact-state-change" } } ] }
  });
  let events = verify(&mut engine, contract, provider.base_url(), Some(hooks));

  let results = results(&events);
  assert_eq!(results[0]["status"], json!("state-unavailable"));
  assert_eq!(results[0]["hook"], json!("fixtures"));
  assert_eq!(
    results[0]["error"]["code"],
    json!("state-unreachable"),
    "the hook's own code, recorded as given — rewriting it would lose the only thing it carried"
  );

  let summary = summary(&events);
  assert_eq!(
    summary["status"],
    json!("failed"),
    "state-unavailable fails the run by default: the provider has not demonstrated it can produce \
     what the consumer demonstrated it handles"
  );
  assert_eq!(summary["variants"]["state-unavailable"], json!(2));
  assert_eq!(
    summary["variants"]["failed"],
    json!(0),
    "counted apart from failures, because its remedy is a contract change, not a code change"
  );
  let invocation = &summary["hooks"]["invocations"][0];
  assert_eq!(invocation["outcome"], json!("unsupported"));
  assert_eq!(invocation["effect"], json!("state-unavailable"));
}

#[test]
fn a_before_verification_hook_that_fails_aborts_the_run_and_says_what_never_ran() {
  let provider = provider_without_auth();
  let mut engine = engine_with_hooks();
  let contract = contract_with_two_variants(&mut engine);

  let hooks = json!({
    "hooks": {
      // `abort-run` is this point's default, and is not written here on purpose: a run that could
      // not complete its own preparation has tested nothing, and the alternative reports the same
      // fact once per interaction with the cause buried.
      "before-verification": [
        { "name": "auth-token",
          "run": { "kind": "exec", "command": "/bin/sh", "args": [fixture("fail.sh")],
                   "env": { "PATH": "/usr/bin:/bin" } } }
      ],
      "after-verification": [
        { "name": "report",
          "run": { "kind": "exec", "command": "/bin/sh", "args": [fixture("observe.sh")],
                   "env": { "PATH": "/usr/bin:/bin" } } }
      ]
    }
  });
  let events = verify(&mut engine, contract, provider.base_url(), Some(hooks));

  assert!(results(&events).is_empty(), "no exchange ran");
  let summary = summary(&events);
  assert_eq!(summary["status"], json!("failed"));
  assert_eq!(summary["aborted"]["point"], json!("before-verification"));
  assert_eq!(summary["aborted"]["hook"], json!("auth-token"));
  assert_eq!(
    summary["hooks"]["aborted"]["exchanges-not-run"],
    json!(2),
    "counted as things not done, never folded into the failed tally"
  );
  assert!(
    hook_events(&events).iter().any(|e| e["hook"] == json!("report")),
    "after-verification runs even after an abort (spec §3.8)"
  );
}

#[test]
fn a_hook_that_changes_what_it_did_not_declare_is_refused_and_the_request_is_untouched() {
  let provider = provider_without_auth();
  let mut engine = engine_with_hooks();
  let contract = contract_with_two_variants(&mut engine);

  let hooks = json!({
    "hooks": { "before-request": [
      { "name": "meddler",
        "run": { "kind": "exec", "command": "/bin/sh", "args": [fixture("rewrite-path.sh")],
                 "env": { "PATH": "/usr/bin:/bin" } },
        // Declares headers; the hook answers with a path.
        "changes": ["parts.request.headers"] } ] }
  });
  let events = verify(&mut engine, contract, provider.base_url(), Some(hooks));

  let results = results(&events);
  assert_eq!(results[0]["status"], json!("failed"));
  assert_eq!(results[0]["hook"], json!("meddler"));
  assert_eq!(results[0]["error"]["code"], json!("hook-change-refused"));
  assert_eq!(
    results[0]["error"]["details"]["path"],
    json!("parts.request.path")
  );

  let invocation = &summary(&events)["hooks"]["invocations"][0];
  assert_eq!(invocation["outcome"], json!("failed"));
  assert_eq!(invocation["effect"], json!("failed-exchange"));
  assert_eq!(
    invocation["changed"],
    json!([]),
    "nothing is partially applied: a hook that half-ran is a state no author tested"
  );
}

#[test]
fn a_teardown_failure_warns_and_leaves_the_result_alone() {
  let provider = provider_without_auth();
  let mut engine = engine_with_hooks();
  let contract = contract_with_two_variants(&mut engine);

  let hooks = json!({
    "hooks": {
      "state-setup": [
        { "name": "fixtures",
          "run": { "kind": "http", "url": format!("{}/_pact/provider-states", provider.base_url()),
                   "format": "pact-state-change" } } ],
      "state-teardown": [
        { "name": "cleanup",
          "run": { "kind": "exec", "command": "/bin/sh", "args": [fixture("fail.sh")],
                   "env": { "PATH": "/usr/bin:/bin" } } } ]
    }
  });
  let events = verify(&mut engine, contract, provider.base_url(), Some(hooks));

  for result in results(&events) {
    assert_eq!(
      result["status"],
      json!("verified"),
      "a teardown failure cannot retroactively change what the provider answered"
    );
  }
  let teardowns: Vec<&Value> = summary(&events)["hooks"]["invocations"]
    .as_array()
    .unwrap()
    .iter()
    .filter(|i| i["hook"] == json!("cleanup"))
    .collect();
  assert_eq!(teardowns.len(), 2);
  assert_eq!(teardowns[0]["outcome"], json!("failed"));
  assert_eq!(
    teardowns[0]["effect"],
    json!("warned"),
    "reported loudly, because leaked state poisons later exchanges — but still a warning"
  );
  assert_eq!(summary(&events)["status"], json!("verified"));
}

#[test]
fn a_selector_runs_a_state_handler_only_for_its_own_state() {
  let provider = provider_without_auth();
  let mut engine = engine_with_hooks();
  let contract = contract_with_two_variants(&mut engine);

  let hooks = json!({
    "hooks": { "state-setup": [
      { "name": "orders",
        "when": { "state": "an order exists" },
        "run": { "kind": "http", "url": format!("{}/_pact/provider-states", provider.base_url()),
                 "format": "pact-state-change" } },
      { "name": "somebody-elses-state",
        "when": { "state": "a shipment exists" },
        "run": { "kind": "exec", "command": "/bin/sh", "args": [fixture("fail.sh")],
                 "env": { "PATH": "/usr/bin:/bin" } } } ] }
  });
  let events = verify(&mut engine, contract, provider.base_url(), Some(hooks));

  assert_eq!(summary(&events)["status"], json!("verified"));
  let invocations = summary(&events)["hooks"]["invocations"].as_array().unwrap();
  assert!(
    invocations
      .iter()
      .all(|i| i["hook"] != json!("somebody-elses-state")),
    "a hook that did not run is absent — which is how a wrong selector is spotted: {invocations:?}"
  );
  assert_eq!(
    invocations
      .iter()
      .filter(|i| i["hook"] == json!("orders"))
      .count(),
    2
  );
}

// ---------------------------------------------------------------------------------------------
// Refusals before the run starts
// ---------------------------------------------------------------------------------------------

#[test]
fn an_implementation_this_engine_cannot_run_is_refused_by_name_before_the_run() {
  let provider = provider_without_auth();
  let mut engine = engine_without_hooks();
  let contract = contract_with_two_variants(&mut engine);

  let refused = send(
    &mut engine,
    "v-1",
    "verification/verify",
    json!({
      "source": { "kind": "inline", "contracts": [contract] },
      "target": {
        "transports": [ { "transport": "http", "options": { "base-url": provider.base_url() } } ],
        "hooks": { "hooks": { "before-request": [
          { "name": "sign", "run": { "kind": "exec", "command": "/bin/true" } } ] } }
      }
    }),
  );
  assert_eq!(refused["error"]["code"], "hook-unavailable");
  assert_eq!(refused["error"]["category"], "component");
  assert_eq!(refused["error"]["details"]["kind"], "exec");
  assert_eq!(refused["error"]["details"]["hook"], "sign");
  assert_eq!(
    refused["error"]["details"]["implementations"],
    json!(["script"]),
    "the answer names what this engine *can* run — `script` always, because the interpreter \
     compiles with the engine (ADR 0015) — rather than skipping the hook and running anyway"
  );
}

#[test]
fn a_configuration_that_names_a_point_this_engine_does_not_have_is_refused_with_a_pointer() {
  let provider = provider_without_auth();
  let mut engine = engine_with_hooks();
  let contract = contract_with_two_variants(&mut engine);

  let refused = send(
    &mut engine,
    "v-1",
    "verification/verify",
    json!({
      "source": { "kind": "inline", "contracts": [contract] },
      "target": {
        "transports": [ { "transport": "http", "options": { "base-url": provider.base_url() } } ],
        "hooks": { "hooks": { "before-lunch": [
          { "name": "sign", "run": { "kind": "exec", "command": "/bin/true" } } ] } }
      }
    }),
  );
  assert_eq!(refused["error"]["code"], "hook-config-invalid");
  assert_eq!(refused["error"]["category"], "document");
  assert_eq!(
    refused["error"]["details"]["problems"][0]["pointer"],
    "/hooks/before-lunch"
  );
}

#[test]
fn hook_activity_arrives_as_events_in_run_order() {
  let provider = provider_without_auth();
  let mut engine = engine_with_hooks();
  let contract = contract_with_two_variants(&mut engine);

  let hooks = json!({
    "hooks": {
      "state-setup": [
        { "name": "fixtures",
          "run": { "kind": "http", "url": format!("{}/_pact/provider-states", provider.base_url()),
                   "format": "pact-state-change" } } ],
      "after-response": [
        { "name": "observe",
          "run": { "kind": "exec", "command": "/bin/sh", "args": [fixture("observe.sh")],
                   "env": { "PATH": "/usr/bin:/bin" } } } ],
      "state-teardown": [
        { "name": "cleanup",
          "run": { "kind": "exec", "command": "/bin/sh", "args": [fixture("observe.sh")],
                   "env": { "PATH": "/usr/bin:/bin" } } } ]
    }
  });
  let events = verify(&mut engine, contract, provider.base_url(), Some(hooks));

  // One exchange's worth of the §4.1 shape, in the order the spec draws it.
  let names: Vec<&str> = events
    .iter()
    .filter(|e| {
      matches!(
        e["kind"].as_str(),
        Some("verification/hook")
          | Some("verification/interaction-started")
          | Some("verification/interaction-result")
      )
    })
    .map(|e| match e["kind"].as_str() {
      Some("verification/hook") => e["payload"]["hook"].as_str().unwrap(),
      Some("verification/interaction-started") => "started",
      _ => "result",
    })
    .collect();
  assert_eq!(
    &names[..5],
    &["started", "fixtures", "observe", "result", "cleanup"],
    "state setup, the exchange, after-response, the result, then teardown: {names:?}"
  );

  // Every invocation is an event and every event is in the report — one shape, two places.
  let report = summary(&events)["hooks"]["invocations"].as_array().unwrap().len();
  assert_eq!(hook_events(&events).len(), report);
}

// ---------------------------------------------------------------------------------------------
// The component implementation (plan task 5.3's "oauth2-shaped built-in component")
// ---------------------------------------------------------------------------------------------

#[test]
fn a_component_hook_acquires_a_credential_once_and_presents_it_on_every_request() {
  // Auth on, and no token anywhere in the configuration: the component fetches one from the
  // provider's own token endpoint, at `before-verification`, and spends it at every exchange.
  let provider = start_provider(ProviderConfig::default()).expect("the sample provider binds");
  let mut engine = engine_with_the_oauth2_component();
  let contract = contract_with_two_variants(&mut engine);

  let credentials = json!({
    "token-url": format!("{}/oauth/token", provider.base_url()),
    "client-id": pact_janus_sample_order_service::DEFAULT_CLIENT_ID,
    "client-secret": pact_janus_sample_order_service::DEFAULT_CLIENT_SECRET,
  });
  let hooks = json!({
    "version": 1,
    "hooks": {
      "before-verification": [
        { "name": "auth", "run": { "kind": "component", "component": "oauth2" },
          "config": credentials } ],
      "state-setup": [
        { "name": "fixtures",
          "run": { "kind": "http", "url": format!("{}/_pact/provider-states", provider.base_url()),
                   "format": "pact-state-change" } } ],
      "before-request": [
        { "name": "auth", "run": { "kind": "component", "component": "oauth2" },
          "config": credentials, "changes": ["parts.request.headers"] } ]
    }
  });

  let events = verify(&mut engine, contract, provider.base_url(), Some(hooks));
  for result in results(&events) {
    assert_eq!(
      result["status"],
      json!("verified"),
      "the component's credential got every request past the provider's auth: {result}"
    );
  }

  let invocations = summary(&events)["hooks"]["invocations"].as_array().unwrap();
  let auth: Vec<&Value> = invocations
    .iter()
    .filter(|i| i["hook"] == json!("auth"))
    .collect();
  assert_eq!(auth.len(), 3, "once at the run point, once per exchange");
  assert_eq!(auth[0]["implementation"], json!("component"));
  assert_eq!(auth[0]["point"], json!("before-verification"));
  assert_eq!(auth[0]["changed"], json!([]), "acquiring changes nothing");
  assert_eq!(auth[1]["changed"], json!(["parts.request.headers"]));

  let rendered = serde_json::to_string(&summary(&events)["hooks"]).unwrap();
  assert!(
    !rendered.contains(DEFAULT_TOKEN) && !rendered.contains("janus-demo-secret"),
    "neither the token nor the client secret reaches the report: {rendered}"
  );
}

#[test]
fn a_component_hook_that_was_never_registered_is_refused_before_the_run() {
  let provider = provider_without_auth();
  // `engine_with_hooks` registers the exec and http kinds but no components at all.
  let mut engine = engine_with_hooks();
  let contract = contract_with_two_variants(&mut engine);

  let refused = send(
    &mut engine,
    "v-1",
    "verification/verify",
    json!({
      "source": { "kind": "inline", "contracts": [contract] },
      "target": {
        "transports": [ { "transport": "http", "options": { "base-url": provider.base_url() } } ],
        "hooks": { "hooks": { "before-request": [
          { "name": "auth", "run": { "kind": "component", "component": "oauth2" } } ] } }
      }
    }),
  );
  assert_eq!(refused["error"]["code"], "hook-unavailable");
  assert_eq!(refused["error"]["details"]["kind"], "component");
  assert_eq!(refused["error"]["details"]["hook"], "auth");
}

// ---------------------------------------------------------------------------------------------
// The scripted implementation (plan task 5.3's stretch: the third kind, end to end)
// ---------------------------------------------------------------------------------------------

#[test]
fn a_scripted_hook_signs_every_replayed_request() {
  let provider = provider_without_auth();
  let mut engine = engine_with_hooks();
  let contract = contract_with_two_variants(&mut engine);

  // The script the sample project ships, loaded the way a project loads it: by path, inlined by
  // the loader, so no file reference crosses the engine boundary.
  let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    .join("../../samples/order-service")
    .canonicalize()
    .expect("the sample provider's directory");
  let config = format!(
    r#"
hooks:
  state-setup:
    - name: fixtures
      run: {{ kind: http, url: "{}/_pact/provider-states", format: pact-state-change }}
  before-request:
    - name: correlation-id
      run: {{ kind: script, path: ./hooks/correlation-id.js }}
      config: {{ prefix: order-service }}
      changes: ["parts.request.headers"]
"#,
    provider.base_url()
  );
  let resolved = loader::load_document(&config, &repo, &HashMap::new()).expect("the config resolves");
  assert!(
    resolved["hooks"]["before-request"][0]["run"]["source"]
      .as_str()
      .is_some_and(|source| source.contains("x-correlation-id")),
    "the loader inlined the script: {resolved}"
  );

  let events = verify(&mut engine, contract, provider.base_url(), Some(resolved));
  for result in results(&events) {
    assert_eq!(result["status"], json!("verified"), "{result}");
  }
  let stamped: Vec<&Value> = summary(&events)["hooks"]["invocations"]
    .as_array()
    .unwrap()
    .iter()
    .filter(|i| i["hook"] == json!("correlation-id"))
    .collect();
  assert_eq!(stamped.len(), 2, "once per exchange");
  assert_eq!(stamped[0]["implementation"], json!("script"));
  assert_eq!(stamped[0]["changed"], json!(["parts.request.headers"]));
}

#[test]
fn a_scripted_hook_is_available_even_when_the_embedding_registered_nothing() {
  // `script` needs no registration: the interpreter compiles with the engine, which is the whole
  // reason ADR 0015 chose it over faster, native-only options.
  let provider = provider_without_auth();
  let mut engine = engine_without_hooks();
  let contract = contract_with_two_variants(&mut engine);

  let hooks = json!({
    "hooks": { "before-request": [
      { "name": "stamp",
        "run": { "kind": "script",
                 "source": "function hook(ctx) { const h = janus.json(ctx.parts.request.headers) || {};                             h['x-janus'] = [ctx.variant.id];                             return { outcome: 'ok', changes: { 'parts.request.headers': janus.slot(h) } }; }" },
        "changes": ["parts.request.headers"] } ] }
  });
  let events = verify(&mut engine, contract, provider.base_url(), Some(hooks));
  let invocations = summary(&events)["hooks"]["invocations"].as_array().unwrap();
  assert_eq!(invocations.len(), 2);
  assert_eq!(invocations[0]["outcome"], json!("ok"));
  assert_eq!(invocations[0]["changed"], json!(["parts.request.headers"]));
}

#[test]
fn a_runaway_script_is_stopped_by_its_deadline_and_fails_the_exchange() {
  let provider = provider_without_auth();
  let mut engine = engine_with_hooks();
  let contract = contract_with_two_variants(&mut engine);

  let hooks = json!({
    "hooks": { "before-request": [
      { "name": "spinner",
        "run": { "kind": "script", "source": "function hook(ctx) { while (true) {} }" },
        "timeout-ms": 50 } ] }
  });
  let events = verify(&mut engine, contract, provider.base_url(), Some(hooks));

  let results = results(&events);
  assert_eq!(results[0]["status"], json!("failed"));
  let invocation = &summary(&events)["hooks"]["invocations"][0];
  assert_eq!(
    invocation["outcome"],
    json!("timed-out"),
    "a hook that spins must not hang a verification (spec §9.5)"
  );
  assert_eq!(invocation["effect"], json!("failed-exchange"));
}
