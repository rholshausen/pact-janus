---
jvm-sdk: patch
typescript-sdk: patch
---

# Bindings know `with_binary_body` and `with_generators`

The generated protocol bindings' vocabulary lists the two `pact_ffi` calls
`consumer-session/add-pact-interaction` now makes (ADR 0070). Nothing in either SDK calls them yet;
pact-jvm's facade will (S2.5b).
