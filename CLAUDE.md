# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.
Keep it in sync with `.github/copilot-instructions.md`.

## Overview

Pact Janus is a prototype of a ground-up redesign of the [Pact](https://pact.io) contract testing
framework, based on the [Pact MkII RFC](https://github.com/pact-foundation/roadmap/pull/146): a single
Rust reference engine behind a coarse-grained versioned protocol, thin language SDKs, declarative
interaction specs compiled to inspectable matching plans, a shape language with variant testing, and
plugins as first-class components.

Three sources are the source of truth and take precedence over this file when they conflict:

- `Documentation/project-plan.md` — the phased plan; task numbers like "1.6" or "G1" refer to it.
- `Documentation/decisions/` — ADRs. Every gate and contested design choice lands here. Do not
  re-litigate an accepted ADR in code; propose a superseding ADR instead.
- `Documentation/specs/` — the Phase 2 design specifications. Normative for the code that
  implements them; the schemas they ship are the specified surface, not documentation of it.
  `engine-protocol/` (task 2.1) governs every frame crossing the engine boundary;
  `shape-language/` (task 2.2) governs shapes — what they admit, how they compose, and the
  variant dimensions they contribute; `variant-semantics/` (task 2.3) governs variants — which
  ones get selected, how they are named and recorded, and how they drive provider state;
  `plan-grammar/` (task 2.4) governs plans — the node grammar, the action set, the text forms
  `explain` prints, and the golden-corpus format; `contract-file/` (task 2.5) governs the Janus
  contract — the recorded artifact's identity, its interaction record, how shapes and exercised
  variants are written down, and v1–v4 pact conversion. It is **not** a "pact v5" (ADR 0011);
  `component-interfaces/` (task 2.6) governs the four interfaces the kernel loads behind it —
  transport, content, matcher/generator, hook — the pipe they speak, what a component contributes,
  the three bindings, and how out-of-tree components are distributed, sandboxed and resolved;
  `lifecycle-hooks/` (task 2.7) governs the hook *system* the hook interface sits inside — the point
  vocabulary and each point's context and mutable set, ordering and failure semantics, the
  configuration document the loader resolves before the engine sees it (ADR 0014), the four
  implementations, and the scripted-hook runtime and its API (ADR 0015);
  `subsumption-check/` (task 2.8) governs the `admits(provider) ⊆ admits(consumer)` walk — the
  composition rules on top of the shape language's own comparability classes, the provider-shape
  artifact, the finding vocabulary and report format, and the warn/block policy with exemption
  scoping (ADR 0016); `sdk-specification/` (task 2.9) governs what a Janus SDK is — the canonical,
  language-independent behavioural spec every idiomatic layer implements, the per-language
  style-guide skeleton, the compatibility-facade classification for today's DSL, and what
  "conformant" means (ADR 0017).

> **Status**: this repo is in the plan/design phase. The layout and commands below describe the intended
> structure from the project plan (task 0.3). Update this file as scaffolding actually lands, and trust
> the repo over this file if they diverge.

## Intended repository layout

```
engine/          Rust workspace members: kernel, built-in component crates, hooks-host —
                 the host side of hooks (loader, exec and http implementations), which lives
                 outside the kernel because it needs a filesystem, a process and a socket — and
                 component-host, the wasmtime loader for out-of-tree WASM components (task 8.1),
                 outside the kernel because a WASM guest cannot host WASM (ADR 0013)
cli/             `janus` CLI (verify, explain, upgrade, check)
sdks/typescript/ TypeScript SDK prototype (DSL + Jest/Vitest integration); src/generated/ holds
                 its protocol bindings (task 6.1)
sdks/jvm/        JVM SDK prototype (DSL + JUnit 5 integration); the Gradle `bindings` project
                 holds its protocol bindings (task 6.1)
sdks/bindings.json  Which spec schema sets the SDKs' bindings are generated from
corpora/         Golden corpora: (spec or pact, expected plan, expected result)
conformance/     The SDK conformance suite (task 6.4): one corpus of cases every SDK runs, in its
                 own language, against a pinned engine — ADR 0017's definition of "conformant".
                 Each SDK's driver lives with its tests; `tools/conformance` checks the corpus
                 and the reports a run writes
samples/         Demo subjects (workspace members), e.g. samples/order-service — the sample
                 provider (task 5.6) with deliberate variance, auth and a v3 provider-state
                 endpoint, used by verification tests, M3 and M5. Its pacts/ holds the v3
                 pact of a consumer that has not upgraded, which task 5.4 verifies against
                 it unchanged, and shapes/ the provider shape its own tests recorded
                 (task 7.2) — the other half of M5, which `janus check` reads
third-party/     Components written as a third party would, from the published interfaces
                 only — each its own Cargo workspace, excluded from ours and depending on
                 nothing in it: third-party/janus-csv, the text/csv content component task 8.1
                 built from the docs (Documentation/third-party-component-report.md)
spikes/          Time-boxed experiments — disposable code, durable findings
benchmarks/      Baseline/trend benchmark harness (task 1.7) — durable, standalone crate
                 (excluded from the workspace; run with `cd benchmarks && cargo run --release`)
tools/           Repo tooling (workspace members), e.g. tools/schema-compat — the CI checker
                 for the open-world rules every schema under Documentation/specs/ follows,
                 and for the specs' worked examples — tools/bindings, the binding-generation
                 pipeline (task 6.1), tools/conformance, the conformance suite's checker
                 (task 6.4), and tools/thinness, the SDK thinness audit (task 6.5) — it reads
                 sdks/thinness.json, counts each SDK by layer and budgets the hand-written ones
Documentation/   Plan, ADRs, specs
```

## Build & test commands

Rust (workspace root at the repo root). One non-Rust build requirement: **libclang**, which
`rquickjs`'s bindgen build needs — the kernel embeds QuickJS as the scripted-hook runtime
(ADR 0015), including in the `wasm32-wasip2` build:

```bash
cargo build                                  # build the workspace
cargo test                                   # run all tests
cargo test --package <crate>                 # one crate
cargo test --package <crate> -- name --exact # one test
RUST_LOG=debug cargo test -- --nocapture     # with tracing output
cargo clippy                                 # lint (must be clean)
cargo fmt --all -- --check                   # formatting
cargo build --target wasm32-wasip2           # WASM component build (engine crates)
```

TypeScript (from `sdks/typescript/`):

```bash
npm ci               # install (npm is the package manager)
npm run build        # compile
npm test             # run tests (Vitest)
npm run lint         # ESLint
```

`npm test` drives a real `janus-engine`: its Vitest global setup builds one with `cargo build` unless
`JANUS_ENGINE` already names the executable. The SDK's DSL is the canonical behavioural specification,
`Documentation/specs/sdk-specification/behavioural-spec.json` — change it there first; `STYLE.md`
records how the TypeScript spelling expresses it.

JVM (from `sdks/jvm/`, JDK 17): `./gradlew build test`.

The SDK conformance suite (plan task 6.4, ADR 0017) is one corpus under `conformance/` that **both**
SDKs run as part of the test commands above, each writing a report to `target/conformance/`. The
corpus is checked in; the reports are build artifacts:

```bash
cargo test -p pact_janus_conformance          # the corpus against its schema and the spec's ids
cargo run -p pact_janus_conformance -- lint   # the same, with a coverage summary
cargo run -p pact_janus_conformance -- check target/conformance/*.json   # did each SDK pass it
```

A conformance id in `behavioural-spec.json` with no case fails the lint, and a case that fails in
either language fails CI. When a case and an SDK disagree and the specification does not settle it,
change `behavioural-spec.json` first — `conformance/README.md` §7.

The thinness audit (plan task 6.5) is the other claim made into a command: every SDK source file
must be classified into a layer in `sdks/thinness.json`, and the hand-written layers stay inside a
budget, so matching logic or orchestration creeping back into an SDK fails the build:

```bash
cargo run -p pact_janus_thinness              # the table, by layer, per SDK
cargo run -p pact_janus_thinness -- check     # the same, as an exit code (CI runs this)
```

A new SDK source file with no layer is an error, not a default — say which layer it belongs to.
Raising a budget is a decision that belongs in the commit message.

Generated protocol bindings (plan task 6.1) — both SDKs' typed views of the spec schemas
`sdks/bindings.json` names, checked in and **never hand-edited**. Regenerate after any change to
those schemas (needs Node, with `npm ci` done in `sdks/typescript/`, and JDK 17); CI regenerates
and fails on any diff:

```bash
cargo run -p pact_janus_bindings -- generate                    # both SDKs
cargo run -p pact_janus_bindings -- generate --only typescript  # or --only jvm
```

The `janus` CLI (`cli/src/main.rs`, plan task 5.5) drives the engine **through the protocol** — it
builds frames and reads frames back, never the kernel's Rust API, so anything a command cannot do
is a finding about the protocol first:

```bash
janus verify <contract-or-pact>... --provider-url <url> [--config verifier.janus.yaml]
             [--variant <id>] [--explain-failures] [--json]
janus check <contract-or-pact>... [--provider-shape <path>]... [--verification <file>]...
            [--policy <file>] [--on-finding warn|block] [--on-review warn|block]
            [--as-of <YYYY-MM-DD>] [--json]
janus explain <document.json> [--index N] [--variant <id>] [--spec] [--plan] [--executed <values.json>]
janus upgrade <pact.json> [--out <file>] [--json] [--quiet]
```

Exit codes are part of the surface: `0` the command did what it was asked, `1` the *subject* failed
(a verification found mismatches — an answer, not an error), `2` the command could not run.
`check` (plan task 7.4) decides rather than tests: it reads documents that already exist — the
consumer contracts or v1–v4 pacts, the shapes a provider published, and the summaries
`janus verify --json` wrote — and answers `can-i-deploy` over them, so it needs no provider
running. Its policy (design 2.8 §7) resolves in layers: the specification's defaults, then
`--policy`'s document, then `--on-finding`/`--on-review`. `block` is exit 1 — the subject failed,
not the command. Both severities default to `warn` (ADR 0016).

Out-of-tree components (plan task 8.1, design 2.6 §10) are **declared**, never discovered: a
`components` list in the project configuration (`verifier.janus.yaml` for `janus verify --config`;
the SDKs' `components` option on the consumer side), each entry naming its source — `file` is a
local `.wasm` implementing `Documentation/specs/component-interfaces/wit/component.wit`. `janus`
and `janus-engine` both register the WASM loader (`engine/component-host`), and `engine/hello` says
so (`components.loaders`). A declared component that cannot be loaded, or an interaction requiring
one nobody declared, fails before anything runs, as `component-unavailable` naming it. The worked
third-party component is `third-party/janus-csv`: its own workspace, built with
`cargo build --release --target wasm32-wasip2` from its directory, and built again by the tests
that load it.

`janus-engine` (the subprocess embedding, `cli/src/bin/janus_engine.rs` — ADR 0003): built by the
normal `cargo build`/`cargo test` above (`cargo build -p pact_janus_cli --bin janus-engine`
targets it alone). Its own protocol-level Node test client — plan task 4.5's "thin test client
speaks the protocol directly," not an SDK, and deliberately outside `sdks/` — lives at
`cli/tests/janus-engine-node/`: `npm install` once, then `npm test` (rebuilds the binary itself
via `cargo build` in a `beforeAll`, so it never runs against a stale one). It installs the
`tracing-subscriber` the kernel's own `tracing` facade needs (CLAUDE.md's Rust conventions below);
`RUST_LOG=trace janus-engine` logs every frame `Engine::dispatch` sees in both directions to
stderr (stdout stays frames-only).

## Architecture rules

- **The kernel knows nothing about HTTP or JSON.** Transports, content handlers, matchers/generators and
  hooks are components behind the interfaces in the component-interface spec — including the built-in
  ones. If a change leaks protocol- or content-specific knowledge into the kernel, stop and flag it
  (plan task 3.8 tracks exactly this).
- **Errors are values at the engine boundary.** Panics must never cross the protocol; every operation
  returns structured errors. Sessions are the only resource — no per-object cleanup calls.
- **SDKs are thin.** No matching logic, no orchestration beyond the protocol operations, in any SDK.
  Generated protocol bindings are never hand-edited — regenerate them via the binding pipeline.
- **Corpora are load-bearing.** Any change to matching behaviour must change `corpora/` in the same
  commit. CI runs plans against the corpora; a behaviour change without a corpus change is a bug.
  The same holds one layer up: a change to what an SDK's DSL does belongs in the behavioural
  specification and `conformance/` — every SDK runs that corpus, and "conformant" means passing it
  (ADR 0017).
- **Spikes are disposable, findings are not.** Code under `spikes/` may rot and is not held to the
  standards below, but every spike directory must contain a `FINDINGS.md`. Never depend on spike code
  from `engine/`, `cli/` or `sdks/`.

## Rust conventions

- Rust edition 2024, latest stable toolchain; `cargo clippy` clean.
- Logging via `tracing` / `tracing-subscriber` (not `log`); async via `tokio` (not on WASM paths).
- Test helpers: `rstest`, `expectest`, `pretty_assertions` — follow these before introducing new ones.
- Engine crates that ship as WASM components must build for `wasm32-wasip2`; keep native-only
  dependencies (`tokio`, `reqwest`, raw sockets) out of those crates or behind target-gated features.

## TypeScript / JavaScript conventions

- TypeScript everywhere in SDK and tooling source; `strict: true`, no `any` in exported API surfaces.
  Plain JavaScript only in throwaway spike code or generated output.
- ESM modules; target the active Node LTS.
- Tests with Vitest; lint with ESLint + Prettier defaults. Keep devDependencies lean — this is a
  prototype of a *thin* SDK, and every dependency is part of the story it tells.
- The public DSL surface follows the RFC's consumer example (`optional`, `anyOf`, `oneOf`, `eachLike`,
  `janus.execute(interaction, async (mock, variant) => …)`); changes to it belong in the SDK
  specification first, not directly in code.

## Commit messages

Conventional Changelog format: `feat:`, `fix:`, `chore:`, `docs:`, `refactor:`, `test:`.

## Reference material

- The RFC: pact-foundation/roadmap PR #146 (local checkout usually at `../roadmap/rfc/0000-pact-mkii.md`).
- `pact-reference` (usually checked out as a sibling directory) supplies `pact_models` (v1–v4 pact file
  model, a dependency) and the v2 matching engine prototype in `rust/pact_matching/src/engine` (forked as
  the kernel starting point — see the reuse-inventory report and ADRs). Read it for context; do not copy
  code from it without recording provenance in the commit message.
