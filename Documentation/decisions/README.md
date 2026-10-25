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
| [0003](0003-embedding-priority-per-language.md) | Ship dual engine artifacts; set embedding priority per SDK language | superseded by 0023 |
| [0004](0004-fork-v2-engine-as-kernel.md) | Fork the pact-reference v2 matching engine as the kernel starting point | accepted |
| [0005](0005-poll-based-event-delivery.md) | Poll-based event delivery on all pipes; push as a negotiated stdio capability | accepted |
| [0006](0006-bytes-in-the-document-model-and-negotiated-frame-encoding.md) | Model bytes explicitly; make frame encoding a negotiated axis with JSON as the baseline | accepted |
| [0007](0007-shapes-denote-value-sets.md) | Define a shape by the set of values it admits, over a domain that includes absence | accepted |
| [0008](0008-deterministic-pairwise-variant-sampling.md) | Select variants by a named, deterministic pairwise algorithm, and fail rather than truncate | accepted |
| [0009](0009-variant-bound-provider-state-parameters.md) | Bind provider-state parameters to variants in a separate member; an unproducible state fails | accepted |
| [0010](0010-plans-are-renderings-the-grammar-is-the-record.md) | Treat plans as renderings and the grammar as the record; version them accordingly | accepted |
| [0011](0011-contracts-as-self-identifying-json-documents.md) | Record contracts as a single self-identifying JSON document, named and versioned independently of the Pact specification | accepted (decision 4 superseded by 0018) |
| [0012](0012-one-interface-two-bindings.md) | Define components as the engine protocol's frames turned around, and give the interface two bindings | accepted |
| [0013](0013-component-hosting-is-an-embedding-capability.md) | Make component hosting a negotiated embedding capability, and distribute out-of-tree components as digest-pinned OCI artifacts | accepted |
| [0014](0014-hooks-are-resolved-configuration-not-callbacks.md) | Declare hooks in configuration a loader resolves; the engine receives values, never callbacks, paths or templates | accepted |
| [0015](0015-quickjs-as-the-scripted-hook-runtime.md) | Make QuickJS the scripted-hook runtime, with the hook context as a script's entire capability surface | accepted |
| [0016](0016-subsumption-defaults-to-warn-with-mandatory-reason-exemptions.md) | Subsumption defaults to warn, not block; exemptions require a reason and are scoped by field, interaction or consumer | accepted |
| [0017](0017-sdk-conformance-is-suite-passing-not-prose-matching.md) | SDK conformance is defined by the shared suite passing against a pinned engine, not by matching another SDK's implementation | accepted |
| [0018](0018-canonical-contract-bytes-are-compact-not-pretty-printed.md) | Write Janus contracts as compact JSON; canonical form carries no formatting promise, and a reader must not assume one of its input | accepted |
| [0019](0019-an-sdk-refuses-what-it-cannot-spell-and-stops-when-the-engine-does.md) | Refuse at the call what an SDK cannot spell the same way in every language, write what it can, and fail a run at once when the engine errors | accepted |
| [0020](0020-a-content-slots-type-is-declared-on-the-interaction.md) | Declare a content slot's type on the interaction, beside its parts, not inside them | accepted |
| [0021](0021-a-component-artifact-is-its-own-type-and-a-pin-is-fetched-by-digest.md) | Distribute a component as its own OCI artifact type, described by its handshake, and fetch a pin by digest alone | accepted; amended by 0052 |
| [0022](0022-a-fragment-replaces-its-slots-plan-and-declares-a-grammar-the-engine-says-it-reads.md) | A plan fragment replaces its slot's plan, declares a grammar the engine says it reads, and is checked before anything runs | accepted |
| [0023](0023-the-subprocess-is-the-primary-embedding-and-wasm-serves-offline-operations.md) | Make the subprocess every SDK's primary embedding, and WASM the embedding for offline operations | accepted |
| [0024](0024-request-dimensions-stay-pinned-and-a-cardinality-point-matches-a-region.md) | Keep request dimensions pinned, and match a cardinality point as a region | accepted |
| [0025](0025-a-provider-shape-entry-may-select-interactions-by-operation.md) | Let a provider-shape entry select the interactions it is about by operation | accepted |
| [0026](0026-every-plan-action-is-namespaced-and-core-namespaces-are-reserved.md) | Namespace every plan action, and reserve the core families as namespaces | accepted (implemented as plan grammar v1, staged plan S0.1) |
| [0027](0027-build-in-the-prototype-repository-and-graduate-sdks-into-their-existing-repositories.md) | Build in the prototype's repository, and graduate SDKs into their existing repositories | accepted |
| [0028](0028-change-files-decide-a-releases-version-and-merging-their-pull-request-releases.md) | Decide a release's version with change files, and release by merging the pull request that consumes them | accepted (decision 9 extended by 0030) |
| [0029](0029-attest-what-a-release-built-and-let-an-sdk-verify-the-digest-it-pins.md) | Attest what a release built, and let an SDK verify the digest it pins | accepted |
| [0030](0030-a-release-carries-the-sdk-gates-and-the-executable-specification.md) | A release carries the SDK gates and the executable specification they run against | accepted |
| [0031](0031-rename-by-a-rehearsed-rewrite-and-read-persisted-identifiers-under-every-spelling.md) | Rename by a rehearsed rewrite, and read persisted identifiers under every spelling | accepted |
| [0032](0032-speak-to-the-broker-through-protocol-operations-backed-by-a-native-host-capability.md) | Speak to the Pact Broker through protocol operations backed by a native host capability | accepted |
| [0033](0033-count-a-runs-variants-per-source-document-in-its-summary.md) | Count a run's variants per source document in its summary | accepted |
| [0034](0034-apply-a-pacts-request-generators-through-a-generator-component-fed-by-state-values.md) | Apply a pact's request generators through a generator component, fed by provider-state values | accepted |
| [0035](0035-reach-providers-and-brokers-through-one-async-http-client-configured-by-one-document.md) | Reach providers and brokers through one async HTTP client, configured by one document | accepted |
| [0036](0036-read-text-bodies-through-a-text-component-and-unclaimed-bodies-as-bytes.md) | Read text bodies through a text component, and bodies no component claims as bytes | accepted |
| [0037](0037-carry-member-order-through-the-engine-and-decide-nothing-by-it.md) | Carry member order through the engine, and decide nothing by it | accepted |
| [0038](0038-read-form-bodies-as-a-query-string-through-a-built-in-form-component.md) | Read form bodies as a query string, through a built-in form component | accepted |
| [0039](0039-read-xml-bodies-by-element-name-through-a-built-in-xml-component.md) | Read XML bodies by element name, through a built-in XML component | accepted |
| [0040](0040-read-multipart-bodies-by-part-name-through-a-built-in-multipart-component.md) | Read multipart bodies by part name, through a built-in multipart component | accepted |
| [0041](0041-refuse-a-filter-that-names-no-recorded-variant-and-report-a-narrowed-run-as-filtered.md) | Refuse a filter that names no recorded variant, and report a narrowed run as filtered | accepted |
| [0042](0042-compile-the-collection-rules-of-a-v1-v4-pact-as-todays-verifier-applies-them.md) | Compile the collection rules of a v1–v4 pact as today's verifier applies them | accepted |
| [0043](0043-check-a-pacts-content-type-once-on-the-header.md) | Check a pact's Content-Type once, on the header | accepted |
| [0044](0044-reach-a-hook-endpoint-as-a-provider-is-and-try-a-state-call-three-times.md) | Reach a hook endpoint as a provider is, and try a state call three times | accepted |
| [0045](0045-refuse-an-interaction-that-needs-a-pact-plugin-by-name-and-verify-the-rest.md) | Refuse an interaction that needs a pact plugin by name, and verify the rest | accepted |
| [0046](0046-ask-a-provider-for-a-message-through-produce-message-and-let-the-transport-name-its-parts.md) | Ask a provider for a message through `produce-message`, and let the transport name its parts | accepted |
| [0047](0047-upgrade-a-body-into-its-components-document-and-check-its-type-on-the-header.md) | Upgrade a body into its component's document, and check its type on the header | accepted; amended by 0048 |
| [0048](0048-carry-generators-on-shapes-and-fill-fresh-values-where-a-value-is-produced.md) | Carry generators on shapes, and fill fresh values where a value is produced | accepted |
| [0049](0049-match-a-content-type-against-the-arrivals-own-octets-and-labels.md) | Match a content type against the arrival's own octets and labels | accepted |
| [0050](0050-explain-compiles-what-a-run-compiles-and-runs-captured-values.md) | `explain` compiles what a run compiles, and runs captured values | accepted |
| [0051](0051-leave-pending-out-of-the-contract-and-let-upgrade-exclude-pending-interactions.md) | Leave pending out of the contract, and let upgrade exclude pending interactions | accepted |
| [0052](0052-declare-a-components-registry-credential-in-its-source.md) | Declare a component's registry credential in its source | accepted |
| [0053](0053-take-todays-verifier-options-where-they-mean-what-they-did.md) | Take today's verifier options where they mean what they did | accepted |

## Decision backlog

Known decisions waiting on evidence, seeded from the RFC's unresolved questions and the project plan.
Each becomes a numbered ADR when its inputs are ready (feeding task in brackets):

- Upstream Pact Broker support for a `janus` specification value — a PR series, not an allowlist
  edit (ADR 0011 decision 6; scoped in the [format review](../prototype/contract-file-format-review.md) §8.4)
- Adopting a binary frame encoding (CBOR the leading candidate) — held open as a capability by
  ADR 0006, waiting on benchmark evidence (task 1.7)
- A consumer's test verdict reaching the engine (`report-variant`), so the engine, not each SDK,
  withholds a dishonest contract — Phase 9 findings 4 and 5 (task 9.4)
- Operator-level substitution in place of whole-slot plan fragments — ADR 0022's tripwire, Phase 9
  finding 21 (task 9.4)
