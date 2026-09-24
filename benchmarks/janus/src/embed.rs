//! The three ways a host reaches the engine, behind one pipe: frame bytes in, frame bytes out
//! (ADR 0002). Every scenario is written once against [`Client`], so what differs between the
//! embeddings' numbers is the embedding and nothing else.

use pact_janus_component_http::HttpTransport;
use pact_janus_component_json::JsonContent;
use pact_janus_hooks_host::{ExecHooks, HttpHooks};
use pact_janus_kernel::component::TransportComponent;
use pact_janus_kernel::hooks::HookInvoker;
use pact_janus_kernel::protocol::Engine;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::Arc;

pub trait Pipe {
  fn call(&mut self, frame: &[u8]) -> Vec<u8>;
}

/// A protocol client over any pipe: builds request frames, returns `ok` or the `error` document.
pub struct Client {
  pipe: Box<dyn Pipe>,
  next: u64,
}

impl Client {
  /// Wraps `pipe` and completes the handshake, as every host must before anything else.
  pub fn new(pipe: Box<dyn Pipe>) -> Self {
    let mut client = Client { pipe, next: 0 };
    client.ok(
      "engine/hello",
      json!({ "protocol-versions": [1], "host": { "name": "pact-bench-janus", "version": "0.1.0" },
              "capabilities": {} }),
    );
    client
  }

  pub fn call(&mut self, op: &str, body: Value) -> Result<Value, Value> {
    self.next += 1;
    let request = json!({ "type": "request", "id": format!("r-{}", self.next), "op": op, "body": body });
    let bytes = self.pipe.call(&serde_json::to_vec(&request).unwrap());
    let mut response: Value = serde_json::from_slice(&bytes).expect("the engine answers JSON frames");
    match response.get_mut("ok") {
      Some(ok) => Ok(ok.take()),
      None => Err(response["error"].take()),
    }
  }

  pub fn ok(&mut self, op: &str, body: Value) -> Value {
    self
      .call(op, body)
      .unwrap_or_else(|err| panic!("{op} failed: {err}"))
  }
}

// ------------------------------------------------------------------ native, in-process

/// The engine in the host's own process, wired the way the `janus` CLI wires it (cli/src/engine.rs):
/// the HTTP transport, the JSON content component, and the `exec` and `http` hook implementations.
/// The lower bound — what the work costs with no boundary to cross.
pub struct Native(Engine);

impl Native {
  pub fn new() -> Self {
    let mut transports: HashMap<String, Arc<dyn TransportComponent>> = HashMap::new();
    transports.insert("http".to_string(), Arc::new(HttpTransport::new()));
    let mut engine = Engine::with_components(transports, Some(Arc::new(JsonContent::new())));
    engine.declare_in_tree("content", "json", "1.0.0");
    engine.register_hook_invoker("exec", Arc::new(ExecHooks::new()) as Arc<dyn HookInvoker>);
    engine.register_hook_invoker("http", Arc::new(HttpHooks::new()) as Arc<dyn HookInvoker>);
    Native(engine)
  }
}

impl Pipe for Native {
  fn call(&mut self, frame: &[u8]) -> Vec<u8> {
    self.0.dispatch(frame)
  }
}

// ------------------------------------------------------------------ subprocess

/// `janus-engine` over its Content-Length stdio framing (ADR 0003, spike 1.3). Dropping it closes
/// stdin, which is the engine's exit signal.
pub struct Subprocess {
  child: Child,
  stdin: Option<BufWriter<ChildStdin>>,
  stdout: BufReader<ChildStdout>,
}

impl Subprocess {
  pub fn spawn(binary: &Path) -> Self {
    // `JANUS_BENCH_LOG=debug` shows the engine's own tracing on stderr, for when a scenario fails.
    let log = std::env::var("JANUS_BENCH_LOG").ok();
    let mut child = Command::new(binary)
      .stdin(Stdio::piped())
      .stdout(Stdio::piped())
      .stderr(if log.is_some() {
        Stdio::inherit()
      } else {
        Stdio::null()
      })
      .env("RUST_LOG", log.as_deref().unwrap_or("error"))
      .spawn()
      .unwrap_or_else(|err| panic!("spawning {}: {err}", binary.display()));
    let stdin = BufWriter::new(child.stdin.take().unwrap());
    let stdout = BufReader::new(child.stdout.take().unwrap());
    Subprocess {
      child,
      stdin: Some(stdin),
      stdout,
    }
  }

