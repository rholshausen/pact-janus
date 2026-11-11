---
jvm-sdk: patch
---

# The generated bindings carry an interaction's sampling policy

The contract bindings' `InteractionSpec` and `Interaction` have an optional `policy`: the
interaction's own sampling policy (ADR 0080), which the engine resolves when the interaction is added
and records in the contract. The SDK writes none yet; S3.2's `exclude` will.
