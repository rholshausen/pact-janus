//! The bridge as the engine meets it: a process spoken to in component frames over stdin and stdout
//! (component-interfaces spec §9.3), started with an empty environment plus what a declaration grants.
//!
//! The plugin is `tests/fixtures/plugins/lines-0.1.0`, a Lua plugin pact-plugin-driver runs inside the
//! bridge's process, copied into a plugin directory of the test's own, so these tests download nothing
//! and run on every operating system. The real plugins are held to the bridge's conformance cases
//! (`conformance/`), which CI runs with `janus component check` against pinned binaries.
//!
//! Two tests need a plugin that is a process of its own: one kills the pinned CSV plugin, and one drives
//! the pinned protobuf plugin's gRPC transport against the area calculator (`examples/area_calculator.rs`,
//! which `cargo test` builds). They use the plugins in the directory `JANUS_PACT_PLUGINS` names
//! (`.github/scripts/pact-plugins.sh` installs it), and skip without one unless `CI` is set.

use pretty_assertions::assert_eq;
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::time::{Duration, Instant};

struct Bridge {
  child: Child,
  stdin: Option<ChildStdin>,
  stdout: BufReader<ChildStdout>,
  next: u64,
  _plugins: Option<tempfile::TempDir>,
}

impl Bridge {
  /// A bridge for `args`, with a plugin directory holding the fixture plugins.
  fn start(args: &[&str]) -> Bridge {
    let plugins = tempfile::tempdir().unwrap();
    let mut bridge = Bridge::start_in(plugins.path(), args);
    bridge._plugins = Some(plugins);
    bridge
  }

  /// A bridge for `args`, with `plugins` as its plugin directory, holding the fixture plugins too.
  fn start_in(plugins: &Path, args: &[&str]) -> Bridge {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/plugins");
    copy_dir(&fixtures, plugins);
    // A plugin whose manifest is found and whose program is not: it fails to start without the driver
    // looking in the plugin index, which a missing manifest would send it to.
    let broken = plugins.join("broken-0.1.0");
    std::fs::create_dir_all(&broken).unwrap();
    std::fs::write(
      broken.join("pact-plugin.json"),
      r#"{ "manifestVersion": 1, "pluginInterfaceVersion": 1, "name": "broken", "version": "0.1.0",
           "executableType": "exec", "entryPoint": "no-such-program" }"#,
    )
    .unwrap();

    let mut command = Command::new(env!("CARGO_BIN_EXE_janus-plugin-bridge"));
    command
      .args(args)
      .env_clear()
      .env("PACT_PLUGIN_DIR", plugins)
      .env("PACT_DO_NOT_TRACK", "true")
      .stdin(Stdio::piped())
      .stdout(Stdio::piped())
      .stderr(Stdio::inherit());
    // As the subprocess loader does (spec §9.3): Winsock does not start without it.
    if let Some(root) = std::env::var_os("SystemRoot") {
      command.env("SystemRoot", root);
    }
    let mut child = command.spawn().unwrap();
    let stdin = child.stdin.take();
    let stdout = BufReader::new(child.stdout.take().unwrap());
    Bridge {
      child,
      stdin,
      stdout,
      next: 0,
      _plugins: None,
    }
  }

  fn lines() -> Bridge {
    let mut bridge = Bridge::start(&["lines", "0.1.0"]);
    let hello = bridge.call("component/hello", json!({ "component-protocol-versions": [1] }));
    assert!(hello.get("ok").is_some(), "{hello}");
    bridge
  }

  /// The response frame to one request.
  fn call(&mut self, op: &str, body: Value) -> Value {
    self.next += 1;
    let id = self.next.to_string();
    self.send(&serde_json::to_vec(&json!({ "type": "request", "id": id, "op": op, "body": body })).unwrap());
    let response = self.receive();
    assert_eq!(response["id"], json!(id), "{response}");
    assert_eq!(response["type"], "response", "{response}");
    response
  }

  fn send(&mut self, body: &[u8]) {
    let stdin = self.stdin.as_mut().unwrap();
    write!(stdin, "Content-Length: {}\r\n\r\n", body.len()).unwrap();
    stdin.write_all(body).unwrap();
    stdin.flush().unwrap();
  }

  fn receive(&mut self) -> Value {
    let mut length = None;
    loop {
      let mut line = String::new();
      assert!(
        self.stdout.read_line(&mut line).unwrap() > 0,
        "the bridge closed stdout"
      );
      let line = line.trim_end();
      if line.is_empty() {
        break;
      }
      let (header, value) = line.split_once(':').expect("a header line");
      if header.eq_ignore_ascii_case("content-length") {
        length = Some(value.trim().parse::<usize>().unwrap());
      }
    }
    let mut body = vec![0; length.expect("a Content-Length")];
    self.stdout.read_exact(&mut body).unwrap();
    serde_json::from_slice(&body).unwrap()
  }

  fn apply(&mut self, actual: &str, argument: Value) -> Value {
    self.call(
      "matcher/apply",
      json!({
        "action": "pact-plugin-lines:compare-contents",
        "values": [{ "content": actual, "encoded": "text" }],
        "config": { "arguments": [argument] }
      }),
    )
  }
}

