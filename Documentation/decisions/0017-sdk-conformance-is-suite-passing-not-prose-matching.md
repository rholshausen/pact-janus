# 0017 — SDK conformance is defined by the shared suite passing against a pinned engine, not by matching another SDK's implementation

- **Status**: proposed
- **Date**: 2026-09-04
- **Plan tasks**: 2.9 (feeds 6.3, 6.4, 6.5)
- **Evidence**: [Pact MkII RFC](https://github.com/pact-foundation/roadmap/pull/146) ("An SDK is
  conformant when it passes the suite against a pinned engine version"; "The guarantee of consistency is
  the conformance suite, not the generation method"), [SDK specification
  spec](../specs/sdk-specification/spec.md) §7, decision backlog item "SDK conformance"

## Context

The decision backlog named this directly: "what the suite must cover for an SDK to be called
conformant." The RFC gives the shape of the answer but not its content — it says conformance is
suite-passing, not that a byte-identical pact file or a matching implementation is required, and it does
not enumerate what the suite must actually cover.

The stakes are specific to this design. Plan task 6.3 writes the JVM SDK *from the 2.9 specification
alone*, deliberately not by porting the TypeScript SDK — the whole point being to find out whether a
written specification actually transmits behaviour the way two lock-step implementations used to (badly,
and only because both were visible to the same small group of maintainers at once). If "conformant" were
left to mean "behaves like the TypeScript SDK," that test would be meaningless: it would re-establish
exactly the reference-implementation dependency B1 exists to remove, just with TypeScript standing in for
what pact-reference stands in for today. And if "conformant" demanded byte-identical pact output, it
would fail on a fact this project never asked any two SDKs to agree on — `metadata.writer` names the SDK
and version that wrote the file, by design (contract spec §3.2).

## Decision

**An SDK is conformant, for a pinned engine version, when the shared conformance suite (task 6.4) passes
against it — not when its output matches another SDK's, and not when its behaviour matches this
specification's prose beyond what the suite checks.** The suite MUST cover four categories at minimum:
DSL→interaction-spec translation, session lifecycle, variant iteration, and pact-output equivalence
defined as **interaction-content identity** (contract spec §5–§6's `parts`, `states` and `selection`, not
the whole file) — never byte-identical files, because `metadata` is deliberately outside content identity.

Three commitments:

1. **No SDK is the reference implementation for another.** Conformance is measured against the suite and
   the pinned engine, period. This is what makes 6.3's "written from the spec, not ported" test meaningful
   at all — if JVM conformance meant "produces what TypeScript produces," the exercise would prove nothing
   about whether §3's `semantics` fields are precise enough, only whether someone read the TypeScript
   source anyway.
2. **Pact-output equivalence is interaction content, not file bytes.** Two SDKs given equivalent DSL usage
   must produce contracts the engine treats as the same demonstrated interaction — same shapes, same
   exercised variants, same evidence. `metadata.writer`, `metadata.created`, and any other fact contract
   spec §3.2 places outside content-addressed identity are explicitly not part of the comparison, because
   this design never asked two SDKs to agree on facts about themselves.
3. **The suite is necessary and sufficient for the claim "conformant"; it is not sufficient for the claim
   "thin."** A suite that samples scenarios cannot, by construction, prove the absence of a hand-rolled
   reimplementation that happens to agree with the engine on every sampled case. This decision does not
   pretend otherwise: task 6.5's thinness audit (LOC by layer, code inspection) is the complementary check
   for a claim conformance testing structurally cannot make, and conformance passing is not evidence of
   thinness.

Spec text: [SDK specification](../specs/sdk-specification/spec.md) §7.

## Alternatives considered

- **Conformance means matching a reference SDK's output or behaviour.** Rejected: reintroduces a
  lock-step dependency between implementations — the exact failure mode B1 exists to remove — under a new
  name, and would make 6.3's test of the specification format meaningless, since "conformant" would
  collapse into "read the other SDK's source."
- **Byte-identical pact files as the equivalence bar.** Rejected: it grades a fact (`metadata.writer` and
  friends) this design deliberately placed outside content identity, and a suite built to this bar would
  fail correctly-behaving SDKs for a reason that has nothing to do with behaviour.
- **Conformance defined by matching this specification's prose directly** (a human reads the `semantics`
  fields and judges an implementation against them). Rejected for the same reason unfalsifiable
  self-certification is rejected everywhere else in this project: prose is what an implementer reads to
  build the thing, not what proves the thing was built correctly. The suite is the only artifact that can
  fail a build.
- **Folding the thinness audit into the conformance bar** (an SDK must prove, structurally, that it
  contains no reimplemented matching logic, as a suite requirement). Rejected as out of reach for an
  automated suite in this prototype's timeframe — detecting "this passes today's samples but is secretly
  reimplementing matching" is a code-inspection problem, not a black-box testing problem, which is why
  task 6.5 exists as a separate, human-driven audit rather than a suite gate.

## Consequences

Easier: 6.3's result is a clean signal about the *specification*, not a referendum on how faithfully
someone read TypeScript source; a broker or CI pipeline can trust "conformant" as a build-checkable claim
rather than a maintainer's assertion; a future third SDK (Go, Python, .NET) has the same bar as JVM did,
with no implicit expectation that it resembles either existing SDK's internals.

Harder: two conformant SDKs can still look and feel quite different in ergonomics, which is a feature of
this decision (§6's compatibility-facade guidance, not this ADR, is where "feels native" gets addressed)
but will read as a gap to anyone expecting "conformant" to mean "interchangeable in every respect"; and the
suite itself becomes the single point of truth for a real, contested claim, which raises the cost of a
gap in its coverage — an under-specified category here is a silent hole in what "conformant" actually
guarantees, not a documentation problem to fix later.

**Tripwire** — revisit if 6.3 reports that JVM conformance required reading the TypeScript source to
resolve ambiguities the suite didn't catch (argues §3.3's bar is not actually being enforced, and either
the spec's `semantics` fields or the suite's coverage need to tighten); 6.5's thinness audit finds a
conformant SDK containing reimplemented matching logic (argues conformance and thinness need a shared gate
after all, contrary to commitment 3); or interaction-content equivalence turns out to be too strict in
practice because legitimate per-SDK differences exist that contract spec §3.2's placement rule did not
anticipate (argues the equivalence definition, not the suite, needs to move).
