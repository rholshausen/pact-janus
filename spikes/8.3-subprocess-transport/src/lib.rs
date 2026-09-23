//! The subprocess binding (component-interfaces spec §9.3), as a [`ComponentLoader`]: a component
//! the engine spawns, speaking the same frames as every other binding over the Engine Protocol's own
//! stdio framing — a `Content-Length` header section, then exactly that many bytes of frame.
//!
//! Spike code (plan task 8.3). What it implements, and where each normative rule lives:
//!
//! - **spawn from the declaration's `source`** — [`Pipe::spawn`]; `reference` is split on
//!   whitespace, because the schema calls it "the command" and says no more (a finding);
//! - **bound every call in time, escalating to a kill** — [`Pipe::call`], a per-call deadline on the
//!   engine's side, since nothing inside another process can be interrupted;
//! - **the component exits on stdin EOF** — the component's obligation; this side's half is that
//!   dropping the last handle closes stdin ([`Drop for Pipe`]);
//! - **stdout carries frames and nothing else** — the reader skips what is not a frame and says so,
//!   rather than dying on an author's first debug print;
//! - **correlation by frame id** — every call waits on its own id, so the exchange loop's
//!   `poll-inbound` and the dispatch thread's `start`/`stop` share one process without queueing.
//!
//! And what it cannot: grants. It enforces the one it can — `env`, by starting the process with an
//! empty environment plus exactly the variables granted — and says, once per load, that `fs` and
//! `network` are documented intent and nothing more.

use pact_janus_kernel::component::{
  ComponentDeclaration, ComponentError, ComponentLoader, ContentSlots, Dispose, DisposeResult, Inbound,
  Loaded, PollInbound, PollInboundResult, Reply, ReplyResult, Send, SendResult, Start, StartResult, Stop,
  StopResult, TransportComponent,
};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const DEFAULT_DEADLINE_MS: u64 = 10_000;

/// Loads `subprocess` sources. Holds nothing: every component is its own process.
#[derive(Default)]
pub struct SubprocessLoader;

impl ComponentLoader for SubprocessLoader {
  fn name(&self) -> &str {
    "subprocess"
  }

  fn sources(&self) -> &[&str] {
    &["subprocess"]
  }

  fn load(&self, declaration: &ComponentDeclaration) -> Result<Loaded, ComponentError> {
    let pipe = Arc::new(Pipe::new(declaration)?);
    let hello = pipe.call("component/hello", pipe.hello.clone(), Duration::ZERO)?;
    tracing::debug!(component = %declaration.name, ?hello, "component handshake");
    if !declaration.grants.fs.is_empty() || !declaration.grants.network {
      tracing::warn!(
        component = %declaration.name,
        "a subprocess component runs with this engine's authority: its fs and network grants are not enforced (spec §9.3)"
      );
    }
    let implements = |interface: &str| {
      hello["interfaces"]
        .as_array()
        .is_some_and(|all| all.iter().any(|i| i == interface))
    };
    let transport = implements("transport").then(|| {
      Arc::new(SubprocessTransport {
        slots: content_slots(&hello),
        pipe: Arc::clone(&pipe),
      }) as Arc<dyn TransportComponent>
    });
    Ok(Loaded {
      hello,
      content: None,
      transport,
      matcher: None,
    })
  }
}

/// Spec §5.5, per kind in the handshake; the kernel's trait asks per component, so the union.
fn content_slots(hello: &Value) -> ContentSlots {
  let mut slots = ContentSlots::new();
  for kind in hello
    .pointer("/contributes/transports")
    .and_then(Value::as_array)
    .into_iter()
    .flatten()
  {
    for (part, names) in kind["content-slots"].as_object().into_iter().flatten() {
      let entry = slots.entry(part.clone()).or_default();
      for name in names.as_array().into_iter().flatten().filter_map(Value::as_str) {
        if !entry.iter().any(|n| n == name) {
          entry.push(name.to_string());
        }
      }
    }
  }
  slots
}

