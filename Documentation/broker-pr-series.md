# The broker PR series

Staged plan S4.3 sends the [broker integration notes](prototype/broker-integration-notes.md)' §10 to
the Pact Broker as pull requests: to the Ruby Pact Broker (`pact-foundation/pact_broker`) and to its
Rust port (`YOU54F/pact_broker-rs`), each row one change made twice. The notes are a record, written
against Ruby's `fd6e5b63`, and are not updated. This document is the series' live working copy, and
its pull requests cite it. It was written for S4.3a.

It was checked against Ruby's `master` at **`9b6a3a62`** and the port's `main` at **`cd5fa88`**, both
on 2026-11-15. Ruby paths are under `lib/pact_broker/`, and the port's under `src/api/resources/`,
unless they say otherwise.

## Contents

1. [The rows and their sites](#1-the-rows-and-their-sites)
2. [What changed since the notes](#2-what-changed-since-the-notes)
3. [The embedding](#3-the-embedding)
4. [The `janus` feature](#4-the-janus-feature)
5. [The baseline: S1.3's checks against the port](#5-the-baseline-s13s-checks-against-the-port)
6. [The outline for the maintainers](#6-the-outline-for-the-maintainers)
7. [Rows 2 and 3: a Janus contract stored under its own word](#7-rows-2-and-3-a-janus-contract-stored-under-its-own-word)
8. [What the client asks of rows 2, 4 and 6](#8-what-the-client-asks-of-rows-2-4-and-6)

## 1. The rows and their sites

| Row | Change | Ruby | pact_broker-rs | Plan item |
|---|---|---|---|---|
| 1 | an unknown `can-i-deploy` badge is grey, as `can-i-merge`'s is | `badges/service.rb:36-41` (`deployable ? "brightgreen" : "red"` at `:39`); `can_i_merge_badge_url` at `:43-59` is the three-way precedent | `can_i_deploy_path/mod.rs:89`, the same two-way `if` on a `bool` | S4.3c. Ruby's half is merged: pact_broker#1093, `6298abd2`; the port's is pact_broker-rs#23 |
| 2 | `specification: "janus"` is stored | `api/contracts/publish_contracts_contract_contract.rb`; `contracts/contract_to_publish.rb:10` (`specification == "pact"`); `contracts/service.rb:133` (`select(&:pact?)`) and `:146-153` (`create_pact_params`) | `pacts/contracts.rs:279-286` (`must be one of: pact`) | S4.3c, with row 3: pact_broker#1094 and pact_broker-rs#24 (§7) |
| 3 | Janus-aware content extraction and SHA | `pacts/content.rb`, `pacts/generate_sha.rb`, `pacts/sort_content.rb` | `src/services/pact_content.rs` (`generate_sha_with` at `:27`, `content_that_affects_verification_results` at `:139`); `metrics.rs:66` (`content_counts`) | S4.3c, with row 2 (§7) |
| 4 | the provider-shape resource and its table | the version routes in `api.rb:106-122`, a migration under `db/migrations/` | `pacticipants/versions.rs`, an entity under `src/entity_pact_broker/`, a migration under `migrations/postgres/` and `migrations/sqlite/` | S4.3d |
| 5 | `provider_shape_changed` | `webhooks/webhook_event.rb:9-17` (the event names) | `webhooks/mod.rs:432-439` (`EVENT_NAMES`), `webhook_templates.rs` | S4.3d |
| 6 | stored subsumption reports | a table keyed (contract SHA, shape SHA, engine version), in row 4's migration | the same, in row 4's migrations | S4.3d |
| 7 | subsumption reasons in the decision | `matrix/reason.rb` (`Reason` at `:4-22`, `IgnoredReason` at `:24-39`, `Warning` at `:106-112`); `matrix/deployment_status_summary.rb` (`counts` at `:20-27`, `deployable?` at `:29-34`); `api/resources/can_i_deploy.rb`; `api/decorators/reason_decorator.rb` | `matrix_reasons.rs:282` (`Reason`), `can_i_deploy.rs`, `matrix/mod.rs:1020-1025` (`deployable`) | S4.3e |
| 8 | stored policy, and the exemptions page | `integrations/` (`integration.rb`, `repository.rb`, `service.rb`) | `integrations.rs`, the templates under `templates/` | S4.3e |
| 9 | recompute a report on a cache miss, for the pair asked about ([ADR 0095](decisions/0095-recompute-a-missing-report-for-the-pair-a-broker-is-asked-about-in-its-wasm-engine-and-never-a-whole-fleet-at-once.md)) | the embedding (§3) | the embedding (§3) | S4.5, built in S4.3e |
| 10 | a Janus contract renderer | `api/renderers/html_pact_renderer.rb:181-189` (`markdown`'s fallback) | `pacts/html.rs:148-155` | S6.10 |

Row 1's Ruby half merged on 2026-11-15, after the check above, as pact_broker#1093 (`6298abd2`). It
reopened pact_broker#1045, which was approved and closed only because a pull request from a fork
cannot pass that repository's required checks. The port's half is pact_broker-rs#23.

Every row but row 1 is behind the `janus` feature (§4). Each row's migration exists in both brokers,
any new column is nullable or defaulted, and the migrations run whether the feature is on or off.
Each row also adds the same operations to the OpenAPI document the port checks both brokers against.

## 2. What changed since the notes

Ruby's `master` is 124 commits past `fd6e5b63`. Of the lines the notes cite, these have moved or
changed:

- **`api.rb:106-121`** (row 4) is now `:106-122`. `8f8852c2` added
  `pacticipants/{name}/deployed-or-released/versions` among the version routes. The routes a shape
  resource sits beside are unchanged.
- **`pacts/generate_sha.rb`** (row 3) takes an optional `parsed_content` (`e5677086`, faster
  publishing of large contracts), so a contract is not parsed twice. Row 3's extraction has to accept
  it too.
- **`pacts/repository.rb`** changed in `dd4541a4` and `4594845a`: a pact version now records
  `has_http_interactions`, and typed v4 messages are counted as messages. The verbatim storage the
  notes cite (`content: json_content` at `:403`) has not moved. Row 3 has to say what a Janus
  contract records in these flags, and three new Ruby migrations (`20261111` to `20261113`) set
  them. The port has not taken those yet, which its issue #14 covers in part.
- **`api/renderers/html_pact_renderer.rb:186-191`** (row 10) is now `:181-189`. It renders from the
  parsed hash (`57e4f431`), and the fallback's note now reads "this content could not be rendered as
  a pact, showing raw JSON instead".

Every other cited line is where the notes say it is. Two of them do not do what the notes assumed:

- **`IgnoredReason` is dead code.** The notes (§5) name it as the precedent for an exempted finding,
  a reason kept and marked as ignored rather than dropped. `85540c85` (2021) removed every place one
  was built. Only the class and `ReasonDecorator`'s `Ignoring: ` prefix are left. The port has no
  counterpart: its `Reason` has no ignored variant. Row 7 therefore builds `IgnoredReason` again in
  Ruby, with the decorator it still has, and adds the variant to the port. It is still the right
  shape; it is just not one anything uses today.
- **`feature_enabled?` is on for every feature outside production** (§4).

## 3. The embedding

[ADR 0092](decisions/0092-decide-in-a-broker-through-the-engines-wasm-component-and-refuse-to-start-without-it.md)
records it. In summary, `janus-engine.wasm` answers `subsumption/check` and `subsumption/decide`
under Ruby's `wasmtime` gem (50.0.0) and under the `wasmtime` crate, granted nothing. Every frame of
a decision is identical to the native engine's: the sample provider's findings, an exemption the day
before it lapses, and the day after.
`engine/wasm/hosts/ruby/pipe.rb` is the Ruby host, and `engine/component-host/tests/engine_wasm.rs`
holds both runtimes to the native engine on every pull request.

What it found:

- **Before S4.3a the artifact could not match a shape selected by operation**, because it had no
  `http:path`. It now registers the HTTP component's matchers.
- **The gem needs Ruby 3.3 or later**, unless it is built from source with Rust. The Ruby broker
  supports 3.2. Its official image runs 3.4.10.
- **The costs.** Compiling the component takes 644 ms once, or 7 ms to load ahead-of-time output. A
  new instance takes 0.5 ms. Over the sample's documents, a decision takes 0.24 ms and a check 0.9 ms.
  A 1,500-interaction contract takes 575 ms to check, about 1.3 times native.

With the feature on, a broker that cannot load the artifact refuses to start (ADR 0092 decision 6).

S4.5 measured recomputing over a fleet
([ADR 0095](decisions/0095-recompute-a-missing-report-for-the-pair-a-broker-is-asked-about-in-its-wasm-engine-and-never-a-whole-fleet-at-once.md)).
A pair costs tens of milliseconds for typical contracts, linear in interactions, and linking the
kernel natively would save about a quarter of it. So both brokers keep the artifact. A report
the broker does not hold is computed when a decision needs it, for that pair, within a time budget
past which the pair is unanswered and computed off the request. Nothing recomputes a whole fleet
when a shape is published or the engine pin moves.

## 4. The `janus` feature

Both brokers already have one feature switch, `PACT_BROKER_FEATURES`. The series adds `janus` to it.

- **Ruby: `PactBroker.feature_enabled?(:janus, true)`** (`feature_toggle.rb:22`). The second
  argument matters. Without it, Ruby turns *every* feature on whenever `RACK_ENV` is not
  `production`, so a development or test broker would serve the series unasked. The port does not do
  this: its `feature_enabled` (`src/config/settings.rs:537`) reads only `features`, because Ruby's
  image runs in production. With `ignore_env` set, the two brokers agree everywhere.
- **The port: `crate::config::settings::feature_enabled("janus")`**, set by `PACT_BROKER_FEATURES`
  or the configuration file's `features`. The embedding is also behind a Cargo feature, so a build
  without it carries no WASM runtime.
- **What off means.** With the feature off, no new route is served, no new link is advertised, no new
  event name is accepted, `specification: "janus"` is refused as it is today, and the artifact is not
  loaded. Each broker has a test that a client sees nothing different with the feature off. The
  switch lands with row 2, the first pair that needs it. Row 1 is a fix and is not behind it.

## 5. The baseline: S1.3's checks against the port

S1.3's three broker checks, run on 2026-11-15 against the port at `cd5fa88`, built with
`cargo build --release` and run on an empty SQLite database with its defaults (it seeds example data,
as Ruby does):

| Check | Against the port |
|---|---|
| The differential harness's broker cases (`pact_janus_verifier_diff run --broker … --any-version`) | **No difference.** 1,383 verdicts, 1,370 agreeing and the 13 the ledger lists, no unexpected and none stale, including what each run published. |
| The client's own pact with the broker (`engine/broker-client/tests/pact.rs`), verified by the pinned `pact_verifier_cli` 1.3.5 | **Every interaction verifies**, each provider state set up through the port's API. |
| The recorded scenario (`engine/broker-client/tests/recorded.rs`), recorded against the port (`RECORDINGS=overwrite`) and diffed with the recording from Ruby 2.121.2 | **The client concludes the same at every step.** Four responses differ (below). |

The four differences, none of which changes what the client does:

| # | Difference | Ruby 2.121.2 | The port | Where it goes |
|---|---|---|---|---|
| B1 | the broker's version | every response carries `X-Pact-Broker-Version` | no such header | [pact_broker-rs#20](https://github.com/YOU54F/pact_broker-rs/issues/20) |
| B2 | a pact fetched by its `metadata` link | has a `pb:consumer-versions` link, naming the consumer version | no such link | [pact_broker-rs#21](https://github.com/YOU54F/pact_broker-rs/issues/21) |
| B3 | a pacticipant created by `POST /contracts/publish` | `displayName` generated from its name (`recorded-order-api` is "Recorded Order Api") | `displayName` is the name. The port has `generate_display_name` (`src/services/display_name.rs`), but not on this path | [pact_broker-rs#22](https://github.com/YOU54F/pact_broker-rs/issues/22) |
| B4 | the notice when a version's content changes | a unified diff of the pretty-printed contracts | a diff of the compact JSON, one line | the port's own backlog: "Ruby `999ed399` (unified diff rendering) against the `similar`-based diff", under *To verify* in `docs/PARITY_BACKLOG.md`, so not reported again |

Members also come back in a different order in most HAL documents. JSON does not order them and no
client reads anything into it, so it is not reported.

One change here came out of the baseline. The harness refused a broker that does not say its version
even with `--any-version`. It now accepts one, since that option checks no version.

To run the baseline again, with the port listening on 9393:

```bash
RECORDINGS=overwrite JANUS_PACT_BROKER=http://127.0.0.1:9393 JANUS_PACT_BROKER_ANY_VERSION=1 \
  cargo test -p pact_janus_broker_client --test recorded      # then diff the recording, and restore it
JANUS_PACT_BROKER=http://127.0.0.1:9393 PACT_VERIFIER_CLI=target/pact-verifier/pact-verifier-linux-x86_64 \
  cargo test -p pact_janus_broker_client --test pact -- --nocapture
PACT_VERIFIER_CLI=target/pact-verifier/pact-verifier-linux-x86_64 \
  cargo run -p pact_janus_verifier_diff -- run --broker http://127.0.0.1:9393 --any-version
```

S4.3f runs the demonstration against both brokers in CI. Until then the baseline is run by hand,
when the series moves to a newer port.

## 6. The outline for the maintainers

What both brokers' maintainers are shown before any pull request beyond row 1 (notes §7.5), which
needs no outline: it is a fix. Whether
the series merges is theirs to decide.

> **Janus in the Pact Broker: a proposed series of pull requests**
>
> Pact Janus is a ground-up redesign of Pact's matching, based on the Pact MkII RFC. Its consumers
> publish *contracts* rather than v1–v4 pacts. Its providers can publish a *provider shape*: what
> they may send, recorded from their own tests or derived from an OpenAPI or AsyncAPI document. A
> subsumption check then finds what a provider may send that no consumer has tested. We would like
> the broker to store these documents and to give that check's answer in `can-i-deploy`. Today a
> Janus contract can be published only as `specification: "pact"`, and a provider shape cannot be
> published at all.
>
> We propose one series, each pull request made to both the Ruby broker and pact_broker-rs, with one
> migration and one set of OpenAPI operations between them. Everything after the first is behind a
> `janus` entry in `PACT_BROKER_FEATURES`. With it off, neither broker answers any differently from
> today.
>
> 1. An unknown `can-i-deploy` badge renders grey, as `can-i-merge`'s does. This is a fix, worth
>    having anyway, and is already merged in the Ruby broker (pact_broker#1093); the port's is
>    pact_broker-rs#23.
> 2. `specification: "janus"` is accepted and stored.
> 3. A Janus contract's content SHA and diff read its interactions.
> 4. A provider shape resource, `PUT`/`GET /pacticipants/{provider}/versions/{version}/provider-shape`
>    and the latest by branch, stored verbatim with its SHA and provenance.
> 5. A `provider_shape_changed` webhook event.
> 6. Stored subsumption reports, which `janus check` publishes, used as a cache, keyed by contract
>    SHA, shape SHA and engine version.
> 7. Subsumption reasons in `can-i-deploy`, decided on read, because an exemption can lapse with no
>    document changing. The decision comes from the Janus engine's released WASM component, hosted
>    with `wasmtime` (the gem in Ruby, the crate in the port), granted no capabilities, so the broker
>    never implements the rules itself. In Ruby it needs Ruby 3.3 or later, and the gem only when the
>    feature is on.
> 8. A subsumption policy stored on the integration, with an exemptions page that shows who exempted
>    what, why, and until when.
>
> Rows 2 and 3 are small and stand on their own. Rows 4–8 are where the design is, and we would value
> your view on the resource shapes (row 4) and on where the policy lives (row 8) before writing them.
> The design notes behind this are
> [broker-integration-notes.md](prototype/broker-integration-notes.md), and the site-by-site map is
> this document.

## 7. Rows 2 and 3: a Janus contract stored under its own word

S4.3c's rows 2 and 3 are one pull request in each broker, a commit per row:
[pact_broker#1094](https://github.com/pact-foundation/pact_broker/pull/1094), on Ruby's `6298abd2`,
and [pact_broker-rs#24](https://github.com/YOU54F/pact_broker-rs/pull/24), on the port's `cd5fa88`,
opened on 2026-11-16. They decide the following, the same way in both.

- **A contract is recognised by its content.** Neither broker records the specification a pact
  version was published under, and adding a column for it would be a migration rows 2 and 3 do not
  otherwise need. A contract identifies itself by its `$format` ([ADR 0011](decisions/0011-contracts-as-self-identifying-json-documents.md),
  contract-file spec §2.3), so with the feature on, a document whose `$format` is `janus-contract/1`
  is read as a Janus contract wherever pact content is read, whichever word published it. A later
  major is not recognised, since it may change what an interaction holds. With the feature off,
  nothing is.
- **`specification: "janus"` must be a Janus contract.** It is accepted only with the feature on,
  and only for content whose `$format` is `janus-contract/1`, refused otherwise as `content published
  with specification 'janus' must be a Janus contract, whose $format is 'janus-contract/1'`. A
  contract published as `pact`, as every client does today (gate B), is still stored. `contentType`
  stays `application/json`: the media type a contract may be served as is a hint (contract-file
  spec §2.3), and the publish request's `contentType` says how `content` is encoded.
- **The SHA is the interactions alone, and is the SHA a pact would get.** Everything that changes
  what a verifier does is inside an interaction (contract-file spec §3.2), so a contract's content
  SHA covers `interactions`, sorted as a pact's are, and not `metadata`, `$format`, a stray `messages`
  member or a `pactSpecification` a contract must not carry. For a contract written as the
  specification says, that is exactly the SHA the broker gave it before the feature existed. So the
  same contract is one pact version whether it was published as `janus` or as `pact`, turning the
  feature on churns no version and fires no webhook, and a client that moves from `pact` to `janus`
  when gate B moves republishes nothing. `contract_content_changed` fires when, and only when, an
  interaction changes. Both brokers' tests pin the fixture's SHA, `21d089c0…`, which Ruby, the port
  and an independent computation agree on.
- **States, messages and merges read the contract's own members.** A contract's provider states are
  each interaction's `states`. Its messages are its emissive interactions (contract-file spec §4.3):
  Ruby's `messages_count`, `interactions_count`, `has_messages` and `has_http_interactions`, and the
  port's metrics, count them as Ruby counts typed v4 messages. It has no `messages` array and no pact
  specification version. A `PATCH` or an `onConflict: merge` publish matches an interaction by its
  description and states (contract-file spec §4.2) and replaces it. Two contracts conflict when one
  interaction is recorded differently, ignoring the `_id` the broker adds, as ADR 0065 decides for a
  merged file. A contract and a pact always conflict, and a merge of the two replaces. Before, Ruby's
  merger read a `request` a Janus interaction does not have, and raised.
- **The diff needs nothing.** The previous-distinct diff sorts and strips `_id` generically, and a
  test in each broker shows a changed contract diffed.
- **How a client learns the broker stores `janus` is S4.3b's.** Neither branch advertises it. The
  client finds each capability by the link the broker advertises (S4.3b), and S4.3b's ADR decides
  that link; its broker half amends row 2 or lands with row 4. Decided since by
  [ADR 0093](decisions/0093-publish-and-fetch-provider-shapes-and-reports-through-the-engines-broker-client-found-by-the-links-a-broker-advertises.md)
  decision 5: with the feature on, the index advertises `pb:publish-janus-contracts`, whose `href`
  is the publication's (§8).
- **The brokers read two persisted names.** `janus` and `janus-contract/1` are spelled once in each
  broker (`PactBroker::Janus`, `services::janus`). A rename of the codename
  ([codename.md](codename.md)) adds the new spellings there, beside the old, as `persisted.rs` does
  here.

What each branch's tests cover, in Ruby's `spec/features/publish_janus_contract_spec.rb` and the
port's `tests/publish_janus_contract.rs`: with the feature on, a contract is stored, counted, deduped
across a metadata change and across the two words, a new version with `contract_content_changed`
when an interaction changes, diffed, and gives its states; a pact published as `janus` is refused.
With it off, `janus` is refused with today's message and nothing is created, and a contract published
as `pact` is stored as it always was. Unit tests in each broker cover recognition, the SHA, the
counts and the merger.

## 8. What the client asks of rows 2, 4 and 6

S4.3b built the Janus side first, so the broker half has a client to answer
([ADR 0093](decisions/0093-publish-and-fetch-provider-shapes-and-reports-through-the-engines-broker-client-found-by-the-links-a-broker-advertises.md)).
What the client asks is in its own pact with the broker, in a second pact file, the `janus`
feature's, written by `engine/broker-client/tests/pact.rs`. That file is these rows' specification
from the client's side: S4.3d's pull requests carry it, and both brokers verify it with the feature
on (`JANUS_PACT_BROKER_JANUS` names such a broker to the client's own test). In short, with the
feature on and only then:

| Index relation | Request | Answer | Row |
|---|---|---|---|
| `pb:publish-janus-contracts` | the publication's `href` (`/contracts/publish`); a Janus contract sent as `specification: "janus"` | as `pb:publish-contracts` | 2 |
| `pb:publish-provider-shape` | `POST`, a body as `pb:publish-contracts`' is: `pacticipantName`, `pacticipantVersionNumber`, `branch`, `tags`, `buildUrl`, and the shape as `contentType` and base64 `content` | `notices` | 4 |
| `pb:provider-shape`, templated `{provider}`, `{version}` | `GET /pacticipants/{provider}/versions/{version}/provider-shape` | the shape, or 404 | 4 |
| `pb:latest-branch-provider-shape`, templated `{provider}`, `{branch}` | `GET /pacticipants/{provider}/branches/{branch}/latest-version/provider-shape` | the shape, or 404 | 4 |

A shape is served as a pact is: the stored document with `_links` added. The client reads
`pb:provider-version`'s `name`, the version the shape belongs to, and `pb:publish-subsumption-report`,
where a report about this shape is published (row 6). A report's body is
`{ engineVersion, contract: { contentType, content }, report }`. The broker keys it by the SHA it gives
the contract (rows 2 and 3), the shape's SHA and `engineVersion`, as notes §3 asks, and answers 201.
Whether it keeps a report about a contract no pact version holds is the broker's to decide.

The notes (§2) sketched a `PUT` on the provider version. The publish-contracts pattern replaces it,
so the version's branch and tags arrive with the shape and a `provider_shape_changed` webhook sees
them (ADR 0093, alternatives).

Until a broker built from S4.3d can be run, the client and the CLI are tested against a stand-in
that serves this table, `tools/test-servers`' `BrokerStandIn`.
