# AGENTS.md

Guidance for AI coding agents working in this repository: Claude Code, which reads it through
`CLAUDE.md`'s import, GitHub Copilot (the coding agent and PR review), and any other agent that reads
`AGENTS.md`. It is the only copy, so edit it here.

## Overview

Pact Janus is a ground-up redesign of the [Pact](https://pact.io) contract testing framework, based on
the [Pact MkII RFC](https://github.com/pact-foundation/roadmap/pull/146). It began as a prototype of
the RFC and is now the real build, hardening that prototype in place (ADR 0027): a single
Rust reference engine behind a coarse-grained versioned protocol, thin language SDKs, declarative
interaction specs compiled to inspectable matching plans, a shape language with variant testing, and
plugins as first-class components.

Three sources are the source of truth and take precedence over this file when they conflict:

- `Documentation/staged-implementation-plan.md` — the plan for the real build, and the live one;
  item ids like "S1.4" refer to it. `Documentation/prototype/project-plan.md` — the phased plan the
  prototype was built to; task numbers like "1.6" or "G1" refer to it. It is finished, and it and the
  reports beside it are a record: cite them, do not update them.
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

> **Status**: the prototype is complete — Phases 0–9 of the project plan are done, and the staged plan
> for the real build is `Documentation/staged-implementation-plan.md` (task 9.4). Since 2026-10-11 that
> plan is the live one: work happens against its item ids (`S0.1`, `S1.4`, …), starting with stage 0a.
> Done so far: S0.12, S0.1 (the plan grammar is v1: every action is `namespace:name`), S0.13, S0.2, S0.3,
> S0.4 (v0.1.0 is released; a change that ships carries a change file), S0.10 (both SDK suites
> run with no Rust toolchain, against a released engine they resolve by its pinned digest), S0.11
> (a release carries `janus-sdk-gates` and `janus-spec.tar.gz`, so one SDK runs its gates from them alone),
> S0.5 (the codename is rename-ready: `Documentation/codename.md`, rehearsed weekly). Stage 0a is done.
> In stage 1a: S1.1 (the differential harness runs in CI, and every verdict `janus verify` and today's
> verifier disagree on is in `corpora/verifier/disagreements.json` with the item that closes it), and
> S1.18 (a v1–v4 regex matches the whole value: the legacy compiler and `janus upgrade` anchor it),
> and S1.2 (ADR 0032: the broker client is a native host capability behind `verification/verify`'s
> `broker` source and `publish` option and `broker/publish-contracts`; specified, built in S1.3),
> and S1.3 (`janus verify` fetches from a Pact Broker, with pending and WIP pacts, and publishes each
> pact's result; `janus publish` publishes contracts; tested against the pinned broker, and weekly
> against the latest), and S1.4 (ADR 0034: a v1–v4 pact's request generators are applied before
> sending, through the built-in `pact` generator component, and a state-change reply's values reach
> its `ProviderState` generators; contracts are S1.19), and S1.5 (ADR 0035: providers and brokers
> are reached through one async, thread-free HTTP client, `engine/http-client`, configured by the
> HTTP client options; `janus verify` takes today's transport options and TLS, proxy and HTTP/2 ones,
> and `Documentation/verifier-migration.md` maps each), and S1.11 (the v1–v4 compatibility layer is
> `engine/kernel/src/legacy/`, the kernel's one HTTP- and JSON-shaped module, and a test holds the rest
> of the kernel to it), and S1.6 (ADR 0036: text bodies decode through the built-in `text` component,
> a body no component claims is bytes, and a v1–v4 text or binary body verifies as today's verifier
> compares it), and S1.20 (ADR 0037: an object's members keep the order they arrived in and nothing is
> decided by it, so a mock serves them as declared; the legacy compiler compiles a body as a document),
> and S1.21 (ADR 0038: form bodies decode through the built-in `form` component, a v1–v4 form body verifies
> as a query string, and its request generators are applied), and S1.22 (ADR 0039: XML bodies decode
> through the built-in `xml` component into a document keyed by element name, a v1–v4 XML body verifies as
> today's verifier compares it, and the legacy harness runs every pact-specification case, skipping none),
> and S1.23 (ADR 0040: multipart bodies decode through the built-in `multipart` component into a document
> keyed by part name, each part's body through the built-in component for its type, and a v1–v4 multipart
> body verifies as today's verifier compares it, so no body kind is refused), and S1.17 (ADR 0041: a
> `--variant` naming an id no document records is refused, and a run that left a variant out reports
> `filtered`, never `verified`, and is not published), and S1.24 (ADR 0042: a v1–v4 body's `values`,
> `eachKey`, `eachValue` and `arrayContains` rules compile as today's verifier applies them), and S1.26
> (ADR 0043: a pact's `Content-Type` is checked once, on the header; today's second check before the
> body is a deliberate difference), and S1.9 (ADR 0044: an `http` hook sends through `engine/http-client`
> with a `client` member and tries a state call three times; hook secrets never reach a log; `exec`
> deadlines end the process group and scripts' memory and stack are bounded), and S1.8 (ADR 0045: a
> v1–v4 interaction that needs a pact plugin is refused by name before anything runs, and the rest of
> its pact verifies), and S1.7 (ADR 0046: a v1–v4 message is asked of the provider through a
> `produce-message` hook; unless one is configured, `janus verify` adds one that asks as today's verifier
> does, routed by `--transports`; a transport names its own parts and rejects a request that does not
> match), and S1.25 (ADR 0047: `janus upgrade` converts text, binary, form, XML and multipart bodies
> into their components' documents, declaring the type a component reads; a pact that names no
> `Content-Type` header has its reply-type check written as a pattern on the header; every v1–v4
> corpus case reaches one verdict compiled directly and upgraded), and S1.19 (ADR 0048: a shape's
> `generator` is `{name, config}`, which `janus upgrade` writes for a v1–v4 generator; a verifier fills a
> contract's request with fresh values after state setup, and a mock its reply on every exchange, while
> the example stays the evidence; S1.1's harness also verifies every fixture's upgraded contract,
> against `corpora/verifier/upgrade-disagreements.json`), and S1.10 (ADR 0049: `match:content-type`
> asks the session's content registry, reading a whole slot from the arrival itself: its declared or
> labelled type, else what its octets are detected as; a v1–v4 `contentType` matcher sniffs the octets),
> and S1.12 (ADR 0050: `verification/explain` takes `components` and `values`, compiling what a run
> compiles and running captured values in the engine; `janus explain --config` and `--executed` use them),
> and S1.28 (ADR 0051: a contract has no pending interactions, pending being the broker's to decide;
> `janus upgrade` converts one without its flag, or leaves it out with `--exclude-pending`), and S1.27
> (ADR 0052: a component's registry credential is its declaration's `source.auth`, interpolated by the
> loader and never repeated; a run reads no environment for it, and only `janus component push|pull`
> read `JANUS_OCI_USERNAME`/`JANUS_OCI_PASSWORD`). S1.13 is built (ADR 0053: `janus verify` takes
> today's spellings and variables where they mean what they did, its filters as `options.filter`,
> pacts by URL as a `urls` source, and `--exit-on-first-error` as `stop-on-first-failure`;
> `Documentation/verifier-migration.md` has a row for every option), and its exit waits on pilot
> providers accepting the dropped rows. S1.29, the CLI output review, is done: an engine error's
> `details` are not printed in text, a component's failure leads with its own code, a hook's failure
> is printed once under the result it failed, a run the engine ends before it verifies anything
> exits 2, and a stopped or aborted run's summary counts `variants.not-run`. So is S1.30: every
> mismatch path is absolute (an element a `for-each` was on, not `~>.value`), and the executed form
> writes an absent value as `ABSENT`. S1.31 coloured the rest of the CLI's output on a terminal, from
> one palette (`cli/src/style.rs`), and stripped it everywhere else. S1.32: `janus explain
> --description` chooses an interaction by its description, listing the candidates when it names
> several or none. S1.33 (ADR 0054): an error carries its causes as `chain`, its layers outermost
> first; the CLI leads with what failed and its root cause, and `--error-chain` prints every layer.
> S1.34 (ADR 0054): a host that declares the `segments` capability gets `text-segments` and
> `message-segments` beside the engine's display text; the CLI colours plans, the `check` page and
> mismatch messages from them, and parses none of it.
> Stage 1a's items are all built; its exit waits on pilot providers. Stage 1b has started: S1.14
> (ADR 0055): an SDK verifies a provider in a test through one `verification/verify`, with state
> handlers and request filters as loopback `http` hooks it serves for the run behind a bearer token,
> and reads `verifier.janus.yaml` through the engine's host (`config/load`, behind `config-loading`).
> Both SDKs have a `Verifier`, and the conformance suite has a `provider` category. S1.15 (ADR 0056):
> message producers are loopback `produce-message` hooks; the JVM SDK has `JanusProviderExtension`
> (`@State`, `@ProvidesMessage`, one dynamic test per interaction result of one run), and the
> TypeScript SDK `pact-janus/verifier`, pact-js's provider API over its own (the thinness audit's
> `facade` layer). pact-js's and pact-jvm's provider examples run on them, their pacts S1.1 fixtures
> under `corpora/verifier/examples/`; `Documentation/provider-test-migration.md` is the migration
> note. They found S1.35, the v4 `statusCode` matcher, now done (ADR 0057): the built-in HTTP component
> runs `http:status-class` as a matcher the embedding registers (`Engine::register_matcher`), and the
> legacy compiler compiles a pact's rule to it. ADR 0058: a provider state no `state-setup` hook
> admits is a `state-unhandled` warning, or fails the interaction with `options.unhandled-states:
> fail` (`janus verify --unhandled-states fail`); the SDKs carry warnings on the result and print them.
> The layout and commands below describe the repo as built; trust the repo over this file if they diverge.

## Intended repository layout

```
engine/          Rust workspace members: kernel, built-in component crates, hooks-host —
                 the host side of hooks (loader, exec and http implementations), which lives
                 outside the kernel because it needs a filesystem, a process and a socket — and
                 component-host, the wasmtime loader for out-of-tree WASM components (task 8.1),
                 outside the kernel because a WASM guest cannot host WASM (ADR 0013);
                 broker-client, the Pact Broker client the kernel's broker trait is implemented
                 by (ADR 0032, S1.3), outside the kernel because it speaks HTTP;
                 http-client, the one HTTP client the HTTP transport's drive role and the broker
                 client send through (ADR 0035, S1.5): reqwest on a current-thread runtime, which
                 starts no thread, configured by the HTTP client options (component-interfaces §5.6);
                 component-text, the built-in content component for `text/*` (ADR 0036, S1.6),
                 registered after component-json by every embedding; component-form, the built-in
                 one for `application/x-www-form-urlencoded` (ADR 0038, S1.21), registered after it;
                 component-xml, the built-in one for XML (ADR 0039, S1.22), registered after
                 component-json and before component-text, so `text/xml` reaches it;
                 component-multipart, the built-in one for `multipart/*` (ADR 0040, S1.23),
                 registered after component-form, which decodes each part through the built-in
                 JSON, XML, text and form components it holds itself;
                 and wasm, the engine itself as a WASM component — the offline artifact a release
                 ships (ADR 0023 decision 2, S0.4), a member that is empty on any target but wasm32
cli/             `janus` CLI (verify, explain, upgrade, check)
sdks/typescript/ TypeScript SDK prototype (DSL + Jest/Vitest integration); src/generated/ holds
                 its protocol bindings (task 6.1)
sdks/jvm/        JVM SDK prototype (DSL + JUnit 5 integration); the Gradle `bindings` project
                 holds its protocol bindings (task 6.1)
sdks/bindings.json  Which spec schema sets the SDKs' bindings are generated from
sdks/engine-release.json  The released janus-engine an SDK resolves when it is not building one:
                 a version and a SHA-256 per target triple (ADR 0029)
corpora/         Golden corpora: (spec or pact, expected plan, expected result); and verifier/,
                 S1.1's fixture corpus — pacts with the providers that answer them — and its
                 disagreement ledger, disagreements.json
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
                 (task 6.4), tools/thinness, the SDK thinness audit (task 6.5) — it reads
                 each SDK's own thinness.json, counts it by layer and budgets the hand-written ones —
                 tools/doc-links, the documentation link check (S0.12) — and tools/engine-pin, which
                 writes sdks/engine-release.json from a release's attested SHA256SUMS (S0.10).
                 tools/sdk-gates is the first three as the one binary a release ships,
                 `janus-sdk-gates` (S0.11, ADR 0030). tools/verifier-diff is S1.1's differential
                 harness: today's verifier and janus verify over corpora/verifier/
                 tools/test-servers holds the servers tests that need a network share (S1.5): a
                 certificate authority made per test, a TLS provider that can demand a client
                 certificate, and a forward proxy that records what it carried.
.changeset/      Change files: what each unreleased change means to someone who pins the engine,
                 and how big it is. `knope prepare-release` consumes them (ADR 0028); knope.toml
                 configures it and CHANGELOG.md is what it writes
Documentation/   The staged plan, ADRs, specs and the charter; prototype/ is the prototype's
                 record — the project plan and each task's report, kept as written (S0.12)
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
cargo build --release -p pact_janus_engine_wasm --target wasm32-wasip2   # the engine as a WASM component
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

The checkers read the repository from the working directory, so run them from its root, or name it
with `--spec <dir>`; `JANUS_SPEC` tells the SDKs' own drivers the same. Either may instead be an
unpacked `janus-spec.tar.gz` from a release, whose paths are the repository's (S0.11, ADR 0030).

A conformance id in `behavioural-spec.json` with no case fails the lint, and a case that fails in
either language fails CI. When a case and an SDK disagree and the specification does not settle it,
change `behavioural-spec.json` first — `conformance/README.md` §7.

The thinness audit (plan task 6.5) is the other claim made into a command: every SDK source file
must be classified into a layer in its SDK's `thinness.json` (`sdks/<sdk>/thinness.json`, paths
relative to it), and the hand-written layers stay inside a
budget, so matching logic or orchestration creeping back into an SDK fails the build:

```bash
cargo run -p pact_janus_thinness              # the table, by layer, per SDK
cargo run -p pact_janus_thinness -- check     # the same, as an exit code (CI runs this)
```

A new SDK source file with no layer is an error, not a default — say which layer it belongs to.
Raising a budget is a decision that belongs in the commit message.

The differential verification harness (S1.1) is "exactly as today" made into a number: it runs
today's verifier (`pact_verifier_cli`, at the version `tools/verifier-diff/pact-verifier.json` pins)
and `janus verify` on every pact in `corpora/verifier/` against the same provider, and compares the
verdicts interaction by interaction. Every disagreement is in `corpora/verifier/disagreements.json`
with the plan item or ADR that accounts for it, or CI's `verifier-diff` job fails — and so does an
entry nothing disagrees on any more, so a change that closes a gap deletes its entries. It also
upgrades each fixture's pacts and verifies the contracts, comparing `janus verify`'s verdicts on the
two (S1.19): those differences are in `corpora/verifier/upgrade-disagreements.json`, by the same rules. Its broker
cases (S1.3) publish their pacts to the Pact Broker `tools/verifier-diff/pact-broker.json` pins, a
container `.github/scripts/pact-broker.sh` starts, and compare what each run published too:

```bash
.github/scripts/pact-verifier.sh target/pact-verifier   # the pinned verifier, checksum verified
.github/scripts/pact-broker.sh start                     # the pinned broker (Docker); prints its URL
cargo build -p pact_janus_cli --bin janus
PACT_VERIFIER_CLI=target/pact-verifier/pact-verifier-linux-x86_64 \
  cargo run -p pact_janus_verifier_diff -- run --broker http://127.0.0.1:9292
```

The documentation link check (S0.12) holds every Markdown file in the repository to one rule: a
relative link resolves to a file that exists, and to a heading in it when the link names one. It runs
offline — links with a scheme are not fetched — and as part of `cargo test`:

```bash
cargo run -p pact_janus_doc_links             # every relative link, as an exit code (CI runs this)
```

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
builds `janus-engine`, `janus` and `janus-sdk-gates` for five target triples, `janus-engine.wasm` and
`janus-spec.tar.gz`, runs each SDK's gates from those files alone, checks that each
binary is statically linked and runs, attests every file, and only then tags `v<version>` and
publishes. It rehearses everything but the publishing on any pull request that touches it. Never push
a `v*` tag by hand, and never hand-edit `CHANGELOG.md`. An SDK resolves a released engine from
`sdks/engine-release.json` — a version and a SHA-256 per target triple — and refuses a download with
any other digest. Both SDK suites do so with `JANUS_ENGINE_RELEASE=1` (S0.10;
`sdks/typescript/test/engine-release.ts`, and `EngineRelease.java` in the JVM SDK's tests, which
Gradle's `resolveEngine` runs), so a contributor needs no Rust toolchain, and build from the tree
otherwise; `JANUS_ENGINE` overrides both. The pin is never edited by hand: after publishing,
`release.yml` writes it, runs both suites against the release with no Rust toolchain reachable, and
pushes it as the `engine-pin` branch to open a pull request from. By hand:

```bash
cargo run -p pact_janus_engine_pin -- <version>   # gh attestation verify on SHA256SUMS, then the pin
```

The one test that still builds Rust is the TypeScript CSV consumer's (`third-party/janus-csv`);
`JANUS_CSV_COMPONENT` names a `janus_csv.wasm` built elsewhere instead, as CI's `sdks` job does.
CI runs the suites through `.github/scripts/without-rust.sh <command>`, which leaves every directory
holding `cargo`, `rustc` or `rustup` off `PATH` and checks they no longer run; run a suite through it
locally to see what still reaches for Rust.

Generated protocol bindings (plan task 6.1) — both SDKs' typed views of the spec schemas
`sdks/bindings.json` names, checked in and **never hand-edited**. Regenerate after any change to
those schemas (needs Node, with `npm ci` done in `sdks/typescript/`, and JDK 17); CI regenerates
and fails on any diff:

```bash
cargo run -p pact_janus_bindings -- generate                    # both SDKs
cargo run -p pact_janus_bindings -- generate --only typescript  # or --only jvm
janus-sdk-gates bindings --spec <unpacked janus-spec> generate --only jvm --sdk .   # one SDK, from a release
```

The `janus` CLI (`cli/`, plan task 5.5; its command line is declared once, with clap, in
`cli/src/args.rs`) drives the engine **through the protocol** — it builds frames and reads frames
back, never the kernel's Rust API, so anything a command cannot do is a finding about the protocol
first:

```bash
janus verify <contract-or-pact>... --provider-url <url> [--config verifier.janus.yaml]
             [--variant <id>] [--explain-failures] [--output text|json]
             [--base-path <p>] [-H <KEY=VALUE>]... [--request-timeout <ms>] [--connect-timeout <ms>]
             [--disable-ssl-verification] [--ca-cert <pem>]... [--client-cert <pem> --client-key <pem>]
             [--proxy <url> [--no-proxy <hosts>]] [--http-version 1.1|2|negotiate]   # S1.5, ADR 0035
             [--transports <name:port[/path]>]...   # where HTTP interactions and messages go (S1.7, ADR 0046)
             [-s <state-change-url> [--state-change-as-query] [--state-change-teardown]]   # write the state hooks
             [--filter-description <re>] [--filter-state <re> | --filter-no-state] [-c <consumer>]...
             [--exit-on-first-error]   # today's options (S1.13, ADR 0053); --hostname/--port for --provider-url
janus verify --broker-url <url> --provider-name <name> --provider-url <url>
             [--consumer-version-selectors <json>]... [--consumer-version-tags <t,...>] [--enable-pending]
             [--include-wip-pacts-since <date>] [--ignore-no-pacts-error] [--retries <n>]
             [--provider-version <v>] [--provider-branch <b>] [--provider-tags <t,...>] [--publish]
             [--build-url <url>] [--user <u> --password <p> | --token <t>]   # today's spellings (S1.13); --broker-* aliases
janus verify --url <pact-url>... | --webhook-callback-url <pact-url>   # pacts by URL, published when a broker served them
janus check <contract-or-pact>... [--provider-shape <path>]... [--verification <file>]...
            [--policy <file>] [--on-finding warn|block] [--on-review warn|block]
            [--as-of <YYYY-MM-DD>] [--output text|json]
janus explain <document.json> [--index N | --description <text>] [--variant <id>] [--spec] [--plan] [--executed <values.json>]
              [--config verifier.janus.yaml]   # the components a run would load (S1.12, ADR 0050)
janus upgrade <pact.json> [--write <file>] [--quiet] [--exclude-pending] [--output text|json]
janus publish <contract-or-pact>... --consumer-app-version <v> --broker-base-url <url>
              [--branch <b>] [--tag <t>]... [--build-url <url>]
              [--broker-username <u> --broker-password <p> | --broker-token <t>]   # pact-broker publish's spellings
janus component push <component.wasm> <reference> [--output text|json]
janus component pull <reference> [--digest <sha256:...>] [--output text|json]
janus completions <shell>
janus version                      # every command also takes --log-level <level>, --no-colour, --error-chain
```

Exit codes are part of the surface: `0` the command did what it was asked, `1` the *subject* failed
(a verification found mismatches — an answer, not an error), `2` the command could not run,
which includes every usage error. The flag spellings are a surface too: S1.13 maps today's
verifier options onto them, so renaming one is a breaking change with a change file. The CLI's
own tests pin both: `cli/tests/cmd/` holds trycmd cases — every `--help`, the usage errors, the
offline commands — and `cli/tests/cli.rs` the runs that need a provider or a registry, compared
whole with snapbox where the output is the CLI's own. After a deliberate change,
`TRYCMD=overwrite` and `SNAPSHOTS=overwrite` refresh them; read the diff. On a terminal, the output
is coloured (S1.31): from one palette, `cli/src/style.rs`. Text the engine wrote — plans, the
`check` page, mismatch messages — is coloured from the segments the engine marks it with (S1.34,
engine-protocol spec §2.7, `cli/src/highlight.rs`), which a command asks for only when it prints
text: never parse the engine's text to style it. Style text where it is built and print it with `outln!`/`errln!`, whose
streams strip the colour off a terminal, under `NO_COLOR` and with `--no-colour`; never print
styled text another way. The tests run without colour whatever `CLICOLOR_FORCE` says, and a test
that asks for colour also checks that the colour is all that differs.

`check` (plan task 7.4) decides rather than tests: it reads documents that already exist — the
consumer contracts or v1–v4 pacts, the shapes a provider published, and the summaries
`janus verify --output json` wrote — and answers `can-i-deploy` over them, so it needs no provider
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

The broker client's own tests replay a scenario recorded from the pinned broker
(`engine/broker-client/tests/recorded/pact-broker.json`, offline, on every system); CI's
`verifier-diff` job records it again and fails on any difference. After a deliberate change, or a
new broker pin, record it with the broker running and read the diff:

```bash
RECORDINGS=overwrite JANUS_PACT_BROKER=http://127.0.0.1:9292 cargo test -p pact_janus_broker_client --test recorded
```

The weekly `Latest Pact Broker` workflow runs every one of these broker checks against
`pactfoundation/pact-broker:latest` (`PACT_BROKER_IMAGE` tells `pact-broker.sh` which image, and
`JANUS_PACT_BROKER_ANY_VERSION=1` lets the recording be of another version), so a broker release
that changes what clients see is found before the pin moves to it.

The client also has a consumer pact of its own with the Pact Broker (`engine/broker-client/tests/pact.rs`,
written with `pact_consumer`): every run writes it and checks the client against it, and with
`JANUS_PACT_BROKER` and `PACT_VERIFIER_CLI` set, as in that job, today's verifier verifies it against the
pinned broker, each provider state set up through the broker's API. `.cargo/config.toml` sets
`PACT_DO_NOT_TRACK`, so no test run sends `pact_consumer`'s usage metrics.

`janus publish` (S1.3) publishes through the engine's broker client (`broker/publish-contracts`), as
`verify --broker-url` fetches through it: no command speaks the Pact Broker's API itself. Its live
tests run against the pinned broker when `JANUS_PACT_BROKER` names it (CI's `verifier-diff` job), and
skip otherwise.

`janus component push|pull` (plan task 8.2) is the one command that does not speak the protocol —
publishing is not an engine operation. `push` writes the artifact's config from the component's own
handshake; `pull` fetches into the cache, checks every byte, and prints the pinned declaration. A pin
is fetched by digest alone and a second run fetches nothing. Loopback registries are plain HTTP.
A run's registry credential is its declaration's `source.auth`, `${VAR}`-interpolated by the loader
(ADR 0052): a run reads no environment for it. Only `push` and `pull`, which have no configuration,
read `JANUS_OCI_USERNAME`/`JANUS_OCI_PASSWORD`. The cache is `JANUS_COMPONENT_CACHE`
(default: the user cache directory's `pact-janus/components`). The OCI tests run against an in-process
registry that tampers on request, and against a real one when `JANUS_OCI_REGISTRY` names it
(`docker run -d -p 5000:5000 registry:2`, then `JANUS_OCI_REGISTRY=localhost:5000`; CI does this).

`janus-engine` (the subprocess embedding, `cli/src/bin/janus_engine.rs` — ADR 0023, every SDK's primary embedding): built by the
normal `cargo build`/`cargo test` above (`cargo build -p pact_janus_cli --bin janus-engine`
targets it alone). Its own protocol-level Node test client — plan task 4.5's "thin test client
speaks the protocol directly," not an SDK, and deliberately outside `sdks/` — lives at
`cli/tests/janus-engine-node/`: `npm install` once, then `npm test` (rebuilds the binary itself
via `cargo build` in a `beforeAll`, so it never runs against a stale one). It installs the
`tracing-subscriber` the kernel's own `tracing` facade needs (this file's Rust conventions below);
`RUST_LOG=trace janus-engine` logs every frame `Engine::dispatch` sees in both directions to
stderr (stdout stays frames-only), with every broker credential redacted: an `auth` member keeps
only its `kind` and a `url` loses its user information. Both binaries also cap the HTTP library's
own logging at `debug` however much is asked for (`cli/src/logging.rs`), because at `trace` it dumps
request bytes, `Authorization` headers included. `cli/tests/cli.rs` runs both commands at `trace`
behind an authenticating door in front of the pinned broker and fails if a credential appears in
anything they print.

## Architecture rules

- **The kernel knows nothing about HTTP or JSON.** Transports, content handlers, matchers/generators and
  hooks are components behind the interfaces in the component-interface spec — including the built-in
  ones. If a change leaks protocol- or content-specific knowledge into the kernel, stop and flag it
  (plan task 3.8 tracks exactly this). The one exception is `engine/kernel/src/legacy/`, the v1–v4
  compatibility layer: a v1–v4 pact is HTTP and JSON by specification, so reading one, compiling its
  matching rules and upgrading it live there and nowhere else (S1.11). Outside it, the kernel names
  no `pact_models` type but the two `engine/kernel/tests/boundary.rs` allows. A new use is either moved
  behind the boundary or added there with its reason.
- **Errors are values at the engine boundary.** Panics must never cross the protocol; every operation
  returns structured errors. An error's causes are its `chain`, layer by layer (ADR 0054), never
  joined into a string where it is produced: lift a library's `source()` chain with
  `chain::layers_of`, and give an error causes with `caused_by`. Sessions are the only resource — no per-object cleanup calls. A plan node
  the engine's own compilers did not build — a component's fragment, a plan document read back — is
  not trusted: the interpreter reads an action's children by position, so every core action has a
  least child count in `engine/kernel/src/plan/actions.rs`, and a new action is added there first.
- **SDKs are thin.** No matching logic, no orchestration beyond the protocol operations, in any SDK.
  Generated protocol bindings are never hand-edited — regenerate them via the binding pipeline.
- **Corpora are load-bearing.** Any change to matching behaviour must change `corpora/` in the same
  commit. CI runs plans against the corpora; a behaviour change without a corpus change is a bug.
  The same holds one layer up: a change to what an SDK's DSL does belongs in the behavioural
  specification and `conformance/` — every SDK runs that corpus, and "conformant" means passing it
  (ADR 0017).
- **A change that ships says so, in a change file.** The version and the changelog come from
  `.changeset/`, not from commit subjects (ADR 0028). The `v<version>` tag and the release's asset
  names (`janus-engine-<triple>`, `janus-<triple>`, `janus-sdk-gates-<triple>`, `janus-engine.wasm`,
  `janus-spec.tar.gz`, `SHA256SUMS`; ADR 0030) are what SDKs
  pin against: changing one is a superseding ADR, not an edit to a workflow. No release step may need
  a credential that outlives a workflow run (ADR 0028 decision 8, ADR 0029).
- **Documents are cited by path, so links are load-bearing too.** Every relative link in the
  repository's Markdown must resolve, to a file and to a heading when it names one; moving or renaming
  a document means fixing what cites it in the same commit. `spikes/` holds each spike's `FINDINGS.md`
  and no code (S0.12): the findings stay where the ADRs cite them, and nothing new is added there.
- **The codename is a codename.** "Janus" is renamed by one command, `.github/scripts/rename.sh <name>`,
  which the weekly `Rename rehearsal` workflow runs on a throwaway checkout (S0.5, ADR 0031,
  `Documentation/codename.md`). Anything new may carry the name; the script finds it. The exception
  is a name the engine **reads back from a document that outlives it** — a `$format`, a media type,
  an exported interface, something a user's configuration or script spells: define it in
  `engine/kernel/src/persisted.rs` and read it with `reads()`, never against a literal, or documents
  written before the rename stop being readable. The rehearsal cannot catch a literal: it renames it.

## Rust conventions

- Rust edition 2024, latest stable toolchain; `cargo clippy` clean.
- Logging via `tracing` / `tracing-subscriber` (not `log`); async via `tokio` (not on WASM paths).
- Test helpers: `rstest`, `expectest`, `pretty_assertions` — follow these before introducing new ones.
  A command's output is tested as a snapshot: `trycmd` for cases that need nothing running, `snapbox`
  inside a Rust test for the rest. Elide text the engine owns (`corpora/` pins it).
- Engine crates that ship as WASM components must build for `wasm32-wasip2`; keep native-only
  dependencies (`tokio`, `reqwest`, raw sockets) out of those crates or behind target-gated features.
- Everything builds and is tested on Linux, macOS and Windows (S0.2; CI runs all three). Never read
  meaning out of a path's separator or spell one in an expected value — compare components, or
  normalise to `/` where the path is an identifier. No Unix-only API or `/bin/...` path without a
  Windows answer. `.gitattributes` pins text to LF in every checkout; keep it that way.

## TypeScript / JavaScript conventions

- TypeScript everywhere in SDK and tooling source; `strict: true`, no `any` in exported API surfaces.
  Plain JavaScript only in generated output.
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
