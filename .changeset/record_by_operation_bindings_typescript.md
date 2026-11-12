---
typescript-sdk: patch
---

# The generated bindings carry a provider-shape recording's operation and write

The protocol bindings' `StartRecording` has an optional `kind`, `ObserveResponse` an optional
`operation`, `ObserveResponseResult` optional `warnings`, `FinaliseRecording` an optional `write`
(`ShapeWrite`) and `FinaliseRecordingResult` an optional `written` (ADR 0086). The SDK records no
shape yet.
