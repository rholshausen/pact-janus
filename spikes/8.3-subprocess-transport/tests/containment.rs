//! The subprocess binding's containment obligations (component-interfaces spec §3.4, §9.3, §11.2)
//! against a component built to break them (`component/misbehaving.mjs`), and the grants question
//! plan task 8.3 names: what, out of process, can actually be enforced?

use pact_janus_kernel::component::{
  ComponentDeclaration, ComponentError, ComponentLoader, Grants, Limits, Loaded, Source, Start,
  TransportComponent,
};
use pretty_assertions::assert_eq;
use serde_json::{Value, json};
use spike_subprocess_transport::SubprocessLoader;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

fn command(script: &str) -> String {
  let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    .join("component")
    .join(script);
  format!("node {}", path.display())
}

fn load(script: &str, grants: Grants, deadline_ms: Option<u64>) -> Loaded {
  SubprocessLoader
    .load(&ComponentDeclaration {
      name: script.trim_end_matches(".mjs").to_string(),
      source: Source {
        kind: "subprocess".to_string(),
        reference: Some(command(script)),
        digest: None,
      },
      grants,
      limits: Limits {
        deadline_ms,
        instances: None,
      },
    })
    .unwrap_or_else(|err| panic!("{err:?}"))
}

fn transport(loaded: &Loaded) -> Arc<dyn TransportComponent> {
  loaded.transport.clone().expect("declared transport")
}

fn start(transport: &dyn TransportComponent, instance: &str, what: &str) -> Result<Value, ComponentError> {
  transport
    .start(Start {
      instance: instance.to_string(),
      kind: "misbehaving".to_string(),
      role: "serve".to_string(),
      options: Some(json!({ "do": what })),
    })
    .map(|started| started.endpoint)
}

fn alive(pid: u64) -> bool {
  if cfg!(windows) {
    let out = Command::new("tasklist")
      .args(["/FI", &format!("PID eq {pid}"), "/NH"])
      .output()
      .unwrap();
    String::from_utf8_lossy(&out.stdout).contains(&pid.to_string())
  } else {
    Command::new("kill")
      .args(["-0", &pid.to_string()])
      .stderr(Stdio::null())
      .status()
      .unwrap()
      .success()
  }
}

fn gone_within(pid: u64, limit: Duration) -> Option<Duration> {
  let started = Instant::now();
  while started.elapsed() < limit {
    if !alive(pid) {
      return Some(started.elapsed());
    }
    std::thread::sleep(Duration::from_millis(10));
  }
  None
}

#[test]
fn a_call_that_never_answers_is_timed_out_and_its_process_killed() {
  let loaded = load("misbehaving.mjs", Grants::default(), Some(300));
  let t = transport(&loaded);
  let first_pid = start(t.as_ref(), "t-1", "pid").unwrap()["pid"].as_u64().unwrap();

  let began = Instant::now();
  let err = start(t.as_ref(), "t-2", "hang").unwrap_err();
  assert_eq!(err.code, "component-timeout", "{err:?}");
  assert_eq!(err.source.as_deref(), Some("engine"));
  assert!(began.elapsed() < Duration::from_secs(2), "{:?}", began.elapsed());
  assert!(
    gone_within(first_pid, Duration::from_secs(2)).is_some(),
    "killed, not left running"
  );

  // The next call gets a new process, handshaken before it is handed anything.
  let second_pid = start(t.as_ref(), "t-3", "pid").unwrap()["pid"].as_u64().unwrap();
  assert_ne!(first_pid, second_pid);
}

#[test]
fn a_component_that_stops_reading_stdin_is_still_bounded() {
  // A busy loop: no event loop, so no reading — the case EOF cannot help with (spike 1.3 finding 2).
  let loaded = load("misbehaving.mjs", Grants::default(), Some(300));
  let t = transport(&loaded);
  let err = start(t.as_ref(), "t-1", "spin").unwrap_err();
  assert_eq!(err.code, "component-timeout", "{err:?}");
  assert!(start(t.as_ref(), "t-2", "pid").is_ok());
}

#[test]
fn a_component_that_exits_is_reported_at_once_not_at_its_deadline() {
  let loaded = load("misbehaving.mjs", Grants::default(), Some(5_000));
  let t = transport(&loaded);
  let began = Instant::now();
  let err = start(t.as_ref(), "t-1", "exit").unwrap_err();
  assert_eq!(err.code, "component-exited", "{err:?}");
  assert!(
    began.elapsed() < Duration::from_secs(1),
    "stdout closing is the signal, not the 5 s deadline: {:?}",
    began.elapsed()
  );
  assert!(start(t.as_ref(), "t-2", "pid").is_ok(), "and it is started again");
}

