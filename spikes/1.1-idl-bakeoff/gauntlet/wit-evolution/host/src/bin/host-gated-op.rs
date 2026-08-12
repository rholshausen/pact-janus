//! E3, gated: bindings target the 0.2.0 interface with @since(0.2.0) on explain.
//! Loading the OLD guest (which exports events@0.1.0) tests whether the gate gives a
//! 0.2.0 host any path to a 0.1.0 artifact.
use gauntlet_host::gauntlet_host_main;

wasmtime::component::bindgen!({ path: "../wit/v0.2.0-gated-op", world: "plugin" });

gauntlet_host_main!();
