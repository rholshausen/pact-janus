---
typescript-sdk: major
---

# A variant fails in its own test, and the engine withholds the contract

`execute` reports each closure's verdict to the engine and fails on each variant the engine did not
verify (ADR 0064): the closure threw, the mock rejected its request, or nothing called the mock.
These used to fail only `finalise`, in the suite's `afterAll`. A closure that never calls the mock
now fails its own `execute`. `VariantFailure` carries the engine's `result`, and `cause` only when
the closure threw.

The SDK no longer withholds a contract itself: the engine does. `ContractWithheldError` loses
`failedTests` and `engineWithheld`, and gains `interactions` (those that did not verify, then those
the engine refused) and `refused`. This needs an engine with `consumer-session/report-variant`,
which an older engine named by `JANUS_ENGINE` answers `operation-unsupported` at the first `execute`.
