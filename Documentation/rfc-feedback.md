# RFC feedback: what the prototype found

Plan task **9.2**. **Date:** 2026-09-24. **Subject:** the [Pact MkII RFC](https://github.com/pact-foundation/roadmap/pull/146)
(`rfc/0000-pact-mkii.md` on the `rfc/pact-mkii` branch), read against everything Pact Janus built and
measured in Phases 0–9.

This is the reference document behind the RFC's revision. It answers every question the RFC left open,
checks the drawbacks it predicted against what happened, and lists what the RFC stated as design that the
evidence changed. Each claim links to the report, ADR or finding it rests on. The RFC itself carries the
conclusions and a link here; the community report (task 9.3) is the narrative version.

The charter's rule applies throughout: a bet that failed, with a documented reason, is a result. The only
failure mode is a claim nobody checked.

## Contents

1. [The five pillars, in one line each](#1-the-five-pillars-in-one-line-each)
2. [The unresolved questions](#2-the-unresolved-questions)
3. [What the RFC stated that the evidence changed](#3-what-the-rfc-stated-that-the-evidence-changed)
4. [The drawbacks, measured](#4-the-drawbacks-measured)
5. [Drawbacks the RFC did not predict](#5-drawbacks-the-rfc-did-not-predict)
6. [The charter's success criteria](#6-the-charters-success-criteria)
7. [What task 9.2 changed](#7-what-task-92-changed)
8. [Carried to 9.4](#8-carried-to-94)

## 1. The five pillars, in one line each

| Pillar | Verdict | The one caveat that matters |
|---|---|---|
| **One core, thin SDKs** | **Holds.** Two SDKs, one engine, 47 of 47 conformance cases each, 723 and 1,802 hand-written lines | The engine is embedded as a subprocess, not as WASM ([ADR 0023](decisions/0023-the-subprocess-is-the-primary-embedding-and-wasm-serves-offline-operations.md)) |
| **Declarative interactions, compiled plans** | **Holds.** v1–v4 rules compile to plans that agree with pact-specification on 583 of 583 cases in scope; `explain` renders every plan | `explain` loads no components, and matching captured values is a CLI feature, not a protocol operation (findings 23, 31) |
| **Everything is a component** | **Holds at the interface.** A third party wrote a CSV component from the docs alone, and it runs in a consumer test and in verification | "Contribute plan fragments" turned out to mean "replace the slot's plan", which is not safe with variants (ADR 0022, finding 21) |
| **Shapes with honest optionality** | **Holds.** One consumer test covers the RFC's order payload, a 24-variant space (the RFC counted 12, leaving out the list-length boundary it declares), with 8 variants, and the SHIPPED/PENDING loop closes end to end | Request-side variants and list lengths needed a decision ([ADR 0024](decisions/0024-request-dimensions-stay-pinned-and-a-cardinality-point-matches-a-region.md)), and the SDK alone knows whether a test passed (finding 4) |
| **Scriptable lifecycle** | **Holds** for HTTP: hooks are configuration the engine receives as values, and scripts run in an embedded QuickJS | Message-interaction hooks were designed and never built |

## 2. The unresolved questions

Each row gives the RFC's question, the answer, and what changes in the RFC.

### 2.1 Through the RFC process

**Naming and versioning: Pact specification v5 + "Pact 6" SDK majors, or a new brand?** — *Framed, not
decided.* This belongs to the community (charter non-goals). One thing the prototype did decide, for
itself only: its artifact is a **Janus contract** (`janus-contract/1`), deliberately not "pact v5"
([ADR 0011](decisions/0011-contracts-as-self-identifying-json-documents.md)), so that the Pact
specification stays free to define its own next version. The RFC should drop "Pact file format v5" as a
committed name for the same reason. Task 9.4 frames the rest.

**Is "everything is a component" day one, or may HTTP/JSON be kernel-privileged at first?** — *Resolved:
day one, for the interfaces.* The built-in HTTP transport and JSON content handler implement the same
four interfaces a third party does ([ADR 0012](decisions/0012-one-interface-two-bindings.md)), and
`third-party/janus-csv` was written from the published specs without reading engine source
([third-party component report](third-party-component-report.md)). Two privileges remain, and the RFC
should name them rather than claim none:

- **Packaging.** Built-ins are compiled in; an out-of-tree component needs a host that can load one, and
  a WASM-embedded engine cannot ([ADR 0013](decisions/0013-component-hosting-is-an-embedding-capability.md)).
- **Structure.** The kernel no longer knows HTTP *vocabulary*, but it still assumes HTTP's *shape*: parts
  named `request` and `response`, and a mismatch answered with a `500`
  ([kernel-boundary review](kernel-boundary-review.md) findings 8 and 9). A message transport cannot use
  either assumption. The fix is known and not made.

**IDL for the engine protocol (WIT, protobuf or both), and the WASM-host story per language?** — *The IDL
is resolved; the WASM story is reversed.*

- **IDL.** Neither. The protocol is schema-governed JSON documents over frozen byte-pipes; WIT defines only
  the one-function pipe ([ADR 0002](decisions/0002-document-first-protocol-over-frozen-pipes.md), gate G1).
  JSON Schema plus a CI checker enforces the open-world evolution rules, and both SDKs' bindings are
  generated from those schemas.
- **WASM host story.** A WASM build of the engine cannot run a consumer test or a verification in any
  language, as built: its exchange loop and HTTP server run on threads, and a `wasm32-wasip2` guest has
  none ([Phase 9 findings](phase-9-findings.md) 3 and 29). It does have sockets; an earlier version of
  this document said otherwise. WASI 0.3's async (`wasm32-wasip3`) could remove the need for the thread,
  which is why ADR 0023 reassesses when that target lands. The zero-import core module meant for
  the JVM and Go cannot be built at all. So the subprocess is every SDK's primary embedding, and WASM is
  the embedding for offline operations — explain, upgrade, the subsumption check, variant enumeration —
  where a host wants no native binary ([ADR 0023](decisions/0023-the-subprocess-is-the-primary-embedding-and-wasm-serves-offline-operations.md)).
  The only route back to a WASM engine that runs a test is a transport the host provides, which is
  named and not taken.

**Variant sampling defaults: is pairwise right, and what are the caps and overrides?** — *Resolved*
([ADR 0008](decisions/0008-deterministic-pairwise-variant-sampling.md), variant-semantics spec §3):
exhaustive below a threshold, a named deterministic pairwise algorithm above it, boundary variants
always, pins always, and a budget that **fails** rather than truncates. The RFC's order payload (a
24-variant space once its list-length boundary is counted — the RFC counted 12) is covered by 8, and two SDKs written independently select the same 8 in the same order
([conformance report](conformance-suite-report.md) §2). Honest limit: no real team has used it, so none
of ADR 0008's tripwires (routinely disabled boundaries, routinely raised budgets) could fire. One
sub-question was settled in 9.2 — what a list-length variant matches ([ADR 0024](decisions/0024-request-dimensions-stay-pinned-and-a-cardinality-point-matches-a-region.md)) —
and one is left open: whether an array admits `[]` by default (finding 2).

**Provider-state/variant linkage (`whenVariant`)?** — *Resolved* ([ADR 0009](decisions/0009-variant-bound-provider-state-parameters.md),
variant-semantics spec §6). The RFC's sketch put the binding inside the state's parameters, where it could
not be told apart from a literal value. Bindings live in a separate member, resolve per variant, and a
provider that cannot produce a state reports `state-unavailable` rather than `failed`, because its remedy
is a contract change. Demonstrated against the sample provider in task 5.2. Same honest limit as above.

**Subsumption policy: warn or block by default, and how are exemptions scoped?** — *Resolved*
([ADR 0016](decisions/0016-subsumption-defaults-to-warn-with-mandatory-reason-exemptions.md)): warn by
default for both decided findings and reviews, and exemptions scoped by field, interaction or consumer,
each needing a `reason` and optionally an `expires`. The evidence behind warn-first:

- Type-derived shapes are noisy exactly as the RFC feared: an ORM-generated OpenAPI document produced
  **4.5×** the findings of a hand-written one for the same API, and all of the excess was true and
  useless — one finding per nullable column ([spike 7.3](../spikes/7.3-type-derived-shapes/FINDINGS.md)).
- A converted v1–v4 pact is noisy the other way: too narrow, because its one example becomes `equality`.
  On the sample provider, one finding in four was that artifact, and task 9.2 removed its cause (finding 9).

Still open: a `provenance` selector on exemptions, which both kinds of noise argue for.

**Subsumption decidability: where does the check degrade to "unknown, review manually"?** — *Resolved*
(shape-language spec §8). Every operator has a comparability class. **Exact** operators (literals,
kinds, `optional`, `nullable`, `any-of`, `one-of`, objects, cardinality) are decided by set containment.
**Conservative** ones (`regex`, the datetime family, `include`, `content-type`) are decided on identity,
on an exactly wider container, and, since 9.2, when the provider admits a finite set of values the
consumer's own matcher can test. Two different regexes are `unknown`. **Opaque** ones (`contains`, and
component operators that declare nothing) are `unknown` unless identical. `unknown` is a first-class
`review`, never a guess. A property test brute-forces `admits` through the real compiler and interpreter
and checks the checker against it on every pair of exact-class operators; the first run found a real
matcher bug (finding 8).

**Governance: who owns the engine, the SDK spec and conformance sign-off, and what is the funding
model?** — *Framed, not decided* (charter non-goals; task 9.4). Evidence the discussion can use:

- a complete SDK's hand-written layer is **723 lines** of TypeScript or **1,802** of Java
  (task 6.5 measured 583 and 1,630, before content components and variant bindings)
  ([thinness audit](thinness-audit-report.md));
- the JVM SDK was written from the specification alone and recorded the same contract, member for member,
  as the TypeScript one ([JVM report](jvm-sdk-from-spec-report.md), conformance report §2);
- a specification change was carried into both SDKs by agents that never saw each other's code, and both
  passed the suite (thinness audit §3).

"Conformant" is a command both SDKs pass (ADR 0017), which gives a governance model something mechanical
to own.

### 2.2 Through implementation

**Plan grammar stability and versioning policy?** — *Resolved, with a strain in the fragment model.*
Plans are renderings and the grammar is the record
([ADR 0010](decisions/0010-plans-are-renderings-the-grammar-is-the-record.md)). Grammar versions are
ordered, the engine says which it reads, and a component's fragment declares which it targets. Every
version mismatch fails by name before anything runs
([ADR 0022](decisions/0022-a-fragment-replaces-its-slots-plan-and-declares-a-grammar-the-engine-says-it-reads.md),
[stress test](plan-fragment-stress-test.md)). The strain is in what a fragment *is*. A content component
that contributes one replaces its slot's whole plan, and is compiled without knowing the variant, so it
widened a pinned variant: the variant passed where the engine's own plan correctly failed (finding 21). The
likely answer is operator-level substitution — the engine compiles the slot, and a component says only
what one value operator means for its content type. It is ADR 0022's tripwire and 9.4's to decide.

**Broker/PactFlow handling of the new artifacts?** — *Partly resolved*
([broker integration notes](broker-integration-notes.md)). A Janus contract already stores, dedupes,
diffs and feeds `can-i-deploy` in today's broker, published as `specification: "pact"` (ADR 0011
decision 6). The **provider shape** has no resource that fits, and the notes lay out a PR series for one.
Verification results need per-pair counts before a broker can key them to a matrix row (finding 10). The
check itself should run in the engine, not be reimplemented in the broker.

**Performance envelope of WASM embeddings vs today's native FFI?** — *Resolved*
([performance report](performance-report.md)):

- Janus is faster than pact_ffi on every scenario it can run the same way. For example, a small mock
  exchange takes 33 µs against pact_ffi's 139 µs.
- The exception was large request bodies, where re-arming a variant recompiles its plan (finding 28).
- The subprocess costs almost nothing over in-process.
- WASM runs the kernel's own work within 10–35% of native.

So performance does not decide the embedding; capability does (see the WASM row above). The
measurements also found a quadratic in the kernel, fixed in 9.1, and a fixed 200 ms per consumer session,
fixed in 9.2.

**Message-interaction hook design (sync message RPC, broker adapters)?** — *Still open; build deferred.*
What was learned:

- Spike 1.5 and design 2.7 specified the hook points (`produce-message`, `consume-message`).
- Spike 8.3 drove a consumer test and a verification through a non-HTTP transport running out of
  process, which shows the transport interface is not HTTP-shaped.
- The kernel still is HTTP-shaped in the two places the kernel-boundary review names, and those are what
  a message transport would hit first.

### 2.3 Out of scope

The deprecation timeline for current implementations, and broker API evolution beyond accepting
artifacts, are framed for the community in 9.4. The broker notes' PR series is the input for the second.

## 3. What the RFC stated that the evidence changed

These are not open questions. The RFC stated them as design, and the revision should restate them.

1. **"WASM component (preferred)."** It is the subprocess, for every SDK. WASM serves offline operations
   ([ADR 0023](decisions/0023-the-subprocess-is-the-primary-embedding-and-wasm-serves-offline-operations.md)).
   The RFC's "per-test-run, protocol-versioned subprocess" is no longer a fallback's excuse. It is the
   design, and spike 1.3's EOF-exit lifecycle is what keeps it from repeating pact-ruby-standalone.
2. **"Pact file format v5."** A Janus contract (ADR 0011), leaving v5 to the Pact specification. The
   migration path is otherwise as the RFC says: v1–v4 are read and verified unchanged, and `janus upgrade`
   converts them.
3. **"Out-of-process over gRPC … today's pact-plugins model retained."** Out-of-process components speak
   the protocol's own stdio framing, with no second wire format (component-interfaces spec §9, spike 8.3).
   Out of process, the `env` grant is enforced and `fs`/`network` are not.
4. **"Content handlers contribute plan fragments."** They can, and a fragment replaces the slot's plan.
   That is not safe with variants (§2.2 above). The RFC should say that contribution is at the level of
   operators, and that the fragment form is an open question.
5. **"Boundary variants (min, min+1)."** They stand, with one clarification: a cardinality point is matched
   as a region. `min+1` means "more than the minimum", and a consumer's or provider's list of five meets
   it. Request-side variants stay pinned, because the RFC's claim that "the reverse direction is covered
   by variant replay" depends on it
   ([ADR 0024](decisions/0024-request-dimensions-stay-pinned-and-a-cardinality-point-matches-a-region.md)).
6. **"A provider may publish a provider shape for each operation."** It can now be matched *by
   operation*. A derived shape has no way to know a consumer's interaction descriptions, and matching on
   them silently checked nothing ([ADR 0025](decisions/0025-a-provider-shape-entry-may-select-interactions-by-operation.md)).
7. **The protocol sketch.** `finalise` cannot tell whether the consumer's test *passed*. Only the SDK knows
   that, so today every SDK must withhold a dishonest contract itself (finding 4). The sketch should gain
   an operation that reports the test's verdict to the engine. The same gap means an engine-side failure
   surfaces at the end of a suite, not in the test that caused it (finding 5).
8. **"Components are distributed as OCI artifacts … the engine resolves, caches and verifies them."**
   True, and built: a component is its own artifact type, pinned by digest and re-verified from a
   content-addressed cache ([ADR 0021](decisions/0021-a-component-artifact-is-its-own-type-and-a-pin-is-fetched-by-digest.md)).
   One refinement: a component published with the WebAssembly ecosystem's own tool (`wkg`) is not
   accepted as-is (finding 17).

## 4. The drawbacks, measured

| RFC drawback | What happened |
|---|---|
| **A very large undertaking** | The prototype covers one engine, two SDKs, a CLI, three embeddings (one for measurement only) and a third-party component. The 9.4 staged plan has to scope the real build against that |
| **Osborne effect / community split** | Not measurable in a prototype. The mitigation the RFC relies on is real: v1–v4 pacts verify unchanged, with 583 of 583 in-scope specification cases agreeing, and `janus upgrade` converts them, reporting every narrowing it makes |
| **WASM host maturity varies; the subprocess reintroduces process management** | **Worse than predicted, and in a different place.** Host maturity was not the limit; the guest was. No WASM engine can serve a mock or drive a provider (ADR 0023). Process management is therefore the main path. It held up on Linux and on real Windows: orphan tests, EOF exit, 0.6–1.2 ms spawn (spikes 1.3, 8.3) |
| **Social cost of pact-jvm ceasing to be independent** | Not measurable. The JVM SDK written from the spec alone shows what a JVM maintainer would own: an idiomatic layer of about 1,560 lines |
| **The plan grammar becomes public, versioned API** | The versioning policy held under a stress test once it was stated (ADR 0022). The strain was in the fragment model, not the versioning (§2.2) |
| **Variant testing has sharp edges** | **Confirmed.** Whether a failing variant can be identified depends on the test framework, not the engine: Vitest and JUnit 5 name it for free, and a flat Rust loop gives no variant context ([variant ergonomics report](variant-ergonomics-report.md)). The engine cannot tell whether the closure handled the response (finding 4). Request-side dimensions make the closure variant-parameterised (ADR 0024). Pairwise stayed small on the RFC payload (8 of 24) |
| **Subsumption findings can overwhelm** | **Confirmed and quantified** (§2.1): 4.5× from generator-derived shapes, and a separate narrowness in converted pacts. Warn-first (ADR 0016) is the right default. A second, worse problem appeared that the RFC did not predict: derived shapes silently matched nothing (ADR 0025) |

## 5. Drawbacks the RFC did not predict

- **The contract can say less than a v1–v4 pact did by default.** v1–v4's `Content-Type: application/json`
  accepts `; charset=utf-8`. A shape has no operator for that without putting HTTP into the core vocabulary,
  so an upgraded contract is stricter (finding 1). The fix belongs to the HTTP component, as a namespaced
  operator.
- **Canonical contract bytes are reconstructed by each SDK.** The engine returns a document; the SDK writes
  it. Two SDKs can write different bytes for the same content (finding 6), which undermines broker dedup.
- **Content types are not all JSON-shaped.** The document model has no member order, which CSV headers and
  XML children need (finding 11). `content/encode` never sees the shape, so an empty CSV has no header row
  (finding 12). A component's decode errors reach the user as ordinary "expected a value" mismatches
  (findings 13, 14).
- **The engine records nothing about the components that took part.** Not their versions, and not the
  digest that says which bytes decoded a body (finding 15).
- **Out of process, "recreate the instance" kills every instance in the process** (finding 18).

## 6. The charter's success criteria

| Criterion | Outcome |
|---|---|
| 1. One engine, thin SDKs | **Met** (M4). Both SDKs pass 47 of 47 conformance cases against one engine; the thinness audit is a CI gate |
| 2. No FFI failure modes | **Met.** No per-object cleanup (sessions are the only resource), errors are values at the boundary, no SDK orchestrates differently. Two gaps remain in the SDKs' hands: whether a test passed (finding 4) and contract bytes (finding 6) |
| 3. Plans carry the semantics | **Met** (M1, M3). 583 of 583 in-scope pact-specification cases agree; 158 are named known gaps (XML bodies, four rules with no coverage) |
| 4. Optionality is answered | **Met** (M2). The RFC's order payload in one test, 8 variants of 24, a careless consumer fails on an identifiable variant, and only exercised variants are recorded |
| 5. The loop closes | **Met** (M5). SHIPPED/PENDING caught, fixed by widening, and the widened shape forced into the consumer's variants |
| 6. Components are real | **Met** (M6). The CSV component, written from the docs, runs in a consumer test and in verification |
| 7. Performance is characterised | **Met** (9.1). The answer changed the embedding decision instead of confirming it |

## 7. What task 9.2 changed

Decisions (only those the RFC's text depends on):

- [ADR 0023](decisions/0023-the-subprocess-is-the-primary-embedding-and-wasm-serves-offline-operations.md)
  supersedes ADR 0003: the subprocess is every SDK's primary embedding; WASM serves offline operations; the
  zero-import core module is withdrawn.
- [ADR 0024](decisions/0024-request-dimensions-stay-pinned-and-a-cardinality-point-matches-a-region.md):
  request dimensions stay pinned; a cardinality point is matched as a region. Implemented, with corpus case
  `shapes/cardinality-pinned-region` (finding 25).
- [ADR 0025](decisions/0025-a-provider-shape-entry-may-select-interactions-by-operation.md): a
  provider-shape entry may carry a `selector` and is matched by operation when no description matches.
  Implemented, with `matched-by` in the report (spike 7.3 §2, recorded as finding 32).

Fixes with no open design question:

| Finding | Change |
|---|---|
| 2 | `janus upgrade` reports a bare `type` on an array as `rule-narrowed`: v1–v4 admitted `[]` and the contract does not |
| 9 (option A) | Subsumption decides an enumerable provider against a string-valued conservative consumer, using the consumer's own matcher. The sample provider's artifact `review` is gone |
| 19 | Component-interfaces spec §9.3: out of process, the `env` grant MUST be enforced |
| 27 | The HTTP transport polls outside its lock, and `stop` wakes the poll. `finalise` went from 200.4 ms to 0.18 ms |
| 30 | `janus` and `janus-engine` build their engine from one function, so the SDKs' engine now runs `exec` and `http` hooks |

## 8. Carried to 9.4

Open, with what the prototype learned (each has an entry in the [findings list](phase-9-findings.md)):

- **Protocol:** per-pair verification counts (10); a consumer's test verdict and per-interaction results
  reaching the engine (4, 5); contract bytes the engine writes (6); `explain` with components and
  captured values (23, 31).
- **Content components:** the fragment model and operator-level substitution (21–24); member order and
  `encode`'s missing shape (11, 12); decode errors and degradations reaching the user (13, 14); recording
  the components that took part (15).
- **Shapes:** a media-type operator owned by the HTTP component (1); the empty-array default (2); a
  `provenance` selector for exemptions (9B).
- **Out-of-process components:** registry credentials (16), `wkg`-published artifacts (17), recreation in
  a shared process (18), the `subprocess` command's form (20).
- **Performance debt:** re-arming recompiles its plan (28); the benchmark harness in CI as a trend line,
  which plan §14 promised and 9.1 found had never run.
- **Kernel structure:** the `request`/`response` part names and the `500` mismatch reply (kernel-boundary
  review 8, 9), before any message transport.
- **Embedding:** per-OS distribution of `janus-engine` for each SDK (ADR 0023 decision 4); whether
  `benchmarks/janus/engine-wasm/` graduates into `engine/`.
