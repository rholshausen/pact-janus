//! The top-level dispatcher: one `Engine` per pipe (engine-protocol spec §3), owning handshake
//! state and the session store. `dispatch` is pipe 3.3's entry point (native/in-process — "no
//! additional rules" beyond the call pipe itself) — JSON bytes in, JSON bytes out — deliberately
//! shaped so the subprocess and WASM pipes (§3.1–3.2) can wrap it later without changing it.

use super::consumer_session::{AddInteraction, Create, Finalise, ServeVariant, StartTransport, Variants};
use super::events::{Event, Poll, Stream};
use super::explain::{self, Explain, ExplainError};
use super::frame::{EngineError, RequestFrame, ResponseFrame};
use super::hello::{self, Hello};
use super::session::{ServeVariantError, SessionStore, VariantsError};
use super::verification::{self, Run, Target, Verify, VerifyError};
use crate::component::{ContentComponent, HookComponent, Start, Stop, TransportComponent};
use crate::hooks::{ConfigError, HookInvoker, HookRunner, ScriptHooks};
use crate::upgrade;
use crate::variant::VariantError;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::panic::{self, AssertUnwindSafe};
use std::sync::Arc;
use std::time::Duration;

pub struct Engine {
  hello_done: bool,
  /// Set by `engine/shutdown` (spec §6): every later call is `engine-shut-down`, and a subprocess
  /// embedding exits once the shutdown response is written ([`Engine::is_shut_down`]).
  shut_down: bool,
  sessions: SessionStore,
  /// Transport components this embedding registered, by name (component-interfaces spec §3.4,
  /// design 2.6) — empty for a plain [`Engine::new`], which is a legitimate engine that simply
  /// answers `start-transport` with `component-unavailable` for anything named.
  transports: HashMap<String, Arc<dyn TransportComponent>>,
  /// The one content component this embedding registered, if any (plan task 4.5's exchange loop;
  /// [`crate::plan::resolve`]'s own "single slot, not a registry" simplification, reused here).
  content: Option<Arc<dyn ContentComponent>>,
  next_transport: u64,
  /// Live event streams by id (spec §9.1), each belonging to the session whose work produces it.
  /// Registered when that work starts and removed when its terminal event is *delivered* —
  /// spec §9.1's "a stream ends when its final event has been delivered", which is why the
  /// registry, not the buffer, is where an ended stream stops being findable.
  streams: HashMap<String, Arc<Stream>>,
  next_stream: u64,
  /// Hook implementations this engine can run, by kind (lifecycle-hooks spec §8.5: an
  /// implementation is an embedding capability, exactly as a component loader is — ADR 0013).
  /// `script` is always here because the interpreter compiles with the engine (ADR 0015); `exec`
  /// and `http` are the embedding's to register, and an engine that was handed neither refuses a
  /// configuration naming them by name.
  hook_invokers: HashMap<String, Arc<dyn HookInvoker>>,
  /// Hook components this embedding registered, by name (`run: { kind: component }`).
  hook_components: HashMap<String, Arc<dyn HookComponent>>,
  /// Verification sessions by id (spec §7.3): one entry per run in flight. A run ends itself at
  /// its terminal event, so nothing here is released by a host operation — [`Engine::handle_poll`]
  /// prunes an entry when its stream's last event is delivered, which is the same edge spec §7.1
  /// makes the session end on.
  verifications: HashMap<String, Run>,
  next_verification: u64,
}

impl Default for Engine {
  fn default() -> Self {
    Self::new()
  }
}

impl Engine {
  /// An engine with no components registered — `start-transport` always answers
  /// `component-unavailable`. What every test and the protocol skeleton used before plan task 4.5
  /// needed, and still all `consumer-session/*` operations short of a live exchange need.
  pub fn new() -> Self {
    Engine {
      hello_done: false,
      shut_down: false,
      sessions: SessionStore::default(),
      transports: HashMap::new(),
      content: None,
      next_transport: 0,
      streams: HashMap::new(),
      next_stream: 0,
      hook_invokers: builtin_hook_invokers(),
      hook_components: HashMap::new(),
      verifications: HashMap::new(),
      next_verification: 0,
    }
  }

