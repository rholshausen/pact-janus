---
jvm-sdk: minor
---

# Each variant a row of its own under Spock, and what the mock did logged as the test runs

Staged plan S3.5, ADR 0084.

- **Spock**: `janus.variants(interaction)` in a feature's `where:` block answers one `VariantRun` per
  selected variant, and `janus.run(variant) { mock, v -> }` runs it, failing with the closure's own
  exception or the engine's verdict. A feature named `"#variant"` unrolls into one iteration per variant,
  named `label [id]`. A message's rows are `MessageVariantRun`s, run with a `MessageTest`.
- **What the mock made of each request is logged while the test runs**, to the SLF4J logger
  `io.pact.janus.engine`: a request it rejected at `warn` (`'get an order' [base]: the request did not
  match`, then each mismatch), one it matched at `debug`. The SDK asks for the session's events only
  when an SLF4J provider is bound; with none, the engine's own warning on its stderr says it, as before.
