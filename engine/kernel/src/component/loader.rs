//! Resolving declared components (component-interfaces spec §2.3, §2.4, §10): from a project's
//! `components` list, as a loader resolved it, to components the engine can call.
//!
//! The kernel owns the rules and none of the machinery. *Loading* — instantiating a `.wasm`,
//! spawning a process — needs a runtime the kernel cannot have (it must build for
//! `wasm32-wasip2`, and a WASM guest cannot host WASM components: ADR 0013), so an embedding
//! registers [`ComponentLoader`]s and the kernel calls them. What happens around a load is the
//! kernel's, because it must be the same whichever loader did it: the declared name must be the name
//! the handshake answers with, contributions must be namespaced with it (§2.4), names are unique
//! across the scope, and every requirement is checked before anything runs (§2.3).

use super::content::ContentComponent;
use super::error::ComponentError;
use super::transport::TransportComponent;
use crate::common::Requirement;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::sync::Arc;

/// One entry of a project's `components` (`component-config.schema.json`'s ComponentDeclaration),
/// as the host's loader resolved it: a `file` source's path is absolute (lifecycle-hooks spec §7.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentDeclaration {
  pub name: String,
  pub source: Source,
  #[serde(default)]
  pub grants: Grants,
  #[serde(default)]
  pub limits: Limits,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Source {
  /// Open vocabulary: `oci`, `file`, `subprocess` (spec §10.2).
  pub kind: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub reference: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub digest: Option<String>,
}

