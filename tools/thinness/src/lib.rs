//! The thinness audit (plan task 6.5): how much of each SDK a maintainer actually writes.
//!
//! The RFC's claim is that a Janus SDK is thin — that the protocol, the generated bindings and the
//! engine take the weight, and what is left in each language is a DSL surface and some glue. That is
//! a claim about a ratio, so it needs a number, and a number that goes stale the moment an SDK
//! changes is worse than none. This crate is the measurement, re-runnable: it reads
//! `sdks/thinness.json`, counts every source file by layer, and holds the hand-written layers to a
//! budget.
//!
//! Two rules keep the number honest. **Every source file must be claimed by exactly one layer** — an
//! unclassified file is an error, not a silent omission, so a new file forces somebody to say which
//! layer it belongs in rather than letting it land wherever the claim looks best. And **both a total
//! and a code count are reported**: these SDKs are deliberately comment-heavy, comments are not the
//! cost the claim is about, and hiding the difference in either direction would be a way of winning
//! the argument by choosing a counting rule.

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

/// The layers, in the order they are reported. `generated` and `tests` carry no budget: generated
/// code is not a cost an SDK maintainer pays, and tests are not part of the surface a user installs.
pub const LAYERS: [&str; 5] = ["generated", "idiomatic", "embedding", "integration", "tests"];

/// What one file, or one layer, or one SDK amounts to.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Count {
  pub files: usize,
  /// Every line in the file.
  pub lines: usize,
  /// Lines that are neither blank nor wholly comment.
  pub code: usize,
}

impl Count {
  pub fn add(&mut self, other: Count) {
    self.files += other.files;
    self.lines += other.lines;
    self.code += other.code;
  }
}

/// One SDK's manifest entry.
#[derive(Debug, Clone)]
pub struct Sdk {
  pub name: String,
  pub language: String,
  pub root: PathBuf,
  pub extensions: Vec<String>,
  pub ignore: Vec<String>,
  /// Layer name -> the paths, relative to `root`, that belong to it. A path may be a directory or
  /// a single file; the longest matching prefix wins, so a file inside a claimed directory can be
  /// pulled into another layer by naming it.
  pub layers: BTreeMap<String, Vec<String>>,
  pub budgets: BTreeMap<String, usize>,
}

/// One SDK's measurement.
#[derive(Debug, Clone)]
pub struct Measured {
  pub sdk: Sdk,
  pub by_layer: BTreeMap<String, Count>,
  /// Files under `root` that no layer claimed — always an error (module docs).
  pub unclassified: Vec<PathBuf>,
}

impl Measured {
  /// The lines a maintainer of this SDK writes and reviews: everything but `generated` and `tests`.
  pub fn hand_written(&self) -> Count {
    let mut total = Count::default();
    for (layer, count) in &self.by_layer {
      if layer != "generated" && layer != "tests" {
        total.add(*count);
      }
    }
    total
  }

  /// The layers that carry a budget, and whether each is inside it.
  pub fn over_budget(&self) -> Vec<(String, usize, usize)> {
    let mut over = Vec::new();
    for (layer, budget) in &self.sdk.budgets {
      let actual = self.by_layer.get(layer).copied().unwrap_or_default().code;
      if actual > *budget {
        over.push((layer.clone(), actual, *budget));
      }
    }
    over
  }
}

/// Anything that stops the audit producing a number it can stand behind.
#[derive(Debug)]
pub enum Error {
  Io(String),
  Manifest(String),
}

impl fmt::Display for Error {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Error::Io(message) | Error::Manifest(message) => write!(f, "{message}"),
    }
  }
}

/// Read `sdks/thinness.json`, relative to the repository root.
pub fn load_manifest(repo_root: &Path) -> Result<Vec<Sdk>, Error> {
  let path = repo_root.join("sdks/thinness.json");
  let text = fs::read_to_string(&path).map_err(|e| Error::Io(format!("{}: {e}", path.display())))?;
  let document: Value =
    serde_json::from_str(&text).map_err(|e| Error::Manifest(format!("{}: {e}", path.display())))?;

  let entries = document
    .get("sdks")
    .and_then(Value::as_array)
    .ok_or_else(|| Error::Manifest("thinness.json: 'sdks' must be an array".to_string()))?;

  entries.iter().map(sdk_from).collect()
}