  /// An engine wired to real components (plan task 4.5) — what an embedding (the CLI's subprocess
  /// binary, an SDK's native binding) builds so `start-transport` and the live exchange loop it
  /// drives have something to call. The kernel never depends on `transports`/`content`'s own
  /// crates (CLAUDE.md's B3); this is the injection point that keeps it that way.
  pub fn with_components(
    transports: HashMap<String, Arc<dyn TransportComponent>>,
    content: Option<Arc<dyn ContentComponent>>,
  ) -> Self {
    Engine {
      hello_done: false,
      shut_down: false,
      sessions: SessionStore::default(),
      transports,
      content,
      next_transport: 0,
      streams: HashMap::new(),
      next_stream: 0,
      hook_invokers: builtin_hook_invokers(),
      hook_components: HashMap::new(),
      verifications: HashMap::new(),
      next_verification: 0,
    }
  }

  /// Register a hook implementation this embedding can run (lifecycle-hooks spec §8.5). `exec` and
  /// `http` arrive this way because spawning a process and opening a socket are things the
  /// embedding can do and a kernel that must build for `wasm32-wasip2` cannot.
  pub fn register_hook_invoker(&mut self, kind: impl Into<String>, invoker: Arc<dyn HookInvoker>) {
    self.hook_invokers.insert(kind.into(), invoker);
  }

  /// Register a hook component, answering `run: { kind: component, component: <name> }`.
  pub fn register_hook_component(&mut self, name: impl Into<String>, component: Arc<dyn HookComponent>) {
    self.hook_components.insert(name.into(), component);
  }

  /// One frame in, one frame out (spec §4). Never panics: the dispatch boundary catches panics
  /// and converts them to `internal` (§10.1) — a panic reaching a caller would itself be a bug.
  ///
  /// Traced at `trace` rather than left to whatever byte-pipe binding wraps this (subprocess,
  /// WASM, native): this is the one place every pipe's frames pass through regardless of
  /// encoding, so it is the one place a `RUST_LOG=trace` capture of "what actually crossed the
  /// wire" can live without instrumenting each binding separately. The embedding installs the
  /// subscriber (this crate ships only the `tracing` facade); `janus-engine` is one such embedding.
  pub fn dispatch(&mut self, bytes: &[u8]) -> Vec<u8> {
    tracing::trace!(frame = %String::from_utf8_lossy(bytes), "frame in");
    let response = self.dispatch_bytes(bytes);
    let out = serde_json::to_vec(&response).expect("ResponseFrame is always representable as JSON");
    tracing::trace!(frame = %String::from_utf8_lossy(&out), "frame out");
    out
  }

  fn dispatch_bytes(&mut self, bytes: &[u8]) -> ResponseFrame {
    let raw: Value = match serde_json::from_slice(bytes) {
      Ok(v) => v,
      Err(err) => {
        return ResponseFrame::err(
          "",
          EngineError::malformed_frame(format!("frame body is not valid JSON ({err})")),
        );
      }
    };
    // Recovered defensively so a schema-violating frame still gets a correlated error where
    // possible (spec §4.4: an empty id means pipe-level, uncorrelatable).
    let id = raw
      .get("id")
      .and_then(Value::as_str)
      .unwrap_or_default()
      .to_string();
    if raw.get("type").and_then(Value::as_str) != Some("request") {
      return ResponseFrame::err(
        id,
        EngineError::malformed_frame("frame 'type' must be 'request' on this pipe"),
      );
    }

    let mut de = serde_json::Deserializer::from_slice(bytes);
    let request: RequestFrame = match serde_path_to_error::deserialize(&mut de) {
      Ok(request) => request,
      Err(err) => return ResponseFrame::err(id, EngineError::malformed_frame(err.to_string())),
    };

    match panic::catch_unwind(AssertUnwindSafe(|| self.dispatch_frame(request))) {
      Ok(response) => response,
      Err(_) => ResponseFrame::err(
        id,
        EngineError::internal("the engine encountered an internal error handling this request"),
      ),
    }
  }

