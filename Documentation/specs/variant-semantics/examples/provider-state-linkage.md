# Worked example — variant-bound provider state

The RFC gives the provider-state linkage one line and marks it a sketch:

```typescript
given('an order exists', { shipped: whenVariant('shippedAt', 'present') })
```

This file takes that line through to a verifier calling `state-setup`, on the same order payload the
[sampling example](order-payload-sampling.md) selects eight variants for. The interesting part is not
the happy path — it is variant 5, which asks the provider for a state it cannot produce.

Fenced blocks below carry a marker naming what they are: `params` blocks are validated against
[`variant-params.schema.json`](../schemas/v1/variant-params.schema.json) and `policy` blocks against
[`sampling-policy.schema.json`](../schemas/v1/sampling-policy.schema.json), by
`cargo test -p pact_janus_schema_compat`. Blocks marked `sketch` show documents owned by designs 2.5
and 2.7.

## 1. What the author writes

```typescript
const getOrder = pact.interaction('get an order')
  .given('an order exists', {
    id: '42',
    shipped: whenVariant('shippedAt', 'present'),
    status: whenVariant('status', { SHIPPED: 'SHIPPED', DELIVERED: 'DELIVERED' }, 'PENDING'),
  })
  .request({ method: 'GET', path: '/orders/42' })
  .response({ status: 200, body: json({ /* the order shape */ }) });
```

Two forms, one of them the RFC's. `whenVariant(dimension, point)` is the boolean predicate; the
three-argument form maps points to values with a fallback. The predicate is sugar — the SDK expands it
to the same document the general form produces, and neither form makes the SDK compute anything
(spec §6.1).

## 2. What crosses the pipe

`id` is a literal, so it stays in the state's own parameters. The other two are bindings, and they
live in a sibling member — never wrapped inside a parameter value, because a parameter value is user
data and an object with a `when-variant` member in it is a legitimate value (spec §6.2):

```json params
{ "variant-params": [
  { "name": "shipped",
    "dimension": "shippedAt",
    "cases": [ { "point": "present", "value": true },
               { "point": "absent", "value": false } ] },
  { "name": "status",
    "dimension": "status",
    "cases": [ { "point": "SHIPPED", "value": "SHIPPED" },
               { "point": "DELIVERED", "value": "DELIVERED" } ],
    "default": "PENDING" } ] }
```

The state as a whole, with the literal half restored (design 2.5 owns this document):

```json sketch
{ "name": "an order exists",
  "params": { "id": "42" },
  "variant-params": [
    { "name": "shipped", "dimension": "response.body.shippedAt#presence",
      "cases": [ { "point": "present", "value": true },
                 { "point": "absent", "value": false } ] },
    { "name": "status", "dimension": "response.body.status#value",
      "cases": [ { "point": "SHIPPED", "value": "SHIPPED" },
                 { "point": "DELIVERED", "value": "DELIVERED" } ],
      "default": "PENDING" } ] }
```

`shippedAt` and `status` have become `response.body.shippedAt#presence` and
`response.body.status#value`. The engine resolved them at `add-interaction`, and what is recorded is
the resolved id, so the verifier reading this pact resolves nothing and cannot resolve it differently
(spec §6.3).

## 3. Resolution, per variant

The eight selected variants resolve as follows (spec §6.4):

| # | origin | shippedAt | status | → `shipped` | → `status` |
|---|---|---|---|---|---|
| 1 | base | present | PENDING | `true` | `"PENDING"` (default — no case names `PENDING`) |
| 2 | boundary | absent | PENDING | `false` | `"PENDING"` |
| 3 | boundary | present | PENDING | `true` | `"PENDING"` |
| 4 | covering | present | SHIPPED | `true` | `"SHIPPED"` |
| 5 | covering | absent | SHIPPED | `false` | `"SHIPPED"` |
| 6 | covering | present | DELIVERED | `true` | `"DELIVERED"` |
| 7 | covering | absent | DELIVERED | `false` | `"DELIVERED"` |
| 8 | covering | present | PENDING | `true` | `"PENDING"` |

Variants 1, 3 and 8 resolve to the same state and differ only in dimensions no binding mentions —
`items` and `payment`. A state binding is a function of the dimensions it names and nothing else, so
identical states across variants are ordinary, not a sign that a variant is redundant.

Resolution is a total function of the assignment and the binding, so the consumer computes these when
recording an exercised variant and the verifier computes them again when replaying it, and the two
cannot disagree. The consumer records them anyway — evidence beats derivation when a pact file is read
five years later by a tool that does not implement §6.4.

## 4. What the pact records

Per exercised variant: the id, the assignment, the produced examples, and the resolved states
(spec §4.4; design 2.5 owns the encoding):

```json sketch
{ "variant": "response.body.items#cardinality=min+1;response.body.shippedAt#presence=absent;response.body.status#value=SHIPPED",
  "assignment": [ { "dimension": "response.body.items#cardinality", "point": "min+1" },
                  { "dimension": "response.body.payment#alternative", "point": "card" },
                  { "dimension": "response.body.shippedAt#presence", "point": "absent" },
                  { "dimension": "response.body.status#value", "point": "SHIPPED" } ],
  "states": [ { "name": "an order exists",
                "params": { "id": "42", "shipped": false, "status": "SHIPPED" } } ] }
```

## 5. Verification

For each recorded variant the verifier resolves the states, runs `state-setup` with them, replays the
recorded request example, and matches the response against the shape *pinned to that variant*
(spec §5.2):