fn sdk_from(entry: &Value) -> Result<Sdk, Error> {
  let string = |key: &str| -> Result<String, Error> {
    entry
      .get(key)
      .and_then(Value::as_str)
      .map(str::to_string)
      .ok_or_else(|| Error::Manifest(format!("thinness.json: an sdk is missing '{key}'")))
  };
  let strings = |key: &str| -> Vec<String> {
    entry
      .get(key)
      .and_then(Value::as_array)
      .map(|values| {
        values
          .iter()
          .filter_map(Value::as_str)
          .map(str::to_string)
          .collect()
      })
      .unwrap_or_default()
  };

  let name = string("name")?;
  let mut layers = BTreeMap::new();
  let declared = entry
    .get("layers")
    .and_then(Value::as_object)
    .ok_or_else(|| Error::Manifest(format!("{name}: 'layers' must be an object")))?;
  for (layer, paths) in declared {
    if !LAYERS.contains(&layer.as_str()) {
      return Err(Error::Manifest(format!(
        "{name}: '{layer}' is not a layer; the layers are {}",
        LAYERS.join(", ")
      )));
    }
    let paths: Vec<String> = paths
      .as_array()
      .ok_or_else(|| Error::Manifest(format!("{name}: layer '{layer}' must be an array of paths")))?
      .iter()
      .filter_map(Value::as_str)
      .map(str::to_string)
      .collect();
    layers.insert(layer.clone(), paths);
  }

  let mut budgets = BTreeMap::new();
  if let Some(declared) = entry.get("budgets").and_then(Value::as_object) {
    for (layer, budget) in declared {
      let budget = budget.as_u64().ok_or_else(|| {
        Error::Manifest(format!(
          "{name}: budget for '{layer}' must be a number of code lines"
        ))
      })?;
      if !layers.contains_key(layer) {
        return Err(Error::Manifest(format!(
          "{name}: budget for '{layer}', which is not a declared layer"
        )));
      }
      budgets.insert(layer.clone(), budget as usize);
    }
  }

  Ok(Sdk {
    name,
    language: string("language")?,
    root: PathBuf::from(string("root")?),
    extensions: strings("extensions"),
    ignore: strings("ignore"),
    layers,
    budgets,
  })
}

/// Count one SDK.
pub fn measure(repo_root: &Path, sdk: &Sdk) -> Result<Measured, Error> {
  let root = repo_root.join(&sdk.root);
  let mut files = Vec::new();
  collect(&root, &root, sdk, &mut files)?;
  files.sort();

  let mut by_layer: BTreeMap<String, Count> = BTreeMap::new();
  let mut unclassified = Vec::new();
  for relative in &files {
    match layer_of(relative, sdk) {
      Some(layer) => {
        let count = count_file(&root.join(relative))?;
        by_layer.entry(layer).or_default().add(count);
      }
      None => unclassified.push(relative.clone()),
    }
  }

  Ok(Measured {
    sdk: sdk.clone(),
    by_layer,
    unclassified,
  })
}

/// The layer whose declared path is the longest prefix of `relative`. Longest wins so that a single
/// file can be lifted out of a directory another layer claims — the JVM's JUnit extension lives in
/// the same package as the DSL, and it is integration, not DSL.
fn layer_of(relative: &Path, sdk: &Sdk) -> Option<String> {
  let relative = relative.to_string_lossy().replace('\\', "/");
  let mut best: Option<(usize, String)> = None;
  for (layer, paths) in &sdk.layers {
    for declared in paths {
      let matches = relative == *declared || relative.starts_with(&format!("{declared}/"));
      if matches && best.as_ref().is_none_or(|(len, _)| declared.len() > *len) {
        best = Some((declared.len(), layer.clone()));
      }
    }
  }
  best.map(|(_, layer)| layer)
}

