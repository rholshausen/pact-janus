# Pact Janus — Prototype Project Plan

This plan sequences the work to prototype the [Pact MkII RFC](https://github.com/pact-foundation/roadmap/pull/146)
in this repository. It covers design and exploration tasks as well as implementation, because the point of
the prototype is to **produce evidence for the RFC's open questions**, not just code.

Task types: **[design]** produces a spec/ADR for review · **[spike]** time-boxed experiment, throw-away code
allowed, produces a written finding · **[explore]** survey/investigation, produces a report ·
**[build]** production-intent prototype code · **[gate]** a decision point that later phases depend on.

---

## 1. What the prototype must prove

The RFC makes five big bets. Each phase below exists to test one or more of them:

| # | Bet | Falsifiable claim to test |
|---|-----|---------------------------|
| B1 | One core, thin SDKs | A coarse-grained document protocol over WASM/subprocess embeddings removes the FFI failure modes (memory cleanup, panic boundaries, async bridging, orchestration divergence), and an SDK really is thin (bindings + DSL sugar only). |
| B2 | Declarative specs, compiled plans | The plan compiler/interpreter (v2 matching engine from pact-reference) can express both v1–v4 matching-rule semantics and the new shape semantics, and `explain` makes matching inspectable. |
| B3 | Everything is a component | The built-in HTTP transport and JSON content handler can be implemented behind the same interfaces a third-party WASM component uses, without crippling performance or ergonomics. |
| B4 | Shapes + variant testing | Optionality/polymorphism is testable with one test and a managed variant matrix, with acceptable test runtime and understandable failures; and subsumption against a provider shape is decidable enough to be useful. |
| B5 | Scriptable lifecycle | Named hooks with declarative config cover the real auth/state/message cases that request filters and callbacks cover today. |

**Prototype non-goals** (explicitly out of scope; noted as design-only where the RFC needs an answer):
production hardening; the full SDK fleet (two SDKs only); broker/PactFlow server-side changes (subsumption
runs locally/CI-side); full transport set (HTTP + one message-ish stretch; gRPC transport is a stretch
goal); `pact upgrade` beyond a basic v3/v4→v5 conversion; the AI-assisted layer (design notes only);
deprecation/migration timelines.

---

## 2. Phase overview and sequencing

```
Phase 0  Foundations ──┐
Phase 1  De-risking spikes ──► G1 (IDL + embedding decisions)
                                │
Phase 2  Core design round ◄────┘   (overlaps with late Phase 1)
                │
Phase 3  Engine kernel: match & explain ──► M1
                │
Phase 4  Consumer flow + variants ──► M2 ──┐
                │                          │
Phase 5  Provider verification ──► M3      │
                │                          │
Phase 6  Thin SDKs + conformance ──► M4 ◄──┘
                │
Phase 7  Provider shapes & subsumption ──► M5
                │
Phase 8  External components ──► M6
                │
Phase 9  Evaluation & RFC feedback
```

Phases 1 and 2 overlap heavily (spike results feed the designs). Phases 4/5 share the kernel from Phase 3
and can interleave. Phase 7 depends only on the shape model (Phase 2/3) plus the verifier (Phase 5) and can
start early on the checker itself. Durations are deliberately omitted at task level — each phase lists an
indicative size instead (S/M/L relative to the others); treat them as sequencing aids, not commitments.

Milestones (each is demo-able):

- **M1** — CLI compiles and `explain`s plans for both a v4 pact and a new-style interaction spec, and
  matches captured values offline.
- **M2** — The RFC's TypeScript example, expressed as a raw interaction-spec document, runs against the
  engine's mock server with the 12-variant space sampled, and a v5 pact file is written.
  - **M3** — The same engine binary verifies that v5 pact *and* an existing v4 pact against a sample
    provider, with auth and state hooks.
- **M4** — The RFC consumer example runs near-verbatim in TypeScript and on the JVM, both passing the
  seed conformance suite with identical behaviour.
- **M5** — The RFC's "provider may produce SHIPPED, consumer only tested PENDING" scenario is reproduced
  end-to-end and reported exactly as the RFC sketches.
- **M6** — A third-party WASM component (matcher or content handler) runs unmodified in both a consumer
  test and verification.

---

## 3. Phase 0 — Project foundations (size: S)

Goal: a place to put decisions and code, and an honest inventory of what can be reused.

- **0.1 [design] Prototype charter.** Write down the success criteria (§1 above, refined), the non-goals,
  and what "prototype complete" means. One page. This is the yardstick for every later scope argument.
- **0.2 [design] Decision log structure.** Set up `Documentation/decisions/` as an ADR log
  (`NNNN-title.md`, status: proposed/accepted/superseded). Every [gate] and every contested design choice
  lands here. The RFC's "Unresolved questions" list seeds the index (see §13 traceability table).
- **0.3 [build] Repo scaffolding.** Rust workspace (`engine/` with kernel + component crates, `cli/`,
  `sdks/`, `corpora/`, `spikes/`), CI (build, test, clippy, fmt), basic contributor docs. Spike code lives
  in `spikes/` and is allowed to rot; everything else is kept green.
- **0.4 [explore] Reuse inventory of pact-reference and pact-jvm.** Systematically assess what Janus can
  import, vendor, or must rewrite: `pact_models` (v1–v4 file model), the v2 matching engine prototype in
  `pact_matching/src/engine` (plan nodes, interpreter, pretty/executed rendering), mock server,
  verifier internals, `pact-compatibility-suite`, pact-plugins driver. Output: a report per crate/module —
  *reuse as dependency / fork and adapt / rewrite* — with the licensing and coupling notes to justify it.
  **Recommendation to validate:** depend on `pact_models` for v1–v4 reading, fork the v2 engine code as
  the kernel starting point, treat everything else as reference material.

---

## 4. Phase 1 — De-risking spikes (size: M, parallelisable)

Goal: settle the two decisions everything else sits on (protocol IDL, embedding strategy) with evidence,
and get a performance baseline before any architecture ossifies. Each spike is time-boxed and ends in a
written finding even if the answer is "it doesn't work".

- **1.1 [spike] Protocol IDL bake-off — evolution first, and wider than WIT vs protobuf.** The RFC
  names WIT and protobuf, but neither arrives with a mandate: protobuf's presence in pact-plugins is
  historical rather than an endorsement, and WIT — despite working well in commercial use — has a known
  sharp edge where adding a variant/enum case later breaks components compiled before the case existed.
  That makes **compatibility under evolution** the deciding criterion, and it differs by surface:
  - *SDK-facing engine protocol*: SDK and engine versions are pinned and negotiated; both ends move.
  - *Plugin-facing component interfaces*: third-party components compiled long ago must keep loading as
    the interface grows. "Add an enum variant" must never break an old plugin, and minting a new
    interface version per addition is overhead that punishes evolution. The bake-off may legitimately
    return different answers for the two surfaces.

  Candidates: WIT (including whether `@since`/`@unstable` feature gates rescue additive evolution);
  protobuf (open enums, unknown-field preservation); FlatBuffers and Cap'n Proto (numbered-field
  evolution); Avro (reader/writer schema resolution); Smithy and TypeSpec (authoring IDLs with
  evolution validators — `smithy-diff` can make "no breaking changes" CI-enforceable — emitting other
  encodings); and the **document-first hybrid**: a tiny, frozen byte-pipe interface (minimal WIT world /
  C ABI / stdio) carrying JSON or CBOR documents governed by a versioned schema (JSON Schema / CDDL)
  with explicit open-world rules — must-ignore unknown fields, open enums, capability negotiation à la
  LSP — moving evolution out of the type system and into document semantics the engine controls.

  Method: (a) define an **evolution gauntlet** — add enum variant, add optional field, add operation,
  add event type, widen a union; old artifact against new interface and the reverse; (b) shortlist on
  paper against the gauntlet and tooling reality; (c) model the protocol slice (`add-interaction` with
  structured errors + the `verify` event stream) in the top 2–3 and generate Rust/TypeScript/JVM
  bindings; (d) run the gauntlet for real on compiled artifacts. Compare expressiveness for
  documents/streams/errors-as-values, codegen quality, and gauntlet results per surface. Feeds G1 and
  RFC unresolved question "IDL choice".
