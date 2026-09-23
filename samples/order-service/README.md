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
| `GET /orders.csv` | yes | every order as `text/csv` (a header row, then one row per order) — what the third-party CSV component is verified against (plan task 8.1) |
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
in it (ADR 0014):

```bash
cargo run -p pact_janus_sample_order_service -- --port 8080 &

PROVIDER_URL=http://127.0.0.1:8080 CLIENT_ID=janus-demo CLIENT_SECRET=janus-demo-secret \
  janus verify samples/order-service/pacts \
    --provider-url http://127.0.0.1:8080 \
    --config samples/order-service/verifier.janus.yaml
```

Add `--explain-failures` and every variant that fails prints the executed plan that produced the
verdict, against the response the provider actually sent.

## The consumer that has not upgraded

[`pacts/web-app-order-service.json`](pacts/web-app-order-service.json) is an ordinary **v3 pact**
— the file a consumer's existing test suite publishes today, in the format it already writes.
There is nothing Janus-shaped in it: no `$format`, no shapes, no variants, and no `Authorization`
header, because the consumer's mock never asked for one.

It verifies against this provider through plan task 5.4's path, under the *same*
`verifier.janus.yaml` above and with no change to either side. That is B5's promise in one file: a
provider adopts Janus first, its consumers keep publishing what they already publish, and the only
difference anyone can point at is the `format` each result names. The engine does not upgrade the
pact on the way in — it compiles its matching rules with design 3.5's compiler and replays the
pact's own recorded request, so nothing is lost in a conversion that never happens.

Two things about it are worth reading for what they demonstrate:

- its provider states (`an order exists` with `{id, shipped}`, `no orders exist`) reach
  `POST /_pact/provider-states` as the very same `{state, params, action}` document that endpoint
  received before Janus existed;
- the credential comes from the `before-request` hook, not from the pact. The run fails on `401`
  without it — which is configuration solving an integration problem, rather than a consumer being
  asked to re-record its pact.

`engine/kernel/tests/legacy_verification.rs` runs it, including one run that verifies this pact and
a Janus contract against this provider together; `cli/tests/cli.rs` runs it through `janus verify`,
and then verifies the contract `janus upgrade` turns it into against the same provider — the two
paths agreeing, observed rather than asserted.

> Hand-written in the published v3 format rather than generated by an SDK — this repo has no JVM or
> Node consumer suite to generate it from. The pacts that genuinely came out of other SDKs live in
> `engine/kernel/tests/fixtures/legacy-pacts/` and are verified by the same test file.

## Recording its shape

The same variance the sections above describe is what makes this provider worth *recording* from
(plan task 7.2). A provider shape says what a provider's own tests produced, and the engine records
one through a third session kind (engine-protocol spec §8.5, behind the
`provider-shape-recording` capability):

```text
provider-shape-session/create   { provider: { name: "order-service" } }        -> { session }
provider-shape-session/observe  { session, description, states, parts }        -> { observations }
provider-shape-session/finalise { session }                                    -> { provider-shape }
```

`observe` carries the **decoded response**, wrapped exactly as a contract wraps a recorded value.
The engine makes no HTTP call of its own and needs no opinion about how the provider was reached:
the provider's own test suite calls it however it already does, and hands over what came back.

`engine/kernel/tests/provider_shape_record.rs` does that against this provider, six responses across
the states it supports, and the document that comes out says the three things this provider was
built to say:

| Recorded | Why it is there |
|---|---|
| `status: any-of['CANCELLED', 'PENDING', 'SHIPPED']` | the states drive it, and `CANCELLED` is a status no consumer contract declares |
| `shippedAt: optional(...)` | absent on an order that has not shipped — variance, not an error |
| `channel` | present on every order and declared by nobody, recorded like any other member because the recorder does not know who asked for what |

Feed that document and the consumer's contract to the subsumption checker (task 7.1) and the result
is M5, reached from the recording side: **one decided finding**, `response.body.status`, provider
`'CANCELLED' | 'PENDING' | 'SHIPPED'` against consumer `'PENDING' | 'SHIPPED'`. Nothing about
`channel` — an extra provider member is must-ignore, and reporting it would teach a team to ignore
the report.

What a recorded shape claims is worth keeping straight, because it is easy to over-read: *these are
the shapes my tests produced*. Had this provider's own tests never set up a cancelled order, the
recorded shape would not mention `CANCELLED`, and the check against that consumer would have passed.
That gap is the provider's test coverage, not the recorder's bug — and it is the reason design 2.8
§2.3 ranks the four provenances at all.

## Checking it — M5 in one command

The recording above is checked in at
[`shapes/order-service.provider-shape.json`](shapes/order-service.provider-shape.json), so the loop
closes over documents alone, with nothing running (plan task 7.4):

```sh
janus check pacts/web-app-order-service.json --provider-shape shapes/
```