  fn dispatch_frame(&mut self, request: RequestFrame) -> ResponseFrame {
    let RequestFrame { id, op, body, .. } = request;

    if self.shut_down {
      return ResponseFrame::err(id, EngineError::engine_shut_down());
    }
    if op != "engine/hello" && !self.hello_done {
      return ResponseFrame::err(id, EngineError::handshake_required());
    }

    match op.as_str() {
      "engine/hello" => self.handle_hello(id, body),
      "engine/shutdown" => self.handle_shutdown(id),
      "consumer-session/create" => self.handle_create(id, body),
      "consumer-session/add-interaction" => self.handle_add_interaction(id, body),
      "consumer-session/variants" => self.handle_variants(id, body),
      "consumer-session/start-transport" => self.handle_start_transport(id, body),
      "consumer-session/serve-variant" => self.handle_serve_variant(id, body),
      "consumer-session/finalise" => self.handle_finalise(id, body),
      "verification/verify" => self.handle_verify(id, body),
      "verification/explain" => self.handle_explain(id, body),
      "upgrade/pact" => self.handle_upgrade(id, body),
      "events/poll" => self.handle_poll(id, body),
      other => ResponseFrame::err(id, EngineError::operation_unsupported(other)),
    }
  }

  fn handle_hello(&mut self, id: String, body: Value) -> ResponseFrame {
    let hello: Hello = match parse_body(body) {
      Ok(hello) => hello,
      Err(err) => return ResponseFrame::err(id, err),
    };
    match hello::negotiate(&hello) {
      Some(_version) => {
        self.hello_done = true;
        ResponseFrame::ok(id, hello::result())
      }
      None => ResponseFrame::err(
        id,
        EngineError::protocol_version_unsupported(&[crate::PROTOCOL_VERSION]),
      ),
    }
  }

  /// `engine/shutdown` (spec §6): release every session, answer `ok: {}`, and go inert. Consumer
  /// sessions end with their transports stopped. A verification run in flight is left to finish on
  /// its own thread with nothing left to deliver its events to — a subprocess embedding's exit ends
  /// it, and a run has no host-visible resource to leak (spec §7.1).
  fn handle_shutdown(&mut self, id: String) -> ResponseFrame {
    self.sessions.end_all();
    self.streams.clear();
    self.verifications.clear();
    self.shut_down = true;
    tracing::info!("engine/shutdown: all sessions released");
    ResponseFrame::ok(id, json!({}))
  }

  /// Whether `engine/shutdown` has been answered: a subprocess embedding exits once it has written
  /// that response (spec §6: "respond `ok: {}`, and then exit").
  pub fn is_shut_down(&self) -> bool {
    self.shut_down
  }

  fn handle_create(&mut self, id: String, body: Value) -> ResponseFrame {
    let create: Create = match parse_body(body) {
      Ok(create) => create,
      Err(err) => return ResponseFrame::err(id, err),
    };
    let session = self.sessions.create(
      create.config.consumer,
      create.config.provider,
      create.config.policy,
    );
    ResponseFrame::ok(id, json!({ "session": session }))
  }

  fn handle_add_interaction(&mut self, id: String, body: Value) -> ResponseFrame {
    let req: AddInteraction = match parse_body(body) {
      Ok(req) => req,
      Err(err) => return ResponseFrame::err(id, err),
    };
    let Some(session) = self.sessions.get_mut(&req.session) else {
      return ResponseFrame::err(id, EngineError::session_not_found(&req.session));
    };
    match session.add_interaction(&req.interaction) {
      Ok(handle) => ResponseFrame::ok(id, json!({ "handle": handle })),
      Err(err) => ResponseFrame::err(id, EngineError::interaction_invalid(&err.problems)),
    }
  }

  fn handle_variants(&mut self, id: String, body: Value) -> ResponseFrame {
    let req: Variants = match parse_body(body) {
      Ok(req) => req,
      Err(err) => return ResponseFrame::err(id, err),
    };
    let Some(session) = self.sessions.get_mut(&req.session) else {
      return ResponseFrame::err(id, EngineError::session_not_found(&req.session));
    };
    match session.variants(&req.handle, req.policy.as_ref()) {
      Ok(result) => ResponseFrame::ok(id, result),
      Err(VariantsError::HandleNotFound) => {
        ResponseFrame::err(id, EngineError::handle_not_found(&req.handle))
      }
      Err(VariantsError::Variant(VariantError::InvalidPolicy(problems))) => {
        ResponseFrame::err(id, EngineError::invalid_policy(&problems))
      }
      Err(VariantsError::Variant(VariantError::BudgetExceeded {
        space,
        selected,
        budget,
        dimensions,
      })) => ResponseFrame::err(
        id,
        EngineError::variant_budget_exceeded(space, selected, budget, &dimensions),
      ),
    }
  }

