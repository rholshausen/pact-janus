//! Host whose bindings target the baseline interface (pact:gauntlet@0.1.0).
use gauntlet_host::gauntlet_host_main;

wasmtime::component::bindgen!({ path: "../wit/v0.1.0", world: "plugin" });

gauntlet_host_main!();
