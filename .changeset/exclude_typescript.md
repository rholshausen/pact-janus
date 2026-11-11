---
typescript-sdk: minor
---

# `exclude` leaves out a combination the provider cannot produce

`InteractionBuilder` and `MessageBuilder` have `exclude(when, reason)` (staged plan S3.2, ADR 0080):

```ts
janus.interaction("get an order")
  .exclude([["status", "SHIPPED"], ["shippedAt", "absent"]], "the order service dates a shipped order")
```

Each call is one exclusion, and calls accumulate. No variant the engine selects is inside an exclusion,
and the contract records each one with its reason and its references resolved to full ids. The
contract still admits the combination, so a provider that sends it passes. A reference that names no
dimension, a point the dimension does not have, and a blank reason fail `execute` with the engine's
`interaction-invalid`. An interaction that does not call `exclude` builds the document it built before.
