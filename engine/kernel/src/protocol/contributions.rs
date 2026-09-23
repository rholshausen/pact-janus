//! What a scope's declared components contribute to a plan beyond decoding (plan task 8.4): a
//! fragment for each content slot they handle, compiled by `content/compile` (component-interfaces
//! spec §6.3), and the component actions those fragments use, run by `matcher/apply` (§7.1).
//!
//! Fragments are asked for, and checked, when an interaction arrives ([`Contributions::check`]) —
//! the same moment its requirements are — so a component whose fragment is written against a grammar
//! this engine cannot read fails the interaction by name, not the fortieth variant of a run. Plans are
//! then spliced wherever they are compiled ([`Contributions::compile`]).
//!
//! Only declared components are asked. The in-tree JSON component contributes no fragment, so
//! asking it would change nothing; a built-in that did contribute one would need asking too, and
//! would be the first time "no privileged path" (spec §9.1) cut the other way.

use crate::common::ContentTypes;
use crate::component::{Apply, Compile, ContentComponent, MatcherComponent, Resolved, Unavailable};
use crate::contract::ShapePart;
use crate::interaction_spec::InteractionSpec;
use crate::plan::fragment::{self, FragmentError, READABLE_GRAMMAR_VERSIONS};
use crate::plan::{self, ActionApplier, Assignment, Node, NodeResult, Plan, RuntimeValue};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::sync::Arc;

struct Contributor {
  name: String,
  content: Option<Arc<dyn ContentComponent>>,
  matcher: Option<Arc<dyn MatcherComponent>>,
  actions: Vec<String>,
}

#[derive(Default)]
pub(crate) struct Contributions {
  contributors: Vec<Contributor>,
}

/// One validated fragment, and where it goes.
struct Fragment {
  part: String,
  slot: String,
  node: Node,
}

impl Contributions {
  pub fn from_resolved(resolved: &[Resolved]) -> Contributions {
    Contributions {
      contributors: resolved
        .iter()
        .map(|component| Contributor {
          name: component.name.clone(),
          content: component.content.clone(),
          matcher: component.matcher.clone(),
          actions: component.actions.clone(),
        })
        .collect(),
    }
  }

  /// Every fragment an interaction's declared content slots get, checked — or the first reason one
  /// cannot be used, as `component-unavailable` naming the component (spec §12.3: "fails the load
  /// naming the skew").
  fn fragments(
    &self,
    content_types: Option<&ContentTypes>,
    parts: &BTreeMap<String, ShapePart>,
  ) -> Result<Vec<Fragment>, Unavailable> {
    let mut fragments = Vec::new();
    for (part, slots) in content_types.into_iter().flatten() {
      for (slot, media_type) in slots {
        let Some(contributor) = self.contributors.iter().find(|c| {
          c.content
            .as_ref()
            .is_some_and(|content| content.handles(media_type))
        }) else {
          continue;
        };
        let Some(shape) = parts.get(part).and_then(|slots| slots.get(slot)) else {
          continue;
        };
        let path = format!("$.{part}.{slot}");
        let content = contributor.content.as_ref().expect("found by its content");
        let compiled = content
          .compile(Compile {
            content_type: media_type.clone(),
            shape: shape.clone(),
            path: path.clone(),
          })
          .map_err(|error| Unavailable {
            component: contributor.name.clone(),
            message: format!(
              "component '{}' could not compile '{part}.{slot}': {}",
              contributor.name, error.message
            ),
            details: json!({ "component": contributor.name, "reason": "compile-failed", "error": error }),
          })?;
        let Some(document) = compiled.fragment else {
          continue;
        };
        let node = fragment::validate(
          &document,
          compiled.grammar_version.as_deref(),
          &contributor.name,
          &contributor.actions,
          &path,
        )
        .map_err(|error| refused(&contributor.name, part, slot, error))?;
        tracing::debug!(component = %contributor.name, %part, %slot, "plan fragment contributed");
        fragments.push(Fragment {
          part: part.clone(),
          slot: slot.clone(),
          node,
        });
      }
    }
    Ok(fragments)
  }

  /// Checked when an interaction arrives, beside its requirements.
  pub fn check(
    &self,
    content_types: Option<&ContentTypes>,
    parts: &BTreeMap<String, ShapePart>,
  ) -> Result<(), Unavailable> {
    self.fragments(content_types, parts).map(|_| ())
  }

