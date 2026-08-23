# 0002 — Define the protocol as schema-governed JSON documents over frozen byte-pipes

- **Status**: proposed
- **Date**: 2026-08-23
- **Plan tasks**: 1.1, 1.8 (G1); evidence also from 1.2, 1.3, 1.4, 1.7
- **Evidence**: [spike 1.1 findings](../../spikes/1.1-idl-bakeoff/FINDINGS.md) (gauntlet + bindings
  round), [1.2](../../spikes/1.2-wasm-embedding/FINDINGS.md) (byte-pipe ergonomics),
  [1.3](../../spikes/1.3-subprocess-embedding/FINDINGS.md) (stdio framing),
  [1.4](../../spikes/1.4-engine-hosting-plugins/FINDINGS.md) (plugin surface over the pipe)

## Context

The engine has two IDL-shaped surfaces: the SDK-facing protocol (both ends released by the project,
versions negotiated) and the plugin-facing component interfaces (third-party artifacts compiled
against vN must keep working against engine vN+k without recompilation, indefinitely). The RFC named
WIT and protobuf as candidates and left the choice unresolved. The deciding criterion is
**compatibility under evolution**, because additions — above all new enum/variant cases (event
kinds, error codes, matcher actions) — are the common case on the roadmap, not the exception.

Spike 1.1 ran an evolution gauntlet (add enum case, optional field, operation, event kind, union
alternative; both directions) against compiled artifacts. The candidates failed in strictly ordered
ways: **WIT breaks** — loud, at instantiation, and unfixable (variants/enums are closed; `@since`
feature gates cannot attach to individual cases; item-level gates are authoring-time only);
**protobuf degrades silently** — nothing ever breaks, but unknown enum values masquerade as
`Unspecified` and unknown union/event cases as `None`, and prost (the Rust implementation Janus
would use most) drops unknown fields on re-encode; **schema-governed documents degrade with the
unknown named** — an old reader sees `"component-unavailable"` verbatim and applies policy, which
is the only failure form that turns evolution into a decision rather than an accident. The bindings
round showed schema-to-type generation is adequate in Rust/TS/JVM, with one gap (typify drops
unknown members) fixable by policy. During task 1.7, the failure class showed up in the wild:
pact_ffi 0.5.6 fails to compile against pact_models 1.3.14 because an enum grew a variant.

## Decision

1. **Both surfaces are document-first.** Protocol requests, responses, events, and plugin
   invocations are JSON frames governed by **versioned, project-owned schemas** — the schemas are
   the specified surface (deliverable of design 2.1), authored under open-world rules: open
   vocabularies as strings with advisory `x-known-values` (never closing `enum`), must-ignore
   unknown members, open discriminators, closed envelopes. Capabilities are negotiated at session
   start (LSP model).
2. **Each embedding gets a frozen, never-growing pipe** carrying the same frames: a minimal WIT
   world (`call: func(request: list<u8>) -> list<u8>`) for the WASM embedding; LSP-style
   `Content-Length` stdio framing for the subprocess (with exit-on-stdin-EOF and the `shutdown`
   op as normative engine behaviour, per 1.3); the three-function C ABI as fallback. WIT carries
   the protocol; it does not type it.
3. **Typed views are generated from the schemas** per language (typify, json-schema-to-typescript,
   jsonschema2pojo or equivalents). The Rust protocol-envelope types are project-owned with an
   explicit extra-fields map, because generated Rust types drop unknown members.
4. **A schema-compatibility checker runs in CI** and fails any change that violates the open-world
   rules — governance does the job the type system no longer does.
5. **protobuf is not adopted** for the new protocol. It remains only at the retained pact-plugins
   compatibility boundary (spike 8.3).

## Alternatives considered

- **WIT as the typing layer**: killed by gauntlet E1/E4 — growing a variant/enum strands every
  previously compiled component, in both directions, and WIT's own evolution mechanism cannot
  express case addition at all.
- **protobuf**: survives every gauntlet change but with the worst failure form — silence — on the
  changes that matter most, and prost's unknown-field loss makes the engine itself a lossy
  intermediary. Its evolution machinery would still need the hybrid's capability negotiation to be
  loud, at which point the hybrid wins outright.
- **FlatBuffers / Cap'n Proto**: nothing protobuf doesn't offer for a document-heavy workload;
  thin TS/JVM ecosystems.
- **Avro**: needs the writer schema at read time — wrong shape for an RPC boundary.
- **Smithy / TypeSpec as the wire answer**: heavyweight toolchains and a second indirection layer;
  retained only as a candidate *authoring* layer for the schemas if `smithy-diff`-grade tooling is
  the fastest way to satisfy decision 4.

## Consequences

Easier: every future addition — new event kind, error code, matcher action — is a schema PR plus
policy, not a breaking interface change; the plugin ABI literally never changes; one protocol
definition serves all three embeddings (1.2–1.4 demonstrated the same frames over all pipes).

Harder: cross-boundary mistakes surface at runtime validation rather than compile time (mitigated
by generated types and the conformance corpus); the schema-compat checker must be selected or
built before Phase 2 designs freeze (2.1 deliverable); schema authoring discipline (short
type-shaped `title`s, open-vocabulary rules) must be enforced in review; runtime validation cost
must be watched in the 1.7 trend (validators must embed schemas and disable remote `$ref`
resolution).

Tripwires: if the schema-compat checker cannot make the open-world rules CI-enforceable, or if
validation cost becomes a dominant term in the benchmark trend, revisit — protobuf with explicit
capability negotiation is the named fallback.
