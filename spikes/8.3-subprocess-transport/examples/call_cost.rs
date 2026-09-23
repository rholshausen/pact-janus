//! What the escape hatch costs: `cargo run --release --example call_cost`.
//!
//! Three numbers, each beside the one it should be read against — spike 1.3's pipe round trip for
//! the engine's own subprocess, and 8.1's ~69 µs fixed cost per WASM component call:
//!
//! 1. spawn to completed handshake (Node's own start-up dominates, and is the number to watch);
//! 2. the pipe: a `transport/stop` of an instance that does not exist, answered synchronously — and,
//!    beside it, `transport/poll-inbound` with `timeout-ms: 0`, which a Node component answers
//!    through a timer (the difference is the component's, not the pipe's);
//! 3. one consumer exchange end to end: a client's line in, the kernel's match, the reply out.

use pact_janus_component_json::JsonContent;
use pact_janus_kernel::component::{
  ComponentDeclaration, ComponentLoader, Grants, Limits, PollInbound, Source, Start,
};
use pact_janus_kernel::protocol::Engine;
use serde_json::{Value, json};
use spike_subprocess_transport::SubprocessLoader;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

fn command() -> String {
  let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("component/tcp-transport.mjs");
  format!("node {}", path.display())
}

fn declaration() -> ComponentDeclaration {
  ComponentDeclaration {
    name: "tcp".to_string(),
    source: Source {
      kind: "subprocess".to_string(),
      reference: Some(command()),
      digest: None,
    },
    grants: Grants::default(),
    limits: Limits::default(),
  }
}

fn median(mut samples: Vec<Duration>) -> Duration {
  samples.sort();
  samples[samples.len() / 2]
}

fn send(engine: &mut Engine, op: &str, body: Value) -> Value {
  let request = json!({ "type": "request", "id": "1", "op": op, "body": body });
  serde_json::from_slice(&engine.dispatch(&serde_json::to_vec(&request).unwrap())).unwrap()
}

fn main() {
  // 1. spawn to handshake
  let spawns: Vec<Duration> = (0..20)
    .map(|_| {
      let started = Instant::now();
      let loaded = SubprocessLoader.load(&declaration()).unwrap();
      let elapsed = started.elapsed();
      drop(loaded);
      elapsed
    })
    .collect();
  println!("spawn to handshake (node)      median {:?}", median(spawns));

  // 2. the pipe round trip
  let loaded = SubprocessLoader.load(&declaration()).unwrap();
  let transport = loaded.transport.clone().unwrap();
  transport
    .start(Start {
      instance: "t-1".to_string(),
      kind: "tcp".to_string(),
      role: "serve".to_string(),
      options: None,
    })
    .unwrap();
  for _ in 0..500 {
    transport
      .poll_inbound(PollInbound {
        instance: "t-1".to_string(),
        timeout_ms: 0,
      })
      .unwrap();
  }
  let calls = 5_000;
  let started = Instant::now();
  for _ in 0..calls {
    transport
      .poll_inbound(PollInbound {
        instance: "t-1".to_string(),
        timeout_ms: 0,
      })
      .unwrap();
  }
  println!(
    "poll-inbound(0) round trip     mean {:?}",
    started.elapsed() / calls
  );
  let started = Instant::now();
  for _ in 0..calls {
    transport
      .stop(pact_janus_kernel::component::Stop {
        instance: "none".to_string(),
      })
      .unwrap();
  }
  println!(
    "synchronous op round trip      mean {:?}",
    started.elapsed() / calls
  );
  drop(loaded);

  // 3. a consumer exchange, end to end
  let mut engine = Engine::with_components(HashMap::new(), Some(Arc::new(JsonContent::new())));
  engine.register_component_loader(Arc::new(SubprocessLoader));
  send(
    &mut engine,
    "engine/hello",
    json!({ "protocol-versions": [1], "host": { "name": "cost", "version": "0" }, "capabilities": {} }),
  );
  let session = send(
    &mut engine,
    "consumer-session/create",
    json!({ "config": { "consumer": { "name": "c" }, "provider": { "name": "p" },
                        "components": [ { "name": "tcp", "source": { "kind": "subprocess", "reference": command() } } ] } }),
  )["ok"]["session"]
    .clone();
  let handle = send(
    &mut engine,
    "consumer-session/add-interaction",
    json!({ "session": session, "interaction": {
      "description": "lookup", "transport": { "kind": "tcp", "mode": "passive" },
      "content-types": { "request": { "body": "application/json" }, "response": { "body": "application/json" } },
      "parts": { "request": { "body": { "shape": "object", "members": { "id": { "shape": "string", "example": "66" } } } },
                 "response": { "body": { "shape": "object", "members": { "status": { "shape": "equality", "example": "PENDING" } } } } } } }),
  )["ok"]["handle"]
    .clone();
  let variant = send(
    &mut engine,
    "consumer-session/variants",
    json!({ "session": session, "handle": handle }),
  )["ok"]["variants"][0]["id"]
    .clone();
  let endpoint = send(
    &mut engine,
    "consumer-session/start-transport",
    json!({ "session": session, "transport": "tcp" }),
  )["ok"]["endpoint"]
    .clone();
  let addr = format!("{}:{}", endpoint["host"].as_str().unwrap(), endpoint["port"]);
  let exchanges: Vec<Duration> = (0..200)
    .map(|_| {
      send(
        &mut engine,
        "consumer-session/serve-variant",
        json!({ "session": session, "handle": handle, "variant": variant }),
      );
      let started = Instant::now();
      let mut stream = TcpStream::connect(&addr).unwrap();
      writeln!(stream, r#"{{"id":"66"}}"#).unwrap();
      let mut reply = String::new();
      BufReader::new(stream).read_line(&mut reply).unwrap();
      started.elapsed()
    })
    .collect();
  println!("consumer exchange, end to end  median {:?}", median(exchanges));
}
