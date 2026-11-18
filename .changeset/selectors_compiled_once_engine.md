---
engine: patch
---

# `subsumption/check` compiles each selector once per check

Staged plan S4.5, ADR 0095. A check compiled every provider-shape entry's selector again for every
interaction it tried to match. It now compiles each once per check, the first time an interaction
needs it, which makes a large contract's check about 40% faster. No verdict changes. A selector that
is not a shape document is reported once as `interaction-invalid`, where it was reported once for
every interaction that read it.
