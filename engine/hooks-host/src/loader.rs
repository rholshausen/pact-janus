//! The loader (lifecycle-hooks spec §7.1): the one transformation between the file a project
//! writes (`project-config.schema.json`, conventionally `verifier.janus.yaml`) and the document
//! the engine receives (`hook-config.schema.json`).
//!
//! Four steps, in this order, and the order matters — a path is resolved after interpolation
//! because `${HOOKS_DIR}/sign.js` is a path only once the variable is substituted:
//!
//! 1. **interpolate** every `${VAR}` from the environment;
//! 2. **inline** every script: read `run.path` and put its JavaScript in `run.source`;
//! 3. **resolve** relative paths (`cwd`, script paths) against the configuration file's directory;
//! 4. **validate** the result against the resolved schema.
//!
//! An unset variable **fails the load**, naming it — never an empty string, because the empty
//! string is how `${TOKEN}` becomes a run of 401s that look like a provider bug (§7.2).

use pact_janus_kernel::error::Problem;
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Why a configuration could not be turned into the document the engine reads. `Unresolved` is
/// spec §11's `hook-unresolved`, raised by the loader because the loader is where the file system
/// is; `Invalid` is `hook-config-invalid`, which the kernel's own validator produces so that a
/// configuration loaded from a file and one handed straight over the protocol are judged by
/// exactly the same code.
///
/// It says its own layer, and why a file or script could not be read is its `source`: the
/// `io::Error` or the parser's error itself, not its text (ADR 0054), so a host can show the cause
/// apart from the headline.
#[derive(Debug, thiserror::Error)]
pub enum LoadError {
  /// An unset `${VAR}`.
  #[error("environment variable '{variable}' is not set (used at {at})")]
  UnsetVariable { variable: String, at: String },
  /// A script that could not be read, or one this loader has no toolchain for.
  #[error("hook '{hook}': could not read script '{path}'")]
  UnreadableScript {
    path: String,
    hook: String,
    #[source]
    cause: Box<dyn std::error::Error + Send + Sync>,
  },
  /// The file itself could not be read or parsed.
  #[error("could not read '{path}'")]
  Unreadable {
    path: String,
    #[source]
    cause: Box<dyn std::error::Error + Send + Sync>,
  },
  #[error("hook configuration is not valid:{}", problems(.0))]
  Invalid(Vec<Problem>),
}

fn problems(problems: &[Problem]) -> String {
  problems
    .iter()
    .map(|problem| format!("\n  {}: {}", problem.pointer, problem.message))
    .collect()
}

/// Load and resolve a configuration file (YAML or JSON — the schema governs the parsed document,
/// not the syntax it arrived in). Returns the document to hand the engine.
pub fn load(path: impl AsRef<Path>) -> Result<Value, LoadError> {
  let path = path.as_ref();
  let text = std::fs::read_to_string(path).map_err(|err| LoadError::Unreadable {
    path: path.display().to_string(),
    cause: err.into(),
  })?;
  let base = path.parent().unwrap_or(Path::new("."));
  load_document(&text, base, &environment())
}

/// The same, from text already in hand — what a test uses, and what a host that reads its
/// configuration from somewhere other than a file uses.
pub fn load_document(text: &str, base: &Path, env: &HashMap<String, String>) -> Result<Value, LoadError> {
  let document: Value = serde_yaml_ng::from_str(text).map_err(|err| LoadError::Unreadable {
    path: base.display().to_string(),
    cause: err.into(),
  })?;
  resolve(&document, base, env)
}

