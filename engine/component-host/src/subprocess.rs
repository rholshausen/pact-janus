//! The subprocess loader (component-interfaces spec §9.3, staged plan S5.2b, ADR 0108): a component the
//! engine spawns, speaking the same frames as every other binding over the Engine Protocol's own stdio
//! framing — a `Content-Length` header section, then exactly that many bytes of frame.
//!
//! What spec §9.3 makes normative, and where each lives:
//!
//! - **spawn from the declaration's `source`** — [`Subprocess::new`] and [`Subprocess::spawn`]: a program,
//!   its arguments and its directory, through no shell (ADR 0108 decisions 1 and 2);
//! - **bound every call in time, escalating to a kill** — [`Subprocess::call`], a deadline on the
//!   engine's side, since nothing inside another process can be interrupted. Out of process the unit of
//!   recreation is the process, and the error names every instance it held (decisions 3 and 4);
//! - **the component exits on stdin EOF** — the component's obligation; this side's half is that
//!   dropping the last handle closes stdin (`Drop for Subprocess`), and that a killed engine's pipe
//!   closes by itself;
//! - **stdout carries frames and nothing else** — the reader skips what is not a frame and says so,
//!   rather than dying on an author's first debug print;
//! - **correlation by frame id** — every call waits on its own id, so an exchange loop's
//!   `transport/poll-inbound` and the dispatch thread's `transport/start` share one process without
//!   queueing behind each other;
//! - **the `env` grant is enforced** — the process starts with an empty environment plus exactly the
//!   variables granted (and `SystemRoot` on Windows). `fs` and `network` are not, and a load that
//!   declares either says so once.

use pact_janus_kernel::component::{
  ComponentDeclaration, ComponentError, ComponentLoader, ContentComponent, Loaded, MatcherComponent,
  TransportComponent, wire,
};
use serde_json::{Value, json};
use std::collections::{BTreeSet, HashMap};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::Duration;

/// Spec §3.4: every call has a deadline. A declaration's `limits.deadline-ms` overrides this.
const DEFAULT_DEADLINE_MS: u64 = 10_000;
/// How long a component is given to take stdin's EOF as its cue before it is killed.
const EXIT_GRACE: Duration = Duration::from_millis(500);
/// The component protocol versions this host speaks (spec §3.2).
const COMPONENT_PROTOCOL_VERSIONS: &[u64] = &[1];

/// Loads `subprocess` sources. Holds nothing: every component is a process of its own, started at load
/// and started again when one dies.
#[derive(Debug, Default)]
pub struct SubprocessLoader;

impl SubprocessLoader {
  pub fn new() -> SubprocessLoader {
    SubprocessLoader
  }
}

impl ComponentLoader for SubprocessLoader {
  fn name(&self) -> &str {
    "subprocess"
  }

  fn sources(&self) -> &[&str] {
    &["subprocess"]
  }

  fn load(&self, declaration: &ComponentDeclaration) -> Result<Loaded, ComponentError> {
    let process = Arc::new(Subprocess::new(declaration)?);
    let hello = process.start()?;
    tracing::debug!(component = %declaration.name, ?hello, "component handshake");
    let grants = &declaration.grants;
    if !grants.fs.is_empty() || grants.network {
      tracing::warn!(
        component = %declaration.name,
        "a subprocess component runs with this engine's authority: its fs and network grants are not enforced (component-interfaces spec §9.3)"
      );
    }
    let declares = |interface: &str| {
      hello
        .get("interfaces")
        .and_then(Value::as_array)
        .is_some_and(|interfaces| interfaces.iter().any(|i| i == interface))
    };
    let client = Arc::new(wire::PipeClient::new(
      Arc::clone(&process),
      wire::declared_media_types(&hello),
    ));
    let transport = if declares("transport") {
      let client = wire::TransportClient::new(Arc::clone(&process), &hello).map_err(|why| {
        load_error(&format!(
          "component '{}' cannot be a transport: {why}",
          declaration.name
        ))
      })?;
      Some(Arc::new(client) as Arc<dyn TransportComponent>)
    } else {
      None
    };
    Ok(Loaded {
      content: declares("content").then(|| Arc::clone(&client) as Arc<dyn ContentComponent>),
      matcher: declares("matcher").then_some(client as Arc<dyn MatcherComponent>),
      transport,
      hello,
      // A program has no digest to pin, and a path is not recorded (ADR 0097, ADR 0108 decision 8).
      fetched: None,
    })
  }
}

