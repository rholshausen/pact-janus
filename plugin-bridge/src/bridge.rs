//! The bridge's answers, one request frame at a time (component-interfaces spec §3.1): the engine is the
//! only caller, and calls are serial, so the bridge holds one plugin and answers in order.
//!
//! - `component/hello` loads the plugin through pact-plugin-driver, as today's verifier would, installing
//!   it from the plugin index if it is missing (ADR 0106 decision 2), and answers with its catalogue
//!   translated ([`crate::catalogue`]). A plugin that cannot be loaded is `unavailable`, so the engine
//!   reports the declaration as one that did not load.
//! - `matcher/apply` of `<component>:compare-contents` compares a body through the plugin's content
//!   matcher ([`crate::compare`]), and of `<component>:generate-content` fills one through its content
//!   generator ([`crate::generate`]). A plugin the call cannot reach is `unavailable`, and the bridge leaves
//!   once it has answered, so the engine's next call starts a new bridge and a new plugin (ADR 0108
//!   decision 3: out of process, the process is what is recreated).
//! - `component/shutdown` is answered and changes nothing: the plugin lives as long as the bridge's
//!   stdin, the binding's lease on life, because the engine may start another instance at any time.

use crate::catalogue::{self, Entry, Plugin};
use crate::compare::{self, Arguments};
use crate::error::ComponentError;
use crate::generate;
use pact_plugin_driver::call_chain;
use pact_plugin_driver::catalogue_manager::{self, CatalogueEntry, CatalogueEntryProviderType};
use pact_plugin_driver::plugin_manager;
use pact_plugin_driver::plugin_models::PluginDependency;
use serde_json::{Value, json};
use std::cell::Cell;
use std::collections::BTreeMap;
use tokio::runtime::Runtime;

/// The plugin a bridge was started for: its arguments (ADR 0106 decision 3).
#[derive(Debug, Clone, PartialEq)]
pub struct Wanted {
  pub name: String,
  pub version: String,
}

pub struct Bridge {
  wanted: Result<Wanted, String>,
  runtime: Runtime,
  loaded: Option<Loaded>,
  /// Set when the plugin could not be reached: the bridge answers that call, and then leaves.
  gone: Cell<bool>,
}

struct Loaded {
  plugin: Plugin,
  entries: Vec<CatalogueEntry>,
  hello: Value,
}

impl Bridge {
  /// `wanted` is what the command line named, or why it named nothing usable: an error the handshake
  /// answers with, so the engine reports it against the declaration rather than as a process that died.
  pub fn new(wanted: Result<Wanted, String>, runtime: Runtime) -> Bridge {
    Bridge {
      wanted,
      runtime,
      loaded: None,
      gone: Cell::new(false),
    }
  }

  /// Whether the plugin is gone, so the bridge should leave after the answer it has just given.
  pub fn gone(&self) -> bool {
    self.gone.get()
  }

  /// The response frame for one request frame's bytes.
  pub fn answer(&mut self, bytes: &[u8]) -> Value {
    let frame: Value = match serde_json::from_slice(bytes) {
      Ok(frame) => frame,
      Err(err) => {
        return response(
          "",
          Err(ComponentError::protocol(
            "malformed-frame",
            format!("the frame is not JSON: {err}"),
          )),
        );
      }
    };
    let id = frame.get("id").and_then(Value::as_str).unwrap_or_default();
    let op = frame.get("op").and_then(Value::as_str);
    let body = frame.get("body").filter(|body| body.is_object());
    match (frame.get("type").and_then(Value::as_str), op, body) {
      (Some("request"), Some(op), Some(body)) => response(id, self.call(op, body)),
      _ => response(
        id,
        Err(ComponentError::protocol(
          "malformed-frame",
          "a request frame needs type 'request', a string 'op' and an object 'body'",
        )),
      ),
    }
  }

  fn call(&mut self, op: &str, body: &Value) -> Result<Value, ComponentError> {
    if op == "component/hello" {
      return self.hello(body);
    }
    let Some(loaded) = &self.loaded else {
      return Err(ComponentError::protocol(
        "handshake-required",
        format!("'{op}' before component/hello"),
      ));
    };
    match op {
      "component/shutdown" => Ok(json!({})),
      "matcher/apply" => self.apply(loaded, body),
      _ => Err(
        ComponentError::protocol(
          "operation-unsupported",
          format!(
            "the bridge for the pact plugin '{}' does not implement '{op}'",
            loaded.plugin.name
          ),
        )
        .with("op", op),
      ),
    }
  }

