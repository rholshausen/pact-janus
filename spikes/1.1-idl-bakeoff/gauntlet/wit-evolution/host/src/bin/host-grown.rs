//! Host whose bindings target the in-place-grown interface (E1+E4, still @0.1.0).
//! Loading the OLD guest here is the critical D-a direction: new engine, old plugin.
use gauntlet_host::gauntlet_host_main;

wasmtime::component::bindgen!({ path: "../wit/v0.1.0-grown", world: "plugin" });

gauntlet_host_main!();
