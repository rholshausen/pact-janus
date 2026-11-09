---
engine: minor
---

# A multipart part's content type is its own label, and a `pact_ffi` multipart body is split at its header's boundary

A v1–v4 multipart body whose type names no `boundary`, as `pact_ffi` writes one, is split at the
boundary its `Content-Type` header names, as today's verifier splits it. `janus upgrade` and
`consumer-session/add-pact-interaction` now shape its parts, where they wrote it with
`body-not-parsed` and asserted nothing about them (staged plan S2.20, ADR 0077).

The content interface gains an optional operation, `content/label` (`{ content-type, value, at }` →
`{ media-type? }`, component-interfaces spec §6.5): what a slot's component labels a member of its
document with. The built-in multipart component answers a part's own `Content-Type` for its `body`. On
the contract path, `match:content-type` on a member of a slot reads that label first, as it reads a
whole slot's, and detects from the member only when it has none, so a `text/plain` part matches a
`content-type` of `text/plain`. An out-of-tree component that does not implement the operation answers
`operation-unsupported`, and its members are detected as before. The v1–v4 path still sniffs.