  /// Closes stdin and waits for the process to exit — the whole of an orderly shutdown.
  pub fn close(mut self) {
    self.stdin.take();
    let _ = self.child.wait();
  }
}

impl Pipe for Subprocess {
  fn call(&mut self, frame: &[u8]) -> Vec<u8> {
    let stdin = self.stdin.as_mut().expect("open");
    // One write per frame, as the SDKs do.
    let mut message = format!("Content-Length: {}\r\n\r\n", frame.len()).into_bytes();
    message.extend_from_slice(frame);
    stdin.write_all(&message).expect("writing a frame");
    stdin.flush().expect("flushing a frame");

    let mut length = None;
    let mut line = String::new();
    loop {
      line.clear();
      if self.stdout.read_line(&mut line).expect("reading a frame header") == 0 {
        panic!("janus-engine closed its stdout");
      }
      let header = line.trim_end();
      if header.is_empty() {
        break;
      }
      if let Some(value) = header.strip_prefix("Content-Length:") {
        length = Some(value.trim().parse::<usize>().expect("a numeric Content-Length"));
      }
    }
    let mut body = vec![0u8; length.expect("a Content-Length header")];
    self.stdout.read_exact(&mut body).expect("reading a frame body");
    body
  }
}

impl Drop for Subprocess {
  fn drop(&mut self) {
    self.stdin.take();
    let _ = self.child.wait();
  }
}

// ------------------------------------------------------------------ WASM

mod guest {
  wasmtime::component::bindgen!({ world: "engine", path: "engine-wasm/wit" });
}

use wasmtime::component::{Component, Linker, ResourceTable};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

pub struct WasiHost {
  ctx: WasiCtx,
  table: ResourceTable,
}

impl WasiView for WasiHost {
  fn ctx(&mut self) -> WasiCtxView<'_> {
    WasiCtxView {
      ctx: &mut self.ctx,
      table: &mut self.table,
    }
  }
}

/// The compiled engine component and a linker for it: what a host pays for once per process.
pub struct WasmRuntime {
  engine: wasmtime::Engine,
  component: Component,
  linker: Linker<WasiHost>,
}

impl WasmRuntime {
  /// Compiles the component from its bytes, with no compilation cache — the cold path.
  pub fn compile(bytes: &[u8]) -> Self {
    let engine = wasmtime::Engine::new(&wasmtime::Config::new()).expect("wasmtime starts");
    let component = Component::from_binary(&engine, bytes).expect("the engine component compiles");
    let mut linker = Linker::new(&engine);
    wasmtime_wasi::p2::add_to_linker_sync(&mut linker).expect("WASI links");
    WasmRuntime {
      engine,
      component,
      linker,
    }
  }

  /// A fresh instance with an empty WASI context: no environment, no files, no sockets.
  pub fn instantiate(&self) -> Wasm {
    let host = WasiHost {
      ctx: WasiCtxBuilder::new().build(),
      table: ResourceTable::new(),
    };
    let mut store = wasmtime::Store::new(&self.engine, host);
    let instance = guest::Engine::instantiate(&mut store, &self.component, &self.linker)
      .expect("the engine component instantiates");
    Wasm { store, instance }
  }
}

pub struct Wasm {
  store: wasmtime::Store<WasiHost>,
  instance: guest::Engine,
}

impl Pipe for Wasm {
  fn call(&mut self, frame: &[u8]) -> Vec<u8> {
    self
      .instance
      .pact_janus_engine_pipe()
      .call_call(&mut self.store, frame)
      .expect("the guest does not trap")
  }
}

impl Wasm {
  /// The bench-only interface (engine-wasm/wit/engine.wit): a compiled plan, run inside the guest.
  pub fn set_plan(&mut self, plan: &[u8]) {
    self
      .instance
      .pact_janus_engine_bench()
      .call_set_plan(&mut self.store, plan)
      .expect("the guest does not trap")
      .expect("the plan document reads back");
  }

  pub fn execute(&mut self, values: &[u8]) -> String {
    self
      .instance
      .pact_janus_engine_bench()
      .call_execute(&mut self.store, values)
      .expect("the guest does not trap")
      .expect("the values read")
  }
}
