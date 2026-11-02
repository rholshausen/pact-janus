---
engine: minor
---

# A consumer test's verdict reaches the engine: `consumer-session/report-variant`

A host reports the test's verdict on each variant once its closure ends, `passed` or `failed` with
a `reason`, and the engine answers with that variant's result: its own verdict on the exchange and
the test's together (ADR 0064, engine-protocol spec §8.2). A host learns in the test that ran a
variant that its request did not match, or that nothing exercised the mock. A report waits for an
exchange still being recorded and disarms the variant, so a later request finds nothing armed. A
reported failure withholds the contract. A variant result carries the report as `test`.

`engine/hello` declares `test-verdicts`. A host that declares it too reports every variant, and in
its sessions an exercised variant with no report is `not-reported`, an `add-interaction` the
session refused withholds the contract, and `finalise` lists the refusals as `refused`. A host that
does not declare it gets the results it got before.
