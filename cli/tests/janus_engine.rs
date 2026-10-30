//! The subprocess embedding's end of engine-protocol spec §6: `engine/shutdown` is answered, and
//! then the process exits — cleanly, so a host can tell it from a crash. And the other way out,
//! which is the one that matters when nobody says goodbye: stdin closing (§7.1).

use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn frame(body: &Value) -> Vec<u8> {
  let bytes = serde_json::to_vec(body).expect("json");
  let mut out = format!("Content-Length: {}\r\n\r\n", bytes.len()).into_bytes();
  out.extend(bytes);
  out
}

fn read_frame(reader: &mut impl BufRead) -> Value {
  let mut length = 0;
  loop {
    let mut line = String::new();
    reader.read_line(&mut line).expect("header line");
    let line = line.trim_end();
    if line.is_empty() {
      break;
    }
    if let Some(value) = line.strip_prefix("Content-Length:") {
      length = value.trim().parse().expect("length");
    }
  }
  let mut body = vec![0; length];
  reader.read_exact(&mut body).expect("body");
  serde_json::from_slice(&body).expect("frame is JSON")
}

#[test]
fn shutdown_is_answered_and_then_the_process_exits_cleanly() {
  let mut child = Command::new(env!("CARGO_BIN_EXE_janus-engine"))
    .env("RUST_LOG", "warn")
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::null())
    .spawn()
    .expect("janus-engine starts");
  let mut stdin = child.stdin.take().expect("stdin");
  let mut stdout = BufReader::new(child.stdout.take().expect("stdout"));

  let hello =
    json!({ "type": "request", "id": "r-1", "op": "engine/hello", "body": { "protocol-versions": [1] } });
  stdin.write_all(&frame(&hello)).expect("write hello");
  assert_eq!(read_frame(&mut stdout)["ok"]["protocol-version"], 1);

  let shutdown = json!({ "type": "request", "id": "r-2", "op": "engine/shutdown", "body": {} });
  stdin.write_all(&frame(&shutdown)).expect("write shutdown");
  let response = read_frame(&mut stdout);
  assert_eq!(response["id"], "r-2");
  assert_eq!(response["ok"], json!({}));

  // stdin is still open: the exit is the shutdown's doing, not EOF's.
  let deadline = Instant::now() + Duration::from_secs(10);
  let status = loop {
    if let Some(status) = child.try_wait().expect("try_wait") {
      break status;
    }
    assert!(
      Instant::now() < deadline,
      "janus-engine did not exit after engine/shutdown"
    );
    std::thread::sleep(Duration::from_millis(20));
  };
  assert!(status.success(), "exit status {status}");
  drop(stdin);
}

/// Waits for the engine to exit, failing with `what` if it has not within ten seconds.
fn wait_for_exit(child: &mut std::process::Child, what: &str) -> std::process::ExitStatus {
  let deadline = Instant::now() + Duration::from_secs(10);
  loop {
    if let Some(status) = child.try_wait().expect("try_wait") {
      return status;
    }
    if Instant::now() >= deadline {
      let _ = child.kill();
      panic!("{what}");
    }
    std::thread::sleep(Duration::from_millis(20));
  }
}

/// Engine-protocol spec §7.1, staged plan S0.2: stdin reaching EOF is how the engine learns its host
/// is gone — a test runner that was killed sends no `engine/shutdown`. It is the whole of orphan
/// prevention (no process groups, no signals), and spike 1.3 left it unproven on Windows, where a
/// pipe closing is not a signal either. With a mock transport serving, because that is when there
/// are threads that could keep the process alive.
#[test]
fn stdin_closing_ends_the_process_even_with_a_transport_serving() {
  let mut child = Command::new(env!("CARGO_BIN_EXE_janus-engine"))
    .env("RUST_LOG", "warn")
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::null())
    .spawn()
    .expect("janus-engine starts");
  let mut stdin = child.stdin.take().expect("stdin");
  let mut stdout = BufReader::new(child.stdout.take().expect("stdout"));
  let mut send = |id: &str, op: &str, body: Value| {
    let request = json!({ "type": "request", "id": id, "op": op, "body": body });
    stdin.write_all(&frame(&request)).expect("write a frame");
    let response = read_frame(&mut stdout);
    assert_eq!(response["id"], id, "{response}");
    assert!(response.get("error").is_none(), "{op}: {response}");
    response["ok"].clone()
  };

  send("r-1", "engine/hello", json!({ "protocol-versions": [1] }));
  let created = send(
    "r-2",
    "consumer-session/create",
    json!({ "config": { "consumer": { "name": "web-app" }, "provider": { "name": "order-api" } } }),
  );
  let started = send(
    "r-3",
    "consumer-session/start-transport",
    json!({ "session": created["session"], "transport": "http" }),
  );
  assert!(
    started["endpoint"]["base-url"].is_string(),
    "a mock is listening: {started}"
  );

  // No `engine/shutdown`: the host just goes away.
  drop(stdin);
  let status = wait_for_exit(
    &mut child,
    "janus-engine was still running ten seconds after its stdin closed",
  );
  assert!(status.success(), "exit status {status}");
}

