---
typescript-sdk: patch
---

# The generated contract bindings read `metadata.components`

A contract's metadata may list the declared components its writer loaded, as `RecordedComponent`
entries with a `RecordedSource` (ADR 0097). The bindings type them. Nothing an SDK does changes.
