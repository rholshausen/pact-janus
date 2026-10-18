# 0032 — Speak to the Pact Broker through protocol operations backed by a native host capability

- **Status**: accepted
- **Date**: 2026-10-18
- **Plan tasks**: S1.2 (decision); S1.3 (the build), S1.14 and S1.15 (the SDKs' provider side)
- **Evidence**: the [broker integration notes](../prototype/broker-integration-notes.md) §9 and §11;
  [Phase 9 finding 10](../prototype/phase-9-findings.md) (per-pair summaries);
  [ADR 0013](0013-component-hosting-is-an-embedding-capability.md) (what a native
  embedding brings that the kernel lacks); the Pact Broker's API as read in the local `pact_broker`
  checkout at `e7a52bc6` (`PactsForVerificationJsonQuerySchema`, `VerifiablePactDecorator`,
  `VerificationContract`); `pact_verifier`, which fetches, verifies and publishes in one call

## Context

Stage 1a ships a provider verifier that works "with a broker" (staged plan S1.3): fetching pacts by
consumer version selectors, with pending and work-in-progress pacts, and publishing results back. The
engine protocol's verification section put fetching out of scope: "fetching from files, URLs or a
broker is host/CLI business in the prototype, which also keeps I/O out of the WASM kernel"
(engine-protocol spec §8.3). That was the right call for a prototype with one host. It does not
survive stage 1b: most providers verify inside a test (S1.14, S1.15), through an SDK, so a broker
client in the CLI is a broker client every SDK must write again, in its own language, under a
thinness audit (task 6.5) whose whole point is that SDKs do not orchestrate.

Three things the broker hands a verifier make the choice of boundary matter. Each fetched pact
carries `pending` and `wip` status and notices, and a pending pact's failures must not fail the run.
Each carries its own link for publishing results, which encodes the selectors that fetched it. And
the results are published per pact, keyed by the broker's interaction ids. A host that fetched pacts
itself would have to carry all three through the verification and back out again.

The kernel knows no HTTP (AGENTS.md's first architecture rule), so whatever speaks to the broker is
not kernel code.

## Decision

1. **The broker client is the engine's, behind the protocol.** No host speaks the Pact Broker's API:
   not the CLI, not an SDK. A host asks the engine, in frames, as for everything else.
2. **It is a native host capability.** The kernel defines a trait in the broker's terms — fetch the
   pacts for verification for these selectors; publish this verification result for a pact it
   fetched; publish these contracts — with no HTTP in it. A native crate implements the trait over
   the broker's HAL API, and the native embeddings, `janus` and `janus-engine`, register it, as they
   register the hook implementations and the WASM loader (ADR 0013). The engine declares the
   `broker` capability only when a client is registered; the WASM offline artifact has none, and
   its broker operations answer `component-unavailable` naming `broker`.
3. **Verification fetches and publishes inside the run.** `verification/verify` gains a source of
   kind `broker` (the broker, the provider, consumer version selectors, the provider's branch and
   tags, pending and WIP), fetched in the run, and an option `publish` (the provider version, branch,
   tags and build URL) that publishes each pact's result to that pact's own link when the run ends.
   A pending pact's failures are reported and do not fail the run. A fetch that fails ends the run as
   any run that cannot complete does: a terminal event with `status: failed` and an `error`
   (engine-protocol spec §9.6). A publish that fails is reported per pact and does not change the
   verdict.
4. **Consumer-side publishing is an operation of its own.** `broker/publish-contracts` publishes Janus
   contracts or v1–v4 pacts for a consumer version. It reaches the network, so it reports on a stream,
   in a broker session that ends at its terminal event, as a verification session does.
5. **Credentials travel in the request and nowhere else.** The broker descriptor every operation
   takes carries the URL and the credentials. The engine never reads them from its environment; a
   host does, by whatever names it supports (`PACT_BROKER_TOKEN` and the rest, S1.13), and passes
   them. They never appear in an event, a summary, an error or a log: the broker is named by its URL
   with any user information removed, and the lifecycle-hooks spec's redaction rules (§7.3) apply.
6. **One HTTP client configuration.** TLS, proxies, timeouts and retries are the same client options
   the HTTP component's client half takes (S1.5), carried in the broker descriptor.
7. **Out of scope until something needs them:** publishing a provider shape, which no broker
   resource accepts yet (broker notes §2; S4.3); `can-i-deploy`, which stays the broker's own
   (S1.3); and fetching contracts or shapes for `janus check`. Each would be a new operation in the
   `broker` area, additive within protocol v1.

This replaces the engine-protocol spec's §8.3 clause quoted above; the spec says so where it said
the opposite.

## Alternatives considered

- **The CLI speaks to the broker, as the prototype assumed.** Every SDK's provider integration
  (S1.15) and every ecosystem that wraps the CLI (S1.16) would need its own client, and the thinness
  audit would rightly fail each one.
- **A fifth component interface**, so a third party could ship a broker client (a PactFlow-specific
  one, say). Nobody needs to swap the client today, and an interface in WIT is a surface to keep
  compatible forever. The trait is where that interface would go; tripwire below.
- **Separate `broker/fetch` and `broker/publish-results` operations**, with an inline source between
  them. More composable, but every host would carry each pact's pending status, publishing link and
  interaction ids from one call to the next, which is the orchestration SDKs must not do.
- **Hooks**: fetch in `before-verification`, publish in `after-verification`. A hook acts on a run;
  it does not supply the run's sources, and consumer-side publishing is not a run at all.

## Consequences

- A provider integration in any language verifies from a broker in one frame, as `pact_verifier`
  does in one call, and the SDK specification's provider side (S1.14) has nothing broker-shaped to
  specify beyond the source and the option.
- The kernel gains a trait and, in its sources, per-pact metadata (pending, the publishing link, the
  broker's interaction ids) that the run carries into its results. Per-pact results in the summary
  are Phase 9 finding 10's option A, which S1.3 builds.
- The offline WASM artifact cannot reach a broker, by design. The broker's own embedding (broker
  notes §10, row 9) recomputes subsumption over documents it already holds and needs no client.
- Secrets now cross the protocol, so a frame can carry a credential. Redaction stops being a hook
  concern only and becomes part of the protocol's own rules.
- **Tripwire**: a second broker client implementation, or a host that needs the broker without a
  verification (`janus check` over a broker's N×M matrix), reopens this. The first promotes the trait
  to a component interface; the second adds an operation.
