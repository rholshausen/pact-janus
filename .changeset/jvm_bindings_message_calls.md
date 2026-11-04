---
jvm-sdk: patch
---

# Bindings know the message calls of `add-pact-interaction`

The generated protocol bindings' vocabulary lists the `pact_ffi` message calls
`consumer-session/add-pact-interaction` now makes (S2.5c). Nothing in the SDK calls them yet;
pact-jvm's facade will (S2.5b).
