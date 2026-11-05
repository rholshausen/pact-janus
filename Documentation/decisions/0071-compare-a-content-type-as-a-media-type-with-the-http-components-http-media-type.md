# 0071 — Compare a `Content-Type` as a media type, with the HTTP component's `http:media-type`

- **Status**: accepted
- **Date**: 2026-11-05
- **Plan tasks**: S2.16 (S3.3's media-type half and S3.4, brought forward by S2.5b)
- **Evidence**: `legacy:header-value` (plan grammar §4.4, `engine/kernel/src/plan/interpret.rs`) and the
  pact-specification cases it passes (`matches content type with charset`, `... with parameters in
  different order`, `content type parameters do not match`); S2.5b's `consumer/junit5` tests, four of
  which fail on it; the 16 entries in `corpora/verifier/upgrade-disagreements.json` that cite S3.4;
  RFC 6838 §4.2 and RFC 9110 §8.3.1;
  [ADR 0057](0057-the-http-component-runs-http-status-class-as-an-in-tree-matcher.md), whose mechanism
  this reuses

## Context

A v1–v4 pact names a `Content-Type` header, usually with no matching rule. Today's verifiers and mocks
compare it as a media type: the same type and subtype, and each parameter the pact names present in
the actual value with the same value, other parameters allowed. The legacy compiler does the same
(`legacy:header-value`), so `janus verify` of a pact agrees with them.

`janus upgrade` cannot say that in the shape language, which has no operator for it. It writes the
header as `equality` and reports the narrowing as a judgement (S1.25). A contract upgraded from a pact
therefore fails a reply whose `Content-Type` adds a charset the pact does not name, which the pact
passes. S1.1's harness records 16 such disagreements, and S3.4 was to end them once S3.3 gave the
shape language a media-type operator.

S2.5b's pact-jvm facade made that urgent. Its contracts are what `upgrade` writes from pact-jvm's
interactions, and pact-jvm's builders write `application/json; charset=UTF-8` where its users' clients
send other spellings, and the reverse. Consumer tests that pass on pact-jvm fail on the facade, at the
mock, on a parameter nobody asserted.

## Decision

1. **The built-in HTTP component contributes `http:media-type`.** It runs as `http:status-class` does,
   as an in-tree matcher the native embeddings register (ADR 0057), and is a component operator the
   kernel does not interpret (shape spec §3.5). Its one member, `type`, is the media type expected.
   It admits a text value that is a media type whose type and subtype equal the expected's, compared
   case-insensitively (RFC 6838 §4.2), and that has each of the expected's parameters with an equal
   value, the names and values compared case-insensitively and quotes and surrounding whitespace
   ignored. Parameters in another order, and parameters the expected type does not name, are allowed.
   Anything else fails the node: `Expected a media type matching 'application/json' but was
   'text/plain'`.
2. **This is `legacy:header-value`'s media-type comparison, made a shape.** The one difference is
   case: with no parameters on either side, `legacy:header-value` compares the values exactly, and
   `http:media-type` compares the type and subtype case-insensitively, as RFC 6838 says. A contract
   upgraded from a pact therefore admits a little more than the pact does, `Application/JSON` for
   `application/json`, and nothing less.
3. **`janus upgrade` writes a `Content-Type` with no matching rule as `http:media-type`,** in a
   request, in a response, and in a message's metadata (`contentType`, as ADR 0067 decision 3 reads
   it). Its `type` and its example are the pact's value, and the interaction requires the `http`
   component, so a verifier without it says `component-unavailable` naming it rather than matching
   less. Its finding becomes a `note`, as the status class's is. A `Content-Type` with a rule keeps
   its rule, and other headers are still `equality`: list-valued headers stay in S3.3.
4. **The legacy compiler does not change.** A pact verified as a pact already compares the header with
   `legacy:header-value`.

## Alternatives considered

- **Make media types a core operator.** The kernel would know what a media type is, which the kernel
  boundary forbids (AGENTS.md: the kernel knows nothing about HTTP or JSON). The HTTP component already
  runs a matcher for exactly this reason.
- **Copy `legacy:header-value` exactly, case included.** It would match today to the byte, but carry a
  case rule RFC 6838 rejects into a shape a contract keeps for good. No pact fails on it.
- **A regular expression, as the body-type check uses (ADR 0047).** `upgrade` already writes one where a
  pact names no header (`media_type_pattern`). A pattern for a type with parameters lists every order
  of them, and reads poorly in a contract a person reviews. A named operator says what it means, and
  its message says what was expected.
- **Leave it to S3.3.** Consumer tests that pass on pact-jvm would fail on the facade until stage 3,
  which is S2.5b's exit and the stage's.

## Consequences

Easier: a contract upgraded from a pact, or recorded by a facade, accepts the `Content-Type` the pact
does. The 16 ledger entries go, and the pact-js examples' provider tests verify their contracts.

Harder: an upgraded contract that names a `Content-Type` requires the `http` component, which every
native embedding has. The WASM engine has none, and a plan reaching `http:media-type` there fails the
node by name, as `http:status-class` does.

Committed to: `http:media-type` and its `type` member, in contracts that outlive this engine, and its
comparison.

Not decided here: list-valued headers, and how the native DSL spells a media type (S3.2, S3.3).

Tripwire: if a provider relies on today's case-sensitive comparison of a `Content-Type` with no
parameters, decision 2 is revisited.
