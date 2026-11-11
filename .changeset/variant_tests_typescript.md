---
typescript-sdk: minor
---

# Each variant a test of its own under Vitest and Jest, and what the mock did logged as the test runs

Staged plan S3.5, ADR 0084.

- **`await janus.variants(interaction, closure)`**, on the `Janus` that `pact-janus/vitest`'s or
  `pact-janus/jest`'s `useJanus` returns, registers one test per variant the engine selects, named
  `label [id]` (`base` for the base variant). Await it at a test file's top level (Vitest also in an
  async `describe`); Jest needs the file to be an ES module. A failing variant fails only its own test,
  with its closure's error or the engine's verdict. `variantTests` on any `Janus` answers the same tests
  as `{ name, run }`, for another framework.
- **What the mock made of each request is logged while the closure runs**: a request it rejected
  through `console.warn` (`pact-janus: 'get an order' [base]: the request did not match`, then each
  mismatch), one it matched through `console.debug`, as `logLevel` (else `LOG_LEVEL`, else `warn`)
  allows. The SDK asks for the session's events and polls them during each closure, which needs an
  engine that answers `create` with a `stream`; with an older one it polls nothing.
