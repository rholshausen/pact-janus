//! The subprocess binding (component-interfaces spec §9.3, staged plan S5.2b, ADR 0108), against two
//! components written in JavaScript: `third-party/janus-csv-js`, the CSV component written again, which
//! must pass the Rust component's own conformance cases through this binding; and
//! `tests/fixtures/subprocess/misbehaving.mjs`, built to break the binding's rules, which holds the loader
//! to spike 8.3's containment obligations. Each test skips without Node, unless `CI` is set.

mod support;

use pact_janus_component_host::SubprocessLoader;
use pact_janus_kernel::component::conformance::{self, Binding, Case};
use pact_janus_kernel::component::{
  ComponentDeclaration, ComponentError, ComponentLoader, ContentComponent, Decode, Loaded, PollInbound,
  SlotValue, Start,
};
use pretty_assertions::assert_eq;
use serde_json::{Value, json};
use std::net::TcpStream;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use support::{gone_within, node, repo};

fn fixtures() -> PathBuf {
  PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/subprocess")
}

/// The misbehaving component, declared as a project would: Node, its script relative to its directory.
/// A deadline for the tests whose point is a call that never answers. Every handshake has to fit in it
/// too, Node starting included, and a CI runner busy with a cold build has taken more than 300 ms to
/// start one. The tests that time a hang assert only that it ends well before they would.
const HANG_DEADLINE_MS: u64 = 2_000;

fn misbehaving(node: &PathBuf, deadline_ms: u64, extra: &[&str]) -> ComponentDeclaration {
  let mut args = vec!["misbehaving.mjs".to_string()];
  args.extend(extra.iter().map(|arg| arg.to_string()));
  serde_json::from_value(json!({
    "name": "misbehaving",
    "source": { "kind": "subprocess", "reference": node, "args": args, "cwd": fixtures() },
    "limits": { "deadline-ms": deadline_ms },
  }))
  .unwrap()
}

fn load(declaration: &ComponentDeclaration) -> Loaded {
  SubprocessLoader::new().load(declaration).expect("it loads")
}

fn decode(content: &dyn ContentComponent, content_type: &str) -> Result<Value, ComponentError> {
  content
    .decode(Decode {
      content_type: content_type.to_string(),
      value: SlotValue {
        content: json!(""),
        encoded: Some("text".to_string()),
        content_type: None,
      },
      options: None,
      embedded: None,
    })
    .map(|result| result.document.to_json())
}

fn pid(content: &dyn ContentComponent) -> u64 {
  decode(content, "x/pid")
    .expect("it answers")
    .as_u64()
    .expect("a pid")
}

