---
typescript-sdk: minor
---

# A consumer test can record a message

`janus.message(description)` builds a message interaction (ADR 0068): `given`, `contents({ metadata,
body })` for the message and `request({ metadata, body })` for the request a synchronous message
answers. A body written with `content(type, …)` declares that type, and any other is JSON.
`janus.execute(message, (message, variant) => …)` starts no mock, and hands the closure each variant's
`ReceivedMessage`: `body` as a `Uint8Array`, `contentType`, `metadata`, `text()`, `json()`, and a
synchronous message's `request`. The contract records it, and a provider's message producers verify
it. It needs an engine with ADR 0067's `serve-variant` parts.
