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
engine/          Rust workspace members: kernel + built-in component crates
cli/             `pact` CLI (verify, explain, upgrade, check)
sdks/typescript/ TypeScript SDK prototype (DSL + Jest/Vitest integration)
sdks/jvm/        JVM SDK prototype (DSL + JUnit 5 integration)
corpora/         Golden corpora: (spec or pact, expected plan, expected result)
spikes/          Time-boxed experiments — disposable code, durable findings
benchmarks/      Baseline/trend benchmark harness (task 1.7) — durable, standalone crate
                 (excluded from the workspace; run with `cd benchmarks && cargo run --release`)
tools/           Repo tooling (workspace members), e.g. tools/schema-compat — the CI checker
                 for the open-world rules every schema under Documentation/specs/ follows,
                 and for the specs' worked examples
Documentation/   Plan, ADRs, specs
```

## Build & test commands

Rust (workspace root at the repo root):

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

JVM (from `sdks/jvm/`): `./gradlew build test`.

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
  `pact.execute(interaction, async (mock, variant) => …)`); changes to it belong in the SDK
  specification first, not directly in code.

## Commit messages

Conventional Changelog format: `feat:`, `fix:`, `chore:`, `docs:`, `refactor:`, `test:`.

## Reference material

- The RFC: pact-foundation/roadmap PR #146 (local checkout usually at `../roadmap/rfc/0000-pact-mkii.md`).
- `pact-reference` (usually checked out as a sibling directory) supplies `pact_models` (v1–v4 pact file
  model, a dependency) and the v2 matching engine prototype in `rust/pact_matching/src/engine` (forked as
  the kernel starting point — see the reuse-inventory report and ADRs). Read it for context; do not copy
  code from it without recording provenance in the commit message.
