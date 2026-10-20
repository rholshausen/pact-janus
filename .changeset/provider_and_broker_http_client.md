---
default: major
---

# Providers and brokers are reached through one HTTP client, with TLS, proxy, timeout and HTTP/2 options

Every request to a provider or a Pact Broker now goes through one client, configured by one
document, the HTTP client options (component-interfaces spec §5.6, ADR 0035). It is today's
verifier's client (reqwest on rustls, the platform's trust store, proxies from the environment),
async and thread-free.

- **Breaking: a provider request times out after 5 s by default**, as today's verifier's does. It
  was 30 s, set by the kernel on every `transport/send`. Verification's sends now carry no
  `timeout-ms`; the HTTP transport's `client.timeouts.request-ms` bounds each request, and
  `janus verify --request-timeout` sets it.
- **Breaking: a variant that fails because the transport could not send** now carries the
  protocol's `component-failed` error (spec §10.2): `message` says why, and the component and its
  own error are under `details`. They were top-level `component` and `error` members, with no
  message.
- **The HTTP transport's drive options** are now specified: `base-url` (its path prefixes every
  request), `headers` (added to every request, replacing the interaction's own of that name in any
  case) and `client`. Options the transport cannot act on fail `verification/verify` with
  `component-failed` carrying `invalid-config` and the member's pointer.
- **The HTTP client options:** `tls.ca-certificates`, `tls.client-certificate` and `tls.insecure`;
  `proxy` (`environment`, the default, `none`, or `url` with `no-proxy`); `timeouts.request-ms` and
  `timeouts.connect-ms`; `http-version` (`1.1`, the default, `2` or `negotiate`); and `retries`, for
  a client that retries. Certificates and keys are PEM text, never paths.
- **The broker descriptor's `client` is read**, with the same document. With none, a broker is
  reached through the platform's trust store and the environment's proxies. It used to be reached
  through the HTTP library's bundled roots. `retries` replaces the fixed three attempts.
- **`janus verify` options:** today's `--base-path`, `--header`/`-H`, `--request-timeout` and
  `--disable-ssl-verification`, with today's environment variables; and new `--ca-cert`,
  `--client-cert`, `--client-key`, `--proxy`, `--no-proxy`, `--connect-timeout` and
  `--http-version`. `Documentation/verifier-migration.md` maps each of today's options.
- **What a provider sees:** no `Accept-Encoding: gzip` (as today's verifier), a `User-Agent` of
  `pact-janus/<version>` unless the interaction records one, and connections kept open between
  requests.
- **Never logged:** a client certificate's key, a `headers` value, and anything the HTTP stack
  writes at `trace`.
