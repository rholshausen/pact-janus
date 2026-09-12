//! Shared test support. Not a test binary itself (Rust's integration-test harness only promotes
//! top-level `tests/*.rs` files to that) — a plain module `tests/variant_select.rs` and friends
//! pull in with `mod support;`.

/// Install a `tracing` subscriber reading `RUST_LOG`, so `RUST_LOG=debug cargo test --
/// -nocapture` (CLAUDE.md's own build-command list) has somewhere to send the kernel's trace
/// output. Call at the top of a test that wants it; safe to call from every test in the binary —
/// `try_init` silently ignores "a subscriber is already installed".
pub fn init_tracing() {
  let _ = tracing_subscriber::fmt()
    .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
    .with_test_writer()
    .try_init();
}