#[test]
fn a_garbage_frame_cannot_be_pinned_on_its_call_so_it_costs_the_process() {
  let loaded = load("misbehaving.mjs", Grants::default(), Some(300));
  let t = transport(&loaded);
  let pid = start(t.as_ref(), "t-1", "pid").unwrap()["pid"].as_u64().unwrap();
  let err = start(t.as_ref(), "t-2", "garbage").unwrap_err();
  // A body that is not JSON carries no id, so it cannot be pinned on the call that caused it: that
  // call learns at its deadline, and the error says a frame went unattributed meanwhile.
  assert_eq!(err.code, "component-timeout", "{err:?}");
  assert_eq!(err.details.unwrap()["unattributed-frames"], 1);
  // The deadline killed the process — the only way to bound a call out of process — so the
  // stream's own resynchronisation is not what the next call sees; a new process is.
  let next = start(t.as_ref(), "t-3", "pid").unwrap()["pid"].as_u64().unwrap();
  assert_ne!(pid, next);
}

#[test]
fn one_hung_call_takes_every_instance_in_the_process_with_it() {
  // Spec §3.4 says a timed-out instance is recreated. Out of process the only way to stop a call is
  // to stop the process, so a hang on instance t-2 ends t-1 too — here, a listening socket some
  // other session was using. For a WASM content call (stateless, instance-per-call) "recreate" is
  // free; for a transport it is every live mock in the process.
  let loaded = load("misbehaving.mjs", Grants::default(), Some(300));
  let t = transport(&loaded);
  let port = start(t.as_ref(), "t-1", "listen").unwrap()["port"]
    .as_u64()
    .unwrap();
  let addr = format!("127.0.0.1:{port}");
  assert!(std::net::TcpStream::connect(&addr).is_ok());

  let err = start(t.as_ref(), "t-2", "hang").unwrap_err();
  assert_eq!(err.code, "component-timeout");
  std::thread::sleep(Duration::from_millis(100));
  assert!(
    std::net::TcpStream::connect(&addr).is_err(),
    "t-1's socket went with the process"
  );
}

#[test]
fn stray_output_on_stdout_is_skipped_and_the_stream_stays_in_sync() {
  let loaded = load("misbehaving.mjs", Grants::default(), None);
  let t = transport(&loaded);
  let pid = start(t.as_ref(), "t-1", "noise").unwrap()["pid"]
    .as_u64()
    .unwrap();
  let again = start(t.as_ref(), "t-2", "pid").unwrap()["pid"].as_u64().unwrap();
  assert_eq!(pid, again, "same process: nothing was lost");
}

#[test]
fn the_env_grant_is_enforced_out_of_process() {
  // SAFETY: tests in this binary do not read the environment concurrently with this write.
  unsafe { std::env::set_var("JANUS_SPIKE_GRANTED", "yes") };
  let loaded = load(
    "misbehaving.mjs",
    Grants {
      env: vec!["JANUS_SPIKE_GRANTED".to_string()],
      ..Grants::default()
    },
    None,
  );
  let env = start(transport(&loaded).as_ref(), "t-1", "env").unwrap()["env"].clone();
  let expected: Vec<&str> = if cfg!(windows) {
    vec!["JANUS_SPIKE_GRANTED", "SystemRoot"]
  } else {
    vec!["JANUS_SPIKE_GRANTED"]
  };
  assert_eq!(
    env,
    json!(expected),
    "exactly what was granted: no PATH, no HOME, no tokens"
  );
}

#[test]
fn the_network_grant_is_not_enforced_out_of_process() {
  // `network: false` is the default grant — and the tcp transport listens anyway. This test
  // passing is the finding: spec §9.3's "grants are not enforceable", demonstrated rather than
  // asserted.
  let loaded = load("tcp-transport.mjs", Grants::default(), None);
  let endpoint = transport(&loaded)
    .start(Start {
      instance: "t-1".to_string(),
      kind: "tcp".to_string(),
      role: "serve".to_string(),
      options: None,
    })
    .unwrap()
    .endpoint;
  let addr = format!("{}:{}", endpoint["host"].as_str().unwrap(), endpoint["port"]);
  assert!(
    std::net::TcpStream::connect(addr).is_ok(),
    "a socket nobody granted"
  );
}

#[test]
fn dropping_the_component_closes_stdin_and_the_process_exits() {
  let loaded = load("misbehaving.mjs", Grants::default(), None);
  let pid = start(transport(&loaded).as_ref(), "t-1", "pid").unwrap()["pid"]
    .as_u64()
    .unwrap();
  drop(loaded);
  assert!(gone_within(pid, Duration::from_secs(2)).is_some());
}

#[test]
fn a_killed_host_leaves_no_component_behind() {
  // SIGKILL on Unix, TerminateProcess on Windows: either way, no chance to clean up. The component
  // sees its stdin close and exits — spike 1.3 finding 2, now for a component process.
  let mut host = Command::new(env!("CARGO_BIN_EXE_host"))
    .args(command("misbehaving.mjs").split_whitespace())
    .stdout(Stdio::piped())
    .spawn()
    .unwrap();
  let mut line = String::new();
  BufReader::new(host.stdout.take().unwrap())
    .read_line(&mut line)
    .unwrap();
  let pid: u64 = line
    .trim()
    .parse()
    .unwrap_or_else(|_| panic!("host said {line:?}"));
  assert!(alive(pid));

  host.kill().unwrap();
  host.wait().unwrap();
  let gone = gone_within(pid, Duration::from_secs(2));
  assert!(gone.is_some(), "component {pid} outlived its host");
  eprintln!("component exited {:?} after its host was killed", gone.unwrap());
}