  fn handle_start_transport(&mut self, id: String, body: Value) -> ResponseFrame {
    let req: StartTransport = match parse_body(body) {
      Ok(req) => req,
      Err(err) => return ResponseFrame::err(id, err),
    };
    let Some(component) = self.transports.get(&req.transport).cloned() else {
      return ResponseFrame::err(id, EngineError::component_unavailable(&req.transport));
    };
    let Some(session) = self.sessions.get_mut(&req.session) else {
      return ResponseFrame::err(id, EngineError::session_not_found(&req.session));
    };
    self.next_transport += 1;
    let instance = format!("t-{}", self.next_transport);
    match session.start_transport(
      &req.transport,
      component,
      self.content.clone(),
      instance,
      req.options,
    ) {
      Ok(endpoint) => ResponseFrame::ok(id, json!({ "endpoint": endpoint })),
      Err(err) => ResponseFrame::err(id, EngineError::component_failed(&req.transport, &err)),
    }
  }

  fn handle_serve_variant(&mut self, id: String, body: Value) -> ResponseFrame {
    let req: ServeVariant = match parse_body(body) {
      Ok(req) => req,
      Err(err) => return ResponseFrame::err(id, err),
    };
    let Some(session) = self.sessions.get_mut(&req.session) else {
      return ResponseFrame::err(id, EngineError::session_not_found(&req.session));
    };
    match session.serve_variant(&req.handle, &req.variant) {
      Ok(()) => ResponseFrame::ok(id, json!({})),
      Err(ServeVariantError::HandleNotFound) => {
        ResponseFrame::err(id, EngineError::handle_not_found(&req.handle))
      }
      Err(ServeVariantError::VariantNotFound { selection }) => {
        ResponseFrame::err(id, EngineError::variant_not_found(&req.variant, &selection))
      }
    }
  }

  fn handle_finalise(&mut self, id: String, body: Value) -> ResponseFrame {
    let req: Finalise = match parse_body(body) {
      Ok(req) => req,
      Err(err) => return ResponseFrame::err(id, err),
    };
    let Some(mut session) = self.sessions.end(&req.session) else {
      return ResponseFrame::err(id, EngineError::session_not_found(&req.session));
    };
    session.stop_transports();
    let results = session.results();
    match session.contract() {
      Ok(Some(contract)) => {
        let contract = serde_json::to_value(&contract).expect("Contract always serializes");
        ResponseFrame::ok(id, json!({ "results": results, "contract": contract }))
      }
      Ok(None) => ResponseFrame::ok(id, json!({ "results": results })),
      Err(problems) => ResponseFrame::err(id, EngineError::contract_invalid(&problems)),
    }
  }

