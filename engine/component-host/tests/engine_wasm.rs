//! The engine as a WASM component (`engine/wasm`, ADR 0023 decision 2), as a release ships it:
//! built for `wasm32-wasip2`, instantiated under wasmtime with an empty WASI context, and spoken to
//! through the one function it exports.
//!
//! It is tested here because this crate is where the workspace's WASM runtime is. Nothing of the
//! loader is involved: the engine component is a guest of whoever hosts it, not a component the
//! engine loads.

use pretty_assertions::assert_eq;
use serde_json::{Value, json};
use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;
use wasmtime::component::{Component, Linker, ResourceTable, TypedFunc};
use wasmtime::{Engine, Store};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

/// The interface the shipped world exports (`engine/wasm/wit/engine.wit`).
const PIPE_INTERFACE: &str = "pact:janus-engine/pipe@0.1.0";

/// Built by the test itself, once per test binary, so it never runs a stale one — unless
/// `JANUS_ENGINE_WASM` names a component already built, which is how the release workflow runs
/// these tests against the bytes it is about to ship.
fn engine_wasm() -> &'static PathBuf {
  static WASM: OnceLock<PathBuf> = OnceLock::new();
  WASM.get_or_init(|| {
    if let Some(built) = std::env::var_os("JANUS_ENGINE_WASM") {
      return PathBuf::from(built);
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let status = Command::new(cargo)
      .args(["build", "--release", "--package", "pact_janus_engine_wasm"])
      .args(["--target", "wasm32-wasip2"])
      .current_dir(&root)
      // A host build's flags are not this target's.
      .env_remove("RUSTFLAGS")
      .status()
      .expect("cargo runs");
    assert!(status.success(), "building engine/wasm for wasm32-wasip2 failed");
    let target = std::env::var_os("CARGO_TARGET_DIR").map_or_else(|| root.join("target"), PathBuf::from);
    target.join("wasm32-wasip2/release/pact_janus_engine_wasm.wasm")
  })
}

struct Host {
  ctx: WasiCtx,
  table: ResourceTable,
}

impl WasiView for Host {
  fn ctx(&mut self) -> WasiCtxView<'_> {
    WasiCtxView {
      ctx: &mut self.ctx,
      table: &mut self.table,
    }
  }
}

/// One instance of the engine component: no environment, no files, no sockets.
struct Guest {
  store: Store<Host>,
  call: TypedFunc<(Vec<u8>,), (Vec<u8>,)>,
}

impl Guest {
  fn new() -> Guest {
    let engine = Engine::default();
    let bytes = std::fs::read(engine_wasm()).expect("the component was built");
    let component = Component::from_binary(&engine, &bytes).expect("it is a WASM component");
    let mut linker: Linker<Host> = Linker::new(&engine);
    wasmtime_wasi::p2::add_to_linker_sync(&mut linker).expect("WASI links");
    let host = Host {
      ctx: WasiCtxBuilder::new().build(),
      table: ResourceTable::new(),
    };
    let mut store = Store::new(&engine, host);
    let instance = linker
      .instantiate(&mut store, &component)
      .expect("it needs nothing but WASI");
    let call = component
      .get_export_index(None, PIPE_INTERFACE)
      .and_then(|interface| component.get_export_index(Some(&interface), "call"))
      .expect("it exports the pipe");
    let call = instance
      .get_typed_func(&mut store, call)
      .expect("call takes one frame and answers one");
    Guest { store, call }
  }

  fn send(&mut self, op: &str, body: Value) -> Value {
    let request = json!({ "type": "request", "id": "1", "op": op, "body": body });
    let (response,) = self
      .call
      .call(&mut self.store, (serde_json::to_vec(&request).unwrap(),))
      .expect("the guest does not trap");
    serde_json::from_slice(&response).expect("the answer is a frame")
  }

  fn hello(&mut self) -> Value {
    self.send(
      "engine/hello",
      json!({ "protocol-versions": [1], "host": { "name": "janus-test", "version": "0.0.0" }, "capabilities": {} }),
    )
  }
}

#[test]
fn it_answers_the_handshake_as_this_engine_version() {
  let hello = Guest::new().hello();
  assert_eq!(
    hello["ok"]["engine"],
    json!({ "name": "janus-engine", "version": pact_janus_kernel::ENGINE_VERSION })
  );
}

