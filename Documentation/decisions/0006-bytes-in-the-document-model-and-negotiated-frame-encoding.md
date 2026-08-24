# 0006 — Model bytes explicitly; make frame encoding a negotiated axis with JSON as the baseline

- **Status**: proposed
- **Date**: 2026-08-24
- **Plan tasks**: 2.1 (feeds 2.5, 2.6, 2.9; tripwire measured by 1.7)
- **Evidence**: [ADR 0002](0002-document-first-protocol-over-frozen-pipes.md) (JSON documents
  over frozen pipes), [spike 1.1 findings](../../spikes/1.1-idl-bakeoff/FINDINGS.md)
  (open-world rules, the governance-not-types principle), pact v4's existing body encoding
  (`pact_models/src/v4/http_parts.rs` — `content` / `contentType` / `contentTypeHint` /
  `encoded`)

## Context

ADR 0002 settled that frames are schema-governed JSON documents. JSON has no byte string: a
JSON string is a sequence of Unicode code points, so an arbitrary octet sequence cannot be one.
Contract testing is precisely the domain that must carry arbitrary octets — protobuf and other
binary content types, compressed and encrypted bodies, binary media, and above all payloads
that are *deliberately* malformed, because a contract test asserts on what the provider
actually sent rather than a repaired transcription of it. A protocol that can only carry
well-formed text cannot express the failures it exists to detect.

The specification as drafted was silent on this, and the silence was load-bearing in the wrong
direction: §8.1 hands bodies to designs 2.2, 2.5 and 2.6 as documents the protocol does not
define, so each would have invented its own base64 convention and the engine would have spent
its life translating between them. That is a decision the protocol has to make once.

Separately, base64 costs a third more bytes and a pass over the data in each direction. Whether
that matters here is **unmeasured** — the benchmark harness (task 1.7) exists to answer it, and
nothing in the plan has yet run a payload big enough to care. So there are two questions with
different answers: what the document model contains (forced, now) and how a frame is written
(an optimisation, on evidence).

## Decision

**1. The document model is the JSON types plus `bytes`.** Bytes are an arbitrary octet
sequence. Their JSON projection is base64 (RFC 4648 §4, standard alphabet, padded), declared in
the schema as `{ "type": "string", "contentEncoding": "base64" }`, lossless in both directions.
Whether a member is text or bytes is fixed for the life of the protocol version. Documents the
protocol carries but does not define (§8.1) MUST use the same projection, so decoding never
depends on which design authored the surrounding document — which also aligns design 2.5 with
pact v4's `encoded: "base64"`, the same projection under an older name.

**2. Frame encoding is a negotiated axis, and v1 defines one value on it.** `json` is
mandatory for both parties; the `engine/hello` exchange is always JSON so a party can always
open the conversation; the host offers what it accepts and the engine names its selection;
absent negotiation, JSON. Because both parties implement the baseline by definition,
**negotiation cannot fail** and needs no error code. An engine MUST be able to run any complete
session in JSON, so a captured trace is always reproducible in text.

**3. No binary encoding is adopted in v1.** Adding one later is additive under §11.2 —
capability-gated, no schema change, no version bump — so the option costs nothing to hold open
and the decision waits for evidence rather than for a release boundary.

Spec text: Engine Protocol specification §2.4 (document model), §3.4 (frame encoding), §5.3
(the `encoding` capability), §11.2–11.4 (evolution rules and the checker).

## Alternatives considered

- **Base64 only, no bytes in the model** (the status quo ante): rejected because it does not
  actually avoid the decision — it relocates it into three other designs — and because without
  a named bytes type there is nothing for a binary encoding to ever exploit, so it forecloses
  option 2 permanently while looking like a deferral.
- **Adopt CBOR (RFC 8949) now** as the second encoding: the strongest candidate by some
  distance — it is the standardised binary encoding of the JSON data model *plus* a native byte
  string, the RFC defines the JSON conversion normatively, it has a diagnostic notation for
  debugging, mature libraries exist in every SDK language, and it brings no schema compiler or
  closed-world codegen, so ADR 0002's reasoning against protobuf does not touch it. Rejected
  **for now on evidence, not on merit**: the saving is unmeasured, every opting-in SDK carries
  a codec against the thin-SDK story, and the conformance suite must then run in both encodings
  or behaviour becomes encoding-dependent — the worst available outcome. Revisit per the
  tripwires below; the spec is shaped so that revisiting costs a capability value.
- **MessagePack**: same shape as CBOR but no standards body, and its JSON mapping is convention
  rather than specification — for a protocol whose whole thesis is a written-down surface, that
  is the wrong trade.
- **Postcard / bincode**: schema-dependent and Rust-shaped — the same bytes cannot be decoded
  without the Rust types, which is fatal for polyglot SDKs.
- **Out-of-band payload transfer** (handles, file paths, shared memory): the correct answer if
  payload *copies* ever dominate — on the WASM pipe `call(list<u8>) -> list<u8>` copies the
  entire frame in both directions, so a denser encoding saves the 33% and never the copy.
  Rejected for v1 because it introduces a resource with a lifetime, colliding head-on with
  "sessions are the only resource" (ADR 0002, spec §1). Named here so that a future
  payload-size problem is diagnosed before an encoding is blamed for it.

## Consequences

Easier: one decoding rule holds across every document the protocol carries, including the ones
it does not define; the engine can carry a non-UTF-8 or malformed body at all, which it
previously could not; a binary encoding becomes a capability value rather than a protocol
version; JSON-always keeps the wire debuggable with a pipe and a text editor and keeps
[`examples/`](../specs/engine-protocol/examples/) authoritative rather than illustrative.

Harder: a member can never move between text and bytes, so the choice must be right per member
— the compatibility checker enforces it, because `contentEncoding` is annotation-only in draft
2020-12 and no validator would. Any SDK that later offers a second encoding owes the
conformance suite a run in each.

Committed to: JSON as a permanent mandatory baseline on every pipe, and to encodings never
carrying semantics — an observable difference between two encodings of the same document is a
bug by definition.

Tripwires: adopt a binary encoding when the 1.7 benchmark trend shows base64 or frame size on a
hot path — the plausible ones are consumer sessions serving many variants of one interaction,
and fixtures with large binary bodies — or when an SDK reports its language's base64 as the
bottleneck. If instead the cost turns out to be the frame *copy* rather than its width, the
answer is out-of-band transfer above, and this ADR should be superseded rather than amended.
