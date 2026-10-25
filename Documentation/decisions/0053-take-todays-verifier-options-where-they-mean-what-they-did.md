# 0053 — Take today's verifier options where they mean what they did

- **Status**: accepted
- **Date**: 2026-10-25
- **Plan tasks**: S1.13; S1.17 and [ADR 0041](0041-refuse-a-filter-that-names-no-recorded-variant-and-report-a-narrowed-run-as-filtered.md),
  whose rules for a filtered run the filters follow; S1.3 and
  [ADR 0032](0032-speak-to-the-broker-through-protocol-operations-backed-by-a-native-host-capability.md),
  whose broker client fetches pacts by URL
- **Evidence**: `pact_verifier_cli` 1.3.5's `--help`, and pact-reference's `pact_verifier_cli`
  (`pact_source`) and `pact_verifier` (`FilterInfo`, `filter_interaction`, `filter_consumers`,
  `fetch_pact_from_url`, `exit_on_first_failure`); `pact-provider-verifier`'s `cli/verify.rb`; the
  fixtures `corpora/verifier/filters/*` and `corpora/verifier/broker/webhook-callback-url`, run
  against both verifiers; [`verifier-migration.md`](../verifier-migration.md), S1.13's table

## Context

Stage 1's promise is that providers switch verifiers at no cost. S1.13 measures the cost of the
command line: a row for each of today's options, saying what it is under `janus verify`. Before this
item, a provider using today's most common options had to rewrite its CI script:
- `--hostname` and `--port`;
- `--state-change-url`;
- `--filter-*`;
- `--url` and `--webhook-callback-url`, which the Pact Broker's "contract requiring verification"
  webhook flow depends on;
- `--consumer-version-tags`;
- `--exit-on-first-error`.

Three facts shape the answer:
- **Two of these are engine work, not spellings.** A broker source's documents arrive inside the
  engine (ADR 0032), so a filter the CLI applied could not see them. A pact fetched by URL can be
  published only through the links it carries, with the credentials the broker client already
  holds.
- **Short options clash.** `-h` is clap's help, `-u` is `janus verify`'s `--provider-url`, and the
  Ruby verifier's `-c`, `-n`, `-p` and `-k` mean other things in `pact_verifier_cli`.
- **CI jobs set today's variables for every pact command.** `PACT_BROKER_BASE_URL` in particular,
  so honouring a variable must not break a run that names its files.

## Decision

1. **Today's long spellings and environment variables are taken wherever they mean what they did.**
   - `pact_verifier_cli`'s are taken as they are.
   - The Ruby verifier's are hidden aliases: `--provider-base-url`, `--pact-broker-base-url`,
     `--provider`, `--provider-app-version`, `--provider-version-branch`, `--provider-version-tag`,
     `--publish-verification-results`, `--broker-username`, `--consumer-version-tag`,
     `--consumer-version-selector`, `--provider-states-setup-url`, and `--custom-provider-header`
     in its own `Name: value` form.
   - `pact_verifier_cli`'s short options are taken where `janus verify` has them free: `-f`, `-d`,
     `-b`, `-n`, `-p`, `-s`, `-c`, `-t`, `-l`. `-h` and `-u` are not.
   - `--transport https` is the `http` transport over TLS, as today's scheme was. Every other value
     still names a transport component.
2. **One kind of source per run.** Files, a broker's selection or pacts by URL; a mix is refused.
   A broker URL that only `PACT_BROKER_BASE_URL` gave yields to files or URLs named on the command
   line. `--provider-name` has no default and is needed with `--broker-url`; today's default,
   `provider`, names no real provider.
3. **Today's state-change options write the hooks they become.**
   - `--state-change-url` writes a `state-setup` hook in `pact-state-change` format, or
     `pact-state-change-query` with `--state-change-as-query`.
   - `--state-change-teardown` writes the same hook at `state-teardown`.
   - Each is bounded by `--request-timeout` and reached with the provider's client options.
   - A configuration with `state-setup` hooks of its own is refused alongside the option, because
     the states would be set up twice.
4. **`verification/verify` takes `options.filter`**, with `description`, `state`, `no-state` and
   `consumers`, applied in the engine as today's `--filter-*` options are:
   - each regular expression is searched for, unanchored;
   - a state filter is met by any of the interaction's provider states;
   - the members are ANDed together;
   - an interaction left out has its variants counted as skipped.

   ADR 0041's rules hold:
   - a filter that admits no interaction is refused, as `filter-matches-nothing`, where today's
     verifiers verify nothing and pass;
   - a narrowed run is `filtered`;
   - a narrowed pact is not published.

   A consumer filter's other pacts are therefore counted and reported as skipped, where today's
   verifier leaves them out of its report.