- **1.2 [spike] WASM component embedding matrix.** Compile a toy engine (echo + one real operation, e.g.
  a trivial matcher) as a WASM component. Host it in: Node (built-in), JVM (Chicory), Go (wazero),
  Python (wasmtime-py), .NET (wasmtime-dotnet if time allows). For each: does the component model work or
  only core WASM + shims; binding ergonomics; cold-start and per-call overhead; payload marshalling cost
  for a ~100 KB JSON document. Output: per-language embedding matrix — the RFC asserts "WASM preferred"
  and this either confirms it or scopes the subprocess fallback per language.
- **1.3 [spike] Subprocess embedding.** The same toy engine as a `pact-engine` executable speaking the
  protocol over stdio with LSP-style framing. Prove: clean spawn/shutdown from a test runner, no orphaned
  processes on test-runner kill, protocol version handshake, Windows behaviour. This is the fallback that
  makes B1 safe, so it must be shown to work early, not assumed.
- **1.4 [spike] Engine hosting WASM components as plugins.** The inverse of 1.2: native Rust engine
  (wasmtime) loading a WASM component that implements a custom matcher action invoked from a plan.
  Tests the component-model story for B3 before Phase 2 designs the interfaces around it.
- **1.5 [spike] Message-transport shape test.** Paper + toy-code check that the draft component interfaces
  (transport as "start/stop endpoint, drive requests, map wire↔abstract parts") actually fit an async
  message transport, not just HTTP. Cheap now; expensive to discover in Phase 8.