impl Drop for Bridge {
  /// Ends the bridge as the engine does: stdin closed, so it shuts its plugin down; killed only if it does
  /// not leave, since a killed bridge leaves a plugin process behind (the loader ends the process group).
  fn drop(&mut self) {
    drop(self.stdin.take());
    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(5) {
      if let Ok(Some(_)) = self.child.try_wait() {
        return;
      }
      std::thread::sleep(Duration::from_millis(20));
    }
    let _ = self.child.kill();
    let _ = self.child.wait();
  }
}

fn copy_dir(from: &Path, to: &Path) {
  std::fs::create_dir_all(to).unwrap();
  for entry in std::fs::read_dir(from).unwrap() {
    let entry = entry.unwrap();
    let target = to.join(entry.file_name());
    if entry.file_type().unwrap().is_dir() {
      copy_dir(&entry.path(), &target);
    } else {
      std::fs::copy(entry.path(), target).unwrap();
    }
  }
}

fn lines_argument(expected: &str) -> Value {
  json!({ "content-type": "text/x-lines", "expected": { "content": expected, "encoded": "text" } })
}

#[test]
fn the_handshake_is_the_plugins_catalogue_translated() {
  let mut bridge = Bridge::start(&["lines", "0.1.0"]);
  let hello = bridge.call(
    "component/hello",
    json!({
      "component-protocol-versions": [1],
      "engine": { "name": "janus-engine", "version": "0.8.0" },
      "grants": { "env": ["PACT_PLUGIN_DIR"], "fs": [], "network": false },
      "capabilities": {}
    }),
  );
  let mut ok = hello["ok"].clone();
  // The bridge's own version is its release's, not this test's to pin.
  assert_eq!(
    ok["capabilities"]["pact-plugin"]["bridge"]["name"],
    "janus-plugin-bridge"
  );
  ok["capabilities"]["pact-plugin"]
    .as_object_mut()
    .unwrap()
    .remove("bridge");
  assert_eq!(
    ok,
    json!({
      "component-protocol-version": 1,
      "component": { "name": "pact-plugin-lines", "version": "0.1.0" },
      "interfaces": ["matcher", "transport"],
      "contributes": {
        "actions": [
          { "name": "pact-plugin-lines:compare-contents" },
          { "name": "pact-plugin-lines:verdict" }
        ],
        "transports": [{
          "kind": "lines",
          "roles": ["drive"],
          "exchange-parts": { "initiating": "request", "answering": "response" }
        }]
      },
      "capabilities": {
        "pact-plugin": {
          "name": "lines",
          "version": "0.1.0",
          "interface-version": 1,
          "catalogue": [
            {
              "type": "content-matcher",
              "key": "lines",
              "content-types": ["text/x-lines", "application/x-lines\\+text"],
              "action": "pact-plugin-lines:compare-contents",
              "values": { "content-types": "text/x-lines;application/x-lines\\+text" }
            },
            {
              "type": "transport",
              "key": "lines",
              "kind": "lines",
              "action": "pact-plugin-lines:verdict",
              "values": {}
            }
          ]
        }
      }
    })
  );
  // A second handshake is answered as the first was: the plugin is loaded once.
  let again = bridge.call("component/hello", json!({ "component-protocol-versions": [1] }));
  assert_eq!(again["ok"]["component"], hello["ok"]["component"]);
}