/// The world a release ships is the pipe and nothing else: the benchmark harness's interface is
/// behind a feature no release build turns on.
#[test]
fn it_exports_the_pipe_and_nothing_else() {
  let engine = Engine::default();
  let bytes = std::fs::read(engine_wasm()).expect("the component was built");
  let component = Component::from_binary(&engine, &bytes).expect("it is a WASM component");
  let exports: Vec<String> = component
    .component_type()
    .exports(&engine)
    .map(|(name, _)| name.to_string())
    .collect();
  assert_eq!(exports, vec![PIPE_INTERFACE.to_string()]);
}

/// An offline operation, end to end: a v3 pact goes in and a Janus contract comes out, with no
/// transport, thread or file involved.
#[test]
fn it_upgrades_a_pact_offline() {
  let mut guest = Guest::new();
  guest.hello();
  let pact = json!({
    "consumer": { "name": "web-app" },
    "provider": { "name": "order-service" },
    "interactions": [ {
      "description": "an order",
      "request": { "method": "GET", "path": "/orders/66" },
      "response": { "status": 200, "headers": { "Content-Type": "application/json" }, "body": { "id": 66 } }
    } ],
    "metadata": { "pactSpecification": { "version": "3.0.0" } }
  });
  let upgraded = guest.send("upgrade/pact", json!({ "pact": pact }));
  assert_eq!(
    upgraded["ok"]["contract"]["$format"],
    json!("janus-contract/1"),
    "{upgraded}"
  );
}

/// What it cannot do, it says by name (ADR 0023 decision 2): there is no transport in it.
#[test]
fn it_refuses_a_transport_by_name() {
  let mut guest = Guest::new();
  guest.hello();
  let created = guest.send(
    "consumer-session/create",
    json!({ "config": { "consumer": { "name": "web-app" }, "provider": { "name": "order-service" } } }),
  );
  let session = created["ok"]["session"].clone();
  let started = guest.send(
    "consumer-session/start-transport",
    json!({ "session": session, "transport": "http" }),
  );
  assert_eq!(
    started["error"]["code"],
    json!("component-unavailable"),
    "{started}"
  );
}

/// The engine as a broker would host it to decide `can-i-deploy` (staged plan S4.3a, ADR 0092):
/// the sample provider's pact checked against the shape its own tests recorded, then decided under
/// no policy, and under a policy that blocks on findings and exempts one, judged on the day before
/// the exemption lapses and the day after. Every frame of the conversation, the handshake included.
fn decide_conversation(mut send: impl FnMut(Value) -> Value) -> Vec<Value> {
  let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../samples/order-service");
  let read = |path: &str| -> Value {
    serde_json::from_slice(&std::fs::read(root.join(path)).expect("the sample provider's documents"))
      .expect("JSON")
  };
  let mut ids = 0;
  let mut request = |op: &str, body: Value| {
    ids += 1;
    send(json!({ "type": "request", "id": format!("r-{ids}"), "op": op, "body": body }))
  };

  let mut answers = vec![request(
    "engine/hello",
    json!({ "protocol-versions": [1], "host": { "name": "a-broker", "version": "0.0.0" }, "capabilities": {} }),
  )];
  let checked = request(
    "subsumption/check",
    json!({
      "contract": read("pacts/web-app-order-service.json"),
      "provider-shape": read("shapes/order-service.provider-shape.json"),
    }),
  );
  let pair = json!({
    "consumer": { "name": "web-app" },
    "provider": { "name": "order-service" },
    "subsumption": checked["ok"]["report"],
    "format": checked["ok"]["format"],
  });
  answers.push(checked);

  let policy = json!({
    "on-finding": "block",
    "exemptions": [ { "path": "response.body.status", "reason": "CANCELLED is a known future state", "expires": "2026-12-01" } ],
  });
  for (policy, as_of) in [
    (json!([]), "2026-11-15"),
    (json!([policy]), "2026-11-30"),
    (json!([policy]), "2026-12-02"),
  ] {
    answers.push(request(
      "subsumption/decide",
      json!({ "pairs": [pair], "verification": [], "policy": policy, "as-of": as_of }),
    ));
  }
  answers
}

/// What `janus check` answers: the kernel in this process, with the JSON content component and the
/// HTTP component, transport and matchers both, as `janus` registers them.
fn decided_natively() -> Vec<Value> {
  use pact_janus_component_http::{HttpTransport, status};
  use pact_janus_kernel::component::TransportComponent;
  use std::sync::Arc;

  let transports = std::collections::HashMap::from([(
    "http".to_string(),
    Arc::new(HttpTransport::new()) as Arc<dyn TransportComponent>,
  )]);
  let mut engine = pact_janus_kernel::protocol::Engine::with_components(
    transports,
    Some(Arc::new(pact_janus_component_json::JsonContent::new())),
  );
  engine.register_matcher(
    status::NAME,
    "1.0.0",
    status::ACTIONS,
    Arc::new(status::HttpMatcher::new()),
  );
  decide_conversation(|frame| {
    serde_json::from_slice(&engine.dispatch(&serde_json::to_vec(&frame).unwrap())).expect("a frame")
  })
}

