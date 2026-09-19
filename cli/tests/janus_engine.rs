//! The subprocess embedding's end of engine-protocol spec §6: `engine/shutdown` is answered, and
//! then the process exits — cleanly, so a host can tell it from a crash.

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