/// One component's process, and everything needed to start it again.
pub struct Subprocess {
  name: String,
  program: PathBuf,
  args: Vec<String>,
  cwd: Option<PathBuf>,
  env: Vec<(String, String)>,
  deadline: Duration,
  hello_body: Value,
  /// The first process's handshake: what every later process must introduce itself as.
  hello: OnceLock<Value>,
  state: Mutex<State>,
  next_id: AtomicU64,
}

#[derive(Default)]
struct State {
  live: Option<Live>,
  /// Transport instances whose process died, by instance, with what ended it (ADR 0108 decision 4):
  /// a call on one fails at once rather than reaching a process that never started it.
  lost: HashMap<String, Lost>,
}

#[derive(Clone)]
struct Lost {
  pid: u32,
  why: String,
}

/// A running process: its stdin, the calls waiting on its stdout, and the transport instances it holds.
struct Live {
  child: Child,
  stdin: ChildStdin,
  waiting: Arc<Mutex<Waiting>>,
  instances: BTreeSet<String>,
}

#[derive(Default)]
struct Waiting {
  calls: HashMap<String, mpsc::Sender<Result<Value, ComponentError>>>,
  /// Set by the reader when stdout ends: the process is gone, whatever `try_wait` says yet.
  closed: bool,
  /// Frames that could not be attributed to a call: not JSON, or an id nobody asked with.
  unattributed: u64,
}

impl Subprocess {
  fn new(declaration: &ComponentDeclaration) -> Result<Subprocess, ComponentError> {
    let source = &declaration.source;
    let refuse = |member: &str, why: &str| {
      Err(load_error(&format!(
        "component '{}': a 'subprocess' source takes no '{member}': {why}",
        declaration.name
      )))
    };
    if source.digest.is_some() {
      return refuse(
        "digest",
        "a program is not pinned by its bytes, and a digest that is not checked is not one",
      );
    }
    if source.auth.is_some() {
      return refuse("auth", "a credential is for a registry");
    }
    if source.signature.is_some() {
      return refuse(
        "signature",
        "a signature is found beside an artifact in a registry",
      );
    }
    let Some(reference) = source.reference.as_deref().filter(|r| !r.is_empty()) else {
      return Err(load_error(&format!(
        "component '{}': a 'subprocess' source needs a 'reference': the program to start",
        declaration.name
      )));
    };
    let cwd = source.cwd.as_deref().map(PathBuf::from);
    if let Some(cwd) = &cwd
      && !cwd.is_dir()
    {
      return Err(load_error(&format!(
        "component '{}': its 'cwd' '{}' is not a directory",
        declaration.name,
        cwd.display()
      )));
    }
    let program = resolve_program(reference, cwd.as_deref())
      .map_err(|why| load_error(&format!("component '{}': {why}", declaration.name)))?;
    // The one grant a subprocess binding can enforce (spec §9.3): exactly the variables granted, read
    // from the engine's environment, and nothing else.
    let mut env: Vec<(String, String)> = declaration
      .grants
      .env
      .iter()
      .filter_map(|name| std::env::var(name).ok().map(|value| (name.clone(), value)))
      .collect();
    // Windows starts little without it (Winsock, the crypto providers): spec §9.3.
    if cfg!(windows)
      && !env
        .iter()
        .any(|(name, _)| name.eq_ignore_ascii_case("SystemRoot"))
      && let Ok(root) = std::env::var("SystemRoot")
    {
      env.push(("SystemRoot".to_string(), root));
    }
    Ok(Subprocess {
      name: declaration.name.clone(),
      program,
      args: source.args.clone(),
      cwd,
      env,
      deadline: Duration::from_millis(declaration.limits.deadline_ms.unwrap_or(DEFAULT_DEADLINE_MS)),
      hello_body: hello_body(declaration),
      hello: OnceLock::new(),
      state: Mutex::new(State::default()),
      next_id: AtomicU64::new(0),
    })
  }