/// Steps 1–4 on a parsed document.
pub fn resolve(document: &Value, base: &Path, env: &HashMap<String, String>) -> Result<Value, LoadError> {
  let interpolated = interpolate(document, env, "")?;
  let mut resolved = Map::new();
  if let Some(version) = interpolated.get("version") {
    resolved.insert("version".to_string(), version.clone());
  }

  let mut hooks = Map::new();
  if let Some(Value::Object(points)) = interpolated.get("hooks") {
    for (point, entries) in points {
      let Some(entries) = entries.as_array() else {
        return Err(LoadError::Invalid(vec![Problem {
          pointer: format!("/hooks/{point}"),
          message: "a point's hooks must be a list".to_string(),
        }]));
      };
      let mut resolved_entries = Vec::with_capacity(entries.len());
      for entry in entries {
        resolved_entries.push(resolve_entry(entry, base)?);
      }
      hooks.insert(point.clone(), Value::Array(resolved_entries));
    }
  }
  resolved.insert("hooks".to_string(), Value::Object(hooks));
  if let Some(components) = interpolated.get("components") {
    resolved.insert("components".to_string(), resolve_components(components, base)?);
  }
  let resolved = Value::Object(resolved);

  // Validated by the kernel's own validator, not a second copy of the rules here: a configuration
  // that came from a file and one handed straight over the protocol must be judged identically, or
  // the file form would grow its own dialect.
  pact_janus_kernel::hooks::config::parse(&resolved).map_err(LoadError::Invalid)?;
  Ok(resolved)
}

/// The project's `components` (component-interfaces spec §10.2), with step 3 applied: a `file`
/// source's path is relative to the configuration file, and the engine — which reads no
/// configuration — is handed it absolute. Each entry is checked against the kernel's own
/// declaration model, so a malformed one fails here, naming its position, rather than at `verify`.
fn resolve_components(components: &Value, base: &Path) -> Result<Value, LoadError> {
  let Some(entries) = components.as_array() else {
    return Err(LoadError::Invalid(vec![Problem {
      pointer: "/components".to_string(),
      message: "'components' must be a list".to_string(),
    }]));
  };
  let mut resolved = Vec::with_capacity(entries.len());
  for (index, entry) in entries.iter().enumerate() {
    let mut entry = entry.clone();
    if let Err(err) =
      serde_json::from_value::<pact_janus_kernel::component::ComponentDeclaration>(entry.clone())
    {
      return Err(LoadError::Invalid(vec![Problem {
        pointer: format!("/components/{index}"),
        message: err.to_string(),
      }]));
    }
    if entry.pointer("/source/kind").and_then(Value::as_str) == Some("file")
      && let Some(reference) = entry.pointer("/source/reference").and_then(Value::as_str)
    {
      let full = resolve_path(reference, base);
      entry["source"]["reference"] = Value::String(full.display().to_string());
    }
    resolved.push(entry);
  }
  Ok(Value::Array(resolved))
}

fn resolve_entry(entry: &Value, base: &Path) -> Result<Value, LoadError> {
  let mut entry = entry.as_object().cloned().unwrap_or_default();
  let hook = entry
    .get("name")
    .and_then(Value::as_str)
    .unwrap_or("<unnamed>")
    .to_string();

  if let Some(Value::Object(run)) = entry.get("run").cloned() {
    let mut run = run;
    // Step 2: inline the script. `path` is a file the engine would have to read, and the engine
    // reads no files — so the loader reads it and what crosses the boundary is source text.
    if let Some(path) = run.get("path").and_then(Value::as_str) {
      let full = resolve_path(path, base);
      if full.extension().is_some_and(|ext| ext == "ts") {
        return Err(LoadError::UnreadableScript {
          path: full.display().to_string(),
          hook,
          cause: "TypeScript hooks need a transpiler this loader does not carry (spec §9.6); \
                  transpile it to JavaScript and point 'path' at the result"
            .into(),
        });
      }
      let source = std::fs::read_to_string(&full).map_err(|err| LoadError::UnreadableScript {
        path: full.display().to_string(),
        hook: hook.clone(),
        cause: err.into(),
      })?;
      run.remove("path");
      run.insert("source".to_string(), Value::String(source));
    }
    // Step 3: a relative `cwd` is relative to the configuration file, not to wherever the run was
    // started from — the property that makes a run reproducible from a different directory.
    if let Some(cwd) = run.get("cwd").and_then(Value::as_str) {
      run.insert(
        "cwd".to_string(),
        Value::String(resolve_path(cwd, base).display().to_string()),
      );
    }
    if let Some(command) = run.get("command").and_then(Value::as_str)
      && (command.starts_with("./") || command.starts_with("../"))
    {
      run.insert(
        "command".to_string(),
        Value::String(resolve_path(command, base).display().to_string()),
      );
    }
    entry.insert("run".to_string(), Value::Object(run));
  }
  Ok(Value::Object(entry))
}

