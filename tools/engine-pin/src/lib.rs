//! The SDKs' engine pin (staged plan S0.10; ADR 0029 decisions 3 and 5): a release's version, and
//! the SHA-256 of its `janus-engine` for each target triple, as `sdks/engine-release.json` holds
//! them. Every resolver in this repository refuses an engine whose digest is not the pinned one, so
//! this file decides which bytes the SDKs run — and it is written from the release's `SHA256SUMS`,
//! never by hand.
//!
//! What is here reads a `SHA256SUMS` and writes the pin; fetching the file and verifying its
//! attestation is `main.rs`'s, through `gh`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// What every pin says about itself.
pub const COMMENT: &str = "The released janus-engine the SDKs resolve when they are not building one from this tree (staged plan S0.4 and S0.10, X.1; ADR 0029 decision 3): a version, and the SHA-256 of the engine binary for each target triple, keyed by the release's asset name. Written by `cargo run -p pact_janus_engine_pin -- <version>`, which reads the digests from that release's SHA256SUMS once `gh attestation verify` has passed on it; never edited by hand. A resolver refuses any download whose digest is not here.";

/// A release asset an engine pin covers: the engine for one target triple. The WASM component
/// (`janus-engine.wasm`) is not one — no resolver runs it.
const ENGINE_PREFIX: &str = "janus-engine-";

/// `sdks/engine-release.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pin {
  #[serde(rename = "$comment")]
  pub comment: String,
  /// `owner/name` of the repository whose releases hold the engine.
  pub repository: String,
  /// The pinned release, without its tag's `v`.
  pub version: String,
  /// Release asset name → the SHA-256 it must have, in hex.
  pub sha256: BTreeMap<String, String>,
}

impl Pin {
  /// The pin for `version` of `repository`, from that release's `SHA256SUMS`.
  pub fn from_sums(repository: &str, version: &str, sums: &str) -> Result<Pin, String> {
    let sha256: BTreeMap<String, String> = parse_sums(sums)?
      .into_iter()
      .filter(|(name, _)| name.starts_with(ENGINE_PREFIX))
      .collect();
    if sha256.is_empty() {
      return Err(format!(
        "SHA256SUMS lists no {ENGINE_PREFIX}<triple>: it is not a release's"
      ));
    }
    Ok(Pin {
      comment: COMMENT.to_string(),
      repository: repository.to_string(),
      version: version.to_string(),
      sha256,
    })
  }

  /// The file as it is checked in: two-space indented, one trailing newline.
  pub fn render(&self) -> String {
    let mut text = serde_json::to_string_pretty(self).expect("a pin is plain JSON");
    text.push('\n');
    text
  }

  /// The TypeScript SDK's copy, `sdks/typescript/src/engine/pin.ts` (ADR 0060 decision 6): the same
  /// pin as a module, since the SDK's CommonJS build cannot find a JSON file shipped beside it. The
  /// JVM SDK copies `sdks/engine-release.json` into its jar as it builds.
  pub fn render_typescript(&self) -> String {
    #[derive(Serialize)]
    struct Body<'a> {
      repository: &'a str,
      version: &'a str,
      sha256: &'a BTreeMap<String, String>,
    }
    let body = serde_json::to_string_pretty(&Body {
      repository: &self.repository,
      version: &self.version,
      sha256: &self.sha256,
    })
    .expect("a pin is plain JSON");
    format!("{TYPESCRIPT_HEADER}\nexport const PIN: EnginePin = {body};\n")
  }
}

/// What the TypeScript SDK's copy of the pin says about itself, and what it imports.
const TYPESCRIPT_HEADER: &str =
  "// The janus-engine this SDK runs when it is not told which (staged plan S1.36; ADR 0029 decision 3,
// ADR 0060 decision 6): sdks/engine-release.json, repeated as a module. Written with it by
// `cargo run -p pact_janus_engine_pin -- <version>`; never edited by hand.

import type { EnginePin } from \"./release.js\";
";

/// A version as a tag spells it or not: `0.1.0` and `v0.1.0` are both `0.1.0`.
pub fn version(given: &str) -> &str {
  given.strip_prefix('v').unwrap_or(given)
}

/// A `SHA256SUMS` in the format `sha256sum` writes and `sha256sum --check` reads: per line, 64 hex
/// digits, a space, a space or `*` (text or binary mode), and the file's name. File name → digest,
/// in lower case. A line that is not that is an error, not something to skip: a pin written from a
/// file this does not understand would be a pin nobody checked.
pub fn parse_sums(text: &str) -> Result<BTreeMap<String, String>, String> {
  let mut sums = BTreeMap::new();
  for (index, line) in text.lines().enumerate() {
    if line.trim().is_empty() {
      continue;
    }
    let malformed = || format!("SHA256SUMS line {}: not '<sha256>  <file>': {line}", index + 1);
    let (digest, rest) = line.split_at_checked(64).ok_or_else(malformed)?;
    if !digest.chars().all(|c| c.is_ascii_hexdigit()) {
      return Err(malformed());
    }
    let name = rest
      .strip_prefix("  ")
      .or_else(|| rest.strip_prefix(" *"))
      .filter(|name| !name.is_empty())
      .ok_or_else(malformed)?;
    let digest = digest.to_ascii_lowercase();
    if let Some(earlier) = sums.insert(name.to_string(), digest.clone())
      && earlier != digest
    {
      return Err(format!("SHA256SUMS lists {name} twice, with different digests"));
    }
  }
  Ok(sums)
}

