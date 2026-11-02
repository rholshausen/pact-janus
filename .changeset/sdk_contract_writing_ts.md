---
typescript-sdk: major
---

# Test files recording one consumer and provider write one contract

`finalise` no longer writes the contract file: it names it, and the engine writes it, merging into
what the file holds (ADR 0065). Several test files recording one consumer and provider, in separate
workers or processes, write one contract with all of their interactions, where the last to finish
used to overwrite the others. An interaction two of them record differently fails the second to
finalise with the engine's `contract-conflict`, a `JanusError` naming it. `overwriteContract: true` replaces the file
instead. Because what merges is whatever the file held, a run should clear its contract directory
first. This needs an engine with `contract-writing`; an older one named by `JANUS_ENGINE` fails
`finalise`, saying so.
