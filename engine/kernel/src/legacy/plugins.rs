//! A v1–v4 interaction that needs a pact plugin, verified through the plugin's bridge (staged plan S5.2d,
//! ADR 0110): `janus-plugin-bridge`, a component named `pact-plugin-<name>` whose handshake's
//! `pact-plugin` capability says what the plugin's catalogue holds (component-interfaces spec §9.3.1).
//!
//! The kernel carries a component's capabilities without reading them; this is where the one a bridge
//! declares is read, because a pact plugin is a v1–v4 pact's notion (S1.11). [`Bridges`] is what a run's
//! bridges serve; [`bridged`] decides whether an interaction is verified through them, with each body a
//! plugin's content matcher claims compiled to its bridge's `compare-contents` (ADR 0110 decision 3), or
//! refused by name and why ([`Refused`], decision 5).

use super::compile::PluginBody;
use super::pact::{MessageInteraction, PactInteraction, PactPlugin, PluginGenerate, PluginInteraction};
use crate::plan::RuntimeValue;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use pact_models::bodies::OptionalBody;
use pact_models::content_types::ContentType;
use pact_models::matchingrules::MatchingRules;
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;

/// The handshake capability a pact plugin bridge describes its plugin under (component-interfaces spec
/// §9.3.1, ADR 0109 decision 4).
pub const CAPABILITY: &str = "pact-plugin";

/// What each plugin a run has a bridge for serves, by the plugin's name.
#[derive(Debug, Default)]
pub struct Bridges {
  served: BTreeMap<String, Served>,
}

#[derive(Debug)]
struct Served {
  component: String,
  /// Each content matcher: the content types it claims, and the action it is reached through.
  matchers: Vec<(Vec<String>, String)>,
  /// Each content generator, likewise (ADR 0110 decision 7).
  generators: Vec<(Vec<String>, String)>,
  /// The keys of the plugin's transport entries, which no bridge serves yet (staged plan S5.2e).
  transports: Vec<String>,
}

impl Bridges {
  /// The bridges among a run's components: each component's name and handshake capabilities. A component
  /// that declares no `pact-plugin` capability is not one.
  pub fn read<'a>(components: impl IntoIterator<Item = (&'a str, &'a Value)>) -> Bridges {
    let mut served = BTreeMap::new();
    for (component, capabilities) in components {
      let Some(described) = capabilities.get(CAPABILITY) else {
        continue;
      };
      let Some(plugin) = described.get("name").and_then(Value::as_str) else {
        continue;
      };
      let entries = described
        .get("catalogue")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();
      let of_type = |kind: &'static str| {
        entries
          .iter()
          .filter(move |entry| entry.get("type").and_then(Value::as_str) == Some(kind))
      };
      let served_by = |entry: &Value| {
        let action = entry.get("action").and_then(Value::as_str)?;
        let patterns = entry
          .get("content-types")
          .and_then(Value::as_array)
          .into_iter()
          .flatten()
          .filter_map(Value::as_str)
          .map(str::to_string)
          .collect();
        Some((patterns, action.to_string()))
      };
      let matchers = of_type("content-matcher").filter_map(served_by).collect();
      let generators = of_type("content-generator").filter_map(served_by).collect();
      let transports = of_type("transport")
        .filter_map(|entry| entry.get("key").and_then(Value::as_str))
        .map(str::to_string)
        .collect();
      served.insert(
        plugin.to_string(),
        Served {
          component: component.to_string(),
          matchers,
          generators,
          transports,
        },
      );
    }
    Bridges { served }
  }

  /// The component serving `plugin`, when the run has one.
  pub fn component(&self, plugin: &str) -> Option<&str> {
    self.served.get(plugin).map(|served| served.component.as_str())
  }

  /// The first of `plugins` whose bridge has a content matcher claiming `content_type`, and the action
  /// it is reached through.
  fn matcher<'a>(&'a self, plugins: &'a [String], content_type: &str) -> Option<(&'a str, &'a str)> {
    self.find(plugins, content_type, |served| &served.matchers)
  }

  /// Likewise, a content generator (ADR 0110 decision 7).
  fn generator<'a>(&'a self, plugins: &'a [String], content_type: &str) -> Option<(&'a str, &'a str)> {
    self.find(plugins, content_type, |served| &served.generators)
  }

  fn find<'a>(
    &'a self,
    plugins: &'a [String],
    content_type: &str,
    entries: impl Fn(&'a Served) -> &'a Vec<(Vec<String>, String)>,
  ) -> Option<(&'a str, &'a str)> {
    plugins.iter().find_map(|plugin| {
      let served = self.served.get(plugin)?;
      entries(served)
        .iter()
        .find(|(patterns, _)| patterns.iter().any(|pattern| claims(pattern, content_type)))
        .map(|(_, action)| (plugin.as_str(), action.as_str()))
    })
  }
}