#[test]
fn a_component_written_in_javascript_passes_the_csv_components_own_cases() {
  let Some(node) = node() else { return };
  let declaration: ComponentDeclaration = serde_json::from_value(json!({
    "name": "csv",
    "source": { "kind": "subprocess", "reference": node, "args": ["csv.mjs"], "cwd": repo("third-party/janus-csv-js") },
  }))
  .unwrap();
  let loaded = load(&declaration);
  let binding = Binding {
    name: "subprocess".to_string(),
    content: loaded.content,
    matcher: loaded.matcher,
    hello: Some(loaded.hello),
  };
  let mut cases: Vec<PathBuf> = std::fs::read_dir(repo("third-party/janus-csv/conformance"))
    .unwrap()
    .map(|entry| entry.unwrap().path())
    .collect();
  cases.sort();
  assert_eq!(cases.len(), 12, "the CSV component's cases");
  for path in cases {
    let case = Case::read(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let report = conformance::run(&case, std::slice::from_ref(&binding));
    assert!(report.passed(), "{}: {:#?}", path.display(), report.problems);
  }
}

/// Spec §3.4 out of process (ADR 0108 decision 3): a call past its deadline ends the process, the error is
/// the engine's, and the next call gets a new process, handshaken first.
#[test]
fn a_call_that_never_answers_is_a_timeout_and_the_next_call_gets_a_new_process() {
  let Some(node) = node() else { return };
  let loaded = load(&misbehaving(&node, HANG_DEADLINE_MS, &[]));
  let content = loaded.content.expect("content");
  let first = pid(content.as_ref());

  let started = Instant::now();
  let timed_out = decode(content.as_ref(), "x/hang").expect_err("it never answers");
  assert_eq!(timed_out.code, "component-timeout", "{timed_out:?}");
  assert_eq!(timed_out.source.as_deref(), Some("engine"));
  assert!(
    started.elapsed() < Duration::from_secs(5),
    "{:?}",
    started.elapsed()
  );
  assert!(
    gone_within(first as u32, Duration::from_secs(5)),
    "the process was killed"
  );

  let second = pid(content.as_ref());
  assert_ne!(first, second, "a new process answers");
}

/// A component that never reads its stdin again, because its event loop is spinning, is bounded the same
/// way: the deadline is the engine's, not the component's.
#[test]
fn a_busy_loop_is_bounded_the_same_way() {
  let Some(node) = node() else { return };
  let loaded = load(&misbehaving(&node, HANG_DEADLINE_MS, &[]));
  let content = loaded.content.expect("content");
  let spun = decode(content.as_ref(), "x/spin").expect_err("it never answers");
  assert_eq!(spun.code, "component-timeout");
  assert_eq!(
    decode(content.as_ref(), "x/fine").expect("a new process"),
    json!("ok")
  );
}

/// Stdout closing is the signal: a process that exits fails its call at once, not at its deadline.
#[test]
fn a_process_that_exits_is_component_exited_at_once() {
  let Some(node) = node() else { return };
  let loaded = load(&misbehaving(&node, 10_000, &[]));
  let content = loaded.content.expect("content");
  let started = Instant::now();
  let exited = decode(content.as_ref(), "x/exit").expect_err("it exits");
  assert_eq!(exited.code, "component-exited", "{exited:?}");
  assert!(
    started.elapsed() < Duration::from_secs(5),
    "at once, not at the 10 s deadline: {:?}",
    started.elapsed()
  );
  assert_eq!(
    decode(content.as_ref(), "x/fine").expect("a new process"),
    json!("ok")
  );
}

/// ADR 0108 decision 5: a frame that is no call's, while exactly one call waits, fails that call at once,
/// and the process, whose stream is still in sync, answers the next.
#[test]
fn a_frame_that_is_not_json_fails_the_only_waiting_call_at_once() {
  let Some(node) = node() else { return };
  let loaded = load(&misbehaving(&node, 10_000, &[]));
  let content = loaded.content.expect("content");
  let before = pid(content.as_ref());
  let started = Instant::now();
  let malformed = decode(content.as_ref(), "x/malformed").expect_err("not a frame");
  assert_eq!(malformed.code, "component-malformed", "{malformed:?}");
  assert!(
    started.elapsed() < Duration::from_secs(5),
    "{:?}",
    started.elapsed()
  );
  assert_eq!(pid(content.as_ref()), before, "the same process answers");
}

/// An author's debug print on stdout is skipped, and the stream stays in sync (spike 8.3 finding 8).
#[test]
fn stray_output_on_stdout_is_skipped_and_the_stream_stays_in_sync() {
  let Some(node) = node() else { return };
  let loaded = load(&misbehaving(&node, 5_000, &[]));
  let content = loaded.content.expect("content");
  let before = pid(content.as_ref());
  assert_eq!(
    decode(content.as_ref(), "x/stray").expect("answered"),
    json!("ok")
  );
  assert_eq!(pid(content.as_ref()), before);
}

/// Dropping the component closes its stdin, and the process exits (spec §9.3).
#[test]
fn dropping_the_component_closes_its_stdin_and_the_process_exits() {
  let Some(node) = node() else { return };
  let loaded = load(&misbehaving(&node, 5_000, &[]));
  let content = loaded.content.expect("content");
  let pid = pid(content.as_ref()) as u32;
  drop(content);
  drop(loaded.transport);
  assert!(
    gone_within(pid, Duration::from_secs(5)),
    "the process exited on EOF"
  );
}

/// Spec §9.3: an empty environment plus exactly the `env` grant, so the component cannot read what the
/// engine had and did not grant (no `PATH`, no `HOME`); and on Windows, `SystemRoot`.
#[test]
fn the_environment_is_exactly_the_env_grant() {
  let Some(node) = node() else { return };
  let mut declaration = misbehaving(&node, 5_000, &[]);
  // Cargo sets it for every test process it runs.
  declaration.grants.env = vec!["CARGO_PKG_NAME".to_string(), "JANUS_NOT_SET_ANYWHERE".to_string()];
  let loaded = load(&declaration);
  let names = decode(loaded.content.expect("content").as_ref(), "x/env").expect("answered");
  // macOS's CoreFoundation sets `__CF_USER_TEXT_ENCODING` inside every process that starts without it,
  // so the component has it however empty an environment it was given: the platform's, not the engine's.
  let names: Vec<&str> = names
    .as_array()
    .expect("names")
    .iter()
    .filter_map(Value::as_str)
    .filter(|name| !(cfg!(target_os = "macos") && *name == "__CF_USER_TEXT_ENCODING"))
    .collect();
  let mut expected = vec!["CARGO_PKG_NAME"];
  if cfg!(windows) {
    expected.push("SystemRoot");
  }
  assert_eq!(names, expected);
}

/// ADR 0108 decision 2: the process starts in its `cwd`.
#[test]
fn the_process_starts_in_its_cwd() {
  let Some(node) = node() else { return };
  let loaded = load(&misbehaving(&node, 5_000, &[]));
  let cwd = decode(loaded.content.expect("content").as_ref(), "x/cwd").expect("answered");
  assert_eq!(
    PathBuf::from(cwd.as_str().unwrap()).canonicalize().unwrap(),
    fixtures().canonicalize().unwrap()
  );
}

/// Finding 18, as ADR 0108 decisions 3 and 4 settle it: one hung call ends the process, and with it every
/// transport instance it held, whose listening sockets close. The error names them, a later call on one
/// fails at once saying it was lost, and a new instance starts on a new process.
#[test]
fn one_hung_call_takes_every_instance_in_the_process_and_says_which() {
  let Some(node) = node() else { return };
  let loaded = load(&misbehaving(&node, HANG_DEADLINE_MS, &[]));
  let transport = loaded.transport.expect("a transport");
  let start = |instance: &str| {
    let started = transport
      .start(Start {
        instance: instance.to_string(),
        kind: "line".to_string(),
        role: "serve".to_string(),
        options: None,
      })
      .expect("it starts");
    started.endpoint["port"].as_u64().expect("a port") as u16
  };
  let one = start("t-1");
  start("t-2");
  assert!(TcpStream::connect(("127.0.0.1", one)).is_ok(), "t-1 listens");

  let hung = transport
    .send(pact_janus_kernel::component::Send {
      instance: "t-2".to_string(),
      parts: Default::default(),
      await_reply: true,
      timeout_ms: None,
    })
    .expect_err("send never answers");
  assert_eq!(hung.code, "component-timeout", "{hung:?}");
  let details = hung.details.as_deref().expect("details");
  assert_eq!(details["instances"], json!(["t-1", "t-2"]), "{details}");
  assert!(hung.message.contains("t-1, t-2"), "{}", hung.message);

  assert!(
    TcpStream::connect(("127.0.0.1", one)).is_err(),
    "t-1's socket went with the process"
  );
  let lost = transport
    .poll_inbound(PollInbound {
      instance: "t-1".to_string(),
      timeout_ms: 0,
    })
    .expect_err("t-1 is gone");
  assert_eq!(lost.code, "component-exited");
  let details = lost.details.as_deref().expect("details");
  assert_eq!(details["lost"], json!(true), "{details}");
  assert_eq!(details["instances"], json!(["t-1"]));

  let three = start("t-3");
  assert!(
    TcpStream::connect(("127.0.0.1", three)).is_ok(),
    "a new process serves t-3"
  );
}

/// A poll that waits is not a slow call: its own `timeout-ms` is added to the deadline (ADR 0108
/// decision 5).
#[test]
fn a_poll_that_waits_longer_than_the_deadline_is_not_a_timeout() {
  let Some(node) = node() else { return };
  let loaded = load(&misbehaving(&node, HANG_DEADLINE_MS, &[]));
  let transport = loaded.transport.expect("a transport");
  transport
    .start(Start {
      instance: "t-1".to_string(),
      kind: "line".to_string(),
      role: "serve".to_string(),
      options: None,
    })
    .expect("it starts");
  let polled = transport
    .poll_inbound(PollInbound {
      instance: "t-1".to_string(),
      timeout_ms: HANG_DEADLINE_MS + 500,
    })
    .expect("a poll that waited longer than the deadline");
  assert_eq!(polled.inbound, None);
}

/// ADR 0108 decision 3: a new process that introduces itself as another version is not this component.
#[test]
fn a_process_that_comes_back_as_another_version_is_refused() {
  let Some(node) = node() else { return };
  let marker = std::env::temp_dir().join(format!("janus-changes-version-{}", std::process::id()));
  let _ = std::fs::remove_file(&marker);
  let marker_arg = marker.display().to_string();
  let loaded = load(&misbehaving(&node, 5_000, &["--changes-version", &marker_arg]));
  assert_eq!(loaded.hello["component"]["version"], json!("1.0.0"));
  let content = loaded.content.expect("content");
  decode(content.as_ref(), "x/exit").expect_err("it exits");
  let refused = decode(content.as_ref(), "x/fine").expect_err("another version answered");
  let _ = std::fs::remove_file(&marker);
  assert_eq!(refused.code, "component-malformed", "{refused:?}");
  assert!(refused.message.contains("2.0.0"), "{}", refused.message);
}

#[test]
fn a_subprocess_source_takes_no_digest() {
  let Some(node) = node() else { return };
  let mut declaration = misbehaving(&node, 5_000, &[]);
  declaration.source.digest = Some("sha256:00".to_string());
  let Err(refused) = SubprocessLoader::new().load(&declaration) else {
    panic!("a digest nothing checks is refused");
  };
  assert!(
    refused.message.contains("takes no 'digest'"),
    "{}",
    refused.message
  );
}

#[test]
fn a_program_that_is_not_there_fails_the_load_naming_it() {
  let declaration: ComponentDeclaration = serde_json::from_value(json!({
    "name": "nothing",
    "source": { "kind": "subprocess", "reference": "./not-a-program", "cwd": fixtures() },
  }))
  .unwrap();
  let Err(refused) = SubprocessLoader::new().load(&declaration) else {
    panic!("there is no such program");
  };
  assert_eq!(refused.code, "unavailable");
  assert!(refused.message.contains("does not exist"), "{}", refused.message);
}

/// The loader through the engine (ADR 0108 decision 8): `engine/hello` lists it, and a consumer session
/// that declares the JavaScript CSV component resolves it. `cli/tests` runs one through `janus` and
/// `janus-engine`.
#[test]
fn an_engine_with_the_loader_lists_it_and_resolves_a_subprocess_component() {
  let Some(node) = node() else { return };
  let mut engine = pact_janus_kernel::protocol::Engine::new();
  engine.register_component_loader(std::sync::Arc::new(SubprocessLoader::new()));
  let send = |engine: &mut pact_janus_kernel::protocol::Engine, op: &str, body: Value| -> Value {
    let request = json!({ "type": "request", "id": "1", "op": op, "body": body });
    serde_json::from_slice(&engine.dispatch(&serde_json::to_vec(&request).unwrap())).unwrap()
  };
  let hello = send(
    &mut engine,
    "engine/hello",
    json!({ "protocol-versions": [1], "client": { "name": "test", "version": "0" } }),
  );
  assert_eq!(
    hello["ok"]["capabilities"]["components"]["loaders"],
    json!(["in-tree", "subprocess"]),
    "{hello}"
  );
  let created = send(
    &mut engine,
    "consumer-session/create",
    json!({ "config": { "consumer": { "name": "c" }, "provider": { "name": "p" },
      "components": [ { "name": "csv", "source": { "kind": "subprocess", "reference": node, "args": ["csv.mjs"],
                                                   "cwd": repo("third-party/janus-csv-js") } } ] } }),
  );
  assert!(created.get("ok").is_some(), "{created}");
}
