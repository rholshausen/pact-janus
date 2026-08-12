// Shared host logic, parameterised by the generated bindings via a macro so each binary
// carries bindings for exactly one interface version.

#[macro_export]
macro_rules! gauntlet_host_main {
  () => {
    fn main() -> anyhow::Result<()> {
      let path = std::env::args().nth(1).expect("usage: <host> <component.wasm>");
      let engine = wasmtime::Engine::default();
      let component = wasmtime::component::Component::from_file(&engine, &path)?;
      let mut linker = wasmtime::component::Linker::new(&engine);
      // The guests' WASI imports come from the wasip1 adapter and are never used;
      // trap-stubs keep this host free of wasmtime-wasi.
      linker.define_unknown_imports_as_traps(&component)?;
      let mut store = wasmtime::Store::new(&engine, ());

      let plugin = Plugin::instantiate(&mut store, &component, &linker)?;
      let events = plugin.pact_gauntlet_events();
      for seq in 0..8u32 {
        match events.call_next_event(&mut store, seq)? {
          Some(ev) => println!("event {seq}: {ev:?}"),
          None => {
            println!("event {seq}: <end of stream>");
            break;
          }
        }
      }
      println!("last-error: {:?}", events.call_last_error(&mut store)?);
      println!("RESULT: OK");
      Ok(())
    }
  };
}