/// Whether a content-type pattern a plugin registered claims `media_type`, as pact-plugin-driver decides
/// it: a regular expression, anchored at both ends, over the type and subtype with any suffix,
/// parameters stripped (component-interfaces spec §9.3.1).
pub fn claims(pattern: &str, media_type: &str) -> bool {
  let Ok(content_type) = ContentType::parse(media_type) else {
    return false;
  };
  let base = match &content_type.suffix {
    Some(suffix) => format!("{}/{}+{}", content_type.main_type, content_type.sub_type, suffix),
    None => format!("{}/{}", content_type.main_type, content_type.sub_type),
  };
  regex::Regex::new(&format!("^(?:{pattern})$")).is_ok_and(|regex| regex.is_match(&base))
}

/// Why an interaction that needs a plugin is not verified (ADR 0110 decision 5).
#[derive(Debug, Clone, PartialEq)]
pub enum Refused {
  /// No bridge was declared for these plugins.
  NoBridge(Vec<String>),
  /// It is carried over a plugin's transport, which no bridge serves yet (staged plan S5.2e).
  Transport { plugin: String, transport: String },
  /// One of its rules is a plugin's, which no bridge serves yet (staged plan S5.2g).
  Rules(Vec<String>),
  /// Its kind is neither HTTP nor a message.
  Kind,
}

impl Refused {
  /// What stopped it, as a refusal's `details.why` says.
  pub fn why(&self) -> &'static str {
    match self {
      Refused::NoBridge(_) => "no-bridge",
      Refused::Transport { .. } => "transport",
      Refused::Rules(_) => "rules",
      Refused::Kind => "kind",
    }
  }

  /// Why it is refused, for the reader of a run's report.
  pub fn message(&self) -> String {
    match self {
      Refused::NoBridge(plugins) => format!(
        "this interaction needs the pact {} {}, and no bridge for {} was declared: a run declares \
         janus-plugin-bridge for each plugin a pact names when it is told where the bridge is (janus verify \
         looks where JANUS_PLUGIN_BRIDGE names it, then beside the janus executable)",
        if plugins.len() == 1 { "plugin" } else { "plugins" },
        named(plugins),
        if plugins.len() == 1 { "it" } else { "them" },
      ),
      Refused::Transport { plugin, transport } => format!(
        "this interaction is carried over the pact plugin '{plugin}''s '{transport}' transport, which \
         janus-plugin-bridge does not serve yet (staged plan S5.2e)"
      ),
      Refused::Rules(plugins) => format!(
        "this interaction has a matching rule a pact plugin provides ({}), which janus-plugin-bridge does \
         not serve yet (staged plan S5.2g)",
        named(plugins)
      ),
      Refused::Kind => "this interaction needs a pact plugin, and is neither an HTTP interaction nor a \
                        message"
        .to_string(),
    }
  }
}

