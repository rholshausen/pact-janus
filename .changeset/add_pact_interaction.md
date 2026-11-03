---
engine: minor
---

# `add-pact-interaction` takes a consumer library's pact_ffi calls

A new operation, `consumer-session/add-pact-interaction` (ADR 0066), takes
`{ session, pact: { specification?, steps } }`: the calls a consumer library built on `pact_ffi`
makes to build one interaction, each `{ call, … }` named after the FFI function without its
`pactffi_` prefix. The engine applies them as `pact_ffi` does, integration JSON, XML builder
documents and multipart files included, then converts the interaction as `upgrade/pact` converts a
pact's, and adds it to the session. It answers `{ handle, findings? }`, the conversion's findings
pointing into the built interaction. Its one variant is `base`, pinning nothing, whatever policy is
asked for; a mock serves it the interaction's own example, and the contract records that example, as
`upgrade` does. A multipart boundary not given is derived from the part, not drawn at random. A step
the engine does not make, or cannot read, is `interaction-invalid`, pointing at `/pact/steps/<n>`.
The calls are pact-js's HTTP builders': `upon_receiving`, `given`, `given_with_params`,
`with_request`, `with_query_parameter`, `with_header_v2`, `response_status_v2`, `with_body`,
`with_binary_file`, `with_matching_rules` and `with_multipart_file_v2`.
