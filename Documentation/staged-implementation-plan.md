# Staged implementation plan for Pact MkII

Plan task **9.4**, the prototype's last. **Date:** 2026-09-24. **Inputs:** [RFC feedback](rfc-feedback.md)
(9.2), the [community report](prototype-report.md) and [demo](../demo/README.md) (9.3), the
[performance report](performance-report.md) (9.1), the [Phase 9 findings](phase-9-findings.md), the
[decision records](decisions/README.md) and their tripwires, and the [reuse inventory](reuse-inventory.md)
(task 0.2).

This is a plan for building the real thing, written by the people who built the prototype. It says:
- what should carry over from Janus, and what should not;
- the order to build in, and how to tell each stage is done;
- which decisions are deliberately left open, and what will reopen them;
- which questions belong to the community rather than to this plan.

It is a proposal for the RFC process, not a commitment on anyone's behalf.

## Contents

1. [Principles](#1-principles)
2. [What carries over from Janus](#2-what-carries-over-from-janus)
3. [The stages](#3-the-stages)
4. [Reassessment gates](#4-reassessment-gates)
5. [Known debt, by stage](#5-known-debt-by-stage)
6. [Risks](#6-risks)
7. [Questions for the community](#7-questions-for-the-community)
8. [Closing the prototype](#8-closing-the-prototype)

## 1. Principles

Each of these is something the prototype learned, not a preference.

1. **Providers first.** A provider switches verifiers at no cost if the new one verifies every existing
   pact exactly as the old one did. Consumers can then adopt the new format without breaking a single
   provider. That is the RFC's migration path, and the prototype showed it is reachable: v1–v4 pacts
   compile to plans that agree with the Pact specification's test cases on 583 of 583 in scope, and
   verify unchanged against a provider that was never modified (tasks 3.5, 5.4).
2. **The protocol is the compatibility surface, not the embedding.** SDKs talk to the engine through
   schema-governed frames over a frozen pipe (ADR 0002). That is what let the WASM-versus-subprocess
   decision change late without touching either SDK's idiomatic layer (ADR 0023), and it is what keeps
   the reassessment in §4 cheap.
3. **The specification grows with the code.** The specs, their schemas, the golden corpora and the
   conformance suite are the executable specification the RFC promised. In Janus every behaviour change
   landed with a corpus or conformance change, and the conformance suite found an engine bug that the
   engine's own tests had pinned (finding 34). The real build keeps that rule from its first commit.
4. **Thin SDKs are enforced, not hoped for.** A CI audit counts every SDK source file by layer and fails
   the build when a hand-written layer outgrows its budget (task 6.5). SDKs stay thin because the build
   says so.
5. **Decide what the evidence decides; name the rest.** Janus left some questions open on purpose
   (§4), each with the event that should reopen it. The real build should keep those open until that
   event happens, and not settle them by accident in code.

## 2. What carries over from Janus

Janus was built to be thrown away, and most of its *code* can be. What carries over is mostly the
*specification*: the documents, schemas and test corpora that say what the engine must do, whoever
writes it. Much of the kernel is worth keeping too, because it was held to those specifications
throughout. "Graduates" means it moves into the real build and is hardened there. "Seed" means it is the
first draft of something that will be rewritten against the final specification.

| Janus part | Verdict | Why, and what it needs |
|---|---|---|
| **Specifications and schemas** (`Documentation/specs/`: engine protocol, shape language, variant semantics, plan grammar, contract file, component interfaces, lifecycle hooks, subsumption check, SDK specification) | **Graduates**, as the v1 drafts of the executable specification | Every one was implemented against, and every change since Phase 2 was made in the spec first. They need community review, and the open items in §5 folded in. Where they live is a community question (§7) |
| **Golden corpora, conformance suite, and their checkers** (`corpora/`, `conformance/`, `tools/schema-compat`, `tools/corpus`, `tools/conformance`, `tools/thinness`) | **Graduates** | These are what "conformant" and "compatible" mean. The pact-specification test cases the legacy harness runs should join them, with the 158 known gaps closed rather than skipped |
| **Kernel** (`engine/kernel`: plan compiler and interpreter, legacy v1–v4 compiler, shapes, variants, subsumption, contract model, upgrade, protocol dispatcher) | **Graduates**, with hardening | About 17,000 lines, all of it under the specs and corpora. The interpreter and plan grammar are the pact-reference v2 engine, forked with provenance (ADR 0004). Before it is the real kernel: remove the two HTTP-shaped assumptions (kernel-boundary review 8, 9); make the exchange loop single-threaded and async-ready (§4, gate W); cache per-variant plans (finding 28); rename its actions to plan grammar v1 (ADR 0026) |
| **Built-in JSON content, OAuth2 hook, hooks host** | **Graduate** | Small, specified, tested. The hooks host needs credential redaction reviewed for production |
| **Built-in HTTP transport** (`engine/component-http`) | **Rewrite** | It is threaded (`tiny_http`, one thread per server), was never built for WASM, and has no media-type or list-header semantics (finding 1). The rewrite should be async and thread-free, which serves gate W, and should own an `http:media-type` operator |
| **WASM component host and OCI distribution** (`engine/component-host`, `janus component push/pull`) | **Graduates** | It already does digest pinning, content-addressed caching and a tamper-testing registry. Needs findings 15–17 decided |
| **Out-of-process component loader** (spike 8.3) | **Rewrite, if kept** | Spike code. It proved the boundary exists, and findings 18–20 say what a real one must do differently |
| **CLI** (`janus explain`, `upgrade`, `check`, `component`) | **Graduates** | `verify` graduates too, but it has no broker integration at all (§3, stage 1) |
| **`janus-engine` subprocess binary** | **Graduates** | The primary embedding (ADR 0023). Needs a release pipeline for one statically linked binary per target triple, and real Windows and macOS CI |
| **WASM engine artifact** (`benchmarks/janus/engine-wasm`) | **Graduates as the offline artifact** | Moves into `engine/` with its own CI build, for explain, upgrade and check in hosts that want no native binary (ADR 0023 decision 2) |
| **TypeScript and JVM SDKs** | **Seeds** | They prove the model: generated bindings, a few hundred lines of idiomatic layer, and conformance. But their DSL is Janus's, not the one the community will design, and they have no compatibility facade for today's pact-js and pact-jvm DSLs. Keep the bindings pipeline (`tools/bindings`) and the drivers; rewrite the idiomatic layers |
| **Sample provider, demo, third-party CSV component** | **Graduate as examples and fixtures** | The demo is the clearest end-to-end regression test there is, and CI already runs it |
| **Benchmark harnesses** (`benchmarks/`) | **Graduate** | As the CI trend line plan §14 promised and 9.1 found had never run |
| **Spikes** (`spikes/`) | **Findings only** | Their code is not a starting point, and their findings are already in the ADRs |

The reuse inventory (task 0.2) noted what Janus took from pact-reference. The real build should settle
early whether it lives in pact-reference, beside `pact_matching`'s v2 engine that the kernel forked,
or in a repository of its own (§7).

## 3. The stages

Sizes are relative to each other: **S**, **M**, **L**, **XL**, where an L stage is roughly twice an M.
Each one weighs two things: what the stage needs that Janus does not have, and the hardening its Janus
parts need to become production code. They are not calendar estimates. Janus was built in seven weeks
of heavily AI-assisted work, which says nothing about a volunteer team's pace. They do say which stages
are big.

```
 0 Foundations ──► 1 Provider verifier ──► 2 Consumer engine + JS/JVM ──► 3 Shapes and variants
                                                                               │
                        6 Breadth (more SDKs, messages, gRPC) ◄── 5 Plugins ◄──┴── 4 Provider shapes
```

Stages 4 and 5 can overlap once stage 3's contract format is frozen. Stage 6 starts with SDKs as soon
as stage 2 has proven the SDK model on two languages.

### Stage 0: Foundations — **S**

Where the code lives, who reviews it, and how it ships.

- A home for the engine and the specifications, with owners (§7).
- CI on Linux, macOS and Windows. The prototype ran real Windows only in spike 8.3, and ADR 0023
  makes the subprocess every SDK's primary embedding, so it has to be solid everywhere.
- A release pipeline that builds, signs and publishes `pact-engine` (the name is §7's) for each target
  triple, plus the offline WASM artifact.
- The specifications published as drafts for review, versioned from the start. The plan grammar is
  published as v1, with ADR 0026's naming: every action namespaced, and the core families reserved.

**Done when** a tagged release publishes signed binaries for at least five triples, and the specs are
open for community comment.

### Stage 1: A drop-in provider verifier — **L**

This is the first thing users get, and it delivers the RFC's first migration step: providers switch
verifiers at no cost.

- **Verification of v1–v4 pacts**, exactly as today's verifiers do it, including the 158
  pact-specification cases Janus skips: XML and form bodies, and the four rules with no coverage in the
  corpus.
- **Broker integration, which Janus does not have at all:**
  - fetching pacts by consumer version selectors;
  - pending and work-in-progress pacts;
  - publishing verification results, keyed per consumer/provider pair (finding 10);
  - `can-i-deploy` unchanged.
- **Hooks in place of today's request filters and state-change callbacks** (the prototype's hooks,
  hardened), with a migration note for each old verifier option.
- **Message pacts:** verifying v3/v4 message interactions needs a message transport. That first needs
  the kernel's `request`/`response` part names and its HTTP-shaped mismatch reply removed
  (kernel-boundary review 8, 9). The hook design (spike 1.5, design 2.7) is ready.
- The verifier for the language ecosystems that embed today's: through the subprocess, with the SDKs
  of stage 2 or a thin wrapper before them.

**Done when** the pact-specification cases pass in full, and pilot providers have run the new verifier
beside their current one on real pacts with identical results for a release cycle.

### Stage 2: The consumer engine, and the JS and JVM SDKs — **XL**

The RFC's second migration step: SDKs ship MkII as a new major version.

- **The consumer side of the engine, production-hardened:** sessions, the rewritten HTTP transport
  serving mocks, and contract writing. Janus has all of it in prototype form.
- **Two SDKs, JavaScript/TypeScript and JVM, built from the SDK specification.** The JVM SDK is the
  proof that pact-jvm becomes one SDK among several. Each needs:
  - generated bindings and an idiomatic layer;
  - the test-framework integrations people use (Jest and Vitest; JUnit 5, plus JUnit 4 and Spock if the
    community wants them);
  - the thinness audit;
  - and, as the new part, a **compatibility facade** for today's pact-js and pact-jvm DSLs, so most
    existing consumer tests need only mechanical changes (SDK spec §6's classification).
- **The protocol gaps the SDKs hit** (findings 4, 5, 6):
  - the consumer's test verdict reaches the engine, so the engine, not each SDK, withholds a dishonest
    contract;
  - results per interaction arrive before the suite ends;
  - the engine writes the contract's canonical bytes.
- **The contract format decided.** Janus writes its own format. Whether that is the next Pact
  specification version is §7's first question, and it must be answered before this stage ships a
  major version.

**Done when** both SDKs pass the conformance suite against a pinned engine, the example suites of
pact-js and pact-jvm run on the compatibility facade, and early adopters publish contracts to a broker
that the stage 1 verifier checks.

### Stage 3: Shapes and variants in the DSLs — **L**

The RFC's answer to optional fields, delivered to users.

- **The native DSL:** `optional`, `anyOf`, `oneOf`, `eachLike`, `forbidden`, `whenVariant` and
  `variantCases`, designed with the community, specified first, then implemented in both SDKs.
- **Decisions to make first:**
  - whether an array admits `[]` by default (finding 2);
  - how to exclude combinations a provider cannot produce (finding 35);
  - how correlated fields should be written, `oneOf` or independent members.
- **`upgrade`** for existing pacts, with its report of what the conversion narrows. It already exists;
  it needs the media-type operator (finding 1) so that `Content-Type` headers convert without
  narrowing.
- **Variant failure reporting** in each test framework: one test per variant where the framework allows
  it (the variant ergonomics report's recommendations).

**Done when** the RFC's consumer example runs natively in both SDKs, the demo runs on them, and the DSL
is frozen as the major version's surface.

### Stage 4: Provider shapes and the subsumption check — **L**

The part of the RFC no Pact user has today: catching what the provider may send that no consumer tested.

- **Recording provider shapes** from provider test suites, in each provider language's test framework.
  Janus records only through its own verifier (task 7.2).
- **Deriving shapes from OpenAPI and protobuf**, emitting selectors so they match consumer interactions
  by operation (ADR 0025). Spike 7.3's importer is the starting point, and its lessons are the
  specification.
- **The broker PR series** from the broker integration notes (task 7.5): a resource for provider shapes,
  decisions computed on read, and rendering from the report's own words. PactFlow's side is PactFlow's to
  decide.
- **Policy:**
  - a `provenance` selector for exemptions (finding 9B);
  - whether `block` ever becomes a default, which should be judged on pilot data, not decided now
    (ADR 0016's tripwire).

**Done when** the demo's loop runs against a real broker, with a provider shape published from a real
provider's test suite.

### Stage 5: Plugins as the core design — **L**

The component model, opened to the ecosystem.

- **WASM components, generally available:** the interfaces, the loader, OCI distribution and signing.
  First decide findings 15 (recording which components took part), 16 (registry credentials) and 17
  (artifacts published with `wkg`).
- **A bridge for today's pact-plugins**, at least the protobuf/gRPC one, so no existing user loses a
  plugin. A shim process speaking both wires is the smaller route (component-interfaces spec §9).
- **Content components that are not JSON:** member order (finding 11), `encode` seeing the shape (12),
  and decode errors reaching the user (13, 14).
- **Plan fragments versus operator-level substitution**, decided by gate F (§4) before any third party
  depends on either.
- **Out-of-process components**, if the community wants them beyond the bridge: the containment rules
  of findings 18–20.

**Done when** a protobuf plugin runs through either the bridge or a native component, and a second
content component exists written by someone outside the core team.

### Stage 6: Breadth — **XL**, ongoing

- **More SDKs:** Go, Python, .NET, Rust, and whatever else the community maintains. Each is generated
  bindings, an idiomatic layer written from the SDK specification, and the conformance suite. The
  regeneration trial (task 6.5) showed agents can carry a specification change into several SDKs under
  the suite's judgement. That is what makes this stage affordable for a small team, but it does not
  replace the suite.
- **Message interactions** end to end, with the `produce-message` and `consume-message` hooks, and
  broker adapters as components.
- **gRPC and other transports** as components.
- **The RFC's fourth migration step:** today's FFI and pact-jvm cores go into maintenance mode as each
  language's MkII SDK passes conformance. The timeline is the community's (§7).

**Done when** it never is. Each SDK has its own exit: conformance passing, and the old SDK's example
suite running on its compatibility facade.

## 4. Reassessment gates

Some decisions were left open on purpose. Each has a trigger, and something to do when it fires.
Several can change the stages above, which is why they are written down here rather than left in the
ADRs.

| Gate | Trigger | What to do | What it can change |
|---|---|---|---|
| **W: the WASM embedding** (ADR 0023) | `wasm32-wasip3` becomes a supported Rust target (it is being promoted to tier 2), and wasmtime plus one SDK-relevant host (jco for Node, or a JVM runtime) run p3 components | Re-run task 9.1's WASM scenarios, including a served mock and a driven verification. The exchange loop runs as an async task over `wasi:sockets`, with no thread; whether p3 brings threads does not matter if the loop needs none. Check `wasi:http`'s p3 server story too | If it works, WASM becomes a candidate primary embedding again, and stage 0's per-OS binary distribution becomes the fallback. Stage 2's transport rewrite should be async and thread-free, so that passing this gate costs a build target, not a redesign |
| **F: plan fragments** (ADR 0022) | A second content component needs a fragment, or finding 21 (a fragment compiled without the variant, which widened a pinned variant) bites a real user | Adopt operator-level substitution: the engine compiles the slot, and a component says only what one value operator means | Stage 5's component interface |
| **E: frame encoding** (ADR 0006) | The benchmark trend shows base64 or frame size on a hot path | Negotiate CBOR, which the protocol already allows for | The protocol's second encoding |
| **C: contract packaging** (ADR 0011) | A realistic contract passes 5 MB, or base64 bodies pass 25% of its bytes | The exploded projection, or a container form | The contract file |
| **D: DSL defaults** (findings 2, 35) | Stage 3's DSL design | Decide the empty-array default and exclusions before the DSL freezes | Stage 3 |
| **P: the subsumption policy** (ADR 0016) | Pilot teams routinely set `block`, or exemptions without an expiry dominate real policies | Revisit the defaults on that evidence | Stage 4 |
| **B: the broker's default specification value** (ADR 0011 decision 6) | The broker PR series lands and deployed brokers catch up | Stop publishing Janus contracts as `specification: "pact"` | Stage 4 |

## 5. Known debt, by stage

Everything the prototype left open, assigned to the stage that needs it. The detail is in the
[findings](phase-9-findings.md) and the [kernel-boundary review](kernel-boundary-review.md).

| Stage | Item | Source |
|---|---|---|
| 0 | Windows and macOS CI for the subprocess | spike 1.3's open risk; ADR 0023 |
| 1 | Plan grammar v1: every action namespaced, the core families reserved (`flow`, `value`, `expect`, `check`, `match`, `legacy`), v0 fragments read by the one-to-one mapping. A rename across the interpreter, both compilers and every corpus snapshot, with no verdict changing. It must land before `explain` output reaches users | ADR 0026 |
| 1 | Close the 158 skipped pact-specification cases (XML, form bodies, four uncovered rules) | task 3.5 |
| 1 | Remove the `request`/`response` part names and the HTTP 500 mismatch reply from the kernel | kernel-boundary review 8, 9 |
| 1 | Per-pair verification counts in the run summary | finding 10 |
| 1 | Matching captured values as a protocol operation, and `explain` loading components | findings 31, 23 |
| 2 | The consumer's test verdict and per-interaction results reaching the engine | findings 4, 5 |
| 2 | The engine writes contract bytes | finding 6 |
| 2 | Arming a variant recompiles its plan: cache per (handle, variant), and decide what a fresh-per-exchange generator means | finding 28 |
| 2 | An async, thread-free exchange loop and HTTP transport | ADR 0023 decision 5; gate W |
| 3 | `http:media-type`, and list-valued headers | finding 1 |
| 3 | The empty-array default; exclusions in the DSL | findings 2, 35 |
| 4 | A `provenance` selector on exemptions | finding 9B |
| 4 | Provider-shape recording in provider test frameworks; importers that emit selectors | tasks 7.2, 7.3; ADR 0025 |
| 5 | Member order; `encode` sees the shape; decode errors and degradations reach the user | findings 11–14 |
| 5 | Recording the components that took part; registry credentials; `wkg` artifacts | findings 15–17 |
| 5 | Out-of-process containment and command form | findings 18–20 |
| 5 | Fragments: the variant, decoding in the plan, action configuration | findings 21, 22, 24; gate F |

## 6. Risks

| Risk | Where it bites | Mitigation |
|---|---|---|
| **Volunteer capacity.** This is a multi-year build for a volunteer-driven project | Every stage | Providers-first ordering puts value in users' hands at stage 1 and stops depending on later stages. The specifications and conformance suite let contributors work in parallel without drifting apart |
| **The Osborne effect.** Announcing MkII stalls today's Pact before MkII is ready | Stages 1–3 | Stage 1 improves life for today's users with no migration at all. The compatibility facade keeps the consumer migration mechanical |
| **Native binaries.** Per-OS binaries were the pain of pact-ruby-standalone | Stages 0, 2 | The per-run, EOF-exit lifecycle (spike 1.3); distribution through each ecosystem's normal package channel; gate W as the way out |
| **Specification governance.** Specs nobody can change become a bottleneck, and specs anyone can change stop meaning anything | From stage 0 | The rule Janus followed: every change lands in the spec, the corpus or the conformance suite in the same change, and ADRs record the contested ones. Who approves them is a community question |
| **Maintainer identity.** pact-jvm ceasing to be an independent implementation | Stage 2 | The JVM SDK is a real product with its own maintainers and idiomatic layer; the JVM report (task 6.3) shows what they would own |
| **Findings the prototype could not produce.** No real team used Janus, so several tripwires (variant budgets, exclusions, subsumption noise at scale) never had data to fire on | Stages 3–4 | Pilot teams at each stage exit, and the tripwires in §4 watched deliberately |

## 7. Questions for the community

The charter says these belong to the community, and this plan does not decide them. What it can do is
say why each one matters and what the prototype found that bears on it.

1. **Naming and versioning.** Janus writes its own contract format and does not claim to be "pact
   v5" (ADR 0011). Some of the options:
   - make that format the Pact specification's next version;
   - keep it a separately versioned format under the Pact name;
   - brand the whole redesign (the RFC's "MkII", "Pact 6").

   It needs deciding before stage 2 ships a major version, because the format's identifier is written
   into every contract.
2. **Where it lives, and who owns what.** The engine, the specifications, and conformance sign-off each
   need an owner. The prototype suggests sign-off can be mechanical (a build passes the suite, ADR 0017),
   which leaves people deciding what goes into the specification rather than whether an SDK complies.
   The engine could live beside `pact_matching` in pact-reference, which it forked, or in a repository
   of its own.
3. **Funding.** Stages 1 and 2 are large, and providers-first means stage 1 pays back before stage 2
   finishes. Whether that is foundation sponsorship, vendor contribution or grants is not this plan's to
   say.
4. **When today's implementations go into maintenance mode.** The RFC ties it to each SDK passing
   conformance on the new engine. This plan's stage exits make that measurable per language. The
   timeline around them is the community's.
5. **How the broker's API evolves.** The broker notes (task 7.5) set out a PR series for provider shapes
   and decisions. How far the open-source broker and PactFlow each go is theirs to decide.

## 8. Closing the prototype

The charter defines "prototype complete" as follows. Each condition is now met.

| Condition | State |
|---|---|
| Phases 0–9 done or explicitly descoped by ADR | Done: this task closes Phase 9 |
| Every success criterion has a demo or a written finding | All seven met ([RFC feedback](rfc-feedback.md) §6) |
| Every unresolved-questions row has an answer or an honest "still open" | Yes ([RFC feedback](rfc-feedback.md) §2; plan §13) |
| The report and the staged plan are published for community review | The [community report](prototype-report.md), this plan, and the revised RFC on [PR #146](https://github.com/pact-foundation/roadmap/pull/146) |

What happens to this repository now is itself a question for the community. It can stay as the
reference prototype, which the real build reads and cites, or seed the real build's repository directly.
§2 is written so that either works.