  /// `verification/verify` (spec §8.3): start a run and answer with its session and stream. Every
  /// failure *before* the run starts is an error here; everything after it is an event, which is
  /// spec §10.2's line ("a verification that ran and found mismatches is a successful operation")
  /// drawn in code.
  fn handle_verify(&mut self, id: String, body: Value) -> ResponseFrame {
    let req: Verify = match parse_body(body) {
      Ok(req) => req,
      Err(err) => return ResponseFrame::err(id, err),
    };
    let contracts = match verification::read_source(&req.source) {
      Ok(contracts) => contracts,
      Err(err) => return ResponseFrame::err(id, verify_error(err)),
    };

    // Every target transport is started before the run begins, so an unreachable or
    // misconfigured provider fails the *call* rather than arriving as a run that verified nothing.
    let mut targets = Vec::with_capacity(req.target.transports.len());
    for binding in &req.target.transports {
      let Some(component) = self.transports.get(&binding.transport).cloned() else {
        return ResponseFrame::err(
          id,
          EngineError::component_unavailable(&format!("transport/{}", binding.transport)),
        );
      };
      self.next_transport += 1;
      let instance = format!("t-{}", self.next_transport);
      let started = component.start(Start {
        instance: instance.clone(),
        kind: binding.transport.clone(),
        role: "drive".to_string(),
        options: binding.options.clone(),
      });
      match started {
        Ok(started) => targets.push(Target {
          kind: binding.transport.clone(),
          component,
          instance,
          endpoint: started.endpoint,
        }),
        Err(error) => {
          stop_targets(&targets);
          return ResponseFrame::err(
            id,
            EngineError::component_failed(&format!("transport/{}", binding.transport), &error),
          );
        }
      }
    }

    self.next_verification += 1;
    let session = format!("vs-{}", self.next_verification);

    // Hooks are resolved and checked **before** the run starts (lifecycle-hooks spec §5.3, §11):
    // a configuration that could not be understood, or that names an implementation this embedding
    // cannot run, has no run to report into. Everything after this point is an outcome.
    let hooks = match &req.target.hooks {
      None => None,
      Some(document) => {
        let run = json!({
          "id": session,
          // The parties as the first contract names them: a run over contracts with different
          // parties is a host's decision to mix them, and a hook that needs to tell them apart
          // reads `interaction` rather than this.
          "consumer": contracts.first().map(|source| json!({ "name": source.consumer() })),
          "provider": contracts.first().map(|source| json!({ "name": source.provider() })),
        });
        match HookRunner::new(
          document,
          self.hook_invokers.clone(),
          self.hook_components.clone(),
          run,
        ) {
          Ok(runner) => Some(runner),
          Err(err) => {
            stop_targets(&targets);
            return ResponseFrame::err(id, hook_config_error(err));
          }
        }
      }
    };

    let stream = self.open_stream();
    let run = verification::start(
      contracts,
      targets,
      self.content.clone(),
      hooks,
      req.options,
      stream,
    );
    let stream_id = run.stream.id().to_string();
    self.verifications.insert(session.clone(), run);
    ResponseFrame::ok(id, json!({ "session": session, "stream": stream_id }))
  }

  /// Allocate and register a stream for work a session is about to start (spec §9.1: ids are
  /// engine-assigned, opaque and scoped to their session). The caller keeps the returned handle to
  /// emit on; the engine keeps its twin so `events/poll` can find it.
  pub(crate) fn open_stream(&mut self) -> Arc<Stream> {
    self.next_stream += 1;
    let stream = Arc::new(Stream::new(format!("s-{}", self.next_stream)));
    self.streams.insert(stream.id().to_string(), Arc::clone(&stream));
    stream
  }

  /// `verification/explain` (spec §8.3): compile one interaction and return the plan's pretty text
  /// form, optionally the structured plan document.
  ///
  /// **A kernel operation precisely so no SDK builds its own** (the RFC). The three subjects a
  /// host can name are the three documents a plan is ever compiled from — an interaction
  /// specification, a Janus contract's interaction, a v1–v4 pact's interaction — and each is
  /// compiled by the *same* compiler the matching path uses, never a rendering-only imitation of
  /// it. `explain` of an *executed* plan is served by the event stream (spec §9), not here.
  fn handle_explain(&mut self, id: String, body: Value) -> ResponseFrame {
    let req: Explain = match parse_body(body) {
      Ok(req) => req,
      Err(err) => return ResponseFrame::err(id, err),
    };
    let plan = match explain::compile(&req.interaction) {
      Ok(plan) => plan,
      Err(err) => return ResponseFrame::err(id, explain_error(err)),
    };
    let mut result = json!({ "text": crate::plan::render_pretty(&plan) });
    if req
      .options
      .as_ref()
      .and_then(|options| options.get("plan"))
      .and_then(Value::as_bool)
      .unwrap_or(false)
    {
      result["plan"] = crate::plan::plan_json(&plan);
    }
    ResponseFrame::ok(id, result)
  }

  /// `upgrade/pact` (spec §8.4): a v1–v4 pact in, a Janus contract and its findings out.
  ///
  /// **Session-less on purpose** — conversion is pure document-in, document-out, so there is
  /// nothing to allocate and nothing to release. The findings are half the result, not a
  /// diagnostic channel: contract-file spec §8.1 requires the conversion to be honest rather than
  /// lossless, and a caller that ignored them would be reading only half of what it was told.
  fn handle_upgrade(&mut self, id: String, body: Value) -> ResponseFrame {
    let req: UpgradePact = match parse_body(body) {
      Ok(req) => req,
      Err(err) => return ResponseFrame::err(id, err),
    };
    match upgrade::pact("pact", &req.pact) {
      Ok(upgraded) => {
        let contract = serde_json::to_value(&upgraded.contract).expect("Contract always serializes");
        let findings = serde_json::to_value(&upgraded.findings).expect("Findings always serialize");
        ResponseFrame::ok(id, json!({ "contract": contract, "findings": findings }))
      }
      Err(err) => ResponseFrame::err(id, contract_error(err)),
    }
  }