  /// Start the first process and return its handshake: what the loader indexes.
  fn start(&self) -> Result<Value, ComponentError> {
    let mut state = self.state.lock().expect("subprocess state lock");
    self.ensure_live(&mut state)?;
    Ok(self.hello.get().cloned().expect("a live process has handshaken"))
  }

  /// The running process's id, when there is one: for tests that watch it come and go.
  pub fn pid(&self) -> Option<u32> {
    let state = self.state.lock().expect("subprocess state lock");
    state.live.as_ref().map(|live| live.child.id())
  }

  /// A live process in `state`, started and handshaken if there is none or the last one died. A dead
  /// one's instances are lost with it (ADR 0108 decision 3).
  fn ensure_live(&self, state: &mut State) -> Result<(), ComponentError> {
    let dead = state
      .live
      .as_ref()
      .is_some_and(|live| live.waiting.lock().expect("waiting lock").closed);
    if dead {
      self.bury(state, "it exited");
    }
    if state.live.is_some() {
      return Ok(());
    }
    let mut fresh = self.spawn()?;
    let hello = match self.exchange(&mut fresh, "component/hello", self.hello_body.clone()) {
      Ok(hello) => hello,
      Err(err) => {
        end(&mut fresh.child);
        return Err(err);
      }
    };
    // The handshake is the only source of truth about what a component provides (spec §3.2), so a
    // process that introduces itself as another component, or another version, is not this one.
    if let Some(first) = self.hello.get() {
      let identity = |hello: &Value| hello.get("component").cloned().unwrap_or(Value::Null);
      if identity(first) != identity(&hello) {
        end(&mut fresh.child);
        let mut error = ComponentError::synthesised(
          "component-malformed",
          format!(
            "component '{}' was started again and introduced itself as {}, where it was {}",
            self.name,
            identity(&hello),
            identity(first)
          ),
        );
        error.details = Some(Box::new(json!({ "component": self.name })));
        return Err(error);
      }
    } else {
      let _ = self.hello.set(hello);
    }
    state.live = Some(fresh);
    Ok(())
  }

  fn spawn(&self) -> Result<Live, ComponentError> {
    let mut command = Command::new(&self.program);
    command
      .args(&self.args)
      .env_clear()
      .envs(self.env.iter().cloned())
      .stdin(Stdio::piped())
      .stdout(Stdio::piped())
      .stderr(Stdio::piped());
    if let Some(cwd) = &self.cwd {
      command.current_dir(cwd);
    }
    lead_a_group(&mut command);
    let mut child = command.spawn().map_err(|err| {
      load_error(&format!(
        "component '{}' could not be started ('{}'): {err}",
        self.name,
        self.program.display()
      ))
    })?;
    let (Some(stdin), Some(stdout), Some(stderr)) =
      (child.stdin.take(), child.stdout.take(), child.stderr.take())
    else {
      end(&mut child);
      return Err(ComponentError::internal(
        "a child process's pipes were not opened",
      ));
    };
    let waiting = Arc::new(Mutex::new(Waiting::default()));
    let (reader, name) = (Arc::clone(&waiting), self.name.clone());
    let started = thread::Builder::new()
      .name(format!("janus-component-{}", self.name))
      .spawn(move || read_frames(stdout, &reader, &name))
      .and_then(|_| {
        let name = self.name.clone();
        thread::Builder::new()
          .name(format!("janus-component-{}-stderr", self.name))
          .spawn(move || {
            for line in BufReader::new(stderr).lines().map_while(Result::ok) {
              tracing::info!(component = %name, "{line}");
            }
          })
      });
    if let Err(err) = started {
      end(&mut child);
      return Err(ComponentError::internal(format!(
        "a thread to read the component could not start: {err}"
      )));
    }
    tracing::debug!(component = %self.name, pid = child.id(), program = %self.program.display(), "component process started");
    Ok(Live {
      child,
      stdin,
      waiting,
      instances: BTreeSet::new(),
    })
  }