fn named(plugins: &[String]) -> String {
  plugins
    .iter()
    .map(|plugin| format!("'{plugin}'"))
    .collect::<Vec<_>>()
    .join(", ")
}

/// `needs` as it is verified through the run's bridges: its HTTP or message interaction, with the body a
/// plugin's content matcher claims compiled to its bridge's action, or why it cannot be.
pub fn bridged(
  needs: &PluginInteraction,
  bridges: &Bridges,
  pact_plugins: &[PactPlugin],
) -> Result<PactInteraction, Refused> {
  let missing: Vec<String> = needs
    .plugins
    .iter()
    .filter(|plugin| !bridges.served.contains_key(*plugin))
    .cloned()
    .collect();
  if !missing.is_empty() {
    return Err(Refused::NoBridge(missing));
  }
  if needs.plugin_rules {
    return Err(Refused::Rules(needs.plugins.clone()));
  }
  if let Some(transport) = &needs.transport
    && let Some(plugin) = needs.plugins.iter().find(|plugin| {
      bridges.served[*plugin]
        .transports
        .iter()
        .any(|key| key.eq_ignore_ascii_case(transport))
    })
  {
    return Err(Refused::Transport {
      plugin: plugin.clone(),
      transport: transport.clone(),
    });
  }
  let configured = |plugin: &str, part: Option<&str>| configuration(needs, pact_plugins, plugin, part);
  match needs.inner.as_deref() {
    Some(PactInteraction::Http(http)) => {
      let mut http = http.clone();
      http.plugin_body = plugin_body(
        &http.response.body,
        &http.response.content_type().map(|ct| ct.to_string()),
        &http.response.matching_rules,
        &needs.plugins,
        bridges,
        |plugin| configured(plugin, Some("response")),
      );
      http.request_generator = request_generator(&http.request, &needs.plugins, bridges, |plugin| {
        // The driver hands a content generator the interaction's whole configuration for the plugin.
        configured(plugin, None)
      });
      Ok(PactInteraction::Http(http))
    }
    Some(PactInteraction::Message(message)) => {
      let mut message: Box<MessageInteraction> = message.clone();
      if let Some(expected) = &message.expected {
        // A synchronous message's response is compared; an asynchronous message is its contents.
        let part = message.synchronous.then_some("response");
        message.plugin_body = plugin_body(
          &expected.contents,
          &expected.message_content_type().map(|ct| ct.to_string()),
          &expected.matching_rules,
          &needs.plugins,
          bridges,
          |plugin| configured(plugin, part),
        );
      }
      Ok(PactInteraction::Message(message))
    }
    _ => Err(Refused::Kind),
  }
}

/// The body a plugin compares, when one of `plugins`' bridges claims its content type. A body no bridge
/// claims is compiled as it would be with no plugin, as today's verifier compares a body no plugin's
/// content matcher claims with its own matchers.
fn plugin_body(
  body: &OptionalBody,
  content_type: &Option<String>,
  rules: &MatchingRules,
  plugins: &[String],
  bridges: &Bridges,
  configured: impl Fn(&str) -> Value,
) -> Option<PluginBody> {
  let OptionalBody::Present(bytes, _, _) = body else {
    return None;
  };
  let content_type = content_type.as_ref()?;
  let (plugin, action) = bridges.matcher(plugins, content_type)?;
  let rules = rules
    .rules_for_category("body")
    .map(|category| category.to_v3_json())
    .unwrap_or_else(|| json!({}));
  Some(PluginBody {
    expected: RuntimeValue::Bytes(bytes.to_vec()),
    action: action.to_string(),
    argument: json!({
      "content-type": content_type,
      "expected": { "content": BASE64.encode(bytes), "encoded": "base64" },
      "rules": rules,
      // Every body a verifier compares, `pact_matching` compares with `AllowUnexpectedKeys`.
      "allow-unexpected-keys": true,
      "plugin-configuration": configured(plugin),
    }),
  })
}

