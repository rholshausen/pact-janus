---
default: patch
---

# A contract's executed plan shows the response it was scored on, not a failing request

When verifying a Janus contract, the `verification/executed-plan` event (and so
`janus verify --explain-failures`) rendered the whole interaction's plan. Its request half was
evaluated against nothing, because only the provider's reply is there to resolve against, so every
request check showed as an error, such as `ERROR(Expected <absent> to equal 'GET')`. The verdict was
never affected: only the response is scored.

The event's `text` is now the scored `response` subtree, the same as for a v1–v4 pact, so the plan
you read is the one that decided the verdict. Its `plan` member is unchanged.
