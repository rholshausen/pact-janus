# 0035 — Reach providers and brokers through one async HTTP client, configured by one document

- **Status**: accepted
- **Date**: 2026-10-21
- **Plan tasks**: S1.5 (the build); S2.1 (the server half, which shares the executor); S1.13 (the
  flag table this fills the transport rows of); S1.9 (the `http` hook, which this leaves on its own
  client)
- **Evidence**: pact-reference's verifier (`pact_verifier::configure_http_client`, which builds one
  reqwest client from `disable_ssl_verification`, `request_timeout` (default 5000 ms) and
  `custom_headers`; `provider_client::create_native_request`, which drops a pact header a custom
  header names; its `Cargo.toml`, reqwest 0.13 with `rustls-no-provider` on ring and no `http2`);
  `pact_verifier_cli`'s provider options; ADR 0032 decision 6; engine-protocol spec §8.7;
  component-interfaces spec §3.4 and §5; ADR 0023 decision 5 and gate W

## Context

The HTTP component's drive role sends each replayed request with a fresh `ureq` agent. It has
no TLS options, no proxy option and no client certificate, and a fixed timeout that the kernel
sets (30 s). A provider that only answers over mutual TLS, or only through a corporate proxy,
cannot be verified, and today's `--request-timeout`, `--header`, `--base-path` and
`--disable-ssl-verification` have nothing to map onto. The broker client (S1.3) is a second
`ureq` agent with the same gaps. ADR 0032 decision 6 says both take one client configuration,
carried as the broker descriptor's `client`, and leaves that document to this item.

What today's verifier does is the standard here. It uses one reqwest client for the provider:
rustls on ring, the platform's trust store, proxies from the environment, redirects followed,
HTTP/1.1 only (it is built without HTTP/2), and a 5 s request timeout. Its broker client is another
reqwest client with the same defaults and no options. `ureq` bundles its own roots, ignoring the
platform's, and asks for gzip by default. So Janus currently trusts different certificates than
today's verifier and sends a header it does not send.

The plan also asks for this half to be async and thread-free, behind the executor seam S2.1 will
use. That way, running the engine on `wasm32-wasip3` (gate W) costs a build target rather than a
redesign.

## Decision

1. **One native HTTP client crate, `engine/http-client`, used by the HTTP component's drive role
   and the broker client.** It is reqwest on rustls with the ring provider and the platform
   verifier: the stack today's verifier uses, already in our tree through `pact_models`, so no new
   native dependency arrives. Certificate trust, proxy variables and redirects then behave as
   today. HTTP/2 is reqwest's `http2` feature; `h2` is already in the tree. When no rustls
   provider is installed, the crate installs ring as the process default, as today's verifier does.