```text
✗ web-app is not compatible with order-service
  interaction 'a request for an order', response body $.items:
    provider may produce an empty list, or up to 3 items
    consumer has only tested at least one item
  interaction 'a request for an order', response body $.shippedAt:
    provider may produce exactly '2026-07-30T09:00:00Z', or absent
    consumer has only tested strings matching '\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z', always present
  ? interaction 'a request for an order', response body $.shippedAt:
    provider exactly '2026-07-30T09:00:00Z' cannot be compared against consumer strings matching '\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z' — review manually
  interaction 'a request for an order', response body $.status:
    provider may produce: 'CANCELLED' | 'PENDING' | 'SHIPPED'
    consumer has only tested exactly 'SHIPPED'
  verification: no result supplied
  ! no verification result was supplied for this pair; a contract nobody replayed is not a passing one
  - 1 of 2 interaction(s) have no published provider shape and were not checked
  ! 3 decided finding(s): the provider may produce responses this consumer has not tested
  ! 1 comparison(s) the checker cannot decide either way; a person has to look
  => WARN: web-app -> order-service

WARN: 0 pair(s) blocked, 1 warned, 0 passed — policy on-finding warn, on-review warn, exemptions as of 2026-09-22
```

`$.status` is the RFC's own scenario, reached from a pact nobody migrated: the provider can produce
`CANCELLED` and this consumer has never seen it. It **warns** rather than blocks, because both
severities default to `warn` (ADR 0016) — a check nobody can adopt catches nothing.

The whole loop, as CI would run it:

```sh
janus verify pacts/web-app-order-service.json --provider-url $URL --config verifier.janus.yaml \
             --json > verified.json
janus check  pacts/web-app-order-service.json --provider-shape shapes/ \
             --verification verified.json --on-finding block
```

Now the page carries both halves — `verification: verified (2 of 2 variant(s))` beside the findings
— and the exit code is 1: the verification passed and the deploy is still a no, which is the entire
point of a second source of truth. `cli/tests/cli.rs` runs exactly that sequence.

Four things in that output are the sample teaching what the loop costs:

- **The pact works, and it is noisier than a contract.** `janus check` reads the v3 pact directly
  (converted with design 2.5's rules), so a provider can publish a shape and get an answer about
  consumers who have not migrated. But a conversion freezes each example as `equality`, so `$.status`
  reads "consumer has only tested exactly 'SHIPPED'" where a Janus contract declaring
  `anyOf('PENDING', 'SHIPPED')` would print the RFC's own second line. Three of the four results are
  true; the fourth — the `review` at `$.shippedAt`, the recorder's single observed timestamp against
  the pact's regex — is a comparison the shape language declines to decide and could.
  [Phase-9 finding 9](../../Documentation/phase-9-findings.md#9-a-converted-pacts-frozen-examples-are-the-consumer-side-of-73s-noise-question)
  measures both.
- **`$.items` says "up to 3 items"** because that is what six recorded responses saw. A recorded
  shape claims what the provider's tests produced, never what it can do — the honesty §2.3 of design
  2.8 ranks the provenances for.
- **The second interaction is not checked, and says so.** The provider recorded nothing for "a
  request for an order that does not exist", and a report full of silent passes is what design 2.8
  §6.3 exists to prevent.
- **The `channel` member never appears.** The provider records it, no consumer asked for it, and an
  extra provider member is must-ignore (shape spec §4.3). A checker that reported it would teach a
  team to ignore the report.

## The CSV export, and a component the engine does not ship (plan task 8.1)

`GET /orders.csv` is the one route that is not JSON, and it exists to be verified by a content
component nobody in the engine wrote: `third-party/janus-csv`, a `text/csv` handler built from the
published component interfaces alone. A consumer that reads the export declares the body's type
(`content("text/csv", …)` in the SDKs — contract spec §5.5) and the component, and its contract
records both. Verifying it takes one more member in the configuration, beside the hooks:

```yaml
version: 1

components:
  - name: csv
    source: { kind: file, reference: ../../third-party/janus-csv/target/wasm32-wasip2/release/janus_csv.wasm }

hooks:
  state-setup:
    - name: fixtures
      run: { kind: http, url: "${PROVIDER_URL}/_pact/provider-states", format: pact-state-change }
```

```sh
(cd ../../third-party/janus-csv && cargo build --release --target wasm32-wasip2)
janus verify reporting-order-service.janus.json --provider-url $URL --config verifier-csv.janus.yaml
```

The path is relative to the configuration file, and the loader makes it absolute before the engine
sees it. Leave the `components` member out and the run does not start: `component-unavailable`,
naming `content/csv`, exit 2 — the contract said what it needs, and the engine checked before the
first exchange rather than at the first body. `cli/tests/cli.rs` runs both.

Three things the export teaches about CSV, which the component says rather than hides:

- **Every field is a string.** CSV has no other type, so a consumer's `items` column is
  `regex("^[0-9]+$", "2")`, not `integer(2)`: the component declares `string-only`, and an
  `integer` shape would reject every value it decodes.
- **An unshipped order's `shippedAt` is an empty field**, which is all CSV has for "absent". The
  component reports `no-null` at that column when it decodes one.
- **`channel` is a column no consumer declares**, as the JSON form's member is: an extra column is
  must-ignore, exactly like an extra member.