fn resolve_path(path: &str, base: &Path) -> PathBuf {
  let path = Path::new(path);
  if path.is_absolute() {
    path.to_path_buf()
  } else {
    base.join(path)
  }
}

/// Step 1 (spec §7.2). One form only: `${VAR}`, no defaults, no nesting, no shell expansion. `$$`
/// is a literal `$`. Interpolation happens once, on the authored document: a value that happens to
/// contain `${…}` after substitution is a value, not a template, and is not re-scanned.
fn interpolate(value: &Value, env: &HashMap<String, String>, at: &str) -> Result<Value, LoadError> {
  match value {
    Value::String(text) => Ok(Value::String(interpolate_text(text, env, at)?)),
    Value::Array(items) => {
      let mut out = Vec::with_capacity(items.len());
      for (i, item) in items.iter().enumerate() {
        out.push(interpolate(item, env, &format!("{at}/{i}"))?);
      }
      Ok(Value::Array(out))
    }
    Value::Object(members) => {
      let mut out = Map::new();
      for (key, member) in members {
        out.insert(key.clone(), interpolate(member, env, &format!("{at}/{key}"))?);
      }
      Ok(Value::Object(out))
    }
    other => Ok(other.clone()),
  }
}

fn interpolate_text(text: &str, env: &HashMap<String, String>, at: &str) -> Result<String, LoadError> {
  let mut out = String::with_capacity(text.len());
  let mut chars = text.chars().peekable();
  while let Some(ch) = chars.next() {
    if ch != '$' {
      out.push(ch);
      continue;
    }
    match chars.peek() {
      Some('$') => {
        chars.next();
        out.push('$');
      }
      Some('{') => {
        chars.next();
        let mut name = String::new();
        let mut closed = false;
        for ch in chars.by_ref() {
          if ch == '}' {
            closed = true;
            break;
          }
          name.push(ch);
        }
        if !closed {
          return Err(LoadError::Invalid(vec![Problem {
            pointer: at.to_string(),
            message: format!("unterminated '${{' in \"{text}\""),
          }]));
        }
        match env.get(&name) {
          Some(value) => out.push_str(value),
          None => {
            return Err(LoadError::UnsetVariable {
              variable: name,
              at: at.to_string(),
            });
          }
        }
      }
      _ => out.push('$'),
    }
  }
  Ok(out)
}

/// The process environment as the loader reads it. A function rather than a direct `std::env` call
/// at the use site so that a test can resolve a document against an environment it controls.
pub fn environment() -> HashMap<String, String> {
  std::env::vars().collect()
}

/// `config/load`'s loader (engine-protocol spec §8.8, ADR 0055): this module's [`load`], registered
/// into the engine so a host that is not the CLI resolves a configuration exactly as the CLI does.
/// The environment is this process's own, which is the host's when the host started it.
#[derive(Debug, Default, Clone, Copy)]
pub struct FileLoader;

impl pact_janus_kernel::hooks::ConfigLoader for FileLoader {
  fn load(&self, path: &str) -> Result<Value, pact_janus_kernel::protocol::EngineError> {
    use pact_janus_kernel::protocol::EngineError;
    if !Path::new(path).is_absolute() {
      return Err(EngineError::malformed_frame(format!(
        "config/load's 'path' must be absolute, and '{path}' is not"
      )));
    }
    load(path).map_err(|err| engine_error(&err))
  }
}