fn reason_codes(decided: &Value) -> Vec<&str> {
  decided["ok"]["report"]["pairs"][0]["reasons"]
    .as_array()
    .expect("reasons")
    .iter()
    .map(|reason| reason["code"].as_str().expect("a code"))
    .collect()
}

/// The `wasmtime` crate is what the Rust port of the Pact Broker would host the component with: it
/// answers every frame of a decision exactly as the native engine does.
#[test]
fn under_the_wasmtime_crate_it_decides_as_the_native_engine_does() {
  let mut guest = Guest::new();
  let answers = decide_conversation(|frame| {
    let (response,) = guest
      .call
      .call(&mut guest.store, (serde_json::to_vec(&frame).unwrap(),))
      .expect("the guest does not trap");
    serde_json::from_slice(&response).expect("the answer is a frame")
  });
  let native = decided_natively();
  assert_eq!(answers, native);

  // And the conversation decides what it was written to: the exemption silences its finding until
  // the day after it lapses, with no document changing (broker notes §4).
  let [_, checked, unpoliced, exempted, lapsed] = &answers[..] else {
    panic!("five answers")
  };
  assert!(checked["ok"]["report"].is_object(), "{checked}");
  // Matched by operation, through the HTTP component's `http:path`: the three findings `janus check`
  // finds over the same files.
  assert_eq!(
    unpoliced["ok"]["report"]["pairs"][0]["subsumption"]["findings"],
    json!(3),
    "{unpoliced}"
  );
  assert!(
    reason_codes(unpoliced).contains(&"subsumption-findings"),
    "{unpoliced}"
  );
  assert!(
    reason_codes(exempted).contains(&"subsumption-exempt"),
    "{exempted}"
  );
  assert!(
    !reason_codes(exempted).contains(&"exemption-lapsed"),
    "{exempted}"
  );
  assert!(reason_codes(lapsed).contains(&"exemption-lapsed"), "{lapsed}");
  assert_eq!(lapsed["ok"]["report"]["decision"], json!("block"), "{lapsed}");
}

/// The sample provider's own tests, as S4.1 records them (`engine/kernel/tests/provider_shape_by_operation.rs`):
/// set up an order, fetch it, and once fetch one that does not exist. Each answer is fetched through
/// the HTTP transport's drive role, so it is the `response` part as the transport presents it
/// (component-interfaces spec §5.9), without the `Date` header, which is the clock. The provider sets
/// no cookie, so the 404 is given a `Set-Cookie`, as a session layer in front of it would add one: a
/// credential the kind declares (ADR 0074), recorded only as present.
fn the_sample_providers_answers() -> Vec<(&'static str, Value)> {
  use pact_janus_component_http::HttpTransport;
  use pact_janus_kernel::component::{Part, Send, SlotValue, Start, Stop, TransportComponent};
  use pact_janus_sample_order_service::{Config, DEFAULT_TOKEN, start};

  let provider = start(Config::default()).expect("an OS-assigned port always binds");
  let transport = HttpTransport::new();
  transport
    .start(Start {
      instance: "provider".to_string(),
      kind: "http".to_string(),
      role: "drive".to_string(),
      options: Some(json!({ "base-url": provider.base_url() })),
    })
    .expect("a drive instance starts");
  let slot = |content: Value, content_type: Option<&str>| SlotValue {
    content,
    encoded: None,
    content_type: content_type.map(str::to_string),
  };
  let send = |method: &str, path: &str, body: Option<Value>| -> Value {
    let mut request = Part::from([
      ("method".to_string(), slot(json!(method), None)),
      ("path".to_string(), slot(json!(path), None)),
      (
        "headers".to_string(),
        slot(
          json!({ "authorization": [format!("Bearer {DEFAULT_TOKEN}")] }),
          None,
        ),
      ),
    ]);
    if let Some(body) = body {
      request.insert(
        "body".to_string(),
        slot(json!(body.to_string()), Some("application/json")),
      );
    }
    let reply = transport
      .send(Send {
        instance: "provider".to_string(),
        parts: [("request".to_string(), request)].into(),
        await_reply: true,
        timeout_ms: Some(5_000),
      })
      .expect("the provider answers")
      .reply
      .expect("an HTTP request is answered");
    let mut response = serde_json::to_value(&reply["response"]).expect("a part serializes");
    response["headers"]["content"]
      .as_object_mut()
      .expect("headers by name")
      .remove("date");
    response
  };

  let mut answers = Vec::new();
  for (status, shipped, items) in [
    ("PENDING", false, 1),
    ("SHIPPED", true, 2),
    ("PENDING", false, 3),
    ("CANCELLED", false, 0),
    ("SHIPPED", true, 1),
    ("PENDING", false, 2),
  ] {
    send(
      "POST",
      "/_pact/provider-states",
      Some(json!({ "action": "setup", "state": "an order exists",
                   "params": { "id": "66", "status": status, "shipped": shipped, "items": items } })),
    );
    answers.push(("/orders/66", send("GET", "/orders/66", None)));
  }
  send(
    "POST",
    "/_pact/provider-states",
    Some(json!({ "action": "setup", "state": "no orders exist" })),
  );
  let mut missing = send("GET", "/orders/999", None);
  missing["headers"]["content"]["set-cookie"] = json!(["session=s3cr3t; HttpOnly"]);
  answers.push(("/orders/999", missing));
  transport
    .stop(Stop {
      instance: "provider".to_string(),
    })
    .expect("the instance stops");
  answers
}

