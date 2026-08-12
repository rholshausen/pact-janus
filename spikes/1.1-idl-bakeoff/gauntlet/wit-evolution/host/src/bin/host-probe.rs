//! E3, feature-detecting host: no generated bindings. Uses the untyped component API to
//! look up the events interface, probe for `explain`, and degrade gracefully when it is
//! absent — the "new ops are optional capabilities" pattern an engine hosting third-party
//! plugins would need. Works against both the old guest (no explain) and guest-new-op.
use anyhow::{Context, Result, anyhow};
use wasmtime::component::{Component, Linker, Val};
use wasmtime::{Engine, Store};

const INTERFACE: &str = "pact:gauntlet/events@0.1.0";

fn main() -> Result<()> {
  let path = std::env::args().nth(1).expect("usage: host-probe <component.wasm>");
  let engine = Engine::default();
  let component = Component::from_file(&engine, &path)?;
  let mut linker: Linker<()> = Linker::new(&engine);
  linker.define_unknown_imports_as_traps(&component)?;
  let mut store = Store::new(&engine, ());
  let instance = linker.instantiate(&mut store, &component)?;

  let events = instance
    .get_export_index(&mut store, None, INTERFACE)
    .ok_or_else(|| anyhow!("interface {INTERFACE} not exported"))?;

  // Baseline operations must exist.
  let next_event = instance
    .get_export_index(&mut store, Some(&events), "next-event")
    .and_then(|i| instance.get_func(&mut store, i))
    .context("next-event missing")?;
  let mut results = [Val::Bool(false)];
  next_event.call(&mut store, &[Val::U32(0)], &mut results)?;
  next_event.post_return(&mut store)?;
  println!("next-event(0): {results:?}");

  // E3 probe: explain is optional; absence is a capability signal, not an error.
  match instance
    .get_export_index(&mut store, Some(&events), "explain")
    .and_then(|i| instance.get_func(&mut store, i))
  {
    Some(explain) => {
      let mut results = [Val::Bool(false)];
      explain.call(&mut store, &[Val::String("get an order".into())], &mut results)?;
      explain.post_return(&mut store)?;
      println!("explain: {results:?}");
    }
    None => println!("explain: <not exported — plugin predates E3, degrading gracefully>"),
  }
  println!("RESULT: OK");
  Ok(())
}