/// One component's process, and everything needed to start it again.
pub struct Pipe {
  name: String,
  program: PathBuf,
  args: Vec<String>,
  env: Vec<(String, String)>,
  deadline: Duration,
  hello: Value,
  live: Mutex<Option<Live>>,
  next_id: AtomicU64,
}

/// A running process: its stdin, and the calls waiting on its stdout.
struct Live {
  child: Child,
  stdin: ChildStdin,
  waiting: Arc<Mutex<Waiting>>,
}

#[derive(Default)]
struct Waiting {
  calls: HashMap<String, mpsc::Sender<Result<Value, ComponentError>>>,
  /// Set by the reader when stdout ends: the process is gone, whatever `try_wait` says yet.
  closed: bool,
  /// Frames that could not be attributed to a call — not JSON, or an id nobody asked with.
  unattributed: u64,
}

impl Pipe {
  fn new(declaration: &ComponentDeclaration) -> Result<Pipe, ComponentError> {
    let Some(command) = declaration.source.reference.as_deref() else {
      return Err(load_error(
        "a 'subprocess' source needs a 'reference': the command to spawn",
      ));
    };
    let mut words = command.split_whitespace().map(str::to_string);
    let Some(program) = words.next() else {
      return Err(load_error("a 'subprocess' source's command is empty"));
    };
    let program =
      which(&program).ok_or_else(|| load_error(&format!("'{program}' is not on this engine's PATH")))?;
    // The `env` grant is the one grant a subprocess binding *can* enforce: the process gets exactly
    // the variables granted, read from the engine's environment, and nothing else.
    let mut env: Vec<(String, String)> = declaration
      .grants
      .env
      .iter()
      .filter_map(|name| std::env::var(name).ok().map(|value| (name.clone(), value)))
      .collect();
    // Windows cannot start much of anything without SystemRoot (Winsock, crypto); spike 1.3's
    // Windows risk, in the one form this side can see coming.
    if cfg!(windows)
      && let Ok(root) = std::env::var("SystemRoot")
    {
      env.push(("SystemRoot".to_string(), root));
    }
    let pipe = Pipe {
      name: declaration.name.clone(),
      program,
      args: words.collect(),
      env,
      deadline: Duration::from_millis(declaration.limits.deadline_ms.unwrap_or(DEFAULT_DEADLINE_MS)),
      hello: json!({
        "component-protocol-versions": [1],
        "engine": { "name": "janus-engine", "version": pact_janus_kernel::ENGINE_VERSION },
        "grants": declaration.grants,
        "capabilities": {},
      }),
      live: Mutex::new(None),
      next_id: AtomicU64::new(0),
    };
    Ok(pipe)
  }

  fn spawn(&self) -> Result<Live, ComponentError> {
    let mut child = Command::new(&self.program)
      .args(&self.args)
      .env_clear()
      .envs(self.env.iter().cloned())
      .stdin(Stdio::piped())
      .stdout(Stdio::piped())
      .stderr(Stdio::piped())
      .spawn()
      .map_err(|err| load_error(&format!("could not start '{}': {err}", self.program.display())))?;
    let stdin = child.stdin.take().expect("piped");
    let stdout = child.stdout.take().expect("piped");
    let stderr = child.stderr.take().expect("piped");
    let waiting = Arc::new(Mutex::new(Waiting::default()));

    let (reader_waiting, name) = (Arc::clone(&waiting), self.name.clone());
    thread::spawn(move || read_frames(stdout, &reader_waiting, &name));
    let name = self.name.clone();
    thread::spawn(move || {
      for line in BufReader::new(stderr).lines().map_while(Result::ok) {
        tracing::info!(component = %name, "{line}");
      }
    });
    tracing::debug!(component = %self.name, pid = child.id(), "component process started");
    Ok(Live {
      child,
      stdin,
      waiting,
    })
  }

