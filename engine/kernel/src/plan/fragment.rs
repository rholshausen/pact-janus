//! Component-contributed plan fragments (plan-grammar spec §7.1, component-interfaces spec §6.3 and
//! §12.3; plan task 8.4): the one plan-shaped document that crosses a version boundary, because it
//! is authored by a component shipped separately from the engine that reads it.
//!
//! Three things happen to a fragment before it runs, in this order, and each is a named failure:
//!
//! 1. **its grammar version is read** ([`readable`]) — a fragment that does not say which grammar it
//!    was written against cannot be checked for skew, and is refused rather than guessed at;
//! 2. **it is parsed** as a plan node, under that grammar;
//! 3. **its actions are checked** ([`check_actions`]): core ones must be core actions this engine
//!    has, component ones must be in the contributing component's own namespace *and* among the
//!    actions its handshake contributed, and every `resolve` must stay inside the slot it was
//!    compiled for.
//!
//! Then it replaces the generic plan for its slot ([`splice`]). Replaces, not joins: a content
//! component contributes a fragment precisely where the generic plan is wrong for its content type
//! (CSV's `integer` is text that spells one), and a generic `match:integer` left beside it would
//! still fail.

use super::model::{Node, NodeKind, node_from_json};
use crate::component::CORE_FAMILIES;
use serde_json::Value;

/// The grammar versions this engine reads, newest last. A fragment written against any of them — or
/// against an older minor of the same major — is readable (spec §7.1: the grammar evolves
/// additively within a major).
pub const READABLE_GRAMMAR_VERSIONS: &[&str] = &["v0"];

/// Every core action this engine implements (spec §4.2–§4.4). A fragment that names a core action
/// outside this list was written against a newer grammar, or is wrong — its declared version says
/// which.
pub const CORE_ACTIONS: &[&str] = &[
  "and",
  "or",
  "if",
  "error",
  "apply",
  "for-each",
  "tee",
  "join",
  "join-with",
  "length",
  "lower-case",
  "upper-case",
  "to-string",
  "expect:object",
  "expect:array",
  "expect:empty",
  "expect:not-empty",
  "expect:count",
  "expect:size",
  "expect:entries",
  "expect:only-entries",
  "expect:absent",
  "check:exists",
  "check:equals",
  "check:null",
  "match:any",
  "match:equality",
  "match:type",
  "match:string",
  "match:number",
  "match:integer",
  "match:decimal",
  "match:boolean",
  "match:null",
  "match:regex",
  "match:datetime",
  "match:date",
  "match:time",
  "match:include",
  "match:content-type",
  "match:semver",
  "match:any-of",
  "match:contains",
  "match:array-contains",
  "match:min-type",
  "match:max-type",
  "match:min-max-type",
  "match:header-value",
];

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
/// case (the grammar grew); a `v0.1` fragment on a `v0` engine may use what `v0` never had; a `v1`
/// fragment is a different grammar.
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
  let node =
    node_from_json(fragment).map_err(|err| FragmentError::Invalid(format!("not a plan node: {err}")))?;
  check_actions(&node, component, contributed, slot_path)?;
  Ok(node)
}

/// Step 3, over the whole fragment.
pub fn check_actions(
  node: &Node,
  component: &str,
  contributed: &[String],
  slot_path: &str,
) -> Result<(), FragmentError> {
  match &node.kind {
    NodeKind::Action { name, children } => {
      match name.split_once(':') {
        Some((namespace, _)) if !CORE_FAMILIES.contains(&namespace) => {
          if namespace != component {
            return Err(FragmentError::Invalid(format!(
              "'{name}' is in another component's namespace; a fragment may use core actions and '{component}:' ones"
            )));
          }
          if !contributed.iter().any(|action| action == name) {
            return Err(FragmentError::Invalid(format!(
              "'{name}' is not among the actions '{component}' contributed in its handshake"
            )));
          }
        }
        _ if !CORE_ACTIONS.contains(&name.as_str()) => {
          return Err(FragmentError::Invalid(format!(
            "'{name}' is not a core action of the grammar the fragment declared"
          )));
        }
        _ => {}
      }
      children
        .iter()
        .try_for_each(|child| check_actions(child, component, contributed, slot_path))
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
        .iter()
        .try_for_each(|child| check_actions(child, component, contributed, slot_path))
    }
    NodeKind::Value(_) | NodeKind::ResolveCurrent { .. } | NodeKind::Annotation { .. } => Ok(()),
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
    let skew = readable(Some("v0.1")).unwrap_err();
    assert!(
      matches!(&skew, FragmentError::Skew { declared: Some(v), .. } if v == "v0.1"),
      "{skew:?}"
    );
  }

  fn check(action: &str) -> Result<Node, FragmentError> {
    validate(
      &json!({ "kind": "action", "name": action,
               "children": [ { "kind": "resolve", "path": "$.response.body" } ] }),
      Some("v0"),
      "csv",
      &["csv:integer".to_string()],
      "$.response.body",
    )
  }

  #[test]
  fn a_fragment_may_use_core_actions_and_its_own_contributed_ones_and_nothing_else() {
    assert!(check("expect:array").is_ok());
    assert!(check("csv:integer").is_ok());
    assert!(matches!(check("csv:decimal"), Err(FragmentError::Invalid(m)) if m.contains("not among")));
    assert!(matches!(check("json:parse"), Err(FragmentError::Invalid(m)) if m.contains("another component")));
    // A core-looking name the engine does not have: from a newer grammar, or a typo. The fragment
    // declared v0, so it is a typo — or a component that declared the wrong version.
    assert!(
      matches!(check("expect:unique"), Err(FragmentError::Invalid(m)) if m.contains("not a core action"))
    );
  }

  #[test]
  fn a_fragment_stays_inside_its_slot() {
    let outside = validate(
      &json!({ "kind": "resolve", "path": "$.request.headers" }),
      Some("v0"),
      "csv",
      &[],
      "$.response.body",
    );
    assert!(matches!(outside, Err(FragmentError::Invalid(m)) if m.contains("outside the slot")));
    let prefix_only = validate(
      &json!({ "kind": "resolve", "path": "$.response.bodyguard" }),
      Some("v0"),
      "csv",
      &[],
      "$.response.body",
    );
    assert!(prefix_only.is_err(), "a shared prefix is not the same slot");
  }
}
