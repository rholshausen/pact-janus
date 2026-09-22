//! Reading documents and failing usefully — the two things every subcommand does.

use pact_janus_kernel::contract::{IdentifyMode, identify};
use serde_json::Value;
use std::path::Path;
use std::process::ExitCode;

/// Exit codes, fixed here so a CI script can depend on them: `0` the command did what it was
/// asked, `1` the *subject* failed (a verification found mismatches — an answer, not an error),
/// `2` the command could not run at all. Keeping the middle one distinct is the same distinction
/// protocol spec §10.2 draws: "a verification that ran and found mismatches is a successful
/// operation".
pub const SUBJECT_FAILED: u8 = 1;
pub const COMMAND_FAILED: u8 = 2;

pub fn fail(message: &str) -> ExitCode {
  eprintln!("{message}");
  ExitCode::from(COMMAND_FAILED)
}

pub fn usage(command: &str, problem: &str, usage: &str) -> ExitCode {
  eprintln!("janus {command}: {problem}");
  eprintln!();
  eprintln!("{usage}");
  ExitCode::from(COMMAND_FAILED)
}

/// Every contract or pact document at `path`: the file itself, or every `.json` file directly
/// inside a directory. Returned with the path each came from, so a message can name the file
/// rather than an index.
///
/// A directory is filtered by **identification, not by name** (ADR 0011): a `.json` file that is
/// neither a Janus contract nor a pact-shaped document is skipped with a warning rather than
/// handed to the engine, because a pact directory in the wild also holds `.gitkeep`, editor
/// backups and the odd config file. A file named *explicitly* is never skipped — if a user points
/// at it, they are entitled to the engine's own refusal by name.
pub fn read_documents(path: &str) -> Result<Vec<(String, Value)>, String> {
  let path = Path::new(path);
  if path.is_dir() {
    let mut documents = Vec::new();
    let mut entries: Vec<_> = std::fs::read_dir(path)
      .map_err(|err| format!("{}: {err}", path.display()))?
      .filter_map(Result::ok)
      .map(|entry| entry.path())
      .filter(|entry| entry.extension().and_then(|e| e.to_str()) == Some("json"))
      .collect();
    entries.sort();
    for entry in entries {
      let name = entry.display().to_string();
      let bytes = std::fs::read(&entry).map_err(|err| format!("{name}: {err}"))?;
      match serde_json::from_slice::<Value>(&bytes) {
        Ok(document) if is_verifiable(&bytes, &document) => documents.push((name, document)),
        _ => tracing::debug!(file = %name, "not a contract or pact document; skipped"),
      }
    }
    return Ok(documents);
  }
  let name = path.display().to_string();
  let bytes = std::fs::read(path).map_err(|err| format!("{name}: {err}"))?;
  let document = serde_json::from_slice(&bytes).map_err(|err| format!("{name}: {err}"))?;
  Ok(vec![(name, document)])
}

/// Whether a document in a directory is worth handing to the engine: it says it is a Janus
/// contract, or it carries a pact file's own required members.
fn is_verifiable(bytes: &[u8], document: &Value) -> bool {
  if identify(bytes, IdentifyMode::Tolerant) {
    return true;
  }
  let has = |name: &str| document.get(name).is_some_and(Value::is_object);
  has("consumer")
    && has("provider")
    && (document.get("interactions").is_some_and(Value::is_array)
      || document.get("messages").is_some_and(Value::is_array))
}

/// Every provider shape at `path`: the file itself, or every `.json` file directly inside a
/// directory that identifies itself as one (design 2.8 §2.4's `$format`).
///
/// Filtered by identification rather than by name, for the reason [`read_documents`] gives, and
/// with the same exception: a file named explicitly is never skipped, so a user who points at the
/// wrong file gets the engine's refusal by name instead of silence.
pub fn read_provider_shapes(path: &str) -> Result<Vec<(String, Value)>, String> {
  let path = Path::new(path);
  if !path.is_dir() {
    let name = path.display().to_string();
    let document = read_json(&name)?;
    return Ok(vec![(name, document)]);
  }
  let mut shapes = Vec::new();
  let mut entries: Vec<_> = std::fs::read_dir(path)
    .map_err(|err| format!("{}: {err}", path.display()))?
    .filter_map(Result::ok)
    .map(|entry| entry.path())
    .filter(|entry| entry.extension().and_then(|e| e.to_str()) == Some("json"))
    .collect();
  entries.sort();
  for entry in entries {
    let name = entry.display().to_string();
    let bytes = std::fs::read(&entry).map_err(|err| format!("{name}: {err}"))?;
    match serde_json::from_slice::<Value>(&bytes) {
      Ok(document)
        if document["$format"]
          .as_str()
          .is_some_and(|format| format.starts_with("janus-provider-shape/")) =>
      {
        shapes.push((name, document))
      }
      _ => tracing::debug!(file = %name, "not a provider shape; skipped"),
    }
  }
  Ok(shapes)
}

/// A configuration document a person wrote: YAML or JSON, parsed to the same `Value` either way.
///
/// The same treatment the hook loader gives `verifier.janus.yaml` (lifecycle-hooks spec §6.1),
/// for the same reason: these documents carry hand-written prose — a hook's purpose, an
/// exemption's `reason` — and YAML is where prose is bearable. Nothing below this function knows
/// which form the file was in.
pub fn read_config(path: &str) -> Result<Value, String> {
  let text = std::fs::read_to_string(path).map_err(|err| format!("{path}: {err}"))?;
  serde_yaml_ng::from_str(&text).map_err(|err| format!("{path}: {err}"))
}

pub fn read_json(path: &str) -> Result<Value, String> {
  let text = std::fs::read_to_string(path).map_err(|err| format!("{path}: {err}"))?;
  serde_json::from_str(&text).map_err(|err| format!("{path}: {err}"))
}
