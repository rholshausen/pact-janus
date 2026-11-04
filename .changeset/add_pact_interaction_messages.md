---
engine: minor
---

# `add-pact-interaction` builds messages

`consumer-session/add-pact-interaction` takes the `pact_ffi` calls pact-js's message builders make
(staged plan S2.5c). A first step `new_async_message` or `new_message` begins an asynchronous message,
and `new_sync_message_interaction` a synchronous one. `message_expects_to_receive`, `message_given`,
`message_with_contents` (its contents as `body`, text, or `contents`, base64) and
`message_with_metadata_v2` build them, and `upon_receiving`, `given`, `given_with_params`,
`with_body`, `with_binary_file` and `with_matching_rules` apply to a message as `pact_ffi`'s message
branches do. The message is converted as `upgrade/pact` converts one (ADR 0067), so `serve-variant`
with no transport started answers its example as `parts`, and the contract records what
`janus upgrade` writes from the pact the library would have written. A call for another kind of
interaction is `interaction-invalid`, pointing at it, but for `message_with_metadata_v2` on a
synchronous message, which `pact_ffi` ignores. A message body given as octets is still served as
its base64 text, until S2.15.
