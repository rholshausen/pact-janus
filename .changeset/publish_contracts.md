---
default: minor
---

# `janus publish` and `broker/publish-contracts` publish a consumer's contracts to a Pact Broker

`broker/publish-contracts` is built (engine-protocol spec §8.7, ADR 0032). It publishes Janus
contracts or v1–v4 pacts, all for one consumer, for a consumer version with its branch, tags and
build URL. Each is published as the broker's `pact` specification, so any Pact Broker since 2.86.0
accepts it.

- **The documents are checked first.** Every document is read as `verification/verify` reads an
  inline one, so the operation refuses a document it could not verify
  (`contract-version-unsupported` or `contract-invalid`, with pointers under `/contracts`) and
  documents from two consumers (`malformed-frame`) before it contacts the broker.
- **What is published is the document as sent,** including members this engine does not know,
  written as compact JSON with its members in name order.
- **The outcome arrives on a stream**, in a broker session: one `broker/published` per contract,
  then `broker/finished`, which carries the broker's URL (without credentials), the consumer and its
  version, every contract's outcome, and the broker's notices as text. The broker accepts or refuses
  a publication whole. A refusal, or a broker that cannot be reached, is `failed`, with
  `component-failed` naming `broker` and carrying the client's own error.
- **Compression only where the broker reads it.** The client gzips a publication only when the
  broker sends `Accept-Encoding: gzip` (RFC 7694), and resends it uncompressed after a 415. Today's
  Pact Broker sends no such header and refuses a gzipped body, so publications go uncompressed.

`janus publish <contract-or-pact>...` drives it, with the options of today's `pact-broker publish`:
`--consumer-app-version` (`-a`), `--branch`, `--tag` (`-t`), `--build-url`,
`--broker-base-url` (`-b`), `--broker-username` (`-u`), `--broker-password` (`-p`) and
`--broker-token` (`-k`). The broker URL and credentials can also come from `PACT_BROKER_BASE_URL`,
`PACT_BROKER_USERNAME`, `PACT_BROKER_PASSWORD` and `PACT_BROKER_TOKEN`. Documents from several
consumers are published one consumer at a time. The exit status is 0 when everything was published,
and 2 otherwise. Not carried over: `--auto-detect-version-properties`, `--tag-with-git-branch` and
`--merge`.
