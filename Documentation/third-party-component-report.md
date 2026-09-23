# Plan task 8.1: a third-party WASM component, written from the docs

**Date:** 2026-09-23. **Subject:** `third-party/janus-csv`, a `text/csv` content component, and what
the engine needed to host it. **Claim under test:** component-interfaces spec §13's last row —
"'Documented by reading the core' is true only if a third party never needs engine source" — and
milestone M6: *a third-party WASM component runs unmodified in both a consumer test and verification*.

## 0. Provenance

The task was done in two roles, one after the other, and the line between them is the point.

**As the component's author, I read no engine source.** Not the kernel, not `engine/component-json`
(the one in-tree content component, which is the obvious thing to copy), not the CLI. The inputs:

- component-interfaces `spec.md`, its CSV worked example, and the v1 schemas (`handshake`, `content`,
  `component-config`, `component-error`, `parts`);
- the engine protocol's frame schema and §3.1/§5;
- lifecycle-hooks §6–§8 (where the component configuration lives) and its project-config schema;
- shape-language §4.2 (what a content component's degradations mean);
- the consumer-session and verification schemas, to see how a declaration reaches the engine.

I needed two things the documents did not have. One was the WIT world, which I found only in a
spike (finding 1). The other was whether the engine could load a component at all, which I learned
by sending `engine/hello` to a built `janus-engine` (finding 2). **Count: zero reads of engine
source, one read of spike code, one black-box probe.**

The component was finished, with 16 frame-level tests, before the engine could load it. It has not
changed since, apart from `cargo fmt` and one comment's wording. The same `.wasm` is what every test
below loads.

**Then, as the engine's implementer, I read whatever I needed.** Hosting did not exist. That is not
a documentation finding: the spec describes a loader, and nobody had built one. What it needed is §1.

## 1. What was built

| | |
|---|---|
| The component | `third-party/janus-csv`: its own Cargo workspace, depending on nothing in this repo. It implements `content` (`decode`, `encode`, `compile` with no fragment, `detect` declined) for `text/csv`. It declares `string-only` and `no-null`, and reports per-decode degradations with their paths. It is a 216 KB `wasm32-wasip2` component. |
| The WIT world | `Documentation/specs/component-interfaces/wit/component.wit`: package `pact:janus-component@1.0.0`, world `component`, exporting `pipe.call`. |
| The loader | `engine/component-host`: wasmtime 47, `file` sources, a digest check before compiling, and imports checked against grants at load. Each call runs in a fresh instance with its own handshake, under an epoch deadline, and traps are synthesised as `component-trapped`/`component-timeout`. Native embeddings only (ADR 0013). |
| Kernel | `component::ContentRegistry` replaces the engine's single content slot and routes by media type. `component::resolve` checks each handshake against its declaration, the namespace rule and name conflicts. `check_requirements` covers spec §2.3. Each consumer session and each verification run gets its own `Scope`, ahead of the in-tree components. `engine/hello` declares `components.loaders`. |
| Declaring a slot's type | ADR 0020 and contract spec §5.5: `content-types` on the interaction. The mock encodes a declared slot through its component, the verifier encodes replayed request bodies, both decode under the declared type, and the recorded evidence carries it. |
| Protocol | `config.components` on `consumer-session/create` and `target.components` on `verification/verify`: resolved declarations, loaded before the call answers. |
| CLI | `verify --config` reads `components` from the project configuration. The hooks loader resolves a `file` path against the configuration's directory. `janus` and `janus-engine` both register the WASM loader. |
| SDKs | Behavioural spec: the `content(media-type, document)` primitive, and `components` on `janus`, with three conformance ids and four cases. TypeScript and JVM both implement them and pass 42 of 42 cases. |
| Sample | `GET /orders.csv` on the order service, with a README section on verifying it. |

**M6, as far as a content component goes: met.** One `.wasm`, unmodified, runs in three places:

1. a TypeScript consumer test through the SDK (`sdks/typescript/test/csv-consumer.test.ts`), whose
   consumer parses the mock's CSV and whose contract records `content-types`;
2. the engine's own end-to-end run: consumer, contract, then verification against the sample
   provider (`engine/component-host/tests/csv_component.rs`, 10 tests);
3. `janus verify --config` against the sample provider, with the component declared by a relative
   path (`cli/tests/cli.rs`).

A matcher component (contributed operators, `matcher/apply`, `compare`) is not exercised. The kernel
has no matcher dispatch yet, and plan task 8.4, which contributes fragments and actions, is where it
gets one.

## 2. Findings

Ordered as they were met. "Fixed" means fixed in this change, and names where.

### 2.1 Found by the author, from the documents

1. **The WIT world was not published.** Spec §9.2 gave `call: func(request: list<u8>) ->
   list<u8>` and nothing else: no package, no interface, no world. A component cannot be built
   against a bare function signature; it exports a named interface from a named world, and the host
   links by those names. The only precedent was `spikes/1.4-engine-hosting-plugins/wit/plugin.wit`.
   **Fixed:** `wit/component.wit`, cited from §9.2. The package names the pipe, not the protocol, so
   it never changes.
2. **The engine did not say whether it could host anything.** Spec §10.1 says `engine/hello` MUST
   declare `components.loaders`. A running engine declared no `components` capability at all, not
   even `["in-tree"]`, and no `hooks` capability either (lifecycle-hooks §8.5). An author could not
   learn from the handshake that loading was unimplemented. **Fixed** for components; the `hooks`
   capability is still missing.
3. **Nothing carried a project's components to the engine.** The component configuration "shares a
   file with hook configuration" (§10.2). But the resolved hook document the engine receives has no
   `components` member, `consumer-session/create` carried only the parties, and `verification/verify`
   carried only transports and hooks. A consumer had no protocol path at all. **Fixed:**
   `config.components` and `target.components`, resolved by the loader like everything else (ADR
   0014).
4. **The CSV worked example promised a coercion nothing performs.** It says declaring
   `numeric-lexical` lets `explain` tell a user that `integer` over a CSV column "is checking the
   *spelling* of an integer". The same example's decode produces strings, and `integer` admits
   numbers, so it rejects every value. Shape spec §4.2 degrades `decimal` to `number` and nothing
   more. **Fixed** in the example: a CSV consumer writes `regex("^[0-9]+$")`, which is what this
   component's own tests and the sample's README now show.
5. **Which imports a grant covers was unspecified.** §9.2 says imports beyond the grants are a load
   failure, and also that `std` imports WASI it never uses and linking those to nothing is harmless.
   A `std` wasip2 build imports `wasi:cli/environment`, stdio, `exit`, clocks and
   `random/insecure-seed`. Is `environment` beyond an empty `env` grant? The author could not tell
   whether the component would load. **Fixed:** a per-package table in §9.2. `std`'s imports link to
   nothing or to exactly the grant, sockets and HTTP need `network`, and anything else is refused. The
   loader implements it, and a fixture proves both directions.
6. **Per-decode degradation paths have no specified syntax.** `$[*].total` is copied from the worked
   example; nothing defines it. The engine also drops degradations (finding 14). **Open**, and folded
   into Phase-9 finding 14.
7. **There is no component conformance runner.** §9.4 calls the corpus "the second thing a
   third-party component author runs after reading this document", and nothing runs it. The author
   wrote frame-level tests instead. **Open.** Nothing in-tree runs through two bindings either (ADR
   0012's decision 4 is still a promise).
8. **How an interaction passes decode options is unspecified.** `content/decode` takes an open
   `options` object, and nothing in an interaction or contract can populate it. The component reads
   RFC 4180's `header` media-type parameter, so `text/csv; header=absent` in `content-types` is the
   channel that works today. **Open.** The media-type parameter may simply be the answer.

### 2.2 Found by the engine side, making the component usable

9. **An interaction could not say a body is CSV.** Slots hold bare shapes, and the engine encoded
   every structured served body as `application/json`, with a comment saying "whoever adds a second
   content type resolves this properly". **Fixed:** ADR 0020. `content-types` sits beside `parts`,
   not inside them, because it is additive and not in-band. Contract spec §5.5 and the SDKs' `content`
   primitive go with it.
10. **Member order is lost** (Phase-9 finding 11). The mock serves CSV columns alphabetically.
11. **`encode` never sees the shape** (Phase-9 finding 12). An empty CSV cannot carry its header.
12. **A wrong content type is reported as the wrong size** (Phase-9 finding 13). A provider answering
    JSON where CSV was agreed fails with "Expected at least 1 item(s) … but got 0".
13. **Component errors and degradations never reach the user** (Phase-9 finding 14). A decode error
    becomes an absent value, never `component-failed` as spec §11.3 requires. This has been true for
    JSON since 4.2 and matters now.
14. **In-tree components have no identity.** They never handshake, have no version, and a
    requirement can only be satisfied by a name the embedding declares
    (`Engine::declare_in_tree("content", "json", "1.0.0")`), at a version chosen by fiat.
    `janus upgrade` also writes a bare `http` requirement where spec §2.3 says `<interface>/<name>`.
    Both are accepted as written, and both are **open** for whoever gives in-tree components a real
    handshake (§9.1 says they must have one).
15. **The native binding needed one method that is not an operation.** Routing by media type needs
    to ask a component what it handles. `ContentComponent::handles` is the native projection of the
    handshake's `content-types`, not an operation, so it strains §9.1's "one method per operation".
    It stays until in-tree components handshake (14); then it can be read from the handshake like
    everyone else's.

### 2.3 Found in the SDKs

16. **The SDKs do not read `consumer.janus.yaml`.** Lifecycle-hooks §6.1 names that file for the
    consumer side, and ADR 0014 makes an SDK's embedding layer its loader. Reading YAML would be each
    SDK's first runtime dependency, so the behavioural specification asks for the declarations
    (`components`) and both style guides say so. **Open:** a JSON form, or a shared tiny loader, are
    the options.
17. **A consumer test over CSV can only be one row long.** `each-like`'s cardinality dimension adds a
    `min+1` variant needing a second order, and the SDKs cannot spell the variant-bound state
    parameter that would ask the provider for one (`given` has literal `params` only). The CSV tests
    therefore pin `max: 1`. This is not new; 8.1 is just the first task that minded.

## 3. Measurements (spec §13)

`cargo run --release -p pact_janus_component_host --example call_cost -- <janus_csv.wasm>`, on the
development machine:

| | WASM CSV decode | native JSON decode |
|---|---|---|
| load: read, compile, link, handshake | 20.8 ms, once per session | — |
| 1 row (28 B) | 69 µs / call | 0.5 µs |
| 100 rows (1.3 KB) | 356 µs | 58 µs |
| 1 000 rows (13.9 KB) | 2.9 ms | 0.6 ms |

The fixed cost is the finding: **about 69 µs per call**, against spike 1.4's 5.8 µs for instantiation
alone. The difference is this loader's policy. Every call is a fresh instance *and a fresh
handshake*, so every call pays two frame round trips and two JSON parses on each side. Spec §3.4
allows it and it is the simplest thing that makes loud poisoning trivially true. At test scale
(tens of bodies) it is invisible. At verification scale it is the first lever: an instance per
session, handshaken once and recreated only after a trap, which is what §3.4 names as the default.
Per row, the pipe costs roughly 2.8 µs against native JSON's 0.6 µs, which is spike 1.4's "per-body
serialisation is the pipe's real cost" measured again, with a CSV parser inside it.

## 4. Not done here, on purpose

- **OCI distribution** was task 8.2, since done: the same `.wasm` is pushed with
  `janus component push`, declared by digest, and verified from a registry (ADR 0021).
- **Plan fragments and contributed actions** are task 8.4. `content/compile` is called by nobody yet,
  and the component answers it with no fragment.
- **`match:content-type` in live runs.** The exchange and verification paths still execute without a
  `ContentDetector` (kernel-boundary review, finding 1's resolution). Decoding and encoding route
  through the registry; detection does not.
- **Instances per session.** See §3.
- **The subprocess binding** is task 8.3.

## 5. What the claim looks like now

**For a content component: true, after two additions to the docs and none to the component.** A
third party can write one from the spec, its schemas and its worked example without reading engine
source, provided the spec publishes its WIT (finding 1) and says which imports a grant covers
(finding 5). Both are now written down. Everything else the author needed was already specified, and
the component that came out of the documents is the one the engine now loads unmodified.

**For using one, the engine was the gap, not the documents.** Six things were missing:

- a loader;
- a way to declare components through the protocol;
- a way to declare a slot's type;
- routing by media type;
- the requirement check;
- the `engine/hello` capability.

The spec described all of them, and the design questions about them sit in Phase-9 findings 11–14.
The two that matter most for the next third party are findings 13 and 14. A component author debugs
their component through the messages the engine prints about it, and today those messages talk about
array sizes.
