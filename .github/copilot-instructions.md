# Copilot Instructions

Pact Janus prototypes a ground-up redesign of the [Pact](https://pact.io) contract testing framework per
the [Pact MkII RFC](https://github.com/pact-foundation/roadmap/pull/146): one Rust reference engine
behind a coarse-grained versioned protocol, thin language SDKs, interaction specs compiled to
inspectable matching plans, a shape language with variant testing, and plugins as first-class components.

Source of truth: `Documentation/project-plan.md` (the phased plan; task numbers like "1.6"/"G1" refer to
it), `Documentation/decisions/` (ADRs — don't re-litigate accepted ones in code; supersede them), and
`Documentation/specs/` (Phase 2 design specs, normative for the code implementing them — the schemas
they ship are the specified surface, not documentation of it; `engine-protocol/` from task 2.1 governs
every frame crossing the engine boundary, `shape-language/` from task 2.2 governs shapes — what they
admit, how they compose, and the variant dimensions they contribute — and `variant-semantics/` from
task 2.3 governs variants — selection, naming, recording, and variant-bound provider state — and
`plan-grammar/` from task 2.4 governs plans: the node grammar, the action set, the text forms
`explain` prints, and the golden-corpus format, and `contract-file/` from task 2.5 governs the Janus
contract — the recorded artifact's identity, its interaction record, how shapes and exercised variants
are written down, and v1–v4 pact conversion; it is *not* a "pact v5", see ADR 0011 — and
`component-interfaces/` from task 2.6 governs the four interfaces the kernel loads behind it: the pipe
components speak, what they contribute, the native/WASM/subprocess bindings, and how out-of-tree components
are distributed, sandboxed and resolved, and `lifecycle-hooks/` from task 2.7 governs the hook system
around that interface — the points and what each is handed, ordering and failure semantics, the
configuration a loader resolves before the engine sees it (ADR 0014), the four implementations, and the
scripted-hook runtime (ADR 0015), and `subsumption-check/` from task 2.8 governs the
`admits(provider) ⊆ admits(consumer)` walk — the composition rules on top of the shape language's
comparability classes, the provider-shape artifact, the finding vocabulary and report format, and the
warn/block policy with exemption scoping (ADR 0016), and `sdk-specification/` from task 2.9 governs
what a Janus SDK is — the canonical, language-independent behavioural spec every idiomatic layer
implements, the per-language style-guide skeleton, the compatibility-facade classification for
today's DSL, and what "conformant" means (ADR 0017)). Keep this file in sync with
`CLAUDE.md`.

> **Status**: plan/design phase. Layout and commands below are the intended structure from plan task
> 0.3; trust the repo over this file as scaffolding lands, and update this file when it does.

## Repository layout

```
engine/          Rust workspace members: kernel, built-in component crates, hooks-host —
                 the host side of hooks (loader, exec and http implementations), which lives
                 outside the kernel because it needs a filesystem, a process and a socket — and
                 component-host, the wasmtime loader for out-of-tree WASM components (task 8.1),
                 outside the kernel because a WASM guest cannot host WASM (ADR 0013)
cli/             `pact` CLI (verify, explain, upgrade, check)
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
                 pipeline (task 6.1), tools/conformance, the conformance suite's checker, and
                 tools/thinness, the SDK thinness audit (task 6.5) — it counts each SDK by layer
                 (task 6.4)
Documentation/   Plan, ADRs, specs
```

## Build, test, and lint commands

Rust — the Cargo workspace root is the repo root. One non-Rust build requirement: **libclang**,
which `rquickjs`'s bindgen build needs — the kernel embeds QuickJS as the scripted-hook runtime
(ADR 0015), including in the `wasm32-wasip2` build:

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

`npm test` drives a real `janus-engine`: its Vitest global setup builds one with `cargo build` unless
`JANUS_ENGINE` already names the executable. The SDK's DSL is the canonical behavioural specification,
`Documentation/specs/sdk-specification/behavioural-spec.json` — change it there first; `STYLE.md`
records how the TypeScript spelling expresses it.

JVM — from `sdks/jvm/` (JDK 17): `./gradlew build test`.

The SDK conformance suite (plan task 6.4, ADR 0017) is one corpus under `conformance/` that both
SDKs run as part of those test commands, each writing a report to `target/conformance/`:
`cargo run -p pact_janus_conformance -- lint` checks the corpus (a conformance id in
`behavioural-spec.json` with no case fails), and `-- check target/conformance/*.json` checks that
each SDK accounted for every case and passed it. See `conformance/README.md`.

The thinness audit (plan task 6.5) makes the "SDKs are thin" claim a command:
`cargo run -p pact_janus_thinness` prints each SDK's lines by layer, and `-- check` (which CI runs)
fails if a hand-written layer exceeds its budget in `sdks/thinness.json`, or if any SDK source file
belongs to no layer. A new file needs a layer; raising a budget belongs in the commit message.

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
janus component push <component.wasm> <reference> [--json]
janus component pull <reference> [--digest <sha256:...>] [--json]
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
local `.wasm` implementing `Documentation/specs/component-interfaces/wit/component.wit`, `oci` a
registry reference (plan task 8.2, ADR 0021) with a `digest` to pin it, `subprocess` a command
spawned over the protocol's stdio framing — loaded only by `spikes/8.3-subprocess-transport`'s
loader, which no shipped embedding registers yet. A declared component may contribute a transport,
looked up by the `kind`s its handshake names. `janus`
and `janus-engine` both register the WASM loader (`engine/component-host`), and `engine/hello` says
so (`components.loaders`). A declared component that cannot be loaded, or an interaction requiring
one nobody declared, fails before anything runs, as `component-unavailable` naming it. The worked
third-party component is `third-party/janus-csv`: its own workspace, built with
`cargo build --release --target wasm32-wasip2` from its directory, and built again by the tests
that load it.

`janus component push|pull` (plan task 8.2) is the one command that does not speak the protocol —
publishing is not an engine operation. `push` writes the artifact's config from the component's own
handshake; `pull` fetches into the cache, checks every byte, and prints the pinned declaration. A pin
is fetched by digest alone and a second run fetches nothing. Loopback registries are plain HTTP;
credentials are `JANUS_OCI_USERNAME`/`JANUS_OCI_PASSWORD`; the cache is `JANUS_COMPONENT_CACHE`
(default: the user cache directory's `pact-janus/components`). The OCI tests run against an in-process
registry that tampers on request, and against a real one when `JANUS_OCI_REGISTRY` names it
(`docker run -d -p 5000:5000 registry:2`, then `JANUS_OCI_REGISTRY=localhost:5000`; CI does this).

`janus-engine` (the subprocess embedding, `cli/src/bin/janus_engine.rs` — ADR 0003): built by the
Rust commands above (`cargo build -p pact_janus_cli --bin janus-engine` targets it alone). Its
protocol-level Node test client — plan task 4.5's "thin test client speaks the protocol directly,"
not an SDK, and deliberately outside `sdks/` — lives at `cli/tests/janus-engine-node/`:
`npm install` once, then `npm test` (rebuilds the binary itself in a `beforeAll`, so it never runs
against a stale one). It installs the `tracing-subscriber` the kernel's own `tracing` facade needs;
`RUST_LOG=trace janus-engine` logs every frame `Engine::dispatch` sees in both directions to
stderr (stdout stays frames-only).

## Architecture rules

- The kernel knows nothing about HTTP or JSON: transports, content handlers, matchers/generators and
  hooks — including built-ins — are components behind the published component interfaces. Flag any
  change that leaks protocol/content knowledge into the kernel.
- Errors are values at the engine boundary; panics never cross the protocol. Sessions are the only
  resource (no per-object cleanup calls).
- SDKs are thin: no matching logic or orchestration beyond the protocol operations. Generated protocol
  bindings are never hand-edited — regenerate them.
- Corpora are load-bearing: any matching-behaviour change must change `corpora/` in the same commit,
  and any change to what an SDK's DSL does belongs in the behavioural specification and
  `conformance/`, which every SDK runs (ADR 0017).
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