  /// `events/poll` (spec §9.4). Every named stream must be live: an unknown or ended id fails the
  /// whole call with `stream-not-found` rather than being skipped, because a host that mixed a
  /// stale id into its list would otherwise read "no events" as "not yet".
  fn handle_poll(&mut self, id: String, body: Value) -> ResponseFrame {
    let req: Poll = match parse_body(body) {
      Ok(req) => req,
      Err(err) => return ResponseFrame::err(id, err),
    };
    let mut streams = Vec::with_capacity(req.streams.len());
    for name in &req.streams {
      match self.streams.get(name) {
        Some(stream) => streams.push(Arc::clone(stream)),
        None => return ResponseFrame::err(id, EngineError::stream_not_found(name)),
      }
    }

    let max = req.max();
    let mut events: Vec<Event> = Vec::new();
    for stream in &streams {
      if events.len() >= max {
        break;
      }
      events.extend(stream.drain(max - events.len()));
    }

    // Long poll (spec §9.4: the engine MAY hold the response). Waiting on the first named stream
    // rather than all of them keeps this a single wait: a host long-polling several streams at
    // once still gets everything pending from all of them on the next pass below, and the wait is
    // a latency optimisation, never a delivery guarantee.
    if events.is_empty() && req.wait_ms > 0 {
      if let Some(first) = streams.first() {
        first.wait_for_event(Duration::from_millis(req.wait_ms));
      }
      for stream in &streams {
        if events.len() >= max {
          break;
        }
        events.extend(stream.drain(max - events.len()));
      }
    }

    // Delivery is what ends a stream (spec §9.1), so the registry is pruned here and not by the
    // producer — and spec §7.1's "a verification session ends automatically when its run reaches a
    // terminal event" is the same edge, so the session goes with it. Joining the run's thread here
    // is what makes "everything the session allocated is released" true rather than hopeful: by
    // the time a host reads `finished`, the run has already stopped its transports.
    let ended: Vec<String> = self
      .streams
      .iter()
      .filter(|(_, stream)| stream.ended())
      .map(|(id, _)| id.clone())
      .collect();
    for stream_id in &ended {
      self.streams.remove(stream_id);
      let sessions: Vec<String> = self
        .verifications
        .iter()
        .filter(|(_, run)| run.stream.id() == stream_id)
        .map(|(session, _)| session.clone())
        .collect();
      for session in sessions {
        if let Some(run) = self.verifications.remove(&session)
          && run.thread.join().is_err()
        {
          tracing::error!(session = %session, "the verification run's thread panicked");
        }
      }
    }

    ResponseFrame::ok(id, json!({ "events": events }))
  }
}

/// An operation's request body, governed by the operation's schema (spec §4.1): a body that
/// doesn't match it is `malformed-frame`, same as an envelope violation (§4.4).
fn parse_body<T: DeserializeOwned>(body: Value) -> Result<T, EngineError> {
  serde_path_to_error::deserialize(body).map_err(|err| EngineError::malformed_frame(err.to_string()))
}

/// `upgrade/pact`'s request body (spec §8.4, `schemas/v1/upgrade.schema.json`).
#[derive(serde::Deserialize)]
struct UpgradePact {
  pact: Value,
  #[allow(dead_code)]
  #[serde(default)]
  options: Option<Value>,
}

/// A document that could not be read at all. Everything a *conversion* could not carry is a
/// finding, not an error (contract-file spec §8.1), so this covers exactly the window before the
/// conversion starts.
fn contract_error(err: crate::contract::ContractError) -> EngineError {
  use crate::contract::ContractError;
  match err {
    ContractError::NotAContract => EngineError::not_a_contract(0, None),
    ContractError::VersionUnsupported { format } => EngineError::not_a_contract(0, Some(&format)),
    ContractError::Invalid { problems } => EngineError::contract_invalid(&problems),
  }
}

