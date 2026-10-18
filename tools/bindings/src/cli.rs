//! `bindings [--spec <dir>] generate [--only typescript|jvm [--sdk <dir>]]` — regenerates the SDKs'
//! protocol bindings from the spec schemas (plan task 6.1): the one command AGENTS.md's "regenerate
//! them via the binding pipeline" means. Shared by this crate's own binary and `janus-sdk-gates
//! bindings` (staged plan S0.11).
//!
//! `--spec` is where `sdks/bindings.json` and the schema sets it names are: this repository's root,
//! or an unpacked `janus-spec.tar.gz` from a release, whose paths are the same. It defaults to the
//! working directory, and staging is written under it. Each SDK is `sdks/<language>` under `--spec`
//! unless `--sdk` names it, which is how an SDK checked out on its own regenerates its bindings from
//! a release.
//!
//! 1. Normalise each schema set `sdks/bindings.json` names into `target/bindings/schemas/<set>/`,
//!    one standalone schema per type; stage each generator's reading of it under
//!    `target/bindings/{typescript,jvm}/<set>/`; and write `target/bindings/sets.json` describing
//!    them. Every change made to a schema on its way to a generator happens here, in tested Rust —
//!    the generator steps below only invoke their generator.
//! 2. Clear each language's generated tree and run its generator over the normalised schemas:
//!    `npm run generate:bindings` in the TypeScript SDK (json-schema-to-typescript), and
//!    `./gradlew :bindings:generateBindings` in the JVM SDK (jsonschema2pojo).
//! 3. Render each set's open vocabularies next to its generated types.
//!
//! `bindings stage` does step 1 alone. CI runs `generate` and then fails on any diff under the
//! SDKs: checked-in bindings that do not match their schemas are a second source of truth.

use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

/// The bindings manifest, and where staging goes, under `--spec`.
pub const MANIFEST: &str = "sdks/bindings.json";
const STAGING: &str = "target/bindings";

const USAGE: &str = "usage: bindings [--spec <dir>] generate [--only typescript|jvm [--sdk <dir>]]\n       bindings [--spec <dir>] stage";

use crate::{
  Manifest, NamedSchema, Vocabulary, normalise, prepare_for_jvm, prepare_for_typescript,
  render_java_vocabularies, render_typescript_vocabularies, vocabularies,
};
/// Where each SDK keeps its generated bindings, relative to the SDK.
const TYPESCRIPT_OUT: &str = "src/generated";
const JVM_OUT: &str = "bindings/src/main/java";

struct Staged {
  set: crate::SchemaSet,
  vocabularies: Vec<Vocabulary>,
}

fn load_set(dir: &Path) -> Result<Vec<(String, Value)>, String> {
  let mut files = Vec::new();
  for entry in std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))? {
    let path = entry.map_err(|e| e.to_string())?.path();
    let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
    if name.ends_with(".schema.json") {
      let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
      files.push((
        name,
        serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?,
      ));
    }
  }
  files.sort_by(|a, b| a.0.cmp(&b.0));
  Ok(files)
}

fn write(path: &Path, contents: &str) -> Result<(), String> {
  if let Some(parent) = path.parent() {
    std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
  }
  std::fs::write(path, contents).map_err(|e| format!("{}: {e}", path.display()))
}

fn write_schema(dir: &Path, schema: &NamedSchema) -> Result<(), String> {
  let text = serde_json::to_string_pretty(&schema.schema).map_err(|e| e.to_string())?;
  write(&dir.join(schema.file_name()), &(text + "\n"))
}

fn reset(dir: &Path) -> Result<(), String> {
  if dir.exists() {
    std::fs::remove_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
  }
  std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))
}

