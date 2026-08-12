# Decision log

Architecture Decision Records for Pact Janus. Every [gate] in the project plan and every contested
design choice lands here. An accepted ADR is not re-litigated in code review or implementation — write a
superseding ADR instead.

## Process

- Copy `template.md` to `NNNN-short-kebab-title.md` (next free number, zero-padded).
- Status flow: **proposed** → **accepted** | **rejected**; later **superseded by NNNN** if replaced.
- Keep ADRs short: context, decision, consequences, evidence. Link spike `FINDINGS.md` files and plan
  task numbers rather than restating them.
- Decisions that belong to the Pact community (naming, governance, funding, deprecation timelines) do
  **not** get ADRs here — they get framed in the Phase 9 report.

## Index

| ADR | Title | Status |
|---|---|---|
| [0001](0001-record-decisions-as-adrs.md) | Record decisions as ADRs | accepted |

## Decision backlog

Known decisions waiting on evidence, seeded from the RFC's unresolved questions and the project plan.
Each becomes a numbered ADR when its inputs are ready (feeding task in brackets):

- Protocol IDL: WIT, protobuf, or hybrid (spike 1.1 → gate G1)
- Primary embedding per SDK language: WASM component vs subprocess (spikes 1.2, 1.3 → G1)
- Kernel starting point: fork of pact-reference v2 matching engine vs rewrite (task 0.4 → G1)
- Script-hook language and runtime (spike 1.6 → design 2.7)
- Components on day one vs HTTP/JSON kernel-privileged (design 2.6, informed by 3.8, 4.2, 8.1)
- Variant sampling defaults: pairwise algorithm, exhaustive threshold, caps (design 2.3)
- Provider-state/variant linkage (`whenVariant`) design (design 2.3)
- Plan grammar versioning and stability policy (design 2.4, stressed by 8.4)
- Pact v5 file format schema (design 2.5)
- Subsumption decidability ladder and warn/block default (design 2.8, informed by 7.3)
- OCI component distribution model (design 2.6, minimal build 8.2)
- SDK conformance: what the suite must cover for an SDK to be called conformant (design 2.9, build 6.4)
