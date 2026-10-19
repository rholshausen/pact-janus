---
default: minor
---

# `janus verify` and `janus-engine` verify from a Pact Broker, and publish results back

Both native binaries now have a Pact Broker client (ADR 0032), and `engine/hello` declares the
`broker` capability. The offline WASM artifact still has none, and still answers every
broker-facing request with `component-unavailable` naming `broker`.

- **`verification/verify` takes a source of kind `broker`.** The run fetches the pacts the
  consumer version selectors choose, with pending and work-in-progress status, and reports them as
  a `verification/fetched` event before verifying anything. A fetch that fails ends the run with
  `component-failed` naming `broker`, carrying the client's own error (`broker-unreachable`,
  `broker-refused`, `broker-response-invalid`). Selectors that select nothing end it with
  `pacts-not-found`, unless the source says `require-pacts: false`.
- **Pending pacts and pending interactions don't fail the run.** This covers a pending pact, a
  work-in-progress pact, and a v4 interaction marked `pending` from any source, including a file.
  Their results say `pending: true` and are still counted in `variants`. A broker run's summary
  says per source whether it was `pending`.
- **`options.publish` publishes each pact's result to that pact's own link** when the run ends,
  keyed by the broker's interaction ids, which now ride in each interaction reference as `id`. It
  records the provider version's branch and tags first. Each publication is a
  `verification/published` event, and the summary lists them as `publication`. A publish that
  fails is reported and does not change the verdict. `publish` with an `inline` source is refused
  as `malformed-frame`.
- **Every link is fetched from the broker you named**, as today's verifier does. Of a link the broker
  writes, only the path and query are kept, and the broker URL's context path is added when the link
  lacks it. So a broker behind a proxy that writes links with an internal host name still works.
- **The client retries only what waiting can fix.** A broker that does not answer, or answers
  with a 5xx, is tried three times in all, with a short backoff. A 429 is retried only when it carries
  `Retry-After`, after that delay. PactFlow answers 429 for a plan's limit, and no retry gets past
  that. A `Retry-After` longer than 10 seconds is reported at once, with the delay, rather than waited
  for. Everything else the broker refuses is reported at once as `broker-refused`.
- **`janus verify` has today's broker options**, spelled as `pact_verifier_cli` spells them:
  `--broker-url`, `--provider-name`, `--consumer-version-selectors`, `--enable-pending`,
  `--include-wip-pacts-since`, `--provider-version`, `--provider-branch`, `--provider-tags`,
  `--publish`, `--build-url`, and `--user`/`--password`/`--token`, which also answer to
  `--broker-user`/`--broker-password`/`--broker-token`. The credentials can also come
  from `PACT_BROKER_USERNAME`, `PACT_BROKER_PASSWORD` and `PACT_BROKER_TOKEN`. As with today's
  verifier, pending and work-in-progress pacts are asked for only when selectors are given.
- **Short flags, and help for a bare command.** Every command's `--log-level` is also `-l`, as
  today's verifier spells it, and `janus verify`'s `--provider-url` is also `-u` and its `--output`
  `-o`. Today's verifier reads `-u` as a pact URL (`--url`), so a moved script that passes one gets
  a usage error. Every command that takes arguments (`verify`, `check`, `explain`, `upgrade`,
  `publish`, `component push`, `component pull`, `completions`) prints its help when given none,
  and exits 2 as before.
- **No credential is logged, at any level.** At `trace`, `janus` and `janus-engine` log every frame
  with each broker descriptor's `auth` reduced to its `kind`, and each `url` without user
  information. The HTTP library's own logging stops at `debug`, where it redacts headers, because
  its `trace` output dumps raw request bytes, `Authorization` included. Below `debug` it follows
  the level asked for, like everything else.

What the ledger says: every broker fixture now agrees with today's verifier, including what each run
publishes, and so does the compatibility suite's pending v4 interaction. The disagreements left are
plain-text, XML, form, multipart and binary bodies (S1.6), a repeated response header (S3.3), and a
provider-state call that is not retried (S1.9).
