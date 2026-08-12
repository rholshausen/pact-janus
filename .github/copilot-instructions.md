# Copilot Instructions

Pact Janus prototypes a ground-up redesign of the [Pact](https://pact.io) contract testing framework per
the [Pact MkII RFC](https://github.com/pact-foundation/roadmap/pull/146): one Rust reference engine
behind a coarse-grained versioned protocol, thin language SDKs, interaction specs compiled to
inspectable matching plans, a shape language with variant testing, and plugins as first-class components.

Source of truth: `Documentation/project-plan.md` (the phased plan; task numbers like "1.6"/"G1" refer to
it) and `Documentation/decisions/` (ADRs — don't re-litigate accepted ones in code; supersede them).
Keep this file in sync with `CLAUDE.md`.

> **Status**: plan/design phase. Layout and commands below are the intended structure from plan task
> 0.3; trust the repo over this file as scaffolding lands, and update this file when it does.

## Repository layout

```
engine/          Rust workspace members: kernel + built-in component crates
cli/             `pact` CLI (verify, explain, upgrade, check)
sdks/typescript/ TypeScript SDK prototype (DSL + Jest/Vitest integration)
sdks/jvm/        JVM SDK prototype (DSL + JUnit 5 integration)
corpora/         Golden corpora: (spec or pact, expected plan, expected result)
spikes/          Time-boxed experiments — disposable code, durable findings
Documentation/   Plan, ADRs, specs
```

## Build, test, and lint commands

Rust — the Cargo workspace root is the repo root:

```bash
cargo build                                  # build the workspace
cargo test                                   # all tests
cargo test --package <crate>                 # one crate
cargo test --package <crate> -- name --exact # one test
RUST_LOG=debug cargo test -- --nocapture     # with tracing output
cargo clippy                                 # lint (must be clean)
cargo fmt --all -- --check                   # formatting
cargo build --target wasm32-wasip2           # WASM component build (engine crates)
```

TypeScript — from `sdks/typescript/`:

```bash
npm ci               # install (npm is the package manager)
npm run build        # compile
npm test             # Vitest
npm run lint         # ESLint
```

JVM — from `sdks/jvm/`: `./gradlew build test`.

## Architecture rules

- The kernel knows nothing about HTTP or JSON: transports, content handlers, matchers/generators and
  hooks — including built-ins — are components behind the published component interfaces. Flag any
  change that leaks protocol/content knowledge into the kernel.
- Errors are values at the engine boundary; panics never cross the protocol. Sessions are the only
  resource (no per-object cleanup calls).
- SDKs are thin: no matching logic or orchestration beyond the protocol operations. Generated protocol
  bindings are never hand-edited — regenerate them.
- Corpora are load-bearing: any matching-behaviour change must change `corpora/` in the same commit.
- Spike code under `spikes/` is disposable and exempt from the conventions below, but every spike
  directory must contain a `FINDINGS.md`. Never depend on spike code from `engine/`, `cli/` or `sdks/`.

## Conventions

Rust:
- Edition 2024, latest stable toolchain, `cargo clippy` clean.
- `tracing`/`tracing-subscriber` for logging (not `log`); `tokio` for async (not on WASM paths).
- Test helpers: `rstest`, `expectest`, `pretty_assertions` — prefer these to new utilities.
- Crates shipped as WASM components must build for `wasm32-wasip2`; keep native-only dependencies out
  of them or behind target-gated features.

TypeScript / JavaScript:
- TypeScript with `strict: true` for all SDK/tooling source; no `any` in exported API surfaces. Plain
  JavaScript only in spikes or generated output.
- ESM modules, active Node LTS, Vitest for tests, ESLint + Prettier for lint/format, lean
  devDependencies.
- The public DSL mirrors the RFC's consumer example (`optional`, `anyOf`, `oneOf`, `eachLike`,
  `pact.execute(...)`); change the SDK specification first, then the code.

Commits follow Conventional Changelog style (`feat:`, `fix:`, `chore:`, `docs:`, `refactor:`, `test:`).

## Reference material

`pact-reference` (usually a sibling checkout) provides `pact_models` (dependency) and the v2 matching
engine prototype (`rust/pact_matching/src/engine`, forked as the kernel starting point). Read for
context; record provenance in the commit message when adapting code from it.
