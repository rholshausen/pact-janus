---
engine: minor
---

# The behavioural specification builds messages

For an SDK maintainer: the behavioural specification gains `message`, `message-contents` and
`message-request` (ADR 0068). A message is an interaction-spec document with `transport`
`{ kind: message, mode: emissive }`, its `message` part, and a synchronous message's `request` part,
each with `metadata` and `body`. A message's body always declares its type: `content`'s, else
`application/json`. `execute` of a message sends no `start-transport`, and hands the closure the
message `serve-variant` answers, its body as octets with its type and metadata, and a synchronous
message's request. Nine conformance ids, with four translation cases, three scripted cases and one
live case; the case format gains a message chain, the scripted engine's `served`, and a step's
`received`.
