---
engine: minor
---

# A failed test's result says when nothing called the mock

A variant whose test was reported failed, and which nothing exercised, now also carries
`exercised: false` in its `VariantResult`, from `consumer-session/report-variant` and `finalise`
alike (engine-protocol spec §8.2). The report still decides its status, `failed`, as before; the new
member says what `not-exercised` would have, so a host can print a missing request beside the test's
own error, as pact-js and pact-jvm do. Nothing else in a result changes.

The conformance suite grows a `facade` category (staged plan S2.6, ADR 0076): a case may name a
subject, a test in one language beside it, which the SDK of that language runs, and `check` holds each
report to the cases its `sdk.language` runs, the cases of the facades `conformance/facades.json` gives
it included. `janus-spec.tar.gz` carries the subjects and `facades.json`. A new conformance id,
`shape.regex.flags-refused`, has a case with a subject in each language. The behavioural
specification's `facade` arrays are gone: a facade's verdicts are its migration note's rows.
