# order-service — the sample provider

The provider [plan task 5.6](../../Documentation/project-plan.md) calls for: "a small order-service
provider with deliberate variance and auth, used for M3, M5 and demos". It is a real HTTP service on
a real socket, not a stub — a verification run drives it exactly as it would drive anything else.

```bash
cargo run -p pact_janus_sample_order_service            # prints its base URL on stdout
cargo run -p pact_janus_sample_order_service -- --port 8080 --token my-token
cargo run -p pact_janus_sample_order_service -- --no-auth
```

Tests embed it instead of spawning it:

```rust
let provider = pact_janus_sample_order_service::start(Config::default())?;
let base_url = provider.base_url();   // stopped when it is dropped
```

## Routes

| Route | Auth | Answers |
|---|---|---|
| `GET /health` | no | `{ "status": "up" }` |
| `GET /orders/{id}` | yes | the order, `404` if it does not exist |
| `GET /orders` | yes | every order |
| `POST /orders` | yes | `201` with the created order |
| `POST /_pact/provider-states` | no | the v3 provider-state protocol (below) |

Auth is `Authorization: Bearer <token>`, on by default with the token
`janus-demo-token`. It is there so plan task 5.3's `before-request` hook has something real to do:
a verification run that does not present a credential gets `401`s, which is the honest shape of the
problem that hook exists to solve.

## The variance, and why it is deliberate

An order is:

```json
{ "id": "66", "status": "PENDING", "shippedAt": "2026-07-30T09:00:00Z",
  "items": [ { "sku": "sku-0", "quantity": 1 } ], "channel": "web" }
```

- **`shippedAt` is present or absent**, never null. Absence is exactly what an `optional` shape
  describes from the consumer side, and a `null` would be a different value that shape does not
  admit.
- **`items` varies in length**, so a cardinality dimension has something to range over.
- **`status` can be `CANCELLED`** and **`channel` is always present** — and no consumer contract in
  this repo declares either. That is undeclared provider variance on purpose: it is what Phase 7's
  subsumption check exists to find, and it has to exist here before it can be found there (M5).

Which of these a request sees is decided by **provider state**, not by the request — that is what
lets a verification run replay several variants of one interaction and have each one mean something
(variant-semantics spec §5.2, §6).

## Provider states

`POST /_pact/provider-states` speaks the v3 protocol providers already implement, unchanged:

```json
{ "state": "an order exists", "params": { "id": "66", "shipped": true, "items": 2 }, "action": "setup" }
```

That is B5's claim in miniature — an existing provider verifies with no changes, and the
`pact-state-change` hook (lifecycle-hooks spec §8.4, plan task 5.3) is what drives this endpoint.

| State | Params | Effect |
|---|---|---|
| `an order exists` | `id`, `shipped` (bool or the point name `present`/`absent`), `status`, `items` | seeds that order |
| `no orders exist` | — | empties the store |

Three answers, three different meanings (variant-semantics spec §6.7):

| Status | Meaning |
|---|---|
| `200`, empty or `{}` | the provider is in the state |
| `200` with `{"outcome": "unsupported", "error": {...}}` | the provider **cannot reach** that state — the remedy is a contract change, not a code change |
| `500` | the handler is broken, or the state is one this provider has never heard of |

The middle row is real rather than contrived: asking for `status: SHIPPED` with `shipped: false`
describes an order that has shipped without a shipping date, and no amount of setup reaches it. It
is `200` rather than a 4xx deliberately — the verifier reads a non-2xx as a *failed handler*
(lifecycle-hooks spec §8.4), and the difference between "cannot reach it" and "my handler threw" is
the difference between a contract change and a bug fix. That outcome body is the only Janus-shaped
thing this endpoint says; a provider that never says it still verifies unchanged.

Every state request is recorded in the store's `state_log`, so a demo can show that setup ran once
per variant — which a verifier MUST do, even for consecutive variants whose parameters are identical
(variant-semantics spec §6.6).

## Verifying it

[`verifier.janus.yaml`](verifier.janus.yaml) is this provider's own hook configuration
(lifecycle-hooks spec §6.1) and is the page a reviewer reads to know what a verification run will
do to it: where its states come from, how it is authenticated, and what else each request carries.
It uses three of the four implementation kinds — the `oauth2` **component** for the credential, an
**http** hook in `pact-state-change` format for the states, and a few lines of **script** in
[`hooks/correlation-id.js`](hooks/correlation-id.js) for the thing that is genuinely this project's
own.

The loader resolves `${PROVIDER_URL}`, `${CLIENT_ID}` and `${CLIENT_SECRET}` from the environment
and inlines the script, so the engine receives a document with no templates and no file references
in it (ADR 0014). `janus verify` is plan task 5.5; until it lands,
`engine/kernel/tests/hooks.rs` drives exactly this shape of document through the protocol.
