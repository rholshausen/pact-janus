---
jvm-sdk: patch
---

# Bindings for `add-pact-interaction`

The generated protocol bindings have the types of the engine's new
`consumer-session/add-pact-interaction` (ADR 0066): `AddPactInteraction`, `PactSteps`, `PactStep`
and `AddPactInteractionResult`. Nothing in the SDK calls it yet; pact-jvm's facade will (S2.5b).