  /// One call: write the frame on the live process, wait on its id. An operation that is meant to wait
  /// has its own wait added to the deadline (ADR 0108 decision 5).
  fn call(&self, op: &str, body: Value) -> Result<Value, ComponentError> {
    let extra = match op {
      "transport/poll-inbound" | "transport/send" => {
        body.get("timeout-ms").and_then(Value::as_u64).unwrap_or(0)
      }
      _ => 0,
    };
    let instance = op
      .starts_with("transport/")
      .then(|| body.get("instance").and_then(Value::as_str).map(str::to_string))
      .flatten();
    let id = self.next_id.fetch_add(1, Ordering::Relaxed).to_string();
    let (sender, receiver) = mpsc::channel();
    let (waiting, pid) = {
      let mut state = self.state.lock().expect("subprocess state lock");
      if let Some(instance) = &instance {
        if op == "transport/start" {
          state.lost.remove(instance);
        } else if let Some(lost) = state.lost.get(instance).cloned() {
          if op == "transport/stop" {
            state.lost.remove(instance);
          }
          return Err(self.lost(op, instance, &lost));
        }
      }
      self.ensure_live(&mut state)?;
      let live = state.live.as_mut().expect("just ensured");
      live
        .waiting
        .lock()
        .expect("waiting lock")
        .calls
        .insert(id.clone(), sender);
      let frame = json!({ "type": "request", "id": id, "op": op, "body": body });
      if let Err(err) = write_frame(&mut live.stdin, &frame) {
        live.waiting.lock().expect("waiting lock").calls.remove(&id);
        let instances = self.bury(&mut state, &format!("its stdin closed ({err})"));
        return Err(self.died("component-exited", op, "its process exited", instances, 0));
      }
      (Arc::clone(&live.waiting), live.child.id())
    };

    let deadline = self.deadline + Duration::from_millis(extra);
    match receiver.recv_timeout(deadline) {
      Ok(result) => {
        if result.is_ok()
          && let Some(instance) = instance
        {
          self.track(op, instance, pid);
        }
        result
      }
      Err(RecvTimeoutError::Disconnected) => {
        // The reader saw stdout end: the process is gone now, not at the deadline.
        let mut state = self.state.lock().expect("subprocess state lock");
        let instances = if state.live.as_ref().is_some_and(|live| live.child.id() == pid) {
          self.bury(&mut state, "it exited")
        } else {
          lost_with(&state, pid)
        };
        Err(self.died("component-exited", op, "its process exited", instances, 0))
      }
      Err(RecvTimeoutError::Timeout) => {
        let unattributed = {
          let mut waiting = waiting.lock().expect("waiting lock");
          waiting.calls.remove(&id);
          waiting.unattributed
        };
        // Spec §3.4, out of process: nothing can interrupt another process's call, so the process is
        // ended, and every instance it held with it (ADR 0108 decision 3).
        let mut state = self.state.lock().expect("subprocess state lock");
        let why = format!("it did not answer '{op}' within {} ms", deadline.as_millis());
        let instances = if state.live.as_ref().is_some_and(|live| live.child.id() == pid) {
          self.bury(&mut state, &why)
        } else {
          lost_with(&state, pid)
        };
        Err(self.died(
          "component-timeout",
          op,
          &format!("{why}; its process was killed"),
          instances,
          unattributed,
        ))
      }
    }
  }

