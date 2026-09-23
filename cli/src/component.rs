//! `janus component push|pull` (plan task 8.2, component-interfaces spec §10.3): publish a WASM
//! component as an OCI artifact, and fetch one into the cache to find out what it is and what to pin.
//!
//! **The one command that does not go through the protocol**, and deliberately: publishing is not
//! an engine operation, and a pull here is the WASM loader's own fetch-and-check, run by hand — the
//! very code a run uses when a declaration names an `oci` source, called from the same crate the
//! engine registers. Nothing here reaches into the kernel. What a run *does* with a component still
//! goes only through the protocol, which is why `verify --config` is how a component is used.
//!
//! Both commands end by printing the declaration to paste, pinned — because a tag is a name and a
//! digest is the artifact (spec §10.3), and the easiest moment to pin is the one where the digest
//! is on the screen.

use crate::args::{Args, Spec};
use crate::io;
use pact_janus_component_host::WasmLoader;
use pact_janus_kernel::component::ComponentError;
use serde_json::{Value, json};
use std::process::ExitCode;

pub const SPEC: Spec = Spec {
  values: &["digest"],
  flags: &["json"],
};

pub const USAGE: &str = "\
usage: janus component push <component.wasm> <reference> [--json]
       janus component pull <reference> [--digest <sha256:...>] [--json]

  push     publish a WASM component as an OCI artifact; its config is written from the
           component's own handshake, so the artifact cannot say it is something else
  pull     fetch a component into the cache, check every byte, and say what it is

  <reference>                registry/repository:tag, or @sha256:... for a digest
  --digest <sha256:...>      the manifest digest the pull must match
  --json                     print the result as one JSON document

Loopback registries (localhost, 127.0.0.1) are plain HTTP; everything else is HTTPS.
Credentials, when a registry wants them: JANUS_OCI_USERNAME and JANUS_OCI_PASSWORD.
The cache is JANUS_COMPONENT_CACHE, or pact-janus/components in the user cache directory.

Exit codes: 0 done; 1 the artifact is not what it claims (a digest that does not match,
not a Janus component, a config that disagrees with the handshake); 2 could not run.";

pub fn run(args: &Args) -> ExitCode {
  let (subcommand, rest) = match args.positionals().split_first() {
    Some((subcommand, rest)) => (subcommand.as_str(), rest),
    None => return io::usage("component", "missing 'push' or 'pull'", USAGE),
  };
  let loader = match WasmLoader::new() {
    Ok(loader) => loader,
    Err(err) => return io::fail(&format!("janus component: {err}")),
  };
  match (subcommand, rest) {
    ("push", [path, reference]) => push(&loader, path, reference, args.flag("json")),
    ("pull", [reference]) => pull(&loader, reference, args.value("digest"), args.flag("json")),
    ("push", _) => io::usage("component", "push takes <component.wasm> <reference>", USAGE),
    ("pull", _) => io::usage("component", "pull takes <reference>", USAGE),
    (other, _) => io::usage("component", &format!("unknown subcommand '{other}'"), USAGE),
  }
}

fn push(loader: &WasmLoader, path: &str, reference: &str, as_json: bool) -> ExitCode {
  let wasm = match std::fs::read(path) {
    Ok(wasm) => wasm,
    Err(err) => return io::fail(&format!("janus component push: {path}: {err}")),
  };
  let pushed = match loader.push(&wasm, reference) {
    Ok(pushed) => pushed,
    Err(err) => return refused("push", &err),
  };
  let summary = json!({
    "reference": pushed.reference,
    "digest": pushed.digest,
    "component": pushed.config,
    "interfaces": pushed.hello.get("interfaces").cloned().unwrap_or(Value::Null),
  });
  if as_json {
    println!("{}", serde_json::to_string_pretty(&summary).unwrap_or_default());
  } else {
    println!("pushed {} to {}", describe(&pushed.hello), pushed.reference);
    println!("digest: {}", pushed.digest);
    declaration(&pushed.config, &pushed.reference, &pushed.digest);
  }
  ExitCode::SUCCESS
}

fn pull(loader: &WasmLoader, reference: &str, digest: Option<&str>, as_json: bool) -> ExitCode {
  let (pulled, hello) = match loader.pull(reference, digest) {
    Ok(pulled) => pulled,
    Err(err) => return refused("pull", &err),
  };
  let summary = json!({
    "reference": pulled.reference.to_string(),
    "digest": pulled.digest,
    "component": pulled.config,
    "interfaces": hello.get("interfaces").cloned().unwrap_or(Value::Null),
    "contributes": hello.get("contributes").cloned().unwrap_or(Value::Null),
  });
  if as_json {
    println!("{}", serde_json::to_string_pretty(&summary).unwrap_or_default());
  } else {
    println!("{} — every byte checked", describe(&hello));
    println!("digest: {}", pulled.digest);
    if let Some(types) = hello
      .pointer("/contributes/content-types")
      .and_then(Value::as_array)
    {
      let types: Vec<&str> = types
        .iter()
        .filter_map(|entry| entry.get("media-type").and_then(Value::as_str))
        .collect();
      println!("content types: {}", types.join(", "));
    }
    let mut unpinned = pulled.reference.clone();
    unpinned.digest = None;
    declaration(&pulled.config, &unpinned.to_string(), &pulled.digest);
  }
  ExitCode::SUCCESS
}

/// `csv 1.0.0 (content)`, from the handshake: what answers, not what an artifact says.
fn describe(hello: &Value) -> String {
  let interfaces: Vec<&str> = hello["interfaces"]
    .as_array()
    .into_iter()
    .flatten()
    .filter_map(Value::as_str)
    .collect();
  format!(
    "{} {} ({})",
    hello["component"]["name"].as_str().unwrap_or("?"),
    hello["component"]["version"].as_str().unwrap_or("?"),
    interfaces.join(", ")
  )
}

fn declaration(config: &Value, reference: &str, digest: &str) {
  println!();
  println!("declare it, pinned (verifier.janus.yaml, or an SDK's components option):");
  println!("  components:");
  println!("    - name: {}", config["name"].as_str().unwrap_or("?"));
  println!("      source: {{ kind: oci, reference: \"{reference}\", digest: \"{digest}\" }}");
}

/// An artifact that is not what it claims is the subject failing — an answer about it, exit 1. A
/// registry that could not be reached, or refused, is the command failing to run — exit 2.
fn refused(doing: &str, err: &ComponentError) -> ExitCode {
  eprintln!("janus component {doing}: {} ({})", err.message, err.code);
  match err.code.as_str() {
    "digest-mismatch" | "not-a-component" | "artifact-mismatch" => ExitCode::from(io::SUBJECT_FAILED),
    _ => ExitCode::from(io::COMMAND_FAILED),
  }
}