  /// One call: write the frame, wait on its id. `extra` lengthens the deadline for an operation
  /// that is *meant* to wait (`poll-inbound`'s own `timeout-ms`).
  pub fn call(&self, op: &str, body: Value, extra: Duration) -> Result<Value, ComponentError> {
    let id = self.next_id.fetch_add(1, Ordering::Relaxed).to_string();
    let (sender, receiver) = mpsc::channel();
    let waiting = {
      let mut live = self.live.lock().expect("pipe lock");
      // Spec §3.4: a dead instance is never reused — here, a dead *process* is replaced, and the
      // replacement is handshaken before it is handed anything else.
      if live
        .as_ref()
        .is_none_or(|l| l.waiting.lock().expect("waiting lock").closed)
      {
        if let Some(mut dead) = live.take() {
          let _ = dead.child.kill();
          let _ = dead.child.wait();
        }
        let mut fresh = self.spawn()?;
        if op != "component/hello" {
          self.exchange(&mut fresh, "component/hello", self.hello.clone(), Duration::ZERO)?;
        }
        *live = Some(fresh);
      }
      let current = live.as_mut().expect("just ensured");
      current
        .waiting
        .lock()
        .expect("waiting lock")
        .calls
        .insert(id.clone(), sender);
      let frame = json!({ "type": "request", "id": id, "op": op, "body": body });
      if let Err(err) = write_frame(&mut current.stdin, &frame) {
        current.waiting.lock().expect("waiting lock").closed = true;
        return Err(exited(&self.name, op, &format!("its stdin is closed ({err})")));
      }
      Arc::clone(&current.waiting)
    };

    let deadline = self.deadline + extra;
    match receiver.recv_timeout(deadline) {
      Ok(result) => result,
      Err(RecvTimeoutError::Disconnected) => Err(exited(&self.name, op, "it exited")),
      Err(RecvTimeoutError::Timeout) => {
        let unattributed = {
          let mut waiting = waiting.lock().expect("waiting lock");
          waiting.calls.remove(&id);
          waiting.unattributed
        };
        // Spec §3.4, out of process: nothing can interrupt another process's call, so the whole
        // process is killed — and with it every transport instance it held, which is the price of
        // "recreate the instance" when the instance is a listening socket (a finding).
        self.kill();
        let mut error = ComponentError::synthesised(
          "component-timeout",
          format!(
            "component '{}' did not answer '{op}' within {} ms; its process was killed",
            self.name,
            deadline.as_millis()
          ),
        );
        error.details =
          Some(json!({ "component": self.name, "op": op, "unattributed-frames": unattributed }));
        Err(error)
      }
    }
  }

  /// A call on a process not yet shared: the handshake a respawn owes before anything else.
  fn exchange(
    &self,
    live: &mut Live,
    op: &str,
    body: Value,
    extra: Duration,
  ) -> Result<Value, ComponentError> {
    let id = self.next_id.fetch_add(1, Ordering::Relaxed).to_string();
    let (sender, receiver) = mpsc::channel();
    live
      .waiting
      .lock()
      .expect("waiting lock")
      .calls
      .insert(id.clone(), sender);
    write_frame(
      &mut live.stdin,
      &json!({ "type": "request", "id": id, "op": op, "body": body }),
    )
    .map_err(|err| exited(&self.name, op, &err.to_string()))?;
    receiver
      .recv_timeout(self.deadline + extra)
      .map_err(|_| exited(&self.name, op, "no handshake"))?
  }

  fn kill(&self) {
    if let Some(mut live) = self.live.lock().expect("pipe lock").take() {
      let _ = live.child.kill();
      let _ = live.child.wait();
    }
  }

  /// The running process's id — for tests that need to see whether it is still there.
  pub fn pid(&self) -> Option<u32> {
    self
      .live
      .lock()
      .expect("pipe lock")
      .as_ref()
      .map(|live| live.child.id())
  }
}

/// Dropping the last handle closes stdin — the component's cue to exit (spec §9.3) — and waits
/// briefly for it to take the cue before killing it.
impl Drop for Pipe {
  fn drop(&mut self) {
    let Some(live) = self.live.get_mut().expect("pipe lock").take() else {
      return;
    };
    let Live { mut child, stdin, .. } = live;
    drop(stdin);
    for _ in 0..50 {
      if let Ok(Some(_)) = child.try_wait() {
        return;
      }
      thread::sleep(Duration::from_millis(10));
    }
    tracing::warn!(component = %self.name, "the component did not exit on stdin EOF; killing it");
    let _ = child.kill();
    let _ = child.wait();
  }
}

