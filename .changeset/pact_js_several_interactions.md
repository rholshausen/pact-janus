---
typescript-sdk: minor
---

# `pact-janus/pact`: several interactions in one `executeTest`

`PactV3`'s `executeTest` serves every interaction added since the last one, and `Pact`'s every
interaction added since its last `executeTest`, as pact-js's do, each as often as the test calls it.
A test that never calls one of them fails, naming its missing request, and writes no contract. It
needs an engine that arms several interactions at once (ADR 0069).
