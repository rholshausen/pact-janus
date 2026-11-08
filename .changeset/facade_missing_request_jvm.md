---
jvm-sdk: patch
---

# A pact-jvm test that throws prints the request it never sent

`janus-facade`'s JUnit 5 extension, when a test throws without calling the mock, now prints "The
following requests were not received" with the interaction beside the test's own failure, as pact-jvm
does. It reads the engine's `exercised: false` (staged plan S2.6), so it needs an engine that writes it.
