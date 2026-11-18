# 0093 — Publish and fetch provider shapes and reports through the engine's broker client, found by the links a broker advertises

- **Status**: accepted
- **Date**: 2026-11-18
- **Plan tasks**: S4.3b (the Janus side of the broker PR series); S4.3d builds the broker half of
  decisions 2 to 4, and the link decision 5 names amends S4.3c's row 2
- **Evidence**: [ADR 0032](0032-speak-to-the-broker-through-protocol-operations-backed-by-a-native-host-capability.md),
  whose decision 7 left these out and whose tripwire this is;
  the [broker integration notes](../prototype/broker-integration-notes.md) §2, §3 and §11;
  [the broker PR series](../broker-pr-series.md) §4 and §7; the Ruby broker's index resource
  (`lib/pact_broker/api/resources/index.rb`) and its `POST /contracts/publish`;
  [ADR 0011](0011-contracts-as-self-identifying-json-documents.md) decision 6 and gate B;
  [ADR 0092](0092-decide-in-a-broker-through-the-engines-wasm-component-and-refuse-to-start-without-it.md)

## Context

ADR 0032 put the broker client behind the protocol and left three things out until something needed
them: publishing a provider shape, which no broker resource accepted; fetching shapes for
`janus check`; and anything over the broker's matrix. S4.3 now gives the broker a resource for
shapes (row 4) and a table for subsumption reports (row 6), and S4.3b is the client that uses them.
Its tripwire said a host needing the broker without a verification adds an operation. `janus check`
is that host.

Three things decide the boundary.

- **A report is a fact about two documents.** The broker stores a report keyed by (contract SHA,
  shape SHA, engine version) and treats it as a cache (notes §3). Whatever publishes the report must
  say exactly which contract and which shape it is about. A host that fetched a shape, checked it and
  published the report itself would carry the shape's identity from one call to the next. That is
  the orchestration ADR 0032 kept out of hosts by fetching and publishing inside the run.
- **A broker says what it can do through its links.** The Ruby broker's index advertises every
  resource as a HAL link, and already lets a feature switch decide whether it advertises one
  (`disable_pacts_for_verification` withdraws `pb:provider-pacts-for-verification`).
  The `janus` feature serves no new route and advertises no new link when it is off (series §4). So
  a link is how a client learns, per broker, what that broker stores. Rows 2 and 3 left this one
  question open: how a client learns that a broker takes `specification: "janus"` (series §7).
- **The client's pact with the broker is the broker half's specification.** The client already has a
  consumer pact with the Pact Broker (S1.3), verified against the pinned Ruby broker. Interactions
  for the new resources, written from what this client needs, are what S4.3d's pull requests
  implement and what the Rust port verifies, as it verifies its other pacts.

The notes (§2) sketched the shape resource as a `PUT` on the provider version. Since then the broker
has gained a better pattern: `POST /contracts/publish` creates the version, its branch and its tags
and stores the documents in one request, so a webhook fired by the publication already sees the
branch.

## Decision

1. **Two operations in the `broker` area, behind the `broker` capability.** Each runs in a broker
   session, reports on its stream and ends at its terminal `broker/finished`, as
   `broker/publish-contracts` does. An engine with no broker client answers each with
   `component-unavailable` naming `broker`.
   - **`broker/publish-provider-shape`** `{ broker, provider-version, provider-shape, branch?, tags?,
     build-url? }`. It publishes one provider shape for a version of the provider the shape names.
     The engine reads the shape before it publishes it, as `subsumption/check` reads one, so a
     document that is not a provider shape is refused in the response. What it publishes is the
     document as the host sent it, written as `publish-contracts` writes a contract: compact JSON,
     members in name order.
   - **`broker/check-contracts`** `{ broker, contracts, provider-version? | provider-branch?,
     publish-reports? }`. For each contract it fetches the shape of the contract's provider at that
     version, or the latest at that branch, runs `subsumption/check` on the pair, and, with
     `publish-reports`, publishes the report. Exactly one of `provider-version` and `provider-branch`
     is given, and `provider-version` only when every contract names the same provider. A shape is
     fetched once per provider. One `broker/checked` event per contract carries its outcome:
     `checked` with the report and `format`; `not-published` when the broker has no shape there; or
     `failed` with an `error`. A publication's own outcome (`published`, `failed` with an `error`, or
     `skipped` with a `reason`) never changes the check's outcome, as a failed result publication
     never changes a verdict (ADR 0032 decision 3).
