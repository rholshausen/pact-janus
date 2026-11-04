---
engine: minor
---

# `add-pact-interaction` takes `with_binary_body` and `with_generators`

`consumer-session/add-pact-interaction` makes two more `pact_ffi` calls (ADR 0070, staged plan S2.5b).
`with_binary_body` (`part`, `content_type?`, `body` as base64) sets a body's octets as they stand,
with no rule added and no integration JSON read from them, and a `Content-Type` header unless the
part has one. A message gets the octets' type in its metadata. `with_generators` (`part`, `generators`)
merges generators by category, as a pact writes them, as `with_matching_rules` merges rules. Both
apply to an HTTP interaction and to either kind of message, as the FFI's do. A library that builds
its own rules, as pact-jvm's DSL does, sends what it built: its example, its rules and its generators.
