//! Component-contributed plan fragments (plan-grammar spec §7.1, component-interfaces spec §6.3 and
//! §12.3; plan task 8.4): the one plan-shaped document that crosses a version boundary, because it
//! is authored by a component shipped separately from the engine that reads it.
//!
//! Three things happen to a fragment before it runs, in this order, and each is a named failure:
//!
//! 1. **its grammar version is read** ([`readable`]) — a fragment that does not say which grammar it
//!    was written against cannot be checked for skew, and is refused rather than guessed at;
//! 2. **it is parsed** as a plan node, under that grammar;
//! 3. **its actions are checked** ([`check_actions`]): core ones must be core actions of the grammar
//!    it declared, component ones must be in the contributing component's own namespace *and*
//!    among the actions its handshake contributed, and every `resolve` must stay inside the slot it
//!    was compiled for.
//!
//! A fragment written against grammar v0 comes out of step 3 in v1's names (ADR 0026 decision 4):
//! the two grammars differ in nothing else, so the rest of the engine — the interpreter, `explain` —
//! only ever sees v1.
//!
//! Then it replaces the generic plan for its slot ([`splice`]). Replaces, not joins: a content
//! component contributes a fragment precisely where the generic plan is wrong for its content type
//! (CSV's `integer` is text that spells one), and a generic `match:integer` left beside it would
//! still fail.

use super::actions::Grammar;
use super::model::{Node, NodeKind, node_from_json};
use serde_json::Value;

/// The grammar versions this engine reads, newest last. A fragment written against any of them — or
/// against an older minor of the same major — is readable (spec §7.1: the grammar evolves
/// additively within a major). `v0` is read by the one-to-one mapping to `v1` (ADR 0026).
pub const READABLE_GRAMMAR_VERSIONS: &[&str] = &["v0", "v1"];

/// Why a fragment cannot be used. `Skew` is a version question; `Invalid` is the component's bug.
#[derive(Debug, Clone, PartialEq)]
pub enum FragmentError {
  Skew {
    declared: Option<String>,
    message: String,
  },
  Invalid(String),
}

/// `v<major>` or `v<major>.<minor>`, as (major, minor). Nothing else is a grammar version.
pub fn parse_version(version: &str) -> Option<(u64, u64)> {
  let digits = version.strip_prefix('v')?;
  let (major, minor) = match digits.split_once('.') {
    Some((major, minor)) => (major, minor),
    None => (digits, "0"),
  };
  Some((major.parse().ok()?, minor.parse().ok()?))
}

/// Whether an engine reading `engine` can read a fragment written against `fragment`: same major,
/// and a minor no newer than the engine's. `v0` fragments on a `v0.1` engine are the designed-for
/// case (the grammar grew); a `v0.1` fragment on a `v0` engine may use what `v0` never had. Another
/// major is another grammar, read only if the engine lists it too ([`READABLE_GRAMMAR_VERSIONS`]).
pub fn readable_by(engine: &str, fragment: &str) -> bool {
  match (parse_version(engine), parse_version(fragment)) {
    (Some((engine_major, engine_minor)), Some((major, minor))) => {
      major == engine_major && minor <= engine_minor
    }
    _ => false,
  }
}

/// Step 1: does any grammar this engine reads cover the version the fragment declared?
pub fn readable(declared: Option<&str>) -> Result<(), FragmentError> {
  let Some(declared) = declared else {
    return Err(FragmentError::Skew {
      declared: None,
      message: format!(
        "the fragment does not say which plan grammar it was written against; this engine reads {}",
        READABLE_GRAMMAR_VERSIONS.join(", ")
      ),
    });
  };
  if READABLE_GRAMMAR_VERSIONS
    .iter()
    .any(|engine| readable_by(engine, declared))
  {
    return Ok(());
  }
  Err(FragmentError::Skew {
    declared: Some(declared.to_string()),
    message: format!(
      "the fragment was written against plan grammar '{declared}', and this engine reads {}",
      READABLE_GRAMMAR_VERSIONS.join(", ")
    ),
  })
}

/// Steps 1–3 for one fragment, contributed by `component` for the slot at `slot_path`.
pub fn validate(
  fragment: &Value,
  declared: Option<&str>,
  component: &str,
  contributed: &[String],
  slot_path: &str,
) -> Result<Node, FragmentError> {
  readable(declared)?;
  let grammar = declared
    .and_then(Grammar::of)
    .expect("a readable version names a grammar this engine has");
  let mut node =
    node_from_json(fragment).map_err(|err| FragmentError::Invalid(format!("not a plan node: {err}")))?;
  check_actions(&mut node, grammar, component, contributed, slot_path)?;
  Ok(node)
}