  /// The plan for `spec` under `assignment`, with every contributed fragment spliced in. A fragment
  /// that stopped being usable since [`Self::check`] — a component answering differently the second
  /// time — leaves the generic plan, loudly.
  pub fn compile(
    &self,
    spec: &InteractionSpec,
    parts: &BTreeMap<String, ShapePart>,
    assignment: &Assignment,
    variant: Option<&str>,
  ) -> Plan {
    let mut compiled = plan::compile(spec, assignment, variant);
    match self.fragments(spec.content_types.as_ref(), parts) {
      Ok(fragments) => {
        for fragment in fragments {
          if !fragment::splice(&mut compiled.root, &fragment.part, &fragment.slot, fragment.node) {
            tracing::warn!(part = %fragment.part, slot = %fragment.slot, "no slot container to splice a fragment into");
          }
        }
      }
      Err(unavailable) => {
        tracing::warn!(component = %unavailable.component, message = %unavailable.message, "fragment refused at compile; using the generic plan");
      }
    }
    compiled
  }
}

fn refused(component: &str, part: &str, slot: &str, error: FragmentError) -> Unavailable {
  let (reason, message, declared) = match error {
    FragmentError::Skew { declared, message } => ("grammar-skew", message, declared),
    FragmentError::Invalid(message) => ("fragment-invalid", message, None),
  };
  Unavailable {
    component: component.to_string(),
    message: format!(
      "component '{component}' contributed a plan fragment for '{part}.{slot}' this engine cannot use: {message}"
    ),
    details: json!({ "component": component, "reason": reason, "slot": format!("{part}.{slot}"),
                     "declared": declared, "readable": READABLE_GRAMMAR_VERSIONS }),
  }
}

/// `matcher/apply` for a fragment's component actions (spec §7.1).
impl ActionApplier for Contributions {
  fn apply(&self, action: &str, arguments: &[RuntimeValue], path: Option<&str>) -> Option<NodeResult> {
    let (namespace, _) = action.split_once(':')?;
    let contributor = self
      .contributors
      .iter()
      .find(|c| c.name == namespace && c.actions.iter().any(|a| a == action))?;
    let Some(matcher) = &contributor.matcher else {
      return Some(NodeResult::Error {
        message: format!(
          "'{}' contributes '{action}' but does not implement the matcher interface that runs it",
          contributor.name
        ),
        path: None,
      });
    };
    // One application. Its first argument is the value under test, wrapped as a `MatchValue` —
    // `content`, tagged `base64` when it is bytes, and the path it came from. The grammar gives an
    // action no `config`, so any further arguments travel as `config.arguments`: the one place
    // Apply's `config` can come from (plan task 8.4, Phase 9 finding 24).
    let (first, rest) = match arguments.split_first() {
      Some((first, rest)) => (Some(first), rest),
      None => (None, &[][..]),
    };
    let mut value = match first {
      Some(RuntimeValue::Bytes(bytes)) => {
        json!({ "content": RuntimeValue::Bytes(bytes.clone()).to_json(), "encoded": "base64" })
      }
      Some(first) => json!({ "content": first.to_json() }),
      None => json!({ "content": null }),
    };
    if let Some(path) = path {
      value["path"] = json!(path);
    }
    let config = (!rest.is_empty())
      .then(|| json!({ "arguments": rest.iter().map(RuntimeValue::to_json).collect::<Vec<_>>() }));
    let applied = matcher.apply(Apply {
      action: action.to_string(),
      config,
      values: vec![value],
    });
    Some(match applied {
      Ok(applied) => match applied.results.first() {
        Some(result) => read_result(result),
        None => NodeResult::Error {
          message: format!("'{}' answered '{action}' with no result", contributor.name),
          path: None,
        },
      },
      // Spec §11.3 says this is `component-failed` at the engine boundary. A plan node has nowhere
      // to put an engine error, so it lands as the node's error, labelled — Phase 9 finding 14's
      // gap, reached from the other side.
      Err(error) => NodeResult::Error {
        message: format!(
          "component '{}' failed running '{action}' ({}): {}",
          contributor.name, error.code, error.message
        ),
        path: None,
      },
    })
  }
}

/// A plan result document (plan-grammar spec §2.3): `ok`, `value` with a plan value, or `error`.
fn read_result(result: &Value) -> NodeResult {
  match result.get("status").and_then(Value::as_str) {
    Some("ok") => NodeResult::Ok,
    Some("value") => NodeResult::Value(RuntimeValue::from_json(
      result.pointer("/value/value").unwrap_or(&Value::Null),
    )),
    Some("error") => NodeResult::Error {
      message: result
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or("the component's action failed")
        .to_string(),
      path: result.get("path").and_then(Value::as_str).map(str::to_string),
    },
    _ => NodeResult::Error {
      message: format!("not a plan result document: {result}"),
      path: None,
    },
  }
}