  /// A call on a process not yet shared: the handshake a new process owes before anything else.
  fn exchange(&self, live: &mut Live, op: &str, body: Value) -> Result<Value, ComponentError> {
    let id = self.next_id.fetch_add(1, Ordering::Relaxed).to_string();
    let (sender, receiver) = mpsc::channel();
    live
      .waiting
      .lock()
      .expect("waiting lock")
      .calls
      .insert(id.clone(), sender);
    let frame = json!({ "type": "request", "id": id, "op": op, "body": body });
    if let Err(err) = write_frame(&mut live.stdin, &frame) {
      return Err(self.died(
        "component-exited",
        op,
        &format!("its stdin closed ({err})"),
        Vec::new(),
        0,
      ));
    }
    match receiver.recv_timeout(self.deadline) {
      Ok(result) => result,
      Err(RecvTimeoutError::Disconnected) => Err(self.died(
        "component-exited",
        op,
        "its process exited before it answered",
        Vec::new(),
        0,
      )),
      Err(RecvTimeoutError::Timeout) => Err(self.died(
        "component-timeout",
        op,
        &format!(
          "it did not answer within {} ms; its process was killed",
          self.deadline.as_millis()
        ),
        Vec::new(),
        0,
      )),
    }
  }

  /// A transport instance lives in the process that answered its `start`, until its `stop`.
  fn track(&self, op: &str, instance: String, pid: u32) {
    let mut state = self.state.lock().expect("subprocess state lock");
    let Some(live) = state.live.as_mut().filter(|live| live.child.id() == pid) else {
      return;
    };
    match op {
      "transport/start" => {
        live.instances.insert(instance);
      }
      "transport/stop" => {
        live.instances.remove(&instance);
      }
      _ => {}
    }
  }

  /// End the live process, and record its instances as lost with it. Returns them.
  fn bury(&self, state: &mut State, why: &str) -> Vec<String> {
    let Some(mut live) = state.live.take() else {
      return Vec::new();
    };
    let pid = live.child.id();
    let status = end(&mut live.child);
    tracing::warn!(component = %self.name, pid, ?status, instances = ?live.instances, "the component's process ended: {why}");
    let instances: Vec<String> = live.instances.iter().cloned().collect();
    for instance in &instances {
      state.lost.insert(
        instance.clone(),
        Lost {
          pid,
          why: why.to_string(),
        },
      );
    }
    instances
  }

  /// Spec §11.2: the component did not answer, so the binding does, marked as the engine's, naming the
  /// instances that went with the process (ADR 0108 decision 4).
  fn died(
    &self,
    code: &str,
    op: &str,
    why: &str,
    instances: Vec<String>,
    unattributed: u64,
  ) -> ComponentError {
    let lost = if instances.is_empty() {
      String::new()
    } else {
      format!(
        ", and the transport instances it held went with it: {}",
        instances.join(", ")
      )
    };
    let mut error = ComponentError::synthesised(
      code,
      format!("component '{}' could not answer '{op}': {why}{lost}", self.name),
    );
    let mut details = json!({ "component": self.name, "op": op, "instances": instances });
    if unattributed > 0 {
      details["unattributed-frames"] = json!(unattributed);
    }
    error.details = Some(Box::new(details));
    error
  }

  fn lost(&self, op: &str, instance: &str, lost: &Lost) -> ComponentError {
    let mut error = ComponentError::synthesised(
      "component-exited",
      format!(
        "component '{}' could not answer '{op}': its transport instance '{instance}' went with process {}, which ended because {}",
        self.name, lost.pid, lost.why
      ),
    );
    error.details = Some(Box::new(json!({
      "component": self.name, "op": op, "instances": [instance], "lost": true,
    })));
    error
  }
}

/// The instances already recorded as lost with process `pid`: what a second caller waiting on a process
/// another call ended reports.
fn lost_with(state: &State, pid: u32) -> Vec<String> {
  let mut instances: Vec<String> = state
    .lost
    .iter()
    .filter(|(_, lost)| lost.pid == pid)
    .map(|(instance, _)| instance.clone())
    .collect();
  instances.sort();
  instances
}

