# 0016 — Subsumption defaults to warn, not block; exemptions require a reason and are scoped by field, interaction or consumer

- **Status**: proposed
- **Date**: 2026-09-04
- **Plan tasks**: 2.8 (feeds 7.3, 7.4)
- **Evidence**: [Pact MkII RFC](https://github.com/pact-foundation/roadmap/pull/146) (drawback "Subsumption
  findings can overwhelm"; unresolved question "Subsumption policy"), [subsumption-check
  spec](../specs/subsumption-check/spec.md) §7, [ADR 0008](0008-deterministic-pairwise-variant-sampling.md)
  (the `Exclusion.reason` precedent this decision reuses)

## Context

The RFC leaves this open by name: "should a failed check warn or block `can-i-deploy` by default, and how
do teams scope exemptions (per field, per interaction, per consumer)?" It also flags, as a drawback rather
than a footnote, that provider shapes derived from types tend to overstate the real response space —
"every field nullable in the ORM ≠ every field absent in practice" — and that a strict policy "would drown
teams in findings and teach them to rubber-stamp."

Those two facts bound the decision together. A default that blocks deployment the first time a team
imports an OpenAPI-derived provider shape teaches the wrong lesson before the mechanism has earned any
trust; a default that never blocks anything is not a policy, it is a report nobody has to read. And
because the subsumption walk itself already answers `unknown` for a real category of comparisons (shape
spec §8's conservative and opaque classes) rather than guessing, a policy that cannot tell a decided `no`
from an honest "review this" would waste that distinction the moment it reached a human.

## Decision

**Both axes — a decided incompatibility (`finding`) and an undecidable one (`review`) — default to
`warn`. Blocking is opt-in per axis. An exemption is scoped by any combination of consumer, interaction
and field, and always carries a mandatory `reason`.**

Three commitments:

1. **`on-finding` and `on-review` default to `warn`, independently configurable to `block`.** Splitting
   the two axes rather than one `warn|block` toggle matters because they are different kinds of evidence:
   `finding` is `admits(P) ⊄ admits(C)`, decided; `review` is "the checker cannot say," never a guess
   (shape spec §8). A team confident enough in its provider shapes' provenance to block on decided
   incompatibilities may still reasonably never want conservative-class regex/datetime disagreements to
   block a deploy, since narrowing those is expensive and the checker itself says it does not know.
   Collapsing the two into one control would force that team to choose between blocking on noise or never
   blocking on a real regression.
2. **Exemptions compose three independent, optional selectors — consumer, interaction, field — and accumulate
   across configuration layers rather than replacing.** This is the exact scoping the RFC names, and it
   reuses ADR 0008's `Exclusion` shape (a selector plus a reason) rather than inventing a second pattern
   for "a team is accepting a gap on purpose," because a reviewer who has already learned to read one
   accumulating, reason-carrying list should not have to learn a second one that behaves differently.
3. **`reason` is required on every exemption; `expires` is offered but not required.** Required because an
   exemption that could be silent is indistinguishable from a checker bug from the outside — the whole
   value of the mechanism is that "why is this exempted" has a one-line answer in the file, not in
   someone's memory. Not required, because forcing a date onto an exemption for a gap that is permanent by
   design (a field the provider will never narrow) would make authors write a meaningless one just to
   satisfy the schema, which is worse than an honestly absent one a dashboard can flag instead.

`provenance` (recorded / derived / authored / observed) is deliberately **not** a policy selector yet.
Task 7.3 is the evidence-gathering step for exactly the RFC's over-broadness worry, on a realistic
OpenAPI-derived spec; adding a provenance-scoped policy control ahead of that evidence would shape the
surface around a guess rather than a measurement. The three selectors above are sufficient to hand-scope
the same outcome (exempt everything from one derived-shape interaction) while 7.3 runs, and a `provenance`
selector is the natural, additive next step once it reports back.

Spec text: [Subsumption check specification](../specs/subsumption-check/spec.md) §7; schema
`subsumption-policy.schema.json`.

## Alternatives considered

- **Block by default.** Rejected on the RFC's own drawback: a mechanism that blocks deploys on its first
  run, before a team has had the chance to widen its consumer shapes or tune provenance sourcing, is a
  mechanism teams disable rather than fix. Warn-first is explicitly named in the RFC's own text as what
  "teams adopting incrementally will want."
- **One `warn|block` toggle for both findings and reviews.** Rejected because it cannot express "block on
  what I've decided is wrong, but never on what the checker has honestly said it cannot decide" — a
  distinction shape spec §8 built into the walk specifically so it would be usable downstream.
- **Expression-based exemptions** (a boolean predicate over path/consumer/interaction). Rejected for the
  same reason ADR 0008 rejected expression-based exclusions: a document that must be evaluated to be
  understood cannot be reviewed, and a policy file is read far more often than it is written.
- **Required `expires` on every exemption.** Rejected because some exempted gaps are permanent by
  authorial intent, not oversight, and a mandatory date on those either lies (a far-future placeholder) or
  gets bumped forever, which teaches the same rubber-stamping habit the RFC's drawback warns against — just
  moved from findings to exemption reviews.
- **A single global `warn`/`block` with no per-scope override.** Rejected because the RFC names three
  scopes explicitly (field, interaction, consumer) as the granularity teams need, and a project big enough
  to run subsumption checks at all is big enough to have one interaction it isn't ready to hold to the
  global default while the rest of the fleet is.

## Consequences

Easier: a team can turn subsumption on without it ever blocking anything until they choose to raise
`on-finding` or `on-review`; a noisy `derived` provider shape is contained by exempting the interactions or
fields it actually gets wrong, without touching the policy's defaults; an exemption's reason is always
readable in the same file the policy lives in, so a security or compliance reviewer does not have to
reconstruct intent from chat history.

Harder: two knobs (`on-finding`, `on-review`) instead of one means two things to explain to a team turning
this on for the first time; exemptions that never expire can accumulate invisibly unless task 7.4/7.5's
tooling actually surfaces missing `expires` values, which is a promise this ADR makes to later work rather
than something it enforces itself; and the deferred `provenance` selector means a team drowning in
`derived`-shape noise today has to hand-scope exemptions per interaction rather than exempting a whole
provenance route in one line, until 7.3 lands.

**Tripwire** — revisit if task 7.3's evidence shows `derived` shapes are noisy enough that per-interaction
exemption scoping is unworkable in practice (argues for the deferred `provenance` selector sooner rather
than later); teams routinely set `on-review` to `block` and then complain about conservative-class noise
(argues the conservative class needs narrowing in shape spec §8, not a policy change here); or exemptions
without `expires` turn out to dominate real policy files (argues for making it required after all, with
the "permanent by design" case handled by a distinct `expires: null` marker rather than absence).
