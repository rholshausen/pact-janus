---
jvm-sdk: patch
---

# The generated bindings describe what a recording's `kind` is read through

`StartRecording.kind`'s description says that a kind's declarations may be held by an embedding with
no transport for it, as the WASM engine holds `http`'s (ADR 0094). Only the documentation changed.
