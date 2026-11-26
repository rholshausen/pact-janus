//! A plugin's catalogue, translated into a Janus handshake (ADR 0106 decision 4, ADR 0109).
//!
//! The bridge is a component named `pact-plugin-<name>`, at the plugin's own version, implementing the
//! matcher interface, and the transport interface when the plugin has a transport. What it contributes is
//! only what it can be called for: a content matcher becomes the action `<component>:compare-contents`, a
//! content generator `<component>:generate-content`, and a transport a kind the bridge drives, whose
//! verdict `<component>:verdict` reads (ADR 0111). Every catalogue entry, bridged or not, is described under
//! the handshake's `pact-plugin` capability, with the content types a content matcher claims, so the
//! engine learns which bodies the plugin compares from the plugin, and nothing about the plugin itself.

use serde_json::{Map, Value, json};
use std::collections::BTreeMap;

/// The component protocol version the bridge speaks (spec §3.2).
pub const PROTOCOL_VERSION: u64 = 1;
/// The capability a bridge's handshake describes its plugin under (ADR 0109).
pub const CAPABILITY: &str = "pact-plugin";

/// The plugin a bridge serves, as its manifest gives it.
#[derive(Debug, Clone, PartialEq)]
pub struct Plugin {
  pub name: String,
  pub version: String,
  pub interface_version: u8,
}

impl Plugin {
  pub fn component(&self) -> String {
    component_name(&self.name)
  }

  pub fn compare_contents(&self) -> String {
    format!("{}:compare-contents", self.component())
  }

  /// The action a content generator is reached through (ADR 0110 decision 7).
  pub fn generate_content(&self) -> String {
    format!("{}:generate-content", self.component())
  }

  /// The action a transport's verdict is read through (ADR 0111 decision 3).
  pub fn verdict(&self) -> String {
    format!("{}:verdict", self.component())
  }
}

/// The component a plugin is served as (ADR 0106 decision 3).
pub fn component_name(plugin: &str) -> String {
  format!("pact-plugin-{plugin}")
}

/// Whether a plugin's name makes a component name (spec §2.1: `[a-z][a-z0-9-]*`, after the prefix).
pub fn usable_name(plugin: &str) -> bool {
  !plugin.is_empty()
    && plugin
      .chars()
      .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// One catalogue entry, as the plugin registered it.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
  /// The driver's name for its type: `content-matcher`, `content-generator`, `transport`, `matcher`,
  /// `interaction` or `generator`.
  pub kind: String,
  pub key: String,
  pub values: BTreeMap<String, String>,
}

impl Entry {
  /// The content types a content matcher or generator claims: the plugin's `content-types` value,
  /// split at `;`. Each is a pattern, matched as pact-plugin-driver matches it ([`claims`]).
  pub fn content_types(&self) -> Vec<String> {
    self
      .values
      .get("content-types")
      .map(|types| {
        types
          .split(';')
          .map(str::trim)
          .filter(|t| !t.is_empty())
          .map(str::to_string)
          .collect()
      })
      .unwrap_or_default()
  }
}

/// Whether a content-type pattern a plugin registered claims `media_type`. As pact-plugin-driver decides
/// it (`catalogue_manager::matches_pattern`): the pattern is a regular expression, anchored at both ends,
/// matched against the type and subtype with any structured-syntax suffix, parameters stripped.
pub fn claims(pattern: &str, media_type: &str) -> bool {
  let Ok(content_type) = pact_models::content_types::ContentType::parse(media_type) else {
    return false;
  };
  let base = match &content_type.suffix {
    Some(suffix) => format!("{}/{}+{}", content_type.main_type, content_type.sub_type, suffix),
    None => format!("{}/{}", content_type.main_type, content_type.sub_type),
  };
  regex::Regex::new(&format!("^(?:{pattern})$")).is_ok_and(|regex| regex.is_match(&base))
}

