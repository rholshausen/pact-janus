---
typescript-sdk: minor
---

# `Janus` takes a `redaction` option

`new Janus({ ..., redaction: { add, keep } })` names, by path, members the contract records redacted
beyond the credentials the transport declares (`add: ["request.headers.x-api-key"]`), and ones it
records as they are (`keep: ["request.headers.authorization"]`, for a fixed token the provider's test
environment accepts). It is passed to the engine as written, as `config.redaction` (ADR 0074). With
an engine that has S2.8, a contract records `Authorization`, `Proxy-Authorization`, `Cookie` and
`Set-Cookie` as `"[redacted]"` whether or not the option is given.
