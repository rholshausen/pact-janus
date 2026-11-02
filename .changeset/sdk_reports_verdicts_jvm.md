---
jvm-sdk: major
---

# A variant fails in its own test, and the engine withholds the contract

`execute`, and each dynamic test `variants` returns, reports the test's verdict to the engine and
fails on each variant the engine did not verify (ADR 0064): the test threw, the mock rejected its
request, or nothing called the mock. These used to fail only `finalise`, after the class's last
test. A test that never calls the mock now fails. `VariantFailure` is `(variant, result, cause)`,
whose `cause` is the test's throwable or an `AssertionError` saying the engine's verdict.

The SDK no longer withholds a contract itself: the engine does. `ContractWithheldException` loses
`failedInteractions()` and `engineWithheld()`, and gains `interactions()` (those that did not verify,
then those the engine refused) and `refused()`. This needs an engine with
`consumer-session/report-variant`, which an older engine named by `JANUS_ENGINE` answers
`operation-unsupported` at the first `execute`.
