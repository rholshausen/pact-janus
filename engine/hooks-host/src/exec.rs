//! The `exec` implementation (lifecycle-hooks spec §8.3): spawn the command, write the context to
//! its stdin and close it, read the result from its stdout.
//!
//! Three properties are the spec's and are load-bearing here:
//!
//! - **The child's environment is exactly `run.env`** — deny-by-default, the same posture ADR 0013
//!   takes for component grants, so a hook cannot pick up an ambient credential the configuration
//!   does not show. A hook that needs `PATH` says so.
//! - **Arguments are a vector, not a shell string**, so nothing is re-parsed by a shell that quotes
//!   differently than the author expected.
//! - **The deadline is enforced**, and expiry is an outcome rather than a hang (§5.4). It kills the
//!   process group, so a child the command started does not outlive the run: on Unix the command
//!   leads a group of its own, and on Windows its process tree is ended (staged plan S1.9).
//!
//! Spawning per invocation costs milliseconds, which is fine at a handful of calls per exchange and
//! wrong for a hook called on every one of a thousand. That case has an answer and it is not a flag
//! here: a long-lived process speaking the same frames is a subprocess *component*.

use pact_janus_kernel::hooks::{HookFailure, HookInvoker, InvokeResult};
use serde_json::{Value, json};
use std::io::{Read, Write};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

/// How much of a child's stderr is kept for the failure message (spec §8.3: "bounded; the tail is
/// kept").
const STDERR_TAIL: usize = 2048;
/// How much of an unparseable stdout is quoted back.
const STDOUT_HEAD: usize = 512;

#[derive(Debug, Default)]
pub struct ExecHooks;

impl ExecHooks {
  pub fn new() -> Self {
    ExecHooks
  }
}

impl HookInvoker for ExecHooks {
  fn invoke(&self, run: &Value, context: &Value, deadline_ms: u64) -> Result<InvokeResult, HookFailure> {
    let Some(command) = run.get("command").and_then(Value::as_str) else {
      return Err(HookFailure::errored("an exec hook must name a command"));
    };

    let mut child = Command::new(command);
    child.env_clear();
    if let Some(Value::Object(env)) = run.get("env") {
      for (name, value) in env {
        if let Some(value) = value.as_str() {
          child.env(name, value);
        }
      }
    }
    if let Some(args) = run.get("args").and_then(Value::as_array) {
      for arg in args.iter().filter_map(Value::as_str) {
        child.arg(arg);
      }
    }
    if let Some(cwd) = run.get("cwd").and_then(Value::as_str) {
      child.current_dir(cwd);
    }
    lead_a_group(&mut child);

    let mut child = child
      .stdin(Stdio::piped())
      .stdout(Stdio::piped())
      .stderr(Stdio::piped())
      .spawn()
      .map_err(|err| HookFailure::errored(format!("could not spawn '{command}': {err}")))?;

    let document = serde_json::to_vec(context).expect("a HookContext always serializes");
    if let Some(mut stdin) = child.stdin.take() {
      // A write failure here is usually a command that exited before reading — recorded, not
      // fatal, because its exit status and stderr say more about why than this error would.
      if let Err(err) = stdin.write_all(&document) {
        tracing::debug!(command, %err, "the hook closed stdin before the context was written");
      }
      // Closed by dropping: a command reading to EOF would otherwise wait forever.
      drop(stdin);
    }

    // stdout and stderr are drained on their own threads: a child that fills a pipe buffer while
    // this thread waits for it to exit is a deadlock, and hook output is not always small.
    let stdout = reader(child.stdout.take());
    let stderr = reader(child.stderr.take());

    let deadline = Instant::now() + Duration::from_millis(deadline_ms);
    let status = loop {
      match child.try_wait() {
        Ok(Some(status)) => break status,
        Ok(None) => {
          if Instant::now() >= deadline {
            kill_the_group(&mut child);
            return Err(HookFailure::timed_out(deadline_ms));
          }
          thread::sleep(Duration::from_millis(2));
        }
        Err(err) => return Err(HookFailure::errored(format!("waiting for '{command}': {err}"))),
      }
    };

    // The command has exited, but a child it started may still hold its output open, and would
    // hold this invocation past its deadline. The rest of the deadline is what the output gets.
    let remaining = || deadline.saturating_duration_since(Instant::now());
    let (Ok(out), Ok(err)) = (stdout.recv_timeout(remaining()), stderr.recv_timeout(remaining())) else {
      kill_the_group(&mut child);
      return Err(HookFailure::timed_out(deadline_ms));
    };
    let err_text = String::from_utf8_lossy(&tail(&err, STDERR_TAIL)).to_string();
    if !err_text.trim().is_empty() {
      tracing::debug!(command, stderr = %err_text, "hook stderr");
    }

    if !status.success() {
      return Ok(InvokeResult {
        outcome: Some(pact_janus_kernel::hooks::Outcome::Failed),
        error: Some(json!({
          "code": "hook-exec-failed",
          "message": format!("'{command}' exited with {}", exit_text(&status)),
          "details": { "stderr": err_text },
        })),
        ..InvokeResult::default()
      });
    }

    if out.iter().all(u8::is_ascii_whitespace) {
      // A command that succeeded and had nothing to say (spec §8.3).
      return Ok(InvokeResult::ok());
    }

    match serde_json::from_slice::<Value>(&out) {
      Ok(document) => InvokeResult::parse(&document).map_err(|message| HookFailure {
        outcome: pact_janus_kernel::hooks::Outcome::Errored,
        error: json!({
          "code": "hook-result-invalid",
          "message": message,
          "details": { "stdout": String::from_utf8_lossy(&head(&out, STDOUT_HEAD)) },
        }),
      }),
      Err(err) => Ok(InvokeResult {
        outcome: Some(pact_janus_kernel::hooks::Outcome::Failed),
        error: Some(json!({
          "code": "hook-result-invalid",
          "message": format!("'{command}' wrote something that is not a hook result: {err}"),
          "details": { "stdout": String::from_utf8_lossy(&head(&out, STDOUT_HEAD)) },
        })),
        ..InvokeResult::default()
      }),
    }
  }
}

