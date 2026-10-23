# 0044 — Reach a hook endpoint as a provider is, and try a state call three times

- **Status**: accepted
- **Date**: 2026-10-23
- **Plan tasks**: S1.9
- **Evidence**: pact-reference's `HttpRequestProviderStateExecutor` and `with_retries`, and their
  history: #197 (`18118e82`, 2022) and `8f86cd92` and `c98502f6` (May 2026); S1.1's
  `state-callbacks/` fixtures and ledger; S1.5's findings (ADR 0035)

## Context

A provider's state-change endpoint is an `http` hook with `format: pact-state-change` (lifecycle-hooks
spec §8.4). S1.1 and S1.5 found three ways it falls short of today's verifier.

- **It is not retried.** Today's verifier tries a state call again when it gets a 5xx, 408 or 429,
  so an endpoint that fails while it warms up passes. The `http` hook tries once, and
  `state-callbacks/fails-once` passes today and fails under Janus.
- **It has its own client.** The hook sent through `ureq`, configured with nothing but a timeout, so
  it could not reach a state endpoint behind the provider's mutual TLS or private CA. A provider
  needs those options to be verified at all (S1.5).
- **`--state-change-as-query` has no hook form.** Today's verifier can send the state as query
  parameters instead of a body.

Today's retry count is not a design. State calls got retries in 2022 (#197): three attempts, on 5xx
only, waiting 100 ms and then 1 s. In May 2026, `8f86cd92` rewrote the shared retry function for
the broker client, adding 408 and 429 and a backoff of 500 ms doubling. `c98502f6` then raised the
state executor's default from 3 to 8, in a commit about broker retries. State calls inherited both.
A state endpoint that keeps failing now takes about two minutes to fail. State endpoints are mostly
reached over the loopback interface, where a failure that lasts that long is not going to clear.

## Decision

1. **An `http` hook reaches its endpoint through `engine/http-client`**, the client a provider and a
   broker are reached through, configured by the entry's `client` member, the HTTP client options
   (ADR 0035). `ureq` leaves the hooks host.
2. **A state call is tried three times.** `client.retries`, how many times a request is tried again,
   defaults to 2 at `state-setup` and `state-teardown` and 0 at every other point. A request is tried
   again after an answer of 5xx, 408 or 429, waiting 100 ms, then 1 s, ten times longer each time
   after. A request that gets no answer is not tried again, as today. This is the 2022 design, with
   today's set of statuses.
3. **`timeout-ms` bounds each attempt**, not the invocation, when a request is tried again. A hook
   that keeps failing therefore ends in at most three times its timeout plus 1.1 s.
4. **`format: pact-state-change-query`** sends the state as query parameters with no body: `state`,
   `action`, and each parameter, a string as it is and anything else as its JSON text, as today's
   `--state-change-as-query` does.
5. **A hook's URL is never repeated with its user information** in a failure.

## Alternatives considered

- **Today's count of eight, with today's backoff.** A provider would keep exactly what it has today.
  But that count came with the broker's retries, not from a decision about state calls, and it makes
  a broken state endpoint cost two minutes per interaction.
- **No retries by default.** Over loopback a failed state call is usually a real failure. But an
  endpoint that is still warming up would fail an interaction that passes today, and a provider would
  have to find a member to fix it.
- **A `retries` member on the hook entry.** The HTTP client options already have one, which the
  broker client reads with the same meaning.

## Consequences

- `state-callbacks/fails-once` passes under Janus as it does today, and its ledger entries go. A state
  endpoint that keeps failing fails under both, in about a second under Janus rather than about two
  minutes. A provider whose endpoint takes longer than 1.1 s to recover sets `client.retries`.
- A state endpoint behind mutual TLS or a private CA is reached with the options the provider is.
- The built-in OAuth2 hook component still has its own client. Moving it is for when a token endpoint
  needs these options.
- **Tripwire**: if pilot providers' state endpoints need longer than three attempts to warm up, the
  default count goes up.