/// A request body's generators as one of `plugins`' content generators applies them (ADR 0110 decision
/// 7): when the body has generators and a bridge's content generator claims its type. A body with none
/// is sent as recorded, as today's verifier sends it.
fn request_generator(
  request: &pact_models::v4::http_parts::HttpRequest,
  plugins: &[String],
  bridges: &Bridges,
  configured: impl Fn(&str) -> Value,
) -> Option<PluginGenerate> {
  if !request.body.is_present() {
    return None;
  }
  let generators = super::pact::body_generators(request)?;
  let content_type = request.content_type()?.to_string();
  let (plugin, action) = bridges.generator(plugins, &content_type)?;
  Some(PluginGenerate {
    action: action.to_string(),
    argument: json!({
      "content-type": content_type,
      "generators": generators,
      "plugin-configuration": configured(plugin),
    }),
  })
}

/// What today's verifier hands `plugin` for an interaction (`pact_matching`'s `plugin_support.rs`): the
/// interaction's configuration for it, its `part` member's when it keeps one by part, and the
/// configuration the pact's metadata records for it (the last entry naming it, as a map by name keeps).
fn configuration(needs: &PluginInteraction, pact: &[PactPlugin], plugin: &str, part: Option<&str>) -> Value {
  let whole = needs.configuration.get(plugin);
  let interaction = match (whole, part) {
    (Some(config), Some(part)) => match config.get(part) {
      Some(Value::Object(by_part)) => Value::Object(by_part.clone()),
      _ => config.clone(),
    },
    (Some(config), None) => config.clone(),
    (None, _) => Value::Object(Map::new()),
  };
  let pact = pact
    .iter()
    .rev()
    .find(|named| named.name == plugin)
    .map(|named| named.configuration.clone())
    .unwrap_or_else(|| Value::Object(Map::new()));
  json!({ "interaction": interaction, "pact": pact })
}

/// The component a plugin's bridge is declared as (ADR 0106 decision 3).
pub fn component_name(plugin: &str) -> String {
  format!("pact-plugin-{plugin}")
}

/// The bridge declarations a run adds for the plugins `pacts` name (ADR 0110 decision 6): one
/// `pact-plugin-<name>` per plugin, with `bridge`'s `source`, `grants` and `limits` and the plugin's name
/// and version appended to the source's `args`, unless `declared` already names the component. A plugin
/// a pact names twice is declared at the highest version named. A plugin whose name makes no component
/// name (component-interfaces spec §2.1) is not declared, and its interactions are refused as having no
/// bridge.
pub fn declarations<'a>(
  pacts: impl IntoIterator<Item = &'a [PactPlugin]>,
  declared: &[&str],
  bridge: &Value,
) -> Vec<Value> {
  let mut wanted: BTreeMap<String, String> = BTreeMap::new();
  for plugin in pacts.into_iter().flatten() {
    let usable = !plugin.name.is_empty()
      && plugin
        .name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if !usable || declared.contains(&component_name(&plugin.name).as_str()) {
      continue;
    }
    wanted
      .entry(plugin.name.clone())
      .and_modify(|version| {
        if newer(&plugin.version, version) {
          *version = plugin.version.clone();
        }
      })
      .or_insert_with(|| plugin.version.clone());
  }
  wanted
    .into_iter()
    .map(|(name, version)| {
      let mut source = bridge.get("source").cloned().unwrap_or_else(|| json!({}));
      let mut args: Vec<Value> = source
        .get("args")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
      args.push(json!(name));
      args.push(json!(version));
      source["args"] = Value::Array(args);
      let mut declaration = json!({ "name": component_name(&name), "source": source });
      for member in ["grants", "limits"] {
        if let Some(value) = bridge.get(member) {
          declaration[member] = value.clone();
        }
      }
      declaration
    })
    .collect()
}

