---
typescript-sdk: minor
---

# `pact-janus/pact`: pact-js's message consumer API

`Pact`'s `addAsynchronousInteraction` and `addSynchronousInteraction`, `MessageConsumerPact`, and
`synchronousBodyHandler`, `asynchronousBodyHandler`, `v4SynchronousBodyHandler` and
`v4AsynchronousBodyHandler` are pact-js's, so a message consumer suite moves by changing its import
(S2.5c). The builders record the `pact_ffi` calls pact-js's make, which needs an engine whose
`add-pact-interaction` builds messages. No mock is started: the test is handed the message the
engine answers, in the form pact-js hands it, and the contract is the one `janus upgrade` writes from
the pact pact-js writes. A synchronous message's response metadata is accepted and does nothing, as
pact-js's does, and `usingPlugin` is refused by name. Until S2.15, a message given as octets is
handed back as their base64 text. `Documentation/consumer-test-migration.md` gives each primitive's
verdict.
