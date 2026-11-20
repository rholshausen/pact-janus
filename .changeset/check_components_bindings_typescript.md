---
typescript-sdk: patch
---

# The generated protocol bindings carry `components` on `subsumption/check` and `broker/check-contracts`

Both operations take the project's declared components (ADR 0102), whose operators the walk compares as
their handshakes declare. The bindings gain the optional member; nothing an SDK does changes.
