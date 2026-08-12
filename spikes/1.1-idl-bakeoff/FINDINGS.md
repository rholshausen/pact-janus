# Spike 1.1 findings — Protocol IDL bake-off

Status: **in progress** — survey done; WIT (E1/E4, E3) and protobuf (E1–E5) gauntlet legs run
(results §3); still to do: document-schema leg, bindings round for the finalists. Method and change
catalog: [README.md](README.md), [evolution-gauntlet.md](evolution-gauntlet.md).

## 1. Framing: two surfaces, one hard requirement

The engine has two IDL-shaped surfaces with different evolution economics:

- **SDK-facing engine protocol.** SDK releases pin an engine version; both ends are released by the
  project. Version negotiation at session start is acceptable and planned (protocol spec 2.1).
- **Plugin-facing component interfaces.** Third-party artifacts compiled against interface vN must
  keep working against engine vN+k *without recompilation*, indefinitely. Additive changes — above
  all new enum/variant cases — are the common case: every new core matcher action, event type, or
  error code is exactly such an addition. A model that forces either "break old plugins" or "new
  interface version per addition" taxes the roadmap forever.

The requirement that decides the bake-off: **growth of a sum type must be invisible to artifacts
that predate it** (or loudly negotiable), in both directions of the gauntlet.

## 2. Candidate survey (paper round)

### WIT / component model

- **For**: the native language of the WASM component embedding — the 1.2/1.4 spikes speak it no
  matter what; rich types (`result`, `variant`, `resource`, streams in P3) that model
  errors-as-values exactly as the RFC wants; one IDL for both surfaces if it works.
- **Against (the reported break)**: variants and enums are *closed* types. Component instantiation
  type-checks structurally and current runtimes apply little/no subtyping, so a counterpart compiled
  when a variant had three cases fails against a four-case view — observed in commercial use, being
  reproduced here as E1/E4 (§3). Package versions mediate resolution, and in the 0.x range semver
  compatibility windows are narrow, so "just bump the version" fragments the plugin fleet.
- **Maybe**: WIT has feature gates (`@since(version = …)`, `@unstable(feature = …)`) designed for
  additive evolution (WASI itself uses them). Whether gates apply cleanly to individual enum/variant
  *cases*, and whether tooling honours them at instantiation time rather than only at
  WIT-resolution time, is exactly what the gated leg of the gauntlet tests.
- **Note**: even if WIT loses the protocol-typing job, a *minimal, frozen* WIT world (bytes in,
  bytes/events out) remains necessary as the WASM embedding's pipe — WIT is unavoidable as plumbing;
  the question is whether it is the *schema*.

### protobuf (proto3)

- **For**: evolution semantics are the strongest of any typed candidate and are battle-tested:
  unknown fields are preserved (in protobuf-java/C++ — see finding 8 for prost), enums are open
  (unknown values are retained as integers), new `oneof` alternatives surface to old readers as an
  unset/unknown field — old artifacts keep working by design across E1–E5. Mature codegen
  everywhere (prost/Rust, protobuf-java, protobuf-es/TS). pact-plugins compatibility is a bonus.
- **Against**: its use in pact-plugins is historical, not an endorsement — and it shows: field-number
  bookkeeping, `oneof` ergonomics for errors-as-values are clunky, no resource/handle concept
  (sessions become integer handles by convention), and it drags a gRPC-shaped worldview. Wire-level
  openness has a flip side: E1 in direction D-a is *silent* — an old plugin receives an enum value it
  has no name for and sees a bare integer; nothing tells it (or the user) that anything is missing
  unless the protocol adds explicit capability negotiation anyway.
- **Encoding note**: protobuf-the-encoding can run over any pipe (stdio frames, a byte-pipe WIT
  world, C ABI) — choosing it does not imply gRPC.

### FlatBuffers / Cap'n Proto

Numbered-field evolution similar in spirit to protobuf (old readers skip unknown fields; unknown
enum ordinals survive). FlatBuffers' builder API is poor ergonomics for a document-heavy protocol;
Cap'n Proto is technically elegant but its TS and JVM ecosystems are thin — a maintenance bet the
project shouldn't make. Neither brings anything protobuf doesn't for this workload (zero-copy is
irrelevant when payloads are JSON documents). **Paper-eliminated unless protobuf stumbles in the
bindings round.**

### Avro

Evolution via reader/writer schema resolution requires the writer schema at read time — natural for
data-at-rest, awkward for an RPC boundary, and it has no operation/service concept. **Paper-
eliminated** for the protocol; possibly interesting much later for pact-file-embedded shapes.

### Smithy / TypeSpec (authoring + governance layers)

Not wire formats — IDLs that *emit* other artifacts (Smithy → its own protocols/codegen,
TypeSpec → JSON Schema/OpenAPI/protobuf). Two genuinely interesting properties: both model
operations/errors/streams first-class, and Smithy ships **`smithy-diff`**, which makes "this change
breaks existing consumers" a CI failure — the governance mechanism the plugin surface needs
*regardless of wire format*. Costs: heavyweight toolchains (Smithy is Gradle/Java-centric), a second
layer of indirection, and Rust-side codegen (smithy-rs) is AWS-flavoured. **Carried forward as a
governance layer option**, not as the wire answer.

