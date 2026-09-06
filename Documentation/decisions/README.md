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
| [0005](0005-poll-based-event-delivery.md) | Poll-based event delivery on all pipes; push as a negotiated stdio capability | accepted |
| [0006](0006-bytes-in-the-document-model-and-negotiated-frame-encoding.md) | Model bytes explicitly; make frame encoding a negotiated axis with JSON as the baseline | accepted |
| [0007](0007-shapes-denote-value-sets.md) | Define a shape by the set of values it admits, over a domain that includes absence | accepted |
| [0008](0008-deterministic-pairwise-variant-sampling.md) | Select variants by a named, deterministic pairwise algorithm, and fail rather than truncate | accepted |
| [0009](0009-variant-bound-provider-state-parameters.md) | Bind provider-state parameters to variants in a separate member; an unproducible state fails | accepted |
| [0010](0010-plans-are-renderings-the-grammar-is-the-record.md) | Treat plans as renderings and the grammar as the record; version them accordingly | accepted |
| [0011](0011-contracts-as-self-identifying-json-documents.md) | Record contracts as a single self-identifying JSON document, named and versioned independently of the Pact specification | accepted |
| [0012](0012-one-interface-two-bindings.md) | Define components as the engine protocol's frames turned around, and give the interface two bindings | accepted |
| [0013](0013-component-hosting-is-an-embedding-capability.md) | Make component hosting a negotiated embedding capability, and distribute out-of-tree components as digest-pinned OCI artifacts | accepted |
| [0014](0014-hooks-are-resolved-configuration-not-callbacks.md) | Declare hooks in configuration a loader resolves; the engine receives values, never callbacks, paths or templates | accepted |
| [0015](0015-quickjs-as-the-scripted-hook-runtime.md) | Make QuickJS the scripted-hook runtime, with the hook context as a script's entire capability surface | accepted |
| [0016](0016-subsumption-defaults-to-warn-with-mandatory-reason-exemptions.md) | Subsumption defaults to warn, not block; exemptions require a reason and are scoped by field, interaction or consumer | accepted |
| [0017](0017-sdk-conformance-is-suite-passing-not-prose-matching.md) | SDK conformance is defined by the shared suite passing against a pinned engine, not by matching another SDK's implementation | accepted |

## Decision backlog

Known decisions waiting on evidence, seeded from the RFC's unresolved questions and the project plan.
Each becomes a numbered ADR when its inputs are ready (feeding task in brackets):

- Upstream Pact Broker support for a `janus` specification value — a PR series, not an allowlist
  edit (ADR 0011 decision 6; scoped in the [format review](../contract-file-format-review.md) §8.4)
- Adopting a binary frame encoding (CBOR the leading candidate) — held open as a capability by
  ADR 0006, waiting on benchmark evidence (task 1.7)
