---
jvm-sdk: patch
---

# The generated protocol bindings describe `verification/explain`'s components as they are now used

`verification/explain`'s `components` description no longer mentions plan fragments, which the engine
withdrew (ADR 0100): a content component's substitutions are compiled in instead. Only the generated
documentation changes, and nothing an SDK does.