/// A recording as a host that embeds the engine makes one (staged plan S4.9, ADR 0094): the handshake,
/// a session for the provider, one observation per answer, each naming its operation as the sample
/// provider's own tests do, with the request it answered, and the shape `finalise` answers.
fn record_conversation(answers: &[(&str, Value)], mut send: impl FnMut(Value) -> Value) -> Vec<Value> {
  let mut ids = 0;
  let mut request = |op: &str, body: Value| {
    ids += 1;
    send(json!({ "type": "request", "id": format!("r-{ids}"), "op": op, "body": body }))
  };
  let mut frames = vec![request(
    "engine/hello",
    json!({ "protocol-versions": [1], "host": { "name": "a-recorder", "version": "0.0.0" }, "capabilities": {} }),
  )];
  let created = request(
    "provider-shape-session/create",
    json!({ "provider": { "name": "order-service" } }),
  );
  let session = created["ok"]["session"].clone();
  frames.push(created);
  for (path, response) in answers {
    frames.push(request(
      "provider-shape-session/observe",
      json!({
        "session": session,
        "description": "GET /orders/{id}",
        "operation": { "request": {
          "method": { "shape": "equality", "example": "GET" },
          "path": { "shape": "http:path", "template": "/orders/{id}", "example": "/orders/id" } } },
        "parts": {
          "request": { "method": { "content": "GET" }, "path": { "content": path },
                       "headers": { "content": { "authorization": ["Bearer a-token"] } } },
          "response": response,
        },
      }),
    ));
  }
  frames.push(request(
    "provider-shape-session/finalise",
    json!({ "session": session }),
  ));
  frames
}

/// What `janus-engine` answers: the kernel in this process, with the HTTP component's transport and
/// matchers and the JSON content component.
fn native_engine() -> pact_janus_kernel::protocol::Engine {
  use pact_janus_component_http::{HttpTransport, status};
  use pact_janus_kernel::component::TransportComponent;
  use std::sync::Arc;

  let transports = std::collections::HashMap::from([(
    "http".to_string(),
    Arc::new(HttpTransport::new()) as Arc<dyn TransportComponent>,
  )]);
  let mut engine = pact_janus_kernel::protocol::Engine::with_components(
    transports,
    Some(Arc::new(pact_janus_component_json::JsonContent::new())),
  );
  engine.register_matcher(
    status::NAME,
    "1.0.0",
    status::ACTIONS,
    Arc::new(status::HttpMatcher::new()),
  );
  engine
}

/// The artifact registers no transport, but holds what the kind `http` declares (ADR 0094): the
/// sample provider's answers recorded through it give the shape the native engine records from the
/// same observations, frame for frame, its `Set-Cookie` recorded as present and nothing more.
#[test]
fn under_the_wasmtime_crate_it_records_a_provider_shape_as_the_native_engine_does() {
  let answers = the_sample_providers_answers();
  let mut guest = Guest::new();
  let recorded = record_conversation(&answers, |frame| {
    let (response,) = guest
      .call
      .call(&mut guest.store, (serde_json::to_vec(&frame).unwrap(),))
      .expect("the guest does not trap");
    serde_json::from_slice(&response).expect("the answer is a frame")
  });
  let mut engine = native_engine();
  let native = record_conversation(&answers, |frame| {
    serde_json::from_slice(&engine.dispatch(&serde_json::to_vec(&frame).unwrap())).expect("a frame")
  });
  assert_eq!(recorded, native);

  // And the shape is what S4.1 records: the order and the error document apart, by status, the
  // cookie a credential whose value went nowhere.
  let shape = &recorded.last().expect("finalise answered")["ok"]["provider-shape"];
  let entries = shape["interactions"].as_array().expect("entries");
  let statuses: Vec<&Value> = entries
    .iter()
    .map(|entry| &entry["selector"]["response"]["status"]["example"])
    .collect();
  assert_eq!(statuses, vec![&json!(200), &json!(404)], "{shape:#}");
  let missing = &entries[1]["parts"]["response"]["headers"]["members"]["set-cookie"];
  assert_eq!(missing, &json!({ "shape": "any" }), "{shape:#}");
  assert!(!shape.to_string().contains("s3cr3t"), "{shape:#}");
  assert!(!recorded.iter().any(|frame| frame.to_string().contains("s3cr3t")));
}

