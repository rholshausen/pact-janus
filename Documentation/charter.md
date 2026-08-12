# Pact Janus — Prototype Charter

*Plan task 0.1 · One page · This is the yardstick for scope arguments. Changes to this document are
decisions and get an ADR.*

## Purpose

Produce **evidence** for the [Pact MkII RFC](https://github.com/pact-foundation/roadmap/pull/146) — for
its five bets where they hold, and honestly against them where they don't — so the RFC can be revised,
accepted or reshaped on facts, and so a staged plan for the real build can be written with known risks.
Janus is a prototype: its code may seed the real build, but its *findings* are the deliverable.

## Success criteria

The prototype is a success if all of the following are demonstrated (milestones in brackets refer to
`project-plan.md`):

1. **One engine, thin SDKs.** The RFC's consumer example runs near-verbatim in TypeScript and on the
   JVM against the same engine over the same protocol, with behaviour proven identical by a conformance
   suite, and each SDK's hand-written layer contains no matching or orchestration logic (M4, audit 6.5).
2. **No FFI failure modes.** Across both SDKs and the CLI there are: no per-object cleanup calls, no
   panics crossing the boundary, no bespoke async bridging, and no SDK-specific orchestration that could
   diverge (inspection against protocol spec 2.1, demonstrated at M2–M4).
3. **Plans carry the semantics.** v1–v4 matching rules compile to plans whose verdicts agree with the
   current implementation on the existing spec test corpus, and `pact explain` / `--executed` renders
   every plan the prototype can produce (M1, M3).
4. **Optionality is answered.** The RFC's order payload (12-variant space) is tested by one consumer
   test with a sampled matrix; a consumer that mishandles a variant fails the build with an
   identifiable variant; only exercised variants are recorded (M2).
5. **The loop closes.** The "provider may produce SHIPPED, consumer only tested PENDING" scenario is
   caught by the subsumption check, fixed by widening the consumer shape, and the widened shape is
   forced to be exercised by variant testing (M5).
6. **Components are real.** A third-party WASM component, built against published interfaces without
   reading engine source, works in both consumer tests and verification (M6).
7. **Performance is characterised.** WASM and subprocess embeddings are benchmarked against today's
   pact_ffi baseline, with numbers good enough to either clear the RFC's performance question or scope
   it precisely (9.1).

A criterion that *fails* does not fail the prototype — a documented finding of *why*, with evidence, is
equally a success outcome. Undocumented abandonment is the only failure mode.

## Non-goals

Janus will **not** attempt: production hardening or API stability; more than two SDK languages; broker
or PactFlow server-side changes (subsumption runs locally/CI-side; broker needs are design notes only);
transports beyond HTTP plus at most one stretch (gRPC or message); a complete `pact upgrade` (basic
v3/v4 → v5 only); the AI-assisted layer (design notes only); migration/deprecation timelines, naming,
governance or funding decisions (framed for the community, not decided here); Windows/exotic-platform
coverage beyond what CI gives for free.

## Definition of "prototype complete"

Phases 0–9 of `project-plan.md` are done or explicitly descoped by ADR; every success criterion above
has either a demo or a written finding; the RFC's unresolved-questions table (plan §13) has an answer or
an honest "still open, here is what we learned" for every row; and the Phase 9 report + staged
implementation plan are published for community review.

## Working practices (summary — details in `project-plan.md` §14)

Decisions land in `Documentation/decisions/` as ADRs. Spikes are disposable, findings are not
(`FINDINGS.md` required). Corpora are load-bearing from Phase 3. Benchmarks trend in CI from Phase 4.
Tasks that exist to gather evidence *against* the RFC's bets (4.6, 6.5, 7.3) are not optional.