2. **Async and thread-free, behind an executor seam.** The client's `send` is an `async fn` from
   an `http::Request` to an `http::Response`. Each client owns an `Executor`: on native targets,
   a current-thread tokio runtime, which starts no thread. Names are resolved inside the request's
   own future, because reqwest's default resolver starts a thread per lookup. The synchronous
   transport and broker traits block on the executor, and S2.1's server half runs on the same one.
   Everything above `send` (parts to request, custom headers, retries, the broker's HAL walk) is
   ordinary async code that knows no runtime. On `wasm32-wasip3`, the executor becomes WASI's and
   `send` becomes `wasi:http`'s outgoing handler, where TLS and proxies belong to the host. Those two
   are the seam.
3. **One options document, the HTTP client options** (component-interfaces spec §5.6,
   `http-transport.schema.json`). It is the `client` member of the HTTP transport's drive options
   and of the broker descriptor:
   - `tls`: `ca-certificates`, PEM text added to the platform's roots rather than replacing them;
     `client-certificate`, `{ certificate, key }` as PEM text; `insecure`, which accepts any
     certificate (today's `--disable-ssl-verification`).
   - `proxy`: absent, or `{ "kind": "environment" }`, reads `HTTPS_PROXY`, `HTTP_PROXY`,
     `ALL_PROXY` and `NO_PROXY` where the engine runs, as today's verifier and curl do;
     `{ "kind": "url", "url", "no-proxy"? }` names one; `{ "kind": "none" }` connects directly.
   - `timeouts`: `request-ms`, the whole request (default 5000 for a provider, today's; 60 000 for
     a broker, the S1.3 client's); `connect-ms`, the connection alone (default: bounded by the
     request).
   - `http-version`: `"1.1"` (the default, today's); `"2"` (HTTP/2 only: ALPN over TLS, prior
     knowledge in the clear); `"negotiate"` (ALPN offers both, HTTP/1.1 in the clear).
   - `retries`: how many times a client that retries tries again. The broker client does (default
     2, S1.3's three attempts). A provider request is never retried: a verification observes the
     provider's first answer, as today's does.
   Certificates and keys are **text, not paths**. The host reads the files, as the hooks loader
   resolves a script before the engine sees it (ADR 0014). The engine reads no file for them,
   needs no filesystem to take them, and never repeats a key: the trace log shows `***`.
4. **The HTTP transport's drive options are `base-url`, `headers` and `client`.** A path in
   `base-url` prefixes every request's path, so it is today's base path. `headers` (name to value)
   go on every request and replace a header of the same name that the interaction carries. Today's
   `--header` does the same, except that its "do not overwrite" check is case-sensitive; Janus
   compares names case-insensitively, as HTTP does. Custom headers are applied when the request is
   sent, after `before-request`, as today's are applied after a request filter. A `headers` value
   is a credential as often as not, and the trace log shows `***`.
5. **The provider's request timeout is the drive instance's.** Verification's `send` no longer
   carries the kernel's fixed 30 s. The HTTP transport bounds each request by `request-ms`, and by
   the shorter of the two when a `send` does carry `timeout-ms`. An out-of-tree transport stays
   bounded by the component host's per-call deadline (component-interfaces spec §3.4), so a run
   still ends when a provider hangs.
6. **`janus verify` takes today's spellings and environment variables**: `--request-timeout`
   (`PACT_PROVIDER_REQUEST_TIMEOUT`), `--header`/`-H` as `KEY=VALUE`, `--base-path`
   (`PACT_PROVIDER_BASE_PATH`) and `--disable-ssl-verification`. Beside them are `--ca-cert`,
   `--client-cert`, `--client-key`, `--proxy`, `--no-proxy`, `--connect-timeout` and
   `--http-version`, for what today's verifier cannot do. They configure the provider connection
   only, as today's do. The broker gets the defaults, which are what today's verifier gives it;
   flags for a broker's TLS are S1.13's to spell, with the rest of the broker's options.

## Alternatives considered

- **Keep `ureq` and map the options onto it.** It has TLS roots, client certificates and proxies.
  But it has no HTTP/2 and is blocking by design, so the item's async exit would need a second
  client anyway. Its bundled roots would also differ from today's trust store.
- **hyper and hyper-util directly, with injected runtime traits.** That is the purest executor
  seam, since hyper takes its I/O, timer and executor as parameters. But it means rebuilding proxy
  matching, TLS assembly with the platform verifier, ALPN and redirects that reqwest already gets
  right, and getting them right in today's verifier's way is the point of stage 1. Under WASI the
  client is `wasi:http` anyway, so hyper's portability buys nothing there.
- **Paths in the options document.** Shorter frames, and no key to redact in the trace log. But the
  engine would read files on the host's behalf, which breaks ADR 0032's rule that what the engine
  is given travels in the request. Keys would also stop working for an embedding with no
  filesystem.
- **A process-wide runtime shared by every client.** It is cheaper to build once. But concurrent
  `block_on` calls from a verification thread and a broker publication would contend for one
  driver. A current-thread runtime costs microseconds to build and nothing while idle.
- **HTTP/2 negotiated by default.** It is the modern default. But today's verifier never speaks
  HTTP/2, so a TLS provider that offers it would see different traffic on the day it moved to
  Janus.

## Consequences

- A provider behind mutual TLS, a private CA or a proxy can be verified. Today's transport flags all
  map onto Janus, and the new ones do what today's verifier cannot. The migration table
  (`Documentation/verifier-migration.md`) records each.
- Provider and broker requests now trust the platform's certificate store, honour `SSL_CERT_FILE`
  where the platform verifier does, and stop asking for gzip, as today's verifier does. A provider
  that took longer than 5 s to answer, and passed under the kernel's 30 s, now fails, as it does
  today; `--request-timeout` raises the limit.
- Installing ring as the default rustls provider is a process-wide side effect, which every host of
  the native engine inherits. It is the same one today's verifier makes.
- The `http` hook (`engine/hooks-host`) is still on `ureq`, so a state-change endpoint behind the
  same mutual TLS cannot be called yet. S1.9 moves it onto this client, with a `client` member on
  an `http` hook entry.
- The component-interfaces spec gains §5.6 and `http-transport.schema.json`, and §5.2 says how
  `timeout-ms` combines with a transport's own timeout. The broker descriptor's `client`
  description points at the new schema. All of this is additive within v1.
- **Tripwire**: if a provider needs the client to behave differently from reqwest's defaults in a
  way the options document cannot express (a proxy that needs NTLM, a TLS version floor, certificate
  revocation lists), that need is evidence for building the native client on hyper directly. It is
  not a reason to add options for every reqwest setting.
