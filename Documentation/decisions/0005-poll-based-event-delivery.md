# 0005 — Poll-based event delivery on all pipes; push as a negotiated stdio capability

- **Status**: proposed
- **Date**: 2026-08-24
- **Plan tasks**: 2.1
- **Evidence**: [spike 1.3 findings](../../spikes/1.3-subprocess-embedding/FINDINGS.md)
  (serial-pipe gap flagged for 2.1), [spike 1.5 findings](../../spikes/1.5-message-transport-shape/FINDINGS.md)
  (poll-don't-call-back, finding 4), [spike 1.2 findings](../../spikes/1.2-wasm-embedding/FINDINGS.md)
  (the frozen pipe is one synchronous function)

## Context

The RFC sketches `verify(...) -> stream<event>`, and the protocol needs engine→host event
flow more generally (verification progress, hook activity, eventually consumer-session
transport events). But ADR 0002/0003 fix the pipes, and two of the three cannot carry an
engine-initiated message at all: the WASM component/core-module pipe and the C ABI are one
synchronous `call(bytes) -> bytes` — the engine only ever speaks when spoken to. Only the
stdio pipe is full-duplex. Spike 1.3 explicitly deferred this: its toy pipe was strictly
serial request/response, and event streams plus correlation were flagged as 2.1 design work.
Spike 1.5 hit the same shape one layer down (transport→engine) and found that a poll model
handled async arrival cleanly while callbacks would re-enter WASM component instances —
wasmtime polices that loudly.

Candidates:

1. **Poll everywhere**: events accumulate in the engine per stream; the host drains them
   with an `events/poll` operation (optionally long-polling). Identical semantics on all
   three pipes.
2. **Push where possible**: event frames pushed on stdio; poll only on the call pipes. Two
   delivery semantics for SDKs to abstract over.
3. **Chunked responses**: the `verify` response held open and streamed in pieces. Breaks the
   one-request-one-response frame model, unimplementable on the call pipes, and reinvents
   framing inside a frame.

## Decision

**Polling is the v1 baseline and the only required model.** Events are buffered per stream
in the engine and drained via `events/poll`; streams are ordered by a per-stream sequence
number; termination is structural (a `last` marker on the final event) so it never depends
on recognising an event kind; the engine never drops events and may instead pause the
producing work when a buffer cap is hit (backpressure over loss).

**Push is an optimisation, negotiated, stdio-only.** When both sides declare the
`push-events` capability in the handshake, the engine may deliver the same events as
EventFrames instead of holding them for poll. Every event is delivered exactly once via one
path or the other; `events/poll` remains valid and simply returns what has not been pushed.
SDKs therefore implement one model (poll) and treat push as a latency upgrade, not a second
semantics.

Spec text: Engine Protocol specification §9; schemas `events.schema.json` and the
EventFrame in `frame.schema.json`.

## Alternatives considered

- **Push where possible (2)**: rejected as the baseline because it forks SDK behaviour by
  embedding — the thin-SDK story wants one shipped semantics — and because the primary
  embedding for several languages (ADR 0003) is a call pipe where push cannot exist.
  Retained as the negotiated optimisation, which captures its latency benefit without the
  fork.
- **Chunked responses (3)**: rejected outright — violates the frame model on every pipe and
  is impossible on `call(bytes) -> bytes`.
- **Callbacks (host-exported functions the engine invokes)**: never seriously in play —
  requires the pipe interface to grow per ADR 0002's frozen-pipe rule, and spike 1.5 showed
  the re-entrancy trap for WASM components.

## Consequences

Easier: one event semantics for every SDK and pipe; the WASM embedding needs nothing new;
long-poll (`wait-ms`) gives the subprocess embedding push-like latency even before
`push-events` is implemented; termination-by-structure keeps the event-kind vocabulary fully
open.

Harder: the engine owns event buffering and its backpressure policy; hosts on call pipes
must interleave polling with their other calls (the pipe is serial, so a long-poll blocks
it — hosts there should use short waits); exactly-once bookkeeping across the push/poll
seam needs care when `push-events` lands.

Tripwires: if verification event volume makes poll round-trips a measurable cost in the 1.7
benchmark trend on the call pipes, revisit with event batching limits (`max`) tuning before
inventing new mechanics; if `push-events` implementations diverge behaviourally from poll,
demote or remove the capability.