/// Whether `candidate` is a later version than `current`, as semantic versions where both are, and as
/// text otherwise.
fn newer(candidate: &str, current: &str) -> bool {
  match (semver::Version::parse(candidate), semver::Version::parse(current)) {
    (Ok(candidate), Ok(current)) => candidate > current,
    _ => candidate > current,
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use pretty_assertions::assert_eq;
  use rstest::rstest;

  fn plugin(name: &str, version: &str) -> PactPlugin {
    PactPlugin {
      name: name.to_string(),
      version: version.to_string(),
      configuration: json!({ "k": name }),
    }
  }

  fn csv_bridge() -> Value {
    json!({
      "pact-plugin": {
        "name": "csv",
        "version": "0.1.0",
        "catalogue": [
          { "type": "content-matcher", "key": "csv", "content-types": ["text/csv", "application/csv"],
            "action": "pact-plugin-csv:compare-contents" },
          { "type": "content-generator", "key": "csv", "content-types": ["text/csv"] }
        ]
      }
    })
  }

  fn protobuf_bridge() -> Value {
    json!({
      "pact-plugin": {
        "name": "protobuf",
        "catalogue": [
          { "type": "content-matcher", "key": "protobuf", "content-types": ["application/protobuf"],
            "action": "pact-plugin-protobuf:compare-contents" },
          { "type": "transport", "key": "grpc" }
        ]
      }
    })
  }

  #[test]
  fn a_run_declares_a_bridge_for_each_plugin_a_pact_names_at_its_highest_version() {
    let one = [
      plugin("protobuf", "0.8.0"),
      plugin("protobuf", "0.6.0"),
      plugin("csv", "0.1.0"),
    ];
    let two = [plugin("protobuf", "0.10.1"), plugin("Bad_Name", "1.0.0")];
    let bridge = json!({
      "source": { "kind": "subprocess", "reference": "/opt/janus-plugin-bridge", "args": ["--quiet"] },
      "grants": { "env": ["PACT_PLUGIN_DIR"] },
      "limits": { "deadline-ms": 30000 }
    });
    let declared = declarations([&one[..], &two[..]], &["pact-plugin-csv"], &bridge);
    assert_eq!(
      declared,
      vec![json!({
        "name": "pact-plugin-protobuf",
        "source": { "kind": "subprocess", "reference": "/opt/janus-plugin-bridge",
                    "args": ["--quiet", "protobuf", "0.10.1"] },
        "grants": { "env": ["PACT_PLUGIN_DIR"] },
        "limits": { "deadline-ms": 30000 }
      })]
    );
  }

  #[test]
  fn a_bridge_is_read_from_its_capability_and_anything_else_is_not_one() {
    let csv = csv_bridge();
    let other = json!({ "batch-apply": {} });
    let bridges = Bridges::read([("pact-plugin-csv", &csv), ("csv", &other)]);
    assert_eq!(bridges.component("csv"), Some("pact-plugin-csv"));
    assert_eq!(bridges.component("protobuf"), None);
    let plugins = vec!["csv".to_string()];
    assert_eq!(
      bridges.matcher(&plugins, "text/csv;charset=utf-8"),
      Some(("csv", "pact-plugin-csv:compare-contents"))
    );
    assert_eq!(bridges.matcher(&plugins, "application/json"), None);
  }

  #[rstest]
  #[case("text/csv", "text/csv;charset=utf-8", true)]
  #[case("application/.*", "application/grpc", true)]
  #[case("application/json", "application/jwt+json", false)]
  #[case("text/csv", "text/csvx", false)]
  fn a_pattern_claims_a_media_type_as_the_driver_decides(
    #[case] pattern: &str,
    #[case] media_type: &str,
    #[case] claimed: bool,
  ) {
    assert_eq!(claims(pattern, media_type), claimed);
  }

  /// A v4 pact with one interaction of `kind`, needing `plugin`.
  fn needs(interaction: Value, plugins: Value) -> (PluginInteraction, Vec<PactPlugin>) {
    let pact = json!({
      "consumer": { "name": "c" }, "provider": { "name": "p" },
      "interactions": [interaction],
      "metadata": { "pactSpecification": { "version": "4.0" }, "plugins": plugins }
    });
    let read = crate::legacy::pact::read("pact", &pact).expect("a pact");
    let pact_plugins = crate::legacy::pact::pact_plugins(read.as_ref());
    match crate::legacy::pact::interactions(read.as_ref()).remove(0) {
      PactInteraction::NeedsPlugins(needs) => (needs, pact_plugins),
      _ => panic!("the interaction needs a plugin"),
    }
  }

  fn csv_interaction() -> Value {
    json!({
      "type": "Synchronous/HTTP", "description": "request for a report",
      "pluginConfiguration": { "csv": { "response": { "csvHeaders": false } } },
      "request": { "method": "GET", "path": "/reports/report001.csv" },
      "response": {
        "status": 200,
        "headers": { "content-type": ["text/csv"] },
        "body": { "content": "Name,100,2000-01-01\n", "contentType": "text/csv;charset=utf-8", "encoded": false },
        "matchingRules": { "body": { "column:2": { "combine": "AND", "matchers": [{ "match": "number" }] } } }
      }
    })
  }

  #[test]
  fn a_bridged_body_compiles_to_the_bridges_action_with_what_the_pact_says_about_it() {
    let (needs, pact_plugins) = needs(
      csv_interaction(),
      json!([{ "name": "csv", "version": "0.1.0", "configuration": { "global": true } }]),
    );
    let csv = csv_bridge();
    let bridges = Bridges::read([("pact-plugin-csv", &csv)]);
    let Ok(PactInteraction::Http(http)) = bridged(&needs, &bridges, &pact_plugins) else {
      panic!("it is verified through the bridge");
    };
    let body = http.plugin_body.expect("the response body is the plugin's");
    assert_eq!(body.action, "pact-plugin-csv:compare-contents");
    assert_eq!(
      body.expected,
      RuntimeValue::Bytes(b"Name,100,2000-01-01\n".to_vec())
    );
    assert_eq!(
      body.argument,
      json!({
        "content-type": "text/csv;charset=utf-8",
        "expected": { "content": BASE64.encode("Name,100,2000-01-01\n"), "encoded": "base64" },
        "rules": { "column:2": { "combine": "AND", "matchers": [{ "match": "number" }] } },
        "allow-unexpected-keys": true,
        "plugin-configuration": { "interaction": { "csvHeaders": false }, "pact": { "global": true } }
      })
    );
  }

  #[test]
  fn what_a_bridge_cannot_verify_is_refused_and_says_why() {
    let (csv_needs, csv_plugins) = needs(
      csv_interaction(),
      json!([{ "name": "csv", "version": "0.1.0", "configuration": {} }]),
    );
    let none = Bridges::default();
    assert_eq!(
      bridged(&csv_needs, &none, &csv_plugins).err(),
      Some(Refused::NoBridge(vec!["csv".to_string()]))
    );

    let mut over_grpc = csv_interaction();
    over_grpc["pluginConfiguration"] = json!({ "protobuf": {} });
    over_grpc["transport"] = json!("grpc");
    let (grpc_needs, grpc_plugins) = needs(
      over_grpc,
      json!([{ "name": "protobuf", "version": "0.8.1", "configuration": {} }]),
    );
    let protobuf = protobuf_bridge();
    let bridges = Bridges::read([("pact-plugin-protobuf", &protobuf)]);
    let refused = bridged(&grpc_needs, &bridges, &grpc_plugins)
      .err()
      .expect("refused");
    assert_eq!(
      refused,
      Refused::Transport {
        plugin: "protobuf".to_string(),
        transport: "grpc".to_string()
      }
    );
    assert_eq!(refused.why(), "transport");
    assert!(refused.message().contains("S5.2e"), "{}", refused.message());
  }
}
