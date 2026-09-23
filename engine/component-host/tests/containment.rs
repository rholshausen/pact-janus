//! The WASM loader's containment obligations (component-interfaces spec §3.4, §9.2, §11.2), against a
//! component built to break them (`tests/fixtures/misbehaving`): a trap, a runaway loop and a
//! malformed frame each reach the caller as a value marked as the engine's, the next call works —
//! nothing is reused after a trap — and a socket import is refused at load unless it was granted.

use pact_janus_component_host::WasmLoader;
use pact_janus_kernel::component::{
  ComponentDeclaration, ComponentLoader, ContentComponent, Decode, Grants, Limits, SlotValue, Source,
};
use pretty_assertions::assert_eq;
use serde_json::json;
use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

/// Build the fixture — with or without its socket import — and return its path.
fn fixture(network: bool) -> String {
  static PLAIN: OnceLock<String> = OnceLock::new();
  static NETWORK: OnceLock<String> = OnceLock::new();
  let cell = if network { &NETWORK } else { &PLAIN };
  cell
    .get_or_init(|| {
      let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/misbehaving");
      let target = if network { "target/network" } else { "target/plain" };
      let mut command = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string()));
      command
        .args([
          "build",
          "--release",
          "--target",
          "wasm32-wasip2",
          "--target-dir",
          target,
        ])
        .current_dir(&dir)
        .env_remove("CARGO_TARGET_DIR")
        .env_remove("RUSTFLAGS");
      if network {
        command.args(["--features", "network"]);
      }
      assert!(
        command.status().expect("cargo runs").success(),
        "building the fixture"
      );
      dir
        .join(target)
        .join("wasm32-wasip2/release/misbehaving.wasm")
        .display()
        .to_string()
    })
    .clone()
}

fn declaration(network: bool, grants: Grants, deadline_ms: Option<u64>) -> ComponentDeclaration {
  ComponentDeclaration {
    name: "misbehaving".to_string(),
    source: Source {
      kind: "file".to_string(),
      reference: Some(fixture(network)),
      digest: None,
    },
    grants,
    limits: Limits {
      deadline_ms,
      instances: None,
    },
  }
}

fn decode(
  content: &dyn ContentComponent,
  content_type: &str,
) -> Result<serde_json::Value, pact_janus_kernel::component::ComponentError> {
  content
    .decode(Decode {
      content_type: content_type.to_string(),
      value: SlotValue {
        content: json!(""),
        encoded: Some("text".to_string()),
        content_type: None,
      },
      options: None,
    })
    .map(|result| result.document.to_json())
}

#[test]
fn a_trap_is_the_engines_error_and_the_next_call_gets_a_fresh_instance() {
  let loader = WasmLoader::new().unwrap();
  let loaded = loader
    .load(&declaration(false, Grants::default(), None))
    .expect("loads");
  let content = loaded.content.expect("declared content");

  let trapped = decode(content.as_ref(), "x/trap").expect_err("a panic is a trap");
  assert_eq!(trapped.code, "component-trapped");
  assert_eq!(
    trapped.source.as_deref(),
    Some("engine"),
    "the component did not say this; the binding did"
  );

  assert_eq!(
    decode(content.as_ref(), "x/fine").expect("a fresh instance answers"),
    json!("ok")
  );
}

#[test]
fn a_runaway_call_is_stopped_at_its_deadline() {
  let loader = WasmLoader::new().unwrap();
  let loaded = loader
    .load(&declaration(false, Grants::default(), Some(200)))
    .expect("loads");
  let content = loaded.content.expect("declared content");

  let started = Instant::now();
  let timed_out = decode(content.as_ref(), "x/spin").expect_err("it never returns by itself");
  assert_eq!(timed_out.code, "component-timeout");
  assert_eq!(timed_out.source.as_deref(), Some("engine"));
  assert!(
    started.elapsed() < Duration::from_secs(5),
    "bounded: {:?}",
    started.elapsed()
  );

  assert_eq!(
    decode(content.as_ref(), "x/fine").expect("and the next call is fine"),
    json!("ok")
  );
}

#[test]
fn a_frame_that_is_not_one_is_the_components_failure_not_a_crash() {
  let loader = WasmLoader::new().unwrap();
  let content = loader
    .load(&declaration(false, Grants::default(), None))
    .expect("loads")
    .content
    .expect("declared content");
  let garbage = decode(content.as_ref(), "x/garbage").expect_err("not a frame");
  assert_eq!(garbage.code, "component-malformed");
  assert_eq!(garbage.source.as_deref(), Some("engine"));
}

#[test]
fn a_socket_import_needs_the_network_grant_and_is_refused_at_load_without_it() {
  let loader = WasmLoader::new().unwrap();
  let refused = loader
    .load(&declaration(true, Grants::default(), None))
    .err()
    .expect("imports exceed grants");
  assert_eq!(refused.code, "capability-denied");
  assert_eq!(refused.details.as_ref().unwrap()["grant"], "network");
  assert!(
    refused.details.as_ref().unwrap()["import"]
      .as_str()
      .unwrap()
      .starts_with("wasi:sockets/"),
    "{refused:?}"
  );

  let granted = Grants {
    network: true,
    ..Grants::default()
  };
  assert!(
    loader.load(&declaration(true, granted, None)).is_ok(),
    "granted, it loads"
  );
}

#[test]
fn std_imports_are_linked_to_nothing_and_do_not_need_a_grant() {
  // The plain fixture imports wasi:cli/environment, stdio, exit and clocks — what `std` brings — and
  // no grant at all.
  let loader = WasmLoader::new().unwrap();
  assert!(loader.load(&declaration(false, Grants::default(), None)).is_ok());
}
