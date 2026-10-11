//! The names of the core action set (plan-grammar spec §4, ADR 0026): which namespaces are the
//! specification's, which actions are in them, and what each one was called in grammar v0.
//!
//! Grammar v1 is v0 renamed and nothing else. Every action is `namespace:name`, the core families
//! are reserved namespaces rather than conventions, and the actions only the v1–v4 compiler may
//! emit are in `legacy`, so that can be read off the name. An engine that reads v1 reads a v0
//! document by the one-to-one mapping this table is ([`from_v0`]) — which is what keeps a component
//! built against v0 working without a rebuild.

/// The namespaces reserved for the specification (spec §4.1). A component may not declare one, and
/// an action in one is a core action or it is nothing: the engine never resolves it to a component.
pub const RESERVED_NAMESPACES: &[&str] = &["flow", "value", "expect", "check", "match", "legacy"];

/// What v0 treated as core families: a `:` prefix that was a convention, not a scope. `convert`
/// never had an action; v0 reserved the name and v1 does not.
const V0_FAMILIES: &[&str] = &["match", "expect", "check", "convert"];

/// Every core action this engine implements (spec §4.2–§4.4), as (v1 name, v0 name).
const CORE: &[(&str, &str)] = &[
  ("flow:and", "and"),
  ("flow:or", "or"),
  ("flow:if", "if"),
  ("flow:error", "error"),
  ("flow:apply", "apply"),
  ("flow:for-each", "for-each"),
  ("flow:tee", "tee"),
  ("value:join", "join"),
  ("value:join-with", "join-with"),
  ("value:length", "length"),
  ("value:lower-case", "lower-case"),
  ("value:upper-case", "upper-case"),
  ("value:to-string", "to-string"),
  ("expect:object", "expect:object"),
  ("expect:array", "expect:array"),
  ("expect:empty", "expect:empty"),
  ("expect:not-empty", "expect:not-empty"),
  ("expect:count", "expect:count"),
  ("expect:size", "expect:size"),
  ("expect:entries", "expect:entries"),
  ("expect:absent", "expect:absent"),
  ("check:exists", "check:exists"),
  ("check:equals", "check:equals"),
  ("check:null", "check:null"),
  ("match:any", "match:any"),
  ("match:equality", "match:equality"),
  ("match:type", "match:type"),
  ("match:string", "match:string"),
  ("match:number", "match:number"),
  ("match:integer", "match:integer"),
  ("match:decimal", "match:decimal"),
  ("match:boolean", "match:boolean"),
  ("match:null", "match:null"),
  ("match:regex", "match:regex"),
  ("match:datetime", "match:datetime"),
  ("match:date", "match:date"),
  ("match:time", "match:time"),
  ("match:include", "match:include"),
  ("match:content-type", "match:content-type"),
  ("match:semver", "match:semver"),
  ("match:any-of", "match:any-of"),
  ("match:contains", "match:contains"),
  ("legacy:only-entries", "expect:only-entries"),
  ("legacy:array-contains", "match:array-contains"),
  ("legacy:min-type", "match:min-type"),
  ("legacy:max-type", "match:max-type"),
  ("legacy:min-max-type", "match:min-max-type"),
  ("legacy:header-value", "match:header-value"),
];

/// The grammar a document's action names are read under. The two differ only in names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grammar {
  V0,
  V1,
}

impl Grammar {
  /// The grammar a declared version (`v0`, `v0.1`, `v1`) names, when this engine reads its major.
  pub fn of(version: &str) -> Option<Grammar> {
    match super::fragment::parse_version(version)?.0 {
      0 => Some(Grammar::V0),
      1 => Some(Grammar::V1),
      _ => None,
    }
  }

  /// The core action `name` is in this grammar, by its v1 name — which is what the interpreter
  /// dispatches on and what `explain` prints, whichever grammar the document was written in.
  pub fn core(self, name: &str) -> Option<&'static str> {
    CORE
      .iter()
      .find(|(v1, v0)| match self {
        Grammar::V0 => *v0 == name,
        Grammar::V1 => *v1 == name,
      })
      .map(|(v1, _)| *v1)
  }

  /// Whether `name` is the specification's to define in this grammar, implemented here or not: in
  /// v1 an action in a reserved namespace, in v0 an unnamespaced action or one in a core family.
  pub fn reserves(self, name: &str) -> bool {
    match (self, name.split_once(':')) {
      (Grammar::V0, None) => true,
      (Grammar::V0, Some((family, _))) => V0_FAMILIES.contains(&family),
      (Grammar::V1, None) => false,
      (Grammar::V1, Some((namespace, _))) => RESERVED_NAMESPACES.contains(&namespace),
    }
  }
}

/// The v1 name of a v0 core action (ADR 0026 decision 4). `None` for a name v0 did not define.
pub fn from_v0(name: &str) -> Option<&'static str> {
  Grammar::V0.core(name)
}

