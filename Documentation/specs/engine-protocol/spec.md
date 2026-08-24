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

### 7.1 The only resource

A **session** is the unit of engine-side state a host can hold: everything the engine
allocates on a host's behalf — interaction handles, compiled plans, running transports,
captured traffic, event streams — belongs to exactly one session and is released when that
session ends. Consequences, normative:

- No operation returns a resource that needs its own cleanup call. Handles, endpoints and
  stream ids are plain identifiers scoped to their session; they become invalid when the
  session ends, and there is no operation to release one individually.
- A session ends in exactly one way per kind: a consumer session by
  `consumer-session/finalise`; a verification session automatically when its run reaches a
  terminal event (§9). `engine/shutdown` and stdin EOF end all sessions.
- Session ids are engine-assigned opaque strings, unique within the life of the pipe.
  Operations targeting a session carry it as the `session` member of `body`. An unknown or
  already-ended session id is answered with error `session-not-found` — hosts holding stale
  ids get a named error, never undefined behaviour.

### 7.2 Ordering and concurrency

Operations that target the same session are executed in submission order. Operations on
different sessions, and session-less operations, MAY be processed concurrently and answered
out of order (stdio pipelining, §3.2). A host that needs cross-session ordering sequences its
own requests.

### 7.3 Session kinds

v1 defines two kinds:

- **Consumer session** (§8.2): drives mock/stub endpoints and message emission during a
  consumer test run, accumulates per-interaction verification status, and produces the pact
  file document at finalisation.
- **Verification session** (§8.3): one provider-verification run. Created by
  `verification/verify`, reports progress as events, ends itself at the terminal event.

New session kinds arrive as new operations plus capabilities, not as changes to existing
ones.

### 7.4 Passive and emissive interactions

From spike 1.5: mocking is the inbound loop, driving is the outbound call, and the difference
between an HTTP interaction and a message interaction is *direction, not kind*. Each
interaction in a consumer session is, per its transport binding (design 2.2/2.5), either

- **passive** — the engine arms the interaction and *waits for the application under test*
  to initiate (HTTP request to a mock endpoint, sync message consumed from a topic the
  engine serves); or
- **emissive** — the engine *delivers on command*: `consumer-session/serve-variant` causes
  the interaction's message to be produced now (message-consumer tests).

The operation surface is identical in both modes; only the semantics of `serve-variant`
differ (§8.2). The passive/emissive tag lives in the interaction specification's transport
binding, not in the protocol envelope.

## 8. Operation set

Operation names are namespaced `<area>/<verb>`, kebab-case. v1 defines the areas `engine`
(§5–6), `consumer-session`, `verification`, `upgrade` and `events` (§9). The vocabulary is
open: new operations may be added within a protocol version, gated by capability when a host
must know in advance (§5.3); unknown operations are answered with `operation-unsupported`.

Request and result body schemas: one schema file per area under
[`schemas/v1/`](schemas/v1/), with per-operation `$defs` named `<Verb>` / `<Verb>Result`.

### 8.1 Documents the protocol carries but does not define

Several operation bodies embed documents whose shape is owned by other Phase 2 designs. The
protocol treats them as objects governed elsewhere, and this spec MUST NOT constrain their
interiors:

| Document | Owner |
|---|---|
| interaction specification (with shapes) | designs 2.2 (shape language), and 3.2 (document model) |
| variant descriptor | design 2.3 (variant semantics); protocol requires only `id` |
| matching plan (pretty/`--executed` forms) | design 2.4 (plan grammar) |
| pact file (v1–v4 read, v5 read/write) | design 2.5 |
| endpoint descriptor, transport options | design 2.6 (component interfaces); open documents per spike 1.5 |

### 8.2 Consumer sessions — `consumer-session/*`

Schema: [`schemas/v1/consumer-session.schema.json`](schemas/v1/consumer-session.schema.json).

