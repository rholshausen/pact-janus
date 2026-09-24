# 0024 — Keep request dimensions pinned, and match a cardinality point as a region

- **Status**: accepted
- **Date**: 2026-09-24
- **Plan tasks**: 9.2 (amends design 2.2 §6.4 and §7.1, design 2.3 §4.1 and §5.2)
- **Evidence**: [Phase 9 findings](../phase-9-findings.md) 25 (and 2, which it leaves open),
  [performance report](../performance-report.md) §5.4, [ADR 0008](0008-deterministic-pairwise-variant-sampling.md),
  corpus case `corpora/shapes/cardinality-pinned-region`

## Context

Task 9.1 wrote the baseline's 100 KB request as an `each-like` of orders, and the mock answered `500`
under `base`: "Expected exactly 1 item(s) but got 310". Every cardinality point pinned the array to one
exact length — `min`, `min + 1`, `max` — and the mock pins a request to the armed variant, so a consumer
whose request carries a list it sizes from its own data (a basket, a batch, a page of ids) had no way to
be accepted under any variant. Finding 25 put three options: stop pinning request dimensions; give
`each-like` an opt-out from its dimension; or add a second every-element operator with no dimension.

The first option is the one that looks cheapest, and it breaks the RFC. The RFC excludes request shapes
from the subsumption check on one argument — "the reverse direction (consumer requests) is already
covered by variant replay" — and design 2.8 §2.1 repeats it. Replay is by example (design 2.3 §5.2): the
verifier sends the requests the consumer actually sent. If request dimensions were not pinned, a request
shape could declare an `optional` field nobody ever sent absent, and no provider would ever be verified
against its absence. The honesty rule would hold for responses and quietly lapse for requests.

The other two options are loopholes the author opts into: correct, and a way to declare width without
demonstrating it.

What the failing case actually shows is narrower. `min+1` is the point for "more than the minimum". It was
matched as *exactly* `min + 1`, which no party with its own data will send — and the same exactness made
a verifier fail a provider that answered "more than one item" with five.

## Decision

1. **Request dimensions stay pinned.** Under a variant the mock accepts only requests that variant admits,
   so a request width is demonstrated point by point, and the RFC's reverse-direction argument stays
   true.
2. **A cardinality point is produced as one length and matched as a region.** `min` and `max` are exactly
   themselves. `min+1` is produced as `min + 1` elements, as before, and matched as every length strictly
   between its neighbours: above `min`, and below `max` when a `max` point exists. The rule applies on both
   sides of a contract test — the mock matching a request, and the verifier matching a response — because
   in both, the length is the other party's data.
3. **Point names and variant ids do not change.** `min+1` keeps its name (shape spec §6.2 makes ids
   stable), and what the engine produces for it is unchanged, so no recorded contract changes meaning.
4. **The default of `each-like`'s `min` stays 1** (shape spec §5.5). Whether an array admits `[]` by
   default (finding 2) is a question about what the DSL means, which this ADR does not settle; `janus
   upgrade` now reports the difference from v1–v4 instead of hiding it.

## Alternatives considered

- **Request dimensions do not pin** (finding 25 option A). Loses the only thing that verifies a provider
  against a request width the consumer declared, and with it the reason request shapes are excluded from
  subsumption.
- **`vary: false` on `each-like`** (option B). A declared width with no demonstration, available on any
  list, request or response. The region rule makes the common case work without it; an author who truly
  wants "any length, never demonstrated" still has `type` on the array, at the cost of saying nothing
  about its elements.
- **A second every-element operator with no dimension** (option C). The same loophole with a new name in
  the core vocabulary, which is forever (shape spec §3.5).

## Consequences

Easier: a consumer whose list length is its own data is accepted under `min+1` with that list; a provider
asked for "more than one" may answer with as many as it has, so its state setup no longer has to produce
exactly two.

Harder: an interaction with request dimensions needs a variant-*parameterised* closure — it reads the
variant to decide what to send — and it must still demonstrate `min` once. That is a cost the RFC's
drawbacks already name in general ("closures must be variant-agnostic or variant-parameterised"); it is
now stated for requests in particular.

Committed to: a pinned operator narrows to the set of values its point stands for, and "produced as" and
"matched as" may differ for any operator whose point stands for more than one value. Today that is
cardinality only.

**Tripwire.** Revisit if consumers routinely write `type` on request arrays to escape the minimal case (the
region did not go far enough, and option B's explicit opt-out has earned its place); or if a provider's
`min+1` verification passes where a consumer later breaks on a length inside the region (the region is
too wide, and the representative should be matched more tightly after all).
