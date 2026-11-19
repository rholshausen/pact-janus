//! Plan grammar versions (plan-grammar spec §7.1, ADR 0022 decision 2): how a plan document says
//! which grammar it was written against, and which of those this engine reads.
//!
//! A plan document the engine reads back, as the benchmark harness hands one to `janus-engine.wasm`,
//! is read under the grammar it declares or not at all. A grammar version is `v<major>` or `v<major>.<minor>`, and
//! a reader reads its own major at any minor no newer than its own. Grammar v0 is read by ADR 0026's
//! one-to-one renaming into v1, so the rest of the engine only ever sees v1.
//!
//! Components wrote plan fragments against these versions until ADR 0100, which withdrew them: a
//! component now says what a value operator means for its content type, and the engine compiles every
//! plan itself.

use super::actions::Grammar;
use super::model::{Node, NodeKind};

/// The grammar versions this engine reads, newest last. A plan written against any of them, or
/// against an older minor of the same major, is readable (spec §7.1: the grammar evolves additively
/// within a major). `v0` is read by the one-to-one mapping to `v1` (ADR 0026).
pub const READABLE_GRAMMAR_VERSIONS: &[&str] = &["v0", "v1"];

/// `v<major>` or `v<major>.<minor>`, as (major, minor). Nothing else is a grammar version.
pub fn parse_version(version: &str) -> Option<(u64, u64)> {
  let digits = version.strip_prefix('v')?;
  let (major, minor) = match digits.split_once('.') {
    Some((major, minor)) => (major, minor),
    None => (digits, "0"),
  };
  Some((major.parse().ok()?, minor.parse().ok()?))
}

/// Whether an engine reading `engine` can read a document written against `document`: same major,
/// and a minor no newer than the engine's. Another major is another grammar, read only if the engine
/// lists it too ([`READABLE_GRAMMAR_VERSIONS`]).
pub fn readable_by(engine: &str, document: &str) -> bool {
  match (parse_version(engine), parse_version(document)) {
    (Some((engine_major, engine_minor)), Some((major, minor))) => {
      major == engine_major && minor <= engine_minor
    }
    _ => false,
  }
}

/// Whether any grammar this engine reads covers `declared`, or why not.
pub fn readable(declared: &str) -> Result<(), String> {
  if READABLE_GRAMMAR_VERSIONS
    .iter()
    .any(|engine| readable_by(engine, declared))
  {
    return Ok(());
  }
  Err(format!(
    "the plan was written against plan grammar '{declared}', and this engine reads {}",
    READABLE_GRAMMAR_VERSIONS.join(", ")
  ))
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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn a_plan_reads_on_its_own_grammar_and_any_later_minor_but_not_an_earlier_or_another_major() {
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
  fn this_engine_reads_v0_and_v1_and_nothing_newer() {
    assert_eq!(readable("v0"), Ok(()));
    assert_eq!(readable("v1"), Ok(()), "both majors are read");
    for newer in ["v0.1", "v1.1", "v2"] {
      let refused = readable(newer).unwrap_err();
      assert!(refused.contains(&format!("'{newer}'")), "{refused}");
    }
  }

  #[test]
  fn a_v0_plan_comes_out_in_v1_names() {
    let mut node = Node::action(
      "for-each",
      vec![
        Node::action("upper-case", vec![]),
        Node::action("match:min-type", vec![]),
        Node::action("expect:array", vec![]),
        Node::action("csv:integer", vec![]),
      ],
    );
    rename_from_v0(&mut node);
    let NodeKind::Action { name, children } = &node.kind else {
      panic!("an action");
    };
    assert_eq!(name, "flow:for-each");
    let names: Vec<&str> = children
      .iter()
      .map(|child| match &child.kind {
        NodeKind::Action { name, .. } => name.as_str(),
        _ => "",
      })
      .collect();
    assert_eq!(
      names,
      [
        "value:upper-case",
        "legacy:min-type",
        "expect:array",
        "csv:integer"
      ]
    );
  }
}
