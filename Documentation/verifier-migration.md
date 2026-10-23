# Moving a provider from today's verifier to `janus verify`

Every option of today's verifiers, `pact_verifier_cli` (pact-reference) and `pact-provider-verifier`
(the Ruby verifier the older wrappers drive), and what it is under `janus verify`: an option, a hook,
or "dropped, because". This is [staged plan S1.13](staged-implementation-plan.md)'s table and the
migration note for providers. Behaviour with no option gets a row too, where a provider relies on it.

It is filled in by area as each area is built. The provider connection is complete
([S1.5](staged-implementation-plan.md), [ADR 0035](decisions/0035-reach-providers-and-brokers-through-one-async-http-client-configured-by-one-document.md)),
and so are provider states and request filters, which become hooks (S1.9). S1.13 adds the rest:
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
| `--transports <kind:port>...` | none yet | A transport per interaction kind, for plugin-backed interactions: the pact-plugins bridge is S5.2, and until then S1.8 refuses such an interaction by name |
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