fn write_frame(stdin: &mut ChildStdin, frame: &Value) -> std::io::Result<()> {
  let body = serde_json::to_vec(frame)?;
  let mut bytes = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
  bytes.extend(body);
  stdin.write_all(&bytes)?;
  stdin.flush()
}

/// The reader: frames off stdout, each to the call waiting on its id. What is not a frame is
/// skipped and counted, never fatal — the length prefix is what lets the stream resynchronise
/// (spike 1.3 finding 6).
fn read_frames(stdout: impl Read, waiting: &Mutex<Waiting>, name: &str) {
  let mut reader = BufReader::new(stdout);
  loop {
    let mut length: Option<usize> = None;
    let mut saw_header = false;
    loop {
      let mut line = String::new();
      match reader.read_line(&mut line) {
        Ok(0) | Err(_) => return close(waiting),
        Ok(_) => {}
      }
      let line = line.trim_end_matches(['\r', '\n']);
      if line.is_empty() {
        if saw_header {
          break;
        }
        continue;
      }
      match line.split_once(':') {
        Some((header, value)) => {
          saw_header = true;
          if header.trim().eq_ignore_ascii_case("content-length") {
            length = value.trim().parse().ok();
          }
        }
        None => {
          tracing::warn!(component = %name, stray = line, "stdout carried something other than a frame; skipped")
        }
      }
    }
    let Some(length) = length else {
      tracing::warn!(component = %name, "a header section with no Content-Length; skipped");
      continue;
    };
    let mut body = vec![0; length];
    if reader.read_exact(&mut body).is_err() {
      return close(waiting);
    }
    let frame: Option<Value> = serde_json::from_slice(&body).ok();
    let id = frame.as_ref().and_then(|f| f["id"].as_str()).map(str::to_string);
    let mut waiting = waiting.lock().expect("waiting lock");
    let Some(sender) = id.and_then(|id| waiting.calls.remove(&id)) else {
      waiting.unattributed += 1;
      tracing::warn!(component = %name, body = %String::from_utf8_lossy(&body), "a frame no call is waiting for; skipped");
      continue;
    };
    let frame = frame.expect("an id came from it");
    let result = match (frame.get("ok"), frame.get("error")) {
      (_, Some(error)) => Err(serde_json::from_value(error.clone()).unwrap_or_else(|_| {
        ComponentError::synthesised(
          "component-malformed",
          format!("'{name}' answered with an error that is not one"),
        )
      })),
      (Some(ok), None) => Ok(ok.clone()),
      (None, None) => Err(ComponentError::synthesised(
        "component-malformed",
        format!("'{name}' answered with neither 'ok' nor 'error'"),
      )),
    };
    let _ = sender.send(result);
  }
}

/// stdout ended: every waiting call learns the process is gone now, not at its deadline.
fn close(waiting: &Mutex<Waiting>) {
  let mut waiting = waiting.lock().expect("waiting lock");
  waiting.closed = true;
  waiting.calls.clear(); // dropping the senders is what `Disconnected` reports
}

fn exited(name: &str, op: &str, why: &str) -> ComponentError {
  let mut error = ComponentError::synthesised(
    "component-exited",
    format!("component '{name}' could not answer '{op}': {why}"),
  );
  error.details = Some(json!({ "component": name, "op": op }));
  error
}

fn load_error(message: &str) -> ComponentError {
  ComponentError {
    code: "unavailable".to_string(),
    category: "component".to_string(),
    message: message.to_string(),
    source: Some("engine".into()),
    details: None,
  }
}