/// Step 3, over the whole fragment, which was written in `grammar`. Each core action is left under
/// its v1 name.
pub fn check_actions(
  node: &mut Node,
  grammar: Grammar,
  component: &str,
  contributed: &[String],
  slot_path: &str,
) -> Result<(), FragmentError> {
  match &mut node.kind {
    NodeKind::Action { name, children } => {
      if let Some(core) = grammar.core(name) {
        *name = core.to_string();
      } else if grammar.reserves(name) {
        return Err(FragmentError::Invalid(format!(
          "'{name}' is not a core action of the grammar the fragment declared"
        )));
      } else if name.split_once(':').map(|(namespace, _)| namespace) != Some(component) {
        return Err(FragmentError::Invalid(format!(
          "'{name}' is in another component's namespace; a fragment may use core actions and '{component}:' ones"
        )));
      } else if !contributed.iter().any(|action| action == name) {
        return Err(FragmentError::Invalid(format!(
          "'{name}' is not among the actions '{component}' contributed in its handshake"
        )));
      }
      children
        .iter_mut()
        .try_for_each(|child| check_actions(child, grammar, component, contributed, slot_path))
    }
    NodeKind::Resolve { path } => {
      if path == slot_path
        || path.starts_with(&format!("{slot_path}."))
        || path.starts_with(&format!("{slot_path}["))
      {
        Ok(())
      } else {
        Err(FragmentError::Invalid(format!(
          "resolves '{path}', outside the slot '{slot_path}' it was compiled for"
        )))
      }
    }
    NodeKind::Container { children, .. } | NodeKind::Pipeline { children } | NodeKind::Splat { children } => {
      children
        .iter_mut()
        .try_for_each(|child| check_actions(child, grammar, component, contributed, slot_path))
    }
    NodeKind::Value(_) | NodeKind::ResolveCurrent { .. } | NodeKind::Annotation { .. } => Ok(()),
  }
}

/// A whole plan written in grammar v0, renamed to v1 in place (ADR 0026 decision 4). A name v0 did
/// not define is left as it is: it is a component's, or it is unknown, and the interpreter says
/// which when it reaches it.
pub fn rename_from_v0(node: &mut Node) {
  match &mut node.kind {
    NodeKind::Action { name, children } => {
      if let Some(core) = Grammar::V0.core(name) {
        *name = core.to_string();
      }
      children.iter_mut().for_each(rename_from_v0);
    }
    NodeKind::Container { children, .. } | NodeKind::Pipeline { children } | NodeKind::Splat { children } => {
      children.iter_mut().for_each(rename_from_v0);
    }
    NodeKind::Value(_)
    | NodeKind::Resolve { .. }
    | NodeKind::ResolveCurrent { .. }
    | NodeKind::Annotation { .. } => {}
  }
}

