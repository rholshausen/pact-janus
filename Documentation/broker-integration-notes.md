# Broker integration notes

Plan task **7.5**, which closes Phase 7. Status: **final**. Design notes: no build, no schema, no ADR —
the plan scopes broker integration as "design notes for the RFC, not prototype code" (§10, task 7.4),
and nothing here reopens a decision that is already made.

These notes start where the [contract file format review](contract-file-format-review.md) §8.4 stopped.
That section settled how a *contract* reaches a broker, and
[ADR 0011](decisions/0011-contracts-as-self-identifying-json-documents.md) decision 6 made the posture —
**compatible by default, enhanced when available** — the design rather than a transitional measure. What
it does not cover is everything Phase 7 added: the **provider shape** (task 7.2/7.3), the **subsumption
report** (7.1), and the **compatibility decision** (7.4). Those are three artifacts that did not exist
when the format review's §8.4 was written, and exactly one of them turns out to be hard.

Every claim about broker behaviour below was read in the local `pact_broker` checkout at **`fd6e5b63`** —
the same revision §3 of the format review used — and is cited by file and line. PactFlow is a separate,
commercial codebase; nothing here is asserted about it, and where its behaviour would matter that is said
rather than guessed.

A bare `§n` below refers to a section of these notes; sections of other documents are named with the
document they belong to.

## Contents