/// On Unix, the command leads a process group of its own, which a deadline kills as one.
fn lead_a_group(command: &mut Command) {
  #[cfg(unix)]
  {
    use std::os::unix::process::CommandExt;
    command.process_group(0);
  }
  #[cfg(not(unix))]
  let _ = command;
}

/// Ends the command and everything it started (spec §5.4: "killing the process group"). On Unix,
/// the group it leads; on Windows, its process tree, which `taskkill /T` ends.
fn kill_the_group(child: &mut Child) {
  #[cfg(unix)]
  {
    use rustix::process::{Pid, Signal, kill_process_group};
    let _ = kill_process_group(Pid::from_child(child), Signal::KILL);
  }
  #[cfg(windows)]
  {
    let _ = Command::new("taskkill")
      .args(["/T", "/F", "/PID", &child.id().to_string()])
      .stdin(Stdio::null())
      .stdout(Stdio::null())
      .stderr(Stdio::null())
      .status();
  }
  let _ = child.kill();
  let _ = child.wait();
}

fn reader<R: Read + Send + 'static>(stream: Option<R>) -> mpsc::Receiver<Vec<u8>> {
  let (tx, rx) = mpsc::channel();
  thread::spawn(move || {
    let mut buffer = Vec::new();
    if let Some(mut stream) = stream {
      let _ = stream.read_to_end(&mut buffer);
    }
    let _ = tx.send(buffer);
  });
  rx
}

fn head(bytes: &[u8], limit: usize) -> Vec<u8> {
  bytes[..bytes.len().min(limit)].to_vec()
}

fn tail(bytes: &[u8], limit: usize) -> Vec<u8> {
  bytes[bytes.len().saturating_sub(limit)..].to_vec()
}

fn exit_text(status: &std::process::ExitStatus) -> String {
  match status.code() {
    Some(code) => format!("status {code}"),
    None => "a signal".to_string(),
  }
}
