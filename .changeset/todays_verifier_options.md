---
default: minor
---

# `janus verify` takes today's verifier options, and the protocol gains filters, pacts by URL and a stop

`janus verify` now takes `pact_verifier_cli`'s long spellings and environment variables wherever they
mean what they did, and the Ruby verifier's as aliases (ADR 0053; `Documentation/verifier-migration.md`
has a row for every option).

Protocol, all additive:

- **`options.filter`** on `verification/verify`: `description` and `state` (regular expressions, searched
  for), `no-state`, `consumers`. An interaction it leaves out is skipped; a run it narrows is `filtered`
  and its narrowed pacts are not published (ADR 0041). One that matches no interaction is refused with a
  new error code, `filter-matches-nothing` (category `session`), carrying the `filter` and how many
  `interactions` there were. A filter that cannot be read is `malformed-frame`.
- **A `urls` source**: `{ kind: "urls", urls, auth?, client? }`. Each pact is fetched by its URL in the
  run, is never pending, and is published to its own links when a Pact Broker served it; otherwise its
  publication is `skipped` with `reason: "not-from-a-broker"`. `options.publish` is allowed with it.
- **`options.stop-on-first-failure`**: the run ends at the first failure that fails it (a pending one does
  not), and the summary says `stopped: "first-failure"`.
- `verification.schema.json` now also lists `options.variants` and `options.variant`.

For an embedding with its own broker client: `BrokerClient` gains `fetch_pact`, and `FetchedPact` a
`publishable` member.

CLI:

- Where the provider is: `--hostname`, `--port`/`-p` and `--transport https` when there is no
  `--provider-url`, which is no longer required on its own.
- Sources: `--file`/`-f`, `--dir`/`-d`, `--url`, `--webhook-callback-url`, `-b` for `--broker-url`,
  `-n` for `--provider-name`, and `PACT_BROKER_BASE_URL`, `PACT_PROVIDER_NAME`,
  `PACT_WEBHOOK_CALLBACK_URL`. One kind of source per run; a broker URL only the environment gave yields
  to files or URLs on the command line. `--publish` works with `--url` too.
- `-s`/`--state-change-url`, `--state-change-as-query`, `--state-change-teardown` write the state hooks.
- `--filter-description`, `--filter-state`, `--filter-no-state`, `--filter-consumer`/`-c`.
- `--exit-on-first-error`, `--consumer-version-tags`, `--retries`, `--ignore-no-pacts-error`, `-t` for
  `--token`, and, on every command, `--no-colour` and `--loglevel`, whose levels gain `off` (alias `none`).
