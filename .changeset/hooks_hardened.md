---
default: minor
---

# Hooks hardened: state calls retried, hook endpoints reached as a provider is, secrets kept out of logs

- **An `http` hook sends through the engine's HTTP client**, the one a provider and a broker are reached
  through, and takes a `client` member: the HTTP client options (TLS, proxies, timeouts, HTTP version). A
  state endpoint behind the provider's mutual TLS or private CA is reached with the same options as the
  provider (ADR 0044).
- **A state call is tried three times.** At `state-setup` and `state-teardown`, an answer of 5xx, 408 or 429
  is tried again after 100 ms and then 1 s. `client.retries` changes the count, and at any other point a
  request is tried once. `timeout-ms` now bounds each attempt. A state endpoint that fails once while it
  warms up passes, as it does under today's verifier.
- **`format: pact-state-change-query`** sends a state as query parameters with no body, as today's
  `--state-change-as-query` does.
- **An `exec` hook's deadline ends everything the command started**: its process group on Unix, its process
  tree on Windows. A command that exits while a child it started holds its output open is ended at the
  deadline too.
- **A scripted hook's memory and stack are bounded**, at 64 MiB and 256 KiB. A script that allocates or
  recurses without end fails as `errored` rather than taking the engine's process with it.
- **No hook secret is logged.** The `trace` log of each frame no longer repeats a hook's `config`, its
  `run.headers` or `run.env`, or the user information in an `http` hook's URL, and a failing `http` hook's
  message no longer repeats that user information.
