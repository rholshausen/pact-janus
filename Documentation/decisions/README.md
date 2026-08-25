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
| [0002](0002-document-first-protocol-over-frozen-pipes.md) | Define the protocol as schema-governed JSON documents over frozen byte-pipes | accepted |
| [0003](0003-embedding-priority-per-language.md) | Ship dual engine artifacts; set embedding priority per SDK language | accepted |
| [0004](0004-fork-v2-engine-as-kernel.md) | Fork the pact-reference v2 matching engine as the kernel starting point | accepted |
| [0005](0005-poll-based-event-delivery.md) | Poll-based event delivery on all pipes; push as a negotiated stdio capability | proposed |
| [0006](0006-bytes-in-the-document-model-and-negotiated-frame-encoding.md) | Model bytes explicitly; make frame encoding a negotiated axis with JSON as the baseline | proposed |
| [0007](0007-shapes-denote-value-sets.md) | Define a shape by the set of values it admits, over a domain that includes absence | proposed |

## Decision backlog

Known decisions waiting on evidence, seeded from the RFC's unresolved questions and the project plan.
Each becomes a numbered ADR when its inputs are ready (feeding task in brackets):

- Script-hook language and runtime (spike 1.6 recommends QuickJS with Boa fallback → design 2.7)
- Components on day one vs HTTP/JSON kernel-privileged (design 2.6, informed by 3.8, 4.2, 8.1)
- Variant sampling defaults: pairwise algorithm, exhaustive threshold, caps (design 2.3)
- Provider-state/variant linkage (`whenVariant`) design (design 2.3)
- Plan grammar versioning and stability policy (design 2.4, stressed by 8.4)
- Pact v5 file format schema (design 2.5)
- Subsumption decidability ladder and warn/block default (design 2.8, informed by 7.3; the per-operator
  comparability classes it consumes are fixed by ADR 0007)
- OCI component distribution model (design 2.6, minimal build 8.2)
- SDK conformance: what the suite must cover for an SDK to be called conformant (design 2.9, build 6.4)
- Adopting a binary frame encoding (CBOR the leading candidate) — held open as a capability by
  ADR 0006, waiting on benchmark evidence (task 1.7)