/// Whether a transport entry's key makes a transport kind the bridge can contribute: a kind is an open
/// vocabulary of names (spec §5.1), and the bridge contributes one only when it reads as one.
pub fn usable_kind(key: &str) -> bool {
  !key.is_empty()
    && key
      .chars()
      .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '+' || c == '.')
}

/// The handshake a bridge answers `component/hello` with, for `plugin` and its catalogue.
pub fn handshake(plugin: &Plugin, entries: &[Entry]) -> Value {
  let compare_contents = plugin.compare_contents();
  let generate_content = plugin.generate_content();
  let verdict = plugin.verdict();
  let matches_contents = entries.iter().any(|entry| entry.kind == "content-matcher");
  let generates_contents = entries.iter().any(|entry| entry.kind == "content-generator");
  // A plugin transport is a kind the bridge drives (ADR 0111 decision 1), its exchange one send.
  let mut kinds: Vec<&str> = entries
    .iter()
    .filter(|entry| entry.kind == "transport" && usable_kind(&entry.key))
    .map(|entry| entry.key.as_str())
    .collect();
  kinds.sort_unstable();
  kinds.dedup();
  let mut sorted: Vec<&Entry> = entries.iter().collect();
  sorted.sort_by(|a, b| (&a.kind, &a.key).cmp(&(&b.kind, &b.key)));
  let catalogue: Vec<Value> = sorted
    .into_iter()
    .map(|entry| {
      let mut described = Map::new();
      described.insert("type".to_string(), json!(entry.kind));
      described.insert("key".to_string(), json!(entry.key));
      if matches!(entry.kind.as_str(), "content-matcher" | "content-generator") {
        described.insert("content-types".to_string(), json!(entry.content_types()));
      }
      if entry.kind == "content-matcher" {
        described.insert("action".to_string(), json!(compare_contents));
      }
      if entry.kind == "content-generator" {
        described.insert("action".to_string(), json!(generate_content));
      }
      if entry.kind == "transport" && usable_kind(&entry.key) {
        described.insert("kind".to_string(), json!(entry.key));
        described.insert("action".to_string(), json!(verdict));
      }
      described.insert("values".to_string(), json!(entry.values));
      Value::Object(described)
    })
    .collect();
  let actions: Vec<Value> = [
    (matches_contents, compare_contents),
    (generates_contents, generate_content),
    (!kinds.is_empty(), verdict),
  ]
  .into_iter()
  .filter(|(served, _)| *served)
  .map(|(_, action)| json!({ "name": action }))
  .collect();
  let mut interfaces = vec!["matcher"];
  let mut contributes = json!({ "actions": actions });
  if !kinds.is_empty() {
    interfaces.push("transport");
    contributes["transports"] = kinds
      .iter()
      .map(|kind| {
        json!({
          "kind": kind,
          "roles": ["drive"],
          "exchange-parts": {
            "initiating": crate::transport::INITIATING,
            "answering": crate::transport::ANSWERING
          }
        })
      })
      .collect();
  }
  json!({
    "component-protocol-version": PROTOCOL_VERSION,
    "component": { "name": plugin.component(), "version": plugin.version },
    "interfaces": interfaces,
    "contributes": contributes,
    "capabilities": {
      CAPABILITY: {
        "name": plugin.name,
        "version": plugin.version,
        "interface-version": plugin.interface_version,
        "bridge": { "name": "janus-plugin-bridge", "version": env!("CARGO_PKG_VERSION") },
        "catalogue": catalogue,
      }
    }
  })
}

#[cfg(test)]
mod tests {
  use super::*;
  use pretty_assertions::assert_eq;
  use rstest::rstest;

  fn entry(kind: &str, key: &str, values: &[(&str, &str)]) -> Entry {
    Entry {
      kind: kind.to_string(),
      key: key.to_string(),
      values: values
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect(),
    }
  }

  fn protobuf() -> Plugin {
    Plugin {
      name: "protobuf".to_string(),
      version: "0.8.1".to_string(),
      interface_version: 1,
    }
  }

