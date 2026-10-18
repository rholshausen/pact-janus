---
default: patch
---

# A verification run that crashes still ends its event stream

If a bug stopped a verification run part way, its stream never sent a terminal event, so a host
polling `events/poll` until `last: true` waited forever. The run now ends with
`verification/finished`, carrying `status: failed` and an `error` member: an `EngineError` with code
`internal` and the panic's message. The tallies died with the run, so `variants` and `failures` may
be absent from that summary (engine-protocol spec §9.6).

A host can rely on this: poll until the terminal event, with no timeout of its own. An empty poll
means nothing happened within `wait-ms`, never that the run is gone.
