//! `janus-engine`: the engine as a subprocess (ADR 0023: every SDK's primary embedding). Frames in
//! on stdin and out on stdout, `Content-Length` framed ([`pact_janus_cli::framing`]); logs on stderr
//! only, so stdout carries nothing but frames.
//!
//! Exit path: stdin EOF (engine-protocol spec §6/§7.1's orphan-prevention mechanism — no process
//! groups or signal choreography needed, spike 1.3 finding 2), or `engine/shutdown`, answered first
//! so a host can tell a clean exit from a crash. A framing violation (a malformed `Content-Length`
//! header, EOF mid-frame) poisons the stream — there is no way to resynchronise a byte-counted
//! protocol — so it is reported on stderr and the process exits non-zero rather than guessing.
//!
//! The command line is `--help` and `--version` and nothing else: an SDK spawns this with no
//! arguments, and everything it configures goes through the protocol.
//!
//! This binary is where the kernel's `tracing` facade goes somewhere (AGENTS.md: "a subscriber is
//! an application concern, installed by whatever embeds the kernel"). `Engine::dispatch` traces
//! every frame at `trace` level, so `RUST_LOG=trace janus-engine` shows exactly what crossed the
//! wire in both directions.

use pact_janus_cli::framing::{read_frame, write_frame};
use std::io::{BufReader, BufWriter};
use tracing_subscriber::EnvFilter;

fn main() {
  clap::Command::new("janus-engine")
    // Not argv[0], which on Windows is `janus-engine.exe`.
    .bin_name("janus-engine")
    .version(pact_janus_cli::VERSION.as_str())
    .about(
      "The Janus engine as a subprocess: protocol frames on standard input and output, logs on \
       standard error. SDKs start it; RUST_LOG sets what it logs (default: info).",
    )
    .get_matches();

  tracing_subscriber::fmt()
    .with_writer(std::io::stderr)
    .with_ansi(pact_janus_cli::stderr_supports_colour())
    .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
    .init();

  tracing::info!(
    pid = std::process::id(),
    protocol_version = pact_janus_kernel::PROTOCOL_VERSION,
    "janus-engine starting"
  );

  // The same engine `janus` drives (phase-9 finding 30): transports, content, the WASM loader for
  // the components a project declares, and the hook implementations a verifier configuration names.
  let mut engine = pact_janus_cli::register::native_engine();

  let stdin = std::io::stdin();
  let stdout = std::io::stdout();
  let mut reader = BufReader::new(stdin.lock());
  let mut writer = BufWriter::new(stdout.lock());

  loop {
    match read_frame(&mut reader) {
      Ok(Some(request)) => {
        let response = engine.dispatch(&request);
        send(&mut writer, &response);
        // engine-protocol spec §6: the shutdown response is written before exit, so a host can
        // tell a clean shutdown from a crash.
        if engine.is_shut_down() {
          tracing::info!("engine/shutdown answered, exiting");
          std::process::exit(0);
        }
      }
      Ok(None) => {
        tracing::info!("stdin closed, exiting");
        std::process::exit(0);
      }
      Err(err) => {
        tracing::error!(%err, "framing error");
        send(
          &mut writer,
          br#"{"type":"response","id":"","error":{"code":"malformed-frame","category":"protocol","message":"malformed Content-Length framing"}}"#,
        );
        std::process::exit(1);
      }
    }
  }
}

/// A write failure means the host is gone; exit rather than spin.
fn send(writer: &mut impl std::io::Write, frame: &[u8]) {
  if let Err(err) = write_frame(writer, frame) {
    tracing::error!(%err, "stdout write failed, exiting");
    std::process::exit(1);
  }
}
