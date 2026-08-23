# Spike 1.5 — Message-transport shape test

Plan task 1.5 · Feeds Phase 2 component-interface design (2.5) · Findings in
[FINDINGS.md](FINDINGS.md) (the durable artifact — code here is disposable).

## Question

The RFC sketches the transport component interface as *"start/stop a mock endpoint; drive requests
at a provider; map wire messages to/from the abstract interaction parts"* — language with an HTTP
accent. Does that shape actually fit an **async message transport** (broker topics, unsolicited
arrival, fire-and-forget, request/reply-over-queue), or does Phase 8 discover the interface was
request/response-shaped all along? Cheap to check now with paper + toy code.

## Method

1. Reduce the RFC sketch to **four transport primitives**, deliberately role-neutral:
   - `start(options) -> endpoint-descriptor` / `stop()`
   - `send(parts, await-reply?) -> optional reply-parts` — outbound: engine puts parts on the wire
   - `poll-inbound(timeout) -> optional (event-id, parts)` — inbound: arrived wire traffic mapped
     to parts, on the transport's schedule
   - `reply(event-id, parts)` — complete an inbound event that requires a wire reply
   with "parts" an **open document** (1.1 rules), not a fixed request/response record.
2. Implement the same trait twice (`toy/`):
   - **HTTP transport** — real TCP sockets, hand-rolled minimal HTTP/1.1;
   - **broker transport** — in-memory topic broker with subscriber threads standing in for an
     external Kafka-ish system (topics, keys, headers, correlation-ids, reply-topics).
3. Run five scenarios through the *same engine-side loop* (the 1.2 type matcher standing in for
   plan execution), checking where the interface pinches:
   - S1 consumer HTTP mock: app thread makes a real HTTP call; engine matches the arrived request
     and replies through the transport.
   - S2 provider HTTP verify: engine drives a request at a provider thread, matches the response.
   - S3 consumer message test: engine *emits* the example message onto a topic ("serve-variant"
     becomes "publish now"); app's subscriber/handler consumes it.
   - S4 provider message verify: a `produce-message` trigger makes the provider app publish;
     the transport *collects* the unsolicited message; engine matches message parts + metadata —
     fire-and-forget, nothing to reply to.
   - S5 sync message over the broker: `send` with awaited correlated reply (reply-topic +
     correlation-id), engine matches the reply parts.

## Layout

```
toy/          One Rust crate: parts docs, the 4-primitive trait, HTTP + broker
              transports, the five scenarios as tests-in-main
FINDINGS.md   Where the RFC phrasing holds, where it needs restating, Phase-2 notes
```