| Operation | Body → Result |
|---|---|
| `consumer-session/create` | `{ config }` → `{ session }` |
| `consumer-session/add-interaction` | `{ session, interaction }` → `{ handle }` |
| `consumer-session/variants` | `{ session, handle }` → `{ variants }` |
| `consumer-session/start-transport` | `{ session, transport, options? }` → `{ endpoint }` |
| `consumer-session/serve-variant` | `{ session, handle, variant }` → `{ }` |
| `consumer-session/finalise` | `{ session }` → `{ results, pact? }` |

- **`create`**: `config` names the consumer and provider (`{ "consumer": { "name": … },
  "provider": { "name": … } }`) plus open, additive options. Returns the session id.
- **`add-interaction`**: submits one complete interaction specification. The engine
  validates it and compiles what it needs; a rejected spec is a structured error
  (`interaction-invalid`) whose `details` carry positions a DSL can surface — errors good
  enough for an SDK user are a stated goal (plan 3.2). The returned `handle` identifies the
  interaction within this session.
- **`variants`**: the interaction's computed variant space (design 2.3), for variant-driven
  test loops. Each variant descriptor carries at least `id`; everything else is 2.3's.
- **`start-transport`**: starts a transport component instance for this session (`transport`
  is an open vocabulary: `"http"`, …). The result `endpoint` is an **open descriptor
  document** — host/port for HTTP, broker/topic details for messaging — never assumed to be
  a URL (spike 1.5, finding 1). A session MAY start several transports.
- **`serve-variant`**: for a **passive** interaction, arms the given variant: the next
  matching inbound traffic on the session's transports is matched against it. For an
  **emissive** interaction, delivers now: the engine produces the message for that variant
  through the bound transport (or the hook path, design 2.7). Multiple interactions may be
  armed concurrently; re-arming a handle with a different variant replaces the previous
  arming.
- **`finalise`**: ends the session unconditionally (transports stopped, all state
  released — even if the result is all failures) and returns per-interaction, per-variant
  results. The `pact` member — the pact file *document*; persistence is the host's business —
  is present iff every interaction verified successfully on its required variants.
  Unmatched-request and missed-interaction detail rides in `results`.

### 8.3 Verification — `verification/*`

Schema: [`schemas/v1/verification.schema.json`](schemas/v1/verification.schema.json).

| Operation | Body → Result |
|---|---|
| `verification/verify` | `{ source, target, options? }` → `{ session, stream }` |
| `verification/explain` | `{ interaction, options? }` → `{ text, plan? }` |

- **`verify`** starts a verification session and returns immediately with its session id and
  the id of the event stream on which the run reports (§9). Progress, hook activity,
  per-interaction/per-variant results and the final summary are all events; the stream's
  terminal event carries the summary document and ends the session. `source` is an open
  descriptor of where the pacts come from — v1 defines kind `"inline"` (the pact documents
  are in the request); fetching from files, URLs or a broker is host/CLI business in the
  prototype, which also keeps I/O out of the WASM kernel. `target` describes the provider
  under test: transport bindings (open descriptors again) plus open options such as state-
  change configuration (design 2.7 owns hook config).
- **`explain`** compiles one interaction (from a spec or a pact interaction — the body says
  which) and returns the plan's pretty text form, optionally the structured plan document
  (design 2.4). It is a kernel operation precisely so no SDK builds its own (RFC). Explain
  of an *executed* plan is served by the event stream (§9), not by this operation.

### 8.4 Upgrade — `upgrade/*`

Schema: [`schemas/v1/upgrade.schema.json`](schemas/v1/upgrade.schema.json).

| Operation | Body → Result |
|---|---|
| `upgrade/pact` | `{ pact, options? }` → `{ pact, findings }` |

Converts a v1–v4 pact document to v5 per design 2.5's rules (matching rules become shapes;
the single example becomes the sole variant). `findings` lists lossy or judgement-call spots
(each with a code from an open vocabulary, a JSON-path location and prose) so the CLI's
`upgrade` command can show its work. Session-less: conversion is pure document-in,
document-out.

