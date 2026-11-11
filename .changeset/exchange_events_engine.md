---
engine: minor
---

# A consumer session can tell what its mock made of each request, as it happens

Staged plan S3.5, ADR 0084.

- **`consumer-session/create` takes `events: true`** and answers with `stream`, which `events/poll`
  drains. Each request a started transport receives is a `consumer-session/exchange` event: the
  transport instance, `status` (`matched`, `mismatched`, `undelivered`, or `unmatched` when nothing was
  armed), and the `handle` and `variant` it went to, with `mismatches`, redacted as `report-variant`'s
  are. `finalise` ends the stream with `consumer-session/finished`, without waiting for the host to
  drain it.
- **The engine's own log line drops to `debug`** for a session whose host asked for events, since the
  host says it: `request did not match the armed variant` and `inbound request arrived with nothing
  armed` are `warn` only in a session with no stream.
- A session created without `events` behaves as before. A host that asks must drain the stream: a
  thousand undrained requests stall the mock until it does.
