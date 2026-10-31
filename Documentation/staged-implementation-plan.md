# Staged implementation plan for Pact MkII

Plan task **9.4**, the prototype's last. **Date:** 2026-09-24; **revised 2026-10-11** (see
[Revision](#revision)). **Inputs:** [RFC feedback](prototype/rfc-feedback.md) (9.2), the
[community report](prototype/prototype-report.md) and [demo](../demo/README.md) (9.3), the
[performance report](prototype/performance-report.md) (9.1), the [Phase 9 findings](prototype/phase-9-findings.md), the
[kernel-boundary review](prototype/kernel-boundary-review.md), the [decision records](decisions/README.md) and
their tripwires, and the [reuse inventory](prototype/reuse-inventory.md) (task 0.2).

This is a plan for building the real thing, written by the people who built the prototype. It says:
- what should carry over from Janus, and what should not;
- the order to build in, and how to tell each stage is done;
- which decisions are deliberately left open, and what will reopen them;
- which questions belong to the community rather than to this plan.

It is a proposal for the RFC process, not a commitment on anyone's behalf. Since the revision it is also
the **live plan for this repository**: work on the real build is tracked against the item ids below
(`S0.1`, `S1.4`, …), the way the prototype's work was tracked against the project plan's task numbers.
The two numbering schemes never collide: the project plan's are bare (`1.6`, `G1`), this plan's carry an
`S`.

## Contents

1. [Principles](#1-principles)
2. [What carries over from Janus](#2-what-carries-over-from-janus)
3. [The stages](#3-the-stages)
4. [Reassessment gates](#4-reassessment-gates)
5. [Known debt, by stage](#5-known-debt-by-stage)
6. [Risks](#6-risks)
7. [Questions for the community](#7-questions-for-the-community)
8. [Closing the prototype](#8-closing-the-prototype)
9. [Revision](#revision)

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
6. **Every item is addressable and has an exit.** An item in §3 has an id, a size, and a test that says
   when it is done: a corpus case, a conformance case, a CI job, or a measurement. "Exactly as today"
   is a claim a harness checks (S1.1), not a feeling.

## 2. What carries over from Janus

Janus was built to be thrown away, and most of its *code* can be. What carries over is mostly the
*specification*: the documents, schemas and test corpora that say what the engine must do, whoever
writes it. Much of the kernel is worth keeping too, because it was held to those specifications
throughout. "Graduates" means it moves into the real build and is hardened there. "Seed" means it is the
first draft of something that will be rewritten against the final specification.

| Janus part | Verdict | Why, and what it needs |
|---|---|---|
| **Specifications and schemas** (`Documentation/specs/`: engine protocol, shape language, variant semantics, plan grammar, contract file, component interfaces, lifecycle hooks, subsumption check, SDK specification) | **Graduates**, as the v1 drafts of the executable specification | Every one was implemented against, and every change since Phase 2 was made in the spec first. They need community review, and the open items in §5 folded in. Where they live is a community question (§7). The SDK specification covers the consumer side only; the provider side is S1.14 |
| **Golden corpora, conformance suite, and their checkers** (`corpora/`, `conformance/`, `tools/schema-compat`, `tools/corpus`, `tools/conformance`, `tools/thinness`) | **Graduates** | These are what "conformant" and "compatible" mean. The pact-specification test cases the legacy harness runs should join them; since S1.22 it runs all 741 and skips none. The conformance suite does not yet cover test-framework integrations, the compatibility facade or per-language refusals (conformance report §7.2; S2.6) |
| **The protocol-level test client** (`cli/tests/janus-engine-node`) | **Graduates**, as the engine's own protocol suite | It drives `janus-engine` frame by frame with no SDK in the way, which is the test the protocol's compatibility policy (engine-protocol spec §11) needs and the SDK suite cannot give |
| **Kernel** (`engine/kernel`: plan compiler and interpreter, legacy v1–v4 compiler, shapes, variants, subsumption, contract model, upgrade, protocol dispatcher) | **Graduates**, with hardening | About 17,000 lines, all of it under the specs and corpora. The interpreter and plan grammar are the pact-reference v2 engine, forked with provenance (ADR 0004). Before it is the real kernel: rename its actions to plan grammar v1 (ADR 0026; S0.1); remove the two HTTP-shaped assumptions (kernel-boundary review 8, 9; S1.7); move the permanently HTTP-shaped legacy compiler behind a marked boundary (kernel-boundary review 3, 6; S1.11); make the exchange loop single-threaded and async-ready (§4, gate W; S2.1); cache per-variant plans (finding 28; S2.3) |
| **Built-in JSON content, OAuth2 hook, hooks host** | **Graduate** | Small, specified, tested. The hooks host needs credential redaction reviewed for production (S1.9), and the kernel's content-detector slot needs to become a registry so `match:content-type` runs in live exchanges and verification (S1.10) |
| **Built-in HTTP transport** (`engine/component-http`) | **Rewrite, in two halves** | It is threaded (`tiny_http`, one thread per server), was never built for WASM, and has no media-type or list-header semantics (finding 1). The **client** half (verification: TLS, proxies, timeouts) is stage 1 (S1.5); the **server** half (mocks: async, thread-free, which serves gate W) is stage 2 (S2.1); the `http:media-type` operator is stage 3 (S3.3) |
| **WASM component host and OCI distribution** (`engine/component-host`, `janus component push/pull`) | **Graduates** | It already does digest pinning, content-addressed caching and a tamper-testing registry. Needs findings 15–17 decided, and signing (S5.1) |
| **Out-of-process component loader** (spike 8.3) | **Rewrite, if kept** | Spike code. It proved the boundary exists, and findings 18–20 say what a real one must do differently (S5.5) |
| **CLI** (`janus explain`, `upgrade`, `check`, `component`) | **Graduates** | `verify` graduates too, but it has no broker integration, no TLS options, no generator support and no flag parity with today's verifier CLI (stage 1a) |
| **`janus-engine` subprocess binary** | **Graduates** | The primary embedding (ADR 0023). Needs a release pipeline for one statically linked binary per target triple, and real Windows and macOS CI (S0.2, S0.4) |
| **WASM engine artifact** (`benchmarks/janus/engine-wasm`) | **Graduates as the offline artifact** | Moves into `engine/` with its own CI build (S0.4), for explain, upgrade and check in hosts that want no native binary (ADR 0023 decision 2); the broker's recompute-on-read embedding is its first real host (S4.5) |
| **TypeScript and JVM SDKs** | **Seeds** | They prove the model: generated bindings, a few hundred lines of idiomatic layer, and conformance. But their DSL is Janus's, not the one the community will design, they have no compatibility facade for today's pact-js and pact-jvm DSLs, and they have no provider side at all. Keep the bindings pipeline (`tools/bindings`) and the drivers; rewrite the idiomatic layers (S1.15, S2.4, S2.5) |
| **Sample provider, demo, third-party CSV component** | **Graduate as examples and fixtures** | The demo is the clearest end-to-end regression test there is, and CI already runs it. The sample provider's v3 pact is the first entry in S1.1's fixture corpus |
| **Benchmark harnesses** (`benchmarks/`) | **Graduate** | As the CI trend line plan §14 promised and 9.1 found had never run (S0.3). Gate E cannot fire without it |
| **Spikes** (`spikes/`) | **Findings only** | Their code is not a starting point, and their findings are already in the ADRs |

The reuse inventory (task 0.2) noted what Janus took from pact-reference. The real build happens in
this repository ([ADR 0027](decisions/0027-build-in-the-prototype-repository-and-graduate-sdks-into-their-existing-repositories.md)):
the engine, specs, CLI, corpora and conformance suite stay together permanently as the executable
specification, and the SDKs are incubated here and graduate into pact-js and pact-jvm at stage 2's
exit. Whether the permanent part later moves beside `pact_matching` in pact-reference is §7's second
question, and a subtree move if it does.

## 3. The stages

Sizes are relative to each other: **S**, **M**, **L**, **XL**, where an L stage is roughly twice an M.
Each one weighs two things: what the stage needs that Janus does not have, and the hardening its Janus
parts need to become production code. They are not calendar estimates. Janus was built in seven weeks
of heavily AI-assisted work, which says nothing about a volunteer team's pace. They do say which stages
are big.

```
 0a Foundations, now ──► 1a CLI verifier ──► 1b Provider integrations ──► 2 Consumer engine + JS/JVM
                                                      │                            │
                                 0b Foundations, community ◄─┘ (once people can run their pipelines)
                                                                                   ▼
          6 Breadth ◄── 5 Plugins ◄──┬── 4 Provider shapes ◄── 3 Shapes and variants
                                     └── A AI-assisted layer (optional)
```

Stage 0a can start today. Stage 0b waits on §7, and §7's answers are not expected until people can run
their existing CI pipelines on the real thing, which is stage 1's exit: a prototype invites playing, not
an opinion. So nothing before stage 2 blocks on 0b, the codename stays until then, and S0.5 makes the
eventual rename mechanical rather than forbidding it. Stage 1a ships value on its own; 1b makes it a
product for the ecosystems that verify inside a test. Stages 4 and 5 can overlap once stage 3's contract format is
frozen. Stage 6 starts with SDKs as soon as stage 2 has proven the SDK model on two languages.

Every item below is `S<stage>.<n>` with a size and an exit. An item's exit is the thing CI, a corpus or
a measurement can check; where an item is a decision, its exit is an ADR. An item that has met its exit
says so in a *Done* line under it, with the date and anything the exit did not foresee.

### Stage 0a: Foundations that can start now — **S**

Engineering with no dependency on a community answer. All of it happens in this repository (S0.6).

- **S0.1 Plan grammar v1** (ADR 0026) — **S**. Every action namespaced, the six core families reserved,
  the legacy-only actions moved to `legacy`, and v0 fragments read through the one-to-one mapping.
  A rename across the interpreter, both compilers, every corpus snapshot and every `explain` example
  in the specs, with no verdict changing. First, because nothing is cheaper before anything else forks
  the corpora, and because `explain` output must not reach users under v0.
  *Exit:* every corpus case passes with renamed snapshots only; `component/hello` lists v1 and v0 in
  `plan-grammar-versions`; `third-party/janus-csv` loads unmodified through the v0 mapping; a component
  declaring a reserved namespace is refused at load, by name.
  *Done 2026-10-11.* All 22 corpus cases pass. Undoing the rename textually on the 28 corpus files that
  changed gives back the previous commit's bytes, so nothing but names moved. Two of the 28 are
  `case.json` files, not snapshots: an expected mismatch records the action that failed, and two of
  those are legacy-only actions that were renamed. Their status, path and message are unchanged. The
  specification is [plan grammar v1](specs/plan-grammar/spec.md), with its schemas in a new `v1`
  directory beside `v0`'s, and §4.7 of it is the v0 mapping. A fragment is held to the grammar it
  declared, and `explain` shows every fragment in v1's names. v0 also reserved a `convert` family that
  never had an action; ADR 0026 reserves six namespaces and that is not one, so v1 does not.
  One thing the rename made visible and did not fix, because fixing it changes behaviour: a shape whose
  operator is spelled in a reserved namespace (`"shape": "legacy:only-entries"`) compiles straight to
  that core action, which plan grammar §4.5 and §5.3 say the shape compiler must never emit. v0 had the
  same hole under the old names. It is S0.13.
- **S0.2 CI on Linux, macOS and Windows for the whole workspace** — **S**. The prototype ran real
  Windows only for spike 8.3. ADR 0023 makes the subprocess every SDK's primary embedding, so the
  kernel, CLI, `janus-engine` and both SDK suites run on all three.
  *Exit:* the CI matrix is green on three operating systems, including the SDK conformance runs.
  *Done 2026-10-11.* The `rust` and `sdks` jobs are a matrix over Linux, macOS and Windows; the checks
  that only read the repository run once, in a job of their own. macOS passed at the first run. Windows
  took four, and what it found is the reason the item exists:
  - a Unix-only `tiny_http` variant matched on in the HTTP component and the sample provider, so
    neither compiled;
  - three places that read meaning out of a path's separator: the specification test-case harness
    took 61 request cases for responses, the conformance lint found no case where its id said it
    belonged, and two hooks-loader tests expected a `/` join;
  - **the two SDKs disagreed**, in one conformance case of 47. A component reference that begins at
    the root and names no drive is absolute to Node and relative to Java, so the JVM SDK rewrote what
    the TypeScript SDK passed as written. The behavioural specification now states the rule on the
    text rather than on a path library's answer, and the JVM SDK follows it;
  - test fixtures that named `/bin/sh` or put a Windows path in a double-quoted YAML string, and a
    checkout that needed `.gitattributes` to stay LF.
  Spike 1.3's open risk has a test of its own now, on all three: `janus-engine` exits when its stdin
  closes, with a mock transport still serving and no `engine/shutdown` sent. The protocol-level Node
  client, which was never in CI, runs on all three as well.
  Three things run on fewer than three systems, each for a stated reason in the workflow: the real OCI
  registry (containers are Linux-only on the runners; elsewhere the tests use the in-process one), the
  bindings regeneration check (what it writes does not depend on where it runs), and the demo (Linux
  and macOS: it is a bash walkthrough built on symbolic links and job control).
- **S0.3 Benchmark trend in CI** — **S**. The harness plan §14 promised and 9.1 found had never run.
  Trend, not gate. Gate E (frame encoding) and gate C (contract packaging) can only fire on this data.
  *Exit:* every merge to `main` records the 1.7 scenarios against Janus and the chart is linkable.
  *Done 2026-10-11.* The chart is <https://rholshausen.github.io/pact-janus/bench/>, and its first
  point is the merge that added it (`497fc53`). The `Benchmarks` workflow runs the Janus harness in all
  three embeddings on every push to `main` and records 51 numbers: each scenario's headline, the
  arming share of a mock exchange, the per-interaction and per-variant costs, throughput, and the
  WASM component's compile time and size. They are kept on the `gh-pages` branch by
  `benchmark-action/github-action-benchmark`, pinned by commit, in the only job in this repository's
  CI that can write to it. Nothing fails for being slower. A pull request that touches the harness
  measures and compares, and records nothing.
  The numbers are a shared runner's: the mock-server scenarios run two to four times slower there
  than on the machine the hand-recorded results in `benchmarks/results/` come from, and they move
  between runs. So the chart answers "did
  this get worse over the last twenty merges", which is what gates E and C ask, and not "how fast is
  Janus", which `benchmarks/results/` and the performance report answer. The harness had not been
  built or tested by CI before this; its tests run in the workflow now, and it had survived S0.1
  and S0.13 unbroken.
- **S0.4 Release pipeline** — **M**. One statically linked `janus-engine` per target triple, signed,
  with checksums the SDKs verify on download; the CLI the same way; and the WASM offline artifact moved
  from `benchmarks/janus/engine-wasm` into `engine/` with its own build. The signing scheme is chosen
  here and recorded as an ADR.
  *Exit:* a tagged release publishes signed binaries for at least five triples and the WASM artifact,
  and an SDK resolves one by version.
  *Done 2026-10-12.* The release is [v0.1.0](https://github.com/rholshausen/pact-janus/releases/tag/v0.1.0),
  built from `f91e51c`: `janus-engine` and `janus` for `x86_64-unknown-linux-musl`,
  `aarch64-unknown-linux-musl`, `x86_64-apple-darwin`, `aarch64-apple-darwin` and
  `x86_64-pc-windows-msvc`, `janus-engine.wasm`, and a `SHA256SUMS` over them. Each binary was checked
  for dynamic dependencies and run on a runner of its own operating system before it was staged. All
  twelve files carry a build provenance attestation naming `release.yml` at that commit, and
  `gh attestation verify` passes on each. The TypeScript SDK resolves it: `sdks/engine-release.json`
  pins 0.1.0 and the engine's digest for each triple, and with `JANUS_ENGINE_RELEASE=1` its suite
  fetched the Linux binary, checked it against the pin and passed all 89 tests on it.
  Two decisions came with it. How a release's version is decided is
  [ADR 0028](decisions/0028-change-files-decide-a-releases-version-and-merging-their-pull-request-releases.md):
  change files read by knope, a release made by merging the pull request that consumes them, and no
  credential that outlives a workflow run. The signing scheme is
  [ADR 0029](decisions/0029-attest-what-a-release-built-and-let-an-sdk-verify-the-digest-it-pins.md):
  keyless attestations, and a digest the SDK pins beside the version. The WASM artifact is the
  `engine/wasm` crate, and the world it ships exports the pipe alone: the benchmark harness's
  interface is behind a feature no release builds with.
  What the exit did not foresee:
  - knope refuses a change file that names no package, so a change nobody who pins the engine could
    notice is declared in the pull request's description (`No-Changeset: <why>`), not by an empty
    file. And before 1.0 knope moves the numbers one place down, so the first version had to be given
    to it;
  - a component's export names are not symbols a native linker accepts, so `engine/wasm` is a
    workspace member that is empty on any target but wasm32, with lints of its own;
  - the released Linux binaries are musl builds, and the trend S0.3 records measures a glibc build of
    the same commit. Nobody has measured the difference;
  - Windows on ARM is not built;
  - the first release broke the build after it. The benchmark harness is a workspace of its own that
    depends on this one's crates by path, so its lockfile names them at a version, and preparing the
    release refreshed only the root one. The harness's build on the release commit rewrote its
    lockfile in place, and the Benchmarks workflow could not switch branches to record its point,
    which is lost. `knope prepare-release` refreshes that lockfile now, and CI resolves every
    lockfile that names a workspace crate with `--locked`, on every pull request.
  Left for S0.10: the resolver is the TypeScript suite's, run by hand on one platform. The JVM SDK
  has none, no CI job resolves a release, and the pin was written by copying five digests, which is
  the tool S0.10 owes.
- **S0.5 Rename-ready under the codename** — **S**. "Janus" stays, everywhere, until §7.1 is
  answered, which is expected late (§3's preamble). What stage 0 does is make the rename a mechanical
  change rather than an archaeology: every surface the name reaches is defined in one place per
  surface, and a rehearsal proves the change is complete. The surfaces: crate and package names
  (`pact_janus_*`, `@pact-foundation/janus`, the Gradle group), the binaries (`janus`, `janus-engine`),
  the configuration file (`verifier.janus.yaml`), the environment-variable prefix (`JANUS_*`), the
  user cache directory, the WIT packages (`pact:janus-engine`, `pact:janus-component`), a release's
  asset names and the repository an SDK's pin names (`janus-engine-<triple>`, `janus-<triple>`,
  `janus-engine.wasm` and `sdks/engine-release.json`; S0.4, ADR 0028 decision 9), and the
  **persisted identifiers**, which are the ones a rename cannot simply
  replace because documents in brokers and registries already carry them: the contract format
  `janus-contract/1` and provider shape `janus-provider-shape/1` (contract-file spec), the OCI artifact
  and config media types `application/vnd.pact.janus.component.v1` (ADR 0021), and the broker's
  `specification` value (ADR 0011 decision 6, gate B). The rule for those is **dual spelling for one
  major version**: the renamed engine reads both and writes the new one, so a contract converted or
  recorded under the codename stays readable, and nothing published before the rename has to be
  republished. New persisted identifiers may be added; they follow the same rule when the rename comes.
  *Exit:* the inventory above is complete and kept in one document; a rehearsal script renames the
  codename to a placeholder across every surface and the whole test suite, both SDKs' conformance runs
  and the demo pass; CI runs the rehearsal on a schedule so drift is caught.
  *Done 2026-10-16.* The inventory is [`codename.md`](codename.md), and the decision is
  [ADR 0031](decisions/0031-rename-by-a-rehearsed-rewrite-and-read-persisted-identifiers-under-every-spelling.md).
  The rename is `.github/scripts/rename.sh <name>`. It rewrites `janus`, `Janus` and `JANUS` in every
  text file, the record included, and moves every path that contains it. It then fails unless the
  codename is left nowhere but the three exemptions: `CHANGELOG.md`, the earlier spellings, and the
  repository's own name, which is S0.7's. The persisted identifiers are `engine/kernel/src/persisted.rs`.
  Each is spelled once, as what it writes, and read under that spelling and every earlier one, which
  the script records in `persisted/earlier.rs` before it rewrites.
  The `Rename rehearsal` workflow runs weekly with a placeholder made from the run's number, and its
  first run passed by hand: 548 files rewritten and 182 paths moved in three seconds. Then, renamed,
  the workspace's clippy and its 680 tests passed, along with the third-party component, both SDK
  suites, 47 of 47 conformance cases in each language, the thinness audit, bindings matching the
  pipeline's own output, the protocol client and the demo. The TypeScript suite loaded a component
  built before the rename.
  With S0.5, stage 0a is done: every item but S0.6, which ADR 0027 decided, has met its exit, and
  v0.1.0 installs on three operating systems.
  What the exit did not foresee:
  - the plan named four persisted identifiers, and there are seven. A component built before a
    rename exports the old interface name, a provider's hook scripts call the library by the old
    global, and its verifier configuration names the old body format. All three are read back from
    something the engine did not write and cannot rewrite;
  - the rehearsal cannot catch a reader that compares a stored name against a literal, because the
    rename rewrites the literal with everything else and the reader passes. Review catches it, and
    CLAUDE.md now says where such a name belongs;
  - the placeholder cannot be written in the workflow, since the script refuses a name the
    repository already contains, and the workflow is in the repository. It is made from the run's
    number instead;
  - the new name has to be one lowercase word. A hyphenated name, or one of two words, needs a
    different spelling per surface and is a different rename. The inventory says so.
- **S0.6 Where the build happens** — decided, [ADR 0027](decisions/0027-build-in-the-prototype-repository-and-graduate-sdks-into-their-existing-repositories.md).
  **In this repository, on `main`, hardening the prototype in place**, with §2's table as the
  inventory of what is rewritten. The engine, specs, CLI, corpora, conformance suite and checkers stay
  together permanently as the executable specification. `sdks/` is an incubator: the SDKs graduate
  into pact-js and pact-jvm at stage 2's exit (S2.9), as those packages' next major version, and
  S0.10–S0.12 make that split cheap now. If §7.2 answers "pact-reference", the move is a subtree of
  the permanent directories, not a rewrite.
- **S0.10 SDKs consume the engine by release** — **S**. Each SDK's test setup resolves a pinned,
  released `janus-engine` from S0.4's pipeline, verified by checksum (X.1), with `cargo build` kept as
  the developer fallback (`JANUS_ENGINE` already overrides). An SDK contributor needs no Rust toolchain.
  *Exit:* both SDK suites pass in a CI job with no Rust toolchain installed.
  *Done 2026-10-16.* CI's `sdks` job runs both suites on Linux, macOS and Windows through
  `.github/scripts/without-rust.sh`, which leaves every directory holding `cargo`, `rustc` or
  `rustup` off `PATH` and checks that none of them runs before it starts a suite. Main's engine can be ahead of the
  pinned release, so the engine there is this tree's, built by an `engine` job and handed over as a
  file: a change to the engine and an SDK can still land in one pull request. The release itself is
  resolved where nothing can have drifted from it, right after it is published. `release.yml` writes
  the pin, and both suites resolve the release through it with `JANUS_ENGINE_RELEASE=1` and no Rust,
  on x86_64 and aarch64 Linux, Apple silicon and Windows. The pin is then pushed as the `engine-pin`
  branch to open a pull request from, as `prepare-release.yml` pushes `release`. A plain local run
  still builds from the tree, and `JANUS_ENGINE_RELEASE=1` is the opt-in.
  The tool S0.4 owed is `tools/engine-pin`. It downloads a release's `SHA256SUMS`, writes nothing
  unless `gh attestation verify` passes on it, and writes `sdks/engine-release.json` from the digests
  in it. Run on v0.1.0, it wrote the five digests that had been copied by hand. On a rehearsal,
  `release.yml` checks that it reproduces the checked-in pin. The JVM resolver is `EngineRelease`, in
  the JVM SDK's tests: the TypeScript one's cases and cache, run by Gradle's `resolveEngine`. Gradle
  now chooses the engine the way the Vitest setup does, and runs cargo only when it builds from the
  tree. Before, it ran cargo even when `JANUS_ENGINE` named an engine.
  What the exit did not foresee:
  - one TypeScript test, the CSV consumer, builds a Rust component (`third-party/janus-csv`).
    `JANUS_CSV_COMPONENT` now names a `janus_csv.wasm` built elsewhere, and CI's `engine` job builds
    one;
  - GitHub's runners ship a Rust toolchain, and it cannot be removed the same way on each of them:
    `rustup self uninstall` works on Linux and Windows, and the macOS runners' rustup is Homebrew's,
    which refuses. So "no Rust toolchain installed" became "none reachable", made the same way on
    every runner. The check runs the tools rather than looking them up, because bash remembers where
    it last found a command, and the first attempt was fooled by a `rustup` it had just uninstalled;
  - the protocol client's setup asks cargo for the engine within a test hook's two minutes. It never
    built cold before, because the TypeScript suite had always built the engine first in the same
    job. It now gets a build step of its own;
  - the conformance checker finds the corpus by a path compiled into it, so it cannot be handed to
    another job as a binary. The SDK jobs upload their reports, and a `conformant` job with Rust checks
    each operating system's pair. The thinness check reads only the repository and moved to `checks`.
    Both are S0.11's to ship as binaries;
  - no runner resolves the x86_64 macOS engine, because `macos-latest` is Apple silicon;
  - X.1's logged `JANUS_ENGINE` override belongs to the SDKs' own runtime resolution, which they do
    not have yet: today only their tests resolve an engine.
- **S0.11 The corpora, the conformance suite and the checkers ship with the engine** — **S**. Every
  engine release carries `corpora/` and `conformance/` as a versioned artifact, and the checkers an
  SDK's CI runs (`tools/conformance`, `tools/thinness`, `tools/bindings`) as binaries, so a graduated
  SDK runs the same gates from another repository against a pinned engine version.
  *Exit:* a checkout containing only one SDK directory runs its conformance, thinness and bindings
  checks from the release artifacts alone, in CI.
  *Done 2026-10-16.* Every release carries `janus-sdk-gates-<triple>` for the five triples and
  `janus-spec.tar.gz`, which makes 17 files
  ([ADR 0030](decisions/0030-a-release-carries-the-sdk-gates-and-the-executable-specification.md),
  which extends ADR 0028 decision 9). The binary's subcommands are the three tools, whose commands
  now live in their libraries. The archive is the executable specification at its paths here: the
  corpora, the conformance suite, the behavioural specification, `sdks/bindings.json` and the schema
  sets it names, made by `git archive`. Each tool now takes `--spec <dir>`, a checkout or an
  unpacked archive, in place of the path compiled into it. The SDKs' drivers read `JANUS_SPEC` the
  same way, and each SDK keeps its own `thinness.json`.
  `release.yml`'s `sdk-gates` job is the exit. It checks out `sdks/typescript` or `sdks/jvm` and
  nothing else, takes the engine, the gates and the specification from the release's own files, and
  runs all three gates. `publish` needs it, and every rehearsal runs it on the files a release would
  publish. Rehearsed by hand first, with Rust hidden: the conformance suite, 47 of 47 cases in each
  language; thinness; and bindings regenerated with no difference. CI's `Change file` workflow now
  counts the gates' sources and the specification's files as shipped.
  What the exit did not foresee:
  - regenerating bindings needs the SDK's own generator: Node with `npm ci`, or a JDK with Gradle.
    The gates binary drives the generator and does not replace it, so "no Rust" is not "no tools".
    Staging is written under `--spec`, which has to be writable;
  - a sparse checkout has no root `.gitignore`, so the job diffs only the generated tree;
  - three things in a lone SDK still point outside it, and are S2.9's. The generated files' header
    says to regenerate with `cargo run -p pact_janus_bindings`, and changing it changes every
    generated file. The engine pin is `sdks/engine-release.json`, where ADR 0029 puts it in the SDK's
    own source. And the TypeScript CSV consumer test and the default conformance-report path reach
    into the repository; `JANUS_CSV_COMPONENT` and `JANUS_CONFORMANCE_REPORT` cover them for now.
- **S0.12 Prune the prototype** — **S**. Spike code is deleted and each `FINDINGS.md` kept, in place,
  so ADR links hold; the phase reports and the project plan move under `Documentation/prototype/`,
  with links updated; the charter and the decision log stay where they are. The README describes the
  real build and points at the prototype's record.
  *Exit:* no code under `spikes/`; every link in `Documentation/` resolves; CI green.
  *Done 2026-10-11.* Both halves of the exit are CI steps now: `tools/doc-links` checks every relative
  link in every Markdown file in the repository, headings included, and a second step fails if
  `spikes/` holds anything but Markdown. The record is indexed at
  [`prototype/README.md`](prototype/README.md); `b8d5010` is the last commit with spike code in it.
  Two things went with the code. The `spike-8-3` CI job was the only one that ran on Windows, so
  until S0.2 lands nothing does. And spike 8.3's loader was the only thing that could load a
  `subprocess` component, so that source is specified and unloadable until S5.5.
- **S0.13 A core action is reached only by the engine's own compilers** — **S**. Found while landing
  S0.1, and in neither the findings list nor the earlier revisions. Two defects, one cause: nothing
  stood between a document somebody else wrote and the interpreter's dispatch table.
  A **shape** may name an operator in any namespace, and the shape compiler turns a namespaced operator
  into the action of the same name. Spelled in a namespace the plan grammar reserves
  (`"shape": "flow:if"`, `"shape": "legacy:only-entries"`), that is a core action, handed the wrong
  arguments: some accept any value, some give verdicts that mean nothing, and two take the process down.
  The reserved namespaces are no component's (S0.1), so such an operator can never be resolved, and the
  shape language now says so: it is `interaction-invalid`, naming the operator.
  The **interpreter** reads an action's children by position and never checked there were enough. A
  component's fragment is validated for the names it uses and not for their arguments, so a fragment
  could do the same. The plan grammar now gives each core action the least number of children it
  takes; a fragment with fewer is refused at load, and the interpreter answers a short node with an
  `error` result rather than a panic, whoever built the plan.
  The specifications change first, and the corpus format gains a way to say "this input is refused",
  which it could not.
  *Exit:* a corpus case in which a shape naming an operator in each of the six reserved namespaces is
  refused, as `interaction-invalid`, naming it; a test that gives every core action in the table
  fewer children than it takes and gets an `error` result on the node each time, with no panic; a
  fragment with a short core action refused at load as `fragment-invalid`; no existing corpus case's
  result or snapshot changes.
  *Done 2026-10-11.* The case is `corpora/shapes/reserved-namespace-operator`, the first with
  `status: invalid`: the runner and the v1 case schema learned that a refused input has problems and
  no plan. The same refusal is tested where an interaction is added and where a contract is verified;
  there it fails the interaction as `contract-invalid` and nothing is sent to the provider. The
  least-children table is plan grammar §4.2, held in the kernel beside each action's name, and one
  test drives all 64 short forms of the 48 actions. A second test gives every action children of
  eleven kinds it was never meant to take, in three counts each, and nothing panics. A
  `flow:for-each` over an empty `splat` would have, and is an error result too. The other 22 corpus
  cases are byte for byte what they were.
  One inconsistency seen and left alone: the shape and plan specifications call a problem's location
  `problems[].path`, and the engine has always written `pointer`. The corpus case uses what the engine
  writes.

**Done when** S0.1–S0.6 and S0.10–S0.13 have their exits and a release from this repository is
installable on three operating systems. Item ids are identifiers, not an order: S0.10–S0.12 were added
by ADR 0027 after 0b's ids were taken, and S0.13 by what S0.1 found.

### Stage 0b: Foundations the community decides — **S**

Not expected before stage 1's exit, for the reason in §3's preamble, and nothing before stage 2 blocks
on it. The plan's assumptions until then: the work happens here (S0.6) and under the codename (S0.5).

- **S0.7 A home and owners** for the engine, the specifications and conformance sign-off (§7.2).
- **S0.8 The specifications published as drafts** for review, versioned from the start, at that home.
  The plan grammar is published as v1 (S0.1 already landed it).
- **S0.9 The name** (§7.1), which S2.7 then writes into every surface S0.5 inventoried, by the rehearsed
  change.

**Done when** the specs are open for community comment at their home, under their owners.

### Stage 1: A drop-in provider verifier — **XL**, as 1a (**L**) and 1b (**M**)

This is the first thing users get, and it delivers the RFC's first migration step: providers switch
verifiers at no cost. "At no cost" is measured, not asserted: S1.1 is built first, and every later
item in 1a is done when S1.1 agrees.

#### Stage 1a: the CLI verifier, for HTTP and message pacts, with a broker

- **S1.1 A differential verification harness and a fixture corpus of real pacts** — **M**. A tool
  that runs today's verifier (`pact_verifier_cli`) and `janus verify` on the same pacts against the
  same provider, and diffs the verdicts interaction by interaction. The fixture corpus starts with the
  sample provider's v3 pact and the pact-foundation example repositories' pacts, and grows with every
  pilot provider's. It is the test for principle 1, and it is what "exactly as today" means from here on.
  *Exit:* the harness runs in CI over the corpus and reports zero disagreements, or names each one with
  a reason (a known gap in §5, or a deliberate difference recorded as an ADR).
  *Done 2026-10-18* (`30e7234`, `feae5e0`). `tools/verifier-diff` runs `pact_verifier_cli`, pinned at
  1.3.5 in `tools/verifier-diff/pact-verifier.json`, and `janus verify` against a freshly started
  provider each, and CI's `verifier-diff` job runs it. Every disagreement is in
  `corpora/verifier/disagreements.json` with the item or ADR that accounts for it, and the run fails on
  one it does not list and on a listed one that has gone. The corpus started with the order-service's
  v3 pact and the pact-compatibility-suite's HTTP provider scenarios, which stood in for the example
  repositories' pacts, each with deterministic mutations of its responses; every later item in 1a added
  fixtures, 101 by the end of the stage. What the first run found became S1.18, and moved plain-text
  bodies into S1.6. Not yet in it: a pilot provider's pacts, which are the stage's exit.
- **S1.2 Where the broker client lives** — decision, ADR. The kernel knows no HTTP, so the broker
  client is not kernel code; but if it is CLI code, every provider integration in 1b and every SDK in
  stage 6 re-implements it, which the thinness audit measures. This plan recommends **protocol
  operations** (`broker/*`: fetch by selectors, publish results, publish a contract or a provider
  shape) implemented by a native-only built-in component, so every embedding gets one client and the
  WASM offline artifact answers `component-unavailable`.
  Decided in [ADR 0032](decisions/0032-speak-to-the-broker-through-protocol-operations-backed-by-a-native-host-capability.md):
  the client is a native host capability, registered as the hook implementations are, and
  verification fetches and publishes inside the run (a `broker` source and a `publish` option on
  `verification/verify`), with `broker/publish-contracts` for consumers. Engine-protocol spec §8.3
  and §8.7, `broker.schema.json`, and the `broker-verification.md` transcript; until S1.3 every
  engine answers them with `component-unavailable` naming `broker`.
  *Exit:* the ADR, and the operations in the engine-protocol spec with schemas and examples.
  *Done 2026-10-18* (`dbf92de`). The SDK bindings were regenerated for the new types, and the
  example checker validates `broker-verification.md`. S1.3 built the client on 2026-10-19.
- **S1.3 Broker integration** — **L**. Fetching pacts by consumer version selectors; pending and
  work-in-progress pacts; publishing verification results keyed per consumer/provider pair (finding
  10, option A: `pairs` in the run summary); `can-i-deploy` unchanged; a `janus publish` verb (broker
  notes §11) for contracts, and later provider shapes (S4.3); compression on the publish path
  (format review §8.2). It builds what S1.2 specified: the kernel's broker-client trait and the
  native crate that implements it, registered by both native binaries; per-pact pending status, the
  publishing link and the broker's interaction ids carried through the run; the `broker` capability;
  and the CLI's options for all of it, named in S1.13's table.
  How it is tested was settled before the build (2026-10-19), because no one source covers it. The
  published OpenAPI documents type shapes but not semantics, and are thinnest where the risk is; the
  broker pacts today's clients publish record what those clients send, not what a broker returns.
  So four layers, each catching what the others cannot (the fourth since dropped):
  1. **A real broker, with today's verifier beside us** — the gate. CI runs the Ruby broker
     (`pactfoundation/pact-broker`, pinned by digest) as a service, as the OCI tests run `registry:2`,
     and seeds it through its own API: pacts on branches and tags, deployments and releases recorded,
     a failing result so a pact is pending, a new branch so another is WIP. S1.1's harness then runs
     `pact_verifier_cli --broker-url` and `janus verify` against the same seeded broker, through a
     recording proxy, and compares three things: the pacts each fetched, with their pending and WIP
     flags and notices; the requests each sent; and the results each published, with the matrix
     after. A disagreement goes in the ledger with what accounts for it, as S1.1's do. The same job
     runs the broker behind basic auth and a bearer token, and fails if either credential appears in
     a frame, event, summary or log (ADR 0032 decision 5). A weekly run against the latest image
     catches the broker moving. CI's service containers are Linux-only, so this layer is too.
  2. **Recorded responses** — layer 1 offline, on all three systems. The client crate's tests replay
     responses recorded from the pinned broker; layer 1's job records them again and fails on any
     diff, as the bindings are regenerated. The paths a broker cannot be asked to take — a 401, a
     50x retried, a body that is not HAL, a base URL with a context path — are stub cases written by
     hand, from the list `pact_verifier`'s own broker tests cover.
  3. **A consumer pact of our own, verified.** The client crate's tests write a pact (`pact_consumer`)
     from the client to the provider `Pact Broker`, using the Ruby broker's provider-state names where
     one fits. It is verified in layer 1's job against the same broker, by today's verifier, each
     state a seeding through the broker's API. A pact nobody verifies is a mock that agrees with
     itself: `pact_verifier`'s broker pacts name the provider `PactBroker`, and Pact-JVM's pending and
     WIP interactions (`PactBrokerClientPactSpec`) use states the Ruby broker does not define. Those
     pacts are where ours takes its request bodies from, because what today's clients send is what
     brokers accept in production; what they expect back is not evidence. Proposing ours upstream,
     with the states it needs, is a follow-up, not the exit.
  4. **A schema lint** against PactFlow's OpenAPI document, fetched and pinned by digest, never
     committed. **Dropped (2026-10-19)**, once layers 1 to 3 were built against the real broker. What
     it was for is covered there: every request the client sends is either compared with today's
     verifier's (layer 1) or sent exactly to the pinned broker, which validates it (layer 3), and a
     misspelled selector is the user's, since the client passes selectors through as written. It
     also had gaps of its own: PactFlow's document omits fetching a pact and publishing results, and
     its licence meant fetching it rather than committing it. Given up: nothing tests PactFlow's own
     behaviour (its plan-limit 429, say); PactFlow runs the same broker beneath (2.122.0 on
     2026-10-19), so the OSS broker stays the reference.
  The publishing link (`pb:publish-verification-results`), which no published document describes,
  and what a selector selects are layer 1's alone.
  Layer 2 is built (2026-10-19): one scenario (two consumers publish, the provider fetches with
  pending and WIP, publishes, fetches again, and a republish is refused) recorded from the pinned
  broker into `engine/broker-client/tests/recorded/`, with the broker's database ids renumbered so
  a recording is the same file however often it is made, and the hand-written cases from
  `pact_verifier`'s list. Writing them found a difference: today's verifier fetches every link from
  the broker it was given, keeping only the link's path and adding a missing context path, so a
  broker behind a proxy that writes an internal host still works; the client now does the same.
  Layer 3 is built (2026-10-19): `engine/broker-client/tests/pact.rs` writes the client's pact with
  the provider `Pact Broker` (eleven interactions: the index; for-verification with a pending pact
  and with a WIP one; fetching each; the provider, its branch and tag; a verification result;
  publishing contracts, and a republish refused), between the Ruby broker's own Condor and Pricing
  Service with its state names, and CI verifies it against the pinned broker with today's verifier.
  Requests are exact and responses match by type, on only what the client reads. Proposing it
  upstream is still the follow-up.
  Compression, found while building `janus publish` (2026-10-19): the pinned broker (2.121.2) reads a
  gzipped body as JSON and answers 400, and no broker says it reads one, so the client compresses
  only when a broker says it can (RFC 7694: `Accept-Encoding` on a response; engine-protocol spec
  §8.7), and today every publication goes uncompressed. A reverse proxy's 1 MB cap (format review
  §8.2) is the deployment's to raise until the broker inflates request bodies, which is a change to
  propose upstream, not this item's exit.
  The credentials run is built (2026-10-19): `cli/tests/cli.rs` puts the pinned broker behind a door
  that demands basic credentials or a bearer token (the OSS broker has no bearer tokens, so the door
  is a proxy for both), and runs `janus publish` and `janus verify --publish` through it at `trace`,
  and with a wrong credential, failing if either appears in anything printed. It found two leaks
  before anything shipped: the engine's trace log of each frame repeated the descriptor's `auth`,
  and the HTTP library's trace dump repeated the `Authorization` header. The weekly run is the
  `Latest Pact Broker` workflow: every check above against `pactfoundation/pact-broker:latest`,
  with the harness's `--any-version` and the recording made again and compared except for its
  version line.
  *Exit:* S1.1's harness runs the fixture corpus fetched from a real broker, pending and WIP semantics
  included, and the broker's matrix shows the results per pair; and the client's consumer pact is
  verified by that broker.
  *Done 2026-10-19.* The harness's broker cases (six fixtures, and the compatibility suite's broker
  steps) are published to the pinned broker and fetched from it by both verifiers, pending and WIP
  included, and agree; what each run published is read back per pair, from each pact's latest
  verification result, which is what the matrix shows, rather than from `/matrix` itself. The rest of
  the corpus still runs from files, which tests nothing S1.3 added. The client's pact is verified by
  the pinned broker in CI and by the latest one weekly. Not foreseen: the schema lint was dropped
  (layer 4); the client's links are fetched from the broker the user named, as today's verifier does,
  found by layer 2; compression cannot be used against any broker today (pact_broker#1062,
  pact_broker-rs#11); the credentials run found two trace-level leaks; PactFlow answers a plan's
  limit with a 429 that no retry passes, so a 429 is retried only with `Retry-After`; and
  `pact_models` moved to 1.3.15 for its merge fix. Left out, each for when something needs it: the
  per-pact `PUT` publish for brokers before 2.86.0, `--merge` and the git-reading options of
  `pact-broker publish`, the HTTP client options in the broker descriptor (S1.5), proposing the
  client's pact upstream, and `janus publish` for provider shapes (S4.3).
- **S1.4 v3/v4 generators in verification** — **M**. The legacy path has no generator support:
  `upgrade` drops them with a `generator-dropped` finding, and verification never applies them.
  Today's verifiers apply `ProviderState` generators with values the state-change call returned,
  `MockServerURL`, and the value generators (random, date and time, UUID, regex) to request bodies,
  headers, paths and query strings. The shape language's `generator` member names a component; the
  v1–v4 generator vocabulary becomes one, applied by the legacy compiler.
  *Exit:* the generator cases from pact-reference's verifier tests pass in S1.1's harness, with a
  state-change response feeding a `ProviderState` generator end to end.
  *Done 2026-10-20* ([ADR 0034](decisions/0034-apply-a-pacts-request-generators-through-a-generator-component-fed-by-state-values.md)).
  The vocabulary is the generator component `pact`, built into the kernel behind `generator/generate`,
  producing values with `pact_models` as today's verifier does. The legacy path generates the request's
  path, headers, query and JSON body after state setup and before `before-request`; response
  generators are not applied, as they are not today. A `state-setup` hook result carries `values`, and
  `pact-state-change` reads a JSON object reply as them, so a state endpoint written for today's
  verifiers feeds `${id}` unchanged. S1.1's corpus has four `generators/` fixtures, all agreeing with
  today's verifier: pact-reference's one generator case (issue #441, a state parameter into the path),
  a state-change reply into a path and a header, every value generator from the compatibility suite's
  fixtures in a body and a random one in a header and a query, and a `ProviderState` expression with
  nothing to read. Run against `janus` from before this, every one of them but the last disagreed.
  The harness's routes gained `patterns`, recognising a generated value by its form, and its state
  callback a `values` answer. Not foreseen: a generator that produces nothing keeps the recorded
  value, as today, and is now reported as a `generator-failed` warning; a `pact-state-change` reply
  that is not JSON is `ok` (today's reading) where it was a hook failure; and `verification/warning`,
  emitted since the prototype, had no row in the protocol spec's event table, and now has one. Left
  out: generators for contracts, which `upgrade` still drops (S1.19), and for XML and form bodies,
  which arrive with S1.21's and S1.22's content components and warn until then.
- **S1.5 The HTTP component's client half, rewritten** — **M**. Async and thread-free (the same
  executor seam S2.1 uses). TLS with custom CA bundles, client certificates and an insecure mode;
  HTTP and HTTPS proxies; request timeouts; HTTP/1.1 and HTTP/2; today's `--request-timeout`,
  `--custom-header` and base-path options. None of this exists in the prototype.
  *Exit:* a verification against a TLS provider with a client certificate behind a proxy, in CI; and
  S1.13's table has a `janus` column for every transport flag.
  *Done 2026-10-21* ([ADR 0035](decisions/0035-reach-providers-and-brokers-through-one-async-http-client-configured-by-one-document.md)).
  One client, `engine/http-client`, is what the HTTP transport's drive role and the broker client
  send through. It is today's verifier's stack (reqwest, rustls on ring, the platform verifier),
  already in the tree through `pact_models`. It runs on a current-thread runtime with names
  resolved inside the request, so it starts no thread; a Linux test counts the process's threads
  around a request. One document configures it, the HTTP client options (component-interfaces spec
  §5.6, `http-transport.schema.json`): TLS roots, a client certificate and an insecure mode;
  proxies (the environment's by default, one named, or none); request and connect timeouts; HTTP/1.1,
  HTTP/2 or negotiated; retries. It is the drive options' `client` and the broker descriptor's. The
  drive options add `base-url` (whose path is the base path) and `headers`. `janus verify` takes
  today's `--base-path`, `--header`, `--request-timeout` and `--disable-ssl-verification`, with
  their environment variables, and adds `--ca-cert`, `--client-cert`/`--client-key`, `--proxy`,
  `--no-proxy`, `--connect-timeout` and `--http-version`. The exit's verification is
  `cli/tests/cli.rs`: a TLS provider demanding a client certificate, through a proxy, at `trace`,
  with neither the key nor the header's token printed. It runs on all three systems. S1.13's table
  is `Documentation/verifier-migration.md`, with every transport option of both of today's
  verifiers. S1.1's corpus has six `connection/` fixtures. Five agree with today's verifier, and
  the sixth is a deliberate difference in the ledger. The whole corpus, broker cases included,
  changed no verdict. Not foreseen: the prototype's client asked providers for gzip and trusted its
  own bundled roots, both unlike today's verifier, and both now as it does. Today's `--header`
  replaces a pact's header only when the names are written in the same case, which Janus does not
  copy (`connection/header-case`, ADR 0035). A transport failure's variant error was not the
  protocol's `component-failed`, and had no message, so `janus verify` printed only the code; it
  now says why. The kernel's fixed 30 s per request became the transport's 5 s, today's default.
  Left out: the `http` hook, still on its own client (S1.9); flags for a broker's TLS, and today's
  `--hostname`/`--port`/`--transport` spellings (S1.13); the server half (S2.1). HTTP/2 is tested
  with prior knowledge in the clear and by what ALPN offers over TLS; no test provider answers in
  HTTP/2 over TLS.
- **S1.6 Plain-text and binary bodies** — **S**. The first of the body kinds a JSON-only engine
  lacks, and the two that need no document structure. S1.1's first run found both: a `text/plain`
  body is `component-unavailable`, and a binary one (`image/jpeg` in a v1 pact) is refused as
  `contract-invalid`. A plain-text content component decodes a `text/*` body the JSON component does
  not claim into a string, compared as today's verifier compares text: by equality, or by the rules
  on `$`. A body whose content type no component claims is compared as bytes, as today. This settles
  how a body that is not JSON reaches the legacy compiler, which S1.21 to S1.23 reuse.
  The body kinds were one item, S1.6, until 2026-10-21. It was split by what each kind needs:
  - text and binary need nothing new in the document model, so they come first (this item);
  - form and XML both need member order and the legacy compiler retyped (S1.20), then a component
    each (S1.21, S1.22);
  - multipart compares each part by its own content type, so it comes last (S1.23).

  Measuring before the split found that the 158 skipped pact-specification cases are 156 XML ones
  and two whose `janus:skip` blames unanchored regexes, which S1.18 anchored; both pass without it.
  *Exit:* S1.1's ledger entries for `compatibility-suite/v1/15-response-with-plain-text-body-positive-case`
  and `…/21-response-with-binary-body-positive-case` gone; corpus cases for a text body and a binary
  body, matching and not; and the two `plain text regex matching that does not match` fixtures
  without `janus:skip`.
  *Done 2026-10-21* ([ADR 0036](decisions/0036-read-text-bodies-through-a-text-component-and-unclaimed-bodies-as-bytes.md)).
  A built-in `text` content component, `engine/component-text`, decodes `text/*` as UTF-8 and encodes
  a string back. Every embedding registers it after JSON, and `engine/hello` lists it. Octets no
  component claims are now bytes rather than absent (component-interfaces spec §6.1). The legacy
  layer reads a v1–v4 body by the pact's content type, as today's verifier picks its matcher: JSON is
  a document, a binary type is bytes, any other text is a string. XML, form and multipart are still
  refused, until S1.21 to S1.23. A text or bytes body compiles to one value at `$`: the rules there,
  or equality. A reply's body is decoded as the pact's type. When the pact names no `Content-Type`
  header, a new legacy action, `legacy:body-type`, fails a reply whose type does not agree, which is
  today's check before a body is compared. The four ledger entries are gone. Six new `bodies/`
  fixtures agree with today's verifier, mutations included: a reply with no `Content-Type`, one
  relabelled, a text regex and an octet stream. The whole corpus is 326 verdicts: 312 agree and 14
  are known. Five corpus cases cover text and binary bodies, matching and not, and a regex on text.
  A binary case's values carry bytes through the corpus format's new `encoded` (plan-grammar spec
  §6.1). The two regex cases run without `janus:skip`, so the legacy harness is 585 of 585, with
  156 skipped, all XML. Not foreseen: the harness's mutations relabel each reply's content type, and
  today's verifier fails every one on the type, so agreement had to be checked rather than
  approximated. `explain` now shows that check in a JSON pact with no `Content-Type` header. A bytes
  mismatch of equal length said nothing, so it now names the first byte that differs. Left out:
  `janus upgrade` still asserts nothing about a body that is not JSON (S1.25); `match:content-type`
  on a binary body still has no detector in verification (S1.10); charsets other than UTF-8 (S6.5).
- **S1.7 Message pact verification** — **L**. Verifying v3 message pacts and v4 message and
  synchronous-message interactions, which pact-jvm users have in numbers. First the kernel's
  `request`/`response` part names and its HTTP 500 mismatch reply go (kernel-boundary review 8, 9):
  the transport binding says which part is inbound and which the answer, and a mismatch is a `dispose`
  with a `reject` disposition. Then a message transport, and the `produce-message` and
  `consume-message` hooks spike 1.5 and design 2.7 specified, hardened as the way a provider's message
  producer is invoked. Consumer-side message interactions are S6.2.
  *Exit:* corpus cases for message interactions; pact-jvm's and pact-js's message provider examples
  verify identically in S1.1.
  *Done 2026-10-23* ([ADR 0046](decisions/0046-ask-a-provider-for-a-message-through-produce-message-and-let-the-transport-name-its-parts.md)).
  A transport names its own parts: its contribution declares `exchange-parts`, HTTP's `request` and
  `response`, and the kernel spells none. A served request that does not match is disposed of as
  `reject` with its mismatches, which the HTTP transport answers with the 500 the kernel used to write.
  `declared_transport.rs`'s line transport now uses parts of its own, so both findings have a test. A
  v1–v4 message is one replay that drives no transport: its `produce-message` hooks produce a `message`
  part (`metadata`, `body`), compared as today's verifier compares a message, with a new legacy action,
  `legacy:message-type`, for its stricter content-type check. The `http` hook gained `format:
  pact-message`, today's POST and reply. Not foreseen: today's verifier routes a message by
  `--transports`, with a fallback to the provider URL that hook selectors cannot express. So `janus
  verify` takes `--transports`, and the hook's `transports` member applies today's rule. When the
  configuration names no producer, `janus verify` adds one, so a provider that verifies messages today
  needs no configuration; a message no hook produces fails as `component-unavailable` naming
  `hook/produce-message`. Not foreseen either: a v4
  message's `content` rules are `pact_models`' `body` category, so pact-jvm's `body` and pact-js's
  `content` are the same rules. S1.1's corpus has `messages/`: pact-js's example, three pact-jvm
  pacts (v3, v4 with metadata rules, a synchronous Kafka request), a pact mixing HTTP and a message,
  all mutated, and the compatibility suite's V3 and V4 message provider scenarios, imported as its
  harness builds them (19 more, the one `@wip` skipped). Every verdict agrees but `messages/error-status`:
  today's verifier reads an asynchronous message from a 500 and passes, Janus fails it, deliberately.
  The whole corpus is 1124 verdicts over 538 cases: 1110 agree and 14 are known. Four corpus cases
  cover messages. Left out: `janus upgrade` still drops
  a message (S6.2); provider-side `consume-message`, which today's verifier does not have; a transport
  for a real broker (S6.3).
- **S1.8 Plugin-backed v4 interactions are refused by name** — **S**. A v4 pact whose interaction
  requires a pact-plugins plugin (protobuf, gRPC, Avro, CSV) cannot verify "exactly as today" until the
  bridge (S5.2). Until then the verifier fails that interaction before anything runs, as
  `component-unavailable` naming the plugin, and the run summary counts it, so a mixed pact is not
  silently half-verified.
  *Exit:* a conformance and corpus case per plugin-bearing interaction kind.
  *Done 2026-10-23* ([ADR 0045](decisions/0045-refuse-an-interaction-that-needs-a-pact-plugin-by-name-and-verify-the-rest.md)).
  An interaction needs a plugin when its `pluginConfiguration` names one, which every CSV, JWT,
  protobuf, gRPC, JSON-RPC and Avro example does. Not foreseen: a plugin that contributes only a
  matcher (pact-plugins' `creditcard`) is named in the pact's metadata alone, so an interaction using
  a rule `pact_models` reads as a plugin's needs every plugin the pact names. Before this, a CSV
  interaction verified with its body compared as text, and a gRPC or Avro one was skipped without a
  word. Now each is `failed` with `component-unavailable` naming `pact-plugin/<name>`, before any
  state or request, and the rest of the pact verifies. Today's verifier refuses the whole run when it
  cannot load a plugin, and the migration note says so. `janus upgrade` drops such an interaction
  with an `interaction-dropped` finding naming the plugin, and `janus explain` refuses it. The exit
  changed in two ways. There is no conformance case, because the conformance suite has no provider
  side until S1.14. There is no S1.1 fixture either: today's verifier installs a missing plugin from
  the plugin index (it did, into an empty `PACT_PLUGIN_DIR`), so a fixture would download plugin
  binaries on every CI run. The refusal is tested offline instead, per kind (HTTP, gRPC, an async
  message, a matcher-only plugin), in `engine/kernel/tests/legacy_verification.rs`, `legacy_pact.rs`
  and `upgrade.rs`, and in two `cli/tests/cmd/` cases. The corpus format has no refusal of this kind:
  its one refusal is an invalid document.
- **S1.9 Hooks in place of request filters and state-change callbacks** — **M**. The prototype's
  hooks, hardened: the hooks host's credential redaction reviewed for production, script and `exec`
  hook timeouts and resource limits enforced, and a migration note for each verifier option they
  replace (S1.13).
  S1.1 found that the two treat a failing state-change call differently. Today's verifier retries a
  state call that gets a 5xx, 408 or 429, eight times, waiting 500 ms and doubling each time (over
  two minutes in all), and fails the interaction only then; any other status fails at once. Nothing
  sets the count: `pact_verifier_cli` and the FFI both build the state executor with its default
  (`HttpRequestProviderStateExecutor`, `retries: 8`), and `--retries` reaches broker requests only.
  An environment override, `PACT_PROVIDER_STATE_CHANGE_RETRIES`, is proposed upstream, so the FFI
  need not change. The `http` hook does not retry at all, so a state endpoint that fails once while
  it warms up passes today and fails under Janus (`state-callbacks/fails-once` in S1.1's ledger).
  The recommendation: an `http` hook takes a retry count with today's rule (the same statuses, the
  same backoff), and a `state-setup` hook defaults to today's eight, so a provider moving over keeps
  what it had. The default is the decision.
  S1.5 found two more. The `http` hook is still on its own client (`ureq`), so it cannot reach a
  state endpoint behind the provider's mutual TLS or private CA. It moves onto `engine/http-client`,
  and an `http` hook entry takes a `client` member, the HTTP client options (ADR 0035). And the
  engine's `trace` log of each frame repeats a hook entry's `config` and its `run.headers` and
  `run.env`, which §7.3 says appear nowhere. The frame redaction (`loggable` in the protocol
  dispatcher) must cover them as it covers a broker's `auth`.
  *Exit:* every state-change and request-filter option in S1.13's table maps to a hook, and the
  redaction rules (lifecycle-hooks spec §7.3) have a test per secret-bearing context member.
  *Done 2026-10-23* ([ADR 0044](decisions/0044-reach-a-hook-endpoint-as-a-provider-is-and-try-a-state-call-three-times.md)).
  The `http` hook sends through `engine/http-client` and takes a `client` member, so a state endpoint
  behind mutual TLS is reached as the provider is. The default is not today's eight: that count came
  with the broker's retries in May 2026 (`c98502f6`), not from a decision about state calls. A state
  call is tried three times, after a 5xx, 408 or 429, waiting 100 ms and then 1 s, which is the 2022
  design (#197). `timeout-ms` bounds each attempt. `state-callbacks/fails-once` now agrees, and its two
  ledger entries are gone: the harness reads 782 verdicts, 770 agreeing and 12 known.
  `format: pact-state-change-query` is today's `--state-change-as-query`. The migration table has a
  row for each state-change option of both CLIs, `--custom-middleware`, and the in-test request
  filters and state handlers, which S1.15 serves as loopback hooks. Redaction, reviewed surface by
  surface: renderings and reports already left out `config`, `env` and `headers`. Two leaks are fixed:
  the `trace` frame log repeated all three, and a failing `http` hook's message repeated its URL's user
  information. Tests cover each secret-bearing member across events, the hook report, the summary,
  frames and the binary's own `trace` output. An `exec` deadline now ends the process group (Unix,
  through `rustix`, with no `unsafe`) or tree (`taskkill /T` on Windows), and so does a command that
  exits while a child it started holds its output. A script's memory (64 MiB) and stack (256 KiB) are
  bounded, with a test each, which closes X.2. Not foreseen: rquickjs already capped the stack at
  256 KiB, unstated, and its memory limit is silently ignored under its custom-allocator features,
  which the kernel does not enable. Left out: registry credentials, X.3's other half, are now S1.27,
  and the built-in OAuth2 hook component still has its own client.
- **S1.10 `match:content-type` in live runs** — **S**. The exchange and verification paths execute
  with no content detector (third-party component report §4); the single `ContentDetector` slot
  becomes a lookup over the session's `ContentRegistry`.
  *Exit:* `corpora/shapes/content-type` runs through `janus verify`, not only `tools/corpus`.
  *Done 2026-10-25*, as [ADR 0049](decisions/0049-match-a-content-type-against-the-arrivals-own-octets-and-labels.md)
  decides it. The detector is the session's content registry, through one adapter the live paths and the
  corpus runner share, and the JSON component has no detector of its own. Not foreseen: by the time a
  plan resolves a body it has been decoded, so the detector is told the path, and a whole slot is read
  from the arrival's own parts. On a contract, a declared or labelled type is the type and detection
  only resolves its absence, as component-interfaces spec §6.5 requires. A v1–v4 `contentType` matcher
  sniffs the octets, as today. `cli/tests/cli.rs` runs the corpus case's shape through `janus verify`.
  Nothing built in detects a binary type yet, which no fixture needs.
- **S1.11 The legacy compiler behind a marked boundary** — **S**. Kernel-boundary review findings 3
  and 6: `plan::legacy` and `legacy_pact` move into their own crate (or a submodule whose `mod.rs`
  states the exception), so "the kernel knows nothing about HTTP or JSON" is checkable by reading the
  module tree.
  *Exit:* the kernel crate has no `pact_models::v4::http_parts` import outside that boundary, and a CI
  grep says so.
  *Done 2026-10-21.* The boundary is a module, `engine/kernel/src/legacy/`, whose `mod.rs` states the
  exception. A crate would have needed the kernel, and the kernel it: the compiler builds the
  kernel's plan nodes, and the verification path calls the compiler. It holds `pact`, the reader (was
  `legacy_pact.rs`); `compile`, the matching-rule compiler (was `plan/legacy.rs`); and `upgrade`, which
  was `upgrade.rs`, since converting a v1–v4 pact needs the same knowledge. The check is
  `engine/kernel/tests/boundary.rs`, part of `cargo test` on all three systems, and stricter than the
  exit asked. Outside `legacy/`, any `pact_models` use fails it unless its list allows it, and the list
  has two entries, neither HTTP nor a pact document: the generator vocabulary of the built-in `pact`
  generator component, and the `datetime` operator's format check. Not foreseen: S1.4 had put a
  `pact_models` request in the verification path, to apply its generators once states were set up. It
  is now an opaque `legacy::pact::RequestTemplate`. No behaviour changed, and nothing an engine pin sees.
- **S1.12 Two protocol gaps `explain` hit** — **S**. Matching captured values becomes a protocol
  operation (finding 31), and `explain` loads the components a run would (finding 23), so what it
  prints is the plan `verify` runs.
  *Exit:* the protocol-level test client exercises both; `janus explain --executed` on a CSV
  interaction prints the fragment.
  *Done 2026-10-25*, as [ADR 0050](decisions/0050-explain-compiles-what-a-run-compiles-and-runs-captured-values.md)
  decides it. `verification/explain` takes `components`, resolved and checked as a run's are, and
  `values`, which it runs with the run's content detection and contributed actions, returning
  `executed`. `janus explain --config` reads a configuration's components, and `--executed` sends
  its values, so the CLI no longer runs a plan through the kernel's API. Not foreseen: `explain`
  rebuilt a contract's specification without its `content-types`, so no fragment could have been
  found. And an interaction no loaded component can carry is now refused, as a run refuses it, where
  `explain` used to print a generic plan. The protocol client's `explain.test.ts` declares the CSV
  component and runs `csv:integer`; CI's `sdk-tooling` job now builds the component for it.
- **S1.13 Flag parity with today's verifier CLI** — **S**, but it gates the stage. A table with a row
  per `pact_verifier_cli` and `pact-provider-verifier` option: the `janus` option, the hook, or
  "dropped, because". The table is the migration note for providers:
  [`verifier-migration.md`](verifier-migration.md), whose provider-connection rows S1.5 filled. Behaviour with no flag gets a
  row too, where a provider relies on it: today's verifier retries a failed state call eight times
  and nothing on its command line changes that (S1.9). The `--filter-*` options are filtered runs as
  [ADR 0041](decisions/0041-refuse-a-filter-that-names-no-recorded-variant-and-report-a-narrowed-run-as-filtered.md)
  decides them (S1.17): a filter that matches nothing is refused, where today's verifiers pass it, and
  a narrowed pact is not published, where `pact_verifier_cli` publishes it. Their rows carry the ADR.
  *Exit:* every row filled; no "dropped" row without a reason a pilot provider accepted.
  *Built 2026-10-25*, as
  [ADR 0053](decisions/0053-take-todays-verifier-options-where-they-mean-what-they-did.md) decides;
  the exit's second half waits on pilot providers. Every row is filled, for `pact_verifier_cli` 1.3.5
  and the Ruby verifier. `janus verify` takes today's long spellings and variables where they mean
  what they did, with the Ruby verifier's as aliases. Today's state-change options write the hooks.
  The `--filter-*` options are `options.filter`, applied in the engine, and pacts by URL
  (`--url`, `--webhook-callback-url`) a `urls` source, fetched and published through the broker
  client. `--exit-on-first-error` is `stop-on-first-failure`, and `--consumer-version-tags`,
  `--retries`, `--ignore-no-pacts-error` and `--no-colour` map onto what exists. The table's
  [Dropped and not taken](verifier-migration.md#dropped-and-not-taken) lists what a pilot must
  accept: today's report formats, the log layouts, `--last-failed`, and the Ruby verifier's options
  that read git, wait, or load Ruby. S1.1's harness compares the filters (`corpora/verifier/filters/`)
  and the webhook flow (`broker/webhook-callback-url`) against today's verifier, so a fixture can pass
  the same options to both. Not foreseen: the harness matched today's report to interactions by
  description alone, so a filtered consumer's interactions took another's results; and today's
  verifier stops at a pending failure, which `janus verify` does not, since it fails nothing.
- **S1.17 A filtered run never reads as a pass** — **S**. Found while rebuilding the CLI before stage
  1a, and in neither the findings list nor the earlier revisions. `janus verify --variant <id>` with an
  id no source records replays nothing, and reports `VERIFIED: 0 verified, 0 failed, 0
  state-unavailable, 2 skipped` with exit 0: the run summary's status is `verified` whenever nothing
  failed (`summary()` in `engine/kernel/src/protocol/verification.rs`). Variant-semantics spec §5.1
  says a verifier "MUST report a filtered run as filtered — a partial run is not a pass". The run is
  reported as filtered (`filtered: true`, and a line in the text), but its status and exit code still
  say pass, so a mistyped id looks exactly like a narrowed run that passed. The verdict is also given
  twice with two answers: `subsumption/decide` already treats a filtered run as a warning
  (`verification-filtered`, engine-protocol spec §8.6), while `verify` treats it as verified. And S1.3
  makes this worse: a run's results get published to a broker, and a filtered run must never be
  published as a pass. Two decisions, both in the variant-semantics spec:
  a filter naming an id that no source records is refused before the run starts, naming the id. That
  is the author's typo, caught where they can fix it, as §6.5 does for bindings. And a filtered run
  that verified everything it replayed carries a status other than `verified`, which the summary, the
  CLI's exit code, `subsumption/decide` and S1.3's publishing all read the same way. S1.13 records
  what today's verifiers do when a `--filter-*` option matches nothing, and S1.1 shows whether the two
  agree, or names the difference in an ADR.
  *Exit:* a corpus or conformance case for an unknown id and one for a filtered run; the snapshot in
  `cli/tests/cli.rs` (`a_filtered_run_says_so_in_its_summary`) changes with them; and a filtered run's
  verification result is never published as a success.
  *Done 2026-10-23* ([ADR 0041](decisions/0041-refuse-a-filter-that-names-no-recorded-variant-and-report-a-narrowed-run-as-filtered.md)).
  A filter naming an id no document records is `variant-not-found`, naming the id and the ids the
  documents do record. For files `verify` refuses it, and `janus verify` exits 2. For a broker, the run
  ends right after `verification/fetched`. A run is filtered when its filter left out a recorded
  variant, as pact-jvm judges it, and then its status is `filtered`, as is each `pairs` entry it
  narrowed. A narrowed pact is not published (`outcome: skipped`, `reason: filtered`), which is
  pact-jvm's behaviour; `pact_verifier_cli` 1.3.5 publishes it as a success, and passes a filter
  that matches nothing with exit 0. `subsumption/decide` now warns per pair. The snapshot became
  `a_variant_no_document_records_is_refused` and `a_filtered_run_says_so_in_its_status_and_its_summary`.
  Not foreseen: the exit asked for the CLI's exit code to read a filtered run as not a pass, and it
  exits 0. It did what it was asked, both of today's verifiers exit 0, and the cases that matter are
  covered by the refusal and by publishing (ADR 0041 decision 5). No corpus or conformance case could
  hold either case: `corpora/` holds compiled plans, and `conformance/` has no provider side until
  S1.14, which gains them. They are protocol-level tests in `engine/kernel/tests/verification.rs`
  and `broker.rs`. S1.1 cannot compare a filtered run until `janus verify` takes the `--filter-*`
  options (S1.13), whose rows carry ADR 0041.

- **S1.18 A regex in a v1–v4 pact matches the whole value, as it does today** — **S**, decided.
  Found by S1.1's first run. The shape-language spec has `regex` match *anywhere* in a string, "exactly
  as v1–v4 pacts behave" (shape-language spec §4.2), and the plan grammar lets one `match:regex` serve
  both compilers on that basis (plan-grammar spec §4.4). The premise is wrong. pact-reference's native
  builds match with Oniguruma's `is_match`, which requires the whole string to match (only its WASM
  build searches), and pact-jvm uses Kotlin's `String.matches`, which does the same. So
  `\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z` accepts `2026-07-30T09:00:00Z-changed` under Janus and
  refuses it today, and every legacy or upgraded pact with an unanchored pattern accepts more than it
  does now. Decided (2026-10-18): the legacy compiler emits the pattern anchored (`^(?:…)$`), so every
  v1–v4 verdict is today's; `janus upgrade` writes the anchored form, so an upgraded contract keeps the
  pact's verdicts; and the shape language keeps its own rule, matching anywhere with explicit anchors,
  with §4.2's rationale corrected now that it cannot rest on what v1–v4 do.
  *Exit:* shape-language spec §4.2 and plan-grammar spec §4.4 say so; corpus cases for an unanchored
  pattern in a v3 pact and in the contract it upgrades into; and S1.1's ledger entry for
  `order-service/mutations/0/0/body/$.shippedAt/same-type` gone.
  *Done 2026-10-18* (`178cb64`). The legacy compiler always emits a pact's pattern as `^(?:…)$`, even
  one that looks anchored, since `^a|b$` matches `xb`; one function says so (`anchored_pattern`, in
  `engine/kernel/src/legacy/compile.rs` since S1.11). `janus upgrade` writes the anchored form, and a
  shape's own `regex` still matches anywhere. Shape-language §4.2, plan-grammar §4.4 and contract-file
  §8.2 say so, and the corpus cases are `legacy/v3-regex-whole-value` and
  `legacy/upgrade-regex-anchored`; no other verdict in the corpus moved. Not foreseen: the
  repeated-header disagreement (S3.3) moved from the negative case to the positive one, since the
  header's list is now judged as one whole value. The ledger says so.

- **S1.19 Generators in contracts** — **M**. S1.4 applies a v1–v4 pact's request generators, and
  `janus upgrade` still drops them (`generator-dropped`), so a pact with a `ProviderState` path
  verifies as today and the contract it upgrades into replays the recorded path. The shape language
  reserves a `generator` member naming a component (shape-language spec §3.4), and S1.4 built the
  component: `upgrade` writes a v1–v4 generator onto the slot's shape, naming `pact:<type>` and its
  configuration, verification applies a contract's request generators as the legacy path does,
  and the mock applies response generators when it serves (variant-semantics spec §5.2's replay by
  example gains its exception, and S2.3's fresh-per-exchange question is answered with it). Design
  first: the `generator` member's form, and whether a generated slot's recorded value is still the
  variant's evidence.
  *Exit:* an upgraded pact with request generators verifies as the pact does in S1.1's harness, with
  no `generator-dropped` left for the vocabulary `pact` provides; corpus cases for a generated
  request slot and a generated response slot; and a consumer conformance case for a served
  generated value.
  *Done 2026-10-25*, as [ADR 0048](decisions/0048-carry-generators-on-shapes-and-fill-fresh-values-where-a-value-is-produced.md)
  decides it, with the conformance case moved to S3.6: no DSL can attach a generator yet, and adding
  one is a behavioural-specification change for both SDKs. A shape's `generator` is `{name, config}`,
  and the example is still what a contract records, since a contract's identity is its content. A
  verifier fills a contract's request after state setup, and a mock its reply on every exchange, which
  answers S2.3's question. `generator/generate`'s context gains `side`. The harness half of the exit
  became a column for every fixture: S1.1's harness upgrades each case's pacts and verifies the
  contracts too, against `corpora/verifier/upgrade-disagreements.json`. 749 of 765 verdicts agree,
  and each of the other 16 has a reason: the header narrowing (S3.4), `contains` (ADR 0047), a
  pending interaction (S1.28, new), and an XML body written in other bytes. Not foreseen: that column
  found S1.25's conversion decoding an unlabelled JSON reply as bytes (JSON bodies now declare their
  type), and `eachKey` and `eachValue` on a list converted against their own rules.
- **S1.20 Member order in the document model, and the legacy body compiler retyped** — **M**. The
  shared prerequisite for form and XML bodies (S1.21, S1.22), split from S1.6. The document model
  loses member order (finding 11, formerly stage 5), though XML children and form fields are ordered.
  So order is kept through decode, the plan and encode; the third-party CSV component's mock serves
  columns alphabetically today (third-party component report item 10). And the legacy compiler
  walks a body as `serde_json::Value` (kernel-boundary review finding 5, closed on the premise that
  no v1–v4 pact body is non-JSON, which XML bodies disprove), so a content component's decoded
  document cannot reach it. It compiles over the document model instead. No verdict should move;
  the corpus and S1.1's ledger staying as they are is the evidence. It rewrites the code S1.11 moves
  behind a marked boundary, so S1.11 comes first. Since S1.6 the compiler's body is a `LegacyBody`,
  whose `Bytes` case becomes the document model's bytes here.
  *Exit:* the legacy compiler's body entry takes the document model, with no `serde_json::Value` in
  its walk; `corpora/` and S1.1's ledger unchanged by it; and a corpus case in which order is
  observable, the CSV component serving columns in the order its contract declares them.
  *Done 2026-10-22* ([ADR 0037](decisions/0037-carry-member-order-through-the-engine-and-decide-nothing-by-it.md)).
  Order is carried, never compared. An object's members keep the order they arrived in, through
  decode, plans, generation and encode: `serde_json`'s `preserve_order` is on, and the runtime value's
  objects and an `object` shape's `members` are insertion-ordered. Two objects with the same members
  are equal in any order, and whatever decides by members walks them by name: plans, the variant
  space, subsumption, and the v1–v4 compiler. A mock serves members as the shape declares them, so
  the CSV component's columns come out as the consumer wrote them. Its host test and the TypeScript
  SDK's CSV consumer now check the order. The legacy compiler's body is `LegacyBody::Document`, a
  runtime value, and its `Bytes` case is the model's bytes. S1.1's harness is unchanged: 326
  verdicts, 312 agree and 14 are known. No plan and no result in `corpora/` moved. The exit's
  "unchanged" held for verdicts but not for snapshots: five `executed.txt` files print a captured body
  in the order the case wrote it, not sorted. A new case, `shapes/member-order`, compares an example
  and a body that hold the same members in other orders. Not foreseen: a contract's canonical bytes
  had relied on sorted maps, so with order kept, every contract would have changed and then varied
  with the SDK that wrote it. The writer now sorts the records and a shape's own members, and keeps
  order only in an `object` shape's `members` and in documents (contract-file spec §2.4). A contract
  whose members were in name order writes the same bytes as before. Two decisions had silently read
  order once it was kept, and now go by name: subsumption's test for two shapes admitting the same
  values, and `janus upgrade`'s sample of an `each-entry` map.
- **S1.21 Form-urlencoded bodies** — **S**, after S1.20. A content component for
  `application/x-www-form-urlencoded`, decoding a body into an ordered document of names and
  values, matched as today's verifier matches a form body; and S1.4's generators applied to it,
  where they warn today.
  *Exit:* S1.1's ledger entries for `compatibility-suite/v1/23-response-with-form-post-body-positive-case`
  gone; corpus cases for a form body, matching and not; and a generator on a form body applied in
  S1.1's harness, with no `generator-failed` warning.
  *Done 2026-10-22* ([ADR 0038](decisions/0038-read-form-bodies-as-a-query-string-through-a-built-in-form-component.md)).
  A built-in `form` content component, `engine/component-form`, decodes a form into the query slot's
  model, an object of string lists in the order names first appear, and encodes one back. Every
  embedding registers it after text, and `engine/hello` lists it. The legacy layer reads a v1–v4
  form body into the same document with `serde_urlencoded`, and the legacy compiler compiles it with
  its query logic, now `compile_parameters`, as today's verifier hands a form to its query matcher.
  So a name the pact lacks is a mismatch in a response too. A form body's request generators go
  through `pact_models`' `FormUrlEncodedHandler`, asking the `pact` generator component. Both ledger
  entries for the compatibility suite's case 23 are gone, and S1.1's harness reads 328 verdicts: 314
  agree and 14 are known. Two corpus cases cover a form body, matching and not, with a rule on a
  name. Not foreseen: today's Rust verifier does not apply generators to a form body.
  `pact_verifier` builds `pact_matching` without its `form_urlencoded` feature, so the handler is
  compiled out, while pact-jvm applies them. Janus applies them (ADR 0038 decision 5), so the new
  `generators/form-body` fixture is a deliberate difference in the ledger, not an agreement. The
  harness's scripted provider gained `form.<name>` patterns to recognise a generated form value.
  Left out: `janus upgrade` still carries a form body across with `body-not-parsed`, which S1.25
  now covers.
- **S1.22 XML bodies** — **L**, after S1.20. An XML content component: elements, attributes, text and
  namespaces, with children in order; the v1–v4 matching-rule paths into XML, attributes and text
  nodes included; and S1.4's generators on XML bodies. This is where the pact-specification cases
  are closed: all 156 that are still skipped are XML.
  *Exit:* every pact-specification case the legacy harness runs passes, with its XML exclusion rule
  deleted and no `janus:skip` left; S1.1's ledger entries for
  `compatibility-suite/v1/19-response-with-xml-body-positive-case` gone.
  *Done 2026-10-22* ([ADR 0039](decisions/0039-read-xml-bodies-by-element-name-through-a-built-in-xml-component.md)).
  A built-in `xml` content component, `engine/component-xml`, claims the types today's verifier counts
  as XML. Every embedding registers it after JSON and before text, so `text/xml` reaches it, and
  `engine/hello` lists it. It parses with `sxd-document`, as today's verifier does. Its document is
  keyed by element name: the root's local name holds the root, and an element is `#namespace`,
  `@<name>` per attribute, `#text`, and an array of children per local name. So pact's XML paths are
  document paths. The legacy layer reads a pact's XML body into the same document, and a test holds
  that reader to the component's over every XML body in the repository. The legacy compiler walks it
  as `match_xml` does, with three new legacy actions: `legacy:xml-children`, `legacy:xml-only-children`
  and `legacy:parses-as`. Plan paths gained a quoted segment, `['@{urn:names}name']`. Request
  generators on an XML body go through `pact_models`' `XmlHandler`. The legacy harness runs all 741
  cases, with no exclusion rule and no `janus:skip`. Both ledger entries for case 19 are gone, and
  S1.1's harness reads 344 verdicts: 330 agree and 14 are known. Three fixtures are new.
  `bodies/xml-rules`, with mutations, and `generators/xml-body` agree with today's verifier.
  `bodies/xml-unexpected-namespace` is a deliberate difference. Corpus cases `v3-xml-body`,
  `v3-xml-body-mismatch` and `v3-type-rule-on-text` are new. Not foreseen:
  - two of the 156 cases, `unexpected xml namespace` in v3 and v4, fail under today's verifier and
    pact-jvm. pact_matching's own tests ignore them as incorrect behaviour fixed in its V2 engine.
    Janus follows the specification: an element the pact puts in no namespace matches its local name
    in any namespace (ADR 0039 decision 4). Keying children by local name is what makes that free;
  - today's verifier reads every text value against a type rule by parsing it, and Janus compared its
    kind. So an `integer` rule on a query parameter, a header or a path failed where today passes. It
    now reads the text (`legacy:parses-as`), for XML and for every other text slot;
  - `sxd-document` keeps no attribute order, so attributes are in name order;
  - the plan-grammar spec forbids the v1–v4 compiler `expect:absent` and `match:any-of`. The XML walk
    uses neither, but the compiler has emitted `expect:absent` for an empty body since the prototype.
    That reproduces today's verdict on a body the pact declares empty, so the spec now allows it there
    (plan-grammar §4.4, §4.5).

  Left out: `janus upgrade` still carries an XML body across with `body-not-parsed`, which S1.25 now
  covers. A size rule on an element counts its child elements, where today's verifier counts child
  nodes, whitespace included. That difference is deliberate (ADR 0039). A Janus contract's XML text
  is trimmed as a pact's is, CDATA sections included; keeping it as written is S2.10.
- **S1.23 Multipart bodies** — **M**, last of the body kinds. `multipart/form-data` and
  `multipart/mixed`, split at the boundary and each part compared by its own content type through
  the components S1.6, S1.21 and S1.22 add, as today's verifier compares them.
  *Exit:* S1.1's ledger entries for `compatibility-suite/v1/25-response-with-multipart-body-positive-case`
  and its mutation gone; and a v3 pact with XML, form, multipart and binary bodies agrees with
  today's verifier in S1.1, which was S1.6's combined exit.
  *Done 2026-10-22* ([ADR 0040](decisions/0040-read-multipart-bodies-by-part-name-through-a-built-in-multipart-component.md)).
  A built-in `multipart` content component, `engine/component-multipart`, claims every `multipart/*`
  type. Every embedding registers it after form, and `engine/hello` lists it. It splits a body with
  `multer`, as today's verifier does. Its document is keyed by part name, with unnamed parts under
  `""`, and a part is `headers`, `filename` and `body`. Each part's body goes through the built-in JSON,
  XML, text and form components the multipart component holds itself; any other part is bytes. The
  legacy layer splits a pact's body the same way and reads each part by its own type, and a test holds
  that reader to the component's over every multipart body in the repository. The legacy compiler
  compares the parts as `match_mime_multipart` does, with one new legacy action, `legacy:mime-part`. A
  reply is decoded under its own label, because its boundary is its own. No body kind is refused any
  more. All four ledger entries for case 25 are gone, and S1.1's harness reads 399 verdicts over 183
  cases: 389 agree and 10 are known. `bodies/multipart-form-data` and `bodies/every-body-kind`, the
  combined exit's v3 pact with an XML, a form, a multipart and a binary body, agree with today's
  verifier, mutations included. Corpus cases `v3-multipart-body` and `v3-multipart-body-mismatch` are
  new. Not foreseen:
  - nothing in the content interface lets one component ask another to decode. The multipart component
    holds the built-in ones itself, which covers everything today's verifier compares a part with; a
    part reaching a declared component is the new S5.6;
  - `multer`'s own `parse_boundary` reads only `multipart/form-data`, so the boundary is read with
    `mime`, as today's verifier reads it. `multer` keeps the last value of a repeated part header, and
    misreads a part with no headers at all by swallowing the next one. Today's verifier shares both;
  - today's verifier checks a rule on the pact's `Content-Type` header against the reply's type in a
    normalised form (`multipart/form-data;boundary=x`, no space) before it compares a body. A regex
    written with the space fails there, and passes in Janus. This holds for any type with parameters,
    not only multipart. It is the new S1.26, and `bodies/content-type-rule-with-a-space` records it;
  - a corpus case tags bytes only at the top of its values, so binary parts are covered by the kernel's
    tests and the verifier fixtures rather than the golden corpus.

  Left out: `janus upgrade` still carries a multipart body across with `body-not-parsed`, which S1.25
  now covers, and request generators on a multipart body are not applied, as today. The differences
  ADR 0040 makes deliberate are where today's behaviour is accidental: a field with a type of its own is
  compared as that type, a part with no `Content-Type` is text, unnamed parts are matched among the
  unnamed ones, and a part that is itself multipart is compared as octets.
- **S1.24 The v3/v4 matching rules the legacy compiler refuses** — **S**. `Values`, `EachKey`,
  `EachValue` and `ArrayContains` compile to a deliberate error node (`legacy/compile.rs`'s module
  docs), so a pact that uses them fails where today's verifier passes it. No pact-specification case
  covers them, which is why the 158 skipped cases never counted them. This was in S1.6's row of the
  findings table, and has nothing to do with body kinds.
  *Exit:* S1.1 fixtures using each of the four rules, matching and not, agree with today's
  verifier; corpus cases for each.
  *Done 2026-10-23* ([ADR 0042](decisions/0042-compile-the-collection-rules-of-a-v1-v4-pact-as-todays-verifier-applies-them.md)).
  The four compile as today's `compare_maps_with_matchingrule` and `compare_lists_with_matchingrule`
  apply them, on a body's objects and arrays. On an object, `values`, `eachValue` and
  `arrayContains` are a `flow:for-each` over its entries that compares each with the pact's value
  for its key, or the pact's first value; `eachKey` holds every key to its rules. On an array,
  `arrayContains` is one `legacy:array-contains` per variant. That action was in the grammar and
  never emitted; it now runs a template and lets one element satisfy two variants, as today, which
  the shape language's `match:contains` does not. S1.1's `matching-rules/` has a fixture per rule,
  with replies that match and that do not, and mutations: 379 verdicts over 183 cases, all agreeing.
  The harness reads 780 verdicts over 367 cases: 768 agree and 12 are known. The corpus cases are
  `legacy/v3-values`, `v4-each-key`, `v4-each-value` and `v4-array-contains`, each with a
  `-mismatch` twin. Not foreseen: an `arrayContains` variant with no `rules` is equality on the whole
  element, an object's extra keys included, and `eachKey` compares only the pact's own members, so a
  `$.prices.*` rule beside it reaches none of the reply's other values. Left out: the four declared
  on text (a path, a header, a query or form value, XML) are still refused, since today reads each
  differently and no fixture shows one; `janus upgrade` still converts them approximately (S1.25);
  and a mismatch inside an entry template is reported at a relative path (`~>.value.id`), as one in
  any rule-driven array already is, for the CLI output review.
- **S1.25 `janus upgrade` converts text, binary, form, XML and multipart bodies** — **S**. Split from
  S1.6, which made text and binary bodies verify; S1.21 added form bodies, S1.22 XML ones and S1.23
  multipart ones, which upgrade the same way. `janus upgrade` still carries a body that is not JSON
  across with a `body-not-parsed` finding and asserts nothing about it, so the contract verifies less
  than the pact it came from (plan-grammar spec §4.4's two-path agreement). The contract should declare
  the body's type in `content-types` (contract-file spec §5.5) and shape it: a text body as a string
  with its rules on `$`, a binary one as bytes with a base64 example, a form as an object of string
  lists with its rules on each name, an XML body in ADR 0039's document with its rules on each element,
  attribute and text, and a multipart body in ADR 0040's document with each part's rules on its body.
  A reply of another content type should fail the contract as `legacy:body-type` fails the pact (ADR
  0036 decision 4).
  *Exit:* corpus cases in which a text body, a binary body, a form body, an XML body and a multipart
  body upgrade and reach the pact's verdict (`also-compiled-from`), matching and not; and no upgrade has
  a `body-not-parsed` finding.
  *Done 2026-10-25*, as [ADR 0047](decisions/0047-upgrade-a-body-into-its-components-document-and-check-its-type-on-the-header.md)
  decides it. A text, form, XML or multipart body declares the pact's type, a multipart one without its
  boundary, which the multipart component now reads from a reply's label, and requires the built-in
  component. Octets stay undeclared, as bytes. Each body is shaped over its component's document as the
  legacy compiler compares it, and its evidence is the pact's octets, so a replay sends what the pact's
  verification sends. The reply-type check: the item asked that a mislabelled reply fail the contract as
  `legacy:body-type` fails the pact. It is a pattern on the reply's `content-type` header, for JSON
  bodies too, since a declaration is not a constraint (ADR 0020). The ten `corpora/legacy/upgrade-*-body*`
  cases are the exit, and a test holds every `upgrade-*` case's `also-compiled-from` to what the upgrade
  writes. Not foreseen: a number rule on text had converted to a kind no string has since S1.22, in a
  query, a header or a path too, and now converts to the pattern `legacy:parses-as` reads. `eachValue`
  and `arrayContains` (S1.24) were converted without their own rules. And the shape compiler resolved
  a member whose name has a dot or a brace as another member. `engine/kernel/tests/upgrade_agreement.rs`
  compiles every v1–v4 case in the corpus both ways and finds one verdict, except where a finding says
  why. A header with no rule still converts to `equality`, which is S3.4.

- **S1.26 A rule on the pact's `Content-Type` header, read as today's body check reads it** — **S**,
  a decision first. Found landing S1.23. Today's verifier checks a rule on the pact's `Content-Type`
  header twice: on the header as sent, and again before it compares a body, against the reply's type
  as `pact_models` writes it, with no space before a parameter and the charset in lower case
  (`application/json;charset=utf-8`). A regex written with the space, as most are, passes the first
  check and fails the second, so today fails a reply whose header the rule allows. Janus checks the
  rule once, on the header (ADR 0036 decision 4), and passes it. This applies to any type with
  parameters: a charset, or a multipart boundary. Either the legacy compiler copies the second check,
  or an ADR makes the difference deliberate, since today's own header check passes the same reply.
  *Exit:* the decision recorded, and S1.1's ledger entries for `bodies/content-type-rule-with-a-space`
  gone, or carrying that ADR as their reason.
  *Done 2026-10-23* ([ADR 0043](decisions/0043-check-a-pacts-content-type-once-on-the-header.md)). The
  difference is deliberate: Janus checks a pact's `Content-Type` once, on the header, and does not copy
  today's second check, which pact-jvm does not make either, comparing base types only. Not foreseen:
  the second check is not only the rule's. When the pact's type has parameters, it requires exactly
  those parameters unless the rule matches, so a reply that adds one (`; version=2`) fails it with no
  rule at all, after the header check allowed it. That is the new fixture
  `bodies/content-type-extra-parameter`. The four ledger entries for the two fixtures carry ADR 0043,
  and the harness reads 782 verdicts: 768 agree and 14 are known. A probe of the neighbouring cases
  found no other difference: a charset in another case, a pact type with no parameters, a rule matched
  with the same charset, and a rule written to allow no space all agree. Nothing the engine does
  changed.

- **S1.27 Registry credentials from the resolved configuration** — **S**. Split from S1.9 on
  2026-10-23, for X.3. The WASM loader reads `JANUS_OCI_USERNAME` and `JANUS_OCI_PASSWORD` itself
  (Phase 9 finding 16), so two runs of the same configuration on two machines can differ in whether
  they start, and nothing in the configuration says why: the one place a run still depends on the
  environment of the process that hosts the engine, which lifecycle-hooks spec §7.1 exists to rule
  out. Finding 16's option B: a component declaration's `source` takes an `auth` member,
  `${VAR}`-interpolated by the loader like a hook's `run.headers`, and the redaction rules of §7.3,
  and the frame log's, extended from hooks to component entries. Whether the two variables stay as a
  fallback for `janus component pull`, which has no configuration, is the decision.
  *Exit:* finding 16 closed; a component declaration's registry credential reaches the registry and
  nothing a run reports or logs, with a test, as S1.9's hook secrets have.
  *Done 2026-10-25*, as option B, which
  [ADR 0052](decisions/0052-declare-a-components-registry-credential-in-its-source.md) decides. A
  component's `source` takes `auth`, in the broker descriptor's shape (`basic` or `bearer`), which the
  loader interpolates, and a run's loader presents it and nothing else: a registry that wants a
  credential nobody declared refuses as `unauthorized`, saying where it belongs. The decision the item
  left open: `janus component push` and `pull` keep the two variables, having no configuration and
  not being runs, and the declaration they print names them as its `auth`. Lifecycle-hooks spec §7.3
  names the credential; the frame log, `Debug` and every refusal leave it out, and a malformed one is
  refused without being quoted, which serde's own message would not have done. `cli/tests/cli.rs`
  runs `janus verify` at `trace` against a registry whose token service wants the credential.
  Not foreseen: `--log-level trace` makes loading a component take seconds, since the WASM compiler
  logs at `trace` too; noted for the CLI output review.

- **S1.28 Pending interactions in a contract** — **S**. Split from S1.19 on 2026-10-25, which found
  it through S1.1's upgraded column. A v4 interaction may be `pending`: today's verifier reports its
  failure and does not fail the run, as a pending pact's is reported. `janus verify` does the same for
  a pact (S1.3). A contract has no pending interaction, so `janus upgrade` drops the flag
  (`pending-dropped`), and a failure of the upgraded interaction fails the run. The decision is whether
  pending belongs in the contract at all, beside the broker's pending pacts. If it does, it belongs on
  the interaction, under contract-file spec §3.2's placement rule, since it changes what a verifier
  does.
  *Exit:* `compatibility-suite/v4/01-verifying-a-pending-http-interaction` leaves
  `upgrade-disagreements.json`, or the flag is decided against in an ADR that the entry cites.
  *Done 2026-10-25*, decided against in
  [ADR 0051](decisions/0051-leave-pending-out-of-the-contract-and-let-upgrade-exclude-pending-interactions.md),
  which the entry cites. Pending is a fact about two codebases at a moment, which the broker works out
  from the provider's results and a fixed contract version cannot follow, so a contract has no
  pending interactions and a pending contract is the broker's to decide, as a pending pact is.
  The v1–v4 path keeps honouring the flag. `janus upgrade` still converts a pending interaction
  without it (`pending-dropped`), and `--exclude-pending` (`upgrade/pact`'s `exclude-pending`
  option) leaves it out instead (`pending-excluded`), for a pact whose pending interactions are known
  to fail. Not foreseen: a finding after a message or plugin interaction pointed at the wrong
  interaction, because upgrade counted only the HTTP ones; every finding now points at the pact's own
  position.

- **S1.29 The CLI output review** — **S**. The CLI's text grew an item at a time (S1.1–S1.28), and
  the oddities each item met were collected for one pass rather than polished inline. A review of
  every command's output on 2026-10-26, run against the sample provider, corpus fixtures and the
  error paths, found three kinds of thing:
  - *Wrong:* an error's `details` line indented as if it belonged to nothing; `--help` saying
    `--log-level debug` shows the frames, which only `trace` does; `janus upgrade` turning a document
    that is not a pact into an empty contract it calls an exact conversion; `janus explain --variant`
    ignoring an id no pact records, which `verify` refuses (ADR 0041); `explain --index` past the end
    reported as a malformed frame, with a hint to pass `--spec`; `janus publish` saying a broker it
    never reached "did not accept" the contracts; engine errors naming a file by its position
    (`contract 0`) when the CLI knows its path; a `--provider-url` that is not a URL refused by the
    transport, in the transport's terms; `pact_models` warnings at the default log level; and
    `--log-level trace` making a WASM component take seconds to load, since its compiler logs at
    `trace` too.
  - *Noise:* every engine error's `details` printed as raw JSON under its message; error chains that
    carry the HTTP library's layers (`error sending request: client error (Connect): tcp connect
    error: ...`) and the component wrapper's (`component-failed: the 'transport/http' component
    failed: ... [transport-failed]`); a hook that fails an exchange printed twice, the first time
    before the interaction it belongs to, and an aborting hook's error printed twice; `[base]` after
    every interaction of a pact, which has no other variant; summaries that list every zero tally and
    write `interaction(s)`; a run that never started printed as one that "did not complete (0
    interaction(s) in 0 document(s))"; a run stopped at its first failure counting only what it
    reached; and `(os error 2)`-style suffixes.
  - *Help text:* "as today's verifier" for `pact_verifier_cli`, the plan's word and not a user's;
    the global options interleaved with each command's own; and environment variables shown two ways.
  Decided with the review: a run the engine ends before it verifies anything — the broker or a URL
  cannot be fetched, no pacts are selected, a variant is named that nothing records — exits 2, as the
  same refusal for files does, which settles ADR 0041's open consequence. Today's verifier exits 1
  for each of these, and for a pact file it cannot read; `Documentation/verifier-migration.md` says so.
  *Exit:* each finding fixed, with the trycmd and snapbox cases that pin the output changed to match.
  *Done 2026-10-26*, in three commits, one per kind. Two findings moved to S1.30, since the engine
  writes the text and the corpora pin it: "Expected a integer", and the `pair(s)` and `variant(s)`
  in `janus check`'s report. Not foreseen:
  - `verify` and `upgrade` called a source they could not read "the session's interactions cannot
    produce a valid contract", `consumer-session/finalise`'s message. Reading a document now has its
    own.
  - The hook host's body excerpt always dropped the last character.
  - A stopped run's summary could not say how much it left: the count was only ever the hook
    report's, which a run with no hooks does not have. The summary now has `variants.not-run`
    (protocol spec §9.6).
  - Several messages leaned on their `details`. `variant-not-found` now names the recorded ids, and
    `hook-http-failed` quotes the reply's body.
  - The registry's credentials hint said "declare them" with nothing for "them" to name.
  - `broker-unreachable` said "the Pact Broker" of a pact fetched by its URL.
  Errors that name a file by its position are mapped back to the file by the CLI, which alone knows
  it. The URL flags are checked as the command line is read. The help names `pact_verifier_cli`, puts
  `--log-level` and `--no-colour` under "Global options", and shows every variable as
  `[env: NAME]`, never its value.

- **S1.30 Mismatch paths and absent values in plan text** — **S**, split from S1.29 because each
  changes what the engine reports, and so the specifications and corpora. A mismatch inside a
  `flow:for-each` template is reported at a relative path (`~>.value`, `~>`) without saying which key
  or index, as every rule-driven array has been since the prototype and ADR 0042's entry templates
  are; a mismatch from an action whose first child is not a `resolve` has no path at all
  (`expect:size(legacy:xml-children(...))`); an absent value prints as `NULL` in executed plan text,
  indistinguishable from a null; and a container for a member with no name is labelled `"$."` or
  `"$.[0]"` (a multipart body's unnamed parts). Also the article in kind messages ("Expected a
  integer", "a array"), and the plurals in `subsumption/decide`'s report text, which `janus check`
  prints (`0 pair(s) blocked`, `2 of 2 variant(s)`) and the corpora pin.
  *Exit:* every mismatch the corpora record has an absolute path; plan-grammar spec §3 says how an
  absent value prints, and the corpus snapshots follow it.
  *Done 2026-10-26.*
  - **Paths:** the interpreter carries, beside each current item, where it is in the interaction. A
    `resolve-current` records the absolute path it resolved, and a mismatch is located there.
  - **Path for a computed value:** a mismatch whose value under test is computed takes the path of the
    computation's own first child. The new corpus case `v3-xml-min-children-mismatch` is the first to
    record such a path.
  - **Spec:** plan-grammar spec §2.3 states both rules. §3.2 writes an absent value as `ABSENT`.
  - **Labels:** they are built from a second path, quoted as resolves are. The dimension path the
    labels used is also what variant ids are made of, and those are recorded, so it is unchanged.
  - **Corpus:** four `case.json` files lost a relative path, and five gained "an integer" or "2 items".
  Not foreseen:
  - The new case is one the upgrade does not agree with: a size rule on an XML element counts its
    children of every name, which no shape counts. The upgrade says so (`rule-unmapped`), so the case
    joins the agreement test's known list.
  - A label at a slot's root was never shortened (`:"$.response.body[0]"`). It is now `:"$[0]"`.
  - `janus upgrade`'s own findings still say `position(s)` and `rule(s)`; nothing here changed them.

- **S1.31 Colour in the CLI's output** — **S**, the first of the user-experience work that follows
  S1.29's review. Only the plans `explain` and `--explain-failures` print were coloured (plan-grammar
  spec §3.3). Every other line was plain on a terminal too, so a run's one red `FAILED` among forty
  `ok`s, or `check`'s `BLOCK`, had to be read for.
  *Exit:* every command's text output is coloured on a terminal and byte-for-byte unchanged off one,
  under `NO_COLOR` and under `--no-colour`, with tests that hold both.
  *Done 2026-10-26.*
  - **One palette** (`cli/src/style.rs`): green for a pass, red for a failure, yellow for what is
    worth a look but did not fail, blue for a location, dim for the line that explains the one above.
    Blue is the colour plans give a `resolve`, so a mismatch's path matches the plan beside it.
  - **Stripped in one place.** Every line `janus` prints goes through `anstream`'s auto stream, which
    drops the colour off a terminal. No caller chooses a styled or a plain way to print. Contract
    bytes and completion scripts are written as they are.
  - **`verify`, `publish`, `upgrade` and `component`** colour their status tags, the summary's status
    and its non-zero tallies, mismatch paths, and `upgrade`'s finding groups, by kind.
  - **`check`** colours the page `subsumption/decide` writes by its markers (`✗ ✓ ? ! ~ -`,
    `=> BLOCK|WARN|PASS`). The subsumption-check spec §6.4–6.5 and engine-protocol spec §8.6 fix
    those markers, so the CLI reads a specified grammar, not prose.
  - **Errors and help.** A command's `janus <command>:` error line is red, as clap's `error:` is.
    `--help` uses cargo's colours.
  Not foreseen:
  - `--no-colour` never reached `--help` or usage errors, since clap prints them while it parses and
    the flag was read afterwards. It is now read from the command line first.
  - clap ends a wrapped line with a space when the wrap falls inside a styled `[env: …]` note, even
    with the colour stripped. Those notes stay plain.

- **S1.32 `explain` chooses an interaction by its description** — **S**. `janus explain` took the
  interaction only by its position, `--index`. Nobody reading a pact knows where in it an interaction
  sits, so finding one meant counting, or running the command again with the next index until the
  right description came up.
  *Exit:* `--description <TEXT>` chooses the interaction, and a description that names none, or
  several, lists what there is to choose from.
  *Done 2026-10-26.*
  - **Matching:** an exact description wins. Failing one, it is the one whose description contains
    the text, ignoring case. An exact match taking precedence is what lets a description that is the
    start of another's be chosen at all (`a request for an order` beside
    `a request for an order that does not exist`).
  - **Several or none:** either is a usage error, exit 2, listing the candidates by position with
    their provider states. A pact describes interactions alike when only their states differ, and
    the states are what tells them apart; the position is what `--index` takes.
  - **In the CLI, not the protocol.** `verification/explain` takes an `index`, and the CLI turns the
    description into one, as it already checks `--index` against the document before the engine
    is asked. Which interaction of a file was meant is the command line's question. `--description`
    cannot be combined with `--index` or `--spec`.

- **S1.33 Errors carry their causes as a chain** — **S**. Decided by
  [ADR 0054](decisions/0054-carry-an-errors-causes-as-a-chain-and-mark-display-text-with-segments.md),
  decisions 1–3. An error's causes are flattened into its `message` where it is produced, so a host
  cannot tell the headline from the root cause. The CLI guesses at the layers: `render_error`
  leads a `component-failed` error with the component's own code.
  - An `EngineError`, and a component's error document, may carry `chain`: its layers outermost
    first, each in its own words.
  - `message` stays the complete sentence.
  - `engine/http-client`, the broker client and the hook host produce chains instead of joined
    strings.
  - The CLI leads with the innermost coded layer and the root cause, and `--error-chain` prints
    every layer.
  - **How it is built:** with the standard library and a small type the kernel owns, not anyhow.
    - A `Layer` is `{ code, message }`, the shape that crosses the wire.
    - A helper walks a foreign error's `source()` chain (reqwest, io, serde) into layers. It takes
      the producer's rule for which layers are only wrappers, so `describe`'s filter moves into it.
    - A `Context` trait on `Result` adds an outer layer, with `.context(message)` or
      `.layer(code, message)`. It reads like anyhow's at a call site, but a layer can carry a code
      and the result is still a plain value.
    - Where a host crate's errors are typed, thiserror's `#[source]` keeps the cause as an error
      rather than a string: `LoadError::Unreadable` keeps its `io::Error`. thiserror is already in the
      tree.

  *Exit:* the engine-protocol and component-interfaces specs and their schemas carry `chain`. The
  bindings are regenerated, and every error the HTTP client, broker client and hooks produce
  carries one. `render_error` has no case for a code. The trycmd and snapbox cases show both
  renderings.
  *Done 2026-10-26.*
  - **Kernel:** `engine/kernel/src/chain.rs` has `Layer`, `layers_of` over a `source()` chain, and
    `Context`. `EngineError` and `ComponentError` carry `chain`, and `component_failed` writes its own
    layer, then the component's.
  - **Producers:** the HTTP transport, the broker client's unreachable, refused and not-JSON answers,
    and the hook host's `http` and `exec` failures carry chains. An error with no cause carries none,
    as the spec allows.
  - **CLI:** an error leads with its innermost coded layer, and says its root cause on a
    `caused by:` line beneath. `--error-chain` prints every layer.
  - **Specs:** engine-protocol §10.1, component-interfaces §11.1 and §11.3, the lifecycle-hooks spec,
    and both error schemas. The bindings gain `ErrorLayer`.
  Not foreseen:
  - **`SendError` says only its own layer**, as Rust's errors conventionally do, with its causes as
    `source()`. Every caller that printed it now reads the chain or joins it.
  - **A failing `exec` hook says why.** Its stderr was only in `details`, which text never shows. Its
    last stderr line is now the root cause, as an `http` hook's reply body already was.
  - **Wording changes.** A broker's retry-after note moved onto the answer it describes, ahead of the
    broker's explanation. A configuration that cannot be read no longer says `(os error 2)`.
  - **`Context` has no caller yet.** None of this item's producers adds a layer above an error it
    already has, so `.context()` and `.layer()` are tested but unused until one does.
  - **Out of scope:** the HTTP client's configuration errors stay a position and a sentence. They are
    problems with what was asked for, not chains of causes. The WASM loader's and OCI registry's
    errors carry no chain yet.

- **S1.34 Display text is marked with segments** — **M**, after S1.33. Decided by
  [ADR 0054](decisions/0054-carry-an-errors-causes-as-a-chain-and-mark-display-text-with-segments.md),
  decisions 4–8. A host styles the engine's text by recovering structure the engine had. The CLI
  re-tokenizes plans by linking the kernel, which no SDK can do, and parses the `check` page's
  markers.
  - A host that declares the `segments` capability receives `text-segments` and
    `message-segments` beside the strings: pieces whose texts, joined, are the string, each with a
    `kind` from one open vocabulary.
  - The first carriers are the plan text forms, the `subsumption/check` and `subsumption/decide`
    text, and mismatch messages.

  *Exit:*
  - The engine-protocol spec defines the capability, the sibling rule and the kinds, and
    plan-grammar spec §3.3 says segments are how a host styles a plan.
  - The kernel's renderers write through one builder that produces both, and a corpus test holds
    every recorded text form to its segments.
  - `cli/src/highlight.rs` maps kinds to styles, with no tokenizer call or marker parser.

  *Done 2026-10-26.*
  - **Kernel:** `engine/kernel/src/display.rs` has the builder, `Marked`, which keeps text and
    segments together. The plan renderer, the subsumption block and the `can-i-deploy` page write
    through it. The interpreter marks the values a mismatch's message quotes. Every message is
    unchanged, and the corpora pin them.
  - **The check:** the corpus runner holds each case's renderer segments to the tokenizer's reading
    of its text, piece by piece. The tokenizer is now a test of the renderer.
  - **Protocol:** the `segments` capability, `text-segments` and `message-segments`
    (engine-protocol spec §2.7, `display.schema.json`). The carriers are explain's two text forms,
    the `executed-plan` event, decide's page, and the mismatches of explain and of
    `interaction-result` events. The bindings gain `Segment` and a typed `Mismatch`.
  - **CLI:** it declares the capability, and colours plans, the `check` page and mismatch values
    from segments. It no longer calls the kernel to style.
  Not foreseen:
  - **`subsumption/check` has no text.** Only `decide` sends a page, so the ADR's
    `subsumption/check` carrier has nothing to carry.
  - **`--output json` would have carried segments**, since the run summary's failures include
    mismatches. A command asks for segments only when it prints text.
  - **The generator mishandles a `$ref` with a sibling `description`.** It names one copy
    `Segments1` and never declares it. The carriers reference `Segments` bare, and the description
    lives on the definition.
  - **A consumer exchange's mismatches stay plain.** They reach SDKs, which style nothing yet. They
    gain segments under the same rules when an SDK needs them.
  - **A mismatch log line printed the marks.** It `Debug`-printed whole mismatches, so it now prints
    only path and message.
- **S1.35 The v4 `statusCode` matcher** — **S**, found by S1.15's examples. pact-js's `matchers`
  example checks a response's status by class (`success`). The legacy compiler refuses the rule, so
  `janus verify` fails an interaction today's verifier passes, and `janus upgrade` writes it as
  `http:status-class`, an operator no component contributes yet. The built-in HTTP component
  contributes the operator, and the legacy compiler uses it for a pact's rule, so the kernel still
  knows nothing of HTTP statuses.
  *Exit:* `examples/pact-js/matchers` has no entry in `corpora/verifier/disagreements.json`, and an
  upgraded contract with a status class verifies.
  *Done 2026-10-30* ([ADR 0057](decisions/0057-the-http-component-runs-http-status-class-as-an-in-tree-matcher.md)):
  - `engine/component-http` implements the matcher interface's `apply` for `http:status-class`: a
    `class` (`success`, `client-error`, …, the ranges today's verifier checks) or a list of `codes`.
  - An embedding registers a matcher compiled into it with `Engine::register_matcher`, and every scope
    runs its actions through `matcher/apply` as it runs a declared component's. `janus` and
    `janus-engine` register the HTTP one; the WASM engine, which has no HTTP component, fails the node
    by name.
  - The legacy compiler compiles a pact's `statusCode` rule to the same action, with the members
    `janus upgrade` writes. The upgrade now writes a code list as `codes`.
  - Evidence: `corpora/verifier/matching-rules/status-code`, every class and a code list, with
    mutations past each class's edge, where both verifiers and the upgraded contracts agree on all
    46 verdicts; `corpora/legacy/v4-status-class` and its `-mismatch` twin, compiled from the pact and
    from its upgraded contract. pact-js's `matchers` example verifies in full, and its upgraded
    contract's interaction joins the other three in the upgrade ledger, on S3.4's charset difference.

**Stage 1a is done when** S1.1 reports zero unexplained disagreements over the fixture corpus, the
pact-specification cases pass in full, and pilot providers have run `janus verify` beside their current
verifier on real pacts, from a broker, with identical results for a release cycle.

#### Stage 1b: provider-side integrations for the ecosystems that verify inside a test

Most providers do not run a CLI. They run verification inside a test: pact-jvm's provider runner with
state methods, pact-js's `Verifier` with state handlers, and the same in Go, .NET and Python. The
hook that makes an in-process state handler possible already exists on paper: the test process serves
a loopback endpoint and the configuration names it as an `http` hook (lifecycle-hooks spec §8.4). No
SDK implements it, and the SDK specification covers only the consumer side.

- **S1.14 The provider side of the SDK specification** — **M**. A provider section of
  `behavioural-spec.json` and conformance cases for it: verify-in-test, state handlers and request
  filters as loopback hooks, selecting pacts by selector, publishing results, and the per-variant
  result surface, including a filter naming no recorded variant and a filtered run (S1.17, whose
  cases are protocol-level tests until then). The thinness layers for it in each SDK's `thinness.json`.
  *Exit:* the conformance lint passes with provider ids; both drivers run the provider cases.
  *Done 2026-10-30*, as [ADR 0055](decisions/0055-verify-a-provider-in-a-test-through-the-run-with-handlers-as-loopback-hooks.md)
  decides. The behavioural specification has a `verification` category: `verifier`, `state-handler`,
  `request-filter` and `verify`, with 23 conformance ids. The conformance suite has a `provider`
  category, 14 cases against the scripted engine, which now calls the SDK's hooks during the run as a
  real engine would, and one `live` case, in which the real engine calls a test's state handler and
  request filter against a provider the driver serves. Both SDKs pass all 62 cases. Each has a
  provider API of its own: `Verifier` in TypeScript, `Verifier.forProvider` on the JVM. A verify is
  one run, with the handlers served on a loopback endpoint behind a bearer token for that run. Not
  foreseen: an SDK had no way to read `verifier.janus.yaml` without writing a loader of its own, so
  the engine's host loads it (`config/load`, engine-protocol spec §8.8, behind `config-loading`).
  The thinness audit has a `loopback` layer, and the TypeScript idiomatic budget moved from 700 to
  850 for the four primitives. A result's interaction is the engine's reference to it, an object,
  not its description, and the spec's own example had it as a string. The JVM SDK resolved a
  relative path without normalising it, which the `config-loaded` case found. Left for S1.15:
  message producers (pact-js's `messageProviders`, a `produce-message` hook whose answer is a whole
  `parts` document), and every test-framework integration.
- **S1.15 JVM and JavaScript provider integrations** — **L**. A JUnit 5 provider runner and a
  `Verifier`-shaped API for Jest and Vitest, over the subprocess, from generated bindings, under the
  thinness audit. Each ships with a migration note from today's annotations and options.
  *Exit:* pact-jvm's and pact-js's provider example suites run on the new integrations with
  identical results in S1.1.
  *Done 2026-10-30*, as [ADR 0056](decisions/0056-provider-test-integrations-message-producers-a-junit-extension-and-a-pact-js-shaped-verifier.md)
  decides. The behavioural specification gains `message-producer`: a test's message producers are
  loopback `produce-message` hooks, as S1.14's handlers are, and a verifier takes the HTTP
  transport's other options as `transport`. Both SDKs pass all 66 conformance cases, including a
  live one in which the real engine asks a test's producers for two messages.
  - The JVM SDK has `JanusProviderExtension`, with `@State` and `@ProvidesMessage` methods. A
    `@TestFactory` returns one dynamic test per interaction result, as the one run reports it
    (`Verifier.start()`).
  - The TypeScript SDK has `pact-janus/verifier`: pact-js's `Verifier`, `MessageProviderPact` and
    `providerWithMetadata`, mapping each option as S1.13 maps the CLI's. It is the thinness audit's
    new `facade` layer.
  - [`provider-test-migration.md`](provider-test-migration.md) is the migration note for both.
  - pact-js's seven runnable provider examples are ported to `sdks/typescript/test/examples`;
    `plugins` is left out, since ADR 0045 refuses a pact plugin. Seven of pact-jvm's JUnit 5 provider
    tests are ported to `PactJvmExamplesTest`. Their pacts are S1.1 fixtures
    (`corpora/verifier/examples/`, and S1.7's `messages/` ones), and every interaction reaches the
    verdict S1.1 records there.
  - Not foreseen:
    - An in-test producer has no reply to take a `contentType` from, and every pact-jvm message pact
      expects one in its metadata. The engine now reads a missing one as the message's own type.
    - pact-js's `matchers` example uses the v4 `statusCode` matcher, which the legacy compiler
      refuses. It is ledgered against the new S1.35.
    - pact-jvm passes `ProviderStateInjectedTest` by generating the expected `LOCATION` header from
      provider-state values, which today's `pact_verifier_cli` does not do. Janus agrees with today's,
      so that fixture records no `expected`.
    - Express adds `charset=utf-8` to every JSON reply, which an upgraded contract's `Content-Type`
      equality fails. These are ten more S3.4 entries in the upgrade ledger.
  - Budgets: the JVM idiomatic layer moved from 2,000 to 2,100 lines and its integration layer from
    140 to 280. The TypeScript facade's is 300.
- **S1.36 Releases of the SDKs** — **M**. Stage 1b's exit needs pilot providers verifying from
  their own test suites, and neither SDK is released: each is at `0.0.0`, a pilot builds it from a
  checkout, and an installed one finds no engine unless it is named or on the `PATH`. The pinned
  resolver S0.10 wrote is test code. Decided by
  [ADR 0060](decisions/0060-release-each-sdk-as-files-on-a-github-release-of-its-own-and-fetch-its-pinned-engine.md):
  each SDK is a knope package of its own, released as files on a GitHub release tagged
  `<package>/v<version>` (an `npm pack` tarball; jars, POMs and Ivy descriptors), and fetches the
  engine its pin names on first use. Registries wait for graduation (S2.9), or for a pilot who
  cannot use a release this way.
  *Exit:* `typescript-sdk/v0.1.0` and `jvm-sdk/v0.1.0` are released. A project outside the repository
  installs each from its release with no Rust toolchain and no `JANUS_ENGINE`, and runs a consumer and
  a provider test against the engine the SDK fetched.
  *Built 2026-10-31*; the exit waits on the first release of each:
  - `knope.toml` has three packages, `engine`, `typescript-sdk` and `jvm-sdk`. The `Change file`
    workflow asks for a change file naming each package a pull request ships in, and
    `prepare-release.yml` takes `<package>=<version>` overrides.
  - `release-sdks.yml` releases an SDK whose version has no tag. It runs the suite against the pinned
    engine with no Rust, then installs the packed files into `sdks/<sdk>/release-check`, copied out of
    the repository, on Linux (x86_64 and aarch64), macOS and Windows. It tags and publishes only then,
    and installs the published release by its URL.
  - The TypeScript SDK's pin is `src/engine/pin.ts`, which `engine-pin` writes beside
    `sdks/engine-release.json`, since its CommonJS build cannot find a JSON file shipped beside it. The
    JVM SDK's build copies the JSON into its jar. `release.yml`'s `engine-pin` branch now carries both,
    and a change file for each SDK.
  - The resolvers moved from each SDK's tests into its `embedding` layer. Its budget went from 250 to
    370 lines in TypeScript, and from 300 to 480 in the JVM SDK.
- **S1.37 The engine's logs, where each runtime's are** — **S**, found verifying PactFlow's
  `example-provider-springboot` through the JVM SDK. Both SDKs inherit the engine's stderr, and only
  `RUST_LOG` sets how much it logs. On the JVM the logs reach no logging framework, no test report
  and no IDE, and Spring's `logging.level` cannot reach them. The pact-js facade ignores `logLevel`.
  Decided by
  [ADR 0061](decisions/0061-the-engines-logs-are-the-runtimes-own-set-and-shown-by-its-logging-configuration.md).
  *Done 2026-10-31*:
  - `janus-engine` and `janus` write each event as a JSON line with `JANUS_LOG_FORMAT=json`.
  - The JVM SDK logs the engine's events through SLF4J, to `io.pact.janus.engine`, whose level
    becomes `RUST_LOG`. `example-provider-springboot` shows them, in Spring's log format and in its
    test report, from `logging.level.io.pact.janus.engine=DEBUG` in `application.properties`.
  - The TypeScript SDK takes `logLevel` on `Janus` and `Verifier`, which the facade passes through,
    else `LOG_LEVEL`, else `warn`.
  - A `RUST_LOG` that is set is passed as it is, in both SDKs.
  - Budgets: the embedding layer moved from 480 to 600 code lines in the JVM SDK, and from 370 to 390
    in TypeScript.
- **S1.16 Other ecosystems** wait for their SDK in stage 6, or wrap the CLI (S1.13's table is their
  migration note).

**Stage 1b is done when** S1.15's exit holds and a pilot provider in each language has verified from
inside its test suite for a release cycle.

### Stage 2: The consumer engine, and the JS and JVM SDKs — **XL**

The RFC's second migration step: SDKs ship MkII as a new major version.

- **S2.1 The consumer side of the engine, production-hardened** — **L**. Sessions; the HTTP
  component's **server** half rewritten async and thread-free, behind an executor seam so that native
  runs on `tokio` and a `wasm32-wasip3` build runs on WASI async with the same loop (gate W then costs
  a build target, not a redesign; ADR 0023 decision 5); and the engine, not the SDK, writes the
  contract's canonical bytes (finding 6).
  *Exit:* 9.1's consumer scenarios run on the new transport with no thread created by the engine, and
  both SDKs' conformance contracts are byte-identical because neither wrote them.
- **S2.2 The consumer's test verdict reaches the engine** — **M**. `report-variant` (ADR backlog;
  findings 4, 5): the engine, not each SDK, withholds a dishonest contract, and per-interaction
  results arrive before the suite ends, so an engine-side failure surfaces in the test that caused it.
  *Exit:* the TypeScript SDK's own withholding logic is deleted; a conformance case fails a suite on
  the variant that failed.
- **S2.3 Plans cached per (handle, variant)** — **S**. Arming a variant recompiles its plan (finding
  28); the cache, and a decision on what a fresh-per-exchange generator means under it. S1.19 decided
  that one (ADR 0048): a reply's generated positions are filled every time it is served, after the
  cached plan and the armed reply, so the cache holds no generated value.
  *Exit:* the 100 KB request scenario in S0.3's trend drops to the small-exchange cost.
- **S2.4 Two SDKs rebuilt from the SDK specification** — **L**. JavaScript/TypeScript and JVM, each
  with generated bindings, an idiomatic layer, the test-framework integrations people use (Jest and
  Vitest; JUnit 5, plus JUnit 4 and Spock if the community wants them), and the thinness audit. The
  JVM SDK is the proof that pact-jvm becomes one SDK among several.
- **S2.5 A compatibility facade** for today's pact-js and pact-jvm DSLs — **L**, so most existing
  consumer tests need only mechanical changes (SDK spec §6's classification). The facade is the
  hand-written layer most likely to grow matching logic, so it is its own layer in
  each SDK's `thinness.json`, with its own budget, and a `kept`/`changed`/`dropped` verdict per old primitive
  is published as the migration note.
  *Exit:* the example suites of pact-js and pact-jvm run on the facade.
- **S2.6 The conformance suite extended** — **M**, to the things it does not cover (conformance
  report §7.2): the test-framework integrations, the facade, and per-language refusals (ADR 0019).
  *Exit:* every facade primitive classified `kept` has a conformance case.
- **S2.7 The rename, when the name is decided** — **S**, blocked on §7.1 (S0.9), and **not** a stage
  2 exit condition: if the answer has not come, stage 2 ships its major version under the codename.
  When it comes, S0.5's rehearsed change runs for real, and the persisted identifiers follow S0.5's
  dual-spelling rule for one major version, so contracts and components published under the codename
  stay readable and are never republished. Converted and recorded contracts may therefore be published
  to a shared broker before the rename.
- **S2.8 Secrets in recorded contracts** — **S**. A consumer test that sends a real token records it.
  The lifecycle-hooks redaction rules extend to the recorded contract and to `explain --executed`
  output, and the SDK specification says which headers and members are redacted by default.
- **S2.9 The SDKs graduate** (ADR 0027) — **M**. The TypeScript SDK moves into pact-js and the JVM
  SDK into pact-jvm, each as that package's next major version, with its conformance driver, its
  thinness classification and its generated bindings, running S0.11's gates against the pinned engine
  release from its new home. This repository keeps no SDK source afterwards; the demo consumes the
  published packages. If a maintainer group declines the graduation, the SDK gets a repository of its
  own under the Pact organisation (ADR 0027's tripwire).
  *Exit:* both SDKs' CI is green in their new repositories against a released engine, the published
  packages pass the conformance suite, and `sdks/` here holds only `bindings.json` and the README that
  says where they went.
- **S2.10 XML text as written, in Janus contracts** — **M**, Janus-only behaviour. The `xml` component
  reads an element's text as today's verifier does: each text node trimmed, then joined (ADR 0039
  decision 2). `sxd-document` reads a CDATA section as one more text node, so its contents are trimmed
  too. That is right for a v1–v4 pact, whose verdict depends on it. In a Janus contract it loses
  whitespace that XML counts, and the literal contents of a CDATA section, and a mock cannot serve
  either. The component keeps text as written, with each text node and CDATA section apart. The legacy
  compiler then trims each node and joins them itself, so no v1–v4 verdict moves. That needs a parser
  that reports CDATA, or a scan for it, accepting exactly the documents `sxd-document` accepts. It
  changes what `#text` means in a contract, so it lands before contracts with XML bodies are published,
  and `janus upgrade` (S1.25) shapes a pact's XML text so that whitespace still does not count. An ADR
  amends ADR 0039 decision 2, and decides whether `encode` writes a CDATA section back as one.
  *Exit:* corpus cases in which a contract's XML text with surrounding whitespace, and a CDATA section
  with it, matches only text written the same way; the legacy harness and S1.1's verdicts unchanged;
  S1.25's XML cases still reach their pact's verdict.

**Done when** both SDKs pass the extended conformance suite against a pinned engine, the example suites
of pact-js and pact-jvm run on the compatibility facade, both SDKs have graduated (S2.9), and early
adopters publish contracts to a broker that the stage 1 verifier checks. S2.7 lands when §7.1 answers,
before or after.

### Stage 3: Shapes and variants in the DSLs — **L**

The RFC's answer to optional fields, delivered to users.

- **S3.1 The DSL decisions, first** (gate D) — **S**: whether an array admits `[]` by default
  (finding 2); how to exclude combinations a provider cannot produce (finding 35); how correlated
  fields are written, `oneOf` or independent members. Each is an ADR before S3.2 starts.
- **S3.2 The native DSL** — **L**: `optional`, `anyOf`, `oneOf`, `eachLike`, `forbidden`,
  `whenVariant` and `variantCases`, designed with the community, specified first, then implemented in
  both SDKs under the conformance suite.
- **S3.3 `http:media-type` and list-valued headers** (finding 1) — **M**, as namespaced operators the
  HTTP component contributes, so a consumer's `application/json` accepts `; charset=utf-8` again.
  List-valued headers stay here rather than in stage 1a, deliberately, though S1.1's ledger carries a
  disagreement they account for (a repeated response header judged as one list). Treating header
  values as lists has broken users more than once across the Pact implementations, in issues and
  pull requests. The trouble is HTTP/1.x's comma-joined values, which only some headers define as
  lists; HTTP/2 and later carry each field line separately and define field values precisely. So the
  semantics are specified per HTTP version, and checked against those reports, before they change.
- **S3.4 `upgrade` without the media-type narrowing** — **S**. It already reports the narrowing; with
  S3.3 it stops making it.
- **S3.5 Variant failure reporting in each test framework** — **M**: one test per variant where the
  framework registers tests dynamically, a per-variant result list where it does not, and the exchange
  loop's match decision as an event a host can surface while the run is going (variant ergonomics
  report, recommendations 1–3).
- **S3.6 Generators in the DSL** — **S**. Split from S1.19 on 2026-10-25. A shape carries a generator
  and a mock fills it fresh on every exchange (ADR 0048), but no DSL can attach one, so a consumer
  suite writes no generated value unless it hands the engine an interaction specification. A DSL
  primitive naming a generator and its configuration, specified in the behavioural specification
  first and implemented in both SDKs. The SDK specification refuses `like(v)` with a random function,
  on ADR 0008's grounds. A declared generator does not run into that refusal: the engine generates the
  value and the recording stays the example.
  *Exit:* the consumer conformance case for a served generated value that S1.19's exit named, passing
  in both SDKs.

**Done when** the RFC's consumer example runs natively in both SDKs, the demo runs on them, and the DSL
is frozen as the major version's surface.

### Stage 4: Provider shapes and the subsumption check — **L**

The part of the RFC no Pact user has today: catching what the provider may send that no consumer tested.

- **S4.1 Recording provider shapes from provider test suites** — **M**, in each provider language's
  test framework, through the S1.15 integrations. Janus records only through its own verifier (task
  7.2).
- **S4.2 Deriving shapes from OpenAPI and protobuf** — **M**, emitting selectors so they match
  consumer interactions by operation (ADR 0025). Spike 7.3's importer is the starting point, and its
  lessons are the specification.
- **S4.3 The broker PR series** (broker notes §10, rows 1–8) — **L**: a resource for provider shapes,
  decisions computed on read, subsumption reasons in `can-i-deploy`, stored policy, and rendering from
  the report's own words; `janus publish` learns provider shapes (S1.3). PactFlow's side is PactFlow's
  to decide.
- **S4.4 Policy** — **S**: a `provenance` selector for exemptions (finding 9B); whether `block` ever
  becomes a default, judged on pilot data, not decided now (gate P).
- **S4.5 The WASM offline artifact in the broker** (broker notes row 9) — **M**: recompute on cache
  miss, the first real host of ADR 0023's offline embedding. Whether N×M recomputation scales is
  measured here (broker notes §12).
- **S4.6 Message interactions in the broker view** — **S**. The walk is direction-agnostic, but a
  message-only provider's shape and decision page are untested (broker notes §12).

**Done when** the demo's loop runs against a real broker, with a provider shape published from a real
provider's test suite.

### Stage 5: Plugins as the core design — **L**

The component model, opened to the ecosystem.

- **S5.1 WASM components, generally available** — **L**: the interfaces, the loader, OCI
  distribution, and **signing and verification** of component artifacts, which the prototype never
  did. First decide findings 15 (recording which components took part, with their digests), 16
  (registry credentials, which 2.7 says a run must not take from the environment) and 17 (artifacts
  published with `wkg`).
- **S5.2 A bridge for today's pact-plugins** — **L**, at least the protobuf/gRPC one, so no existing
  user loses a plugin. A shim process speaking both wires is the smaller route (component-interfaces
  spec §9). This closes S1.8's refusal.
- **S5.3 Content components that are not JSON** — **M**: `encode` seeing the shape (finding 12), and
  decode errors and degradations reaching the user as their own finding kind, not as "expected a
  value" (findings 13, 14). Member order moved to S1.20.
- **S5.4 Plan fragments versus operator-level substitution** — decision, gate F, before any third
  party depends on either; with it, a fragment's access to octets (finding 22) and where action
  configuration lives in the grammar (finding 24, ADR 0022 decision 6's provisional mapping).
- **S5.5 Out-of-process components**, if the community wants them beyond the bridge — **M**: the
  containment rules of findings 18 and 20 (recreation in a shared process; what a `subprocess`
  source's command is).
- **S5.6 A multipart part reaches the registry's components** — **M**. The built-in multipart component
  decodes and encodes each part's body through the built-in JSON, XML, text and form components it holds
  itself, and any other part is bytes (ADR 0040 decision 3). Stage 1a needs nothing more, because today's
  verifier compares a part with its built-in matchers too. But a Janus contract cannot shape a part only
  a declared component reads, such as a protobuf part, and a declared component for `text/csv` or JSON is
  passed over for a part of that type. The content interface gains a way for a component to ask the
  engine to decode and encode embedded content through the registry: a host callback in the native
  binding, an import in the WASM world, and a request on the subprocess pipe (component-interfaces spec
  §9). The multipart document does not change.
  *Exit:* the worked CSV component (`third-party/janus-csv`) decodes a `text/csv` part of a multipart
  body into rows a contract shapes, in each binding; and an ADR supersedes ADR 0040 decision 3.

**Done when** a protobuf plugin runs through either the bridge or a native component, and a second
content component exists written by someone outside the core team.

### Stage 6: Breadth — **XL**, ongoing

- **S6.1 More SDKs:** Go, Python, .NET, Rust, Ruby, PHP, and whatever else the community maintains.
  Each is generated bindings, an idiomatic layer written from the SDK specification, **both sides**
  (consumer and provider, S1.14), and the conformance suite, incubated here and graduated into its
  language's existing repository the way S2.9 did (ADR 0027). The regeneration trial (task 6.5) showed
  agents can carry a specification change into several SDKs under the suite's judgement. That is what
  makes this stage affordable for a small team, but it does not replace the suite.
- **S6.2 Consumer-side message interactions**, end to end, with broker adapters as components. The
  provider side landed in S1.7.
- **S6.3 gRPC and other transports** as components.
- **S6.4 The RFC's fourth migration step:** today's FFI and pact-jvm cores go into maintenance mode as
  each language's MkII SDK passes conformance on both sides. The timeline is the community's (§7).
- **S6.5 Text in charsets other than UTF-8**, in the built-in `text` component. It reads and writes
  every body as UTF-8 whatever its `charset` says (ADR 0036 decision 1), which is today's verifier's
  behaviour and so stage 1a's. A body in another charset fails to decode. Worse, a consumer mock
  serving `text/plain; charset=utf-16` sends UTF-8 bytes under that label. The component decodes
  and encodes by the `charset` parameter, using the WHATWG encoding labels (`encoding_rs`), and
  refuses a charset it does not know rather than mislabel the body. A v1–v4 pact whose body names no
  charset is still read as UTF-8, so S1.1's verdicts do not move. In practice this is Windows-1252
  and Latin-1 from older stacks more than UTF-16. It supersedes ADR 0036's rejected alternative
  "Decode `charset`s other than UTF-8".

**Done when** it never is. Each SDK has its own exit: conformance passing, and the old SDK's example
suites, consumer and provider, running on its compatibility facade.

### Stage A: The AI-assisted layer — **M**, optional, after stage 3

The RFC's optional layer, which the prototype designed and did not build
([design notes](prototype/ai-layer-design-notes.md), task 2.10). The notes found the deterministic design needs
no new protocol operation, event kind or hook kind for either capability, so this stage is additive
and can be skipped without anything above it noticing.

- **SA.1 Mismatch diagnosis** over `explain --executed` output: a summariser, its prompt and its
  evaluation, which the notes deferred.
- **SA.2 Agentic verification**: the orchestration loop behind `exec`/`http` hooks that stand up a
  provider and satisfy states, with its retry policy.
- **SA.3 The security review** the notes defer: an agent with provider control-plane access crosses the
  boundary human-written state-change scripts already cross, and that trust model is reviewed, not
  assumed, before either capability is shipped.

### Cross-cutting: security

The prototype's one security line is "credential redaction reviewed for production". A verifier that
runs in CI with broker credentials, executes scripts from a repository's configuration and loads
third-party code needs more than that. Each item is owned by a stage; together they are the threat
model, written down as it is built.

| Item | Stage | What it covers |
|---|---|---|
| **X.1 The subprocess binary's provenance** | S0.4 | How an SDK resolves `janus-engine`: pinned version, verified checksum, the `JANUS_ENGINE` override logged, never a bare `PATH` lookup |
| **X.2 The scripted-hook sandbox** | S1.9 | QuickJS runs scripts a repository's `verifier.janus.yaml` names (ADR 0015). The hook context is the script's whole capability surface; memory, time and recursion limits are enforced, and that claim has a test |
| **X.3 Credentials** | S1.3, S1.9, S1.27 | Broker, OAuth2 and registry credentials come from the loader's resolved configuration, never the run's environment (finding 16's exception closed, S1.27), and the redaction rules cover every report they could reach |
| **X.4 Secrets in recorded artifacts** | S2.8 | Contracts, executed plans and hook reports |
| **X.5 Component sandboxing** | S5.1, S5.5 | The WASM grants (`fs`, `network`, `env`) enforced in process; out of process, `fs` and `network` are unenforceable (finding 19) and the specification says so to the user |
| **X.6 Component supply chain** | S5.1 | Signed artifacts, verified at pull and at load; the digest recorded in the run (finding 15) |
| **X.7 The agentic boundary** | SA.3 | As above |

### Cross-cutting: documentation

Each stage exit includes its user documentation on the Pact documentation site: stage 1a's is S1.13's
table and the hook migration notes; 1b's the per-language provider migration guides; stage 2's the
facade classification and the consumer migration guide; stage 3's the DSL reference; stage 5's the
component author's guide, which `third-party/janus-csv`'s report is the first draft of.

### Not staged

Named so their absence reads as a decision. The RFC's future possibilities: semantic contract diffing,
property-based provider fuzzing, a fully provider-driven workflow, a consumer-side runtime guard,
runtime drift observation, centrally executed verification, stateful interaction sequences, and a WASM
engine that runs tests through a host-provided transport (gate W is the other route). Each is a
proposal for after stage 6, or an ADR if evidence from a stage argues for it sooner.

## 4. Reassessment gates

Some decisions were left open on purpose. Each has a trigger, and something to do when it fires.
Several can change the stages above, which is why they are written down here rather than left in the
ADRs.

| Gate | Trigger | What to do | What it can change |
|---|---|---|---|
| **W: the WASM embedding** (ADR 0023) | `wasm32-wasip3` becomes a supported Rust target (it is being promoted to tier 2), and wasmtime plus one SDK-relevant host (jco for Node, or a JVM runtime) run p3 components | Re-run task 9.1's WASM scenarios, including a served mock and a driven verification. The exchange loop runs as an async task over `wasi:sockets`, with no thread; whether p3 brings threads does not matter if the loop needs none. Check `wasi:http`'s p3 server story too | If it works, WASM becomes a candidate primary embedding again, and S0.4's per-OS binary distribution becomes the fallback. S1.5 and S2.1 are async and thread-free behind an executor seam, so that passing this gate costs a build target, not a redesign |
| **F: plan fragments** (ADR 0022) | A second content component needs a fragment, or finding 21 (a fragment compiled without the variant, which widened a pinned variant) bites a real user | Adopt operator-level substitution: the engine compiles the slot, and a component says only what one value operator means | S5.4, stage 5's component interface |
| **E: frame encoding** (ADR 0006) | S0.3's trend shows base64 or frame size on a hot path | Negotiate CBOR, which the protocol already allows for | The protocol's second encoding |
| **C: contract packaging** (ADR 0011) | A realistic contract passes 5 MB, or base64 bodies pass 25% of its bytes | The exploded projection, or a container form | The contract file |
| **D: DSL defaults** (findings 2, 35) | S3.1 | Decide the empty-array default and exclusions before the DSL freezes | Stage 3 |
| **P: the subsumption policy** (ADR 0016) | Pilot teams routinely set `block`, or exemptions without an expiry dominate real policies | Revisit the defaults on that evidence | S4.4 |
| **B: the broker's default specification value** (ADR 0011 decision 6) | The broker PR series lands and deployed brokers catch up | Stop publishing Janus contracts as `specification: "pact"` | S2.7, S4.3 |

## 5. Known debt, by stage

Everything the prototype left open, assigned to the item that needs it. The detail is in the
[findings](prototype/phase-9-findings.md) and the [kernel-boundary review](prototype/kernel-boundary-review.md).

| Item | Debt | Source |
|---|---|---|
| S0.1 | Plan grammar v1: every action namespaced, the core families reserved (`flow`, `value`, `expect`, `check`, `match`, `legacy`), v0 fragments read by the one-to-one mapping. A rename across the interpreter, both compilers and every corpus snapshot, with no verdict changing. It must land before `explain` output reaches users | ADR 0026 |
| S0.2 | Windows and macOS CI for the subprocess, and for the whole workspace | spike 1.3's open risk; ADR 0023 |
| S0.3 | The benchmark harness as a CI trend, which plan §14 promised and 9.1 found had never run; gates E and C depend on it | performance report §6 |
| S0.4 | Per-OS distribution of `janus-engine`; `engine-wasm/` graduating into `engine/` | ADR 0023 decisions 2, 4 |
| S0.5 | The codename reaches every surface, with no inventory and no rehearsal of the rename; the persisted identifiers (`janus-contract/1`, `janus-provider-shape/1`, the OCI media types, the broker `specification` value) need a dual-spelling rule | ADR 0011, ADR 0021 |
| S0.13 | A shape operator spelled in a reserved namespace compiles to that core action; the interpreter indexes an action's children without checking there are enough, and fragment validation checks names only | found landing S0.1; plan grammar §4.5, §5.3; the errors-are-values rule |
| S1.3 | Per-pair verification counts in the run summary (`pairs`, option A); a publish verb; compression on the publish path | finding 10; broker notes §11 |
| S1.4 | v3/v4 generators are dropped by `upgrade` and never applied in verification | new in this revision; `engine/kernel/src/legacy/upgrade.rs` |
| S1.5 | No TLS, proxy or timeout options in the CLI or the HTTP component | new in this revision |
| S1.6 | Plain-text and binary bodies; split from the rest of the body kinds on 2026-10-21 | task 3.5; S1.1's first run |
| S1.7 | The `request`/`response` part names and the HTTP 500 mismatch reply in the kernel; the `produce-message`/`consume-message` hooks designed and not built | kernel-boundary review 8, 9; spike 1.5; design 2.7 |
| S1.8 | Plugin-bearing v4 interactions have no path until S5.2 | new in this revision |
| S1.10 | `match:content-type` runs with no detector in live exchanges and verification | third-party component report §4 |
| S1.11 | Nothing in the module tree marks the legacy compiler as the permanent HTTP-shaped exception | kernel-boundary review 3, 6 |
| S1.12 | Matching captured values as a protocol operation; `explain` loading components | findings 31, 23 |
| S1.14 | The SDK specification and conformance suite have no provider side | new in this revision |
| S1.17 | A filtered run that verified what it replayed — or replayed nothing, because no recorded variant has the id it named — has status `verified` and exits 0 | found rebuilding the CLI; variant-semantics spec §5.1 |
| S1.18 | A regex matches anywhere in a value under Janus and the whole value under today's verifiers, so an unanchored pattern in a v1–v4 pact accepts more than it does today | S1.1's first run; shape-language spec §4.2 |
| S1.19 | `upgrade` drops v1–v4 generators, so a contract replays a recorded value where its pact generated one; a shape's `generator` member is parsed and never applied | S1.4; ADR 0034 decision 6 |
| S1.20 | **Member order in the document model** (moved from stage 5); the legacy body compiler typed to `serde_json::Value` | split from S1.6; finding 11; kernel-boundary review 5 |
| S1.21 | Form-urlencoded bodies | split from S1.6 |
| S1.22 | XML bodies, and the 156 pact-specification cases skipped for them | split from S1.6; task 3.5 |
| S1.23 | Multipart bodies | split from S1.6 |
| S1.24 | `Values`, `EachKey`, `EachValue` and `ArrayContains` compile to an error node | split from S1.6; task 3.5 |
| S1.25 | `upgrade` carries a body that is not JSON across with nothing asserted about it | split from S1.6; ADR 0036 |
| S1.26 | A rule on the pact's `Content-Type` header passes a reply that today's body check fails, when its regex expects a space before a parameter | found landing S1.23; ADR 0036 decision 4 |
| S1.29 | The CLI's output, grown an item at a time: errors that are wrong, duplicated or carry raw `details`, summaries, help text, and exit 1 for a broker run that never started | collected through S1.1–S1.28; ADR 0041 |
| S1.30 | Relative and missing mismatch paths; an absent value printed as `NULL`; unnamed members' labels; "a integer" | S1.22, S1.24, S1.25; ADR 0042 |
| S1.36 | Neither SDK is released, so a pilot builds one from source, and an installed one finds no engine | stage 1b's exit; ADR 0028's tripwire |
| S1.37 | The engine's logs reach no JVM logging framework, and the pact-js facade's `logLevel` does nothing | found verifying `example-provider-springboot` |
| S2.1 | An async, thread-free exchange loop and HTTP server; the engine writes contract bytes | ADR 0023 decision 5; gate W; finding 6 |
| S2.2 | The consumer's test verdict and per-interaction results reaching the engine | findings 4, 5 |
| S2.3 | Arming a variant recompiles its plan: cache per (handle, variant) | finding 28 |
| S2.5 | The compatibility facade is an unbudgeted hand-written layer | thinness audit |
| S2.6 | The conformance suite does not cover test-framework integrations, the facade or per-language refusals | conformance report §7.2 |
| S2.7 | The rename itself, whenever §7.1 answers; contracts and components already published under the codename must stay readable | ADR 0011; §7.1 |
| S2.10 | A Janus contract's XML text is trimmed as today's verifier trims a pact's, CDATA sections included | found in review of S1.22; ADR 0039 decision 2 |
| S3.3 | `http:media-type`, and list-valued headers | finding 1 |
| S3.1 | The empty-array default; exclusions in the DSL; correlated fields | findings 2, 35 |
| S3.5 | The exchange loop's match decision is a log line, not an event | variant ergonomics report, recommendation 3 |
| S4.4 | A `provenance` selector on exemptions | finding 9B |
| S4.1, S4.2 | Provider-shape recording in provider test frameworks; importers that emit selectors | tasks 7.2, 7.3; ADR 0025 |
| S4.5, S4.6 | Whether N×M recomputation scales; message-only providers in the broker view | broker notes §12 |
| S5.3 | `encode` sees the shape; decode errors and degradations reach the user | findings 12–14 |
| S5.1 | Recording the components that took part; registry credentials; `wkg` artifacts; signing | findings 15–17 |
| S5.5 | Out-of-process containment and command form | findings 18, 20 |
| S5.6 | A multipart part's body reaches only the built-in content components | ADR 0040 decision 3 |
| S5.4 | Fragments: the variant, decoding in the plan, action configuration | findings 21, 22, 24; gate F |
| SA.3 | The security review an agent with provider access needs | AI-layer notes §4 |

## 6. Risks

| Risk | Where it bites | Mitigation |
|---|---|---|
| **One pipeline carries everything** until S2.9: Rust, Node, the JDK, WASM, a registry and the demo, under one tag namespace | Stages 0–2 | ADR 0027: the SDKs leave at stage 2's exit, and S0.10–S0.11 build the pieces that move needs before it is urgent |
| **Volunteer capacity.** This is a multi-year build for a volunteer-driven project | Every stage | Providers-first ordering puts value in users' hands at stage 1a and stops depending on later stages. The specifications and conformance suite let contributors work in parallel without drifting apart |
| **Stage 1 is the biggest stage**, not the L the first revision sized it. "Exactly as today" covers generators, TLS, XML and form bodies, message pacts, a broker client and in-test state handlers, none of which the prototype has | Stage 1 | Split into 1a, which ships a CLI verifier with value on its own, and 1b. S1.1's harness turns "exactly" into a number from the first week, so scope is discovered early rather than at the pilot |
| **The Osborne effect.** Announcing MkII stalls today's Pact before MkII is ready | Stages 1–3 | Stage 1 improves life for today's users with no migration at all. The compatibility facade keeps the consumer migration mechanical |
| **Native binaries.** Per-OS binaries were the pain of pact-ruby-standalone | Stages 0, 2 | The per-run, EOF-exit lifecycle (spike 1.3); distribution through each ecosystem's normal package channel; gate W as the way out |
| **Specification governance.** Specs nobody can change become a bottleneck, and specs anyone can change stop meaning anything | From stage 0 | The rule Janus followed: every change lands in the spec, the corpus or the conformance suite in the same change, and ADRs record the contested ones. Who approves them is a community question |
| **The name comes late.** Users will not weigh in until they can run their pipelines, so the codename reaches brokers and registries before the rename, and a rename then touches them | Stages 0–3 | S0.5 inventories every surface and rehearses the rename in CI; the persisted identifiers are read under both spellings for one major version, so nothing is republished |
| **Maintainer identity.** pact-jvm ceasing to be an independent implementation | Stage 2 | The JVM SDK is a real product with its own maintainers and idiomatic layer; the JVM report (task 6.3) shows what they would own |
| **Findings the prototype could not produce.** No real team used Janus, so several tripwires (variant budgets, exclusions, subsumption noise at scale) never had data to fire on | Stages 3–4 | Pilot teams at each stage exit, and the tripwires in §4 watched deliberately |

## 7. Questions for the community

The charter says these belong to the community, and this plan does not decide them. What it can do is
say why each one matters, what the prototype found that bears on it, and which items wait on it.

1. **Naming and versioning.** Janus writes its own contract format and does not claim to be "pact
   v5" (ADR 0011). Some of the options:
   - make that format the Pact specification's next version;
   - keep it a separately versioned format under the Pact name;
   - brand the whole redesign (the RFC's "MkII", "Pact 6").

   The format's identifier is written into every contract, so the first revision wanted this decided
   before stage 2 ships a major version. This revision expects it later: people will not have an
   opinion until they can run their pipelines on the real thing (§3). So stage 2 may ship under the
   codename, and S0.5 makes the rename, whenever it comes, mechanical and backward-readable. Waits on
   it: S0.9, S2.7.
2. **Where it lives, and who owns what.** The engine, the specifications, and conformance sign-off each
   need an owner. The prototype suggests sign-off can be mechanical (a build passes the suite, ADR 0017),
   which leaves people deciding what goes into the specification rather than whether an SDK complies.
   The engine could live beside `pact_matching` in pact-reference, which it forked, or in a repository
   of its own. Waits on it: S0.7, S0.8, and the same timing applies: the work happens here (S0.6) until
   there is something to move.
3. **Funding.** Stages 1 and 2 are large, and providers-first means stage 1a pays back before stage 2
   finishes. Whether that is foundation sponsorship, vendor contribution or grants is not this plan's to
   say.
4. **When today's implementations go into maintenance mode.** The RFC ties it to each SDK passing
   conformance on the new engine. This plan's stage exits make that measurable per language, on both
   sides. The timeline around them is the community's. Waits on it: S6.4.
5. **How the broker's API evolves.** The broker notes (task 7.5) set out a PR series for provider shapes
   and decisions. How far the open-source broker and PactFlow each go is theirs to decide. Waits on it:
   S4.3, gate B.

## 8. Closing the prototype

The charter defines "prototype complete" as follows. Each condition is now met.

| Condition | State |
|---|---|
| Phases 0–9 done or explicitly descoped by ADR | Done: this task closes Phase 9 |
| Every success criterion has a demo or a written finding | All seven met ([RFC feedback](prototype/rfc-feedback.md) §6) |
| Every unresolved-questions row has an answer or an honest "still open" | Yes ([RFC feedback](prototype/rfc-feedback.md) §2; plan §13) |
| The report and the staged plan are published for community review | The [community report](prototype/prototype-report.md), this plan, and the revised RFC on [PR #146](https://github.com/pact-foundation/roadmap/pull/146) |

What happens to this repository was itself a question for the community. ADR 0027 answers it: the real
build proceeds here, hardening the prototype in place, with §2 as the inventory of what that means and
the SDKs graduating into their existing repositories at stage 2's exit. §7.2 can still move the
permanent part; the history comes along.

## Revision

**2026-10-11.** The first revision was a proposal; this one is the plan the repository works from. A
review against the Phase 9 findings, the kernel-boundary review, the RFC, the ADR backlog, the broker
notes and the code found three structural problems and a list of gaps, all folded in:

- **Every item has an id, a size and an exit** (principle 6), so progress can be recorded against it.
- **The repository layout is decided** (ADR 0027, same day): build here, the executable
  specification stays together, the SDKs are incubated and graduate into pact-js and pact-jvm at
  stage 2's exit (S2.9), and S0.10–S0.12 make the split cheap now.
- **Stage 0 is split** into what can start now (0a) and what waits on §7 (0b). Two decisions the first
  revision left to "either works" are now stated as assumptions: the build happens here (S0.6), and the
  codename stays, with the rename made mechanical and rehearsed rather than forbidden (S0.5). The
  community's answers are not expected until people can run their pipelines on the real thing, so
  nothing before stage 2 blocks on them and stage 2 may ship under the codename (S2.7). The
  plan-grammar v1 rename moved
  from stage 1 to the first item of stage 0a, since the first revision said both that stage 0 publishes
  the grammar as v1 and that stage 1 renames it.
- **Stage 1 is split and resized** from L to XL, as 1a (the CLI verifier) and 1b (provider-side
  integrations). Items that a drop-in verifier needs and the first revision lacked: a differential
  harness and fixture corpus (S1.1), the broker client's home (S1.2), v3/v4 generators (S1.4), TLS,
  proxies and timeouts (S1.5), multipart and binary bodies (S1.6), flag parity (S1.13), the provider
  side of the SDK specification and the JVM and JS provider integrations (S1.14, S1.15). Items moved
  into stage 1 from later stages because stage 1 depends on them: member order (finding 11, from
  stage 5) and the legacy body compiler's retyping (kernel-boundary review 5) for XML bodies; the
  message hooks (from stage 6) for message pact verification; the HTTP component's client half (from
  stage 2). Items that stage 1 cannot do and now says so: plugin-backed v4 interactions (S1.8, closed
  by S5.2). Two gaps in neither the findings list nor the first revision: `match:content-type` runs
  with no detector in live runs (S1.10), and the legacy compiler's boundary (S1.11).
- **Added:** the benchmark trend as an item (S0.3; gates E and C could never fire without it); the
  conformance suite's own gaps and the facade's thinness budget (S2.5, S2.6); `upgrade` output as
  the rename as a non-blocking item (S2.7); secrets in recorded contracts (S2.8); the exchange
  loop's match decision as an event (S3.5); the broker's offline-WASM host and the message-only
  provider view (S4.5, S4.6); component signing (S5.1); the protocol-level test client in §2; the
  AI-assisted layer as an optional stage; a cross-cutting security table; documentation per stage exit;
  and a "not staged" list for the RFC's future possibilities.