fn collect(root: &Path, directory: &Path, sdk: &Sdk, out: &mut Vec<PathBuf>) -> Result<(), Error> {
  let entries = fs::read_dir(directory).map_err(|e| Error::Io(format!("{}: {e}", directory.display())))?;
  for entry in entries {
    let entry = entry.map_err(|e| Error::Io(format!("{}: {e}", directory.display())))?;
    let path = entry.path();
    let relative = path.strip_prefix(root).unwrap_or(&path).to_path_buf();
    let name = entry.file_name().to_string_lossy().to_string();
    let relative_text = relative.to_string_lossy().replace('\\', "/");
    if sdk
      .ignore
      .iter()
      .any(|ignored| name == *ignored || relative_text == *ignored)
      || name.starts_with('.')
    {
      continue;
    }
    if path.is_dir() {
      collect(root, &path, sdk, out)?;
    } else if sdk.extensions.iter().any(|extension| name.ends_with(extension)) {
      out.push(relative);
    }
  }
  Ok(())
}

/// Total lines, and lines that are neither blank nor wholly comment. Both languages here use C-style
/// comments, so one scanner serves: it tracks `/* … */` across lines and treats a line whose first
/// non-space text begins a `//` or sits inside a block comment as not code. A line with code *and* a
/// trailing comment counts as code, which is the answer that matches what the number is for.
fn count_file(path: &Path) -> Result<Count, Error> {
  let text = fs::read_to_string(path).map_err(|e| Error::Io(format!("{}: {e}", path.display())))?;
  let mut count = Count {
    files: 1,
    lines: 0,
    code: 0,
  };
  let mut in_block = false;
  for line in text.lines() {
    count.lines += 1;
    if is_code(line, &mut in_block) {
      count.code += 1;
    }
  }
  Ok(count)
}

/// Whether this line carries code, advancing `in_block` across `/* … */`. String literals holding
/// comment markers are not tracked: the miscount that would need a `"/*"` in a string is rarer than
/// the complexity of a full lexer, and the audit reports total lines alongside, so a reader can see
/// the size of what this approximates.
fn is_code(line: &str, in_block: &mut bool) -> bool {
  let bytes: Vec<char> = line.chars().collect();
  let mut index = 0;
  let mut saw_code = false;
  while index < bytes.len() {
    if *in_block {
      if bytes[index] == '*' && bytes.get(index + 1) == Some(&'/') {
        *in_block = false;
        index += 2;
      } else {
        index += 1;
      }
      continue;
    }
    if bytes[index] == '/' && bytes.get(index + 1) == Some(&'/') {
      break;
    }
    if bytes[index] == '/' && bytes.get(index + 1) == Some(&'*') {
      *in_block = true;
      index += 2;
      continue;
    }
    if !bytes[index].is_whitespace() {
      saw_code = true;
    }
    index += 1;
  }
  saw_code
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn a_line_of_code_with_a_trailing_comment_is_code() {
    let mut in_block = false;
    assert!(is_code("let x = 1; // why", &mut in_block));
  }

  #[test]
  fn blank_and_comment_only_lines_are_not_code() {
    let mut in_block = false;
    assert!(!is_code("", &mut in_block));
    assert!(!is_code("   ", &mut in_block));
    assert!(!is_code("  // a note", &mut in_block));
  }

  #[test]
  fn a_block_comment_spans_lines_and_releases_the_line_it_ends_on() {
    let mut in_block = false;
    assert!(!is_code("/* opening", &mut in_block));
    assert!(in_block);
    assert!(!is_code(" * continued", &mut in_block));
    assert!(is_code(" */ let x = 1;", &mut in_block));
    assert!(!in_block);
  }

  #[test]
  fn the_longest_declared_prefix_decides_the_layer() {
    let mut layers = BTreeMap::new();
    layers.insert("idiomatic".to_string(), vec!["src/main/java/io/pact".to_string()]);
    layers.insert(
      "integration".to_string(),
      vec!["src/main/java/io/pact/Ext.java".to_string()],
    );
    let sdk = Sdk {
      name: "t".into(),
      language: "Java".into(),
      root: PathBuf::from("x"),
      extensions: vec![".java".into()],
      ignore: vec![],
      layers,
      budgets: BTreeMap::new(),
    };
    assert_eq!(
      layer_of(Path::new("src/main/java/io/pact/Ext.java"), &sdk).as_deref(),
      Some("integration")
    );
    assert_eq!(
      layer_of(Path::new("src/main/java/io/pact/Dsl.java"), &sdk).as_deref(),
      Some("idiomatic")
    );
    assert_eq!(layer_of(Path::new("src/other/Thing.java"), &sdk), None);
  }
}
