# 0009 — Bind provider-state parameters to variants in a separate member, and treat an unproducible state as a failure

- **Status**: proposed
- **Date**: 2026-08-27
- **Plan tasks**: 2.3 (feeds 5.2, 5.3, 2.5, 2.7)
- **Evidence**: [Pact MkII RFC](https://github.com/pact-foundation/roadmap/pull/146) ("Provider
  verification", unresolved question "provider-state/variant linkage (`whenVariant` above is a
  sketch)"), [ADR 0007](0007-shapes-denote-value-sets.md) (no in-band markers in user data),
  [ADR 0008](0008-deterministic-pairwise-variant-sampling.md) (the sample is the contract)

## Context

Variant testing on the provider side needs the provider to actually be in the state each variant
describes: replaying the `shippedAt`-absent variant against an order the fixture shipped proves nothing.
The RFC's answer is one line — `given('an order exists', { shipped: whenVariant('shippedAt', 'present') })`
— and it marks it a sketch. Three things are undecided in it: where the binding lives in a document,
what it evaluates to, and how `'shippedAt'` becomes a dimension id that a thin SDK cannot compute.

Underneath those sits the question the plan calls out explicitly: what happens when a variant needs a
state the provider *cannot* produce. That is not an edge case — it is the normal consequence of a
consumer declaring a width wider than the provider's reality, which is exactly the situation the RFC
designs subsumption to surface.

## Decision

**Variant-bound state parameters are declared in their own member, resolve by a total function of the
assignment, and a state the provider cannot reach fails the run.**

Four commitments:

1. **A separate member, never a wrapped value.** Bindings live in `variant-params`, a sibling of the
   state's literal parameters. A state parameter's value is user data, so "a value that has a
   `when-variant` member means this is a binding" is ambiguous against a payload that legitimately has
   one — the same trap ADR 0007 refused for shapes, with the same answer.
2. **Cases, with the RFC's predicate as DSL sugar.** The canonical document maps points to values,
   because the useful parameter is often not boolean ("the order's status is `SHIPPED`"). `whenVariant(d, p)`
   expands to the two-case boolean form in the SDK. One canonical document, sugar in the DSLs.
3. **The engine resolves dimension references; pacts record them resolved.** `shippedAt`,
   `body.shippedAt` and `response.body.shippedAt#presence` all resolve to the same dimension, uniquely
   or not at all — a reference matching nothing, or several things, is `interaction-invalid` at
   `add-interaction`, with the candidates listed. A thin SDK cannot expand a short reference because it
   does not compute variant spaces; the engine can, and doing it there is what turns a typo into a
   message instead of a state parameter that silently never fires. A binding on a **gated** dimension
   must declare a `default`, because such a dimension is inactive in whole classes of variants by
   construction.
4. **`state-unavailable` is a failure, waivable only per variant and with a reason.** When a
   `state-setup` hook answers `unsupported`, the variant is reported `state-unavailable` — a status
   distinct from `failed`, because the remedy is a contract change rather than a code change, and a
   summary that conflates them sends the reader to the wrong team. It fails the run, because the
   consumer demonstrated it can *handle* the variant and nothing has demonstrated the provider can
   *produce* it; reporting that as a pass is the false confidence this whole mechanism exists to remove.
   The fix is an exclusion with a reason (ADR 0008) or a narrower shape. Because that loop can block a
   provider team on a consumer release, the waiver is specified rather than improvised: scoped to a
   named interaction and variant, carrying a required reason, reported as *waived* and not as passing.
   There is deliberately no global switch.

Spec text: [Variant semantics and sampling specification](../specs/variant-semantics/spec.md) §6;
schema `variant-params.schema.json`. The encoding inside a pact file and an interaction specification is
design 2.5's to ratify; the semantics are 2.3's.

## Alternatives considered

- **The RFC's inline form as the wire format** (`{ shipped: whenVariant(...) }`). Ambiguous against user
  data, and it puts a function call in a document that has to be readable years later.
- **Boolean predicates only.** Simpler, and it forces authors to encode "status is SHIPPED" as three
  booleans the state handler has to reassemble.
- **Let the SDK expand short dimension references.** It would need the variant space, which means the
  SDK computing dimensions from shapes — matching logic in the SDK, which the architecture forbids.
- **Treat `state-unavailable` as a skip or a warning.** The comfortable choice, and it makes a variant
  that nobody can produce look like one that passed.
- **A global `--ignore-state-failures`.** The flag that gets set once during an incident and outlives
  everyone who understood why.
- **Derive the states rather than record them.** Resolution is total, so a verifier could recompute the
  parameters from the assignment. Recording them anyway costs a few bytes and makes a pact file readable
  by a tool that does not implement §6.4 — which, in five years, is most tools.

## Consequences

Easier: a variant that needs a specific fixture says so declaratively, so `state-setup` scripts stop
being written per-combination; the failure that used to be "the provider returned PENDING" is now "the
provider cannot be put into this state", which is the finding the RFC's subsumption loop wants; and
validation at submission catches the typo class (`'shipedAt'`, `'SHIPED'`, a gated binding with no
default) while the author is still looking at their DSL.

Harder: providers must be able to reach every state the consumer's *sampled* variants need, which is a
real new obligation — an order service that cannot produce a `SHIPPED` order without a timestamp now has
to say so, and someone has to write the exclusion; state setup runs per variant, so a six-variant
interaction is six setup calls (grouping consecutive equal states helps less than it sounds, because the
sampler exists to vary dimensions together); and design 2.5 inherits a new member in the provider-state
document.

Committed to: `variant-params` as a separate member; resolution as a total function of assignment and
binding, computed identically on both sides; resolved dimension ids in recorded artifacts; and
`state-unavailable` as a first-class status that no global option can turn green.

**Tripwire** — revisit if any of these show up in Phase 5: waivers outnumber exclusions (teams are
routing around the failure rather than fixing the contract); state setup dominates verification wall
time (the grouping rule needs to become a scheduling rule, or states need to be declarable as
variant-independent); or authors regularly want a state parameter that depends on *two* dimensions,
which this design does not express and which would argue for a small, still-declarative combinator.