## 9. Events and streams

Delivery model fixed by [ADR 0005](../../decisions/0005-poll-based-event-delivery.md):
**polling is the baseline on every pipe; push is a negotiated, stdio-only optimisation.**
Schema: [`schemas/v1/events.schema.json`](schemas/v1/events.schema.json).

### 9.1 Streams

A **stream** is an ordered sequence of events produced by work inside one session (v1:
the verification run; consumer-session transport events are a named future capability).
Stream ids are engine-assigned opaque strings, scoped to their session and handed to the
host in the result of the operation that started the work (`verification/verify`). A stream
ends when its final event has been *delivered*; polling an unknown or ended stream is
answered with error `stream-not-found`.

### 9.2 The Event shape

```json
{ "stream": "s-1", "seq": 4, "kind": "verification/interaction-result", "payload": { … }, "last": false }
```

- `seq` (integer): starts at 1, increments by 1, no gaps — the host can detect its own
  bookkeeping errors.
- `kind` (string): open vocabulary, namespaced like operations. A host encountering an
  unknown kind applies policy with the name in hand (skip, log, surface) — it MUST NOT
  fail, and MUST still honour `seq` and `last`.
- `payload` (object): kind-specific document.
- `last` (boolean, default false): **termination is structural.** The final event of a
  stream carries `last: true`, whatever its kind — so completion never depends on
  recognising a vocabulary value. After it, the stream id is invalid, and a session that
  ends with its work (§7.3) is closed.

### 9.3 Ordering and delivery guarantees

Within a stream, events are delivered in `seq` order, exactly once. Across streams there is
no ordering guarantee. The engine buffers undelivered events and MUST NOT drop them; if a
buffer cap is reached it applies backpressure by pausing the producing work until the host
drains (loss is never the pressure valve).

### 9.4 `events/poll`

| Operation | Body → Result |
|---|---|
| `events/poll` | `{ streams, max?, wait-ms? }` → `{ events }` |

Drains up to `max` (default: engine's choice) pending events from the given streams, in
per-stream order. If none are pending and `wait-ms` is positive, the engine MAY hold the
response up to that long (long-poll) and respond as soon as an event arrives; the default is
`0` (return immediately). On the call pipes the pipe is serial, so hosts there SHOULD use
short waits and interleave polls with their other requests; on stdio, long-polls pipeline
freely (§3.2).

### 9.5 Push delivery (`push-events` capability)

When both parties declared `push-events` (§5.3; meaningful on the stdio pipe only), the
engine MAY deliver events as EventFrames (§4.3) instead of holding them for poll. Every
event still arrives exactly once — pushed events are never also returned by `events/poll` —
and all §9.2–9.3 guarantees are unchanged. Hosts implement the poll model regardless and
treat push as a latency upgrade, not a second semantics.

### 9.6 Event kinds (v1)

All on the verification stream; the vocabulary is open and grows without a version bump:

| Kind | Payload | Notes |
|---|---|---|
| `verification/started` | run description (pact count, provider) | first event |
| `verification/interaction-started` | interaction ref, variant | |
| `verification/interaction-result` | interaction ref, variant, status, mismatches | one per interaction × variant |
| `verification/hook` | hook point, outcome (design 2.7) | hook activity is events, per the RFC |
| `verification/executed-plan` | the executed plan document/text (design 2.4) | emitted when `verify` options request it — this is how `explain --executed` gets its input |
| `verification/finished` | the summary document | carries `last: true`; ends the session |

## 10. Error taxonomy

*Drafted in a later chunk: `EngineError` structure, code vocabulary and categories,
component-sourced errors. Schema:* [`schemas/v1/engine-error.schema.json`](schemas/v1/engine-error.schema.json).

## 11. Compatibility policy

*Drafted in a later chunk: what an SDK pinned to protocol N can expect from engine N+1 and
vice versa; the additive-evolution rules; the schema-compatibility checker.*
