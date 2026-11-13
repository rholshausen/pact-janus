---
engine: major
---

# The conformance suite holds an SDK to recording the messages a provider produces

The behavioural specification has a new primitive, `observe-message` (staged plan S4.6, ADR 0090): a
shape recorder records a message as a message producer returns one, with an operation over the
message's own `metadata` and `body`, or with a `request` over the request's, of the kind `message`.
Four cases cover it: `provider/recorder-message-frame`, `provider/recorder-message-synchronous`,
`provider/recorder-message-description-refused` and `live/recorded-message-shape`, which pins the shape
the engine writes. `janus-sdk-gates conformance check` expects them in every report, so an SDK must
implement `observe-message` to move to this release.

A case's recorder observation may be a `message`, with an optional `message-request`, in place of a
`response`.
