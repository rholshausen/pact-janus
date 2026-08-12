//! E3, generated-bindings host: bindings target the grown-op interface (explain added
//! in place, still @0.1.0). Loading the OLD guest tests whether an added operation
//! strands old plugins when the host uses eager generated bindings.
use anyhow::Result;

wasmtime::component::bindgen!({ path: "../wit/v0.1.0-grown-op", world: "plugin" });

fn main() -> Result<()> {
  let path = std::env::args().nth(1).expect("usage: host-grown-op <component.wasm>");
  let engine = wasmtime::Engine::default();
  let component = wasmtime::component::Component::from_file(&engine, &path)?;
  let mut linker = wasmtime::component::Linker::new(&engine);
  linker.define_unknown_imports_as_traps(&component)?;
  let mut store = wasmtime::Store::new(&engine, ());

  let plugin = Plugin::instantiate(&mut store, &component, &linker)?;
  let events = plugin.pact_gauntlet_events();
  println!("explain: {}", events.call_explain(&mut store, "get an order")?);
  println!("last-error: {:?}", events.call_last_error(&mut store)?);
  println!("RESULT: OK");
  Ok(())
}
