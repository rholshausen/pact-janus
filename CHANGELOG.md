## 0.2.0 (2026-10-20)

### Breaking Changes

#### The `janus` command line is rebuilt: `--json` becomes `--output json`, and `upgrade --out` becomes `--write`

Two flags are renamed, so a script that runs `janus` must change to move to this version:

- `--json` is now `--output json` on `verify`, `check`, `upgrade`, `component push` and
  `component pull`. `--output text` is the default. This frees `--json <file>` for parity with
  `pact_verifier_cli`, where it writes a report to a file.
- `janus upgrade --out <file>` is now `--write <file>`, so it cannot be confused with `--output`.

The exit codes are unchanged: `0` done, `1` the subject failed, `2` the command could not run.
Everything else is new or fixed:

- **Stricter command lines.** A usage error now names the problem and shows the usage of the
  subcommand it is about, and exits 2. Mistakes that used to be ignored are now refused: a value
  that is not `warn` or `block`, an `--as-of` that is not a calendar date, an `--index` that is not
  a number, extra arguments, and flags that cannot be used together (`explain --spec` with `--index`
  or `--variant`; `explain --plan` with `--executed`; `upgrade --output json` with `--write` or
  `--quiet`). `component push --help` and `component pull --help` now exist.
- **`janus verify` no longer gives up on a slow run.** It used to fail with "the run stopped
  producing events" whenever 10 seconds passed with no events, such as a slow provider or a slow
  state hook. It now waits until the run ends.
- **A closed pipe is no longer a crash.** `janus verify ... | head` used to panic when `head`
  stopped reading. The command now runs to its end and exits with its own answer.
- **Help and usage errors name the program `janus`** on every platform; on Windows they said
  `janus.exe`. The same goes for `janus-engine`.
- **`janus check --output json`** now prints the "no provider shape supplied for ..." warning on
  stderr, as the text form always did.
- **New:** `--log-level <error|warn|info|debug|trace>` on every command (`RUST_LOG` still works when
  it is absent), `janus --version`, and `janus completions <shell>` for bash, zsh, fish, PowerShell
  and elvish. Logs are coloured only on a terminal, and never when `NO_COLOR` is set.
- **`janus-engine --version` and `--help`** now answer instead of waiting on stdin. SDKs start it
  with no arguments, as before. A frame whose `Content-Length` claims more bytes than the host sends
  is now an end-of-file error, not an allocation of the claimed size.

### Features

#### The engine protocol specifies how to verify from, and publish to, a Pact Broker

ADR 0032 decides that the broker client is the engine's, so no host speaks the broker's API. The
engine-protocol spec now specifies it, and the generated bindings carry the new types
(`BrokerDescriptor`, `BrokerAuth`, `ConsumerVersionSelector`, `PublishResults`, `VerifyOptions`,
`PublishContracts`):

