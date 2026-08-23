# Spike 1.5 findings — Message-transport shape test

Status: **complete** — the RFC's transport sketch survives contact with async messaging, but only
after restating it in role-neutral terms; the restatement and the resulting spec obligations are
the findings. Toy evidence: one interface, two real implementations (TCP/HTTP and an in-memory
topic broker), six role/direction combinations, all green. Method: [README.md](README.md).

## 1. The restatement (the core finding)

The RFC phrase — *"start/stop a mock endpoint; drive requests at a provider; map wire messages
to/from the abstract interaction parts"* — names three verbs with an HTTP accent: "mock endpoint"
presumes serving, "drive requests" presumes request/response. Neither survives messaging
literally. What does survive is the shape under it, restated as **four role-neutral primitives**:

```
start(options) -> endpoint-descriptor      // an open document, not a URL
stop()
send(parts, await-reply?) -> reply-parts?  // outbound; the reply half is optional
poll-inbound(timeout) -> (event-id, parts)?  // arrived wire traffic, transport's schedule
reply(event-id, parts)                     // complete an inbound event that has a reply half
```

Every scenario in both transports is a composition of these — and the compositions pair up
across transports:

| Scenario | Composition |
|---|---|
| S1 consumer HTTP mock | `poll-inbound` → match → `reply` |
| S5b sync-message consumer mock | **the same loop** (reply-topic + correlation handled inside the transport) |
| S2 provider HTTP verify | `send(await-reply)` → match reply |
| S5a sync-message provider verify | **the same call** |
| S3 consumer message test | `send(no-reply)` — "serve-variant" becomes "publish now" |
| S4 provider message verify | `poll-inbound`, match, *no reply* — dropping the event is the end state |

The mock/verify duality is direction, not kind: mocking is the inbound loop, driving is the
outbound call, and each transport uses both in different scenarios.

## 2. Findings

1. **The RFC's assertion holds, with the wording fixed.** No scenario needed a fifth primitive,
   a message-specific interface, or a transport-specific engine loop. The plan's fear — an
   interface that is request/response-shaped all along — is averted by two specific changes:
   the reply half of both flows must be **optional** (fire-and-forget has no response to match),
   and "endpoint" must be an **open descriptor document** (host/port for HTTP; broker/topic
   info for messages — which is also exactly the v5 pact file's per-interaction "transport
   binding").
2. **"Map wire ↔ abstract parts" is the load-bearing clause, and parts must be an open
   vocabulary.** Message metadata (topic, key, headers, content-type) are just parts alongside
   the payload: S4 matched metadata identity (`topic`), shaped metadata (`key`, `content-type`)
   and payload with the *same* matcher and the same mismatch format as HTTP method/path/body.
   No metadata special-casing anywhere. This is the 1.1 document-first rule paying off again —
   a typed request/response record here would have been the Phase-8 trap.
3. **The passive/emissive inversion is real but lands in the protocol layer, not the transport
   interface.** `serve-variant` means "arm and wait for the app" for HTTP, but "emit now" for a
   message consumer test. The transport primitives don't care; the **consumer-session operations
   in protocol spec 2.1 must distinguish passive interactions (served on arrival) from emissive
   ones (delivered on command)** — that's a semantic tag on the interaction/transport binding,
   not an interface change.
4. **Poll, don't call back.** Inbound traffic needs a path from transport to engine; the toy's
   poll model (engine drives, transport never calls into the engine) handled async arrival for
   both transports — the HTTP transport is internally threaded (accept loop → queue) but that
   never leaks. For WASM component transports this matters doubly: an exported `poll` is
   trivial, while an engine-callback import would re-enter component instances (1.4 showed how
   loudly wasmtime polices instance re-entry). Recommend poll/event-queue in the 2.5 interface;
   the protocol's own event-stream design (2.1) can then surface transport events uniformly.
5. **Correlation is wire business and stays inside the transport.** Request/reply-over-broker
   needed reply-topics and correlation-ids; `send(await-reply)` created them and `reply()`
   copied them from the pending inbound event. The engine never saw a correlation-id — it saw
   parts in, parts out, same as HTTP. The cost: inbound events need **engine-visible event-ids**
   (for `reply` addressing and result attribution), which is the one piece of state the
   interface carries.
6. **The transport is optional for messages, and the interface should say so.** Classic
   handler-level message tests (today's Pact) need no wire at all: a `produce-message` /
   `consume-message` hook hands parts directly to the engine. Because everything is parts
   documents, the wire-level path (this spike) and the hook-level path produce identical inputs
   to matching — S4's trigger *is* the produce-hook with a broker behind it. Phase 2 should
   specify message verification as parts-in, with "via transport" and "via hook" as
   interchangeable sources.
7. **Deliberate toy gaps that become spec items, not risks**: (a) bytes↔document conversion was
   skipped — parts carried parsed JSON; in the real design the content component owns that
   boundary and transports carry bytes + content-type; (b) real broker semantics (acks,
   redelivery, consumer groups, partitions/ordering) hide behind the transport — but a real
   broker transport will need an **inbound-event disposition** (ack/nack) distinct from wire
   `reply`; the in-memory broker let S4 silently drop the event, a real one must not. Add a
   completion op (or disposition parameter) to the 2.5 interface and validate it in Phase 8;
   (c) multi-message sequences (sagas, websockets) — `poll`/`send` are already
   sequence-friendly; the gap is plan/interaction grammar, which the RFC's future-work section
   already owns.

## 3. Recommendation

Adopt the four-primitive, parts-document transport interface (with optional reply halves, open
endpoint descriptors, engine-visible inbound event-ids, and an inbound disposition op) as the
starting point for the Phase 2 component-interface spec (2.5). Specify the passive/emissive
distinction in the protocol's consumer-session semantics (2.1). The Phase-8 message-transport
work then implements against an interface already shaped for it — which was the point of paying
for this check now.