impl wire::Pipe for Subprocess {
  fn name(&self) -> &str {
    &self.name
  }

  fn call(&self, op: &str, body: Value) -> Result<Value, ComponentError> {
    Subprocess::call(self, op, body)
  }
}

/// Dropping the last handle closes stdin, the component's cue to exit (spec §9.3), and gives it a moment
/// to take the cue before killing it.
impl Drop for Subprocess {
  fn drop(&mut self) {
    let Some(live) = self.state.get_mut().ok().and_then(|state| state.live.take()) else {
      return;
    };
    let Live { mut child, stdin, .. } = live;
    drop(stdin);
    let waited = std::time::Instant::now();
    while waited.elapsed() < EXIT_GRACE {
      if let Ok(Some(_)) = child.try_wait() {
        return;
      }
      thread::sleep(Duration::from_millis(10));
    }
    tracing::warn!(component = %self.name, "the component did not exit when its stdin closed; killing it");
    end(&mut child);
  }
}

fn hello_body(declaration: &ComponentDeclaration) -> Value {
  json!({
    "component-protocol-versions": COMPONENT_PROTOCOL_VERSIONS,
    "engine": { "name": "janus-engine", "version": pact_janus_kernel::ENGINE_VERSION },
    "grants": declaration.grants,
    // The engine answers a content component's embedded content (spec §6.6, ADR 0103).
    "capabilities": { "embedded-content": {} },
  })
}

fn write_frame(stdin: &mut ChildStdin, frame: &Value) -> std::io::Result<()> {
  let body = serde_json::to_vec(frame)?;
  let mut bytes = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
  bytes.extend(body);
  stdin.write_all(&bytes)?;
  stdin.flush()
}