- **1.6 [spike] Script-hook engine bake-off.** Hooks need a low-friction scripting path (the
  `wasm-script` sketch in the RFC), and user feedback from prior Lua embeddings is that people want
  JS/TS. The crux is an architectural constraint, not language taste: **the script runtime must work in
  all three engine embeddings**, and if the engine itself ships as a WASM component, a natively-embedded
  V8/SpiderMonkey cannot live inside it. Candidates to evaluate:
  - *Lua*: `mlua` (native, proven, tiny) — check whether Lua-compiled-to-WASM keeps it viable inside the
    WASM embedding; pure-Rust `piccolo` as a maturity check.
  - *JS-engines-compiled-to-WASM*: QuickJS via the Javy toolchain (small, fast start);
    StarlingMonkey/ComponentizeJS (SpiderMonkey as a WASM component — component-model native, heavier).
    These give the JS/TS experience without embedding V8 natively.
  - *Pure-Rust JS*: Boa — compiles wherever the engine compiles; assess spec coverage and speed.
  - *Native V8/SpiderMonkey* (`rusty_v8` / `mozjs`): benchmark as the ceiling, but note it is only
    available in the subprocess/native embeddings — if chosen, scripted hooks become a
    "subprocess-embedding only" feature, which is a real design fork to make explicit.
  - *Rust-native DSLs* (Rhai, Starlark) as a control group.

  TS support is a transpile question in every option (esbuild/swc at load time or a precompile step), so
  evaluate it as tooling, not engine choice. Criteria: works in all three embeddings; sandboxing and
  capability control (no ambient fs/network unless granted); per-invocation overhead and cold start on a
  realistic hook (sign a request, mutate headers); distribution size; hook-context API ergonomics;
  maintenance risk of the embedding. "Bring your own precompiled WASM component" remains the polyglot
  escape hatch regardless — this spike picks the low-friction default. Feeds design 2.7 (not G1).
