# Engine Protocol specification (v1, draft)

Plan task: **2.1**. Status: **draft — under review**.

This document specifies the protocol between a Pact Janus SDK (or the CLI, or any other host)
and the Pact Janus engine: the session model, the operation set, event and stream semantics,
the structured error taxonomy, version and capability negotiation, and the compatibility policy.

The protocol's architecture is fixed by [ADR 0002](../../decisions/0002-document-first-protocol-over-frozen-pipes.md):
schema-governed JSON documents ("frames") carried over frozen, never-growing byte-pipes, one
pipe per embedding ([ADR 0003](../../decisions/0003-embedding-priority-per-language.md)). The
JSON Schemas under [`schemas/v1/`](schemas/v1/) are the **specified surface** — this prose
defines their semantics; the schemas define their shapes. When prose and schema disagree, that
is a bug in this specification; file it rather than inferring precedence.

Evidence and obligations feeding this design come from spikes
[1.1](../../../spikes/1.1-idl-bakeoff/FINDINGS.md) (evolution gauntlet, bindings round),
[1.3](../../../spikes/1.3-subprocess-embedding/FINDINGS.md) (subprocess lifecycle and framing)
and [1.5](../../../spikes/1.5-message-transport-shape/FINDINGS.md) (transport shape,
passive/emissive interactions).

## Contents

