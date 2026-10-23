# Moving a provider from today's verifier to `janus verify`

Every option of today's verifiers, `pact_verifier_cli` (pact-reference) and `pact-provider-verifier`
(the Ruby verifier the older wrappers drive), and what it is under `janus verify`: an option, a hook,
or "dropped, because". This is [staged plan S1.13](staged-implementation-plan.md)'s table and the
migration note for providers. Behaviour with no option gets a row too, where a provider relies on it.

It is filled in by area as each area is built. The provider connection is complete
([S1.5](staged-implementation-plan.md), [ADR 0035](decisions/0035-reach-providers-and-brokers-through-one-async-http-client-configured-by-one-document.md)),
and so are provider states and request filters, which become hooks (S1.9), pact plugins, which
are refused by name until S5.2 (S1.8), and message pacts (S1.7). S1.13 adds the rest:
pact sources, the Pact Broker, filtering, publishing and reporting.

Where a row says the two differ, [`corpora/verifier/`](../corpora/verifier/README.md) has a fixture
that shows it, and the difference is in its [ledger](../corpora/verifier/disagreements.json) with the
reason.

## Provider connection

How `janus verify` reaches the provider. These options configure the provider connection only, as
today's do; a Pact Broker is reached with the defaults below, as today's verifier reaches it.

| Today | `janus verify` | Notes |
|---|---|---|
| `--hostname`/`-h`, `--port`/`-p`, `--transport`/`--scheme` (`PACT_PROVIDER_HOSTNAME`, `PACT_PROVIDER_PORT`, `PACT_PROVIDER_TRANSPORT`) | `--provider-url <scheme>://<host>:<port>` | One URL in place of three options. Janus's `--transport` names the transport component (`http`, which speaks both `http` and `https`), not the scheme. Whether `janus verify` also takes today's three spellings is S1.13's to decide |
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
Whether `janus verify` also takes today's option spellings and writes the hook itself is S1.13's to
decide. [`samples/order-service/verifier.janus.yaml`](../samples/order-service/verifier.janus.yaml) is
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
| `--state-change-url`/`-s` (`PACT_PROVIDER_STATE_CHANGE_URL`) | A `state-setup` hook: `run: { kind: http, url, format: pact-state-change }` | Each state is a POST of `{ state, params, action: "setup" }`. A JSON object in the reply is the provider-state values a `ProviderState` generator reads, as today (ADR 0034). The hook's `headers` carry a credential the endpoint needs, and its `client` the TLS and proxy options it is reached with, the same options a provider takes (ADR 0044) |
| `--state-change-as-query` (`PACT_PROVIDER_STATE_CHANGE_AS_QUERY`) | `format: pact-state-change-query` | `state`, `action` and each parameter as query parameters, and no body, as today |
| `--state-change-teardown` (`PACT_PROVIDER_STATE_CHANGE_TEARDOWN`) | The same hook again under `state-teardown` | Sends `action: "teardown"` after the interaction, as today |
| `pact-provider-verifier --provider-states-setup-url` | A `state-setup` hook, as `--state-change-url` | The Ruby verifier's body is the same `{ state, params, action }` |
| `pact-provider-verifier --custom-middleware <FILE>` | A `before-request` hook that may change `parts.request` | Rack middleware cannot run outside Ruby. What it did to a request, a `script` hook does in JavaScript, or an `exec` or `http` hook in any language, and the entry's `changes` names what it may change |
| A request filter in a test-framework integration (pact-js `requestFilter`, pact-jvm's `@TargetRequestFilter` and the build plugins' `requestFilter`) | A `before-request` hook | Not an option of either CLI. The in-test integrations that offer one now are S1.15's; each serves it as a loopback `http` hook (lifecycle-hooks spec §8.4) |
| State handlers in a test-framework integration (pact-js `stateHandlers`, pact-jvm's `@State` methods) | A `state-setup` and a `state-teardown` hook | S1.15, as above |

### Behaviour with no option

| Today | `janus verify` |
|---|---|
| A state call that gets a 5xx, 408 or 429 is tried again, up to eight times in all, waiting from 1 s and doubling (about two minutes before an endpoint that keeps failing fails) | Tried three times in all, waiting 100 ms and then 1 s. `client.retries` on the hook changes the count. **Differs** deliberately: today's count came with the broker's retries, not from a decision about state calls (ADR 0044). Fixture `state-callbacks/fails-once` |
| A state call is bounded by `--request-timeout` | Each attempt is bounded by the hook's `timeout-ms`, default 5000 ms |
| An interaction with a provider state and no state-change URL is verified, with a warning that the state was ignored | Verified; no `state-setup` hook runs |
