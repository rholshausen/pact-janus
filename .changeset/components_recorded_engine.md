---
engine: minor
---

# A contract and a verification summary name the declared components that were loaded

Staged plan S5.1a, ADR 0097. A consumer session that declared components writes them into its
contract as `metadata.components`, and a verification run that declared components lists them in its
summary as `components`. Each entry gives the `name` and `version` the component's handshake declared
and its `source`: the declaration's `kind`, an `oci` source's `reference` without its digest, and the
`digest` a declaration would pin. That is the resolved manifest digest for a registry, a tag's included,
and the SHA-256 of the bytes for a `file` source, whose path is never written. A session or run that
declared no components writes exactly what it did before, and the engine's own version is not recorded.

- A contract merged into one already written (`finalise`'s `write`) lists every component either
  writer loaded, once each, sorted by name, then version, then source.
- `pact_janus_kernel::component::Loaded` has a `fetched` member, where a loader says where it found
  the bytes, and `Resolved` a `source`. A loader that sets `fetched: None` is recorded by its source's
  kind alone.
- `pact_janus_kernel::contract::Metadata` has `components`, a list of `RecordedComponent`.
