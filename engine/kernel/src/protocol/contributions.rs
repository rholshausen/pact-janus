//! What a scope's components contribute to a plan beyond decoding: the actions a content component
//! substitutes for value operators under its content type (component-interfaces spec §6.3, ADR 0100),
//! and the component actions plans name, run by `matcher/apply` (§7.1).
//!
//! A slot's substitution comes from the declared component that decodes it, from the handshake it
//! loaded with, so it was checked when the component loaded. The engine compiles every slot itself,
//! pinning included, and only the value operators' actions are the component's. When an interaction
//! arrives, that component is asked `content/compile` for its slots, which lets it refuse a shape it
//! cannot carry (`compile-failed`). An in-tree component is asked the same by the scope.
//!
//! An in-tree matcher's actions are run as a declared one's are (ADR 0057): the built-in HTTP
//! component's `http:status-class` is one.

use crate::common::ContentTypes;
use crate::component::{
  Apply, Compile, ContentComponent, ContentSubstitutions, InTreeMatcher, MatcherComponent, Resolved,
  Unavailable, media_type_matches,
};
use crate::contract::ShapePart;
use crate::interaction_spec::InteractionSpec;
use crate::plan::{
  self, ActionApplier, Assignment, NodeResult, Plan, RuntimeValue, SlotSubstitutions, Substitution,
};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::sync::Arc;

struct Contributor {
  name: String,
  content: Option<Arc<dyn ContentComponent>>,
  matcher: Option<Arc<dyn MatcherComponent>>,
  actions: Vec<String>,
  substitutions: Vec<ContentSubstitutions>,
}

#[derive(Default)]
pub(crate) struct Contributions {
  contributors: Vec<Contributor>,
}

impl Contributions {
  /// The declared components, then the embedding's own matchers. A name is unique across both
  /// (spec §2.4), so the order decides nothing.
  pub fn from_resolved(resolved: &[Resolved], in_tree: &[InTreeMatcher]) -> Contributions {
    let declared = resolved.iter().map(|component| Contributor {
      name: component.name.clone(),
      content: component.content.clone(),
      matcher: component.matcher.clone(),
      actions: component.actions.clone(),
      substitutions: component.substitutions.clone(),
    });
    let in_tree = in_tree.iter().map(|component| Contributor {
      name: component.name.clone(),
      content: None,
      matcher: Some(Arc::clone(&component.matcher)),
      actions: component.actions.clone(),
      substitutions: Vec::new(),
    });
    Contributions {
      contributors: declared.chain(in_tree).collect(),
    }
  }

  /// The declared component that decodes `media_type`: the first that handles it, as the scope's
  /// registry routes it.
  fn decoder(&self, media_type: &str) -> Option<&Contributor> {
    self.contributors.iter().find(|c| {
      c.content
        .as_ref()
        .is_some_and(|content| content.handles(media_type))
    })
  }

  /// Whether a declared component decodes `media_type`, and so is asked to compile its slots here.
  pub fn claims(&self, media_type: &str) -> bool {
    self.decoder(media_type).is_some()
  }

  /// Checked when an interaction arrives, beside its requirements: each declared slot a declared
  /// component decodes is offered to it with its shape, and a refusal is `component-unavailable`
  /// with `reason: compile-failed`, naming the component (spec §6.3).
  pub fn check(
    &self,
    content_types: Option<&ContentTypes>,
    parts: &BTreeMap<String, ShapePart>,
  ) -> Result<(), Unavailable> {
    for (part, slots) in content_types.into_iter().flatten() {
      for (slot, media_type) in slots {
        let Some(contributor) = self.decoder(media_type) else {
          continue;
        };
        let Some(shape) = parts.get(part).and_then(|slots| slots.get(slot)) else {
          continue;
        };
        let content = contributor.content.as_ref().expect("found by its content");
        content
          .compile(Compile {
            content_type: media_type.clone(),
            shape: shape.clone(),
            path: format!("$.{part}.{slot}"),
          })
          .map_err(|error| Unavailable {
            component: contributor.name.clone(),
            message: format!(
              "component '{}' cannot carry the shape of '{part}.{slot}' as '{media_type}': {}",
              contributor.name, error.message
            ),
            details: json!({ "component": contributor.name, "reason": "compile-failed",
                             "slot": format!("{part}.{slot}"), "error": error }),
          })?;
      }
    }
    Ok(())
  }

  /// Each declared slot's substitution (ADR 0100): what its decoding component's first
  /// `content-types` entry that matches the slot's type substitutes.
  fn substitutions(&self, content_types: Option<&ContentTypes>) -> SlotSubstitutions {
    let mut substitutions = SlotSubstitutions::new();
    for (part, slots) in content_types.into_iter().flatten() {
      for (slot, media_type) in slots {
        let Some(contributor) = self.decoder(media_type) else {
          continue;
        };
        let Some(entry) = contributor
          .substitutions
          .iter()
          .find(|entry| media_type_matches(&entry.media_type, media_type))
        else {
          continue;
        };
        substitutions.insert(
          (part.clone(), slot.clone()),
          Substitution {
            component: contributor.name.clone(),
            content_type: media_type.clone(),
            operators: entry.operators.clone(),
          },
        );
      }
    }
    substitutions
  }