### Document-first hybrid (LSP model)

The transport interface is tiny and *frozen* (create-session / call-with-document / poll-event, as
bytes) in whatever each embedding natively speaks (minimal WIT world, stdio framing, 3-function C
ABI — the RFC's three embeddings already imply this shape). All real structure lives in versioned
**document schemas** (JSON Schema or CDDL) with open-world rules by construction: must-ignore
unknown members, enums modeled as open strings + `x-known-values`, capabilities negotiated at
session start (LSP's model, which evolved 3.x for a decade without breaking clients).

- **For**: E1–E5 pass *by construction* on the wire — evolution is policy, not type-system luck; the
  plugin ABI literally never changes; matches the RFC's "documents in, documents and events out"
  and the interaction-spec/plan documents that dominate the payload anyway; schemas can generate
  types per language (typify/schemars for Rust, json-schema-to-typescript, jsonschema2pojo) so SDK
  authors still get compile-time types.
- **Against**: gives up cross-boundary compile-time checking — mistakes surface at runtime
  validation, not at link time; runtime validation cost (measure in 1.7/9.1); JSON Schema's own
  evolution/diff tooling is weaker than `smithy-diff` (mitigation: author schemas in
  TypeSpec/Smithy and emit JSON Schema, or adopt a schema-compat checker in CI); "documents
  everywhere" can hide sloppy contracts unless the schemas are treated as the specified surface
  with the same rigour the plan already demands for the plan grammar (2.4).

## 3. Gauntlet results

### WIT — E1+E4 (variant/enum growth), wasmtime 35 / cargo-component 0.21

Setup (`gauntlet/wit-evolution/`): guest components compiled against `pact:gauntlet@0.1.0` (3-case
variant, 4-case enum) and against the grown view; hosts with bindings for the baseline, the
in-place-grown view (same package version — "nobody bumped"), and a `@since`-gated 0.2.0 view.

| Direction | Host bindings | Guest artifact | Result |
|---|---|---|---|
| sanity | v0.1.0 baseline | old (v0.1.0) | OK — events and enum round-trip |
| **D-a critical** | v0.1.0-grown (E1+E4 in place) | old (v0.1.0) | **BREAK** — `type mismatch with results: expected variant of 4 cases, found 3 cases` at typed-binding acquisition |
| D-b | v0.1.0 baseline | new (grown, emits new case) | **BREAK** — symmetric: `expected variant of 3 cases, found 4 cases` |
| D-a gated | v0.2.0 `@since`-gated cases | old (v0.1.0) | **NOT EXPRESSIBLE** — the WIT parser rejects gates on enum/variant *cases* (`expected an identifier or string, found '@'`); item-level gates (e.g. on a function) parse fine |

Findings, WIT leg (wasm-tools 1.239 / wasmtime 35 / cargo-component 0.21):

1. **The reported break reproduces exactly**, in both directions, without any version bump — a
   same-version in-place type growth is enough. The failure is loud and early (instantiation-time
   type error naming the arity mismatch), which is the best possible *form* of failure, but it is
   still a hard BREAK: an engine that adds one event type strands every previously compiled plugin.
2. **Feature gates do not cover sum-type growth.** `@since` attaches to items (functions, types,
   interface members) but not to individual enum/variant cases, so WIT's designed evolution
   mechanism cannot express "this case arrived in 0.2.0" at all. Case addition — the single most
   common evolution in a matcher/event vocabulary — has no gated path. (Gated *operation* addition,
   E3, is expressible and still needs a runtime test: does a host built against the gated view
   instantiate an old guest that lacks the gated export?)
3. Consequence for the plugin surface: bare WIT typing of open vocabularies (event kinds, error
   codes, matcher/action kinds) is disqualified by E1/E4 unless the design confines variants to
   truly closed sets and moves everything open into documents or strings — which is precisely the
   document-first hybrid.

### WIT — E3 (operation growth), same setup

`explain: func(spec: string) -> string` added to the interface; the old guest predates it. Hosts:
generated bindings against the in-place-grown view (`host-grown-op`), generated bindings against a
`@since(0.2.0)`-gated view (`host-gated-op`), and a hand-rolled feature-detecting host using the
untyped `Val` API (`host-probe`) that treats `explain` as an optional capability.

| Leg | Host | Guest artifact | Result |
|---|---|---|---|
| **D-a, generated bindings** | grown-op view (@0.1.0) | old | **BREAK** — `instance export 'pact:gauntlet/events@0.1.0' does not have export 'explain'` at instantiation |
| control | grown-op view | new (exports explain) | OK |
| D-a, gated | gated view (@0.2.0) | old | **BREAK** — `no exported instance named 'pact:gauntlet/events@0.2.0'`; resolution fails on the versioned interface name before the gate is ever consulted |
| **D-a, feature-detect** | probe (untyped API) | old | **NEGOTIATED** — baseline ops work; `explain` absence detected and degraded gracefully |
| feature-detect control | probe | new | OK — probe finds and calls `explain` |
| D-b | baseline bindings (@0.1.0) | new | **PASS** — extra export ignored |

Findings, E3 leg:

4. **Operation growth is survivable in WIT — but only by abandoning generated bindings.** The type
   system is not the obstacle here (no types changed); *bindings discipline* is. wasmtime's
   `bindgen!` resolves every export eagerly at instantiation, so an old plugin missing the new op
   fails outright. The untyped-API probe host implements "new ops are optional capabilities"
   cleanly — which is capability negotiation built by hand at the export-name level. If WIT is kept
   for plugin interfaces, hosts must either probe like this or put every new operation in a new,
   optionally-resolved interface.
5. **Item-level gates are authoring-time, not runtime.** The gate never participates at runtime:
   interface resolution matches versioned export names first (`events@0.1.0` ≠ `events@0.2.0`, and
   0.x minors are not semver-compatible), and an old artifact's exports were fixed when it was
   compiled. `@since` is a mechanism for publishing one WIT document that can be *viewed* at
   several versions — useful for spec authoring (WASI uses it for that), useless for keeping an old
   binary loadable.
6. Combined with E1/E4: the export/expectation asymmetry means plugins may safely *offer* more than
   the engine knows (D-b passes), but every engine-side expectation growth needs explicit optionality
   handling — and for sum types there is no such handling at all. An engine that hosts long-lived
   third-party WIT plugins therefore ends up hand-building the negotiation machinery the
   document-first hybrid has by construction, while still being unable to grow its variants.

### protobuf — E1–E5 over serialized frames (prost 0.13, no gRPC)

Setup (`gauntlet/proto-evolution/`): the same proto package compiled at two schema versions into one
binary — v1 is "the old plugin's view", v2 "the grown engine's view"; frames encoded under one view,
decoded under the other, both directions.

| Change | D-a (old reader, new frame) | D-b (new reader, old frame) |
|---|---|---|
| E1 enum value | **PASS (silent)** — decodes; raw `i32 = 5` survives (even round-trips through the old view, since the field itself is known); but the typed accessor reports `Unspecified` with no unknown-value signal. Detectable only if code hand-checks `try_from` on the raw value | PASS |
| E2 optional field | **PASS** — invisible to the old view; **but prost drops unknown fields on re-encode**, so a Rust intermediary loses the new field in pass-through (protobuf-java/C++ preserve unknown fields; prost's omission is long-standing) | PASS — absent → `None` |
| E3 new operation | **NEGOTIATED (explicit)** — old callee decodes `op = None` and can answer "unsupported operation"; the rejection is codeable, unlike a link error | PASS |
| E4 event case | **PASS (silent-worst)** — `event = None`, indistinguishable from an empty event; a stream consumer just sees nothing | PASS |
| E5 union alternative | **PASS (silent-worst)** — same `None` shape as E4 | PASS |

Findings, protobuf leg:

7. **Nothing ever breaks — and that is both the feature and the trap.** No change strands an old
   artifact (no link-time coupling exists to strand it), but every D-a "pass" on vocabulary growth
   is *silent by default*: unknown enum values masquerade as `Unspecified`, unknown event/union
   cases as `None`. E3 is the only naturally loud case, and only because a request/response pairing
   gives the old side somewhere to hang an explicit rejection. Making the silent cases loud requires
   exactly the capability-negotiation/validation machinery the document-first hybrid needs anyway —
   protobuf just makes the gap easy to not notice.
8. **prost does not preserve unknown fields.** Confirmed: a new field vanishes when a frame
   round-trips through a prost-compiled old view (unknown *enum values* survive, being known
   fields). Any Rust intermediary — and the engine is Rust — is lossy for pass-through under
   protobuf unless this is engineered around. The survey's textbook claim about unknown-field
   preservation is implementation-dependent and false for the implementation Janus would use most.
9. Scoring under the gauntlet: protobuf clears the plugin-surface bar (old artifacts keep working)
   but with the *worst possible failure form* — silence — on the changes that matter most (E1, E4,
   E5). The gauntlet's own scoring note applies: a loud failure beats a silent one within the same
   grade.

## 4. Emerging picture (to be validated, not yet a recommendation)

The survey suggested — and the WIT gauntlet leg now supports — a split answer: a **document-first
protocol** over frozen per-embedding pipes for both surfaces, with **typed structure only where it
cannot break** (closed control-flow shapes like `result`; never open vocabularies), and a governance
tool (schema diff in CI) doing the job that closed type systems fail at and open ones silently skip.
WIT remains the plumbing of the WASM embedding either way, but E1/E4 BREAK plus gates-can't-gate-cases
rules it out as the schema for anything that grows, and E3 shows even operation growth forces hosts
into hand-built feature-detection — negotiation machinery the document model has by construction. protobuf remains the strongest *typed* fallback if document-schema tooling disappoints in the
bindings round — its gauntlet leg confirms old artifacts never break, at the price of silent
degradation on exactly the changes that matter (findings 7–9), plus a prost-specific pass-through
loss the engine would have to engineer around. The document leg and bindings round must still run
before this hardens into the G1 ADR.