2. **Deciding stays with the host's `subsumption/decide`.** `check-contracts` returns reports; it
   does not decide. The decision takes verification summaries, policy layers and a date the broker
   has no part in, and `janus check` already asks `decide` for it. A `not-published` contract is a
   pair with no report, which `decide` answers as `provider-shape-missing`: replay-only, the RFC's
   adoption path. A `failed` one is the command failing to run.
3. **The broker resources the client expects**, all advertised on the index only with the `janus`
   feature on:

   | Relation | Request | Answer |
   |---|---|---|
   | `pb:publish-provider-shape` | `POST`, a body as `pb:publish-contracts`' is: `pacticipantName`, `pacticipantVersionNumber`, `branch`, `tags`, `buildUrl`, and the shape as `contentType` and base64 `content` | `notices`, as a contract publication's |
   | `pb:provider-shape`, templated `{provider}`, `{version}` | `GET` `/pacticipants/{provider}/versions/{version}/provider-shape` | the shape, or 404 |
   | `pb:latest-branch-provider-shape`, templated `{provider}`, `{branch}` | `GET` `/pacticipants/{provider}/branches/{branch}/latest-version/provider-shape` | the shape, or 404 |

   A shape is served as the broker serves a pact: the stored document with `_links` added. The
   client reads two of them. `pb:provider-version`'s `name` is the version the shape belongs to.
   `pb:publish-subsumption-report` is where a report about this shape is published. A provider
   shape's reader ignores members it does not know, so the served document checks as published.
   A 404 from either `GET` means no shape was published there, whatever the broker could not find:
   the pacticipant, the version, the branch or the shape.
4. **A report is published to the shape's own link, with the contract it is about.** The body is
   `{ engineVersion, contract: { contentType, content }, report }`. The contract is base64 bytes
   written as `publish-contracts` writes them. The broker computes the contract's SHA as it computes
   a pact version's (rows 2 and 3), the shape's SHA from the resource the link names, and keys the
   report by the two and `engineVersion`. Sending the contract, rather than naming a consumer
   version, means the report is stored against the document that was checked even when that is not
   the one a consumer version holds. It also means a report and a publication of the same contract
   meet at one SHA, whichever arrives first. Whether a broker keeps a report about a SHA it holds no
   pact version for is the broker's to decide (S4.3d). A shape with no report link publishes
   nothing: the publication is `skipped`, `not-supported`.
5. **A Janus contract is published as `janus` where the broker advertises
   `pb:publish-janus-contracts`, and as `pact` everywhere else.** The relation's `href` is the
   publication's (`/contracts/publish` on both brokers). Its presence says that broker stores
   `specification: "janus"`. A v1–v4 pact is always `pact`. A broker without the relation is sent
   what it is sent today, so no deployed broker sees a change. Gate B's default does not move: what
   a broker that says nothing receives is still `pact`. Because a contract's SHA is the same under
   either word (series §7), a consumer whose broker starts advertising the relation republishes
   nothing and churns no version. The relation and the word carry the codename, and the client
   reads one and writes the other, so both are persisted identifiers (`persisted.rs`). The word sent
   is the spelling that matches the relation found.
