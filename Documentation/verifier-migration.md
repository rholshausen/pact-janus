# Moving a provider from today's verifier to `janus verify`

Every option of today's verifiers, `pact_verifier_cli` (pact-reference, at the version
[S1.1's harness pins](../tools/verifier-diff/pact-verifier.json), 1.3.5) and `pact-provider-verifier`
(the Ruby verifier the older wrappers drive), and what it is under `janus verify`: an option, a hook,
or "dropped, because". This is [staged plan S1.13](staged-implementation-plan.md)'s table and the
migration note for providers. Behaviour with no option gets a row too, where a provider relies on it.

`janus verify` takes today's long spellings and environment variables wherever they mean what they
did, so most CI scripts move by changing the command's name
([ADR 0053](decisions/0053-take-todays-verifier-options-where-they-mean-what-they-did.md)). Three
short options cannot: `-h` is help, `-u` is `--provider-url`, and the Ruby verifier's `-c`, `-n`, `-p`
and `-k` are `pact_verifier_cli`'s, which `janus verify` follows. Each row says where it differs.
A row marked **dropped** or **not taken** gives its reason; S1.13 is done when a pilot provider has
accepted each one ([Dropped and not taken](#dropped-and-not-taken) lists them).

Where a row says the two differ, [`corpora/verifier/`](../corpora/verifier/README.md) has a fixture
that shows it, and the difference is in its [ledger](../corpora/verifier/disagreements.json) with the
reason.

## Provider connection

How `janus verify` reaches the provider. These options configure the provider connection only, as
today's do; a Pact Broker is reached with the defaults below, as today's verifier reaches it.

| Today | `janus verify` | Notes |
|---|---|---|
| `--hostname`/`-h` (`PACT_PROVIDER_HOSTNAME`), default `localhost` | `--hostname`, the same variable and default | No `-h`: it is help. `--provider-url`, when given, wins over all three |
| `--port`/`-p` (`PACT_PROVIDER_PORT`), default the scheme's | `--port`/`-p`, the same variable and default | |
| `--transport` (`PACT_PROVIDER_TRANSPORT`), default `http` | `--transport`, the same variable and default | `https` is the `http` transport over TLS, as today. Any other value names a transport component: `grpc` has none until the plugin bridge (S5.2) |
| One of the three, at least | `--provider-url <scheme>://<host>:<port>`, alias `--provider-base-url` | One URL in place of three options, and the Ruby verifier's spelling |
| `--transports <NAME:PORT[/PATH]>...` | `--transports <NAME:PORT[/PATH]>`, repeatable or several in one quoted value | Plain HTTP on the provider URL's host, as today. An `http` entry is where HTTP interactions go; a message is asked for where [Message pacts](#message-pacts) says. A plugin transport (`grpc`) has nothing to route until the bridge (S5.2): S1.8 refuses such an interaction by name. Fixtures `messages/*`, and `cli/tests/cli.rs` |
| `--base-path` (`PACT_PROVIDER_BASE_PATH`) | `--base-path`, the same variable; or a path in `--provider-url` | Fixture `connection/header-and-base-path` |
| `--request-timeout` (`PACT_PROVIDER_REQUEST_TIMEOUT`), default 5000 ms | `--request-timeout`, the same variable and default | Today's also bounds a state-change call. Janus's state-change call is an `http` hook with its own `timeout-ms`, default 5000 ms. Fixtures `connection/request-timeout` and `connection/request-timeout-raised` |
| `--header`/`-H KEY=VALUE`, repeatable | `--header`/`-H KEY=VALUE`, repeatable | Added to every request, replacing the pact's header of that name. **Differs:** Janus replaces a header whose name differs only in case; today's verifier keeps the pact's and sends its value (fixture `connection/header-case`, ADR 0035) |
| `--disable-ssl-verification` | `--disable-ssl-verification` | Fixtures `connection/tls-insecure` and `connection/tls-untrusted` |
| `pact-provider-verifier --provider-base-url` | `--provider-url` | |
| `pact-provider-verifier --custom-provider-header "Name: value"`, repeatable | `--header Name=value` | |

### Behaviour with no option

| Today | `janus verify` |
|---|---|
| Proxies from `HTTPS_PROXY`, `HTTP_PROXY`, `ALL_PROXY` and `NO_PROXY` | The same, for the provider and the broker; `--proxy` and `--no-proxy` override them for the provider |
| The platform's trust store (on Linux, `SSL_CERT_FILE` and `SSL_CERT_DIR` too) | The same, through the same verifier; `--ca-cert` adds to it |
| HTTP/1.1 only | The same; `--http-version 2` or `negotiate` to speak HTTP/2 |
| Redirects followed, up to ten | The same |
| No `Accept-Encoding` | The same (the prototype's client asked for gzip; S1.5 stopped it) |
| A provider request is never retried | The same |
| `User-Agent: pact_verifier/<version>`, unless the pact records one | `User-Agent: pact-janus/<version>`, unless the pact records one |

### New in `janus verify`

None of these has an equivalent today. Each maps onto a member of the HTTP client options
(component-interfaces spec [§5.6](specs/component-interfaces/spec.md#56-the-built-in-http-transports-options)),
which an SDK passes directly.

| Option | Client options member |
|---|---|
| `--ca-cert <FILE>`, repeatable: CA certificates to trust beside the system's | `tls.ca-certificates` |
| `--client-cert <FILE>` and `--client-key <FILE>`: a certificate for a provider that demands one | `tls.client-certificate` |
| `--proxy <URL>`, `--no-proxy <HOSTS>` | `proxy` |
| `--connect-timeout <MS>` | `timeouts.connect-ms` |
| `--http-version 1.1\|2\|negotiate` | `http-version` |

## Where the pacts come from

| Today | `janus verify` | Notes |
|---|---|---|
| `--file`/`-f <FILE>`, repeatable | `--file`/`-f`, or the file as an argument | A Janus contract or a v1–v4 pact |
| `--dir`/`-d <DIR>`, repeatable | `--dir`/`-d`, or the directory as an argument | |
| `--url`/`-u <URL>`, repeatable, fetched with `--user`/`--password` or `--token` | `--url`, the same credentials | No `-u`: it is `--provider-url`. A pact a Pact Broker served carries its links, and `--publish` publishes its result to them; one from anywhere else is verified and not published, as today. `cli/tests/cli.rs` |
| `--webhook-callback-url <URL>` (`PACT_WEBHOOK_CALLBACK_URL`), with `--broker-url` | `--webhook-callback-url`, the same variable | The pact a `contract_requiring_verification_published` webhook names, fetched by its URL and published to its links. `--broker-url` is accepted beside it, as today needs. Fixture `broker/webhook-callback-url` |
| `--broker-url`/`-b` (`PACT_BROKER_BASE_URL`) | `--broker-url`/`-b`, alias `--pact-broker-base-url`, the same variable | Below |
| `--ignore-no-pacts-error` | `--ignore-no-pacts-error` | A broker selection that finds nothing passes, verifying nothing |
| `pact-provider-verifier PACT_URL...` and `--pact-urls` | The files as arguments, or `--url` | `--pact-urls` is deprecated in the Ruby verifier too, and is not taken |
| `pact-provider-verifier --fail-if-no-pacts-found` | Accepted | It is the default |

### Behaviour with no option

| Today | `janus verify` |
|---|---|
| Files, directories, URLs and a broker's selection combine in one run | **Differs:** one kind per run, refused otherwise. A broker URL only `PACT_BROKER_BASE_URL` gave, which a CI job sets for every pact command, gives way to files or `--url` named on the command line |
| A pact fetched by `--url` from a broker is publishable; any other is not | The same |
| A pact by URL is never pending | The same |

## The Pact Broker and publishing

Fetching from a broker, pending and work-in-progress pacts, and publishing ([S1.3](staged-implementation-plan.md),
[ADR 0032](decisions/0032-speak-to-the-broker-through-protocol-operations-backed-by-a-native-host-capability.md)).
The fixtures under `corpora/verifier/broker/` run both verifiers against the pinned broker and compare
what each published.

| Today | `janus verify` | Notes |
|---|---|---|
| `--provider-name`/`-n` (`PACT_PROVIDER_NAME`), default `provider` | `--provider-name`/`-n`, alias `--provider`, the same variable | **Differs:** no default; `--broker-url` needs it |
| `--consumer-version-selectors <JSON>`, repeatable | `--consumer-version-selectors`, alias `--consumer-version-selector` | Passed to the broker as written |
| `--consumer-version-tags <TAGS>`, comma-separated | `--consumer-version-tags`, alias `--consumer-version-tag` | The latest pact for each tag, as today. Not with selectors |
| `--enable-pending` | `--enable-pending` | Only with selectors or tags, as today. Fixtures `broker/pending`, `broker/not-pending`, `broker/pending-without-selectors` |
| `--include-wip-pacts-since <DATE>` | `--include-wip-pacts-since` | Fixture `broker/wip` |
| `--user`, `--password` (`PACT_BROKER_USERNAME`, `PACT_BROKER_PASSWORD`) | `--user`, `--password`, aliases `--broker-user`, `--broker-username`, `--broker-password`, the same variables | Never logged; `cli/tests/cli.rs` checks it at `trace` |
| `--token`/`-t` (`PACT_BROKER_TOKEN`) | `--token`/`-t`, alias `--broker-token`, the same variable | **Differs:** given both, today's verifier uses the username and `janus verify` the token, and the Ruby verifier refuses |
| `--retries <N>` (`PACT_BROKER_HTTP_RETRIES`), default 8, doubling from 500 ms | `--retries`, the same variable | **Differs:** the default is 2, waiting 500 ms more each time, so a run that cannot reach its broker says so in seconds, not minutes |
| `--publish` | `--publish`, alias `--publish-verification-results` | With `--broker-url`, `--url` or `--webhook-callback-url`. A pact the run's filter narrowed is not published (ADR 0041) |
| `--provider-version <V>` | `--provider-version`, alias `--provider-app-version` | Needed by `--publish` |
| `--provider-branch <B>` | `--provider-branch`, alias `--provider-version-branch` | |
| `--provider-tags <TAGS>`, comma-separated | `--provider-tags`, alias `--provider-version-tag` | |
| `--build-url <URL>` | `--build-url` | |
| `pact-provider-verifier --publish-verification-results` from `PACT_BROKER_PUBLISH_VERIFICATION_RESULTS=true` | `--publish` | **Not taken:** the variable. A script names the option |
| `pact-provider-verifier --tag-with-git-branch`/`-g` | `--provider-tags "$(git branch --show-current)"` | **Dropped:** it reads git, which `janus publish` does not either |

### Behaviour with no option

| Today | `janus verify` |
|---|---|
| `--broker-url` with no selectors fetches the latest pact from each consumer, from an endpoint that knows no pending status | The pacts-for-verification endpoint with no selectors, whose default is the same pacts; pending is asked for only with selectors, as today (fixture `broker/pending-without-selectors`) |
| A narrowed pact is published (`--filter-*`) | **Differs:** it is not, and says so ([ADR 0041](decisions/0041-refuse-a-filter-that-names-no-recorded-variant-and-report-a-narrowed-run-as-filtered.md)) |

## Filtering interactions

[ADR 0053](decisions/0053-take-todays-verifier-options-where-they-mean-what-they-did.md) takes today's
filters as they are, with [ADR 0041](decisions/0041-refuse-a-filter-that-names-no-recorded-variant-and-report-a-narrowed-run-as-filtered.md)'s
rules for a filtered run: one that matches nothing is refused, a narrowed run reports `FILTERED` and
exits 0, and a narrowed pact is not published. The fixtures under `corpora/verifier/filters/` run
each against today's verifier.

| Today | `janus verify` | Notes |
|---|---|---|
| `--filter-description <REGEX>` (`PACT_DESCRIPTION`) | `--filter-description`, the same variable | Searched for in the description, as today |
| `--filter-state <REGEX>` (`PACT_PROVIDER_STATE`) | `--filter-state`, the same variable | Searched for in each provider state's name |
| `--filter-no-state` (`PACT_PROVIDER_NO_STATE`) | `--filter-no-state`, the same variable | |
| `--filter-consumer`/`-c <NAME>`, repeatable | `--filter-consumer`/`-c` | **Differs:** the other consumers' pacts are counted as skipped and not published, where today's verifier leaves them out of its report |
| A filter that matches nothing verifies nothing and passes | **Differs:** refused before anything runs, exit 2 (`filter-matches-nothing`) | Fixture `filters/matches-nothing`, in the ledger |
| Not in today's CLI | `--variant <ID>`: one recorded variant of a contract | |

## Pact plugins

Today's verifiers load the pact-plugins plugins a pact names (protobuf and gRPC, Avro, CSV and the
others). Janus has no bridge to them yet: that is [S5.2](staged-implementation-plan.md). Until then
`janus verify` refuses each interaction that needs one, by name, and verifies the rest
([ADR 0045](decisions/0045-refuse-an-interaction-that-needs-a-pact-plugin-by-name-and-verify-the-rest.md)).

| Today | `janus verify` | Notes |
|---|---|---|
| `PACT_PLUGIN_DIR`, default `~/.pact/plugins`: where plugins are found and installed | none yet | S5.2 |
| A plugin the pact names is loaded before anything is verified; a missing one is installed from the plugin index, and one that cannot be ends the run with no results | Each interaction that needs a plugin fails as `component-unavailable` naming `pact-plugin/<name>`, before anything runs for it, and the pact is not verified. Interactions that need none are verified | **Differs** until S5.2. An interaction needs a plugin when its `pluginConfiguration` names one, or when it uses a matcher only a plugin provides, which needs every plugin the pact names |

`janus upgrade` drops such an interaction from the contract it writes, with an `interaction-dropped`
finding naming the plugin, and `janus explain` refuses it as `verify` does.

## Message pacts

Today's verifiers verify a v3 message pact, and a v4 pact's asynchronous and synchronous messages, by
asking the provider for each message over HTTP. `janus verify` asks the same way, through a
`produce-message` hook ([S1.7](staged-implementation-plan.md),
[ADR 0046](decisions/0046-ask-a-provider-for-a-message-through-produce-message-and-let-the-transport-name-its-parts.md)).
A provider that verifies messages today needs no configuration: when the verifier configuration names
no `produce-message` hook, `janus verify` adds one, `provider-messages`, which does what today's
verifier does. A provider whose producer is reached some other way names its own:

```yaml
version: 1
hooks:
  produce-message:
    - name: messages
      run: { kind: http, url: "http://localhost:8081/messages", format: pact-message,
             headers: { authorization: "Bearer ${MESSAGE_TOKEN}" } }
      changes: [parts]
```

A configuration's own `produce-message` hooks replace the default whole. A `script` or `exec` hook can
produce the message too, as the `message` part with `metadata` and `body` slots (lifecycle-hooks spec
§3.4).

| Today | `janus verify` | Notes |
|---|---|---|
| A message is asked for by a POST of `{ description, providerStates }`, and a synchronous message's `request` too | The same POST, from `provider-messages` or an `http` hook with `format: pact-message` | Fixtures `messages/*` and `compatibility-suite/v3-messages`, `v4-messages` |
| The POST goes to the `--transports` entry the interaction's transport names, else to one named `message` or `async-message` (`sync-message`), else to the provider's URL | The same, from `--transports` | `cli/tests/cli.rs` |
| The reply's body is the message; its `Content-Type` and the base64 JSON of a `Pact-Message-Metadata` header are its metadata | The same | |
| `--header` and a request filter reach the message POST | `--header` does. A `before-request` hook does not run, since no request is sent: a credential goes in the `produce-message` hook's `headers` | |
| A synchronous message's reply must be a success; an asynchronous message is read from any reply | **Differs:** every reply must be a 2xx. A producer that answered 500 has not produced the message | Fixture `messages/error-status`, in the ledger |
| A synchronous message with more than one response compares the first, and fails | The same | |
| A provider with no message endpoint fails each message on the request | The same, through the hook; with a configuration whose `produce-message` hooks select no interaction, a message fails as `component-unavailable` naming `hook/produce-message` | |

`janus upgrade` still drops a message interaction, with an `interaction-dropped` finding: a contract has
no message form until the consumer side has one (S6.2). `janus explain` shows a message's plan.

## Provider states and request filters

Today's state-change and request-filter options become **hooks** in a verifier configuration,
`verifier.janus.yaml`, which `janus verify --config` reads ([lifecycle-hooks spec](specs/lifecycle-hooks/spec.md),
[S1.9](staged-implementation-plan.md)). A provider's existing state endpoint keeps working unchanged:
an `http` hook with `format: pact-state-change` sends it today's `{ state, params, action }` body.
`janus verify` takes today's three state-change options too, and writes the hooks below itself
(ADR 0053), so a script that uses them needs no configuration. A configuration with `state-setup`
hooks of its own and `--state-change-url` together is refused: the states would be set up twice.
[`samples/order-service/verifier.janus.yaml`](../samples/order-service/verifier.janus.yaml) is
a worked example.

```yaml
version: 1
hooks:
  state-setup:
    - name: states
      run: { kind: http, url: "${PROVIDER_URL}/_pact/provider-states", format: pact-state-change }
  state-teardown:            # today's --state-change-teardown
    - name: states
      run: { kind: http, url: "${PROVIDER_URL}/_pact/provider-states", format: pact-state-change }
```

| Today | `janus verify` | Notes |
|---|---|---|
| `--state-change-url`/`-s` (`PACT_PROVIDER_STATE_CHANGE_URL`) | `--state-change-url`/`-s`, the same variable; or a `state-setup` hook: `run: { kind: http, url, format: pact-state-change }` | The option writes that hook, bounded by `--request-timeout` and reached with the provider's client options. Each state is a POST of `{ state, params, action: "setup" }`. A JSON object in the reply is the provider-state values a `ProviderState` generator reads, as today (ADR 0034). The hook's `headers` carry a credential the endpoint needs, and its `client` the TLS and proxy options it is reached with, the same options a provider takes (ADR 0044) |
| `--state-change-as-query` (`PACT_PROVIDER_STATE_CHANGE_AS_QUERY`) | `--state-change-as-query`, the same variable; or `format: pact-state-change-query` | `state`, `action` and each parameter as query parameters, and no body, as today |
| `--state-change-teardown` (`PACT_PROVIDER_STATE_CHANGE_TEARDOWN`) | `--state-change-teardown`, the same variable; or the same hook again under `state-teardown` | Sends `action: "teardown"` after the interaction, as today |
| A provider state with no state-change URL | A `state-unhandled` warning, and the exchange runs without setup; `--unhandled-states fail` fails it instead | New: today's verifier runs it and says nothing. A state no `state-setup` hook's `when` admits is warned of the same way ([ADR 0058](decisions/0058-a-provider-state-no-hook-sets-up-is-a-warning-or-a-failure-by-choice.md)) |
| `pact-provider-verifier --provider-states-setup-url`/`-c` | `--provider-states-setup-url`, an alias of `--state-change-url` | The Ruby verifier's body is the same `{ state, params, action }`. No `-c`: it is `--filter-consumer`, as in `pact_verifier_cli` |
| `pact-provider-verifier --custom-middleware <FILE>` | A `before-request` hook that may change `parts.request` | Rack middleware cannot run outside Ruby. What it did to a request, a `script` hook does in JavaScript, or an `exec` or `http` hook in any language, and the entry's `changes` names what it may change |
| A request filter in a test-framework integration (pact-js `requestFilter`, pact-jvm's `@TargetRequestFilter` and the build plugins' `requestFilter`) | A `before-request` hook | Not an option of either CLI. The in-test integrations that offer one now are S1.15's; each serves it as a loopback `http` hook (lifecycle-hooks spec §8.4) |
| State handlers in a test-framework integration (pact-js `stateHandlers`, pact-jvm's `@State` methods) | A `state-setup` and a `state-teardown` hook | S1.15, as above |

### Behaviour with no option

| Today | `janus verify` |
|---|---|
| A state call that gets a 5xx, 408 or 429 is tried again, up to eight times in all, waiting from 1 s and doubling (about two minutes before an endpoint that keeps failing fails) | Tried three times in all, waiting 100 ms and then 1 s. `client.retries` on the hook changes the count. **Differs** deliberately: today's count came with the broker's retries, not from a decision about state calls (ADR 0044). Fixture `state-callbacks/fails-once` |
| A state call is bounded by `--request-timeout` | Each attempt is bounded by the hook's `timeout-ms`, default 5000 ms |
| An interaction with a provider state and no state-change URL is verified, with a warning that the state was ignored | Verified; no `state-setup` hook runs |

## Logging, reports and output

`janus verify` prints its progress and a summary on standard output and logs on standard error.
`--output json` prints the run summary document instead (engine-protocol spec §9.6), the document
`janus check --verification` reads.

| Today | `janus verify` | Notes |
|---|---|---|
| `--loglevel`/`-l <LEVEL>` (`error`, `warn`, `info`, `debug`, `trace`, `none`), default `warn` | `--log-level`/`-l`, alias `--loglevel`; `none` is `off` | Without it, `RUST_LOG` is read |
| `--pretty-log`, `--full-log`, `--compact-log` | One format | **Dropped:** a log's layout is not something a script reads; `RUST_LOG` chooses what is logged |
| `--json`/`-j <FILE>` (`PACT_VERIFIER_JSON_REPORT`) | `--output json > <FILE>` | **Not taken:** today's report format. The summary is Janus's own document, which names each pair and failure; a tool that reads today's report reads the summary instead |
| `--junit`/`-x <FILE>` (`PACT_VERIFIER_JUNIT_REPORT`) | none | **Not taken:** a new output, not parity, and an optional feature of today's verifier. An `after-verification` hook is handed the summary and can write one |
| `--html <FILE>` (`PACT_VERIFIER_HTML_REPORT`), `--xslt <FILE>` | none | **Not taken**, for the same reasons |
| `--no-colour`, `--no-color` | `--no-colour`, `--no-color` | Colour is only ever on a terminal; `NO_COLOR` turns it off too |
| `--help`, `--version`/`-v` | `--help`, `--version`/`-V`, `janus version` | |
| `pact-provider-verifier --verbose`/`-v` (`VERBOSE`) | `--log-level debug` | |
| `pact-provider-verifier --log-level`, default `debug`, and `--log-dir` | `--log-level`, default `warn`, on standard error | **Dropped:** `--log-dir`. Logs go to standard error, which a CI job keeps |
| `pact-provider-verifier --format`/`-f` (`json`, `RspecJunitFormatter`) and `--out`/`-o <FILE>` | `--output json`, redirected | **Not taken:** RSpec's formats, as `--junit` |

### Exit status

| Today | `janus verify` |
|---|---|
| 0 every pact verified | 0, and also for a filtered run that nothing failed (ADR 0041) |
| 1 a verification failed | 1, and also for a run a hook aborted |
| 1 a pact could not be loaded: a file that cannot be read, a broker or URL that cannot be fetched from, or a broker that selects no pacts, recorded as a failed result | **Differs:** 2, the command could not run, before anything is verified; so is a `--variant` no pact records. A pipeline that only tests for non-zero sees no difference (staged plan S1.29) |
| 2 a report could not be written, or the verifier itself failed | 2 |

## Development options

| Today | `janus verify` | Notes |
|---|---|---|
| `--exit-on-first-error` | `--exit-on-first-error` | **Differs:** a pending pact's failure does not stop the run, as it does today, since it does not fail it. What the run did not reach is not run, and the summary says it stopped |
| `--last-failed` (needs `--json`) | `--filter-description` or `--variant` for a rerun | **Not taken:** it reads today's JSON report from the previous run |
| `pact-provider-verifier --wait <SECONDS>` | A CI step that waits for the provider | **Not taken:** waiting for a service is the CI job's, which knows what ready means |
| `pact-provider-verifier --ignore-failures` | `janus verify ... \|\| true` | **Dropped:** a hidden option whose answer is the shell's; exit 1 is the provider failing, 2 the command |
| `pact-provider-verifier --monkeypatch <FILE>` | none | **Dropped:** a hidden option that loads Ruby |
| `pact-provider-verifier --provider-states-url`/`-s` | none | **Dropped:** hidden and deprecated in the Ruby verifier, which no longer calls it |
| `pact-provider-verifier --broker-username`/`-n`, `--broker-password`/`-p`, `--broker-token`/`-k` | `--broker-username`, `--broker-password`, `--broker-token` | No short forms: `-n` and `-p` are `pact_verifier_cli`'s |

## Dropped and not taken

Each needs a pilot provider to accept its reason before S1.13 is done.

| Option | | Reason |
|---|---|---|
| `--pretty-log`, `--full-log`, `--compact-log` | dropped | One log format; `RUST_LOG` chooses what is logged |
| `--json`/`-j`, today's report format | not taken | `--output json` writes the run summary, a different document |
| `--junit`/`-x`, `--html`, `--xslt`, the Ruby verifier's `--format` | not taken | New outputs, not parity; an `after-verification` hook can write one |
| `--last-failed` | not taken | Reads today's JSON report; a filter narrows a rerun |
| The Ruby verifier's `--wait` | not taken | Waiting for the provider is the CI job's |
| The Ruby verifier's `--tag-with-git-branch` | dropped | Reads git; `--provider-tags` takes the branch |
| The Ruby verifier's `--log-dir`, `--ignore-failures`, `--monkeypatch`, `--provider-states-url`, `--pact-urls` | dropped | Logs go to standard error; the shell ignores failures; Ruby code; deprecated |
| `PACT_BROKER_PUBLISH_VERIFICATION_RESULTS` | not taken | A script names `--publish` |
| The short options `-h` (hostname), `-u` (url), and the Ruby verifier's `-c`, `-n`, `-p`, `-k`, `-a`, `-r`, `-g`, `-v`, `-f`, `-o` | not taken | Each is help, another option of `janus verify`, or `pact_verifier_cli`'s; the long spelling works |

