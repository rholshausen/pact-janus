# The prototype's record

Pact Janus began as a prototype of the [Pact MkII RFC](https://github.com/pact-foundation/roadmap/pull/146),
built in seven weeks to test the RFC's bets against real code. It is complete, and the real build now
proceeds in this repository under the [staged implementation plan](../staged-implementation-plan.md)
([ADR 0027](../decisions/0027-build-in-the-prototype-repository-and-graduate-sdks-into-their-existing-repositories.md)).

This directory is what the prototype wrote down: the plan it was built to and the report each task
left. Nothing here is normative and nothing here is maintained. The documents are kept as written,
because the [decision records](../decisions/README.md), the [specifications](../specs) and the staged
plan cite them as evidence. Task numbers in them (`1.6`, `G1`) are the project plan's; item ids with
an `S` (`S1.4`) are the staged plan's.

Three things from the prototype are not here, because they are still live: the
[charter](../charter.md), the decision records and the specifications.

## Start here

- [Community report](prototype-report.md) (task 9.3): what was built and what was learned, in a few pages.
- [RFC feedback](rfc-feedback.md) (9.2): every question the RFC left open, answered with evidence.
- [Phase 9 findings](phase-9-findings.md): the numbered findings the staged plan refers to
  ("finding 11") — what using the prototype turned up.

## The plan

- [Project plan](project-plan.md): the phases, tasks, gates and milestones the prototype was built to.

## Reports, by task

| Task | Report | Tested |
|---|---|---|
| 0.4 | [Reuse inventory](reuse-inventory.md) | What to take from pact-reference, pact-jvm and friends, and how |
| 2.5 | [Contract file format review](contract-file-format-review.md) | The evidence behind ADR 0011: a contract is not a "pact v5" |
| 2.10 | [AI-layer design notes](ai-layer-design-notes.md) | Where diagnosis and agentic verification sit on a deterministic core |
| 3.8 | [Kernel-boundary review](kernel-boundary-review.md) | "The kernel knows nothing about HTTP or JSON", against what was built |
| 4.6 | [Variant ergonomics report](variant-ergonomics-report.md) | Whether a failing variant is identifiable and the sampled matrix understandable |
| 6.3 | [The JVM SDK from the specification alone](jvm-sdk-from-spec-report.md) | Whether the behavioural spec is enough to write an SDK from |
| 6.4 | [Conformance suite report](conformance-suite-report.md) | "Conformant" as a claim a build makes |
| 6.5 | [Thinness audit report](thinness-audit-report.md) | "SDKs are thin", as a number, and the AI-regeneration trial |
| 7.5 | [Broker integration notes](broker-integration-notes.md) | What the broker needs for provider shapes and decisions |
| 8.1 | [Third-party component report](third-party-component-report.md) | A WASM component written from the published docs only |
| 8.4 | [Plan-fragment stress test](plan-fragment-stress-test.md) | A component contributing plan fragments across a grammar version change |
| 9.1 | [Performance report](performance-report.md) | Janus in three embeddings against the `pact_ffi` baseline |

## Spikes

The time-boxed experiments left a `FINDINGS.md` each, under [`spikes/`](../../spikes/README.md). Their
code was deleted when the real build started (S0.12); the findings stay at the paths the decision
records cite.