1. [The four artifacts, and which of them has a home](#1-the-four-artifacts-and-which-of-them-has-a-home)
2. [The provider shape has no request shape that fits](#2-the-provider-shape-has-no-request-shape-that-fits)
3. [Who runs the walk](#3-who-runs-the-walk)
4. [Decisions are computed on read, because exemptions expire](#4-decisions-are-computed-on-read-because-exemptions-expire)
5. [The broker already answers three-valued — keep it that way](#5-the-broker-already-answers-three-valued--keep-it-that-way)
6. [Where the policy lives](#6-where-the-policy-lives)
7. [Rendering: reuse the report's own words](#7-rendering-reuse-the-reports-own-words)
8. [Events](#8-events)
9. [What a team can do today, against an unmodified broker](#9-what-a-team-can-do-today-against-an-unmodified-broker)
10. [The change set, as a PR series](#10-the-change-set-as-a-pr-series)
11. [What this asks of Janus, not of the broker](#11-what-this-asks-of-janus-not-of-the-broker)
12. [What these notes do not decide](#12-what-these-notes-do-not-decide)

## 1. The four artifacts, and which of them has a home

| Artifact | Produced by | Keyed to | In today's broker |
|---|---|---|---|
| Janus contract ([2.5](specs/contract-file/spec.md)) | the consumer build | (consumer version, provider) | **stored, deduped, diffed, feeds `can-i-deploy`** — as `specification: "pact"` (format review §8.4) |
| Provider shape ([2.8 §2](specs/subsumption-check/spec.md)) | the provider build (7.2), or derived from types (7.3) | a **provider version** | **no resource accepts it** (§2 below) |
| Subsumption report (2.8 §6) | `subsumption/check` over one (contract, shape) pair | (consumer version, provider version) | nothing |
| Compatibility decision (`janus-compatibility-report/1`, [protocol §8.6](specs/engine-protocol/spec.md)) | `subsumption/decide`, at deploy time | a *question*, not an artifact | `can-i-deploy` answers the verification half of it |

The shape of the problem is the opposite of what the format review expected. The contract — the artifact
the whole of §4 of that review agonised over — is the one that already works everywhere, because ADR 0011
spent three member names to make it work. **The provider shape is the binding constraint**, and not for
any reason a format choice could fix.

## 2. The provider shape has no request shape that fits

The format review's §8.4 established that widening the `specification` allowlist alone publishes
nothing, because `ContractToPublish#pact?` (`lib/pact_broker/contracts/contract_to_publish.rb:10`)
filters the contract out at `lib/pact_broker/contracts/service.rb:133`. For provider shapes the problem
is one layer earlier,
and it is structural rather than a missing branch:

- `PublishContractsSchema` requires a top-level `pacticipantName` and `pacticipantVersionNumber`, and
  **validates that each contract's `consumerName` equals `pacticipantName`**
  (`lib/pact_broker/api/contracts/publish_contracts_schema.rb:9-10,25-32`);
- `ContractsService#create_pact_params` then builds every pact with
  `consumer_name: parsed_contracts.pacticipant_name` (`lib/pact_broker/contracts/service.rb:146-153`).

So `POST /contracts/publish` does not merely accept pacts — it models **the publishing pacticipant as the
consumer of everything in the request**. A provider shape is published by the provider, about itself,
with no consumer anywhere in it: 2.8 §2.1 is explicit that a shape "carries no evidence, no variants, and
no consumer name, because one provider shape is compared against every consumer that has a pact with that
provider." There is no assignment of the existing fields that expresses that. Any encoding that squeezed
it in — the provider publishing a contract whose consumer is itself, say — would create a spurious
pacticipant edge, a spurious matrix row, and a `can-i-deploy` answer about a relationship that does not
exist. That is worse than the silent-discard failure the format review's §8.4 warned about, because it
would look like it
worked.

**A provider shape needs its own resource.** The version-scoped routes already in the API
(`lib/pact_broker/api.rb:106-121`) are the natural place, because they already carry the branch, tag and
environment machinery that `can-i-deploy`'s selectors resolve against:

```
PUT  /pacticipants/{provider}/versions/{version}/provider-shape
GET  /pacticipants/{provider}/versions/{version}/provider-shape
GET  /pacticipants/{provider}/branches/{branch}/latest-version/provider-shape
```

What the broker stores, and why each part:

| Stored | Why |
|---|---|
| the bytes, verbatim | same reason pacts are stored verbatim (`pacts/repository.rb:403`): the file is read by people, and reformatting it loses the formatting the writer chose |
| a content SHA over the whole document | "did the provider's shape change" is the event that invalidates every downstream answer (§8). A shape has no `interactions`-versus-metadata split to exclude — unlike a contract (format review §8.5), nothing volatile is in it — so the whole document hashes |
| `provenance`, extracted | it is policy input and reader context (2.8 §2.3), and a UI that shows `derived` next to a wall of findings has explained most of them |
| the association to the provider version, branch, tag, environment | so a shape resolves by the same selectors a verification result does |

**The document carries no version of its own, and should not start.** A shape's identity comes from the
request that published it, exactly as a pact's consumer version does. That keeps one shape comparable
against every consumer (2.8 §2.1) and keeps the artifact honest about what it is: a claim about what this
build of the provider produces, not about who asked.

**Partial publication is normal and must survive storage.** 2.8 §6.3's `not-published` exists because a
provider may publish shapes for some interactions and not others — that is the RFC's per-provider
adoption path. A broker that merged shapes across provider versions to "fill in" missing interactions
would be inventing evidence: the union of what two different builds produced is a claim neither build
made. Store each version's shape as published, and let the walk report `not-published` where it belongs.

## 3. Who runs the walk

This is the question 7.1–7.4 raises that the format review could not have. A subsumption result is a
fact about a **pair** — (consumer version, provider version) — and both sides change independently.
There are three
places the walk can run, and the choice decides how much of the broker this feature is.

**(a) The broker reimplements the walk in Ruby.** Rejected, and not marginally. The walk is composition
over the shape language's per-operator comparability classes ([2.2 §8](specs/shape-language/spec.md)),
and the vocabulary is open — components contribute operators and their own `compare`
([2.6 §7.5](specs/component-interfaces/spec.md)). A second implementation would have to track the first
forever, and the first failure mode is the worst one available: shape spec §3.7 says an engine meeting an
operator it does not implement fails **by name**, so two implementations do not disagree about a verdict,
they disagree about which operators exist. The RFC's whole premise is one reference engine; a broker-side
matcher is the second one.

**(b) CI publishes reports, the broker stores them.** This is what protocol §8.6 anticipates when it says
`check`'s report is "the artifact a broker would store", and it is right as far as it goes — but on its
own it goes stale silently. A stored report is a function of two documents; publishing a new provider
shape invalidates the stored report of **every consumer** of that provider, and the consumer whose report
is now wrong has no build running. The broker can detect this (the shape's SHA changed) but cannot fix
it, so the best it can do unaided is mark N reports unknown and wait for N consumer builds that may be
weeks apart.

**(c) The broker embeds the engine and recomputes. Recommended.** The two operations were designed into a
shape that makes this small, for reasons that had nothing to do with brokers:

- both are **session-less, document-in/document-out** (protocol §8.6), like `upgrade/pact`;
- neither touches the network, the filesystem, or a provider;
- **the engine has no clock** — `as-of` is supplied by the host (2.8 §7.2), specifically so the same
  documents decide the same way on every engine;
- the kernel builds for `wasm32-wasip2` today (verified while writing these notes:
  `cargo build -p pact_janus_kernel --target wasm32-wasip2`), and
  [ADR 0013](decisions/0013-component-hosting-is-an-embedding-capability.md) makes hosting an embedding
  capability rather than a kernel one.

So the broker's dependency is a `.wasm` file and a JSON document, not a Rust toolchain, a subprocess, or
a network call to a service someone has to operate. The guest needs no WASI capability beyond what a pure
function needs, and a Ruby `wasmtime` binding exists — that last point is the one claim in these notes
taken on report rather than read, and it is the first thing to check before committing to this route.
This is the same "compatible by default, enhanced when available" posture ADR 0011 took, applied one
layer up: a broker without the embedding degrades to (b), a broker with it answers the question at any
time about any pair.

**The two combine, and should.** Store what CI published, keyed by `(contract SHA, provider-shape SHA,
engine version)`, and treat it as a cache; recompute on a miss. The cache is sound precisely because the
walk is deterministic and clock-free — the property 2.8 §7.2 demanded for CI reproducibility turns out to
be the property that makes broker-side caching correct. Include the engine version in the key because
`kind` is an open vocabulary (2.8 §9): a newer engine may report a finding an older one did not, and a
cached report from the older one is not wrong so much as incomplete.

## 4. Decisions are computed on read, because exemptions expire

2.8 §7.2 states the consequence plainly: "under `on-finding: block`, a deploy that passed yesterday can
block today with no change to any document." An exemption lapses by date, and the lapse is a real state
change with no event behind it.

Therefore: **cache reports, never decisions.** A report is a function of two documents and is timeless; a
decision is a function of a report, a policy and a date. A broker that stored `can-i-deploy: yes` for a
pair would serve yesterday's answer the morning an exemption lapsed, which is the one morning the answer
matters. The compatibility report's `exemption-lapsed` (warn) and `expiry-not-evaluated` (note) reason
codes exist so that this is visible in the answer rather than inferred from it.

This is also where the clock lands. The engine has none by design (protocol §8.6); the *host* supplies
`as-of`, and `janus check` already defaults it to today (`cli/src/check.rs`). A broker is just another
host in that respect — but it is the one that answers the same question repeatedly over inputs that
stopped changing, so the date becomes the only moving part between two otherwise identical answers.
That is the whole argument for storing reports and computing decisions: everything date-free is
cacheable, and the one thing that is not must not be. A dashboard listing exemptions expiring this month
is then a query the broker can actually run, which no CLI invocation can.

## 5. The broker already answers three-valued — keep it that way

The single most encouraging finding in this review: `can-i-deploy` is **already** three-valued.
`DeploymentStatusSummary#deployable?` returns `false`, `nil` or `true`, and returns `nil` specifically
when a row's `success` is unknown or a required integration has no row at all
(`lib/pact_broker/matrix/deployment_status_summary.rb:29-34`); `counts` reports `success`, `failed`,
`unknown` and `ignored` separately (`:20-27`). That is the same Kleene shape the walk produces and 2.8
§6.2 aggregates, and it means the hardest conceptual work — teaching a deploy gate that "I don't know" is
not "no" — is already done and shipped.

The matrix row is also already the right key: a row is (`consumer_version_id`, `provider_version_id`,
`pact_version_id`) with verification details left-outer-joined
(`lib/pact_broker/matrix/matrix_row.rb:38-69`, and the comment at `matrix/reason.rb:71-78` recording that
the join yields a row with blank verification details). **A subsumption result is a second column on a
row that already exists**, not a new axis — which is why §2's recommendation to key provider shapes to a
provider *version* matters: it is what makes the result land on that row.

The reason vocabulary maps cleanly onto the broker's own, which is `Reason` subclasses carrying a `type`
of `:info`, `:warning` or `:error` (`matrix/reason.rb:4-22,106-112`):

| Compatibility report reason (protocol §8.6) | Broker `Reason` type | Note |
|---|---|---|
| `verification-failed` | `:error` | already `VerificationFailed` |
| `verification-missing` | `:error` | already `PactNotEverVerifiedByProvider` — **and stronger than `janus check`'s `warn`**, correctly (below) |
| `verification-incomplete`, `verification-filtered` | `:warning` | **new** — today an aborted or filtered run is indistinguishable from a complete one once the summary is stored |
| `subsumption-findings`, `subsumption-reviews` | `:error` or `:warning`, per policy | the only two the policy governs |
| `provider-shape-missing`, `interactions-not-published` | `:info` | **must not be an error, and must not be silence** |
| `subsumption-exempt`, `exemption-no-expiry`, `exemption-unused` | `:info` | the dashboard material 2.8 §7.2 asks for |
| `exemption-lapsed` | `:warning` | §4 |

**One row of that table is a deliberate divergence, not a mismatch.** `janus check` raises
`verification-missing` at `warn` because "no summary covers this pair" may only mean the host did not
hand one over — the CLI sees the files it was given and cannot tell absence from omission. A broker can:
it holds every verification result there is, so a missing one means the verification does not exist,
which is the broker's existing `PactNotEverVerifiedByProvider` error. The severity of a "missing input"
reason depends on whether the answerer is authoritative about what exists, and the broker is the first
component in this system that is. Worth stating in the RFC, because it is the one place where the same
reason code should *not* produce the same severity in both places.

Two cautions, both verified:

- **The `IgnoredReason` wrapper is the right precedent for exemptions** (`matrix/reason.rb:24-39`): it
  keeps the root reason and marks it ignored rather than deleting it. That is exactly rule 2 of protocol
  §8.6 — an exempted finding is still a `no`, listed with `disposition: "exempt"`, never dropped. Whoever
  implements this should reach for `IgnoredReason`'s shape and not invent a second one.
- **The badge collapses the third state.** `can_i_deploy_badge_url` renders `deployable ? "brightgreen" :
  "red"` (`lib/pact_broker/badges/service.rb:39`), so `nil` — unknown — renders red, while
  `can_i_merge_badge_url` right below it has the three-way case and renders `nil` as a grey "unknown"
  (`:43-59`). Adding subsumption makes `nil` far more common (`not-published` on any provider that has
  not adopted shapes), so this inconsistency stops being cosmetic. It is a two-line fix in the OSS
  broker and worth sending on its own merits, independent of Janus.

## 6. Where the policy lives

`janus check` resolves policy in layers — specification defaults, then `--policy`'s document, then the
two flags — with **scalars overriding and `exemptions` accumulating**
([ADR 0016](decisions/0016-subsumption-defaults-to-warn-with-mandatory-reason-exemptions.md), 2.8 §7.1).
Protocol §8.6 deliberately passes the layers to the engine as an unmerged *list*, because "a host that
merged the layers itself would be reimplementing that rule, and two hosts would eventually disagree about
a team's accepted exemptions." A broker asked to decide is such a host, and inherits that constraint: it
passes its layers, it does not merge them.

The design question is whose layers those are. A policy file in the consumer's repository is invisible at
deploy time — the deploy is often a different pipeline, sometimes a different team, and `can-i-deploy` is
asked about a version that was built days ago. Three candidate homes, and the recommendation:

| Home | For | Against |
|---|---|---|
| inside the contract | travels with the artifact | violates 2.5 §3.2's placement rule (a policy changes no verifier's behaviour) and lets a consumer exempt itself from the provider's findings — the exemption would be a self-signed certificate |
| a file in the repo, passed at query time | already works; it is what `janus check --policy` does | invisible to a broker UI, no audit trail, and every pipeline that asks the question needs its own copy |
| **a broker resource on the integration** (consumer↔provider pair — `lib/pact_broker/integrations/`) plus a per-query override | the broker has users, timestamps and a UI, which is what a mandatory `reason` and an optional `expires` were asking for all along | a new resource, and teams who want policy-as-code need the file layer to still work |

**Recommended: the broker becomes one more layer, not a different mechanism.** Specification default →
broker-stored policy on the integration → per-query override, resolved by the rule ADR 0016 already
fixed. That keeps `janus check` and the broker answering identically when handed the same inputs, which
is the property that makes a local reproduction of a CI failure meaningful.

The audit-trail argument deserves its own line, because it is the strongest one. 2.8 §7.2 requires
`reason` and makes `expires` optional but "a smell a report or dashboard SHOULD surface". A file in a
repo answers "why" and "until when" but not "who" and "when did they decide"; a broker resource answers
all four for free, from machinery it already has. And the three `:info` reason codes — `exemption-unused`,
`exemption-no-expiry`, `subsumption-exempt` — are precisely a dashboard's rows. That dashboard is the
thing 2.8 §7.2 names as task 7.5's, and this is it:

```
Exemptions — orders-ui → orders-api
  response.body.status   expires 2026-12-01   "CANCELLED is a known future state; ORD-451"   in use
  response.body.tracking no expiry            "third-party field, never narrowed"            in use
  response.headers.etag  expired 2026-09-01   "…"                                            LAPSED, now blocking
  response.body.priority expires 2027-01-01   "…"                                            unused for 43 days
```

**Do not fold this into the existing `--ignore`.** `RowIgnorer` drops whole matrix rows by pacticipant, or
by pacticipant *and* version (`lib/pact_broker/matrix/row_ignorer.rb:27-32`) — per-run, no reason, no
expiry, and coarse by design ("some provider that is not ready yet"). Subsumption exemptions are
per-path, reasoned and dated. They are different instruments and collapsing either into the other loses
the point of it; the interesting direction, if anything, is the broker borrowing `reason` and `expires`
for its own ignores.

## 7. Rendering: reuse the report's own words

**The text form is specified, so a broker should render it rather than paraphrase it.** 2.8 §6.4 fixes
the per-interaction block and §6.5 fixes the phrase table; protocol §8.6 fixes the `~` marker for an
exempted finding and requires the RFC's own `provider may produce` / `consumer has only tested` lines
"verbatim, so the RFC's own sketch is reproducible". A broker UI that wrote its own prose for the same
finding would give a team two vocabularies for one fact and make "the CI output and the dashboard
disagree" a support question. The phrase table is a shared asset; treat it as one.

Three views, in order of value:

1. **The pair view** — the subsumption report for (consumer version, provider version), rendered as §6.4
   specifies, with the verification line and the decision around it. This is the page the RFC's scenario
   ends on, and it is where a reader decides whether to widen a shape or exempt a field.
2. **The provider-shape view** — one page per provider version: `provenance`, which interactions are
   covered, and which consumers currently fail against it. The last column is the one a provider team
   cannot get today, and it is the whole value of publishing a shape.
3. **The contract view** — today a Janus contract renders as pretty-printed JSON under the broker's
   "could not be parsed to a v1 or v2 Pact" note (`api/renderers/html_pact_renderer.rb:186-191`, §3 of
   the format review). That degrades honestly and is the **least** urgent item in that review's change
   set; findings and shapes are what nobody can see at all.

**One opportunity worth naming.** The broker diffs pact content as text. The interesting diff between two
provider shapes is not textual — it is "which fields widened", which is a subsumption walk of the new
shape against the previous version of the *same* provider's shape. Same operation, same report, both
sides provider-owned; it answers "did this release change what we may produce in a way that could break
someone" before any consumer is involved. That is a better provider-side diff than anything a text diff
gives, and it costs nothing beyond what §3 already builds.

## 8. Events

The broker's event vocabulary is `contract_published`, `contract_content_changed`,
`provider_verification_published`, `contract_requiring_verification_published`, plus the verification
succeeded/failed pair (`lib/pact_broker/webhooks/webhook_event.rb:9-17`). The consumer side is already
free: because a Janus contract names its interaction array `interactions`, the broker's
`content_that_affects_verification_results` hashing works on it unchanged, so `contract_content_changed`
fires when and only when something that matters changed (format review §8.5).

**One new event is needed: `provider_shape_changed`**, fired when a published shape's content SHA differs
from the previous version's. It is the trigger that invalidates cached reports (§3) and the natural hook
for "tell the consumers of this provider that the response space moved". Everything else stays as it is —
a subsumption finding is not an event, it is a property of a pair that is recomputed on read (§4), and
inventing `subsumption_finding_raised` would create a notification whose truth expires with a date nobody
watched.

## 9. What a team can do today, against an unmodified broker

Stated plainly, because it is the honest answer to the RFC's unresolved question:

| Step | Today |
|---|---|
| publish the Janus contract | **works** — `POST /contracts/publish`, `specification: "pact"`, `contentType: "application/json"`; stored, deduped, diffed, webhooked, `can-i-deploy` edge created (format review §3, ADR 0011 decision 6) |
| publish the provider shape | **nothing fits** (§2). Interim: a build artifact, an OCI artifact, or a file in the provider's repo |
| publish the subsumption report | nothing. Interim: a CI artifact |
| decide | `janus check` locally, over files: contracts or v1–v4 pacts, `--provider-shape`, `--verification` (what `janus verify --json` wrote), `--policy`, `--on-finding`/`--on-review`, `--as-of`. Exit 1 on `block` |

So **M5's loop is fully reproducible today, and fully reproducible only locally.** The subsumption half
of `can-i-deploy` runs in CI against files a pipeline already has; it does not run in a broker, and the
reason is §2, not the format and not the protocol. A team could adopt the whole of Phase 7 tomorrow by
passing the provider's shape to the consumer's pipeline as a build artifact — which is exactly what
`samples/order-service/shapes/` demonstrates — and would lose the thing a broker is for: the N×M
cross-product, resolved by version selectors, without every pair being wired up by hand.

## 10. The change set, as a PR series

Extending the format review's §8.4 table with what Phase 7 adds. Ordered so each row lands with value on
its own, and so that nothing before row 9 requires the engine embedding.

| # | Sites | Change | Value on its own |
|---|---|---|---|
| 1 | `badges/service.rb:36-41` | render `deployable == nil` as grey/unknown, matching `can_i_merge` | fixes a wrong badge today, Janus or not |
| 2 | `api/contracts/publish_contracts_*.rb`, `contracts/contract_to_publish.rb:10`, `contracts/service.rb:133` | first-class `specification: "janus"` (format review §8.4's series) | contracts stop being stored under someone else's word |
| 3 | `pacts/content.rb`, `generate_sha.rb`, `sort_content.rb` | Janus-aware content extraction | correct dedup and diffs for Janus contracts |
| 4 | new resource + table, on `api.rb:106-121`'s version routes | publish, store and retrieve a provider shape (§2) | providers can publish a shape at all; a shape page and a shape-vs-previous diff (§7) |
| 5 | `webhooks/webhook_event.rb` | `provider_shape_changed` | the "response space moved" notification |
| 6 | new table keyed `(contract SHA, shape SHA, engine version)` | store subsumption reports CI published (§3b) | the pair view (§7.1) for pairs CI has checked |
| 7 | `matrix/reason.rb`, `deployment_status_summary.rb`, `api/resources/can_i_deploy.rb` | subsumption reasons in the decision, three-valued already (§5) | `can-i-deploy` answers the RFC's question |
| 8 | new resource on `integrations/` | stored policy and exemptions, with the dashboard (§6) | the audit trail, and policy visible where the decision is made |
| 9 | a `wasmtime` embedding of the engine | recompute on cache miss (§3c) | the cross-product answered without waiting for N consumer builds |
| 10 | `api/renderers/html_pact_renderer.rb` | a Janus contract renderer | replaces an honest degradation with a good page |

Rows 1–3 are the format review's series plus a bug. Rows 4–8 are what Phase 7 adds and are where the
design work is. Rows 9–10 are improvements over a system that, by then, already works.

## 11. What this asks of Janus, not of the broker

Three things fall on this side of the line, and are recorded here rather than acted on, since 7.5 is
notes:

- **Per-pair verification summaries.** [Phase 9 finding 10](phase-9-findings.md) lists three options for
  a summary that cannot say how many variants *one pair* ran, and notes the decision is worth making with
  these notes in hand. These notes come down on **option A**: a broker stores verification results keyed
  by pair, and a run-level tally is unusable for that. A broker offered option C — "tally the event
  stream yourself" — would be reimplementing a summary the protocol already writes, which is the same
  mistake as reimplementing the walk (§3a), one document smaller.
- **A publish surface.** `janus check` reads local paths; nothing in the CLI publishes anything. Whatever
  publishes a contract today is the Pact Broker CLI, and a provider shape has nowhere to go (§2). If the
  prototype ever demonstrates the broker loop, the missing piece is a `publish` verb that speaks the
  broker's HTTP API, and per the CLI's own rule (`cli/src/main.rs`, plan task 5.5) anything it cannot do
  through the protocol is a finding about the protocol first.
- **Compression on the publish path.** The format review's §8.2 risk — reverse proxies capping request
  bodies at 1 MB, well below any format tripwire — applies to contracts with a large variant multiplier.
  Still unverified against a real deployment, and still the right answer rather than a different format.

## 12. What these notes do not decide

- **Anything about PactFlow.** The `specification: "oas"` provider-contract support is commercial and not
  in the OSS tree; it is plausibly the nearest existing precedent for §2's resource, and a design
  conversation with that codebase's owners would be worth more than inference from the outside.
- **Whether recomputation scales.** §3c's recommendation assumes the walk is cheap relative to a broker
  request. The prior is good — the walk is structural over shapes with no I/O, and the format review §8.3
  measured even an 823 KB document parsing in 2.1 ms — but the N×M case on a large fleet is unmeasured,
  and 9.1's performance work is where a number would come from.
- **Whether a provider shape is keyed to a version or to a branch.** §2 says version, because that is
  what makes a matrix row work; a fleet whose providers publish per commit may want the latest-per-branch
  view to be the primary one. That is a UX question a real deployment answers.
- **Message interactions.** 2.8's walk is direction-agnostic and a shape carries whatever parts exist;
  nothing here assumes HTTP. But the hook and message design is deferred beyond the prototype (plan §13),
  so the broker view of a message-only provider is untested.

These notes are input to [9.2](project-plan.md) — the RFC's "broker/PactFlow handling of v5" unresolved
question — and to 9.4's staged plan, where §10's series is the shape of the upstream contribution.
