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
#[derive(Debug)]
pub enum LoadError {
  /// An unset `${VAR}`.
  UnsetVariable {
    variable: String,
    at: String,
  },
  /// A script that could not be read, or one this loader has no toolchain for.
  UnreadableScript {
    path: String,
    hook: String,
    message: String,
  },
  /// The file itself could not be read or parsed.
  Unreadable {
    path: String,
    message: String,
  },
  Invalid(Vec<Problem>),
}

impl std::fmt::Display for LoadError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      LoadError::UnsetVariable { variable, at } => {
        write!(f, "environment variable '{variable}' is not set (used at {at})")
      }
      LoadError::UnreadableScript { path, hook, message } => {
        write!(f, "hook '{hook}': could not read script '{path}': {message}")
      }
      LoadError::Unreadable { path, message } => write!(f, "could not read '{path}': {message}"),
      LoadError::Invalid(problems) => {
        write!(f, "hook configuration is not valid:")?;
        for problem in problems {
          write!(f, "\n  {}: {}", problem.pointer, problem.message)?;
        }
        Ok(())
      }
    }
  }
}

impl std::error::Error for LoadError {}

/// Load and resolve a configuration file (YAML or JSON — the schema governs the parsed document,
/// not the syntax it arrived in). Returns the document to hand the engine.
pub fn load(path: impl AsRef<Path>) -> Result<Value, LoadError> {
  let path = path.as_ref();
  let text = std::fs::read_to_string(path).map_err(|err| LoadError::Unreadable {
    path: path.display().to_string(),
    message: err.to_string(),
  })?;
  let base = path.parent().unwrap_or(Path::new("."));
  load_document(&text, base, &environment())
}

/// The same, from text already in hand — what a test uses, and what a host that reads its
/// configuration from somewhere other than a file uses.
pub fn load_document(text: &str, base: &Path, env: &HashMap<String, String>) -> Result<Value, LoadError> {
  let document: Value = serde_yaml_ng::from_str(text).map_err(|err| LoadError::Unreadable {
    path: base.display().to_string(),
    message: err.to_string(),
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
  let resolved = Value::Object(resolved);

  // Validated by the kernel's own validator, not a second copy of the rules here: a configuration
  // that came from a file and one handed straight over the protocol must be judged identically, or
  // the file form would grow its own dialect.
  pact_janus_kernel::hooks::config::parse(&resolved).map_err(LoadError::Invalid)?;
  Ok(resolved)
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
          message: "TypeScript hooks need a transpiler this loader does not carry (spec §9.6); \
                    transpile it to JavaScript and point 'path' at the result"
            .to_string(),
        });
      }
      let source = std::fs::read_to_string(&full).map_err(|err| LoadError::UnreadableScript {
        path: full.display().to_string(),
        hook: hook.clone(),
        message: err.to_string(),
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
      json!("/projects/orders/./fixtures")
    );
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
}