- **1.7 [build] Benchmark baseline.** Harness that measures today's stack (pact_ffi mock server
  throughput/latency, verification wall-time on a corpus of pacts). Kept running against Janus from
  Phase 4 on, so "performance envelope of WASM vs native FFI" (RFC unresolved question) is answered with
  a trend line, not a one-off.
- **1.8 [gate] G1 — Decisions: protocol IDL; embedding priority per language; kernel starting point.**
  ADRs for: IDL choice (or the "IDL-neutral document schema + per-embedding encoding" hybrid if the
  bake-off points there); which embedding is primary for which SDK language; adopt-or-rewrite for the v2
  engine code (from 0.4). Phase 2 designs are written against these decisions.

---

## 5. Phase 2 — Core design round (size: L, mostly parallel, review-gated)

Goal: turn the RFC's sketches into reviewable specifications. Each item is a spec/ADR with worked
examples, reviewed before its consumers in later phases build against it. These documents are the seeds of
the "executable specification" — they graduate into it as golden corpora and suites attach to them
(Phases 3 and 6).

- **2.1 [design] Engine Protocol specification.** Session model, full operation set (consumer-session,
  verification, explain, upgrade), event/stream semantics, structured error taxonomy, protocol version
  negotiation, and the compatibility policy (what an SDK pinned to protocol N can expect from engine N+1).
  Deliverable: the IDL file(s) + prose semantics. Explicitly design *errors as values* and *no per-object
  cleanup* — the two FFI lessons the RFC calls out.
- **2.2 [design] Shape language specification.** The operator set (`type/regex/datetime/…`, `optional`,
  `nullable`, `anyOf`, `oneOf`, `eachLike` + cardinality, `forbidden`), composition rules, the JSON
  encoding inside interaction specs, semantics of *admits(shape)* (needed by subsumption), and each
  operator's variant dimension. Include the must-ignore-extra-fields default and how `forbidden` overrides
  it. Worked example: the RFC's order payload.
