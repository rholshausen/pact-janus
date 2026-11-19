---
typescript-sdk: patch
---

# The generated contract bindings read a recorded component's `signer`

A recorded component's source may name the signer whose Sigstore signature the engine verified, as a
`RecordedSigner` (ADR 0099). The bindings type it. Nothing an SDK does changes.
