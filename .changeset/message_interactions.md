---
engine: minor
---

# A contract records messages, and a consumer session hands one to its test

A Janus contract has a message form (ADR 0067): an interaction whose `transport.mode` is `emissive`,
with a `message` part, `metadata` and `body`, and, for a synchronous message, a `request` part beside
it. `janus upgrade` converts v3 messages and v4 asynchronous and synchronous messages into it, where
it dropped them with `interaction-dropped`. Two new findings: `body-type-on-metadata`, when a
message's metadata names no `contentType` and today's type check becomes a pattern there, and
`responses-dropped`, when a synchronous message expects more than one response. A metadata
`contentType` with no rule becomes `equality`, reported as `rule-narrowed`, as a header is.

`janus verify` asks a contract's message of its `produce-message` hooks, as it asks a pact's, with
the same `pact-message` request: the description, the provider states and a synchronous message's
recorded request, without matching rules. A produced message's metadata `contentType` is read as its
body's label, else its own, else what its octets are detected as.

`consumer-session/serve-variant` for an emissive interaction, with no transport of its kind started,
answers `{ parts }`: the variant's message, every part of it, a declared body encoded and labelled.
The variant is exercised, and `report-variant` decides it. With a transport of its kind started it is
`operation-unsupported`, until delivery through a transport is built (S6.2). A passive interaction's
answer is `{}`, as before.