- **2.3 [design] Variant semantics and sampling.** Variant-space computation; pairwise sampling algorithm
  choice (evaluate existing covering-array approaches vs a simple greedy pairwise); the exhaustive
  threshold and hard caps; explicit pinning; the `whenVariant` provider-state linkage (the RFC marks this
  a sketch — this task designs it properly, including what happens when a variant needs a state the
  provider can't produce). Answers RFC unresolved question "variant sampling defaults".
- **2.4 [design] Plan grammar and core action set.** Freeze a v0 of the node grammar (containers, actions,
  values, resolvers, pipelines) and the core actions (`match:*`, `expect:*`, `convert:*`, …); namespacing
  rules for component-contributed actions and plan fragments; the pretty/`--executed` text forms; and a
  *versioning and stability policy* (RFC lists this as an implementation unknown — the prototype should
  propose one and stress it in Phase 8 when plugins contribute fragments). Define the golden-corpus
  format: (input spec or pact, expected plan, expected result against captured values).
- **2.5 [design] Pact file format v5.** Schema: interaction description, typed provider-state parameters,
  transport binding, parts with shape + exercised example variants, component requirements
  (`content/protobuf >= 2`), metadata. Rules for v3/v4 → v5 conversion (matching rules become shapes;
  the single example becomes the sole variant). Broker compatibility notes (self-contained JSON the
  current broker can store even if it can't render it).
- **2.6 [design] Component interfaces.** The four interfaces (transport, content, matcher/generator,
  hook) in the chosen IDL, with the in-tree/out-of-tree symmetry rule from the RFC: built-ins implement
  exactly these. Decide the prototype's answer to "is everything a component on day one, or are HTTP/JSON
  kernel-privileged initially" — the prototype should *try* full symmetry and report where it hurts
  (that report is the evidence the RFC question needs). Include the out-of-process gRPC escape hatch
  boundary and the OCI distribution model (design only; minimal implementation in Phase 8).
- **2.7 [design] Lifecycle hooks.** Named hook points (`before-request`, `state-setup`,
  `produce-message`, `consume-message`, `after-verification`, …), the declarative config schema
  (`verifier.pact.yaml` and its consumer-side equivalent), hook implementations (built-in components,
  `exec`, HTTP endpoint, scripted), ordering/failure semantics, and secret handling
  (`${AUTH_URL}`-style interpolation). Includes the **scripted-hook ADR** from spike 1.6: the default
  script language/runtime, the hook-context API scripts see (request/response/state objects, what is
  mutable), and how a script hook is packaged and referenced from config — with "precompiled WASM
  component" as the polyglot escape hatch alongside it.
- **2.8 [design] Subsumption check.** The `admits(provider) ⊆ admits(consumer)` walk; the asymmetric
  finding rules (extra fields OK unless `forbidden`; wider value spaces, broader types, weaker presence
  are findings); the *decidability degradation ladder* — which comparisons are exact (enums, presence,
  discriminated unions), which are conservative (regex vs regex, datetime formats), and when the checker
  reports "unknown, review manually" instead of guessing (RFC unresolved question). Finding output format
  (the RFC's `can-i-deploy` rendering) and the warn/block policy surface, including exemption scoping
  (per field / interaction / consumer).
- **2.9 [design] SDK specification template.** What a Janus SDK is: generated bindings + idiomatic DSL +
  test-framework integration + conformance run. The canonical behavioural spec format and per-language
  style-guide skeleton that Phase 6 instantiates twice and that the AI-assisted regeneration idea would
  consume. Includes the compatibility-facade guidance (how much of today's DSL surface each SDK keeps).
- **2.10 [design] AI-layer design notes (no build).** One short doc positioning `explain --executed`
  traces as diagnosis input and the agentic-verification task format, so the deterministic design doesn't
  accidentally preclude them. Explicitly deferred beyond that.

---

## 6. Phase 3 — Engine kernel: match and explain (size: L)

Goal: the heart of B2 — specs in, plans out, plans executable and inspectable — *before* any transport or
process machinery, so matching is testable offline against corpora.

- **3.1 [build] Pact model integration.** v1–v4 pact reading via the Phase 0.4 decision (likely
  `pact_models` as a dependency). v5 model per design 2.5 (read and write).
- **3.2 [build] Interaction-spec document model.** Parser/validator for the declarative interaction spec
  (description, states, parts, shapes per 2.2), with structured errors good enough to surface through the
  protocol to a DSL user.
- **3.3 [build] Plan compiler — shapes → plans.** HTTP request/response parts and JSON bodies first.
  Every shape operator from 2.2 compiles; variant dimensions are represented so Phase 4 can enumerate
  them.
- **3.4 [build] Plan interpreter and value resolvers.** Adapt the v2 engine prototype: plan node
  execution, resolvers for HTTP request/response and a captured-value form (for offline corpus tests).
- **3.5 [build] Plan compiler — v1–v4 matching rules → plans.** The single home of cascading/precedence
  semantics. Validate by running the compiled plans against the existing `pact_matching`/specification
  test cases and diffing verdicts against the current implementation — this is the strongest evidence
  available that plans can carry the old semantics.
- **3.6 [build] `explain` and `--executed`.** Pretty plan rendering and executed-plan annotation as kernel
  operations, exposed through the CLI skeleton.
- **3.7 [build] Golden corpora v0.** Check in the corpus (per 2.4's format) covering: each shape operator,
  representative v2/v3/v4 matching-rule constructs, and the RFC's order example. CI runs plans against
  corpora on every change; this is executable-specification artifact #2 taking shape.
- **3.8 [explore] Kernel-boundary review.** Short written check after 3.1–3.6: did HTTP/JSON knowledge
  leak into the kernel (violating B3)? Findings feed 2.6's day-one-components answer and Phase 4's
  transport work.

**Milestone M1** closes this phase.

---

## 7. Phase 4 — Consumer flow end-to-end with variants (size: M)

Goal: the consumer side of B1 and B4, driven through the real protocol (no SDK yet — a thin test client
speaks the protocol directly, proving the protocol is sufficient before DSLs exist).

- **4.1 [build] Session lifecycle.** `create`/`add-interaction`/`finalise` per 2.1: sessions as the only
  resource, structured errors, per-interaction results.
- **4.2 [build] HTTP transport component (mock side).** Mock server implementing the transport interface
  from 2.6 — in-tree, but through the interface. JSON content component likewise.
- **4.3 [build] Variant machinery.** Variant-space computation from compiled plans, pairwise sampler per
  2.3, `variants`/`serve-variant` operations, generators producing each variant's concrete payload.
- **4.4 [build] v5 pact writing.** Shape once + concrete example per exercised variant; only exercised
  variants recorded (the honesty rule).
- **4.5 [build] Protocol-level consumer test harness.** Rust/TS integration tests that play the SDK role
  over the protocol: submit the RFC order interaction, iterate variants, run a real HTTP client against
  the mock per variant, finalise, assert the pact file. Also exercises 1.2/1.3 embeddings against the
  real engine, not the toy.
- **4.6 [explore] Variant ergonomics report.** Deliberately write consumer code that mishandles a variant
  (crashes on absent `shippedAt`) and evaluate the failure experience: is the failing variant identifiable?
  Is the sampled matrix understandable? Sharp edges here are an RFC "drawbacks" item — document them
  honestly.

**Milestone M2** closes this phase.

---

## 8. Phase 5 — Provider verification (size: M)

Goal: the verifier as *the same engine*, replaying variants, with hooks (B5), and honouring the migration
promise (old pacts verify).

- **5.1 [build] `verify` operation.** Source (pact file/dir; broker fetch is a stretch) → target (running
  provider) with the event stream from 2.1; results per interaction per variant.
- **5.2 [build] Request-variant replay and response shape matching.** Every recorded request variant
  replayed; responses matched with optional/`oneOf`/discriminator semantics; variant-pinned provider
  states (`whenVariant` per 2.3) passed to state setup.
- **5.3 [build] Hooks.** `state-setup` (exec + HTTP endpoint) and `before-request` (an oauth2-shaped
  built-in component + exec), configured from `verifier.pact.yaml` per 2.7. One scripted hook using the
  runtime chosen in 1.6/2.7 (e.g. a JS `before-request` that signs the request) as a stretch, to prove
  the third implementation kind end-to-end.
- **5.4 [build] v3/v4 verification.** Verify an existing real-world pact (e.g. from an example project)
  through the plan path from 3.5 — the "providers upgrade first at no cost" claim, demonstrated.
- **5.5 [build] CLI.** `pact verify`, `pact explain` (incl. `--executed` after a failure), `pact upgrade`
  (basic v3/v4→v5 per 2.5). Same engine, subprocess embedding — this doubles as the 1.3 result hardened.
- **5.6 [build] Sample provider.** A small order-service provider (any convenient stack) with deliberate
  variance and auth, used for M3, M5 and demos.

**Milestone M3** closes this phase.

---

## 9. Phase 6 — Thin SDKs and conformance (size: L)

Goal: prove B1's second half — that SDKs are thin, generated where possible, and behaviourally identical.
Two languages chosen to cover both hard embedding stories: **TypeScript** (matches the RFC's example;
Node WASM hosting) and **JVM** (Chicory; also the politically important pact-jvm convergence case).
Revisit this choice at G1 if the embedding matrix says otherwise.

- **6.1 [build] Binding generation pipeline.** IDL → generated protocol bindings for TS and JVM, wired
  into CI so regeneration is a command, not a chore.
- **6.2 [build] TypeScript SDK.** Idiomatic DSL reproducing the RFC consumer example (`optional`, `anyOf`,
  `oneOf`, `eachLike`, `pact.execute` with the variant-aware closure), Jest/Vitest integration, WASM
  embedding with subprocess fallback.
- **6.3 [build] JVM SDK.** Same behavioural surface, JUnit 5 integration, Chicory embedding with
  subprocess fallback. Deliberately written *from the 2.9 SDK spec* rather than by porting the TS code —
  that tests whether the SDK-spec approach actually transmits behaviour.
- **6.4 [build] Conformance suite seed.** Grow from `pact-compatibility-suite`: scenarios covering
  DSL→spec translation, session lifecycle, variant iteration, pact output equality. Both SDKs pass it in
  CI against a pinned engine — executable-specification artifact #3.
- **6.5 [explore] Thinness audit + AI-regeneration trial.** Measure each SDK: LOC by layer
  (generated/idiomatic/integration), and specifically what had to be hand-written that the RFC claims
  shouldn't exist (matching logic? orchestration?). Then trial the AI-assisted flow: change the SDK spec
  (add a small DSL feature), have an agent regenerate the idiomatic layer in both languages, measure
  review burden and conformance results. This is direct evidence for the RFC's "one team, eight SDKs"
  claim.

**Milestone M4** closes this phase.

---

## 10. Phase 7 — Provider shapes and subsumption (size: M)

Goal: B4's second half — the loop that catches undeclared variance. The checker itself only needs the
shape model, so 7.1 can start any time after Phase 3.

- **7.1 [build] Subsumption checker.** Implement 2.8: structural walk, asymmetric findings, the
  degradation ladder with explicit "unknown, review manually" verdicts. Property-test it: generate value
  spaces, check `admits` agreement between the checker's verdict and brute-force sampling.
- **7.2 [build] Provider self-contract recording (provenance #1).** Engine mode that records the union of
  response shapes produced by the provider's own tests into a provider-shape artifact. Demonstrate with
  the Phase 5.6 sample provider.
- **7.3 [spike] Type-derived shapes (provenance #2).** Import one format — recommend OpenAPI (broadest
  reach; protobuf as the alternative if a gRPC stretch happens) — into provider shapes via a content
  component. Evaluate the RFC's over-broadness worry (everything-nullable ORM schemas) on a realistic
  spec and document how noisy the findings get; that finding directly informs the warn-vs-block default.
- **7.4 [build] Compatibility CLI.** `pact check` (working name): combine verification results +
  subsumption findings into the RFC's `can-i-deploy`-style report, with `--policy warn|block` and
  exemption scoping per 2.8. Local/CI only — broker integration is design notes for the RFC, not
  prototype code.
- **7.5 [design] Broker integration notes.** What the broker would need to store/render/decide for v5
  artifacts, provider shapes and subsumption results (RFC implementation unknown "broker/PactFlow
  handling of v5"). Written with the 7.1–7.4 experience in hand.

**Milestone M5** closes this phase.

---

## 11. Phase 8 — External components (size: M)

Goal: B3 beyond the in-tree case — the extension path as the well-trodden path.

- **8.1 [build] Third-party WASM component.** An out-of-tree content or matcher component (CSV or XML is
  a good candidate — real, small, historically a plugin) built against the published component interfaces,
  loaded by version from project config, working in both consumer tests and verification. Written by
  following the docs only — track every point where reading engine source was necessary, as a docs/design
  finding.
- **8.2 [build] OCI distribution, minimal.** Push/pull the 8.1 component as an OCI artifact; engine
  resolves, caches and integrity-checks it per the 2.6 design. Registry can be a local one.
- **8.3 [spike] Out-of-process transport escape hatch.** A gRPC-based transport component out-of-process
  (today's pact-plugins model as the retained escape hatch). Stretch: an actual gRPC *protocol* transport
  reusing it. Scoped tightly — the goal is to prove the boundary exists, not to rebuild pact-protobuf.
- **8.4 [explore] Plan-fragment stress test.** Have the 8.1 component contribute plan fragments and a
  custom action; check the 2.4 versioning policy holds up (what happens when the component targets plan
  grammar v0 and the engine moves to v0.1?).

**Milestone M6** closes this phase.

---

## 12. Phase 9 — Evaluation and RFC feedback (size: S/M)

Goal: convert the prototype into the RFC's next revision and a credible staged plan for the real build.

- **9.1 [build] Performance report.** Janus (WASM and subprocess embeddings) vs the 1.7 baseline: mock
  throughput, verification wall-time, variant-matrix overhead, cold start per test file. Answers the
  RFC's performance-envelope unknown with numbers.
- **9.2 [design] Unresolved-questions resolution.** Walk the RFC's list (see §13); for each, an ADR or an
  honest "still open, here's what we learned". Update/annotate the RFC (or draft its successor) with
  prototype evidence, including the drawbacks that materialised (variant sharp edges from 4.6, subsumption
  noise from 7.3, WASM host gaps from 1.2).
- **9.3 [doc] Demo and report.** A demo repo/screencast walking M2→M5 (consumer test with variants →
  verification with hooks → subsumption finding → fix by widening the shape → variants force the consumer
  to handle it — the full RFC loop), plus a short prototype report for the community.
- **9.4 [design] Staged implementation plan for the real build.** Informed by everything above: what
  graduates from Janus, what gets rewritten, ordering, and the governance/naming questions surfaced for
  the community process (those are decided by the community, not this prototype — but the prototype should
  frame them).

---

## 13. Traceability: RFC unresolved questions → plan tasks

| RFC unresolved question | Addressed by |
|---|---|
| IDL choice (WIT vs protobuf) + WASM-host story per language | 1.1, 1.2, 1.3 → G1 (1.8) |
| Components day-one vs HTTP/JSON kernel-privileged | 2.6, 3.8, 4.2, 8.1 |
| Variant sampling defaults, caps, overrides | 2.3, 4.3, 4.6 |
| Provider-state/variant linkage (`whenVariant`) | 2.3, 5.2 |
| Subsumption warn/block default + exemption scoping | 2.8, 7.3, 7.4 |
| Subsumption decidability limits | 2.8, 7.1 |
| Plan grammar stability/versioning policy | 2.4, 8.4 |
| Broker handling of v5 artifacts | 2.5, 7.5 |
| Performance envelope WASM vs native FFI | 1.7, 9.1 |
| Message-interaction hook design | 1.5, 2.7 (design); build deferred beyond prototype |
| Naming/versioning (v5 + "Pact 6" vs new brand); governance/funding | Out of prototype scope; framed for the community in 9.4 |

---

## 14. Cross-cutting practices

- **Decision log** (`Documentation/decisions/`): every gate and contested choice is an ADR; the RFC
  discussion thread gets linked so community input lands in the record.
- **Corpora are load-bearing**: from Phase 3 on, matching behaviour changes require a corpus change in the
  same commit; corpora + protocol IDL + conformance suite are the executable specification growing in
  place.
- **Benchmarks in CI** from Phase 4 (trend, not gate).
- **Spikes are disposable, findings are not**: every `spikes/` entry has a `FINDINGS.md`; code may be
  deleted, the finding is referenced by ADRs.
- **Honesty about drawbacks**: 4.6, 7.3 and 6.5 exist specifically to gather evidence *against* the RFC's
  bets where it exists — a prototype that can't fail can't de-risk anything.

## 15. Immediate next steps

1. Review/adjust this plan (especially the non-goals in §1 and the SDK language pair in the Phase 6 intro).
2. Execute Phase 0: charter (0.1), ADR log (0.2), workspace scaffolding (0.3), reuse inventory (0.4).
3. Kick off spikes 1.1 and 1.2 — everything downstream waits on G1.
