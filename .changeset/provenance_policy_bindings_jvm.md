---
jvm-sdk: patch
---

# The generated bindings carry a compatibility report's provenance

The protocol bindings' `PolicyView` has an optional `provenance`, a map to the new
`ProvenanceActions`, and `FindingEntry` and `ExemptionResult` an optional `provenance` (ADR 0089).
Nothing in the SDK reads them.
