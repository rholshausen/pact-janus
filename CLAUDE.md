# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.
Keep it in sync with `.github/copilot-instructions.md`.

## Overview

Pact Janus is a prototype of a ground-up redesign of the [Pact](https://pact.io) contract testing
framework, based on the [Pact MkII RFC](https://github.com/pact-foundation/roadmap/pull/146): a single
Rust reference engine behind a coarse-grained versioned protocol, thin language SDKs, declarative
interaction specs compiled to inspectable matching plans, a shape language with variant testing, and
plugins as first-class components.

Two documents are the source of truth and take precedence over this file when they conflict:

- `Documentation/project-plan.md` — the phased plan; task numbers like "1.6" or "G1" refer to it.
- `Documentation/decisions/` — ADRs. Every gate and contested design choice lands here. Do not
  re-litigate an accepted ADR in code; propose a superseding ADR instead.

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
