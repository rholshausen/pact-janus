//! Host whose bindings target the feature-gated 0.2.0 interface (@since gates on E1/E4).
//!
//! RESULT (kept deliberately): this binary DOES NOT COMPILE — the WIT parser rejects
//! @since on enum/variant cases, so the gated interface cannot even be expressed.
//! See FINDINGS.md §3. Build the other bins with `cargo build --release --bin host-old
//! --bin host-grown`.
use gauntlet_host::gauntlet_host_main;

wasmtime::component::bindgen!({ path: "../wit/v0.2.0-gated", world: "plugin" });

gauntlet_host_main!();
