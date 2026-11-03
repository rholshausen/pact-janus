---
typescript-sdk: minor
---

# `pact-janus/pact`: pact-js's consumer API

`pact-janus/pact` exports pact-js's names: `Pact` (`PactV4`), `PactV3`, `Matchers` (`MatchersV3`),
`XmlBuilder`, `SpecificationVersion`, and the provider side's `Verifier`, `MessageProviderPact` and
`providerWithMetadata`, so a pact-js suite moves by changing its import (ADR 0066). The matchers are
pact-js's own data; the builders, HTTP and GraphQL, record the `pact_ffi` calls pact-js's would make,
and `executeTest` hands them to the engine's `consumer-session/add-pact-interaction`, which needs an
engine with that operation. A test runs once, against its example, and records the Janus contract
`janus upgrade` would write from the pact pact-js writes, in `<dir>/<consumer>-<provider>.janus.json`,
merged with other test files'. `usingPlugin`, `tls` and, until S2.5c, the message API are refused by
name; `pending`, `comment`, `testName` and `reference` are accepted and do nothing. Several
interactions in one `executeTest` are refused until S2.13. `Documentation/consumer-test-migration.md`
gives each primitive's verdict.
