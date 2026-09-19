//! `janus-bindings generate [--only typescript|jvm]` — regenerates every SDK's protocol bindings
//! from the spec schemas (plan task 6.1): the one command CLAUDE.md's "regenerate them via the
//! binding pipeline" means.
//!
//! 1. Normalise each schema set `sdks/bindings.json` names into `target/bindings/schemas/<set>/`,
//!    one standalone schema per type; stage each generator's reading of it under
//!    `target/bindings/{typescript,jvm}/<set>/`; and write `target/bindings/sets.json` describing
//!    them. Every change made to a schema on its way to a generator happens here, in tested Rust —
//!    the generator steps below only invoke their generator.
//! 2. Clear each language's generated tree and run its generator over the normalised schemas:
//!    `npm run generate:bindings` in `sdks/typescript` (json-schema-to-typescript), and
//!    `./gradlew :bindings:generateBindings` in `sdks/jvm` (jsonschema2pojo).
//! 3. Render each set's open vocabularies next to its generated types.
//!
//! `janus-bindings stage` does step 1 alone. CI runs `generate` and then fails on any diff under
//! `sdks/`: checked-in bindings that do not match their schemas are a second source of truth.

use pact_janus_bindings::{
  Manifest, NamedSchema, Vocabulary, normalise, prepare_for_jvm, prepare_for_typescript,
  render_java_vocabularies, render_typescript_vocabularies, vocabularies,
};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

const TYPESCRIPT_OUT: &str = "sdks/typescript/src/generated";
const JVM_OUT: &str = "sdks/jvm/bindings/src/main/java";

struct Staged {
  set: pact_janus_bindings::SchemaSet,
  vocabularies: Vec<Vocabulary>,
}

fn repo_root() -> Result<PathBuf, String> {
  let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
  root
    .canonicalize()
    .map_err(|e| format!("{}: {e}", root.display()))
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
  let manifest_path = root.join("sdks/bindings.json");
  let manifest: Manifest = serde_json::from_str(
    &std::fs::read_to_string(&manifest_path).map_err(|e| format!("{}: {e}", manifest_path.display()))?,
  )
  .map_err(|e| format!("{}: {e}", manifest_path.display()))?;

  let staging = root.join("target/bindings");
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
  for line in pact_janus_bindings::GENERATED_HEADER.lines() {
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

fn run(command: &mut Command) -> Result<(), String> {
  let status = command.status().map_err(|e| format!("{command:?}: {e}"))?;
  if status.success() {
    Ok(())
  } else {
    Err(format!("{command:?} failed: {status}"))
  }
}

fn generate_typescript(root: &Path, staged: &[Staged]) -> Result<(), String> {
  let out = root.join(TYPESCRIPT_OUT);
  reset(&out)?;
  let sdk = root.join("sdks/typescript");
  let npm = if cfg!(windows) { "npm.cmd" } else { "npm" };
  run(
    Command::new(npm)
      .current_dir(&sdk)
      .args(["run", "--silent", "generate:bindings", "--"])
      .arg(root.join("target/bindings/sets.json"))
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

fn generate_jvm(root: &Path, staged: &[Staged]) -> Result<(), String> {
  let out = root.join(JVM_OUT);
  reset(&out)?;
  let sdk = root.join("sdks/jvm");
  let gradlew = if cfg!(windows) { "gradlew.bat" } else { "./gradlew" };
  run(
    Command::new(sdk.join(gradlew))
      .current_dir(&sdk)
      .args(["--quiet", ":bindings:generateBindings"])
      .arg(format!(
        "-PbindingsSets={}",
        root.join("target/bindings/sets.json").display()
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

fn main() -> ExitCode {
  let args: Vec<String> = std::env::args().skip(1).collect();
  let result =
    repo_root().and_then(
      |root| match args.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
        ["stage"] => stage(&root).map(|_| ()),
        ["generate"] => {
          stage(&root).and_then(|s| generate_typescript(&root, &s).and_then(|_| generate_jvm(&root, &s)))
        }
        ["generate", "--only", "typescript"] => stage(&root).and_then(|s| generate_typescript(&root, &s)),
        ["generate", "--only", "jvm"] => stage(&root).and_then(|s| generate_jvm(&root, &s)),
        _ => Err(
          "usage: janus-bindings generate [--only typescript|jvm]\n       janus-bindings stage".to_string(),
        ),
      },
    );
  match result {
    Ok(()) => ExitCode::SUCCESS,
    Err(message) => {
      eprintln!("{message}");
      ExitCode::from(2)
    }
  }
}
