# Copilot Instructions

Pact Janus is a ground-up redesign of the [Pact](https://pact.io) contract testing framework per
the [Pact MkII RFC](https://github.com/pact-foundation/roadmap/pull/146) — begun as a prototype of the
RFC, now the real build, hardening that prototype in place (ADR 0027): one Rust reference engine
behind a coarse-grained versioned protocol, thin language SDKs, interaction specs compiled to
inspectable matching plans, a shape language with variant testing, and plugins as first-class components.

Source of truth: `Documentation/staged-implementation-plan.md` (the plan for the real build, and the
live one; item ids like "S1.4" refer to it), `Documentation/prototype/project-plan.md` (the phased plan
the prototype was built to; task numbers like "1.6"/"G1" refer to it — it and the reports beside it are
a finished record: cite them, do not update them), `Documentation/decisions/` (ADRs — don't re-litigate accepted ones in code; supersede them), and
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

> **Status**: the prototype is complete — Phases 0–9 of the project plan are done, and the staged plan
> for the real build is `Documentation/staged-implementation-plan.md` (task 9.4). Since 2026-10-11 that
> plan is the live one: work happens against its item ids (`S0.1`, `S1.4`, …), starting with stage 0a.
> Done so far: S0.12, S0.1 (the plan grammar is v1: every action is `namespace:name`), S0.13, S0.2, S0.3,
> S0.4 (v0.1.0 is released; a change that ships carries a change file), S0.10 (both SDK suites
> run with no Rust toolchain, against a released engine they resolve by its pinned digest).
> The layout and commands below describe the repo as built; trust the repo over this file if they diverge.

## Repository layout

```
engine/          Rust workspace members: kernel, built-in component crates, hooks-host —
                 the host side of hooks (loader, exec and http implementations), which lives
                 outside the kernel because it needs a filesystem, a process and a socket — and
                 component-host, the wasmtime loader for out-of-tree WASM components (task 8.1),
                 outside the kernel because a WASM guest cannot host WASM (ADR 0013);
                 and wasm, the engine itself as a WASM component — the offline artifact a release
                 ships (ADR 0023 decision 2, S0.4), a member that is empty on any target but wasm32
cli/             `pact` CLI (verify, explain, upgrade, check)
sdks/typescript/ TypeScript SDK prototype (DSL + Jest/Vitest integration); src/generated/ holds
                 its protocol bindings (task 6.1)
sdks/jvm/        JVM SDK prototype (DSL + JUnit 5 integration); the Gradle `bindings` project
                 holds its protocol bindings (task 6.1)
sdks/bindings.json  Which spec schema sets the SDKs' bindings are generated from
sdks/engine-release.json  The released janus-engine an SDK resolves when it is not building one:
                 a version and a SHA-256 per target triple (ADR 0029)
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
demo/            The RFC's whole loop as a runnable walkthrough (task 9.3): web-app/, a TypeScript
                 consumer of samples/order-service; run.sh, which records its contract, verifies
                 it with hooks, runs janus check, widens and fixes it (--ci asserts each step, and CI
                 runs it); demo.tape, the VHS script that records demo.gif
third-party/     Components written as a third party would, from the published interfaces
                 only — each its own Cargo workspace, excluded from ours and depending on
                 nothing in it: third-party/janus-csv, the text/csv content component task 8.1
                 built from the docs (Documentation/prototype/third-party-component-report.md)
spikes/          The prototype's time-boxed experiments: each one's FINDINGS.md, at the path the
                 ADRs cite, and no code (S0.12 deleted it; `b8d5010` is the last commit with it)
benchmarks/      Baseline/trend benchmark harness (task 1.7) — durable, standalone crate
                 (excluded from the workspace; run with `cd benchmarks && cargo run --release`).
                 benchmarks/janus runs the same scenarios against Janus in each embedding
                 (task 9.1, `cd benchmarks/janus && cargo run --release`); it builds engine/wasm
                 with a benchmark-only interface to measure the WASM embedding. Every merge to main
                 runs it in CI and adds a point to the trend chart (S0.3, the Benchmarks workflow;
                 a trend, not a gate — see benchmarks/README.md)
tools/           Repo tooling (workspace members), e.g. tools/schema-compat — the CI checker
                 for the open-world rules every schema under Documentation/specs/ follows,
                 and for the specs' worked examples — tools/bindings, the binding-generation
                 pipeline (task 6.1), tools/conformance, the conformance suite's checker
                 (task 6.4), tools/thinness, the SDK thinness audit (task 6.5) — it counts each SDK
                 by layer — tools/doc-links, the documentation link check (S0.12) — and
                 tools/engine-pin, which writes sdks/engine-release.json from a release's
                 attested SHA256SUMS (S0.10)
.changeset/      Change files: what each unreleased change means to someone who pins the engine,
                 and how big it is. `knope prepare-release` consumes them (ADR 0028); knope.toml
                 configures it and CHANGELOG.md is what it writes
Documentation/   The staged plan, ADRs, specs and the charter; prototype/ is the prototype's
                 record — the project plan and each task's report, kept as written (S0.12)
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
cargo build --release -p pact_janus_engine_wasm --target wasm32-wasip2   # the engine as a WASM component
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

Releases (S0.4, ADR 0028 and ADR 0029) are decided by **change files**, never by commit subjects. A
pull request that changes what a release ships — the engine's or the CLI's sources or manifests,
`Cargo.lock`, `corpora/`, `conformance/` — adds a file under `.changeset/`, or carries a line
`No-Changeset: <why>` in its description; CI's `Change file` workflow fails it otherwise:

```bash
knope document-change               # asks for the size and a summary, and writes .changeset/<name>.md
knope prepare-release --dry-run     # what the change files gathered so far add up to
```

The size is what the change costs someone who pins the engine: `major`, they must change something
to move to it; `minor`, they can do something new; `patch`, a fix. Before 1.0, `major` moves the
minor version and the other two the patch. The body is the changelog entry, written for an SDK
maintainer deciding whether to move a pin. A release is prepared as a pull request (the
`Prepare release` workflow, or `knope prepare-release` by hand) and made by merging it: `release.yml`
builds `janus-engine` and `janus` for five target triples and `janus-engine.wasm`, checks that each
binary is statically linked and runs, attests every file, and only then tags `v<version>` and
publishes. It rehearses everything but the publishing on any pull request that touches it. Never push
a `v*` tag by hand, and never hand-edit `CHANGELOG.md`. An SDK resolves a released engine from
`sdks/engine-release.json` — a version and a SHA-256 per target triple — and refuses a download with
any other digest. Both SDK suites do so with `JANUS_ENGINE_RELEASE=1` (S0.10;
`sdks/typescript/test/engine-release.ts`, and `EngineRelease.java` in the JVM SDK's tests, which
Gradle's `resolveEngine` runs), so a contributor needs no Rust toolchain, and build from the tree
otherwise; `JANUS_ENGINE` overrides both. The pin is never edited by hand: after publishing,
`release.yml` writes it, runs both suites against the release with no Rust installed, and pushes it as
the `engine-pin` branch to open a pull request from. By hand:

```bash
cargo run -p pact_janus_engine_pin -- <version>   # gh attestation verify on SHA256SUMS, then the pin
```

The one test that still builds Rust is the TypeScript CSV consumer's (`third-party/janus-csv`);
`JANUS_CSV_COMPONENT` names a `janus_csv.wasm` built elsewhere instead, as CI's `sdks` job does.

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
spawned over the protocol's stdio framing — specified (design 2.6 §9.3) but loaded by nothing:
spike 8.3's loader proved the binding and was deleted with the rest of the spike code (S0.12), and a
real one is S5.5. A declared component may contribute a transport,
looked up by the `kind`s its handshake names. A declared content component may contribute a **plan
fragment** for a slot (plan task 8.4, ADR 0022): it replaces the slot's generic plan, must declare a
plan grammar the engine lists in `component/hello` (`plan-grammar-versions`: `v1`, and `v0`, read by ADR 0026's
one-to-one renaming), may use only core actions
and its own contributed ones — run through `matcher/apply` — and is checked when the interaction arrives,
failing as `component-unavailable` with `reason: grammar-skew` or `fragment-invalid`. `janus`
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

`janus-engine` (the subprocess embedding, `cli/src/bin/janus_engine.rs` — ADR 0023, every SDK's primary embedding): built by the
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
  resource (no per-object cleanup calls). A plan node the engine's own compilers did not build (a
  component's fragment, a plan document read back) is not trusted: every core action has a least
  child count in `engine/kernel/src/plan/actions.rs`, and a new action is added there first.
- SDKs are thin: no matching logic or orchestration beyond the protocol operations. Generated protocol
  bindings are never hand-edited — regenerate them.
- Corpora are load-bearing: any matching-behaviour change must change `corpora/` in the same commit,
  and any change to what an SDK's DSL does belongs in the behavioural specification and
  `conformance/`, which every SDK runs (ADR 0017).
- A change that ships says so, in a change file: the version and the changelog come from
  `.changeset/`, not from commit subjects (ADR 0028). The `v<version>` tag and the release's asset
  names (`janus-engine-<triple>`, `janus-<triple>`, `janus-engine.wasm`, `SHA256SUMS`) are what SDKs
  pin against — changing one is a superseding ADR, not a workflow edit — and no release step may need
  a credential that outlives a workflow run (ADR 0028 decision 8, ADR 0029).
- Documents are cited by path, so links are load-bearing too: every relative link in the repository's
  Markdown must resolve (`cargo run -p pact_janus_doc_links`, which CI runs), and moving a document
  means fixing what cites it in the same commit. `spikes/` holds each spike's `FINDINGS.md` and no
  code (S0.12): the findings stay where the ADRs cite them, and nothing new is added there.

## Conventions

Rust:
- Edition 2024, latest stable toolchain, `cargo clippy` clean.
- `tracing`/`tracing-subscriber` for logging (not `log`); `tokio` for async (not on WASM paths).
- Test helpers: `rstest`, `expectest`, `pretty_assertions` — prefer these to new utilities.
- Crates shipped as WASM components must build for `wasm32-wasip2`; keep native-only dependencies out
  of them or behind target-gated features.
- Everything builds and is tested on Linux, macOS and Windows (S0.2; CI runs all three). Never read
  meaning out of a path's separator or spell one in an expected value; no Unix-only API or `/bin/...`
  path without a Windows answer. `.gitattributes` pins text to LF in every checkout.

TypeScript / JavaScript:
- TypeScript with `strict: true` for all SDK/tooling source; no `any` in exported API surfaces. Plain
  JavaScript only in generated output.
- ESM modules, active Node LTS, Vitest for tests, ESLint + Prettier for lint/format, lean
  devDependencies.
- The public DSL mirrors the RFC's consumer example (`optional`, `anyOf`, `oneOf`, `eachLike`,
  `pact.execute(...)`); change the SDK specification first, then the code.

Commits follow Conventional Changelog style (`feat:`, `fix:`, `chore:`, `docs:`, `refactor:`, `test:`).

## Reference material

`pact-reference` (usually a sibling checkout) provides `pact_models` (dependency) and the v2 matching
engine prototype (`rust/pact_matching/src/engine`, forked as the kernel starting point). Read for
context; record provenance in the commit message when adapting code from it.
