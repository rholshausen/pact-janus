//! Staged plan S1.11 (kernel-boundary review findings 3 and 6): "the kernel knows nothing about
//! HTTP or JSON" is checkable by reading the module tree. The one part of the kernel that knows a
//! v1–v4 pact, its HTTP interactions and its JSON matching rules is `src/legacy/`, whose `mod.rs`
//! says so, and this test holds everything else to it: outside `src/legacy/`, `pact_models` is used
//! only where the list below says, for what it says. A new use elsewhere is either a leak across
//! the boundary, to be moved behind it, or a deliberate exception, to be added here with its
//! reason.

use std::path::{Path, PathBuf};

/// `(file under src/, the pact_models module it may use, why)`.
const ALLOWED: &[(&str, &str, &str)] = &[
  (
    "component/generator.rs",
    "generators",
    "the built-in `pact` generator component (ADR 0034): the v1–v4 generator vocabulary, which contracts will name too (S1.19)",
  ),
  (
    "plan/interpret.rs",
    "time_utils",
    "the `datetime` operator's format check, the same Java-style patterns today's verifier reads",
  ),
];

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
  for entry in std::fs::read_dir(dir).unwrap_or_else(|err| panic!("{}: {err}", dir.display())) {
    let path = entry.expect("a directory entry").path();
    if path.is_dir() {
      rust_files(&path, out);
    } else if path.extension().is_some_and(|extension| extension == "rs") {
      out.push(path);
    }
  }
}

#[test]
fn only_the_legacy_module_knows_a_v1_v4_pact() {
  let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
  let mut files = Vec::new();
  rust_files(&src, &mut files);

  let mut leaks = Vec::new();
  for file in files {
    // Compared by component, never by separator, so this reads the same on Windows.
    let relative: Vec<String> = file
      .strip_prefix(&src)
      .expect("under src/")
      .components()
      .map(|component| component.as_os_str().to_string_lossy().to_string())
      .collect();
    if relative.first().is_some_and(|first| first == "legacy") {
      continue;
    }
    let relative = relative.join("/");
    let text = std::fs::read_to_string(&file).expect("a readable source file");
    for (number, line) in text.lines().enumerate() {
      let code = line.split("//").next().unwrap_or_default();
      let mut rest = code;
      while let Some(at) = rest.find("pact_models::") {
        rest = &rest[at + "pact_models::".len()..];
        let module: String = rest
          .chars()
          .take_while(|c| c.is_alphanumeric() || *c == '_')
          .collect();
        let allowed = ALLOWED
          .iter()
          .any(|(path, allowed, _)| *path == relative && *allowed == module);
        if !allowed {
          leaks.push(format!("src/{relative}:{}: pact_models::{module}", number + 1));
        }
      }
    }
  }
  assert!(
    leaks.is_empty(),
    "outside src/legacy/, the kernel uses pact_models where tests/boundary.rs does not allow it:\n  {}",
    leaks.join("\n  ")
  );
}