/// Replace the generic plan for `part`.`slot` with `fragment`. The slot's own container, and its
/// label, stay: `explain` still prints `:body` above whatever the component compiled. Returns
/// whether the slot was found.
pub fn splice(root: &mut Node, part: &str, slot: &str, fragment: Node) -> bool {
  let NodeKind::Container { children, .. } = &mut root.kind else {
    return false;
  };
  for child in children.iter_mut() {
    if let NodeKind::Container {
      label: Some(label),
      children: slots,
    } = &mut child.kind
      && label == part
    {
      for slot_node in slots.iter_mut() {
        if let NodeKind::Container {
          label: Some(name),
          children: contents,
        } = &mut slot_node.kind
          && name == slot
        {
          *contents = vec![fragment];
          return true;
        }
      }
    }
    if splice(child, part, slot, fragment.clone()) {
      return true;
    }
  }
  false
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json::json;

  #[test]
  fn a_fragment_reads_on_its_own_grammar_and_any_later_minor_but_not_an_earlier_or_another_major() {
    assert!(readable_by("v0", "v0"));
    assert!(
      readable_by("v0.1", "v0"),
      "the designed-for case: the grammar grew"
    );
    assert!(readable_by("v0.1", "v0.1"));
    assert!(!readable_by("v0", "v0.1"), "it may use what v0 never had");
    assert!(!readable_by("v1", "v0"), "another major is another grammar");
    assert!(!readable_by("v0", "v1"));
    assert!(readable_by("v1", "v1"));
    assert!(!readable_by("v1", "v1.1"));
    assert!(!readable_by("v0", "0"), "not a grammar version");
    assert!(!readable_by("v0", "v0.x"));
  }

  #[test]
  fn an_undeclared_version_is_refused_rather_than_guessed() {
    assert!(matches!(
      readable(None),
      Err(FragmentError::Skew { declared: None, .. })
    ));
    assert_eq!(readable(Some("v0")), Ok(()));
    assert_eq!(readable(Some("v1")), Ok(()), "both majors are read");
    for newer in ["v0.1", "v1.1", "v2"] {
      let skew = readable(Some(newer)).unwrap_err();
      assert!(
        matches!(&skew, FragmentError::Skew { declared: Some(v), .. } if v == newer),
        "{skew:?}"
      );
    }
  }

  fn check_in(grammar: &str, action: &str) -> Result<Node, FragmentError> {
    validate(
      &json!({ "kind": "action", "name": action,
               "children": [ { "kind": "resolve", "path": "$.response.body" } ] }),
      Some(grammar),
      "csv",
      &["csv:integer".to_string()],
      "$.response.body",
    )
  }

  fn check(action: &str) -> Result<Node, FragmentError> {
    check_in("v1", action)
  }

  fn name_of(node: Result<Node, FragmentError>) -> String {
    match node.expect("a valid fragment").kind {
      NodeKind::Action { name, .. } => name,
      other => panic!("not an action: {other:?}"),
    }
  }

  #[test]
  fn a_fragment_may_use_core_actions_and_its_own_contributed_ones_and_nothing_else() {
    assert!(check("expect:array").is_ok());
    assert!(check("flow:for-each").is_ok());
    assert!(check("csv:integer").is_ok());
    assert!(matches!(check("csv:decimal"), Err(FragmentError::Invalid(m)) if m.contains("not among")));
    assert!(matches!(check("json:parse"), Err(FragmentError::Invalid(m)) if m.contains("another component")));
    // A reserved namespace, and a name the engine does not have in it: from a newer grammar, or a
    // typo. The fragment declared v1, so it is a typo — or a component that declared the wrong
    // version.
    assert!(
      matches!(check("expect:unique"), Err(FragmentError::Invalid(m)) if m.contains("not a core action"))
    );
    // v1 has no unnamespaced actions, so this is nobody's.
    assert!(matches!(check("for-each"), Err(FragmentError::Invalid(m)) if m.contains("another component")));
  }

  #[test]
  fn a_v0_fragment_is_read_by_the_one_to_one_mapping_and_comes_out_in_v1_names() {
    assert_eq!(name_of(check_in("v0", "for-each")), "flow:for-each");
    assert_eq!(name_of(check_in("v0", "upper-case")), "value:upper-case");
    assert_eq!(name_of(check_in("v0", "match:min-type")), "legacy:min-type");
    assert_eq!(name_of(check_in("v0", "expect:array")), "expect:array");
    assert_eq!(name_of(check_in("v0", "csv:integer")), "csv:integer");
    // A fragment is held to the grammar it declared, not to the newest one the engine reads.
    assert!(
      matches!(check_in("v0", "flow:for-each"), Err(FragmentError::Invalid(m)) if m.contains("another component")),
      "v0 has no flow namespace"
    );
    assert!(
      matches!(check_in("v0", "expect:unique"), Err(FragmentError::Invalid(m)) if m.contains("not a core action"))
    );
    assert!(
      matches!(check_in("v1", "match:min-type"), Err(FragmentError::Invalid(m)) if m.contains("not a core action")),
      "in v1 it is legacy:min-type"
    );
  }

  #[test]
  fn a_fragment_stays_inside_its_slot() {
    let outside = validate(
      &json!({ "kind": "resolve", "path": "$.request.headers" }),
      Some("v1"),
      "csv",
      &[],
      "$.response.body",
    );
    assert!(matches!(outside, Err(FragmentError::Invalid(m)) if m.contains("outside the slot")));
    let prefix_only = validate(
      &json!({ "kind": "resolve", "path": "$.response.bodyguard" }),
      Some("v1"),
      "csv",
      &[],
      "$.response.body",
    );
    assert!(prefix_only.is_err(), "a shared prefix is not the same slot");
  }
}
