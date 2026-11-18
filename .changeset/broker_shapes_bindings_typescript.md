---
typescript-sdk: patch
---

# The generated bindings carry the broker's provider-shape operations

The protocol bindings have `PublishProviderShape`, `CheckContracts` and their results, and the event
vocabulary has `broker/checked` (ADR 0093). Nothing in the SDK reads them.
