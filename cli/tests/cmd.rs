//! The command surface as snapshots: every case under `tests/cmd/` runs the shipped binary and
//! compares what it prints and how it exits. `help.md` is the CLI's reference — a flag that changes
//! shows up there as a diff in review — and `usage.md` the usage errors and their exit code. The
//! `.toml` cases run offline commands from the repository root against checked-in documents.
//!
//! The snapshots hold only what the CLI itself says. Text the engine writes — a plan, a
//! compatibility report, a finding's message — is elided with `...` or `[..]`: `corpora/` pins it,
//! and pinning it twice would make every engine change a CLI change.
//!
//! After a deliberate change: `TRYCMD=overwrite cargo test -p pact_janus_cli --test cmd`, then read
//! the diff.

#[test]
fn cli_cases() {
  trycmd::TestCases::new()
    .register_bin("janus", std::path::Path::new(env!("CARGO_BIN_EXE_janus")))
    .register_bin(
      "janus-engine",
      std::path::Path::new(env!("CARGO_BIN_EXE_janus-engine")),
    )
    // Help wraps to the terminal; a test has none, and a CI runner's COLUMNS must not decide.
    .env("COLUMNS", "100")
    .env("RUST_LOG", "off")
    .case("tests/cmd/*.md")
    .case("tests/cmd/*.toml");
}
