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
use crate::style::{self, paint};
use pact_janus_component_host::WasmLoader;
use pact_janus_component_host::oci::Credentials;
use pact_janus_component_host::signature::Trust;
use pact_janus_kernel::component::{ComponentError, Signer};
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
  let wasm = std::fs::read(&args.component)
    .map_err(|err| format!("{}: {}", args.component.display(), crate::io::io_error(&err)))?;
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
      outln!(
        "{} {} to {}",
        paint(style::PASS, "pushed"),
        describe(&pushed.hello),
        pushed.reference
      );
      outln!("{} {}", paint(style::DIM, "digest:"), pushed.digest);
      declaration(
        &pushed.config,
        &pushed.reference,
        &pushed.digest,
        credentials.is_some(),
        None,
      );
    }
  }
  Ok(Outcome::Done)
}

fn pull(loader: &WasmLoader, args: &PullArgs, credentials: Option<&Credentials>) -> Result<Outcome, Error> {
  let trusted_root = args.trusted_root.as_ref().map(|path| path.display().to_string());
  let trust = Trust::new(
    args.certificate_identity.as_deref(),
    args.certificate_identity_regexp.as_deref(),
    args.certificate_oidc_issuer.as_deref(),
    trusted_root.as_deref(),
  )
  .map_err(Error::Failed)?;
  let (pulled, hello, signers) =
    match loader.pull(&args.reference, args.digest.as_deref(), credentials, &trust) {
      Ok(pulled) => pulled,
      Err(err) => return refused(err, credentials.is_some()),
    };
  // Who answers, from the handshake: a Janus artifact's config says the same (the loader checked),
  // and one in the CNCF Wasm layout, as `wkg` publishes, says no name at all (ADR 0098).
  let identity = json!({ "name": hello["component"]["name"], "version": hello["component"]["version"] });
  match args.output {
    Output::Json => outln!(
      "{}",
      crate::output::pretty(&json!({
        "reference": pulled.reference.to_string(),
        "digest": pulled.digest,
        "component": identity,
        "interfaces": hello.get("interfaces").cloned().unwrap_or(Value::Null),
        "contributes": hello.get("contributes").cloned().unwrap_or(Value::Null),
        "signers": signers,
      }))
    ),
    Output::Text => {
      outln!(
        "{} — {}",
        describe(&hello),
        paint(style::PASS, "every byte checked")
      );
      outln!("{} {}", paint(style::DIM, "digest:"), pulled.digest);
      if let Some(types) = hello
        .pointer("/contributes/content-types")
        .and_then(Value::as_array)
      {
        let types: Vec<&str> = types
          .iter()
          .filter_map(|entry| entry.get("media-type").and_then(Value::as_str))
          .collect();
        outln!("{} {}", paint(style::DIM, "content types:"), types.join(", "));
      }
      // ADR 0099: who signed it, verified — or, with no signer named, everyone whose signature
      // verifies, any of whom a declaration may name.
      if signers.is_empty() {
        outln!(
          "{} {}",
          paint(style::DIM, "signed by:"),
          "no signature that verifies"
        );
      }
      for signer in &signers {
        outln!(
          "{} {} {}",
          paint(style::DIM, "signed by:"),
          signer.identity,
          paint(style::DIM, format!("(issuer {})", signer.issuer))
        );
      }
      let signature = match signers.as_slice() {
        [signer] => Some(SignatureLine {
          signer,
          trusted_root: trusted_root.as_deref(),
        }),
        _ => None,
      };
      declaration(
        &identity,
        &pulled.reference.without_digest().to_string(),
        &pulled.digest,
        credentials.is_some(),
        signature,
      );
    }
  }
  Ok(Outcome::Done)
}

/// The signature a printed declaration names: the one signer that verified, and the trusted root it
/// verified against when that was not the public-good instance's.
struct SignatureLine<'a> {
  signer: &'a Signer,
  trusted_root: Option<&'a str>,
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
fn declaration(
  config: &Value,
  reference: &str,
  digest: &str,
  credentials: bool,
  signature: Option<SignatureLine<'_>>,
) {
  outln!();
  outln!(
    "{}",
    paint(
      style::DIM,
      "declare it, pinned (verifier.janus.yaml, or an SDK's components option):"
    )
  );
  outln!("  components:");
  outln!("    - name: {}", config["name"].as_str().unwrap_or("?"));
  let auth = if credentials {
    ", auth: { kind: basic, username: \"${JANUS_OCI_USERNAME}\", password: \"${JANUS_OCI_PASSWORD}\" }"
  } else {
    ""
  };
  let Some(SignatureLine { signer, trusted_root }) = signature else {
    outln!("      source: {{ kind: oci, reference: \"{reference}\", digest: \"{digest}\"{auth} }}");
    return;
  };
  let root = trusted_root
    .map(|root| format!(", trusted-root: {}", serde_json::Value::from(root)))
    .unwrap_or_default();
  outln!("      source:");
  outln!("        kind: oci");
  outln!("        reference: \"{reference}\"");
  outln!("        digest: \"{digest}\"");
  if let Some(auth) = auth.strip_prefix(", ") {
    outln!("        {auth}");
  }
  outln!(
    "        signature: {{ identity: {}, issuer: {}{root} }}",
    serde_json::Value::from(signer.identity.as_str()),
    serde_json::Value::from(signer.issuer.as_str())
  );
}

/// An artifact that is not what it claims is the subject failing — an answer about it, exit 1. A
/// registry that could not be reached, or refused, is the command failing to run — exit 2.
fn refused(err: ComponentError, credentials: bool) -> Result<Outcome, Error> {
  let mut message = err.message.clone();
  if err.code == "unauthorized" && !credentials {
    message.push_str("; set JANUS_OCI_USERNAME and JANUS_OCI_PASSWORD");
  }
  // Code first, as every other error the CLI prints is.
  let message = format!("{}: {message}", err.code);
  match err.code.as_str() {
    "digest-mismatch"
    | "not-a-component"
    | "artifact-mismatch"
    | "signature-missing"
    | "signature-untrusted" => Err(Error::Refused(message)),
    _ => Err(Error::Failed(message)),
  }
}