```text
verifying 'get an order' — 8 recorded variants
  1. base                                          state {id:42, shipped:true,  status:PENDING}   ✓
  2. shippedAt=absent                              state {id:42, shipped:false, status:PENDING}   ✓
  3. items=min+1                                   state {id:42, shipped:true,  status:PENDING}   ✓
  4. payment=invoice;status=SHIPPED                state {id:42, shipped:true,  status:SHIPPED}   ✓
  5. items=min+1;shippedAt=absent;status=SHIPPED   state {id:42, shipped:false, status:SHIPPED}   ✗
  6. status=DELIVERED                              state {id:42, shipped:true,  status:DELIVERED} ✓
  7. items=min+1;payment=invoice;shippedAt=absent;status=DELIVERED
                                                   state {id:42, shipped:false, status:DELIVERED} ✓
  8. payment=invoice                               state {id:42, shipped:true,  status:PENDING}   ✓
```

**Eight variants, eight state setups** — including variants 1, 3 and 8, which resolve to exactly the
same state and differ only in `items` and `payment`, dimensions no binding mentions.

Reusing a setup across them is forbidden, not merely discouraged (spec §6.6). Two of the three are not
even adjacent, and reordering to make them so is forbidden on its own terms. But the deeper reason is
that the verifier cannot establish what skipping would require: variants 1 and 2 issued real requests
to a real provider between the setups, and whether `GET /orders/42` disturbed the order — a
last-accessed timestamp, an audit row, a cache — is invisible from this side of the boundary. Nor can
the verifier know whether this provider's handler is idempotent. Only whoever wrote it knows that.

The saving that buys is tiny and the failure it risks is the worst kind. If only `shipped` were bound,
the eight would run `true, false, true, true, false, true, false, true` and exactly one adjacent pair
— variants 3 and 4 — could collapse: one setup call in eight. In exchange, a variant that fails
because an earlier variant moved the data underneath it fails depending on what ran before it, and the
report blames the wrong interaction. Pairwise selection exists to vary dimensions *together*, so
consecutive variants rarely share a state anyway — the optimisation is small precisely where it is
most dangerous.

## 6. The variant the provider cannot produce

Variant 5 asks for an order that is `SHIPPED` with no `shippedAt`. The order service sets the
timestamp whenever it sets the status, so its state handler cannot build one. It says so rather than
silently building something else (design 2.7 owns the hook protocol):

```json sketch
{ "outcome": "unsupported",
  "reason": "an order cannot be SHIPPED without a shippedAt timestamp" }
```

The verifier reports variant 5 as `state-unavailable`, and the run fails (spec §6.7):

```text
✗ order-consumer / order-service — 1 of 8 variants not verified
  'get an order', variant items=min+1;shippedAt=absent;status=SHIPPED
    state 'an order exists' {shipped: false, status: SHIPPED}: unsupported
    "an order cannot be SHIPPED without a shippedAt timestamp"

  The consumer declared shippedAt as optional and status as one of three values,
  so this combination is part of the contract. Either exclude it with a reason,
  or narrow the declaration.
```

That is a contract finding wearing the clothes of a test failure, and reporting it as a pass would be
the exact false confidence variant testing exists to remove: the consumer proved it can *handle* the
variant, and nothing has proved the provider can *produce* it.

The fix is on the consumer side, and it is the exclusion from the
[sampling example](order-payload-sampling.md#5-excluding-a-combination-the-provider-cannot-produce):

```json policy
{ "exclude": [
  { "when": [ { "dimension": "status", "point": "SHIPPED" },
              { "dimension": "shippedAt", "point": "absent" } ],
    "reason": "the order service sets shippedAt whenever it sets SHIPPED" } ] }
```

The next consumer run selects eight variants that do not include the combination, records the
exclusion and its reason, and the verification passes — with the contract now saying, in writing, that this
region was never demonstrated.

**The waiver exists for the case where that loop is too slow.** A provider team blocked on a consumer
release can waive the variant, scoped and with a reason, and the summary reports it as waived rather
than passing (spec §6.7):

```json sketch
{ "allow-state-unavailable": [
  { "interaction": "get an order",
    "variant": "response.body.items#cardinality=min+1;response.body.shippedAt#presence=absent;response.body.status#value=SHIPPED",
    "reason": "PACT-1043: consumer to publish the exclusion in the next release" } ] }
```

There is deliberately no global switch. A per-variant waiver with a reason is a note somebody will
read in review; `--ignore-state-failures` is a flag that gets set once and outlives everyone who
understood why.

## 7. What is rejected at submission

All of these fail at `add-interaction`, while the author is still looking at the DSL that produced
them (spec §6.5):

```text
✗ interaction-invalid — 'get an order'
  states[0].variant-params[0].dimension: 'shipedAt' matches no dimension
    did you mean: response.body.shippedAt#presence
  states[0].variant-params[1].cases[0].point: dimension
    'response.body.status#value' has no point 'SHIPED'
    points: PENDING, SHIPPED, DELIVERED
  states[0].variant-params[2].dimension: 'dueDate' resolves to a gated dimension
    (response.body.payment@invoice.dueDate#presence) and the binding has no 'default'
  states[0].variant-params[3].name: 'id' is also a literal parameter of this state
```

The third is the one only a design that took gating seriously would catch. A binding on
`payment@invoice.dueDate#presence` has no case to apply in any `card` variant — four of the ten in §6
of the sampling example, the base and the minimal boundary among them — and without a `default` the state parameter would simply vanish for those variants.
Requiring `default` on a gated dimension turns a silent hole into a message.
