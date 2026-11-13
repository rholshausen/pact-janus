---
jvm-sdk: patch
---

# The generated bindings carry an observation's kind

The protocol bindings' `ObserveResponse` has an optional `kind` (ADR 0090), which `observeMessage`
sends.
