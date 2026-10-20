# Moving a provider from today's verifier to `janus verify`

Every option of today's verifiers, `pact_verifier_cli` (pact-reference) and `pact-provider-verifier`
(the Ruby verifier the older wrappers drive), and what it is under `janus verify`: an option, a hook,
or "dropped, because". This is [staged plan S1.13](staged-implementation-plan.md)'s table and the
migration note for providers. Behaviour with no option gets a row too, where a provider relies on it.

It is filled in by area as each area is built. The provider connection is complete
([S1.5](staged-implementation-plan.md), [ADR 0035](decisions/0035-reach-providers-and-brokers-through-one-async-http-client-configured-by-one-document.md)).
S1.13 adds the rest: pact sources, the Pact Broker, provider states and request filters (which
become hooks, S1.9), filtering, publishing and reporting.

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