1. [Scope and conformance](#1-scope-and-conformance)
2. [The schema regime](#2-the-schema-regime)
3. [Byte-pipes and framing](#3-byte-pipes-and-framing)
4. [Frames: the protocol envelope](#4-frames-the-protocol-envelope)
5. [Handshake, version and capability negotiation](#5-handshake-version-and-capability-negotiation)
6. [Engine lifecycle: shutdown and liveness](#6-engine-lifecycle-shutdown-and-liveness)
7. [Sessions](#7-sessions) *(drafted in a later chunk)*
8. [Operation set](#8-operation-set) *(drafted in a later chunk)*
9. [Events and streams](#9-events-and-streams) *(drafted in a later chunk; ADR 0005)*
10. [Error taxonomy](#10-error-taxonomy) *(drafted in a later chunk)*
11. [Compatibility policy](#11-compatibility-policy) *(drafted in a later chunk)*

---

## 1. Scope and conformance

The key words **MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT** and **MAY** are to be
interpreted as described in RFC 2119.

Two conformance roles:

- **Engine**: the party that implements the operations — the Janus kernel behind any of the
  three pipes.
- **Host**: the party that invokes them — an SDK's embedding layer, the CLI driving a local
  engine, or a test harness.

The protocol is *coarse-grained and document-oriented*: a host submits complete documents (an
interaction specification, a verification request) and receives complete documents and events
back. Orchestration lives inside the engine. Two rules from the RFC are load-bearing and appear
throughout:

- **Errors are values.** Every operation returns either a result document or a structured
  error document (§10). Panics, exceptions and traps MUST NOT cross the pipe; an engine that
  cannot produce a result MUST produce an error frame instead.
- **Sessions are the only resource.** No operation creates a handle that requires its own
  cleanup call; everything a session allocates is released when the session ends (§7). There
  are no free/destroy/close operations other than session finalisation and engine `shutdown`.

Out of scope here: the component interfaces behind the engine (design 2.6), the interaction
specification and shape language carried *inside* frames (designs 2.2/2.4 — this spec treats
those documents as opaque objects with their own schemas), and the pact file format (2.5).

## 2. The schema regime

### 2.1 Schemas are the surface

Every frame and every operation body is governed by a JSON Schema (draft 2020-12) under
`schemas/`, versioned by directory (`schemas/v1/`, `schemas/v2/`, …). The directory version is
the **protocol version** negotiated in the handshake (§5). Within one protocol version, schemas
evolve **additively only** (§11); a change the compatibility checker (§11.4) classifies as
breaking requires a new protocol version directory, which is expected to be rare by design —
the open-world rules below exist so that vocabulary growth, the common case, is never breaking.

### 2.2 Open-world authoring rules

These rules are normative for every schema in this specification, and for every schema a
component or plugin contributes under its own namespace. They are the mechanism by which an old
party degrades *with the unknown named* instead of breaking or silently misreading (spike 1.1,
findings 10–12). The compatibility checker enforces them in CI.

1. **Open vocabularies are strings, never `enum`.** Any set expected to grow — operation
   names, event kinds, error codes, action names, capability names — is typed as a plain
   `string` with the advisory annotation **`x-known-values`** listing the values known at
   authoring time. The `enum` keyword is reserved for genuinely closed sets (none exist in v1).
2. **Unknown members are ignored, and preserved where practical.** A reader encountering an
   object member it does not know MUST ignore it (never fail) and SHOULD preserve it when
   re-encoding the document. Schemas MUST NOT set `additionalProperties: false`.
3. **Discriminators are open.** A reader encountering an unknown value in a discriminator
   position (frame `type`, event `kind`, error `code`) applies *policy* — skip, warn, surface —
   chosen with the unknown value in hand; it MUST NOT crash and MUST NOT silently coerce the
   value to a default.
4. **Envelopes are closed control-flow shapes.** The frame envelope (§4) and result/error
   pairing are small, project-owned structures whose *required* members are fixed here. New
   envelope members may be added additively (rule 2 covers old readers), but structural change
   to the envelope is a protocol-version bump. Typed structure lives only where it cannot
   break; everything that grows lives in open vocabularies and payload documents.
5. **Titles are short and type-shaped.** Schema `title`s name the type (`RequestFrame`,
   `EngineError`) because binding generators derive type names from them (spike 1.1,
   finding 15). Prose belongs in `description`.
6. **Schemas are self-contained.** No remote `$ref`; validators MUST embed the schemas and
   MUST NOT resolve references over the network (spike 1.1, finding 13).

### 2.3 Typed views

Language bindings are *generated from* the schemas (typify for Rust,
json-schema-to-typescript, jsonschema2pojo or equivalents) and are never hand-edited. One
exception, normative for the engine itself: the **Rust envelope types are project-owned** and
carry an explicit extra-fields map (`#[serde(flatten)]`), because generated Rust types drop
unknown members and would make the engine a lossy intermediary (spike 1.1, finding 14).

## 3. Byte-pipes and framing

One protocol, three pipes (ADR 0002/0003). Each pipe carries the same frames (§4) and is
**frozen**: the pipe interface itself never grows; all evolution happens inside the frames.

### 3.1 WASM component / core module

The canonical WIT world is a single function:

```wit
call: func(request: list<u8>) -> list<u8>;
```

(with the equivalent three-function shim — create, call-with-buffer, free — on the zero-import
core module and the C ABI; see ADR 0003.) One request frame in, exactly one response frame
out, strictly serial per instance. Event frames never appear on this pipe; event delivery is
by polling (§9). The bytes are one UTF-8 JSON document with no framing header.

### 3.2 Subprocess (stdio)

`pact-engine` speaks LSP-style framing on standard I/O:

- Each frame is preceded by a header section: `Content-Length: <bytes>\r\n`, optionally other
  headers, then `\r\n`. The body is exactly `Content-Length` bytes of UTF-8 JSON. Receivers
  MUST ignore unknown headers. The declared length, not the JSON, delimits the stream — a
  malformed body is reported in-band (§4.4) and the stream stays in sync (spike 1.3,
  finding 6).
- **stdout carries only protocol frames.** Logs and diagnostics go to stderr. The engine MUST
  ensure nothing else in its process writes to stdout (spike 1.3, finding 8).
- **The engine treats stdin as its lease on life**: on stdin EOF the engine MUST release all
  sessions and exit promptly (exit code 0) even if no `shutdown` was received. This is the
  orphan-prevention mechanism — it is normative engine behaviour, not host hygiene (spike
  1.3, finding 2). Hosts SHOULD additionally apply ordinary kill-escalation to a wedged
  process; that is host hygiene, not protocol.
- Hosts MAY pipeline requests without waiting for responses; correlation is by frame id
  (§4.1). The engine MAY respond out of submission order, subject to the per-session ordering
  rule in §7.

### 3.3 Native (CLI in-process)

The CLI links the kernel directly and calls the same dispatch entry point with the same
frames. No additional rules; it is pipe 3.1 without the WASM boundary.

## 4. Frames: the protocol envelope

Schema: [`schemas/v1/frame.schema.json`](schemas/v1/frame.schema.json). A frame is a single
JSON object discriminated by its `type` member — an open discriminator with three values
defined in v1: `request`, `response`, `event`.

### 4.1 RequestFrame

```json
{ "type": "request", "id": "r-42", "op": "consumer-session/add-interaction", "body": { … } }
```

- `id` (string, required): correlation id, chosen by the host, non-empty, unique among the
  host's in-flight requests on this pipe. The engine treats it as opaque.
- `op` (string, required): the operation name — an open vocabulary, namespaced
  `<area>/<verb>` (§8). An engine receiving an unknown or unsupported `op` MUST respond with
  error code `operation-unsupported`, naming the op in `details` (never silence — spike 1.1,
  E3).
- `body` (object, required): the operation's request document, governed by the operation's
  schema. Operations with no parameters take `{}`.

### 4.2 ResponseFrame

Exactly one response per request, carrying exactly one of `ok` or `error`:

```json
{ "type": "response", "id": "r-42", "ok": { … } }
{ "type": "response", "id": "r-42", "error": { "code": "…", "message": "…", "details": { … } } }
```

- `id` echoes the request's id.
- `ok` (object): the operation's result document, governed by the operation's schema.
- `error` (object): an `EngineError` (§10). Errors are values: this is the *only* way an
  operation fails.

### 4.3 EventFrame

```json
{ "type": "event", "event": { "stream": "…", "seq": 3, "kind": "…", "payload": { … } } }
```

Event frames occur only where push delivery has been negotiated (§9; stdio pipe only). Under
the default poll model the same `Event` objects travel inside `ok` documents. The `Event`
shape and its semantics are specified in §9.

### 4.4 Malformed frames

If a frame's body is not valid JSON or violates the frame schema, the engine responds with an
error frame with code `malformed-frame`. If the offending frame's `id` cannot be recovered,
the response carries `"id": ""` — hosts MUST treat an empty-id error as pipe-level, not
correlated to any request. On the stdio pipe the engine then continues reading at the next
header boundary (the framing, not the JSON, delimits the stream); on the call pipes the error
is simply the returned frame.

### 4.5 Unknown frame types

A party receiving a frame whose `type` it does not know MUST NOT crash. On a call pipe the
engine returns `malformed-frame` (an unknown type cannot be dispatched); on stdio a receiver
MUST ignore the frame apart from logging. Parties MUST NOT send frame types the peer has not
declared support for (new frame types arrive as negotiated capabilities, §5.3).

## 5. Handshake, version and capability negotiation

Schema: [`schemas/v1/hello.schema.json`](schemas/v1/hello.schema.json).

### 5.1 The `engine/hello` operation

The first request on every pipe MUST be `engine/hello`. Any other operation before a
successful handshake is answered with error code `handshake-required`.

Request body (`Hello`):

```json
{
  "protocol-versions": [1],
  "host": { "name": "pact-js", "version": "0.1.0" },
  "capabilities": { }
}
```

- `protocol-versions`: the protocol major versions the host can speak, in preference order.
- `host`: identification, for diagnostics only; semantics MUST NOT depend on it.
- `capabilities`: what the host can do (§5.3).

Result (`HelloResult`):

```json
{
  "protocol-version": 1,
  "engine": { "name": "pact-engine", "version": "0.1.0" },
  "capabilities": { }
}
```

The engine picks the first version in `protocol-versions` it supports; all subsequent frames
on the pipe are governed by that version's schemas. If it supports none, it MUST respond with
error code `protocol-version-unsupported` and list its supported versions in
`details.supported` (spike 1.3 scenario), then keep the pipe usable so the host can report a
good error — the host is expected to shut down after such a failure.

### 5.2 Version semantics

The protocol version is a single integer, incremented only for changes the compatibility
policy (§11) classifies as breaking. Additions — new operations, event kinds, error codes,
optional members — do **not** bump it; they are discovered via capabilities and the
open-world rules. An SDK pinned to protocol N can expect engine N+k to speak N verbatim
(§11.2).

### 5.3 Capabilities

`capabilities` is an object whose member names are an open vocabulary; each member's value is
an object (its shape is defined where the capability is defined; `{}` when mere presence is
the signal). Absence of a member means the capability is not available. Both sides MUST
ignore unknown capability names.

Capabilities defined in v1:

| Name | Declared by | Meaning |
|---|---|---|
| `push-events` | both | On the stdio pipe: sender may deliver events as EventFrames instead of waiting to be polled (§9). Effective only when both sides declare it. |

Optional operations and future frame types are gated the same way: an engine that implements
an optional area declares it as a capability; a host MUST NOT rely on operations behind a
capability the engine did not declare.

## 6. Engine lifecycle: shutdown and liveness

- **`engine/shutdown`** (request body `{}`): the engine MUST release all sessions, respond
  `ok: {}`, and then exit (subprocess) or become inert (call pipes: any subsequent call
  returns error `engine-shut-down`). The response is sent *before* exit so hosts can
  distinguish clean shutdown from a crash.
- **stdin EOF** (stdio pipe): equivalent to `shutdown` with no response, per §3.2.
- There is no keep-alive/ping operation in v1: on the call pipes liveness is trivial, and on
  stdio process liveness is observable by the host. If a need appears it arrives as a
  capability, not a version bump.

## 7. Sessions

*Drafted in a later chunk: session model, the only-resource rule, per-session operation
ordering, session kinds (consumer, verification), passive vs emissive interactions.*

## 8. Operation set

*Drafted in a later chunk: `consumer-session/*`, `verification/*`, `explain`, `upgrade/*`,
`events/poll`; per-operation request/result schemas.*

## 9. Events and streams

*Drafted in a later chunk against ADR 0005 (event delivery model): the `Event` shape, stream
identity, ordering and completion guarantees, event-kind vocabulary.*

## 10. Error taxonomy

*Drafted in a later chunk: `EngineError` structure, code vocabulary and categories,
component-sourced errors. Schema:* [`schemas/v1/engine-error.schema.json`](schemas/v1/engine-error.schema.json).

## 11. Compatibility policy

*Drafted in a later chunk: what an SDK pinned to protocol N can expect from engine N+1 and
vice versa; the additive-evolution rules; the schema-compatibility checker.*
