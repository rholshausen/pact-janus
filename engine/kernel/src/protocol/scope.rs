//! A resolution scope (component-interfaces spec §2.4, §10): the components one consumer session
//! or one verification run can call — the ones its `components` declared, loaded when it started,
//! ahead of the ones compiled into the embedding.
//!
//! Scoped per session rather than per engine because spec §3.4 says sessions are the only
//! resource and that rule "extends inward": a component a session loaded ends with it, and two
//! sessions on one pipe may declare different ones.

use crate::common::{ContentTypes, Requirement};
use crate::component::{
  self, ComponentLoader, ContentComponent, ContentRegistry, InTree, Resolved, Unavailable, check_requirements,
};
use serde_json::{Value, json};
use std::sync::Arc;

pub(crate) struct Scope {
  resolved: Vec<Resolved>,
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
      in_tree: Vec::new(),
      loaders: Vec::new(),
      content: ContentRegistry::new(),
    }
  }

  /// Load `declarations` through `loaders` and put their content components ahead of `in_tree`'s.
  pub fn resolve(
    declarations: &[Value],
    loaders: &[Arc<dyn ComponentLoader>],
    in_tree: &[InTree],
    in_tree_content: &ContentRegistry,
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

  /// Everything an interaction needs before it can run (spec §2.3): its requirements, and a content
  /// component for every content type it declares (contract-file spec §5.5). Checked when the
  /// interaction arrives, never when its first body does.
  pub fn check<'a>(
    &self,
    requirements: impl IntoIterator<Item = &'a Requirement>,
    content_types: Option<&ContentTypes>,
  ) -> Result<(), Unavailable> {
    check_requirements(requirements, &self.resolved, &self.in_tree, &self.loaders)?;
    for (part, slots) in content_types.into_iter().flatten() {
      for (slot, media_type) in slots {
        if self.content.route(media_type).is_none() {
          let component = format!("content/{media_type}");
          return Err(Unavailable {
            message: format!(
              "'{part}.{slot}' is declared as '{media_type}', and no loaded content component handles it; declare one in the project's components"
            ),
            details: json!({ "component": component, "content-type": media_type, "part": part, "slot": slot }),
            component,
          });
        }
      }
    }
    Ok(())
  }
}