/// [`ExplainError`] as the protocol's error taxonomy: a subject kind the engine does not know is
/// an operation it does not have, and a document that will not parse is the user's to fix.
fn explain_error(err: ExplainError) -> EngineError {
  match err {
    ExplainError::KindUnsupported(kind) => {
      EngineError::operation_unsupported(&format!("verification/explain (subject kind: {kind})"))
    }
    ExplainError::Missing(member) => EngineError::malformed_frame(format!(
      "verification/explain: '{member}' is required for this subject kind"
    )),
    ExplainError::NoSuchInteraction { index, count } => EngineError::malformed_frame(format!(
      "verification/explain: no interaction at index {index}; the document has {count}"
    )),
    ExplainError::Invalid(problems) => EngineError::interaction_invalid(&problems),
    ExplainError::NotReadable(message) => EngineError::malformed_frame(message),
  }
}

/// [`VerifyError`] as the protocol's own error taxonomy (spec §10.2). Each arm is a different
/// remedy: a source the engine cannot read is the host using an operation it doesn't have, a
/// document that is not a contract is a user/authoring problem, and a transport is a component.
fn verify_error(err: VerifyError) -> EngineError {
  match err {
    VerifyError::SourceUnsupported(kind) => {
      EngineError::operation_unsupported(&format!("verification/verify (source kind: {kind})"))
    }
    VerifyError::NotAContract { index, found } => EngineError::not_a_contract(index, found.as_deref()),
    VerifyError::ContractInvalid { problems } => EngineError::contract_invalid(&problems),
  }
}

/// The implementations every engine has, whatever embeds it: exactly one, `script`, and that is
/// the property ADR 0015 chose QuickJS for — a scripted hook runs in the WASM component, the
/// subprocess and the native embedding alike, so a project's hooks are not a function of how its
/// engine happens to be hosted.
fn builtin_hook_invokers() -> HashMap<String, Arc<dyn HookInvoker>> {
  let mut invokers: HashMap<String, Arc<dyn HookInvoker>> = HashMap::new();
  invokers.insert("script".to_string(), Arc::new(ScriptHooks::new()));
  invokers
}

/// [`ConfigError`] as the protocol's taxonomy (lifecycle-hooks spec §11). Two different remedies:
/// a document the author must fix, and an engine that cannot run what the document asks for.
fn hook_config_error(err: ConfigError) -> EngineError {
  match err {
    ConfigError::Invalid(problems) => EngineError::hook_config_invalid(&problems),
    ConfigError::Unavailable {
      kind,
      hook,
      available,
    } => EngineError::hook_unavailable(&kind, &hook, &available),
  }
}

