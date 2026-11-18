---
engine: minor
---

# A subsumption check no longer tries every provider-shape entry for every interaction

Staged plan S4.11, ADR 0096. Selecting the entry an interaction is checked against ran every entry's
selector plan, so a check cost interactions × entries. Each entry's selector now fixes constraints,
an `equality`'s value and a prefix a component operator declares, and an entry one of them certainly
refuses is passed over before its plan runs. The entries selected, and so every report, are the same.
A 50-interaction check against a 400-entry shape costs each interaction 269 µs where it cost 1,358 µs.

- The matcher interface has an optional `matcher/prefix` (`{ operator, node }` → `{ prefix? }`,
  component-interfaces spec §7.6), and `MatcherComponent::prefix` its native binding. The built-in HTTP
  component answers it for `http:path`.
- `subsumption/check` reads a provider shape from the frame once, rather than writing it out and
  parsing it twice. An invalid shape's error no longer gives a line and column, which pointed into
  that text.
- `pact_janus_kernel::subsumption::provider_shape_of` reads a provider shape already parsed.