  fn hello(&mut self, body: &Value) -> Result<Value, ComponentError> {
    let offered = body
      .get("component-protocol-versions")
      .and_then(Value::as_array)
      .is_some_and(|versions| {
        versions
          .iter()
          .any(|v| v.as_u64() == Some(catalogue::PROTOCOL_VERSION))
      });
    if !offered {
      return Err(
        ComponentError::protocol(
          "protocol-version-unsupported",
          "janus-plugin-bridge speaks component protocol 1 only",
        )
        .with("supported", json!([catalogue::PROTOCOL_VERSION])),
      );
    }
    if let Some(loaded) = &self.loaded {
      return Ok(loaded.hello.clone());
    }
    let wanted = self.wanted.clone().map_err(ComponentError::unavailable)?;
    let dependency = PluginDependency {
      name: wanted.name.clone(),
      version: Some(wanted.version.clone()),
      dependency_type: Default::default(),
    };
    let plugin = self
      .runtime
      .block_on(plugin_manager::load_plugin(&dependency))
      .map_err(|err| {
        ComponentError::unavailable(format!(
          "the pact plugin '{}' version {} could not be loaded",
          wanted.name, wanted.version
        ))
        .with("plugin", wanted.name.as_str())
        .with("version", wanted.version.as_str())
        .caused_by(&err)
      })?;
    let manifest = plugin.manifest;
    let entries: Vec<CatalogueEntry> = catalogue_manager::all_entries()
      .into_iter()
      .filter(|entry| {
        entry.provider_type == CatalogueEntryProviderType::PLUGIN
          && entry.plugin.as_ref().is_some_and(|p| p.name == manifest.name)
      })
      .collect();
    let plugin = Plugin {
      name: manifest.name.clone(),
      version: manifest.version.clone(),
      interface_version: manifest.plugin_interface_version,
    };
    let translated: Vec<Entry> = entries
      .iter()
      .map(|entry| Entry {
        kind: entry.entry_type.to_string(),
        key: entry.key.clone(),
        values: entry
          .values
          .iter()
          .map(|(k, v)| (k.clone(), v.clone()))
          .collect::<BTreeMap<_, _>>(),
      })
      .collect();
    let hello = catalogue::handshake(&plugin, &translated);
    tracing::debug!(plugin = %plugin.name, version = %plugin.version, %hello, "plugin loaded");
    self.loaded = Some(Loaded {
      plugin,
      entries,
      hello: hello.clone(),
    });
    Ok(hello)
  }

  fn apply(&self, loaded: &Loaded, body: &Value) -> Result<Value, ComponentError> {
    let plugin = &loaded.plugin;
    let action = body.get("action").and_then(Value::as_str).unwrap_or_default();
    // Which catalogue entries the action is served by: a content matcher's or a content generator's.
    let (kind, served) = if action == plugin.compare_contents() {
      ("content-matcher", "content matcher")
    } else if action == plugin.generate_content() {
      ("content-generator", "content generator")
    } else {
      ("", "")
    };
    let entries: Vec<Entry> = loaded
      .entries
      .iter()
      .filter(|entry| !kind.is_empty() && entry.entry_type.to_string() == kind)
      .map(|entry| Entry {
        kind: entry.entry_type.to_string(),
        key: entry.key.clone(),
        values: entry.values.clone().into_iter().collect(),
      })
      .collect();
    if entries.is_empty() {
      return Err(
        ComponentError::document(
          "unknown-action",
          format!(
            "the bridge for the pact plugin '{}' contributes no action '{action}'",
            plugin.name
          ),
        )
        .with("action", action),
      );
    }
    let values = body
      .get("values")
      .and_then(Value::as_array)
      .ok_or_else(|| ComponentError::invalid_config("matcher/apply has no 'values'"))?;
    let config = body.get("config");
    let content_type = if kind == "content-matcher" {
      Arguments::read(config)?.content_type
    } else {
      generate::Arguments::read(config)?.content_type
    };
    let entry = entries
      .iter()
      .find(|entry| {
        entry
          .content_types()
          .iter()
          .any(|pattern| catalogue::claims(pattern, &content_type))
      })
      .ok_or_else(|| {
        ComponentError::document(
          "unsupported-content-type",
          format!(
            "the pact plugin '{}' has no {served} for '{content_type}'",
            plugin.name
          ),
        )
        .with("content-type", content_type.as_str())
      })?;
    tracing::debug!(entry = %entry.key, %content_type, %action, "applying");
    let dependency = PluginDependency {
      name: plugin.name.clone(),
      version: Some(plugin.version.clone()),
      dependency_type: Default::default(),
    };
    let Some(instance) = plugin_manager::lookup_plugin(&dependency) else {
      return Err(self.lost(
        plugin,
        anyhow::anyhow!("the plugin driver no longer has it registered"),
      ));
    };
    let mut results = Vec::with_capacity(values.len());
    for value in values {
      let result = if kind == "content-matcher" {
        let arguments = Arguments::read(config)?;
        let request = arguments.request(&arguments.actual(value)?);
        match self.runtime.block_on(instance.compare_contents_with_chain(
          request,
          &call_chain::new_call_chain_id(),
          call_chain::default_deadline_ms(),
        )) {
          Err(err) if compare::unreachable(&err) => return Err(self.lost(plugin, err)),
          answer => compare::result(&compare::mismatches(answer)),
        }
      } else {
        let request = generate::Arguments::read(config)?.request(value)?;
        match self.runtime.block_on(instance.generate_content_with_chain(
          request,
          &call_chain::new_call_chain_id(),
          call_chain::default_deadline_ms(),
        )) {
          Err(err) if compare::unreachable(&err) => return Err(self.lost(plugin, err)),
          answer => generate::result(answer),
        }
      };
      results.push(result);
    }
    Ok(json!({ "results": results }))
  }

  /// The plugin could not be reached: the bridge's failure, never the provider's (ADR 0109 decision 6).
  fn lost(&self, plugin: &Plugin, err: anyhow::Error) -> ComponentError {
    self.gone.set(true);
    tracing::error!(plugin = %plugin.name, "the pact plugin could not be reached ({err:#}); the bridge will exit");
    ComponentError::unavailable(format!(
      "the pact plugin '{}' version {} could not be reached",
      plugin.name, plugin.version
    ))
    .with("plugin", plugin.name.as_str())
    .with("version", plugin.version.as_str())
    .caused_by(&err)
  }

  /// stdin has ended: the plugin goes with the bridge.
  pub fn shutdown(self) {
    if self.loaded.is_some() {
      plugin_manager::shutdown_plugins();
    }
    self.runtime.shutdown_background();
  }
}

fn response(id: &str, result: Result<Value, ComponentError>) -> Value {
  match result {
    Ok(ok) => json!({ "type": "response", "id": id, "ok": ok }),
    Err(error) => json!({ "type": "response", "id": id, "error": error.to_json() }),
  }
}
