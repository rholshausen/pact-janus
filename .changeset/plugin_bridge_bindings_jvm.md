---
jvm-sdk: patch
---

# The generated protocol bindings carry `options.plugin-bridge` and `Mismatch.at`

`verification/verify`'s and `verification/explain`'s options gain `plugin-bridge`, how the engine starts
janus-plugin-bridge for a v1–v4 pact's plugins (ADR 0110), and a `Mismatch` gains `at`, where the
action that found it located it inside the value. Explain's options are typed as `ExplainOptions`. The
bindings gain the optional members; nothing an SDK does changes.
