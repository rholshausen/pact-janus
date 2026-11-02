---
engine: patch
---

# Arming a variant again costs what arming a small one does

`consumer-session/serve-variant` generated a variant's payload and compiled its request plan every
time it armed it, and copied both into each transport. It now does that the first time a variant is
armed, and every later arming shares it. Arming a variant of a 100 KB request went from about 20 ms
to about 22 µs, and a small one from 12 µs to 4 µs. A generator still fills a fresh value each time a
reply is served (ADR 0048), so what the mock answers and what a contract records do not change.
