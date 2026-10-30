//! A resolution scope (component-interfaces spec §2.4, §10): the components one consumer session
//! or one verification run can call — the ones its `components` declared, loaded when it started,
//! ahead of the ones compiled into the embedding.
//!
//! Scoped per session rather than per engine because spec §3.4 says sessions are the only
//! resource and that rule "extends inward": a component a session loaded ends with it, and two
//! sessions on one pipe may declare different ones.

use super::contributions::Contributions;
use crate::common::{ContentTypes, Requirement};
use crate::component::{
  self, Compile, ComponentLoader, ContentComponent, ContentRegistry, InTree, InTreeMatcher, Resolved,
  TransportComponent, Unavailable, check_requirements,
};
use crate::contract::ShapePart;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::sync::Arc;

pub(crate) struct Scope {
  resolved: Vec<Resolved>,
  /// What the declared components contribute to plans: fragments and the actions they use (plan
  /// task 8.4), and the actions the embedding's own matchers run (ADR 0057). Shared, because a verification run and an exchange loop outlive the call that
  /// built the scope.
  contributions: Arc<Contributions>,
  in_tree: Vec<InTree>,
  loaders: Vec<Arc<dyn ComponentLoader>>,
  content: ContentRegistry,
}

impl Scope {
  /// A scope with no components at all: what a session built outside an [`super::Engine`] has.
  #[cfg(test)]
  pub fn none() -> Scope {
    Scope {
      resolved: Vec::new(),
      contributions: Arc::default(),
      in_tree: Vec::new(),
      loaders: Vec::new(),
      content: ContentRegistry::new(),
    }
  }

  /// Load `declarations` through `loaders` and put their content components ahead of `in_tree`'s.
  /// The embedding's own matchers run their actions beside the declared components' (ADR 0057).
  pub fn resolve(
    declarations: &[Value],
    loaders: &[Arc<dyn ComponentLoader>],
    in_tree: &[InTree],
    in_tree_content: &ContentRegistry,
    in_tree_matchers: &[InTreeMatcher],
  ) -> Result<Scope, Unavailable> {
    let resolved = component::resolve(declarations, loaders, in_tree)?;
    for component in &resolved {
      tracing::info!(name = %component.name, version = %component.version, interfaces = ?component.interfaces, "component loaded");
    }
    let content = resolved
      .iter()
      .filter_map(|component| component.content.clone())
      .fold(ContentRegistry::new(), ContentRegistry::with)
      .then(in_tree_content);
    Ok(Scope {
      contributions: Arc::new(Contributions::from_resolved(&resolved, in_tree_matchers)),
      resolved,
      in_tree: in_tree.to_vec(),
      loaders: loaders.to_vec(),
      content,
    })
  }

  /// What decodes and encodes this scope's content slots, or `None` when nothing can.
  pub fn content(&self) -> Option<Arc<dyn ContentComponent>> {
    (!self.content.is_empty()).then(|| Arc::new(self.content.clone()) as Arc<dyn ContentComponent>)
  }

  pub fn contributions(&self) -> Arc<Contributions> {
    Arc::clone(&self.contributions)
  }

  /// The declared transport contributing `kind`, if one does (plan task 8.3). Declared components
  /// come ahead of the embedding's in-tree ones, as they do for content: the project named it.
  pub fn transport(&self, kind: &str) -> Option<Arc<dyn TransportComponent>> {
    self
      .resolved
      .iter()
      .find(|component| component.transport_kinds.iter().any(|k| k == kind))
      .and_then(|component| component.transport.clone())
  }

  /// The components that took part, as a contract's `metadata.writer` or a run's report names them
  /// (spec §10.3: "the engine records the resolved name and version of every component").
  #[allow(dead_code)]
  pub fn components(&self) -> Vec<Value> {
    self
      .resolved
      .iter()
      .map(|c| json!({ "name": c.name, "version": c.version, "interfaces": c.interfaces }))
      .collect()
  }

  /// Everything an interaction needs before it can run (spec §2.3): its requirements, a content
  /// component for every content type it declares (contract-file spec §5.5) that can carry the
  /// slot's shape, and every plan fragment a declared component contributes for its slots (plan
  /// task 8.4). Checked when the interaction arrives, never when its first body does.
  ///
  /// An in-tree component is asked to `compile` the slot's shape here (spec §6.3), as a declared one
  /// is by [`Contributions::check`], so one that claims a whole family of types can refuse a shape
  /// it cannot carry: the built-in `text` component claims `text/*`, and a `text/csv` body of rows
  /// still needs a CSV component (ADR 0036).
  pub fn check<'a>(
    &self,
    requirements: impl IntoIterator<Item = &'a Requirement>,
    content_types: Option<&ContentTypes>,
    parts: &BTreeMap<String, ShapePart>,
  ) -> Result<(), Unavailable> {
    check_requirements(requirements, &self.resolved, &self.in_tree, &self.loaders)?;
    for (part, slots) in content_types.into_iter().flatten() {
      for (slot, media_type) in slots {
        let component = format!("content/{media_type}");
        let Some(content) = self.content.route(media_type) else {
          return Err(Unavailable {
            message: format!(
              "'{part}.{slot}' is declared as '{media_type}', and no loaded content component handles it; declare one in the project's components"
            ),
            details: json!({ "component": component, "content-type": media_type, "part": part, "slot": slot }),
            component,
          });
        };
        if self.contributions.claims(media_type) {
          continue;
        }
        let Some(shape) = parts.get(part).and_then(|slots| slots.get(slot)) else {
          continue;
        };
        content
          .compile(Compile {
            content_type: media_type.clone(),
            shape: shape.clone(),
            path: format!("$.{part}.{slot}"),
          })
          .map_err(|error| Unavailable {
            message: format!(
              "'{part}.{slot}' is declared as '{media_type}', and the in-tree component that handles it cannot carry its shape: {}; declare a component for '{media_type}' in the project's components",
              error.message
            ),
            details: json!({ "component": component, "content-type": media_type, "part": part, "slot": slot,
                             "reason": "compile-failed", "error": error }),
            component,
          })?;
      }
    }
    self.contributions.check(content_types, parts)
  }
}