  /// The plan for `spec` under `assignment`, each declared slot's value operators compiled to its
  /// content component's actions where it substitutes them.
  pub fn compile(&self, spec: &InteractionSpec, assignment: &Assignment, variant: Option<&str>) -> Plan {
    plan::compile_substituted(
      spec,
      assignment,
      variant,
      &self.substitutions(spec.content_types.as_ref()),
    )
  }
}

/// What runs `matchers`' actions when a plan reaches one, as a run's scope does (ADR 0057): for a
/// caller that executes plans outside a session, such as the golden corpus.
pub fn in_tree_actions(matchers: &[InTreeMatcher]) -> impl crate::subsumption::OperatorComparer + use<> {
  Contributions::from_resolved(&[], matchers)
}

/// [`in_tree_actions`], with `content` decoding what a selector names (ADR 0090 decision 2): what
/// `subsumption/check` runs a selector with.
pub fn check_components<'a>(
  matchers: &[InTreeMatcher],
  content: &'a dyn ContentComponent,
) -> impl crate::subsumption::OperatorComparer + use<'a> {
  CheckComponents {
    operators: Contributions::from_resolved(&[], matchers),
    content,
  }
}

struct CheckComponents<'a> {
  operators: Contributions,
  content: &'a dyn ContentComponent,
}

impl ActionApplier for CheckComponents<'_> {
  fn apply(&self, action: &str, arguments: &[RuntimeValue], path: Option<&str>) -> Option<plan::NodeResult> {
    self.operators.apply(action, arguments, path)
  }
}

impl crate::subsumption::OperatorComparer for CheckComponents<'_> {
  fn comparability(&self, operator: &str) -> crate::component::Comparability {
    self.operators.comparability(operator)
  }

  fn compare(
    &self,
    operator: &str,
    provider: &serde_json::Value,
    consumer: &serde_json::Value,
  ) -> Option<(crate::subsumption::Verdict, Option<String>)> {
    self.operators.compare(operator, provider, consumer)
  }

  fn prefix(&self, operator: &str, node: &serde_json::Value) -> Option<String> {
    self.operators.prefix(operator, node)
  }

  fn content(&self) -> Option<&dyn ContentComponent> {
    Some(self.content)
  }
}

/// What the subsumption walk asks about a component's operators (component-interfaces spec §7.5,
/// ADR 0087): the matcher of the component whose namespace the operator is in.
impl crate::subsumption::OperatorComparer for Contributions {
  fn comparability(&self, operator: &str) -> crate::component::Comparability {
    self
      .matcher_for(operator)
      .map(|matcher| matcher.comparability(operator))
      .unwrap_or(crate::component::Comparability::Opaque)
  }

  fn compare(
    &self,
    operator: &str,
    provider: &serde_json::Value,
    consumer: &serde_json::Value,
  ) -> Option<(crate::subsumption::Verdict, Option<String>)> {
    use crate::subsumption::Verdict;
    let result = self
      .matcher_for(operator)?
      .compare(crate::component::Compare {
        operator: operator.to_string(),
        provider: provider.clone(),
        consumer: consumer.clone(),
      })
      .ok()?;
    let verdict = match result.verdict.as_str() {
      "yes" => Verdict::Yes,
      "no" => Verdict::No,
      // An answer this engine does not know is no answer: the walk must not guess.
      _ => Verdict::Unknown,
    };
    Some((verdict, result.reason))
  }

  fn prefix(&self, operator: &str, node: &serde_json::Value) -> Option<String> {
    self
      .matcher_for(operator)?
      .prefix(crate::component::Prefix {
        operator: operator.to_string(),
        node: node.clone(),
      })
      .ok()?
      .prefix
  }
}

impl Contributions {
  /// The matcher of the component that contributes `operator`, by its namespace.
  fn matcher_for(&self, operator: &str) -> Option<&Arc<dyn MatcherComponent>> {
    let (namespace, _) = operator.split_once(':')?;
    self
      .contributors
      .iter()
      .find(|contributor| {
        contributor.name == namespace && contributor.actions.iter().any(|action| action == operator)
      })?
      .matcher
      .as_ref()
  }
}

/// `matcher/apply` for the component actions a plan names (spec §7.1, §7.2).
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
        marks: None,
      });
    };
    // One application. Its first argument is the value under test, wrapped as a `MatchValue` —
    // `content`, tagged `base64` when it is bytes, and the path it came from. Any further arguments
    // travel as `config.arguments`, in order (spec §7.2, ADR 0100 decision 4).
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
          marks: None,
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
        marks: None,
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
      marks: None,
    },
    _ => NodeResult::Error {
      message: format!("not a plan result document: {result}"),
      path: None,
      marks: None,
    },
  }
}