/// Stop whatever was already started when a later target fails to start: a `verify` that returns
/// an error must leave nothing running (spec §7.1 — there is no session yet to release it).
fn stop_targets(targets: &[Target]) {
  for target in targets {
    let _ = target.component.stop(Stop {
      instance: target.instance.clone(),
    });
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  /// `events/poll` at the dispatch boundary. The stream's own guarantees (ordering, `seq`,
  /// backpressure) are [`super::events`]'s tests; these cover what only dispatch can show —
  /// the handshake gate, `stream-not-found` on either reason, and a stream id going invalid at
  /// the moment its terminal event is delivered.
  fn send(engine: &mut Engine, op: &str, body: Value) -> Value {
    let request = json!({ "type": "request", "id": "r-1", "op": op, "body": body });
    let bytes = engine.dispatch(&serde_json::to_vec(&request).expect("a json! literal serializes"));
    serde_json::from_slice(&bytes).expect("dispatch always returns valid JSON")
  }

  fn engine_after_hello() -> Engine {
    let mut engine = Engine::new();
    let ok = send(
      &mut engine,
      "engine/hello",
      json!({ "protocol-versions": [1], "host": { "name": "t", "version": "0" }, "capabilities": {} }),
    );
    assert_eq!(ok["ok"]["protocol-version"], 1);
    engine
  }

  #[test]
  fn shutdown_releases_every_session_answers_ok_and_leaves_the_engine_inert() {
    let mut engine = engine_after_hello();
    let created = send(
      &mut engine,
      "consumer-session/create",
      json!({ "config": { "consumer": { "name": "c" }, "provider": { "name": "p" } } }),
    );
    let session = created["ok"]["session"].as_str().expect("session id").to_string();

    let shutdown = send(&mut engine, "engine/shutdown", json!({}));
    assert_eq!(
      shutdown["ok"],
      json!({}),
      "spec §6: respond ok: {{}} before exiting"
    );
    assert!(engine.is_shut_down());

    // Inert: every later call, the handshake included, is engine-shut-down — and the session it
    // released is not reachable through it.
    for (op, body) in [
      ("consumer-session/finalise", json!({ "session": session })),
      ("engine/hello", json!({ "protocol-versions": [1] })),
      ("engine/shutdown", json!({})),
    ] {
      let err = send(&mut engine, op, body);
      assert_eq!(err["error"]["code"], "engine-shut-down", "{op}");
    }
  }

  #[test]
  fn polling_before_the_handshake_is_a_protocol_error() {
    let mut engine = Engine::new();
    let err = send(&mut engine, "events/poll", json!({ "streams": ["s-1"] }));
    assert_eq!(err["error"]["code"], "handshake-required");
  }

  #[test]
  fn polling_an_unknown_stream_names_it() {
    let mut engine = engine_after_hello();
    let err = send(&mut engine, "events/poll", json!({ "streams": ["s-99"] }));
    assert_eq!(err["error"]["code"], "stream-not-found");
    assert_eq!(err["error"]["category"], "session");
    assert_eq!(err["error"]["details"]["stream"], "s-99");
  }

  #[test]
  fn one_stale_id_among_live_ones_fails_the_whole_call() {
    let mut engine = engine_after_hello();
    let stream = engine.open_stream();
    stream.emit("verification/started", json!({}));
    let id = stream.id().to_string();

    let err = send(&mut engine, "events/poll", json!({ "streams": [id, "s-99"] }));
    assert_eq!(err["error"]["code"], "stream-not-found");
    assert_eq!(err["error"]["details"]["stream"], "s-99");
  }

  #[test]
  fn events_are_drained_in_order_and_the_stream_id_dies_with_its_terminal_event() {
    let mut engine = engine_after_hello();
    let stream = engine.open_stream();
    let id = stream.id().to_string();
    stream.emit("verification/started", json!({ "contracts": 1 }));
    stream.emit("verification/interaction-result", json!({ "status": "verified" }));
    stream.finish("verification/finished", json!({ "status": "verified" }));

    let first = send(&mut engine, "events/poll", json!({ "streams": [&id], "max": 2 }));
    let events = first["ok"]["events"].as_array().unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0]["seq"], 1);
    assert_eq!(events[0]["kind"], "verification/started");
    assert_eq!(events[1]["seq"], 2);
    assert_eq!(events[0]["last"], false);

    let second = send(&mut engine, "events/poll", json!({ "streams": [&id] }));
    let events = second["ok"]["events"].as_array().unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["kind"], "verification/finished");
    assert_eq!(events[0]["last"], true);

    // Delivered, therefore over (spec §9.1) — the same answer as an id that never existed.
    let third = send(&mut engine, "events/poll", json!({ "streams": [&id] }));
    assert_eq!(third["error"]["code"], "stream-not-found");
  }

  #[test]
  fn an_empty_poll_is_success_not_an_error() {
    let mut engine = engine_after_hello();
    let stream = engine.open_stream();
    let id = stream.id().to_string();
    let ok = send(&mut engine, "events/poll", json!({ "streams": [id] }));
    assert_eq!(ok["ok"]["events"], json!([]));
  }

  #[test]
  fn a_long_poll_collects_what_a_producer_thread_emits_while_it_waits() {
    let mut engine = engine_after_hello();
    let stream = engine.open_stream();
    let id = stream.id().to_string();
    let producer = Arc::clone(&stream);
    let thread = std::thread::spawn(move || {
      std::thread::sleep(std::time::Duration::from_millis(20));
      producer.finish("verification/finished", json!({ "status": "verified" }));
    });

    let ok = send(
      &mut engine,
      "events/poll",
      json!({ "streams": [id], "wait-ms": 5_000 }),
    );
    thread.join().expect("producer thread panicked");
    let events = ok["ok"]["events"].as_array().unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["last"], true);
  }

  #[test]
  fn stream_ids_are_distinct_within_the_pipe() {
    let mut engine = engine_after_hello();
    let first = engine.open_stream().id().to_string();
    let second = engine.open_stream().id().to_string();
    assert_ne!(first, second);
  }
}
