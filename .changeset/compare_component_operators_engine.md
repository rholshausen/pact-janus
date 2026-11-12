---
engine: major
---

# The subsumption check decides the HTTP component's operators

Staged plan S4.7, ADR 0087. `janus check` and `subsumption/check` compared every component operator as
opaque: `yes` when the two nodes were identical, a review otherwise. Now a component may declare an
operator's comparability, and the walk asks it:

- **A finite provider** (`equality`, `any-of`) against an operator declared `exact` or `conservative`
  is decided by that operator's own matcher: a recorded `Content-Type` of `application/json` against an
  upgraded pact's `http:media-type` is `yes`, where it was a review on every interaction.
- **Anything wider** is the component's `matcher/compare`. The HTTP component declares
  `http:status-class`, `http:media-type` and `http:list` `exact` and `http:path` `conservative`, and
  decides two status classes by their statuses, two media types by whether the consumer's admits the
  provider's type, and two lists by their elements.

A comparison that was a review can now be a decided finding, a `wider-values`, which `on-finding`
governs rather than `on-review`: a check that warned under `on-finding: block` may now block. The
native binding's `MatcherComponent` has `comparability` and `compare`, defaulting to opaque, so an
embedding's own matcher compiles unchanged. A declared component's operators are still opaque to
`janus check` (S5.7).

The conformance case `live/recorded-shape` records the sample provider's 404 as the provider sends it,
`{"error":"not-found"}`, and pins the shape that writes.
