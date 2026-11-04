---
jvm-sdk: patch
---

# Bindings for a delivered message

The generated protocol bindings have `ServeVariantResult.parts` and `DeliveredSlotValue`, the message
`consumer-session/serve-variant` answers for an emissive interaction (ADR 0067), and the upgrade
findings `body-type-on-metadata` and `responses-dropped`. Nothing in the SDK builds a message
interaction yet (S2.14).