5. **A `urls` source fetches each pact by its URL**, through a new broker-client operation,
   `fetch_pact`, with the run's `auth` and HTTP `client` options.
   - A pact is never pending.
   - It is publishable when a Pact Broker served it, which the client tells from its links. The
     kernel never reads them.
   - One from anywhere else has its publication `skipped`, with `reason: "not-from-a-broker"`.
   - `--url` and `--webhook-callback-url` both become this source. `--broker-url` is accepted beside
     the webhook's URL, as today requires it there.
6. **`options.stop-on-first-failure`** ends the run at the first failure that fails it:
   `--exit-on-first-error`. A pending failure does not stop it, because it does not fail the run;
   today's verifier stops there, then passes having verified less. The summary says
   `stopped: "first-failure"`, and a source the run did not finish is not published.
7. **Cheap mappings:**
   - `--consumer-version-tags` becomes `{ tag, latest: true }` selectors, as today.
   - `--retries` is the broker client's `retries`. Its default stays 2, where today's is 8.
   - `--ignore-no-pacts-error` is `require-pacts: false`.
   - `--no-colour` turns colour off.
   - `--loglevel` is an alias of `--log-level`, whose levels gain `off`, alias `none`.
8. **Not taken, or dropped, with a reason.** Each still needs a pilot provider's acceptance:
   - today's report formats (`--json`, `--junit`, `--html`, `--xslt`, the Ruby `--format`), since
     `--output json` writes Janus's run summary, a different document;
   - the log layouts;
   - `--last-failed`;
   - the Ruby verifier's `--wait`, `--tag-with-git-branch`, `--log-dir`, `--ignore-failures`,
     `--monkeypatch`, `--provider-states-url` and `--pact-urls`;
   - `PACT_BROKER_PUBLISH_VERIFICATION_RESULTS`;
   - the clashing short options.

## Alternatives considered

- **Keep Janus's spellings, and let the table translate.** Smaller, and nothing to keep in step.
  But every provider would edit every script, which is the cost stage 1 promised not to charge.
- **Filter in the CLI.** No protocol change. But a broker source's documents never reach the CLI,
  and an SDK would need its own copy, which "SDKs are thin" rules out.
- **Fetch URLs in the host and send them inline.** That is what the spec said a URL was. But a
  pact fetched that way cannot be published, and the webhook flow exists to publish. Credentials,
  TLS and proxies would also need a second client.
- **Refuse `--transport https`.** `--transport` names a component in Janus. But no component is
  called `https`, and every script that uses it means TLS.
- **Stop on a pending failure too, as today.** Exact parity. But it makes a pending pact fail a
  build it is defined not to fail, unless the build passes having verified less, which is the
  worse of the two.
- **Default `--retries` to 8.** Parity. But eight doubling waits keep a run that cannot reach its
  broker for about two minutes, which S1.3 chose against. The option is there for a provider that
  wants it.

## Consequences

- Most CI scripts move by changing the command's name. The differences are listed in the table,
  each with a fixture or a test, and the S1.1 harness compares the filters and the webhook flow
  against today's verifier.
- The protocol grows these additive members:
  - a source kind, `urls`, with `urls`, `auth` and `client`;
  - two options, `filter` and `stop-on-first-failure`;
  - an error code, `filter-matches-nothing`;
  - a summary member, `stopped`;
  - a publication reason, `not-from-a-broker`.

  The `BrokerClient` trait gains `fetch_pact`, and a fetched pact says whether it is `publishable`.
  An embedding with its own client implements both.
- `janus verify` reads more of the environment: `PACT_PROVIDER_HOSTNAME`, `PACT_PROVIDER_PORT`,
  `PACT_PROVIDER_TRANSPORT`, `PACT_PROVIDER_NAME`, `PACT_BROKER_BASE_URL`,
  `PACT_WEBHOOK_CALLBACK_URL`, `PACT_BROKER_HTTP_RETRIES`, the state-change variables, and the
  filters' variables. Each means what it did. A variable left over in a CI job can now change a run
  that never named the option, as it did under today's verifier.
- **Tripwire**: if a pilot provider cannot accept a dropped row, that row becomes an option. The
  report formats are the likeliest; an `after-verification` hook writing today's JSON report is the
  first answer to try.
