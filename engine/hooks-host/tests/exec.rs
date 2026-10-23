//! The `exec` implementation's deadline (lifecycle-hooks spec §5.4, staged plan S1.9): it kills the
//! process group, so a child the command started does not outlive the invocation.
//! The tests are Unix-only: they observe the group through `/bin/sh` and `kill -0`.

#![cfg(unix)]

use pact_janus_hooks_host::exec::ExecHooks;
use pact_janus_kernel::hooks::{HookInvoker, Outcome};
use serde_json::json;
use std::time::{Duration, Instant};

/// Whether a process is still running, asked of the system's own `kill -0`.
fn running(pid: &str) -> bool {
  std::process::Command::new("/bin/sh")
    .args(["-c", &format!("kill -0 {pid} 2>/dev/null")])
    .status()
    .is_ok_and(|status| status.success())
}

#[test]
fn a_deadline_ends_the_command_and_every_process_it_started() {
  let dir = std::env::temp_dir().join(format!("janus-exec-group-{}", std::process::id()));
  std::fs::create_dir_all(&dir).unwrap();
  let pid_file = dir.join("grandchild.pid");
  // The command starts a grandchild that would sleep for a minute, says which it is, and waits.
  let script = format!("sleep 60 & echo $! > '{}'; wait", pid_file.display());
  let run = json!({ "kind": "exec", "command": "/bin/sh", "args": ["-c", script],
                    "env": { "PATH": "/usr/bin:/bin" } });

  let started = Instant::now();
  let failure = ExecHooks::new()
    .invoke(&run, &json!({ "point": "before-request" }), 300)
    .expect_err("the deadline ends it");
  assert_eq!(failure.outcome, Outcome::TimedOut);
  assert!(
    started.elapsed() < Duration::from_secs(5),
    "{:?}",
    started.elapsed()
  );

  let pid = std::fs::read_to_string(&pid_file)
    .expect("the grandchild started")
    .trim()
    .to_string();
  let gone = (0..100).any(|_| {
    std::thread::sleep(Duration::from_millis(20));
    !running(&pid)
  });
  assert!(gone, "the grandchild {pid} outlived the invocation");
  let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_command_that_exits_but_leaves_a_child_holding_its_output_is_ended_at_the_deadline() {
  let run = json!({ "kind": "exec", "command": "/bin/sh", "args": ["-c", "sleep 60 &"],
                    "env": { "PATH": "/usr/bin:/bin" } });
  let started = Instant::now();
  let failure = ExecHooks::new()
    .invoke(&run, &json!({ "point": "before-request" }), 300)
    .expect_err("the output never closes, so the deadline ends it");
  assert_eq!(failure.outcome, Outcome::TimedOut);
  assert!(
    started.elapsed() < Duration::from_secs(5),
    "{:?}",
    started.elapsed()
  );
}