/// Phase-9 finding 30: `janus-engine` is the engine both SDKs embed, and it registered no hook
/// implementations, so a verifier configuration naming an `http` or `exec` hook — the sample
/// provider's own — was refused by it and accepted by `janus`. Both now start from one function.
#[test]
fn a_verification_naming_an_http_hook_is_not_refused() {
  assert_hook_kind_accepted(
    "http",
    json!({ "kind": "http", "url": "http://127.0.0.1:1/state" }),
  );
}

#[test]
fn a_verification_naming_an_exec_hook_is_not_refused() {
  assert_hook_kind_accepted("exec", json!({ "kind": "exec", "command": "true" }));
}

fn assert_hook_kind_accepted(kind: &str, run: Value) {
  let mut child = Command::new(env!("CARGO_BIN_EXE_janus-engine"))
    .env("RUST_LOG", "warn")
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::null())
    .spawn()
    .expect("janus-engine starts");
  let mut stdin = child.stdin.take().expect("stdin");
  let mut stdout = BufReader::new(child.stdout.take().expect("stdout"));

  let hello =
    json!({ "type": "request", "id": "r-1", "op": "engine/hello", "body": { "protocol-versions": [1] } });
  stdin.write_all(&frame(&hello)).expect("write hello");
  read_frame(&mut stdout);

  let pact: Value = serde_json::from_str(include_str!(
    "../../samples/order-service/pacts/web-app-order-service.json"
  ))
  .expect("the sample pact is JSON");
  let verify = json!({ "type": "request", "id": "r-2", "op": "verification/verify", "body": {
    "source": { "kind": "inline", "contracts": [pact] },
    "target": {
      "transports": [ { "transport": "http", "options": { "base-url": "http://127.0.0.1:1" } } ],
      "hooks": { "hooks": { "before-request": [ { "name": "sign", "run": run } ] } }
    } } });
  stdin.write_all(&frame(&verify)).expect("write verify");
  let response = read_frame(&mut stdout);
  assert_ne!(
    response["error"]["code"],
    json!("hook-unavailable"),
    "{kind} hooks refused: {response}"
  );
  drop(stdin);
  let _ = child.wait();
}

/// `config/load` (ADR 0055): the subprocess engine resolves a configuration file with its own
/// environment, which is the one its host started it with, and says it can in the handshake.
#[test]
fn config_load_resolves_a_file_with_the_engines_environment() {
  let dir = std::env::temp_dir().join(format!("janus-config-load-{}", std::process::id()));
  std::fs::create_dir_all(&dir).expect("temp dir");
  let file = dir.join("verifier.janus.yaml");
  std::fs::write(
    &file,
    "version: 1\nhooks:\n  state-setup:\n    - name: fixtures\n      run: { kind: http, url: \"${STATE_URL}/state\" }\n",
  )
  .expect("write the configuration");

  let mut child = Command::new(env!("CARGO_BIN_EXE_janus-engine"))
    .env("RUST_LOG", "warn")
    .env("STATE_URL", "http://127.0.0.1:9999")
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::null())
    .spawn()
    .expect("janus-engine starts");
  let mut stdin = child.stdin.take().expect("stdin");
  let mut stdout = BufReader::new(child.stdout.take().expect("stdout"));

  let hello =
    json!({ "type": "request", "id": "r-1", "op": "engine/hello", "body": { "protocol-versions": [1] } });
  stdin.write_all(&frame(&hello)).expect("write hello");
  assert_eq!(
    read_frame(&mut stdout)["ok"]["capabilities"]["config-loading"],
    json!({})
  );

  let load = json!({ "type": "request", "id": "r-2", "op": "config/load",
    "body": { "path": file.display().to_string() } });
  stdin.write_all(&frame(&load)).expect("write config/load");
  let response = read_frame(&mut stdout);
  assert_eq!(
    response["ok"]["config"]["hooks"]["state-setup"][0]["run"]["url"],
    json!("http://127.0.0.1:9999/state"),
    "{response}"
  );
  drop(stdin);
  let _ = child.wait();
  let _ = std::fs::remove_dir_all(&dir);
}
