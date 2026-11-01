# 0063 — Carry a contract across the engine boundary as its text, and pin how numbers are written

- **Status**: accepted
- **Date**: 2026-11-01
- **Plan tasks**: S2.1 ("the engine, not the SDK, writes the contract's canonical bytes"); S2.11,
  which this splits off
- **Evidence**: Phase 9 finding 6 ([RFC feedback](../prototype/rfc-feedback.md) §5: "two SDKs can
  write different bytes for the same content"); `sdks/typescript/src/janus.ts`, which writes
  `JSON.stringify` of the parsed `contract`; `EngineClient.okMemberBytes` in `sdks/jvm/sdk`, which slices the
  member's raw bytes out of the frame; `sdks/typescript/src/verifier.ts`, which `JSON.parse`s each pact
  before sending it; conformance case `lifecycle/finalise-writes-contract`, whose note leaves number
  spellings untested because an SDK "can only reconstruct those"; `janus upgrade` on a pact whose
  body holds `1.0`, `9007199254740993`, `1e21`, `1e16` and `1.10`; pact-reference, whose serde_json
  has no `arbitrary_precision`

## Context

The contract-file spec defines canonical bytes (§2.4): compact UTF-8, `$format` first, members in a
specified order, one trailing LF. The same content must be the same bytes, because a broker
deduplicates on a content hash. The kernel writes them (`contract::write_canonical`). But no contract
leaves the engine as bytes. `consumer-session/finalise` and `upgrade/pact` answer with the contract as
a document inside a JSON frame, and each host writes it again.

- **The TypeScript SDK changes the contract it writes.** It parses the frame and writes
  `JSON.stringify(contract)`, and JavaScript numbers are doubles with their own spelling. `1.0` is
  written `1`, `1e16` as `10000000000000000`, and an integer above 2^53 is corrupted:
  `9007199254740993` becomes `9007199254740992`. That is a different contract, not a different
  spelling of the same one.
- **The JVM SDK writes the engine's bytes, by a trick.** It finds the `contract` member's byte range
  in the raw frame and copies it out. That holds only while frames are JSON. Under CBOR (gate E,
  ADR 0006) there are no JSON bytes to copy, and a third SDK in a language without a position-reporting
  parser would have to hand-write one.
- **The same loss happens on the way in.** The TypeScript verifier `JSON.parse`s each pact file
  before sending it inline, so the provider is verified against an example the parse already
  changed.
- **§2.4 never says how a number is written.** The engine writes what serde_json writes: `1.0`,
  `1e+16`, `5e-324`, `-0.0`, and `1.10` as `1.1`. That is deterministic, but nothing pins it. A
  serde_json release that changed a spelling would give every affected contract a new hash at the
  broker, and no test would notice.

The engine, with serde_json and no `arbitrary_precision`, reads an integer that fits in 64 bits
exactly and any other number as the nearest double. pact-reference reads them the same way. That
rounding is not this decision's problem. A second, different rounding in each SDK is.

## Decision

1. **A capability, `contract-text`, declared by both sides** (engine-protocol spec §5.3), value
   `{}`. The engine declares that it reads a contract's text wherever a request carries a contract
   (decision 3). A host declares that it wants contracts back as text (decision 2). An engine that
   gets no declaration answers exactly as protocol version 1 does today (§5.2).
2. **To a host that declared it, an outbound contract is its canonical text.** `consumer-session/finalise`
   and `upgrade/pact` answer with `contract-text` in place of `contract`. It holds the contract's
   canonical bytes (contract-file spec §2.4) as a string, trailing LF included, and is present exactly
   when `contract` would be, so finalise still withholds a contract by leaving it out. A host writes
   the string's UTF-8 encoding to the file, and adds and removes nothing. A JSON frame carries it as a
   JSON string and a CBOR frame as a text string, exact under either. It replaces `contract` rather
   than sitting beside it because a contract is the largest thing a frame carries, and twice is
   waste.
3. **Wherever a request carries a contract or a v1–v4 pact, it may carry the document's text.** That
   covers the `inline` source's `contracts` in `verification/verify`, `upgrade/pact`'s `pact`, and
   `subsumption/check`'s `contract`. The engine reads a string there as it reads a file: tolerantly,
   whatever its layout (§2.4, "a reader MUST NOT assume its input is canonically formatted"). A
   contract and a pact are always objects, so a string is never ambiguous. A host reads the file and
   sends its text unparsed.
4. **How a number is written is part of §2.4, and a corpus pins it.**
   - An integer with no fraction or exponent that fits in a signed or unsigned 64-bit integer is
     written as its digits.
   - Any other number is the nearest IEEE 754 double, written with the fewest significant digits
     that read back as that double. At least 1e-5 and below 1e16 in magnitude, it is positional, with `.0`
     when there is no fraction (`100.0`, `0.00001`). Outside that range it is digits, an optional
     fraction, then `e`, a sign and an exponent (`1e+16`, `1.5e-7`, `5e-324`). Zero is `0.0` and
     negative zero `-0.0`, which is also how `-0` is read.
   - An author's spelling is not kept (`1.10` is `1.1`, `1E2` is `100.0`). Nor is precision beyond
     a double, outside 64-bit integers, as in pact-reference.

   That is what the engine writes now, so no contract the engine has written changes. A contract
   corpus case holds every spelling above. A dependency upgrade that moves one fails CI, and moving
   one on purpose needs a change file, since every contract containing it gets a new hash.
5. **The SDKs write the text and parse no contract.** `finalise` writes `contract-text` as given,
   with no LF of its own, since the text ends with one. The JVM SDK's frame slicing is deleted. Both
   verifiers read each file as UTF-8 and send its text. A file that is not a contract becomes the
   engine's document error, with positions, instead of a parse error from the SDK. The CLI declares the
   capability too, and `janus upgrade --write` writes `contract-text`. The behavioural spec's
   `finalise` and `verify` say so, and both SDKs and the CLI declare `contract-text` in their hello.
6. **The conformance suite checks the bytes, not the content.** `lifecycle/finalise-writes-contract`
   gains numbers JavaScript would respell (`1.0`, `9007199254740993`, `1e+16`), and its note stops
   excusing them. A live case records the same interactions in each SDK against the real engine,
   each report carries the SHA-256 of the contract that SDK wrote, and
   `pact_janus_conformance check` fails when two SDKs' digests differ. That is S2.1's exit,
   "byte-identical because neither wrote them", made into a check.
7. **Merging contracts is S2.11.** pact-js and pact-jvm merge every test file's interactions into
   one pact file under a file lock. A Janus SDK overwrites, so with several test files for one
   consumer and provider, the last one written wins. Merging is logic an SDK must not own, so the
   engine's host will write and merge files. Which interactions are the same, what a conflict is,
   and the lock are that item's ADR, not this one.

## Alternatives considered

- **Slice the member out of the frame in every SDK, as the JVM SDK does.** It works only for JSON
  frames, and it needs a JSON parser that reports byte positions, which JavaScript does not ship.
- **`contract-text` beside `contract`, always.** It needs no capability, but every finalise carries
  the contract twice.
- **Make `contract` a string.** That changes a protocol version 1 member's type, which §5.2 makes a
  new protocol version, for one member a capability can carry.
- **The engine writes the file.** It needs a filesystem on the host's side and the merge rules
  decided first. That is S2.11, and decision 2's text is what such a host would write anyway.
- **A lossless JSON parser in the TypeScript SDK.** That is a dependency, the SDK would still
  re-serialise, and decision 4's spelling rule would be reimplemented in every language.
- **serde_json's `arbitrary_precision` in the kernel**, so a number keeps its spelling and precision.
  But every number comparison the matcher makes would change, and a v1–v4 verdict has to follow
  pact-reference's doubles.

## Consequences

Easier: two SDKs write the same bytes because neither writes a contract. The TypeScript SDK stops
corrupting large integers, on the way out and on the way in. The JVM SDK loses its slicing code, and
the next SDK writes a string to a file. Gate E's CBOR no longer breaks anything a host does with
contracts.

Harder: number spelling is now part of the contract format. A dependency that moves it fails the
corpus, and a deliberate move is a change file. A host that does not declare `contract-text` gets a
document, as before, and can still reconstruct it wrongly.

Committed to: the `contract-text` capability and member, the string form of a contract in a request,
and §2.4's rule for numbers.

Tripwires:

- **A contract needs a number kept beyond a double's precision**: money written as JSON numbers, or
  identifiers wider than 64 bits. Revisit `arbitrary_precision`, decided with its effect on matching.
- **Protocol version 2.** A document-form `contract` is retired with it, and the capability is
  dropped.
