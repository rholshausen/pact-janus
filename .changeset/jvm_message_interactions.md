---
jvm-sdk: minor
---

# A consumer test can record a message

`janus.message(description)` builds a `MessageInteraction` (ADR 0068): `given`,
`contents(m -> m.metadata(..).body(..))` for the message and `request(m -> ...)` for the request a
synchronous message answers. A body written with `Shapes.content(type, …)` declares that type, and any
other is JSON. `janus.execute(message, (received, variant) -> ...)` starts no mock, and hands the test
each variant's `ReceivedMessage`: `body()`, `contentType()`, `metadata()`, `text()`, `json()`, and a
synchronous message's `request()`. `JanusExtension` has `message`, `execute` and `variants` for it.
It needs an engine with ADR 0067's `serve-variant` parts.
