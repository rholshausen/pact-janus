---
typescript-sdk: patch
---

# A pact-js test that throws prints the request it never sent

`pact-janus/pact`'s `executeTest`, when the test throws without calling the mock, now prints
"Missing request: the interaction '…' was expected, but not received" beside rethrowing the test's
error, as pact-js does. It reads the engine's `exercised: false` (staged plan S2.6), so it needs an
engine that writes it.