- `verification/verify` takes a source of kind `broker` (the broker, the provider, consumer version
  selectors, the provider's branch and tags, pending and work-in-progress pacts), fetched inside the
  run, and an option `publish` that sends each pact's result to that pact's own link. A pending
  pact's failures do not fail the run.
- `broker/publish-contracts` publishes a consumer's contracts, reporting on a stream.
- The `broker` capability says whether an engine has a client. New event kinds:
  `verification/fetched`, `verification/published`, `broker/published`, `broker/finished`. New error
  code: `pacts-not-found`.

No engine has a broker client yet (staged plan S1.3 builds it), so every engine answers a `broker`
source, a `publish` option and `broker/publish-contracts` with `component-unavailable` naming
`broker`. A `broker` source used to be answered with `operation-unsupported`.

#### `janus explain` and `janus verify --explain-failures` colour the plan on a terminal

The plan's structure is in bold, its paths and values are coloured, and every `BOOL(false)` and
`ERROR(...)` is red, so the branch that failed is easy to find. Colour only wraps the text: with it
removed, the output is the plan-grammar spec's text form exactly (spec §3.3, new).

- **Where colour appears.** On a terminal, unless `NO_COLOR` is set or `TERM` is `dumb`. Anywhere
  when `CLICOLOR_FORCE` is set, which is how a CI log that shows colour asks for it. `NO_COLOR`
  wins over both. A pipe or a file gets plain text, as before.
- **What does not change.** The engine still renders the plan, and nothing in the protocol is
  styled: the `verification/explain` result and the `verification/executed-plan` event carry the
  same text as before. `explain --plan` prints JSON, never coloured.
- **For Rust hosts.** The kernel reads a rendered plan back as tokens (`plan::tokens`), so a host
  can colour it without parsing the text form itself. Every text form the corpora record reads
  back with nothing lost.

#### A run's summary counts its variants per source document

The summary `verification/finished` carries gains `pairs` (ADR 0033). It is positionally parallel
to `consumers` and `providers`, with one entry per source document: its `consumer` and
`provider`, its `status` (`verified`, `failed`, or `incomplete` when the run stopped before
finishing it), and its own `variants` counts. The run-level `variants` is their sum. A pending
document that failed is `failed` in its entry; whether that fails the run is still the run's
`status`.

`subsumption/decide`, and so `janus check`, now reports a pair's own variant counts for a run over
several contracts. When a run verified two documents between the same consumer and provider, for
example a consumer's `main` and deployed pacts fetched from a broker, their counts are added up.
Before, the counts were left out for any run over more than one pair. A summary from before `pairs`
is read as it was.

#### `janus publish` and `broker/publish-contracts` publish a consumer's contracts to a Pact Broker

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

#### A release carries the gates an SDK's CI runs, and the specification they run against

Two kinds of file join every release:

- `janus-sdk-gates-<triple>` (`.exe` on Windows) for the same five targets as `janus-engine`: one
  statically linked binary holding the conformance check (`janus-sdk-gates conformance check
  <report.json>`), the thinness audit (`janus-sdk-gates thinness check`, which reads the
  `thinness.json` at the SDK's root) and the binding generator (`janus-sdk-gates bindings generate
  --only <language> --sdk <dir>`).
- `janus-spec.tar.gz`: the conformance suite, the corpora, the behavioural specification, and the
  schemas the bindings are generated from, at the paths they have in the repository. Point the gates
  at it with `--spec <dir>`, and an SDK's conformance driver with `JANUS_SPEC`.

An SDK that pins this version can run its conformance, thinness and bindings gates from the release
alone, with no checkout of this repository and no Rust toolchain. Both are listed in `SHA256SUMS`
and attested like every other file.

#### `janus verify` and `janus-engine` verify from a Pact Broker, and publish results back

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

### Fixes

#### A verification run that crashes still ends its event stream

If a bug stopped a verification run part way, its stream never sent a terminal event, so a host
polling `events/poll` until `last: true` waited forever. The run now ends with
`verification/finished`, carrying `status: failed` and an `error` member: an `EngineError` with code
`internal` and the panic's message. The tallies died with the run, so `variants` and `failures` may
be absent from that summary (engine-protocol spec §9.6).

A host can rely on this: poll until the terminal event, with no timeout of its own. An empty poll
means nothing happened within `wait-ms`, never that the run is gone.

#### A regex in a v1–v4 pact matches the whole value, as it does in today's verifiers

Janus matched a pact's regex anywhere in the value, so `\d{4}-\d{2}-\d{2}` accepted
`2026-07-30 or later`, and `PENDING|SHIPPED` accepted `NOT SHIPPED`. Today's verifiers match it
against the whole value (pact-reference and pact-jvm both do), and now Janus does too:

- **Verifying a v1–v4 pact**, the pattern is compiled anchored, `^(?:…)$`, and `explain` and the
  executed plan print it that way. A provider that passed only because a pattern matched part of a
  value now fails, as it does under today's verifier.
- **`janus upgrade`** writes the anchored pattern into the contract, so a converted pact keeps its
  verdicts. A contract upgraded by an earlier release still has the unanchored pattern, which a
  shape matches anywhere; upgrade it again to get the pact's behaviour.

A regex in a shape (`"shape": "regex"`) is unchanged: it matches anywhere, and an author anchors
it with `^` and `$` (shape-language spec §4.2).

#### The verifier fixture corpus gains broker fixtures

`corpora/verifier/` in `janus-spec.tar.gz` now holds fixtures verified from a Pact Broker. Their
pacts are published to a real broker first, and both verifiers fetch them from it and publish their
results back. They cover the sample provider's own pact, pending and not-pending pacts, a
work-in-progress pact, a pact the consumer version selectors do not fetch, and the
pact-compatibility-suite's four broker scenarios, which were skipped until now. A fixture's `broker`
member says how its pacts are published, what was verified before the run, and the options the
verifiers get. `disagreements.json` gains keys `<pact>/published` for what a run published, and a
verdict `absent`.

Running them takes the Pact Broker pinned in `tools/verifier-diff/pact-broker.json`, which is not
in the archive: anything reading the corpus from the archive alone should skip a fixture with a
`broker` member. Nothing an SDK runs today reads the corpus.

What the ledger says about this release: `janus verify` cannot fetch from a broker or publish
results yet (S1.3). Every broker fixture is a disagreement until it can.

#### `janus-spec.tar.gz` carries the verifier fixture corpus

The specification archive gains `corpora/verifier/`: pacts with the providers that answer them, and
`disagreements.json`, every verdict on which `janus verify` and today's `pact_verifier_cli` (1.3.5)
are known to disagree, each with the plan item that accounts for it. The fixtures include the
pact-compatibility-suite's HTTP provider scenarios, V1 to V4 (Apache-2.0). Nothing an SDK runs today
reads it. A provider integration (staged plan S1.15) will be held to it.

What the ledger says about this release, for anyone deciding whether `janus verify` can replace
today's verifier yet: plain-text, XML, form, multipart and binary bodies are not verified (S1.6); a
pending interaction fails the run (S1.3); a repeated response header is judged as one list (S3.3);
and a failed provider-state call is not retried, where today's verifier retries it eight times
(S1.9).

## 0.1.0 (2026-10-12)

### Features

#### The first release

`janus-engine` and the `janus` CLI as statically linked binaries for five targets, and the engine as a WASM component for offline operations (`janus-engine.wasm`), with a `SHA256SUMS` file and build provenance attestations.

The engine speaks protocol version 1 and reads plan grammars `v1` and `v0`.
