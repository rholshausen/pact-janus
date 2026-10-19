# The verifier fixture corpus

Staged plan S1.1's corpus: pacts and the providers that answer them, which
[`tools/verifier-diff`](../../tools/verifier-diff/src/main.rs) verifies twice, once with today's verifier
(`pact_verifier_cli`) and once with `janus verify`, and compares interaction by interaction. "Exactly as
today" is measured here: every verdict the two disagree on is either in
[`disagreements.json`](disagreements.json) with its reason, or a failure.

```sh
.github/scripts/pact-verifier.sh target/pact-verifier   # the pinned pact_verifier_cli, checksum verified
.github/scripts/pact-broker.sh start                     # the pinned Pact Broker; prints its URL
cargo build -p pact_janus_cli --bin janus
PACT_VERIFIER_CLI=target/pact-verifier/pact-verifier-linux-x86_64 \
  cargo run -p pact_janus_verifier_diff -- run --broker http://127.0.0.1:9292
```

The broker is needed only by the broker cases (below): without `--broker`, a run that includes
them stops and says so, and `--filter` can leave them out. CI runs this as the `verifier-diff` job, and `cargo test` checks that every fixture and ledger entry
reads without running anything. One case takes about two minutes on its own: when a provider-state
call fails, today's verifier retries it before giving up (staged plan S1.9 records the difference).

## What a fixture is

A directory holding `fixture.json` and the pacts it names. The case id is the directory's path below
this one.

| Member | Meaning |
|---|---|
| `description` | What the fixture is for |
| `source` | Where it came from, for a reader |
| `pacts` | Pact files, relative to the directory |
| `provider` | `{ "kind": "order-service" }`: the sample provider, started with auth off, whose provider-state endpoint is the state callback. `{ "kind": "scripted", "state-callback": "none" \| "ok" \| "fail" \| "fail-once", "routes": [...] }`: the harness's own provider, answering each route. `fail` answers every state call with a 500; `fail-once` answers the first with a 503 and the rest with success |
| `expected` | `pass` or `fail`, when the source says what the run should conclude. Today's verifier is held to it, so a fixture it disagrees with is a fixture translated wrongly |
| `mutations` | Also run every interaction against mutated responses |
| `broker` | Fetch the pacts from a Pact Broker instead of files, and publish results back (below) |

The scripted provider recognises a request strictly by its example: method, path, query, the headers
the route names, and the body when the route has one. It answers anything else with a 404, as pact's
mock server does, so a verifier that sends the wrong request fails as it would against a real
provider. When several routes recognise a request, the one whose `states` were just set up answers.

Each verifier gets its own, freshly started provider, so nothing one verifier does to a provider (the
states it set up, a callback it has already made fail) changes what the other sees.

A mutation changes one thing in an interaction's response: the status, a header, a body value (to
another of its type, or to another type), a member removed, an array emptied or lengthened, a member
added. Its id says which, as in `order-service/mutations/0/0/body/$.shippedAt/same-type`, which is
pact 0, interaction 0. Nothing says which verdict a mutation should get. Both verifiers must give
the same one.

## Broker fixtures

A fixture with a `broker` member (staged plan S1.3) is published to a real Pact Broker, the Ruby
broker's own image at the version [`pact-broker.json`](../../tools/verifier-diff/pact-broker.json)
pins, and both verifiers fetch it from there. Each verifier's pacts are published under a prefix of
their own on every pacticipant name, so what one verifier publishes changes nothing the other
fetches, as each gets a provider of its own.

| Member | Meaning |
|---|---|
| `publish` | How a pact is published, by its file name: `version` (default `1.0.0`), `branch` (default `main`), `tags`. A pact not listed takes the defaults |
| `results` | Verification results there before the run, as an earlier provider build left them: `pact`, `provider-version`, `provider-branch`, `success`. This is how a fixture makes a pact *not* pending |
| `verify` | The options both verifiers get, in today's verifier's terms: `selectors` (consumer version selectors, as the broker reads them), `enable-pending`, `include-wip-pacts-since`, `provider-version` (default `1.0.0`), `provider-branch`, `publish` |
| `expected-published` | What the source says the run publishes for a pact, by its file name: `pass`, `fail` or `absent`. Today's verifier is held to it, as to `expected` |

A broker case is also compared on what each run published: key `<pact>/published`, read back from the
broker, `absent` when the run published nothing for that pact. An interaction the selectors did not
fetch is `absent` too. The case's pacts must share one provider, must not also be mutated, and their
interaction descriptions must be unique across them: today's report does not always say which
consumer a result is for, and the broker decides the order.

Each verifier reaches the broker through a recording proxy. What it asked (method, path, body,
status, with its prefix and the proxy's address replaced by `{prefix}` and `{broker}`) is in the
case's `broker` member of `run --json`'s output, and in `<case>/today|janus/broker-traffic.json`
under `--keep`. It is recorded, not yet compared.

`janus verify` is given today's option spellings (`--broker-url` and the rest), which it takes as
they are (S1.13's table records them). Its summary names the pacts it fetched (`consumers`), so an
interaction of a pact it did not fetch is `absent` there too.

## What is here

- `order-service/`: the sample provider's own v3 pact against the sample provider, and its mutations.
- `broker/`: pacts fetched from a broker: the sample provider's own pact; pending, and not pending
  because the provider's main branch verified it before; work in progress; a pact the selectors do
  not fetch; and pending asked for with no selectors, where today's verifier uses the broker's older
  endpoint and reports no pending status.
- `state-callbacks/`: provider-state callbacks that misbehave in ways the compatibility suite does not
  cover, such as `fails-once`, an endpoint that fails while it warms up.
- `compatibility-suite/`: the [pact-compatibility-suite](https://github.com/pact-foundation/pact-compatibility-suite)'s
  HTTP provider scenarios, V1 to V4 (Apache-2.0), each pact built as the suite's own harness builds it,
  its broker scenarios as broker fixtures.
  Generated: `cargo run -p pact_janus_verifier_diff -- import-compatibility-suite <checkout>`
  rewrites the directory, and its `import.json` records the commit and every scenario skipped, with
  the plan item that will let it in.

## The ledger

[`disagreements.json`](disagreements.json) lists every disagreement by case and key (`run` for the
whole run, `<pact>/<interaction>` for one interaction, `<pact>/published` for what a broker case
published) with both verdicts and a reason. A reason
starts with what accounts for it: the staged-plan item that closes it (`S1.6: …`), or the ADR that
makes it deliberate (`ADR 0007: …`). The harness fails on a disagreement the ledger does not list,
on one listed with the verdicts the other way round, and on an entry nothing disagrees on any more,
so closing a gap means deleting its entries in the same change. `run --skeleton` prints entries
for the unexpected disagreements, to fill in once each has a reason.

The comparison is by verdict. Where both verifiers fail an interaction for different reasons, for
example Janus because it has no XML component and today's verifier because the body differs, that
counts as agreement. The ledger entry for the passing case of the same body kind is what records
the gap.
