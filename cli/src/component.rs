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
//!
//! They are also the only things that read `JANUS_OCI_USERNAME` and `JANUS_OCI_PASSWORD`: they have
//! no configuration to declare a credential in, and they are not runs. A run's credential is its
//! declaration's `source.auth` (ADR 0052), so when these commands were given one, the declaration
//! they print carries it, as the same two variables for the loader to interpolate.

use crate::Outcome;
use crate::args::{ComponentCommand, Output, PullArgs, PushArgs};
use crate::error::Error;
use pact_janus_component_host::WasmLoader;
use pact_janus_component_host::oci::Credentials;
use pact_janus_kernel::component::ComponentError;
use serde_json::{Value, json};

pub fn run(command: &ComponentCommand) -> Result<Outcome, Error> {
  let loader = WasmLoader::new().map_err(|err| err.to_string())?;
  let credentials = Credentials::from_env();
  match command {
    ComponentCommand::Push(args) => push(&loader, args, credentials.as_ref()),
    ComponentCommand::Pull(args) => pull(&loader, args, credentials.as_ref()),
  }
}

fn push(loader: &WasmLoader, args: &PushArgs, credentials: Option<&Credentials>) -> Result<Outcome, Error> {
  let wasm = std::fs::read(&args.component).map_err(|err| format!("{}: {err}", args.component.display()))?;
  let pushed = match loader.push(&wasm, &args.reference, credentials) {
    Ok(pushed) => pushed,
    Err(err) => return refused(err, credentials.is_some()),
  };
  match args.output {
    Output::Json => outln!(
      "{}",
      crate::output::pretty(&json!({
        "reference": pushed.reference,
        "digest": pushed.digest,
        "component": pushed.config,
        "interfaces": pushed.hello.get("interfaces").cloned().unwrap_or(Value::Null),
      }))
    ),
    Output::Text => {
      outln!("pushed {} to {}", describe(&pushed.hello), pushed.reference);
      outln!("digest: {}", pushed.digest);
      declaration(
        &pushed.config,
        &pushed.reference,
        &pushed.digest,
        credentials.is_some(),
      );
    }
  }
  Ok(Outcome::Done)
}

fn pull(loader: &WasmLoader, args: &PullArgs, credentials: Option<&Credentials>) -> Result<Outcome, Error> {
  let (pulled, hello) = match loader.pull(&args.reference, args.digest.as_deref(), credentials) {
    Ok(pulled) => pulled,
    Err(err) => return refused(err, credentials.is_some()),
  };
  match args.output {
    Output::Json => outln!(
      "{}",
      crate::output::pretty(&json!({
        "reference": pulled.reference.to_string(),
        "digest": pulled.digest,
        "component": pulled.config,
        "interfaces": hello.get("interfaces").cloned().unwrap_or(Value::Null),
        "contributes": hello.get("contributes").cloned().unwrap_or(Value::Null),
      }))
    ),
    Output::Text => {
      outln!("{} — every byte checked", describe(&hello));
      outln!("digest: {}", pulled.digest);
      if let Some(types) = hello
        .pointer("/contributes/content-types")
        .and_then(Value::as_array)
      {
        let types: Vec<&str> = types
          .iter()
          .filter_map(|entry| entry.get("media-type").and_then(Value::as_str))
          .collect();
        outln!("content types: {}", types.join(", "));
      }
      let mut unpinned = pulled.reference.clone();
      unpinned.digest = None;
      declaration(
        &pulled.config,
        &unpinned.to_string(),
        &pulled.digest,
        credentials.is_some(),
      );
    }
  }
  Ok(Outcome::Done)
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

/// With credentials, the declaration names the same two variables for the loader to interpolate, so a
/// run that pastes it reaches the registry as this command did (ADR 0052).
fn declaration(config: &Value, reference: &str, digest: &str, credentials: bool) {
  outln!();
  outln!("declare it, pinned (verifier.janus.yaml, or an SDK's components option):");
  outln!("  components:");
  outln!("    - name: {}", config["name"].as_str().unwrap_or("?"));
  let auth = if credentials {
    ", auth: { kind: basic, username: \"${JANUS_OCI_USERNAME}\", password: \"${JANUS_OCI_PASSWORD}\" }"
  } else {
    ""
  };
  outln!("      source: {{ kind: oci, reference: \"{reference}\", digest: \"{digest}\"{auth} }}");
}

/// An artifact that is not what it claims is the subject failing — an answer about it, exit 1. A
/// registry that could not be reached, or refused, is the command failing to run — exit 2.
fn refused(err: ComponentError, credentials: bool) -> Result<Outcome, Error> {
  let mut message = err.message.clone();
  if err.code == "unauthorized" && !credentials {
    message.push_str("; set JANUS_OCI_USERNAME and JANUS_OCI_PASSWORD");
  }
  let message = format!("{message} ({})", err.code);
  match err.code.as_str() {
    "digest-mismatch" | "not-a-component" | "artifact-mismatch" => Err(Error::Refused(message)),
    _ => Err(Error::Failed(message)),
  }
}
