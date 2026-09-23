//! What one call through the WASM binding costs (component-interfaces spec §13's measurement table,
//! plan task 8.1): the third-party CSV component's `decode`, each call a fresh instance with its own
//! handshake — the loader's instance-per-call policy — against the in-tree JSON component's native
//! `decode` of the same rows. Run after building `third-party/janus-csv` for `wasm32-wasip2`:
//!
//!   cargo run --release -p pact_janus_component_host --example call_cost -- <path to janus_csv.wasm>

use pact_janus_component_host::WasmLoader;
use pact_janus_component_json::JsonContent;
use pact_janus_kernel::component::{
  ComponentDeclaration, ComponentLoader, ContentComponent, Decode, Grants, Limits, SlotValue, Source,
};
use serde_json::json;
use std::time::Instant;

fn main() {
  let path = std::env::args().nth(1).expect("the path of janus_csv.wasm");
  let loader = WasmLoader::new().expect("wasmtime starts");
  let started = Instant::now();
  let csv = loader
    .load(&ComponentDeclaration {
      name: "csv".to_string(),
      source: Source {
        kind: "file".to_string(),
        reference: Some(path),
        digest: None,
      },
      grants: Grants::default(),
      limits: Limits::default(),
    })
    .expect("loads")
    .content
    .expect("a content component");
  println!("load (read, compile, link, handshake): {:?}", started.elapsed());

  for rows in [1usize, 100, 1_000] {
    let text: String = std::iter::once("id,status,items\n".to_string())
      .chain((0..rows).map(|n| format!("{n},PENDING,{}\n", n % 7)))
      .collect();
    let json_text = serde_json::to_string(
      &(0..rows)
        .map(|n| json!({ "id": n.to_string(), "status": "PENDING", "items": (n % 7).to_string() }))
        .collect::<Vec<_>>(),
    )
    .unwrap();
    let wasm = time(csv.as_ref(), "text/csv", &text);
    let native = time(&JsonContent::new(), "application/json", &json_text);
    println!(
      "{rows:>5} rows, {:>7} bytes: wasm csv decode {wasm:>9.1} µs/call   native json decode {native:>8.1} µs/call",
      text.len()
    );
  }
}

fn time(component: &dyn ContentComponent, content_type: &str, text: &str) -> f64 {
  let request = || Decode {
    content_type: content_type.to_string(),
    value: SlotValue {
      content: json!(text),
      encoded: Some("text".to_string()),
      content_type: None,
    },
    options: None,
  };
  component.decode(request()).expect("decodes"); // warm
  let calls = 200;
  let started = Instant::now();
  for _ in 0..calls {
    component.decode(request()).expect("decodes");
  }
  started.elapsed().as_secs_f64() * 1e6 / calls as f64
}
