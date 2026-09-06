# 0010 — Treat plans as renderings and the grammar as the record; version them accordingly

- **Status**: accepted
- **Date**: 2026-08-27
- **Plan tasks**: 2.4 (feeds 3.3, 3.4, 3.6, 3.7, 8.4)
- **Evidence**: [Pact MkII RFC](https://github.com/pact-foundation/roadmap/pull/146) ("Seeing what
  matching will do"; plan-grammar stability listed as an implementation unknown),
  [ADR 0004](0004-fork-v2-engine-as-kernel.md) (the forked v2 engine),
  [ADR 0007](0007-shapes-denote-value-sets.md) (operators frozen once published),
  [ADR 0008](0008-deterministic-pairwise-variant-sampling.md) (the sample is recorded, so it is frozen)

## Context

The RFC lists plan-grammar stability as an implementation unknown, and it is a real one: plans are
generated, not authored, and the engine will get better at generating them. Task 8.4 sharpens the
question to its practical form — what happens when a component targets plan grammar v0 and the engine
moves to v0.1?

Two decisions already on the books pull in opposite directions. ADR 0002 makes every document crossing
the boundary open-world and additively evolved. ADRs 0007 and 0008 freeze names forever — operators,
dimension ids, the sampling algorithm — because the artifacts using them outlive the engine that wrote
them. Which regime a plan belongs to depends on a question nobody had asked: does anything record one?

## Decision

**A plan is a rendering. The grammar is the record.** The test is whether an artifact outlives the
engine that produced it, and applied to every place a plan appears, only one does.

1. **Nothing in the ecosystem records a plan, and a pact file never will.** Recording a plan in a pact
   would freeze a compiler's output into a contract — improving the compiler would break pacts nobody
   can re-run — and would stand a second source of truth beside the shape the verifier actually
   matches against. This is stated as a prohibition rather than left unmentioned, because it is the
   obvious thing to propose later.
2. **The plan compiled for a given input is not stable across engine versions**, and nothing may
   depend on it being so. A newer engine may compile a better plan for the same shape.
3. **Compilation is deterministic within a version** — same inputs, same engine, same plan, node for
   node. This is not a weaker form of (2): it is the property golden corpora need to be usable at all,
   since a compiler that varies its output makes every corpus case flaky.
4. **The grammar is a versioned compatibility surface**, evolving additively under the protocol's
   rules and enforced by the same CI checker, because component-contributed fragments are authored
   against it. It ships as **v0**, not v1, because no external consumer has yet tried to extend it and
   claiming stability before task 8.4 tests it would be claiming something unearned.
5. **An action's semantics are frozen once published.** A component emitting `match:regex` breaks if
   an engine redefines it, so new behaviour is a new action name — ADR 0007's commitment, applied to
   actions for the same reason.

**Golden corpora record plans anyway, as snapshots.** A corpus case pins the plan text *and* the
verdicts over captured values, and the two failures mean opposite things: a verdict diff is a bug,
while a plan-text diff with verdicts unchanged is an acceptable optimisation that must nevertheless be
seen and reviewed. The plan text is what `explain` prints, so a silently restructured plan is a
user-visible change that a verdict-only corpus would miss. The verdicts are evidence the change was
safe — not proof, since deciding whether two plans accept the same values is the subsumption problem
over a richer language, and out of reach.

Spec text: [Plan grammar specification](../specs/plan-grammar/spec.md) §6–§7; schemas
`plan.schema.json` and `corpus-case.schema.json`.

## Alternatives considered

- **Freeze plans the way shapes and samples are frozen.** Consistent-looking, and it would forbid the
  compiler from ever improving — for an artifact nobody keeps.
- **Record plans in pact files** so a verifier replays a plan rather than recompiling. It moves the
  engine's internals into the contract, and makes every compiler fix a breaking change to published
  pacts.
- **Version individual plans** (a `plan-version` beside the grammar version). Precision nobody would
  consume: there is no reader that needs to know which compiler emitted a plan it is reading now.
- **Corpora that record verdicts only.** Cheaper to maintain, and blind to exactly the regression the
  RFC's inspectability claim cares about — a plan that restructures without changing a verdict.
- **Corpora that record the structured plan document rather than the text.** A stricter pin on
  something users never see, and unreadable in a pull request. Two plans that render identically are
  indistinguishable to users, which is the audience the snapshot exists to protect.
- **Call the grammar v1.** It would match the other Phase 2 schema sets, and would spend a version
  number on a surface no external consumer has touched.

## Consequences

Easier: the compiler is free to improve, and improving it costs a reviewed snapshot diff rather than a
compatibility event; corpora catch both behaviour changes and inspectability regressions with one
mechanism; task 8.4 has a concrete thing to stress — the grammar — rather than a vague notion of plan
stability.

Harder: every compiler change touches `corpora/`, so the corpus must stay small enough to regenerate
and large enough to matter (task 3.7 owns that balance); a reviewer has to tell two kinds of corpus
red apart, which the format makes explicit but does not make automatic; and debugging a failure that
reproduces only on one engine version means keeping the executed-plan text in CI artifacts, since
nothing else preserves it.

Committed to: no plan in any pact file, ever; the grammar evolving additively with frozen action
semantics; determinism within a version as a testable property; and v0 meaning "not yet stressed by an
external consumer" rather than "unfinished".

**Tripwire** — revisit if any of these show up in Phase 3 or 8: corpus snapshot churn is large enough
that reviewers start regenerating without reading (the snapshot has stopped being a check); task 8.4
finds that additive grammar evolution is not enough for real fragments, which would argue for a
negotiated grammar version rather than a single additive line; or a consumer emerges that genuinely
needs to read a plan produced by a different engine version, which would move plans out of the
rendering category and invalidate the whole decision.