#[test]
fn nothing_is_answered_before_the_handshake() {
  let mut bridge = Bridge::start(&["lines", "0.1.0"]);
  let response = bridge.call(
    "matcher/apply",
    json!({ "action": "pact-plugin-lines:compare-contents", "values": [] }),
  );
  assert_eq!(response["error"]["code"], "handshake-required");
  assert_eq!(response["error"]["category"], "protocol");
}

#[test]
fn a_protocol_version_it_does_not_speak_is_refused_with_the_ones_it_does() {
  let mut bridge = Bridge::start(&["lines", "0.1.0"]);
  let response = bridge.call("component/hello", json!({ "component-protocol-versions": [2] }));
  assert_eq!(response["error"]["code"], "protocol-version-unsupported");
  assert_eq!(response["error"]["details"]["supported"], json!([1]));
}

#[test]
fn equal_bodies_are_ok_and_different_ones_are_one_mismatch_naming_each_difference() {
  let mut bridge = Bridge::lines();
  let same = bridge.apply("a\nb\n", lines_argument("a\nb\n"));
  assert_eq!(same["ok"], json!({ "results": [{ "status": "ok" }] }));

  let different = bridge.apply("a\nc\nd\n", lines_argument("x\nb\n"));
  assert_eq!(
    different["ok"]["results"][0],
    json!({
      "status": "error",
      "message": "$: Expected 2 line(s) but got 3; $[0]: Expected 'x' but got 'a'; $[1]: Expected 'b' but got 'c'",
      "mismatches": [
        { "message": "Expected 2 line(s) but got 3", "at": "$", "expected": "2", "actual": "3" },
        { "message": "Expected 'x' but got 'a'", "at": "$[0]", "expected": "x", "actual": "a" },
        { "message": "Expected 'b' but got 'c'", "at": "$[1]", "expected": "b", "actual": "c" }
      ]
    })
  );
}

#[test]
fn the_pacts_rules_and_the_plugins_configuration_reach_the_plugin() {
  let mut bridge = Bridge::lines();
  let ruled = bridge.apply(
    "order-42\n",
    json!({
      "content-type": "text/x-lines;charset=utf-8",
      "expected": { "content": "order-1\n", "encoded": "text" },
      "rules": { "$[0]": { "combine": "AND", "matchers": [{ "match": "regex", "regex": "order%-%d+" }] } }
    }),
  );
  assert_eq!(ruled["ok"]["results"][0], json!({ "status": "ok" }), "{ruled}");

  let configured = bridge.apply(
    "HELLO\n",
    json!({
      "content-type": "application/x-lines+text",
      "expected": { "content": "aGVsbG8K", "encoded": "base64" },
      "plugin-configuration": { "interaction": { "ignore-case": true }, "pact": {} }
    }),
  );
  assert_eq!(
    configured["ok"]["results"][0],
    json!({ "status": "ok" }),
    "{configured}"
  );
}

