# Spike 1.1 — Protocol IDL bake-off (evolution first)

Plan task 1.1 · Feeds gate G1 · Findings in [FINDINGS.md](FINDINGS.md) (the durable artifact — code
here is disposable).

## Question

Which IDL/contract mechanism should define (a) the SDK-facing engine protocol and (b) the
plugin-facing component interfaces? These may get different answers. The deciding criterion is
**compatibility under evolution**: an interface addition — above all *adding an enum/variant case* —
must never break an already-compiled third-party component, and minting a whole new interface version
per addition is overhead that punishes evolution.

Context that seeds this spike: protobuf's use in pact-plugins is historical, not an endorsement; WIT
has worked well in commercial use *except* that adding a variant case later broke components compiled
before the case existed. That failure is gauntlet case E1/E4 and gets reproduced systematically here,
including whether WIT feature gates (`@since`) change the outcome.

## Method

1. **Gauntlet definition** — [evolution-gauntlet.md](evolution-gauntlet.md): five interface changes ×
   two directions (old artifact/new counterpart, new artifact/old counterpart).
2. **Paper survey** of candidates against the gauntlet and tooling reality → shortlist (survey lives
   in FINDINGS §1–2).
3. **Model the protocol slice** (`slice/`) in the shortlisted candidates: `add-interaction` with
   structured errors, plus the `verify` event stream — deliberately using the evolution-sensitive
   constructs (enum, variant, record, stream).
4. **Run the gauntlet** on compiled artifacts (`gauntlet/`), per candidate, per surface.
5. **Bindings check**: generate Rust/TypeScript/JVM bindings for the finalists; judge codegen quality
   and errors-as-values ergonomics.

## Candidates

WIT (bare, and with `@since` feature gates) · protobuf · FlatBuffers · Cap'n Proto · Avro · Smithy ·
TypeSpec · document-first hybrid (frozen byte-pipe interface carrying schema-governed JSON/CBOR
documents, LSP-style).

## Tooling status (this machine)

Present: `wasm-tools` 1.239, `cargo-component` 0.21, `wit-bindgen`, `wasmtime`, `protoc`.
Missing (install if shortlisted): `jco`/`componentize-js` (TS↔WIT), `buf` (proto), `smithy` CLI.

## Layout

```
evolution-gauntlet.md   The change catalog and scoring rules
slice/                  Protocol slice modeled per candidate (slice.wit, slice.proto, …)
gauntlet/wit-e1e4/      WIT: old guest vs grown interface (bare + @since-gated)
gauntlet/proto-e1e4/    protobuf: same changes over serialized frames
FINDINGS.md             Survey, gauntlet results, recommendation
```