/// A load error as the protocol carries it (lifecycle-hooks spec §11): `hook-config-invalid` with
/// its problems, or `hook-unresolved` naming what could not be resolved, its causes as the chain.
pub fn engine_error(err: &LoadError) -> pact_janus_kernel::protocol::EngineError {
  use pact_janus_kernel::chain::{layers_of, without_os_error};
  use pact_janus_kernel::protocol::EngineError;
  use serde_json::json;
  let causes = |cause: &(dyn std::error::Error + Send + Sync)| layers_of(cause, without_os_error);
  match err {
    LoadError::Invalid(problems) => EngineError::hook_config_invalid(problems),
    LoadError::UnsetVariable { variable, at } => EngineError::hook_unresolved(
      err.to_string(),
      json!({ "variable": variable, "at": at }),
      Vec::new(),
    ),
    LoadError::UnreadableScript { path, hook, cause } => EngineError::hook_unresolved(
      err.to_string(),
      json!({ "path": path, "hook": hook }),
      causes(cause.as_ref()),
    ),
    LoadError::Unreadable { path, cause } => {
      EngineError::hook_unresolved(err.to_string(), json!({ "path": path }), causes(cause.as_ref()))
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json::json;

  fn env(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
      .iter()
      .map(|(k, v)| (k.to_string(), v.to_string()))
      .collect()
  }

  /// What the loader makes of a path relative to the configuration file: the platform's own join,
  /// so the separator is the one the process that receives it expects.
  fn joined(base: &str, relative: &str) -> String {
    Path::new(base).join(relative).display().to_string()
  }

  #[test]
  fn a_yaml_file_and_a_json_file_resolve_to_the_same_document() {
    let yaml = r#"
version: 1
hooks:
  before-request:
    - name: sign-requests
      run: { kind: exec, command: /usr/bin/sign }
      changes: ["parts.request.headers"]
"#;
    let json = r#"{ "version": 1, "hooks": { "before-request": [
      { "name": "sign-requests", "run": { "kind": "exec", "command": "/usr/bin/sign" },
        "changes": ["parts.request.headers"] } ] } }"#;
    let from_yaml = load_document(yaml, Path::new("."), &env(&[])).expect("yaml loads");
    let from_json = load_document(json, Path::new("."), &env(&[])).expect("json loads");
    assert_eq!(
      from_yaml, from_json,
      "the schema governs the document, not the syntax"
    );
  }

  #[test]
  fn every_variable_reference_is_substituted_wherever_it_appears() {
    let document = json!({
      "hooks": { "state-setup": [
        { "name": "fixtures",
          "run": { "kind": "http", "url": "${PROVIDER_URL}/_pact/state",
                   "headers": { "authorization": "Bearer ${TOKEN}" } },
          "config": { "tenant": "${TENANT}" } } ] }
    });
    let resolved = resolve(
      &document,
      Path::new("."),
      &env(&[
        ("PROVIDER_URL", "http://localhost:8080"),
        ("TOKEN", "t-1"),
        ("TENANT", "acme"),
      ]),
    )
    .expect("resolves");
    let entry = &resolved["hooks"]["state-setup"][0];
    assert_eq!(entry["run"]["url"], json!("http://localhost:8080/_pact/state"));
    assert_eq!(entry["run"]["headers"]["authorization"], json!("Bearer t-1"));
    assert_eq!(entry["config"]["tenant"], json!("acme"));
  }

  #[test]
  fn an_unset_variable_fails_the_load_naming_it() {
    let document = json!({
      "hooks": { "before-request": [
        { "name": "sign", "run": { "kind": "http", "url": "${AUTH_URL}/token" } } ] }
    });
    let err = resolve(&document, Path::new("."), &env(&[])).expect_err("no AUTH_URL");
    match err {
      LoadError::UnsetVariable { variable, at } => {
        assert_eq!(variable, "AUTH_URL");
        assert!(at.contains("url"), "the message locates it: {at}");
      }
      other => panic!("expected an unset variable, got {other:?}"),
    }
  }

  #[test]
  fn an_empty_string_is_never_substituted_for_a_missing_variable() {
    // The whole reason §7.2 refuses defaults: an empty token is a run of 401s that look like a
    // provider bug, discovered an hour later by someone reading the wrong logs.
    let document = json!({
      "hooks": { "before-request": [
        { "name": "sign", "run": { "kind": "http", "url": "http://x", "headers": { "authorization": "Bearer ${TOKEN}" } } } ] }
    });
    assert!(resolve(&document, Path::new("."), &env(&[])).is_err());
  }

  #[test]
  fn a_literal_dollar_is_written_twice_and_survives() {
    let document = json!({
      "hooks": { "before-request": [
        { "name": "sign", "run": { "kind": "exec", "command": "/bin/sign" },
          "config": { "template": "$${not-a-variable}" } } ] }
    });
    let resolved = resolve(&document, Path::new("."), &env(&[])).expect("resolves");
    assert_eq!(
      resolved["hooks"]["before-request"][0]["config"]["template"],
      json!("${not-a-variable}")
    );
  }

  #[test]
  fn a_substituted_value_that_looks_like_a_template_is_a_value() {
    let document = json!({
      "hooks": { "before-request": [
        { "name": "sign", "run": { "kind": "exec", "command": "/bin/sign" },
          "config": { "secret": "${SECRET}" } } ] }
    });
    let resolved =
      resolve(&document, Path::new("."), &env(&[("SECRET", "${NOT_RESCANNED}")])).expect("resolves");
    assert_eq!(
      resolved["hooks"]["before-request"][0]["config"]["secret"],
      json!("${NOT_RESCANNED}"),
      "interpolation happens once, on the authored document"
    );
  }

  #[test]
  fn a_script_is_inlined_and_its_path_does_not_reach_the_engine() {
    let dir = std::env::temp_dir().join(format!("janus-loader-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
      dir.join("sign.js"),
      "function hook(ctx) { return { outcome: 'ok' }; }",
    )
    .unwrap();

    let document = json!({
      "hooks": { "before-request": [
        { "name": "sign", "run": { "kind": "script", "path": "./sign.js" } } ] }
    });
    let resolved = resolve(&document, &dir, &env(&[])).expect("resolves");
    let run = &resolved["hooks"]["before-request"][0]["run"];
    assert!(run["source"].as_str().unwrap().contains("function hook"));
    assert!(
      run.get("path").is_none(),
      "the engine reads no files, so no path crosses the boundary: {run}"
    );
    std::fs::remove_dir_all(&dir).ok();
  }

  #[test]
  fn a_script_path_that_does_not_exist_fails_the_load_naming_the_hook() {
    let document = json!({
      "hooks": { "before-request": [
        { "name": "sign", "run": { "kind": "script", "path": "./nope.js" } } ] }
    });
    let err = resolve(&document, Path::new("/tmp"), &env(&[])).expect_err("no such file");
    match err {
      LoadError::UnreadableScript { hook, path, .. } => {
        assert_eq!(hook, "sign");
        assert!(path.ends_with("nope.js"));
      }
      other => panic!("expected an unreadable script, got {other:?}"),
    }
  }

  #[test]
  fn a_relative_cwd_resolves_against_the_configuration_file_not_the_working_directory() {
    let document = json!({
      "hooks": { "state-setup": [
        { "name": "fixtures", "run": { "kind": "exec", "command": "make", "cwd": "./fixtures" } } ] }
    });
    let resolved = resolve(&document, Path::new("/projects/orders"), &env(&[])).expect("resolves");
    assert_eq!(
      resolved["hooks"]["state-setup"][0]["run"]["cwd"],
      json!(joined("/projects/orders", "./fixtures"))
    );
  }

  #[test]
  fn a_file_components_path_resolves_against_the_configuration_file_and_nothing_else_changes() {
    let document = json!({
      "components": [
        { "name": "csv", "source": { "kind": "file", "reference": "components/csv.wasm", "digest": "sha256:ab" },
          "grants": { "network": false }, "limits": { "deadline-ms": 500 } },
        { "name": "xml", "source": { "kind": "oci", "reference": "ghcr.io/acme/janus-xml:1" } } ]
    });
    let resolved = resolve(&document, Path::new("/projects/orders"), &env(&[])).expect("resolves");
    assert_eq!(
      resolved["components"],
      json!([
        { "name": "csv", "source": { "kind": "file", "reference": joined("/projects/orders", "components/csv.wasm"), "digest": "sha256:ab" },
          "grants": { "network": false }, "limits": { "deadline-ms": 500 } },
        { "name": "xml", "source": { "kind": "oci", "reference": "ghcr.io/acme/janus-xml:1" } } ])
    );
  }

  /// ADR 0052: a component's registry credential is declared, and interpolated like any other
  /// member, so a run's credential is in its configuration and not the engine's environment. One
  /// that is malformed fails the load, naming where, without repeating the interpolated secret.
  #[test]
  fn a_components_registry_credential_is_interpolated_and_a_malformed_one_never_repeated() {
    let declared = |auth: Value| {
      json!({ "components": [
        { "name": "csv", "source": { "kind": "oci", "reference": "ghcr.io/acme/csv:1", "auth": auth } } ] })
    };
    let secrets = env(&[("REGISTRY_USER", "bob"), ("REGISTRY_PASSWORD", "s3cret")]);
    let resolved = resolve(
      &declared(
        json!({ "kind": "basic", "username": "${REGISTRY_USER}", "password": "${REGISTRY_PASSWORD}" }),
      ),
      Path::new("."),
      &secrets,
    )
    .expect("resolves");
    assert_eq!(
      resolved["components"][0]["source"]["auth"],
      json!({ "kind": "basic", "username": "bob", "password": "s3cret" })
    );

    match resolve(&declared(json!("${REGISTRY_PASSWORD}")), Path::new("."), &secrets).expect_err("refused") {
      LoadError::Invalid(problems) => {
        assert_eq!(problems[0].pointer, "/components/0");
        assert!(!problems[0].message.contains("s3cret"), "{}", problems[0].message);
      }
      other => panic!("expected an invalid document, got {other:?}"),
    }
  }

  #[test]
  fn a_component_declaration_the_kernel_cannot_read_fails_the_load_naming_it() {
    let document = json!({ "components": [ { "name": "csv" } ] });
    match resolve(&document, Path::new("."), &env(&[])).expect_err("no source") {
      LoadError::Invalid(problems) => assert_eq!(problems[0].pointer, "/components/0"),
      other => panic!("expected an invalid document, got {other:?}"),
    }
  }

  #[test]
  fn the_kernels_own_validator_judges_the_result() {
    let document = json!({
      "hooks": { "after-response": [
        { "name": "tidy", "run": { "kind": "exec", "command": "/bin/tidy" },
          "changes": ["parts.response.headers"] } ] }
    });
    let err = resolve(&document, Path::new("."), &env(&[])).expect_err("after-response mutates nothing");
    match err {
      LoadError::Invalid(problems) => {
        assert_eq!(problems[0].pointer, "/hooks/after-response/0/changes/0")
      }
      other => panic!("expected an invalid document, got {other:?}"),
    }
  }

  #[test]
  fn the_file_loader_refuses_a_relative_path() {
    use pact_janus_kernel::hooks::ConfigLoader;
    let err = FileLoader
      .load("verifier.janus.yaml")
      .expect_err("a relative path");
    assert_eq!(err.code, "malformed-frame");
  }

  #[test]
  fn an_unset_variable_is_hook_unresolved_naming_it() {
    let err = load_document(
      "version: 1\nhooks:\n  state-setup:\n    - name: s\n      run: { kind: http, url: \"${STATE_URL}\" }\n",
      Path::new("/project"),
      &env(&[]),
    )
    .expect_err("STATE_URL is unset");
    let error = engine_error(&err);
    assert_eq!(error.code, "hook-unresolved");
    assert_eq!(error.category, "document");
    assert_eq!(
      error.details.as_deref().and_then(|d| d.get("variable")),
      Some(&json!("STATE_URL"))
    );
  }

  #[test]
  fn a_file_that_cannot_be_read_is_hook_unresolved_with_its_cause() {
    use pact_janus_kernel::hooks::ConfigLoader;
    let missing = std::env::temp_dir()
      .join("janus-no-such-dir")
      .join("verifier.janus.yaml");
    let err = FileLoader
      .load(&missing.display().to_string())
      .expect_err("no such file");
    assert_eq!(err.code, "hook-unresolved");
    assert_eq!(
      err.details.as_deref().and_then(|d| d.get("path")),
      Some(&json!(missing.display().to_string()))
    );
    assert!(err.chain.as_ref().is_some_and(|chain| chain.len() > 1), "{err:?}");
  }

  #[test]
  fn an_invalid_document_is_hook_config_invalid() {
    let err = load_document(
      "version: 1\nhooks:\n  no-such-point: []\n",
      Path::new("/project"),
      &env(&[]),
    )
    .expect_err("no such point");
    assert_eq!(engine_error(&err).code, "hook-config-invalid");
  }
}