#[cfg(test)]
mod tests {
  use pretty_assertions::assert_eq;

  use super::*;

  const A: &str = "a8a45e176b8138867906abfa91214a991b4f86733660ae0c83410a7286e83c28";
  const B: &str = "9938aefda0dee23890d22db54caa38abe40d9763534756c6fd7ec9ae9b4bce2f";
  const W: &str = "0000000000000000000000000000000000000000000000000000000000000001";

  fn sums() -> String {
    format!(
      "{A}  janus-aarch64-apple-darwin\n{A}  janus-engine-aarch64-apple-darwin\n{W}  janus-engine.wasm\n{B} *janus-engine-x86_64-unknown-linux-musl\n"
    )
  }

  #[test]
  fn pins_the_engine_for_each_triple_and_nothing_else() {
    let pin = Pin::from_sums("example/janus", "1.2.3", &sums()).unwrap();
    assert_eq!(
      pin.sha256,
      BTreeMap::from([
        ("janus-engine-aarch64-apple-darwin".to_string(), A.to_string()),
        (
          "janus-engine-x86_64-unknown-linux-musl".to_string(),
          B.to_string()
        ),
      ])
    );
    assert_eq!(pin.version, "1.2.3");
    assert_eq!(pin.repository, "example/janus");
  }

  #[test]
  fn renders_as_the_file_is_checked_in() {
    let pin = Pin::from_sums("example/janus", "1.2.3", &sums()).unwrap();
    let expected = format!(
      "{{\n  \"$comment\": {},\n  \"repository\": \"example/janus\",\n  \"version\": \"1.2.3\",\n  \"sha256\": {{\n    \"janus-engine-aarch64-apple-darwin\": \"{A}\",\n    \"janus-engine-x86_64-unknown-linux-musl\": \"{B}\"\n  }}\n}}\n",
      serde_json::to_string(COMMENT).unwrap()
    );
    assert_eq!(pin.render(), expected);
    let read: Pin = serde_json::from_str(&pin.render()).unwrap();
    assert_eq!(read, pin);
  }

  #[test]
  fn the_checked_in_pin_reads() {
    let text = include_str!("../../../sdks/engine-release.json");
    let pin: Pin = serde_json::from_str(text).unwrap();
    assert_eq!(
      pin.render(),
      text,
      "sdks/engine-release.json is not as engine-pin writes it"
    );
  }

  #[test]
  fn the_typescript_sdks_copy_is_the_checked_in_pin() {
    let pin: Pin = serde_json::from_str(include_str!("../../../sdks/engine-release.json")).unwrap();
    assert_eq!(
      pin.render_typescript(),
      include_str!("../../../sdks/typescript/src/engine/pin.ts"),
      "sdks/typescript/src/engine/pin.ts is not as engine-pin writes it from sdks/engine-release.json"
    );
  }

  #[test]
  fn digests_are_lower_case() {
    let sums = format!("{}  janus-engine-x86_64-apple-darwin\n", A.to_ascii_uppercase());
    let pin = Pin::from_sums("example/janus", "1.2.3", &sums).unwrap();
    assert_eq!(pin.sha256["janus-engine-x86_64-apple-darwin"], A);
  }

  #[test]
  fn refuses_what_it_does_not_understand() {
    for bad in [
      "not a digest  janus-engine-x86_64-apple-darwin".to_string(),
      format!("{A} janus-engine-x86_64-apple-darwin"),
      format!("{A}  "),
      format!("{}  janus-engine-x86_64-apple-darwin", &A[..63]),
      format!("{}g  janus-engine-x86_64-apple-darwin", &A[..63]),
    ] {
      assert!(parse_sums(&bad).is_err(), "accepted: {bad}");
    }
    let twice = format!("{A}  janus-engine-x\n{B}  janus-engine-x\n");
    assert_eq!(
      parse_sums(&twice).unwrap_err(),
      "SHA256SUMS lists janus-engine-x twice, with different digests"
    );
  }

  #[test]
  fn refuses_a_file_with_no_engine() {
    let sums = format!("{W}  janus-engine.wasm\n");
    assert!(Pin::from_sums("example/janus", "1.2.3", &sums).is_err());
  }

  #[test]
  fn takes_a_version_with_or_without_its_tag_prefix() {
    assert_eq!(version("v0.1.0"), "0.1.0");
    assert_eq!(version("0.1.0"), "0.1.0");
  }
}