/// Frames off stdout, each to the call waiting on its id. What is not a frame is skipped and counted,
/// never fatal: the length prefix is what lets the stream resynchronise (spike 1.3 finding 6). A frame
/// that cannot be attributed while exactly one call waits can only be that call's, which fails at once
/// (ADR 0108 decision 5).
fn read_frames(stdout: impl Read, waiting: &Mutex<Waiting>, name: &str) {
  let mut reader = BufReader::new(stdout);
  loop {
    let mut length: Option<usize> = None;
    let mut saw_header = false;
    loop {
      let mut line = Vec::new();
      match reader.read_until(b'\n', &mut line) {
        Ok(0) | Err(_) => return close(waiting),
        Ok(_) => {}
      }
      let line = String::from_utf8_lossy(&line);
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
          tracing::warn!(component = %name, stray = line, "the component wrote something other than a frame to stdout; skipped")
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
    let id = frame
      .as_ref()
      .and_then(|frame| frame.get("id"))
      .and_then(Value::as_str)
      .map(str::to_string);
    let mut waiting = waiting.lock().expect("waiting lock");
    let sender = match id.and_then(|id| waiting.calls.remove(&id)) {
      Some(sender) => sender,
      None => {
        waiting.unattributed += 1;
        let excerpt: String = String::from_utf8_lossy(&body).chars().take(200).collect();
        tracing::warn!(component = %name, body = %excerpt, "a frame no call is waiting for; skipped");
        if waiting.calls.len() == 1 {
          let only = waiting.calls.keys().next().cloned().expect("one");
          if let Some(sender) = waiting.calls.remove(&only) {
            let _ = sender.send(Err(ComponentError::synthesised(
              "component-malformed",
              format!(
                "component '{name}' answered with a frame that is not a response to anything: {excerpt}"
              ),
            )));
          }
        }
        continue;
      }
    };
    let frame = frame.unwrap_or(Value::Null);
    let result = match (frame.get("ok"), frame.get("error")) {
      (_, Some(error)) => Err(serde_json::from_value(error.clone()).unwrap_or_else(|_| {
        ComponentError::synthesised(
          "component-malformed",
          format!("component '{name}' answered with an error that is not a ComponentError"),
        )
      })),
      (Some(ok), None) => Ok(ok.clone()),
      (None, None) => Err(ComponentError::synthesised(
        "component-malformed",
        format!("component '{name}' answered with neither 'ok' nor 'error'"),
      )),
    };
    let _ = sender.send(result);
  }
}

/// stdout ended: every waiting call learns the process is gone now, not at its deadline.
fn close(waiting: &Mutex<Waiting>) {
  let mut waiting = waiting.lock().expect("waiting lock");
  waiting.closed = true;
  // Dropping the senders is what each waiting call's `Disconnected` reports.
  waiting.calls.clear();
}

/// The program a `reference` names (ADR 0108 decision 2): a path, resolved against `cwd`, or a bare name
/// found on the engine's `PATH`, since the component's own environment is empty.
fn resolve_program(reference: &str, cwd: Option<&Path>) -> Result<PathBuf, String> {
  let path = Path::new(reference);
  if path.is_absolute() || path.components().count() > 1 {
    let full = match cwd {
      Some(cwd) if path.is_relative() => cwd.join(path),
      _ => path.to_path_buf(),
    };
    return if full.is_file() {
      Ok(full)
    } else {
      Err(format!("its program '{}' does not exist", full.display()))
    };
  }
  let names: Vec<String> = if cfg!(windows) {
    let extensions = std::env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".to_string());
    std::iter::once(reference.to_string())
      .chain(
        extensions
          .split(';')
          .filter(|extension| !extension.is_empty())
          .map(|extension| format!("{reference}{}", extension.to_ascii_lowercase())),
      )
      .collect()
  } else {
    vec![reference.to_string()]
  };
  std::env::var_os("PATH")
    .and_then(|paths| {
      std::env::split_paths(&paths).find_map(|dir| {
        names.iter().find_map(|name| {
          let candidate = dir.join(name);
          candidate.is_file().then_some(candidate)
        })
      })
    })
    .ok_or_else(|| format!("its program '{reference}' is not on this engine's PATH"))
}

/// On Unix, the component leads a process group of its own, which ending it ends as one.
fn lead_a_group(command: &mut Command) {
  #[cfg(unix)]
  {
    use std::os::unix::process::CommandExt;
    command.process_group(0);
  }
  #[cfg(not(unix))]
  let _ = command;
}

/// End a component and everything it started: on Unix the group it leads, on Windows its process tree,
/// which `taskkill /T` ends. Returns how it ended.
fn end(child: &mut Child) -> Option<std::process::ExitStatus> {
  if let Ok(Some(status)) = child.try_wait() {
    return Some(status);
  }
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
  child.wait().ok()
}

fn load_error(message: &str) -> ComponentError {
  ComponentError {
    code: "unavailable".to_string(),
    category: "component".to_string(),
    message: message.to_string(),
    source: Some("engine".into()),
    details: None,
    chain: None,
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use pretty_assertions::assert_eq;

  #[test]
  fn a_relative_path_is_resolved_against_cwd() {
    let dir = std::env::temp_dir();
    let name = format!("janus-subprocess-{}", std::process::id());
    std::fs::write(dir.join(&name), "").unwrap();
    let found = resolve_program(&format!("./{name}"), Some(&dir));
    std::fs::remove_file(dir.join(&name)).unwrap();
    assert_eq!(found, Ok(dir.join(".").join(&name)));
  }

  #[test]
  fn a_bare_name_not_on_the_path_says_so() {
    assert_eq!(
      resolve_program("janus-no-such-program", None),
      Err("its program 'janus-no-such-program' is not on this engine's PATH".to_string())
    );
  }

  #[test]
  fn a_path_that_does_not_exist_says_so() {
    let err = resolve_program("./janus-no-such-program", Some(Path::new("/"))).unwrap_err();
    assert!(err.contains("does not exist"), "{err}");
  }
}
