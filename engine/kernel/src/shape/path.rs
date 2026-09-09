//! Dimension-id path syntax (shape-language spec §6.2): `<path>#<facet>`, built depth-first as a
//! shape tree is walked. One set of builders here is the single source of truth for both
//! consumers — [`super::variant_space`], which emits the ids, and the plan compiler
//! ([`crate::plan`]), which looks them up to pin a variant (plan-grammar spec §5.1) — because a
//! dimension id computed one way and looked up another would silently stop pinning from working.

/// A part's slot, the root every path in that slot starts from: `"<part>.<slot>"`.
pub fn root(part: &str, slot: &str) -> String {
  format!("{part}.{slot}")
}

/// An `object` member (spec §6.2): `.<member>`.
pub fn member(base: &str, name: &str) -> String {
  format!("{base}.{name}")
}

/// `each-like`'s `items` (spec §6.2): `[*]`.
pub fn each_like_item(base: &str) -> String {
  format!("{base}[*]")
}

/// `each-entry`'s `values` (spec §6.2): `{*}`.
pub fn each_entry_value(base: &str) -> String {
  format!("{base}{{*}}")
}

/// `each-entry`'s `keys` (spec §6.2): `{key}`.
pub fn each_entry_key(base: &str) -> String {
  format!("{base}{{key}}")
}

/// The `i`th entry of an `array` (spec §6.2): `[i]`.
pub fn array_index(base: &str, index: usize) -> String {
  format!("{base}[{index}]")
}

/// The segment where a `one-of` alternative is entered (spec §6.2): `@<alternative>`.
pub fn alternative(base: &str, name: &str) -> String {
  format!("{base}@{name}")
}

/// `<path>#<facet>` (spec §6.2).
pub fn dimension_id(path: &str, facet: &str) -> String {
  format!("{path}#{facet}")
}