fn stage(root: &Path) -> Result<Vec<Staged>, String> {
  let manifest_path = root.join(MANIFEST);
  let manifest: Manifest = serde_json::from_str(
    &std::fs::read_to_string(&manifest_path).map_err(|e| format!("{}: {e}", manifest_path.display()))?,
  )
  .map_err(|e| format!("{}: {e}", manifest_path.display()))?;

  let staging = root.join(STAGING);
  reset(&staging)?;
  let mut staged = Vec::new();
  let mut described = Vec::new();
  for set in manifest.sets {
    let files = load_set(&root.join(&set.schemas))?;
    let schemas: Vec<NamedSchema> =
      normalise(&files).map_err(|e| format!("{}:\n  {}", set.name, e.join("\n  ")))?;
    let vocabularies = vocabularies(&schemas).map_err(|e| format!("{}:\n  {}", set.name, e.join("\n  ")))?;
    // The normalised set, then each generator's reading of it (see `prepare_for_*`).
    let shared = staging.join("schemas").join(&set.name);
    let typescript = staging.join("typescript").join(&set.name);
    let jvm = staging.join("jvm").join(&set.name);
    for schema in &schemas {
      write_schema(&shared, schema)?;
      write_schema(&typescript, &prepare_for_typescript(schema))?;
      write_schema(&jvm, &prepare_for_jvm(schema, &set.jvm))?;
    }
    described.push(json!({
      "name": set.name,
      "schemas": { "typescript": typescript, "jvm": jvm },
      "types": schemas.iter().map(|s| &s.name).collect::<Vec<_>>(),
      "typescript": set.typescript,
      "jvm": set.jvm,
    }));
    staged.push(Staged { set, vocabularies });
  }
  write(
    &staging.join("sets.json"),
    &serde_json::to_string_pretty(&json!({ "header": header_comment(), "sets": described }))
      .map_err(|e| e.to_string())?,
  )?;
  Ok(staged)
}

/// `GENERATED_HEADER` as a block comment, which both TypeScript and Java read.
fn header_comment() -> String {
  let mut out = String::from("/*\n");
  for line in crate::GENERATED_HEADER.lines() {
    out.push_str(&format!(" * {line}\n"));
  }
  out.push_str(" */\n");
  out
}

/// jsonschema2pojo has no header option, so every Java file gets its header here.
fn add_java_headers(dir: &Path) -> Result<(), String> {
  for entry in std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))? {
    let path = entry.map_err(|e| e.to_string())?.path();
    if path.is_dir() {
      add_java_headers(&path)?;
    } else if path.extension().is_some_and(|e| e == "java") {
      let source = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
      write(&path, &format!("{}\n{source}", header_comment()))?;
    }
  }
  Ok(())
}

fn absolute(path: &Path) -> Result<PathBuf, String> {
  std::path::absolute(path).map_err(|e| format!("{}: {e}", path.display()))
}

fn invoke(command: &mut Command) -> Result<(), String> {
  let status = command.status().map_err(|e| format!("{command:?}: {e}"))?;
  if status.success() {
    Ok(())
  } else {
    Err(format!("{command:?} failed: {status}"))
  }
}

fn generate_typescript(spec: &Path, sdk: &Path, staged: &[Staged]) -> Result<(), String> {
  let out = sdk.join(TYPESCRIPT_OUT);
  reset(&out)?;
  let npm = if cfg!(windows) { "npm.cmd" } else { "npm" };
  invoke(
    Command::new(npm)
      .current_dir(sdk)
      .args(["run", "--silent", "generate:bindings", "--"])
      .arg(spec.join(STAGING).join("sets.json"))
      .arg(&out),
  )?;
  let mut index = header_comment();
  for s in staged {
    let module = &s.set.typescript;
    write(
      &out.join(format!("{module}.vocabulary.ts")),
      &render_typescript_vocabularies(&s.vocabularies)?,
    )?;
    index.push_str(&format!(
      "export * as {module} from \"./{module}.js\";\nexport * as {module}Vocabulary from \"./{module}.vocabulary.js\";\n"
    ));
  }
  write(&out.join("index.ts"), &index)
}

fn generate_jvm(spec: &Path, sdk: &Path, staged: &[Staged]) -> Result<(), String> {
  let out = sdk.join(JVM_OUT);
  reset(&out)?;
  let gradlew = if cfg!(windows) { "gradlew.bat" } else { "./gradlew" };
  invoke(
    Command::new(sdk.join(gradlew))
      .current_dir(sdk)
      .args(["--quiet", ":bindings:generateBindings"])
      .arg(format!(
        "-PbindingsSets={}",
        spec.join(STAGING).join("sets.json").display()
      )),
  )?;
  for s in staged {
    let package_dir = out.join(s.set.jvm.replace('.', "/"));
    write(
      &package_dir.join("Vocabulary.java"),
      &render_java_vocabularies(&s.set.jvm, &s.vocabularies)?,
    )?;
  }
  add_java_headers(&out)
}