/// What the engine records redacts what the kind `http` declares a credential (ADR 0094 decision
/// 3), in the artifact as natively, though there it has no transport to ask: a pact upgraded there
/// records its `Authorization` header as `[redacted]` (ADR 0074).
#[test]
fn under_the_wasmtime_crate_an_upgraded_pact_redacts_the_credentials_http_declares() {
  let pact = json!({
    "consumer": { "name": "web-app" },
    "provider": { "name": "order-service" },
    "interactions": [ {
      "description": "an order",
      "request": { "method": "GET", "path": "/orders/66", "headers": { "Authorization": "Bearer s3cr3t" } },
      "response": { "status": 200, "headers": { "Set-Cookie": "session=s3cr3t" } }
    } ],
    "metadata": { "pactSpecification": { "version": "3.0.0" } }
  });
  let mut guest = Guest::new();
  guest.hello();
  let upgraded = guest.send("upgrade/pact", json!({ "pact": pact }));
  let mut engine = native_engine();
  let mut native = |op: &str, body: Value| -> Value {
    let frame = json!({ "type": "request", "id": "1", "op": op, "body": body });
    serde_json::from_slice(&engine.dispatch(&serde_json::to_vec(&frame).unwrap())).expect("a frame")
  };
  native(
    "engine/hello",
    json!({ "protocol-versions": [1], "host": { "name": "janus-test", "version": "0.0.0" }, "capabilities": {} }),
  );
  assert_eq!(upgraded, native("upgrade/pact", json!({ "pact": pact })));
  let contract = upgraded["ok"]["contract"].to_string();
  assert!(contract.contains("[redacted]"), "{upgraded:#}");
  assert!(!contract.contains("s3cr3t"), "{upgraded:#}");
}

/// Ruby's `wasmtime` gem is what the Ruby Pact Broker would host the component with — the one claim
/// the broker notes (§3c) took on report. `engine/wasm/hosts/ruby/pipe.rb` is that host, given each
/// frame as a line; it answers every frame exactly as the native engine does.
///
/// `JANUS_RUBY` names the Ruby to run it with, which must find the gems of the Gemfile beside it
/// (`bundle install` there); unset, this test says so and passes, as the broker tests do with no
/// broker. CI's Linux job sets it.
#[test]
fn under_rubys_wasmtime_gem_it_decides_as_the_native_engine_does() {
  use std::io::{BufRead, BufReader, Write};

  let Some(ruby) = std::env::var_os("JANUS_RUBY").filter(|ruby| !ruby.is_empty()) else {
    eprintln!("not run: JANUS_RUBY names no Ruby with engine/wasm/hosts/ruby's gems");
    return;
  };
  let host = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../wasm/hosts/ruby");
  let mut child = Command::new(ruby)
    .arg("-rbundler/setup")
    .arg(host.join("pipe.rb"))
    .arg(engine_wasm())
    .env("BUNDLE_GEMFILE", host.join("Gemfile"))
    .stdin(std::process::Stdio::piped())
    .stdout(std::process::Stdio::piped())
    .spawn()
    .expect("JANUS_RUBY runs");
  let mut stdin = child.stdin.take().expect("piped");
  let mut stdout = BufReader::new(child.stdout.take().expect("piped"));
  let answers = decide_conversation(|frame| {
    writeln!(stdin, "{frame}").expect("the host reads a frame");
    stdin.flush().expect("flushed");
    let mut line = String::new();
    stdout.read_line(&mut line).expect("the host answers");
    serde_json::from_str(&line).unwrap_or_else(|err| panic!("{err}: the host answered {line:?}"))
  });
  drop(stdin);
  assert!(child.wait().expect("the host exits").success());
  assert_eq!(answers, decided_natively());
}
