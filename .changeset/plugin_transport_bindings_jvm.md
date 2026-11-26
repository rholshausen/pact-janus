---
jvm-sdk: patch
---

# The generated protocol bindings carry a transport binding's `optional`

`verification/verify`'s `TransportBinding` gains `optional`, which holds a binding no loaded component
provides yet rather than failing the call (ADR 0111). The bindings gain the optional member; nothing an
SDK does changes.