/// Runs the command on its arguments (the program name excluded).
pub fn run(arguments: &[String]) -> ExitCode {
  match parse(arguments).and_then(|command| command.execute()) {
    Ok(()) => ExitCode::SUCCESS,
    Err(message) => {
      eprintln!("{message}");
      ExitCode::from(2)
    }
  }
}

#[derive(Debug, PartialEq)]
enum Language {
  TypeScript,
  Jvm,
}

#[derive(Debug, PartialEq)]
struct Invocation {
  spec: PathBuf,
  stage_only: bool,
  only: Option<Language>,
  sdk: Option<PathBuf>,
}

fn parse(arguments: &[String]) -> Result<Invocation, String> {
  let mut invocation = Invocation {
    spec: PathBuf::from("."),
    stage_only: false,
    only: None,
    sdk: None,
  };
  let mut command = None;
  let mut arguments = arguments.iter();
  while let Some(argument) = arguments.next() {
    let mut value = |flag: &str| {
      arguments
        .next()
        .cloned()
        .ok_or(format!("{flag} takes a value\n{USAGE}"))
    };
    match argument.as_str() {
      "--spec" => invocation.spec = PathBuf::from(value("--spec")?),
      "--sdk" => invocation.sdk = Some(PathBuf::from(value("--sdk")?)),
      "--only" => {
        invocation.only = Some(match value("--only")?.as_str() {
          "typescript" => Language::TypeScript,
          "jvm" => Language::Jvm,
          other => return Err(format!("--only takes typescript or jvm, not '{other}'\n{USAGE}")),
        })
      }
      "generate" | "stage" if command.is_none() => command = Some(argument.clone()),
      other => return Err(format!("unexpected '{other}'\n{USAGE}")),
    }
  }
  match command.as_deref() {
    Some("generate") => {}
    Some(_) if invocation.only.is_none() && invocation.sdk.is_none() => invocation.stage_only = true,
    _ => return Err(USAGE.to_string()),
  }
  if invocation.sdk.is_some() && invocation.only.is_none() {
    return Err(format!("--sdk names one SDK, so it needs --only\n{USAGE}"));
  }
  Ok(invocation)
}

impl Invocation {
  fn execute(&self) -> Result<(), String> {
    // Absolute, because each generator runs in its SDK's directory and is handed staging's path.
    let spec = absolute(&self.spec)?;
    let staged = stage(&spec)?;
    if self.stage_only {
      return Ok(());
    }
    let sdk = |language: &str| match &self.sdk {
      Some(sdk) => absolute(sdk),
      None => Ok(spec.join("sdks").join(language)),
    };
    if self.only != Some(Language::Jvm) {
      generate_typescript(&spec, &sdk("typescript")?, &staged)?;
    }
    if self.only != Some(Language::TypeScript) {
      generate_jvm(&spec, &sdk("jvm")?, &staged)?;
    }
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn parsed(arguments: &[&str]) -> Result<Invocation, String> {
    parse(&arguments.iter().map(|a| a.to_string()).collect::<Vec<_>>())
  }

  #[test]
  fn generates_every_sdk_under_the_working_directory_by_default() {
    assert_eq!(
      parsed(&["generate"]).unwrap(),
      Invocation {
        spec: PathBuf::from("."),
        stage_only: false,
        only: None,
        sdk: None
      }
    );
  }

  #[test]
  fn regenerates_one_sdk_checked_out_on_its_own_from_an_unpacked_release() {
    assert_eq!(
      parsed(&["--spec", "/tmp/spec", "generate", "--only", "jvm", "--sdk", "."]).unwrap(),
      Invocation {
        spec: PathBuf::from("/tmp/spec"),
        stage_only: false,
        only: Some(Language::Jvm),
        sdk: Some(PathBuf::from(".")),
      }
    );
  }

  #[test]
  fn refuses_what_it_cannot_mean() {
    assert!(parsed(&[]).is_err());
    assert!(parsed(&["generate", "--sdk", "."]).is_err());
    assert!(parsed(&["generate", "--only", "python"]).is_err());
    assert!(parsed(&["stage", "--only", "jvm"]).is_err());
    assert!(parsed(&["generate", "generate"]).is_err());
  }
}