/// The program on the *engine's* PATH: the component's own environment is empty unless granted,
/// so it cannot be what finds its interpreter.
fn which(program: &str) -> Option<PathBuf> {
  let direct = PathBuf::from(program);
  if direct.components().count() > 1 {
    return direct.is_file().then_some(direct);
  }
  let names: Vec<String> = if cfg!(windows) {
    ["exe", "cmd", "bat"]
      .iter()
      .map(|ext| format!("{program}.{ext}"))
      .collect()
  } else {
    vec![program.to_string()]
  };
  std::env::split_paths(&std::env::var_os("PATH")?).find_map(|dir| {
    names.iter().find_map(|name| {
      let candidate = dir.join(name);
      candidate.is_file().then_some(candidate)
    })
  })
}

/// The transport interface over the pipe: the kernel's documents, as the frames' JSON and back.
pub struct SubprocessTransport {
  pipe: Arc<Pipe>,
  slots: ContentSlots,
}

impl SubprocessTransport {
  pub fn pipe(&self) -> &Pipe {
    &self.pipe
  }

  fn call(&self, op: &str, body: Value, extra: Duration) -> Result<Value, ComponentError> {
    self.pipe.call(op, body, extra)
  }
}

fn parse<T: serde::de::DeserializeOwned>(value: Option<&Value>, what: &str) -> Result<T, ComponentError> {
  serde_json::from_value(value.cloned().unwrap_or(Value::Null))
    .map_err(|err| ComponentError::synthesised("component-malformed", format!("{what}: {err}")))
}

impl TransportComponent for SubprocessTransport {
  fn content_slots(&self) -> ContentSlots {
    self.slots.clone()
  }

  fn start(&self, req: Start) -> Result<StartResult, ComponentError> {
    let mut body = json!({ "instance": req.instance, "kind": req.kind, "role": req.role });
    if let Some(options) = req.options {
      body["options"] = options;
    }
    let ok = self.call("transport/start", body, Duration::ZERO)?;
    Ok(StartResult {
      endpoint: ok.get("endpoint").cloned().unwrap_or(Value::Null),
    })
  }

  fn stop(&self, req: Stop) -> Result<StopResult, ComponentError> {
    self.call(
      "transport/stop",
      json!({ "instance": req.instance }),
      Duration::ZERO,
    )?;
    Ok(StopResult {})
  }

  fn send(&self, req: Send) -> Result<SendResult, ComponentError> {
    let mut body = json!({ "instance": req.instance, "parts": req.parts, "await-reply": req.await_reply });
    if let Some(timeout) = req.timeout_ms {
      body["timeout-ms"] = json!(timeout);
    }
    let extra = Duration::from_millis(req.timeout_ms.unwrap_or(0));
    let ok = self.call("transport/send", body, extra)?;
    Ok(SendResult {
      reply: match ok.get("reply") {
        Some(reply) => Some(parse(Some(reply), "transport/send's reply")?),
        None => None,
      },
    })
  }

  fn poll_inbound(&self, req: PollInbound) -> Result<PollInboundResult, ComponentError> {
    let ok = self.call(
      "transport/poll-inbound",
      json!({ "instance": req.instance, "timeout-ms": req.timeout_ms }),
      Duration::from_millis(req.timeout_ms),
    )?;
    let inbound = match ok.get("inbound") {
      Some(inbound) => Some(Inbound {
        event: inbound["event"].as_str().unwrap_or_default().to_string(),
        parts: parse(inbound.get("parts"), "an inbound's parts")?,
        expects_reply: inbound["expects-reply"].as_bool().unwrap_or(false),
      }),
      None => None,
    };
    Ok(PollInboundResult { inbound })
  }

  fn reply(&self, req: Reply) -> Result<ReplyResult, ComponentError> {
    self.call(
      "transport/reply",
      json!({ "instance": req.instance, "event": req.event, "parts": req.parts }),
      Duration::ZERO,
    )?;
    Ok(ReplyResult {})
  }

  fn dispose(&self, req: Dispose) -> Result<DisposeResult, ComponentError> {
    self.call(
      "transport/dispose",
      json!({ "instance": req.instance, "event": req.event, "disposition": req.disposition }),
      Duration::ZERO,
    )?;
    Ok(DisposeResult {})
  }
}