6. **A missing capability fails by name.** `check-contracts` against a broker whose index has no
   `pb:provider-shape` (or no `pb:latest-branch-provider-shape`, for a branch) fails as
   `component-failed` naming `broker`, with `broker-response-invalid` saying the broker does not
   store provider shapes. It does not report every pair as `not-published`. That answer would make a
   broker without the feature look like a fleet of providers that published nothing, which `decide`
   would let through as replay-only. `publish-provider-shape` without `pb:publish-provider-shape`
   fails the same way.
7. **Credentials, client options and redaction are ADR 0032's** (decisions 5 and 6): the same
   descriptor, and the broker named by its URL without user information.
8. **The CLI.** `janus publish` publishes provider shapes as well as contracts, each document
   identified by its `$format`: contracts and pacts for `--consumer-app-version`, shapes for
   `--provider-app-version`. `janus check --broker-url` fetches each provider's shape at
   `--provider-version` or `--provider-branch` in place of `--provider-shape`, and `--publish`
   publishes the reports. The credentials are taken as `janus publish` takes them, by today's
   spellings and variables.
9. **Still out of scope:** fetching contracts or verification results from the broker. A decision
   over the broker's N×M matrix is `can-i-deploy`'s, which rows 7 and 8 give subsumption reasons
   (S4.3e). `janus check` decides over the contracts and summaries its pipeline already holds.

## Alternatives considered

- **Separate fetch and publish operations, with `subsumption/check` between them.** This is more
  composable, but every host would carry the shape's report link from the fetch to the publication.
  ADR 0032 rejected the same split for verification.
- **`check-contracts` deciding too.** This would make the broker operation a second `decide` with the
  broker's inputs bolted on. The policy layers and verification summaries are the host's, and
  `decide` is already the one place the rule lives.
- **A `PUT` on the provider version (notes §2).** Branch and tags would then be separate requests
  after the shape was stored, so a `provider_shape_changed` webhook could fire before the version had
  its branch. The publish-contracts pattern does it in one request, and both brokers already have it.
- **Naming the consumer version in a report publication.** Then the report would be stored against
  whatever that version holds, which may not be the contract that was checked. The broker would also
  refuse a report sent before the contract was published. Sending the contract keys the report by
  content, as the cache needs.
- **Inferring Janus contract storage from `pb:provider-shape`.** Both resources share one feature
  switch today, but they are separate capabilities. The plan asks for each to be found by its own
  link, and a broker that stores contracts under their word but not shapes is a reasonable
  deployment.
- **A property on `pb:publish-contracts`' link listing the specifications it takes.** HAL defines a
  link object's properties, and a client reading one more would rely on a broker writing outside
  them. A relation is the broker's own idiom for "this broker can".
- **Every pair `not-published` when the broker stores no shapes.** Rejected in decision 6: a missing
  capability would read as a pass.

## Consequences

- A consumer's pipeline checks against the shapes its providers published with one flag and no file
  passed between pipelines. A provider publishes its shape as it publishes a contract.
- The broker half of S4.3d is specified by the client's pact. The new interactions are in a second
  pact file, the `janus` feature's, which today's pinned broker cannot verify. It is verified against
  a broker with the feature on when one is named (`JANUS_PACT_BROKER_JANUS`), and S4.3d's pull
  requests carry it. Until a broker built from S4.3d exists, the client's tests run against a broker
  stand-in in `tools/test-servers` that serves what this ADR says. S4.3b's exit waits on the real one.
- `broker-pr-series.md` §7's open question is answered: row 2's broker half advertises
  `pb:publish-janus-contracts` with the feature on. That amends pact_broker#1094 and
  pact_broker-rs#24, or lands with row 4.
- Two persisted identifiers join `persisted.rs`: the relation and the `specification` word.
- The kernel's broker trait gains three methods: publish a shape, fetch one, publish a report.
- **Tripwire**: a host that needs a broker's stored *decision* rather than its documents (a
  dashboard, a GitHub check) means `can-i-deploy` has become the interface, which is rows 7 and 8's
  and not a new operation here. A second broker that advertises the same capabilities under other
  relations means the relations become configuration.
