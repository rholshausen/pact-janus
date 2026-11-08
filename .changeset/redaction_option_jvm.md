---
jvm-sdk: minor
---

# `JanusOptions` takes the members to redact and to keep

`JanusOptions.withRedacted(paths)` names, by path, members the contract records redacted beyond the
credentials the transport declares (`request.headers.x-api-key`), and `withKept(paths)` ones it records
as they are (`request.headers.authorization`, for a fixed token the provider's test environment
accepts). They are passed to the engine as written, as `config.redaction` (ADR 0074). With an engine
that has S2.8, a contract records `Authorization`, `Proxy-Authorization`, `Cookie` and `Set-Cookie` as
`"[redacted]"` whether or not they are given.