  #[test]
  fn a_content_matcher_is_the_compare_contents_action_and_every_entry_is_described() {
    let hello = handshake(
      &protobuf(),
      &[
        entry("transport", "grpc", &[]),
        entry(
          "content-matcher",
          "protobuf",
          &[("content-types", "application/protobuf;application/grpc")],
        ),
        entry(
          "content-generator",
          "protobuf",
          &[("content-types", "application/protobuf;application/grpc")],
        ),
      ],
    );
    assert_eq!(
      hello["component"],
      json!({ "name": "pact-plugin-protobuf", "version": "0.8.1" })
    );
    assert_eq!(hello["interfaces"], json!(["matcher", "transport"]));
    assert_eq!(
      hello["contributes"],
      json!({
        "actions": [
          { "name": "pact-plugin-protobuf:compare-contents" },
          { "name": "pact-plugin-protobuf:generate-content" },
          { "name": "pact-plugin-protobuf:verdict" }
        ],
        "transports": [{
          "kind": "grpc",
          "roles": ["drive"],
          "exchange-parts": { "initiating": "request", "answering": "response" }
        }]
      })
    );
    let described = &hello["capabilities"]["pact-plugin"];
    assert_eq!(described["name"], "protobuf");
    assert_eq!(described["interface-version"], 1);
    assert_eq!(
      described["catalogue"],
      json!([
        {
          "type": "content-generator",
          "key": "protobuf",
          "content-types": ["application/protobuf", "application/grpc"],
          "action": "pact-plugin-protobuf:generate-content",
          "values": { "content-types": "application/protobuf;application/grpc" }
        },
        {
          "type": "content-matcher",
          "key": "protobuf",
          "content-types": ["application/protobuf", "application/grpc"],
          "action": "pact-plugin-protobuf:compare-contents",
          "values": { "content-types": "application/protobuf;application/grpc" }
        },
        {
          "type": "transport",
          "key": "grpc",
          "kind": "grpc",
          "action": "pact-plugin-protobuf:verdict",
          "values": {}
        }
      ])
    );
  }

  #[test]
  fn a_plugin_with_nothing_the_bridge_serves_contributes_nothing() {
    let hello = handshake(&protobuf(), &[entry("matcher", "credit-card", &[])]);
    assert_eq!(hello["interfaces"], json!(["matcher"]));
    assert_eq!(hello["contributes"], json!({ "actions": [] }));
  }

  #[test]
  fn a_transport_whose_key_is_no_kind_is_described_and_not_contributed() {
    let hello = handshake(&protobuf(), &[entry("transport", "My Transport", &[])]);
    assert_eq!(hello["interfaces"], json!(["matcher"]));
    assert_eq!(hello["contributes"], json!({ "actions": [] }));
    assert_eq!(
      hello["capabilities"]["pact-plugin"]["catalogue"][0],
      json!({ "type": "transport", "key": "My Transport", "values": {} })
    );
  }

  #[rstest]
  #[case("text/csv", "text/csv", true)]
  #[case("text/csv", "text/csv;charset=utf-8", true)]
  #[case(
    "application/protobuf",
    "application/protobuf;message=.area_calculator.ShapeMessage",
    true
  )]
  #[case("application/.*", "application/grpc", true)]
  #[case("text/csv", "text/csvx", false)]
  #[case("application/jwt\\+json", "application/jwt+json", true)]
  #[case("application/json", "application/jwt+json", false)]
  #[case("text/csv", "not a media type", false)]
  fn a_pattern_claims_a_media_type_as_the_driver_decides(
    #[case] pattern: &str,
    #[case] media_type: &str,
    #[case] claimed: bool,
  ) {
    assert_eq!(claims(pattern, media_type), claimed);
  }

  #[rstest]
  #[case("protobuf", true)]
  #[case("pact-avro-plugin", true)]
  #[case("credit-card2", true)]
  #[case("Avro", false)]
  #[case("my_plugin", false)]
  #[case("", false)]
  fn a_plugin_name_must_make_a_component_name(#[case] name: &str, #[case] usable: bool) {
    assert_eq!(usable_name(name), usable);
  }
}
