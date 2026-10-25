//! Reading the documents the commands are handed.

use pact_janus_kernel::contract::{IdentifyMode, identify};
use serde_json::Value;
use std::path::Path;

/// Every contract or pact document at `path`: the file itself, or every `.json` file directly
/// inside a directory. Returned with the path each came from, so a message can name the file
/// rather than an index.
///
/// A directory is filtered by **identification, not by name** (ADR 0011): a `.json` file that is
/// neither a Janus contract nor a pact-shaped document is skipped, because a pact directory in the
/// wild also holds `.gitkeep`, editor backups and the odd config file. A file named *explicitly* is
/// never skipped — if a user points at it, they are entitled to the engine's own refusal by name.
pub fn read_documents(path: &Path) -> Result<Vec<(String, Value)>, String> {
  read_identified(path, "a contract or pact document", is_verifiable)
}

/// Every provider shape at `path`: the file itself, or every `.json` file directly inside a
/// directory that identifies itself as one (design 2.8 §2.4's `$format`). Filtered, and not
/// filtered, for the reasons [`read_documents`] gives.
pub fn read_provider_shapes(path: &Path) -> Result<Vec<(String, Value)>, String> {
  read_identified(path, "a provider shape", |_, document| {
    document["$format"]
      .as_str()
      .is_some_and(|format| pact_janus_kernel::persisted::PROVIDER_SHAPE_FORMAT.names_family(format))
  })
}

/// The file at `path`, or the `.json` files in the directory at `path` that `keep` accepts, in
/// name order so a run is the same run on every machine.
fn read_identified(
  path: &Path,
  what: &str,
  keep: impl Fn(&[u8], &Value) -> bool,
) -> Result<Vec<(String, Value)>, String> {
  if !path.is_dir() {
    return Ok(vec![(path.display().to_string(), read_json(path)?)]);
  }
  let mut entries: Vec<_> = std::fs::read_dir(path)
    .map_err(|err| format!("{}: {}", path.display(), io_error(&err)))?
    .filter_map(Result::ok)
    .map(|entry| entry.path())
    .filter(|entry| entry.extension().and_then(|e| e.to_str()) == Some("json"))
    .collect();
  entries.sort();
  let mut documents = Vec::new();
  for entry in entries {
    let name = entry.display().to_string();
    let bytes = std::fs::read(&entry).map_err(|err| format!("{name}: {}", io_error(&err)))?;
    match serde_json::from_slice::<Value>(&bytes) {
      Ok(document) if keep(&bytes, &document) => documents.push((name, document)),
      _ => tracing::debug!(file = %name, "not {what}; skipped"),
    }
  }
  Ok(documents)
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

/// A configuration document a person wrote: YAML or JSON, parsed to the same `Value` either way.
///
/// The same treatment the hook loader gives `verifier.janus.yaml` (lifecycle-hooks spec §6.1),
/// for the same reason: these documents carry hand-written prose — a hook's purpose, an
/// exemption's `reason` — and YAML is where prose is bearable.
pub fn read_config(path: &Path) -> Result<Value, String> {
  let text =
    std::fs::read_to_string(path).map_err(|err| format!("{}: {}", path.display(), io_error(&err)))?;
  serde_yaml_ng::from_str(&text).map_err(|err| format!("{}: {err}", path.display()))
}

pub fn read_json(path: &Path) -> Result<Value, String> {
  let text =
    std::fs::read_to_string(path).map_err(|err| format!("{}: {}", path.display(), io_error(&err)))?;
  serde_json::from_str(&text).map_err(|err| format!("{}: not JSON: {err}", path.display()))
}

/// An I/O error as a person reads it: `No such file or directory`, without the `(os error 2)` the
/// platform's own number adds.
pub fn io_error(err: &std::io::Error) -> String {
  let text = err.to_string();
  match text.rfind(" (os error ") {
    Some(at) if text.ends_with(')') => text[..at].to_string(),
    _ => text,
  }
}

/// The documents at every path, in the order given, failing when there are none at all: a run
/// over nothing would report success about nothing.
pub fn read_all_documents(paths: &[std::path::PathBuf]) -> Result<Vec<(String, Value)>, String> {
  let mut documents = Vec::new();
  for path in paths {
    documents.extend(read_documents(path)?);
  }
  if documents.is_empty() {
    let names: Vec<String> = paths.iter().map(|path| path.display().to_string()).collect();
    return Err(format!(
      "no contract or pact documents found in {}",
      names.join(", ")
    ));
  }
  Ok(documents)
}
