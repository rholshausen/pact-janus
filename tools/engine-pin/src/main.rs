//! `engine-pin` — writes the SDKs' engine pin (staged plan S0.10; ADR 0029 decision 5).
//!
//! `engine-pin <version> [--repo <owner/name>] [--out <file>]` fetches that release's `SHA256SUMS`,
//! has `gh attestation verify` check it was made by this repository's release workflow, and writes
//! the engine's digest for each target triple into `sdks/engine-release.json` (or `--out`). The
//! repository defaults to the one the current pin names. Nothing is written unless the attestation
//! verifies: the attested `SHA256SUMS` is what makes the digests in it trustworthy, and the pin is
//! what every resolver then trusts.
//!
//! Needs `gh`, signed in or given `GH_TOKEN`. Exit codes: 0 written, 2 could not.

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use pact_janus_engine_pin::{Pin, version};

const USAGE: &str = "usage: engine-pin <version> [--repo <owner/name>] [--out <file>]";

fn main() -> ExitCode {
  match run() {
    Ok(()) => ExitCode::SUCCESS,
    Err(error) => {
      eprintln!("engine-pin: {error}");
      ExitCode::from(2)
    }
  }
}

fn run() -> Result<(), String> {
  let mut given_version = None;
  let mut repository = None;
  let mut out = None;
  let mut arguments = std::env::args().skip(1);
  while let Some(argument) = arguments.next() {
    match argument.as_str() {
      "--repo" => repository = Some(arguments.next().ok_or(USAGE)?),
      "--out" => out = Some(PathBuf::from(arguments.next().ok_or(USAGE)?)),
      flag if flag.starts_with('-') => return Err(format!("unknown option {flag}; {USAGE}")),
      _ if given_version.is_none() => given_version = Some(argument),
      _ => return Err(USAGE.to_string()),
    }
  }
  let version = version(given_version.as_deref().ok_or(USAGE)?).to_string();
  let out = out.unwrap_or_else(|| repo_root().join("sdks").join("engine-release.json"));
  let repository = match repository {
    Some(repository) => repository,
    None => current_repository(&out)?,
  };
  let tag = format!("v{version}");

  let scratch = std::env::temp_dir().join(format!("engine-pin-{}", std::process::id()));
  std::fs::create_dir_all(&scratch).map_err(|e| format!("{}: {e}", scratch.display()))?;
  let result = pin(&repository, &version, &tag, &scratch);
  let _ = std::fs::remove_dir_all(&scratch);
  let pin = result?;

  std::fs::write(&out, pin.render()).map_err(|e| format!("{}: {e}", out.display()))?;
  println!(
    "✓ {}: janus-engine {version} from {repository}, {} triples",
    out.display(),
    pin.sha256.len()
  );
  for (asset, digest) in &pin.sha256 {
    println!("  {digest}  {asset}");
  }
  Ok(())
}

/// Fetch the release's `SHA256SUMS` into `scratch`, verify its attestation, and read the pin from it.
fn pin(repository: &str, version: &str, tag: &str, scratch: &Path) -> Result<Pin, String> {
  let dir = scratch.to_string_lossy();
  gh(&[
    "release",
    "download",
    tag,
    "--repo",
    repository,
    "--pattern",
    "SHA256SUMS",
    "--dir",
    &dir,
  ])?;
  let sums = scratch.join("SHA256SUMS");
  gh(&[
    "attestation",
    "verify",
    &sums.to_string_lossy(),
    "--repo",
    repository,
  ])?;
  let text = std::fs::read_to_string(&sums).map_err(|e| format!("{}: {e}", sums.display()))?;
  Pin::from_sums(repository, version, &text)
}

fn gh(arguments: &[&str]) -> Result<(), String> {
  let status = Command::new("gh")
    .args(arguments)
    .status()
    .map_err(|e| format!("gh could not be run ({e}); it is how a release is fetched and verified"))?;
  if status.success() {
    Ok(())
  } else {
    Err(format!("`gh {}` failed ({status})", arguments.join(" ")))
  }
}

/// The repository the pin at `path` already names.
fn current_repository(path: &Path) -> Result<String, String> {
  let text = std::fs::read_to_string(path)
    .map_err(|e| format!("{}: {e}; name the repository with --repo", path.display()))?;
  let pin: serde_json::Value = serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
  pin["repository"]
    .as_str()
    .map(str::to_string)
    .ok_or_else(|| format!("{} names no repository; name it with --repo", path.display()))
}

/// The repository root: this crate sits at `tools/engine-pin`, two levels below it.
fn repo_root() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .parent()
    .and_then(Path::parent)
    .map(Path::to_path_buf)
    .unwrap_or_else(|| PathBuf::from("."))
}