#[test]
fn what_the_bridge_cannot_answer_is_an_error_in_the_specifications_words() {
  let mut bridge = Bridge::lines();
  let unknown = bridge.call(
    "matcher/apply",
    json!({ "action": "pact-plugin-lines:other", "values": [{ "content": "" }] }),
  );
  assert_eq!(unknown["error"]["code"], "unknown-action");
  assert_eq!(unknown["error"]["details"]["action"], "pact-plugin-lines:other");

  let unclaimed = bridge.apply("", json!({ "content-type": "text/csv" }));
  assert_eq!(unclaimed["error"]["code"], "unsupported-content-type");
  assert_eq!(unclaimed["error"]["details"]["content-type"], "text/csv");

  let unread = bridge.apply("", json!({ "expected": { "content": "" } }));
  assert_eq!(unread["error"]["code"], "invalid-config");

  let compare = bridge.call(
    "matcher/compare",
    json!({ "operator": "x", "provider": {}, "consumer": {} }),
  );
  assert_eq!(compare["error"]["code"], "operation-unsupported");
  assert_eq!(compare["error"]["details"]["op"], "matcher/compare");

  let shutdown = bridge.call("component/shutdown", json!({}));
  assert_eq!(shutdown["ok"], json!({}));
}

#[test]
fn a_frame_that_is_not_a_request_is_malformed_and_the_stream_stays_in_sync() {
  let mut bridge = Bridge::lines();
  bridge.send(b"not json");
  let response = bridge.receive();
  assert_eq!(response["error"]["code"], "malformed-frame");
  bridge.send(br#"{ "type": "request", "id": "x", "op": "matcher/apply" }"#);
  let response = bridge.receive();
  assert_eq!(response["id"], "x");
  assert_eq!(response["error"]["code"], "malformed-frame");
  let still = bridge.call("component/shutdown", json!({}));
  assert_eq!(still["ok"], json!({}));
}

#[test]
fn a_plugin_that_cannot_be_loaded_is_unavailable_with_its_cause() {
  let mut bridge = Bridge::start(&["broken", "0.1.0"]);
  let response = bridge.call("component/hello", json!({ "component-protocol-versions": [1] }));
  let error = &response["error"];
  assert_eq!(error["code"], "unavailable", "{response}");
  assert_eq!(error["category"], "component");
  assert_eq!(
    error["details"],
    json!({ "plugin": "broken", "version": "0.1.0" })
  );
  let chain = error["chain"].as_array().unwrap();
  assert_eq!(
    chain[0]["message"],
    "the pact plugin 'broken' version 0.1.0 could not be loaded"
  );
  assert!(chain.len() > 1, "{response}");
}

#[test]
fn arguments_that_name_no_usable_plugin_are_answered_at_the_handshake() {
  for args in [&["lines"][..], &["My_Plugin", "1.0.0"][..]] {
    let mut bridge = Bridge::start(args);
    let response = bridge.call("component/hello", json!({ "component-protocol-versions": [1] }));
    assert_eq!(response["error"]["code"], "unavailable", "{args:?}: {response}");
  }
}

#[test]
fn it_exits_when_stdin_ends() {
  let mut bridge = Bridge::lines();
  drop(bridge.stdin.take());
  let started = Instant::now();
  loop {
    if let Some(status) = bridge.child.try_wait().unwrap() {
      assert!(status.success(), "{status}");
      break;
    }
    assert!(
      started.elapsed() < Duration::from_secs(10),
      "the bridge outlived its stdin"
    );
    std::thread::sleep(Duration::from_millis(20));
  }
}

#[test]
fn version_and_help_are_for_a_person() {
  let output = Command::new(env!("CARGO_BIN_EXE_janus-plugin-bridge"))
    .arg("--version")
    .output()
    .unwrap();
  assert!(output.status.success());
  assert_eq!(
    String::from_utf8_lossy(&output.stdout).trim(),
    format!("janus-plugin-bridge {}", env!("CARGO_PKG_VERSION"))
  );
}

/// The pinned plugins' directory, or `None` when this machine has none and is not CI. Only the Unix test
/// that kills a plugin process needs one.
#[cfg(unix)]
fn pinned_plugins() -> Option<std::path::PathBuf> {
  match std::env::var_os("JANUS_PACT_PLUGINS") {
    Some(dir) => Some(dir.into()),
    None if std::env::var_os("CI").is_some() => {
      panic!("JANUS_PACT_PLUGINS names no plugin directory, and CI is set")
    }
    None => {
      eprintln!("skipped: JANUS_PACT_PLUGINS names no directory of the pinned plugins");
      None
    }
  }
}

/// ADR 0109 decision 6: a plugin the bridge cannot reach is the bridge failing, not the provider's body.
/// The call is answered `unavailable`, and the bridge leaves, so the engine's next call starts a new one.
#[cfg(unix)]
#[test]
fn a_plugin_that_cannot_be_reached_is_unavailable_and_the_bridge_leaves() {
  let Some(plugins) = pinned_plugins() else {
    return;
  };
  let mut bridge = Bridge::start_in(&plugins, &["csv", "0.1.0"]);
  let hello = bridge.call("component/hello", json!({ "component-protocol-versions": [1] }));
  assert!(hello.get("ok").is_some(), "{hello}");
  let argument = json!({
    "content-type": "text/csv",
    "expected": { "content": "a,1\n", "encoded": "text" },
    "plugin-configuration": { "interaction": { "csvHeaders": false }, "pact": {} }
  });
  let apply = |bridge: &mut Bridge| {
    bridge.call(
      "matcher/apply",
      json!({
        "action": "pact-plugin-csv:compare-contents",
        "values": [{ "content": "a,1\n", "encoded": "text" }],
        "config": { "arguments": [argument.clone()] }
      }),
    )
  };
  assert_eq!(apply(&mut bridge)["ok"]["results"][0], json!({ "status": "ok" }));

  // The plugin is the bridge's one child process.
  let children = Command::new("pgrep")
    .args(["-P", &bridge.child.id().to_string()])
    .output()
    .unwrap();
  let children = String::from_utf8_lossy(&children.stdout);
  let plugin = children.split_whitespace().next().expect("the plugin's process");
  assert!(
    Command::new("kill")
      .args(["-9", plugin])
      .status()
      .unwrap()
      .success()
  );
  std::thread::sleep(Duration::from_millis(200));

  let response = apply(&mut bridge);
  let error = &response["error"];
  assert_eq!(error["code"], "unavailable", "{response}");
  assert_eq!(error["category"], "component");
  assert_eq!(error["details"], json!({ "plugin": "csv", "version": "0.1.0" }));
  assert_eq!(
    error["chain"][0]["message"],
    "the pact plugin 'csv' version 0.1.0 could not be reached"
  );

  let started = Instant::now();
  loop {
    if let Some(status) = bridge.child.try_wait().unwrap() {
      assert!(!status.success(), "the bridge leaves as a failure: {status}");
      break;
    }
    assert!(
      started.elapsed() < Duration::from_secs(10),
      "the bridge stayed with its plugin gone"
    );
    std::thread::sleep(Duration::from_millis(20));
  }
}

/// The area calculator example, listening on a port of its own until it is dropped.
struct AreaCalculator {
  child: Child,
  port: u16,
}

impl AreaCalculator {
  fn start(answers: &str) -> AreaCalculator {
    let bridge = Path::new(env!("CARGO_BIN_EXE_janus-plugin-bridge"));
    let program = bridge
      .parent()
      .unwrap()
      .join("examples")
      .join(format!("area_calculator{}", std::env::consts::EXE_SUFFIX));
    let mut child = Command::new(&program)
      .args(["--answers", answers])
      .stdin(Stdio::piped())
      .stdout(Stdio::piped())
      .spawn()
      .unwrap_or_else(|err| panic!("{}: {err}", program.display()));
    let mut line = String::new();
    BufReader::new(child.stdout.take().unwrap())
      .read_line(&mut line)
      .unwrap();
    let port = line
      .trim()
      .strip_prefix("listening on ")
      .and_then(|port| port.parse().ok())
      .unwrap_or_else(|| panic!("the area calculator said '{line}'"));
    AreaCalculator { child, port }
  }
}

impl Drop for AreaCalculator {
  fn drop(&mut self) {
    drop(self.child.stdin.take());
    let _ = self.child.kill();
    let _ = self.child.wait();
  }
}

/// ADR 0111: a plugin's transport is a kind the bridge drives, and one send is the whole exchange. The
/// plugin builds the request from the pact, calls the provider, and its verdict is the reply, which the
/// bridge's `verdict` action reads.
#[test]
#[cfg(unix)]
fn a_plugin_transport_is_one_send_whose_reply_is_the_plugins_verdict() {
  let Some(plugins) = pinned_plugins() else {
    return;
  };
  let pact_file = Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("tests/fixtures/pacts/grpc-consumer-go-area-calculator-provider.json");
  let mut pact: Value = serde_json::from_slice(&std::fs::read(pact_file).unwrap()).unwrap();
  pact["interactions"][0]["key"] = json!("area-1");

  for (answers, ok) in [("areas", true), ("nothing", false)] {
    let calculator = AreaCalculator::start(answers);
    let mut bridge = Bridge::start_in(&plugins, &["protobuf", "0.8.1"]);
    let hello = bridge.call("component/hello", json!({ "component-protocol-versions": [1] }));
    assert_eq!(
      hello["ok"]["contributes"]["transports"][0]["kind"], "grpc",
      "{hello}"
    );

    let started = bridge.call(
      "transport/start",
      json!({ "instance": "t-1", "kind": "grpc", "role": "drive",
              "options": { "base-url": format!("http://127.0.0.1:{}", calculator.port) } }),
    );
    assert_eq!(
      started["ok"]["endpoint"],
      json!({ "host": "127.0.0.1", "port": calculator.port }),
      "{started}"
    );
    let sent = bridge.call(
      "transport/send",
      json!({ "instance": "t-1", "await-reply": true, "parts": { "request": {
        "pact": { "content": pact }, "key": { "content": "area-1" }, "provider-state": { "content": {} }
      } } }),
    );
    let verdict = &sent["ok"]["reply"]["response"]["verdict"]["content"];
    assert_eq!(verdict["ok"], json!(ok), "{answers}: {sent}");

    let read = bridge.call(
      "matcher/apply",
      json!({ "action": "pact-plugin-protobuf:verdict", "values": [{ "content": verdict }] }),
    );
    let result = &read["ok"]["results"][0];
    if ok {
      assert_eq!(result, &json!({ "status": "ok" }), "{read}");
    } else {
      assert_eq!(result["status"], "error", "{read}");
      assert_eq!(
        result["mismatches"][0],
        json!({ "message": "Expected message 'AreaResponse' but was missing or empty", "at": "$" })
      );
    }
    assert_eq!(
      bridge.call("transport/stop", json!({ "instance": "t-1" }))["ok"],
      json!({})
    );
  }

  // It drives the plugin's transports, of the kinds the plugin has, and serves none yet (S5.2f).
  let mut bridge = Bridge::start_in(&plugins, &["protobuf", "0.8.1"]);
  bridge.call("component/hello", json!({ "component-protocol-versions": [1] }));
  let serve = bridge.call(
    "transport/start",
    json!({ "instance": "t-1", "kind": "grpc", "role": "serve" }),
  );
  assert_eq!(serve["error"]["code"], "operation-unsupported", "{serve}");
  let unknown = bridge.call(
    "transport/start",
    json!({ "instance": "t-2", "kind": "kafka", "role": "drive" }),
  );
  assert_eq!(unknown["error"]["code"], "invalid-config", "{unknown}");
  let unstarted = bridge.call("transport/send", json!({ "instance": "t-3", "parts": {} }));
  assert_eq!(unstarted["error"]["code"], "invalid-config", "{unstarted}");
}
