---
engine: minor
---

# Provider shapes and subsumption reports go through the engine's broker client

Staged plan S4.3b, ADR 0093. The Pact Broker client publishes and fetches provider shapes and
publishes subsumption reports, against the resources the brokers' `janus` feature serves (the broker
PR series, S4.3d). It finds each resource by the link the broker's index advertises.

- `broker/publish-provider-shape` publishes one provider shape for a version of its provider, with
  the version's branch, tags and build URL, in a broker session.
- `broker/check-contracts` fetches each contract's provider shape at a provider version, or the
  latest on a provider branch, checks the pair, and with `publish-reports` publishes each report
  beside its shape, with the contract it is about. One `broker/checked` event per contract: `checked`
  with the report, `not-published`, or `failed`. It returns reports; the host decides over them with
  `subsumption/decide`. A broker that stores no shapes fails every contract by name rather than
  reporting none published.
- `broker/publish-contracts` publishes a Janus contract as `specification: "janus"` to a broker whose
  index advertises `pb:publish-janus-contracts`, and as `pact` to every other broker, as before. A
  v1–v4 pact is always `pact`. No deployed broker advertises the link yet, so what any broker
  receives today is unchanged.
- A contract sent to `broker/publish-contracts` as its text (ADR 0063) is published as the document
  the text holds. It was published as a JSON string.
- `janus publish` publishes provider shapes as well as contracts, each document identified by its
  `$format`, a shape for `--provider-app-version`. `--consumer-app-version` is needed only when there
  are contracts to publish.
- `janus check --broker-url` takes each provider's shape from a broker, at `--provider-version` or
  the latest on `--provider-branch`, in place of `--provider-shape`. `--publish` publishes the
  reports. The credentials are taken as `janus publish` takes them.
