---
default: minor
---

# The engine protocol specifies how to verify from, and publish to, a Pact Broker

ADR 0032 decides that the broker client is the engine's, so no host speaks the broker's API. The
engine-protocol spec now specifies it, and the generated bindings carry the new types
(`BrokerDescriptor`, `BrokerAuth`, `ConsumerVersionSelector`, `PublishResults`, `VerifyOptions`,
`PublishContracts`):

- `verification/verify` takes a source of kind `broker` (the broker, the provider, consumer version
  selectors, the provider's branch and tags, pending and work-in-progress pacts), fetched inside the
  run, and an option `publish` that sends each pact's result to that pact's own link. A pending
  pact's failures do not fail the run.
- `broker/publish-contracts` publishes a consumer's contracts, reporting on a stream.
- The `broker` capability says whether an engine has a client. New event kinds:
  `verification/fetched`, `verification/published`, `broker/published`, `broker/finished`. New error
  code: `pacts-not-found`.

No engine has a broker client yet (staged plan S1.3 builds it), so every engine answers a `broker`
source, a `publish` option and `broker/publish-contracts` with `component-unavailable` naming
`broker`. A `broker` source used to be answered with `operation-unsupported`.
