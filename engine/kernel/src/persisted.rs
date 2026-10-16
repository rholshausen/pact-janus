//! Persisted identifiers (staged plan S0.5, ADR 0031): the names the engine writes into documents
//! that outlive it — contracts in a broker, provider shapes beside a build, components in an OCI
//! registry — and reads back from them. They carry the codename, and a rename cannot simply replace
//! them, because a document recorded before the rename still says the old one.
//!
//! So each is spelled once, here, as what it **writes**, and read under that spelling and every
//! **earlier** one: a renamed engine writes the new name and reads both, for one major version
//! (`Documentation/codename.md`). The earlier spellings live in `persisted/earlier.rs`, which the
//! rename writes and never renames. A new persisted identifier is added here in the form the rename
//! reads — `pub const NAME: Persisted = Persisted::new("…", earlier::NAME);`, however rustfmt wraps
//! it — with an empty `NAME` list in `earlier.rs`, and an entry in [`ALL`].

mod earlier;

/// One persisted identifier: the spelling written, and the spellings also read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Persisted {
  writes: &'static str,
  earlier: &'static [&'static str],
}

impl Persisted {
  pub const fn new(writes: &'static str, earlier: &'static [&'static str]) -> Self {
    Persisted { writes, earlier }
  }

  /// The spelling written.
  pub const fn writes(&self) -> &'static str {
    self.writes
  }

  /// The spellings read but no longer written.
  pub const fn earlier(&self) -> &'static [&'static str] {
    self.earlier
  }

  /// Whether `found` is this identifier, under any spelling.
  pub fn reads(&self, found: &str) -> bool {
    found == self.writes || self.earlier.contains(&found)
  }

  /// For a versioned format, `<name>/<major>`: whether `found` names this format at any major and
  /// under any spelling — a document of this kind, though perhaps one this engine cannot read.
  pub fn names_family(&self, found: &str) -> bool {
    let family = |spelling: &str| spelling.rsplit_once('/').map(|(name, _)| format!("{name}/"));
    std::iter::once(self.writes)
      .chain(self.earlier.iter().copied())
      .filter_map(family)
      .any(|prefix| found.starts_with(&prefix))
  }
}

/// A contract's `$format` (contract-file spec §2.3, ADR 0011).
pub const CONTRACT_FORMAT: Persisted = Persisted::new("janus-contract/1", earlier::CONTRACT_FORMAT);
/// A provider shape's `$format` (subsumption-check spec §4).
pub const PROVIDER_SHAPE_FORMAT: Persisted =
  Persisted::new("janus-provider-shape/1", earlier::PROVIDER_SHAPE_FORMAT);
/// An OCI component artifact's `artifactType` (ADR 0021).
pub const COMPONENT_ARTIFACT_TYPE: Persisted = Persisted::new(
  "application/vnd.pact.janus.component.v1",
  earlier::COMPONENT_ARTIFACT_TYPE,
);
/// An OCI component artifact's config media type (ADR 0021).
pub const COMPONENT_CONFIG_MEDIA_TYPE: Persisted = Persisted::new(
  "application/vnd.pact.janus.component.config.v1+json",
  earlier::COMPONENT_CONFIG_MEDIA_TYPE,
);

/// The interface a component's world exports (`wit/component.wit`, ADR 0013): compiled into every
/// component, including the ones already in a registry.
pub const COMPONENT_INTERFACE: Persisted =
  Persisted::new("pact:janus-component/pipe@1.0.0", earlier::COMPONENT_INTERFACE);

/// The global a scripted hook calls the hook library by (lifecycle-hooks spec §9, ADR 0015):
/// written into every script in a provider's repository.
pub const HOOK_SCRIPT_GLOBAL: Persisted = Persisted::new("janus", earlier::HOOK_SCRIPT_GLOBAL);
/// An http hook's `format` for the hook context as the body (lifecycle-hooks spec §8): written into
/// a provider's `verifier.janus.yaml`.
pub const HOOK_BODY_FORMAT: Persisted = Persisted::new("janus", earlier::HOOK_BODY_FORMAT);

/// Every persisted identifier, by name, for whatever must visit them all.
pub const ALL: [(&str, Persisted); 7] = [
  ("CONTRACT_FORMAT", CONTRACT_FORMAT),
  ("PROVIDER_SHAPE_FORMAT", PROVIDER_SHAPE_FORMAT),
  ("COMPONENT_ARTIFACT_TYPE", COMPONENT_ARTIFACT_TYPE),
  ("COMPONENT_CONFIG_MEDIA_TYPE", COMPONENT_CONFIG_MEDIA_TYPE),
  ("COMPONENT_INTERFACE", COMPONENT_INTERFACE),
  ("HOOK_SCRIPT_GLOBAL", HOOK_SCRIPT_GLOBAL),
  ("HOOK_BODY_FORMAT", HOOK_BODY_FORMAT),
];

#[cfg(test)]
mod tests {
  use super::*;

  const RENAMED: Persisted = Persisted::new("next-contract/1", &["janus-contract/1"]);

  #[test]
  fn reads_what_it_writes_and_every_earlier_spelling() {
    assert!(RENAMED.reads("next-contract/1"));
    assert!(RENAMED.reads("janus-contract/1"));
    assert!(!RENAMED.reads("next-contract/2"));
    assert!(!RENAMED.reads("other-contract/1"));
  }

  #[test]
  fn knows_its_family_at_any_major_under_any_spelling() {
    assert!(RENAMED.names_family("next-contract/2"));
    assert!(RENAMED.names_family("janus-contract/7"));
    assert!(!RENAMED.names_family("next-contracts/1"));
    assert!(!RENAMED.names_family("pact/4.0"));
  }

  #[test]
  fn every_identifier_is_listed_once() {
    let mut names: Vec<_> = ALL.iter().map(|(name, _)| *name).collect();
    names.dedup();
    assert_eq!(names.len(), ALL.len());
    for (_, persisted) in ALL {
      assert!(!persisted.earlier().contains(&persisted.writes()));
    }
  }
}