/// Whether `name` is a core action this engine implements, by its v1 name.
pub fn is_core(name: &str) -> bool {
  Grammar::V1.core(name).is_some()
}

/// Whether `name` is a component's action: in a namespace the specification has not reserved.
/// Every action is namespaced (spec §4.1), so a name with no `:` is neither core nor a component's.
pub fn is_component_action(name: &str) -> bool {
  name
    .split_once(':')
    .is_some_and(|(namespace, _)| !RESERVED_NAMESPACES.contains(&namespace))
}

/// Whether only the v1–v4 compiler may emit `name` (spec §4.5): in v1, exactly the `legacy`
/// namespace.
pub fn is_legacy_only(name: &str) -> bool {
  name.starts_with("legacy:")
}

#[cfg(test)]
mod tests {
  use super::*;
  use pretty_assertions::assert_eq;
  use rstest::rstest;
  use std::collections::BTreeSet;

  #[test]
  fn every_core_action_is_in_a_reserved_namespace_with_exactly_one_colon() {
    for (v1, _) in CORE {
      let (namespace, name) = v1
        .split_once(':')
        .unwrap_or_else(|| panic!("'{v1}' has no namespace"));
      assert!(
        RESERVED_NAMESPACES.contains(&namespace),
        "'{v1}' is not in a reserved namespace"
      );
      assert!(
        !name.is_empty() && !name.contains(':'),
        "'{v1}' is not namespace:name"
      );
    }
  }

  #[test]
  fn the_v0_mapping_is_one_to_one() {
    let v1: BTreeSet<&str> = CORE.iter().map(|(v1, _)| *v1).collect();
    let v0: BTreeSet<&str> = CORE.iter().map(|(_, v0)| *v0).collect();
    assert_eq!(v1.len(), CORE.len(), "a v1 name appears twice");
    assert_eq!(v0.len(), CORE.len(), "a v0 name appears twice");
  }

  /// ADR 0026's table, and the two families that had no prefix.
  #[rstest]
  #[case::control("if", "flow:if")]
  #[case::control_with_a_hyphen("for-each", "flow:for-each")]
  #[case::value("upper-case", "value:upper-case")]
  #[case::unchanged("match:regex", "match:regex")]
  #[case::unchanged_assertion("expect:absent", "expect:absent")]
  #[case::closed_objects("expect:only-entries", "legacy:only-entries")]
  #[case::array_contains("match:array-contains", "legacy:array-contains")]
  #[case::min_type("match:min-type", "legacy:min-type")]
  #[case::max_type("match:max-type", "legacy:max-type")]
  #[case::min_max_type("match:min-max-type", "legacy:min-max-type")]
  #[case::header_value("match:header-value", "legacy:header-value")]
  fn a_v0_name_reads_as_its_v1_name(#[case] v0: &str, #[case] v1: &str) {
    assert_eq!(from_v0(v0), Some(v1));
  }

  #[test]
  fn a_name_is_core_in_the_grammar_that_spells_it_that_way_and_not_in_the_other() {
    assert_eq!(Grammar::V1.core("flow:if"), Some("flow:if"));
    assert_eq!(Grammar::V1.core("if"), None, "v1 has no unnamespaced actions");
    assert_eq!(Grammar::V1.core("match:min-type"), None, "it moved to legacy");
    assert_eq!(Grammar::V0.core("flow:if"), None, "v0 never had the namespace");
    assert_eq!(Grammar::V0.core("legacy:min-type"), None);
  }

  #[test]
  fn what_a_grammar_reserves_is_wider_than_what_it_defines() {
    assert!(
      Grammar::V1.reserves("expect:unique"),
      "reserved, though no such action exists"
    );
    assert!(Grammar::V1.reserves("legacy:anything"));
    assert!(!Grammar::V1.reserves("csv:integer"));
    assert!(
      !Grammar::V1.reserves("convert:anything"),
      "v1 does not reserve convert"
    );
    assert!(
      Grammar::V0.reserves("unique"),
      "an unnamespaced name was the specification's"
    );
    assert!(Grammar::V0.reserves("convert:anything"));
    assert!(
      !Grammar::V0.reserves("flow:if"),
      "in v0 that is a component called flow"
    );
  }

  #[test]
  fn a_component_action_is_one_in_a_namespace_nobody_reserved() {
    assert!(is_component_action("csv:integer"));
    assert!(!is_component_action("match:integer"));
    assert!(!is_component_action("legacy:header-value"));
    assert!(!is_component_action("if"), "not namespaced, so nobody's");
  }

  #[rstest]
  #[case("v0", Some(Grammar::V0))]
  #[case("v0.1", Some(Grammar::V0))]
  #[case("v1", Some(Grammar::V1))]
  #[case("v2", None)]
  #[case("1", None)]
  fn a_declared_version_names_a_grammar(#[case] version: &str, #[case] expected: Option<Grammar>) {
    assert_eq!(Grammar::of(version), expected);
  }
}