/// Deny-by-default sandbox grants (spec §10.4). `Default` is the grant of nothing.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Grants {
  #[serde(default)]
  pub env: Vec<String>,
  #[serde(default)]
  pub fs: Vec<FsGrant>,
  #[serde(default)]
  pub network: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FsGrant {
  pub path: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub access: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Limits {
  #[serde(rename = "deadline-ms", default, skip_serializing_if = "Option::is_none")]
  pub deadline_ms: Option<u64>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub instances: Option<String>,
}

/// What a loader hands back: the component's own handshake result (spec §3.2 — "the only source of
/// truth about what a component provides"), and the interfaces it implements, bound for calling.
pub struct Loaded {
  /// The `component/hello` result, verbatim. The kernel reads identity and contributions from it.
  pub hello: Value,
  /// The content interface, bound, when the component declared it.
  pub content: Option<Arc<dyn ContentComponent>>,
  /// The transport interface, bound, when the component declared it (plan task 8.3: the
  /// subprocess binding's first client, and the reason a declared component may contribute one).
  pub transport: Option<Arc<dyn TransportComponent>>,
}

/// A way of loading components, which an embedding registers (ADR 0013: hosting is a capability of
/// the embedding, not of the kernel). One loader serves one or more source kinds.
pub trait ComponentLoader: Send + Sync {
  /// The loader's name, as `engine/hello` lists it in `components.loaders` (spec §10.1).
  fn name(&self) -> &str;
  /// The source kinds it loads (`file`, `oci`, …).
  fn sources(&self) -> &[&str];
  /// Load, instantiate and handshake one declared component. Everything that can go wrong is a
  /// value: a digest that does not match, imports beyond the grants, a trap in `component/hello`.
  fn load(&self, declaration: &ComponentDeclaration) -> Result<Loaded, ComponentError>;
}

/// A component this scope can call: who it said it is, and what it implements.
#[derive(Clone)]
pub struct Resolved {
  pub name: String,
  pub version: String,
  pub interfaces: Vec<String>,
  pub content: Option<Arc<dyn ContentComponent>>,
  /// The transport interface, and the `kind`s its handshake contributed (spec §3.3's
  /// `contributes.transports`) — what `start-transport` and a verification target look it up by.
  pub transport: Option<Arc<dyn TransportComponent>>,
  pub transport_kinds: Vec<String>,
}

/// A component compiled into the embedding, named so requirements and conflicts can see it. Only
/// its identity matters here; it is called through the embedding's own registries.
#[derive(Debug, Clone, PartialEq)]
pub struct InTree {
  pub interface: String,
  pub name: String,
  pub version: String,
}

/// Why a resolution failed: always `component-unavailable` at the engine boundary (spec §11.3), with
/// `details` saying which case it is.
#[derive(Debug, Clone, PartialEq)]
pub struct Unavailable {
  pub component: String,
  pub message: String,
  pub details: Value,
}

/// Resolve a scope's declared components (spec §10.3 steps 4–5, §2.4): load each through the loader
/// for its source kind, check its handshake against its declaration, and check names are unique
/// across the declarations and the in-tree components. The first failure stops resolution — a
/// scope with one unloadable component is a scope that cannot run.
pub fn resolve(
  declarations: &[Value],
  loaders: &[Arc<dyn ComponentLoader>],
  in_tree: &[InTree],
) -> Result<Vec<Resolved>, Unavailable> {
  let loader_names: Vec<&str> = std::iter::once("in-tree")
    .chain(loaders.iter().map(|l| l.name()))
    .collect();
  let mut resolved: Vec<Resolved> = Vec::new();
  for (index, raw) in declarations.iter().enumerate() {
    let declaration: ComponentDeclaration =
      serde_json::from_value(raw.clone()).map_err(|err| Unavailable {
        component: raw.get("name").and_then(Value::as_str).unwrap_or("?").to_string(),
        message: format!("components[{index}] is not a component declaration: {err}"),
        details: json!({ "reason": "invalid-declaration", "index": index }),
      })?;
    let name = declaration.name.clone();

    // Spec §2.4: two components with one name is a configuration error, found at resolution and
    // never at first use — including a declared one shadowing an in-tree one, which would otherwise
    // make "which `json` answered" depend on registration order.
    if resolved.iter().any(|r| r.name == name) || in_tree.iter().any(|t| t.name == name) {
      return Err(Unavailable {
        component: name.clone(),
        message: format!("two components are named '{name}'; names are unique within a resolution scope"),
        details: json!({ "code": "component-conflict", "component": name }),
      });
    }

    let Some(loader) = loaders
      .iter()
      .find(|l| l.sources().contains(&declaration.source.kind.as_str()))
    else {
      return Err(Unavailable {
        component: name.clone(),
        message: format!(
          "no loader for component '{name}': this embedding cannot load a '{}' source",
          declaration.source.kind
        ),
        details: json!({ "component": name, "source": declaration.source.kind, "loaders": loader_names }),
      });
    };

    let loaded = loader.load(&declaration).map_err(|error| Unavailable {
      component: name.clone(),
      message: format!("component '{name}' could not be loaded: {}", error.message),
      details: json!({ "component": name, "reason": "load-failed", "error": error }),
    })?;

    resolved.push(check_handshake(&declaration, loaded)?);
  }
  Ok(resolved)
}

/// Spec §3.2 and §2.4, against what the component actually answered: the name it gives must be the
/// name it was declared under, and every name it contributes must be namespaced with it. A
/// contribution in someone else's namespace is `component-invalid` at load, which is what makes the
/// namespace a partition rather than a convention.
fn check_handshake(declaration: &ComponentDeclaration, loaded: Loaded) -> Result<Resolved, Unavailable> {
  let declared = &declaration.name;
  let invalid = |message: String| Unavailable {
    component: declared.clone(),
    message,
    details: json!({ "component": declared, "reason": "component-invalid" }),
  };
  let hello = &loaded.hello;
  let name = hello
    .pointer("/component/name")
    .and_then(Value::as_str)
    .unwrap_or_default();
  let version = hello
    .pointer("/component/version")
    .and_then(Value::as_str)
    .unwrap_or_default();
  if name != declared {
    return Err(invalid(format!(
      "component declared as '{declared}' introduced itself as '{name}'; the handshake is the source of truth, and it disagrees"
    )));
  }
  if major(version).is_none() {
    return Err(invalid(format!(
      "component '{declared}' declared version '{version}', which is not a semantic version"
    )));
  }
  let interfaces: Vec<String> = hello
    .get("interfaces")
    .and_then(Value::as_array)
    .map(|list| {
      list
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect()
    })
    .unwrap_or_default();
  if interfaces.is_empty() {
    return Err(invalid(format!("component '{declared}' declared no interfaces")));
  }

  let prefix = format!("{declared}:");
  if let Some(contributes) = hello.get("contributes").and_then(Value::as_object) {
    for member in ["actions", "operators", "generators"] {
      for entry in contributes
        .get(member)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
      {
        let contributed = entry.get("name").and_then(Value::as_str).unwrap_or_default();
        if !contributed.starts_with(&prefix) || contributed.len() == prefix.len() {
          return Err(invalid(format!(
            "component '{declared}' contributes {member} entry '{contributed}', which is not in its own namespace '{prefix}'"
          )));
        }
      }
    }
  }

  let content = if interfaces.iter().any(|i| i == "content") {
    loaded.content
  } else {
    None
  };
  let (transport, transport_kinds) = if interfaces.iter().any(|i| i == "transport") {
    let kinds: Vec<String> = hello
      .pointer("/contributes/transports")
      .and_then(Value::as_array)
      .into_iter()
      .flatten()
      .filter_map(|entry| entry.get("kind").and_then(Value::as_str))
      .map(str::to_string)
      .collect();
    if loaded.transport.is_some() && kinds.is_empty() {
      return Err(invalid(format!(
        "component '{declared}' implements 'transport' and contributes no transport kind; nothing could ever start it"
      )));
    }
    (loaded.transport, kinds)
  } else {
    (None, Vec::new())
  };
  Ok(Resolved {
    name: declared.clone(),
    version: version.to_string(),
    interfaces,
    content,
    transport,
    transport_kinds,
  })
}

/// Spec §2.3: collect the union of requirements and fail the unsatisfiable ones before anything
/// runs. A requirement names a role — `content/csv` — or, as `janus upgrade` writes for a status
/// class, a bare component name; either is satisfied by a declared or in-tree component with that
/// name (and interface, when named) whose major version is at least the floor.
pub fn check_requirements<'a>(
  requirements: impl IntoIterator<Item = &'a Requirement>,
  resolved: &[Resolved],
  in_tree: &[InTree],
  loaders: &[Arc<dyn ComponentLoader>],
) -> Result<(), Unavailable> {
  let mut seen = BTreeMap::new();
  for requirement in requirements {
    seen
      .entry(requirement.component.clone())
      .or_insert(requirement.min_version);
  }
  for (component, min_version) in seen {
    let (interface, name) = match component.split_once('/') {
      Some((interface, name)) => (Some(interface), name),
      None => (None, component.as_str()),
    };
    let floor = min_version.unwrap_or(0);
    let satisfied = resolved.iter().any(|r| {
      r.name == name
        && interface.is_none_or(|i| r.interfaces.iter().any(|x| x == i))
        && major(&r.version).is_some_and(|m| m >= floor)
    }) || in_tree.iter().any(|t| {
      t.name == name
        && interface.is_none_or(|i| t.interface == i)
        && major(&t.version).is_some_and(|m| m >= floor)
    });
    if !satisfied {
      let loaders: Vec<&str> = std::iter::once("in-tree")
        .chain(loaders.iter().map(|l| l.name()))
        .collect();
      let mut details = json!({ "component": component, "loaders": loaders });
      if let Some(min_version) = min_version {
        details["min-version"] = json!(min_version);
      }
      return Err(Unavailable {
        component: component.clone(),
        message: match min_version {
          Some(v) => format!(
            "no component satisfies '{component}' at major version {v} or above; declare one in the project's components"
          ),
          None => format!("no component satisfies '{component}'; declare one in the project's components"),
        },
        details,
      });
    }
  }
  Ok(())
}

fn major(version: &str) -> Option<u64> {
  version.split('.').next()?.parse().ok()
}
