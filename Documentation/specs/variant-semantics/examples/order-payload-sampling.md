# Worked example — sampling the RFC's order payload

[Design 2.2's worked example](../../shape-language/examples/order-payload.md) takes the RFC's order
response from DSL to shape to variant space and stops there, with 24 variants and a note that
"pairwise sampling over these four dimensions needs 6 variants, not 24". This file is where that
number is produced rather than asserted — 6 for the covering array, 8 once the base and boundary
variants are seeded — and where the rest of the machinery (the exhaustive threshold, pins, exclusions,
gating and the budget) is shown on the same payload.

Every ```` ```json selection ````, ```` ```json policy ```` and ```` ```json variants ```` block below
is validated against the schemas by `cargo test -p pact_janus_schema_compat`. The selections were
computed with the algorithm of [spec §3.4](../spec.md#34-the-selection-algorithm); if an implementation
disagrees with them, one of the two is wrong and the test says which.

## 1. The space

The shape's four dimensions, as design 2.2 emits them:

```json variants
{ "dimensions": [
  { "id": "response.body.items#cardinality", "path": "response.body.items",
    "facet": "cardinality", "operator": "each-like", "default": "min",
    "points": [ { "name": "min", "size": 1 }, { "name": "min+1", "size": 2 } ] },
  { "id": "response.body.payment#alternative", "path": "response.body.payment",
    "facet": "alternative", "operator": "one-of", "default": "card",
    "points": [ { "name": "card" }, { "name": "invoice" } ] },
  { "id": "response.body.shippedAt#presence", "path": "response.body.shippedAt",
    "facet": "presence", "operator": "optional", "default": "present",
    "points": [ { "name": "present" }, { "name": "absent" } ] },
  { "id": "response.body.status#value", "path": "response.body.status", "facet": "value",
    "operator": "any-of", "default": "PENDING",
    "points": [ { "name": "PENDING", "value": "PENDING" },
                { "name": "SHIPPED", "value": "SHIPPED" },
                { "name": "DELIVERED", "value": "DELIVERED" } ] } ] }
```

Nothing is gated, so the size is the plain product: 2 × 2 × 2 × 3 = **24** (spec §2.3). At strength 2
there are **30 reachable pairs** — 4 + 4 + 6 + 4 + 6 + 6, summed over the six pairs of dimensions.

24 is above the exhaustive threshold of 8, so `auto` resolves to `t-wise` at strength 2 (spec §3.2).

## 2. The selection

```json selection
{ "variants": [
  { "id": "base", "label": "base", "origin": "base",
    "assignment": [ { "dimension": "response.body.items#cardinality", "point": "min" },
                    { "dimension": "response.body.payment#alternative", "point": "card" },
                    { "dimension": "response.body.shippedAt#presence", "point": "present" },
                    { "dimension": "response.body.status#value", "point": "PENDING" } ] },
  { "id": "response.body.shippedAt#presence=absent",
    "label": "shippedAt=absent", "origin": "boundary",
    "assignment": [ { "dimension": "response.body.items#cardinality", "point": "min" },
                    { "dimension": "response.body.payment#alternative", "point": "card" },
                    { "dimension": "response.body.shippedAt#presence", "point": "absent" },
                    { "dimension": "response.body.status#value", "point": "PENDING" } ] },
  { "id": "response.body.items#cardinality=min+1",
    "label": "items=min+1", "origin": "boundary",
    "assignment": [ { "dimension": "response.body.items#cardinality", "point": "min+1" },
                    { "dimension": "response.body.payment#alternative", "point": "card" },
                    { "dimension": "response.body.shippedAt#presence", "point": "present" },
                    { "dimension": "response.body.status#value", "point": "PENDING" } ] },
  { "id": "response.body.payment#alternative=invoice;response.body.status#value=SHIPPED",
    "label": "payment=invoice;status=SHIPPED", "origin": "covering",
    "assignment": [ { "dimension": "response.body.items#cardinality", "point": "min" },
                    { "dimension": "response.body.payment#alternative", "point": "invoice" },
                    { "dimension": "response.body.shippedAt#presence", "point": "present" },
                    { "dimension": "response.body.status#value", "point": "SHIPPED" } ] },
  { "id": "response.body.items#cardinality=min+1;response.body.shippedAt#presence=absent;response.body.status#value=SHIPPED",
    "label": "items=min+1;shippedAt=absent;status=SHIPPED", "origin": "covering",
    "assignment": [ { "dimension": "response.body.items#cardinality", "point": "min+1" },
                    { "dimension": "response.body.payment#alternative", "point": "card" },
                    { "dimension": "response.body.shippedAt#presence", "point": "absent" },
                    { "dimension": "response.body.status#value", "point": "SHIPPED" } ] },
  { "id": "response.body.status#value=DELIVERED",
    "label": "status=DELIVERED", "origin": "covering",
    "assignment": [ { "dimension": "response.body.items#cardinality", "point": "min" },
                    { "dimension": "response.body.payment#alternative", "point": "card" },
                    { "dimension": "response.body.shippedAt#presence", "point": "present" },
                    { "dimension": "response.body.status#value", "point": "DELIVERED" } ] },
  { "id": "response.body.items#cardinality=min+1;response.body.payment#alternative=invoice;response.body.shippedAt#presence=absent;response.body.status#value=DELIVERED",
    "label": "items=min+1;payment=invoice;shippedAt=absent;status=DELIVERED", "origin": "covering",
    "assignment": [ { "dimension": "response.body.items#cardinality", "point": "min+1" },
                    { "dimension": "response.body.payment#alternative", "point": "invoice" },
                    { "dimension": "response.body.shippedAt#presence", "point": "absent" },
                    { "dimension": "response.body.status#value", "point": "DELIVERED" } ] },
  { "id": "response.body.payment#alternative=invoice",
    "label": "payment=invoice", "origin": "covering",
    "assignment": [ { "dimension": "response.body.items#cardinality", "point": "min" },
                    { "dimension": "response.body.payment#alternative", "point": "invoice" },
                    { "dimension": "response.body.shippedAt#presence", "point": "present" },
                    { "dimension": "response.body.status#value", "point": "PENDING" } ] } ],
  "report": {
    "space": { "size": 24, "exact": true, "dimensions": 4 },
    "strategy": "t-wise", "strength": 2, "algorithm": "janus-ipog-v1", "selected": 8,
    "coverage": { "targets": 30, "covered": 30, "removed": 0, "dropped": 0 },
    "budgets": { "exhaustive-threshold": 8, "max-variants": 50 },
    "boundaries": true } }
```

Read as a table, which is how `explain` renders it (spec §7):

| # | items | payment | shippedAt | status | origin |
|---|---|---|---|---|---|
| 1 | min | card | present | PENDING | base |
| 2 | min | card | **absent** | PENDING | boundary (minimal) |
| 3 | **min+1** | card | present | PENDING | boundary (maximal) |
| 4 | min | invoice | present | SHIPPED | covering |
| 5 | min+1 | card | absent | SHIPPED | covering |
| 6 | min | card | present | DELIVERED | covering |
| 7 | min+1 | invoice | absent | DELIVERED | covering |
| 8 | min | invoice | present | PENDING | covering |

Eight variants: six for the covering array, two for the boundaries. Rows 4–8 cover 30 of 30 pairs on
their own — every `status` appears with every `payment`, with both `items` cardinalities and with both
`shippedAt` presences, and the same for every other pair of columns. What is *not* covered is any
particular triple: `(min, invoice, absent)` never runs, and that is the strength-2 bargain, stated
rather than hidden.

Rows 2 and 3 are not there for coverage at all. They are the least and the most this contract permits
the provider to send: nothing optional, one item; everything optional, two items. Pairwise would cover
`absent` and `min+1` anyway — it does, in rows 5 and 7 — but never in isolation, and a pact whose
recorded evidence skips both ends of its own declared width has recorded the wrong samples.

**Note what the maximal variant is not.** Row 3 has `payment = card`, because `alternative` is an
unordered facet and the maximal variant takes the default (spec §3.1). It is the largest payload
*among the card responses*, not among all of them — `one-of` alternatives are mutually exclusive, so
no single variant can be the maximum, and the report does not claim one is.

Three details of the algorithm are visible here:

- **The base variant is row 1**, and the RFC's own example payload is what it produces (`PENDING`, a
  card payment, one item, `shippedAt` present). The first failure a user sees is the ordinary case.
- **`status` was ordered first** by step 2 — it has three points, the others two — so the initial block
  is `status × items` and the covering array is 6, the size of the largest single pair set. Ordering
  the dimensions as the shape emits them instead yields 7 for the same coverage.
- **Ids omit defaults**, so the boundary variants have the shortest ids in the selection: row 2 is
  `response.body.shippedAt#presence=absent` and nothing else. Adding a fifth dimension to the shape
  leaves all eight ids unchanged (spec §2.2).

## 3. A smaller space runs exhaustively

Drop `anyOf` and `eachLike` from the response and two dimensions remain — `payment` (2) and
`shippedAt` (2) — for a space of 4. That is at or below the threshold of 8, so `auto` selects
`exhaustive` and all four variants run; the seeds of §3.1 are already among them, so they cost
nothing. Pairwise would also have selected 4 here: at two dimensions, "every pair" *is* the whole
space (spec §3.2's table). The threshold matters at three dimensions and up, where 8 variants becomes
4, and it stops mattering at 12, where the saving is large enough that sampling is the point.

## 4. Pinning

An author who cares specifically about the shipped-with-no-timestamp case says so:

```json policy
{ "pin": [
  { "assignment": [ { "dimension": "status", "point": "SHIPPED" },
                    { "dimension": "shippedAt", "point": "absent" } ],
    "reason": "the case the order-history screen renders differently" } ] }
```

Both references are short forms, resolved against the space by the engine (spec §6.3):
`status` → `response.body.status#value`, `shippedAt` → `response.body.shippedAt#presence`.

The pin is a *partial* assignment: `items` and `payment` are unspecified and complete to their
defaults, giving `(min, card, absent, SHIPPED)`. It lands as row 4, behind the base and the two
boundaries (spec §3.1), and the selection grows to **9** variants, still covering 30 of 30 pairs — the
pin is a seed, so the covering step works around it rather than duplicating it. A pin never shrinks
coverage and rarely costs more than the one variant it names.

## 5. Excluding a combination the provider cannot produce

The order service sets `shippedAt` whenever it sets `SHIPPED`, so the pair `(SHIPPED, absent)` is not
something any provider run can produce. Nothing in the shape says so — dimensions are independent by
construction — so it is said in the policy, with a reason (spec §3.5):

```json policy
{ "exclude": [
  { "when": [ { "dimension": "status", "point": "SHIPPED" },
              { "dimension": "shippedAt", "point": "absent" } ],
    "reason": "the order service sets shippedAt whenever it sets SHIPPED" } ] }
```

The selection stays at **8** variants and the coverage report changes to `targets: 29, covered: 29,
removed: 1`: the excluded pair is not counted as a miss, because it is not a target. Row 5 of §2 —
`(min+1, card, absent, SHIPPED)` — is replaced by `(min+1, card, present, SHIPPED)`, and the run no
longer asks the provider for something it cannot do.

The boundary variants are untouched: the minimal variant is `(min, card, absent, PENDING)`, which the
exclusion does not name. An exclusion that *did* cover a boundary would remove it like any other
variant — the seeds are variants of the space, not exemptions from it.

What has *not* changed is what the contract admits. `admits` is untouched: a provider that does return
`SHIPPED` with no `shippedAt` still matches the shape. The exclusion says only that the combination
was never demonstrated, and the reason travels into the pact file so a reader knows it was a decision
rather than an accident.

## 6. Gating

Give the `invoice` alternative an optional `dueDate` and a fifth dimension appears, gated on
`payment#alternative = invoice`:

```json variants
{ "dimensions": [
  { "id": "response.body.payment#alternative", "path": "response.body.payment",
    "facet": "alternative", "operator": "one-of", "default": "card",
    "points": [ { "name": "card" }, { "name": "invoice" } ] },
  { "id": "response.body.payment@invoice.dueDate#presence",
    "path": "response.body.payment@invoice.dueDate", "facet": "presence",
    "operator": "optional", "default": "present",
    "points": [ { "name": "present" }, { "name": "absent" } ],
    "gated-by": [ { "dimension": "response.body.payment#alternative", "point": "invoice" } ] } ] }
```

Added to the other three dimensions of §1, the arithmetic is not 2 × 2 × 2 × 3 × 2 = 48. `payment`
weighs `1 + 2 = 3` — one way to be a card, two ways to be an invoice — so the space is
2 × 3 × 2 × 3 = **36** (spec §2.3).

Reachable pairs are 46, not 60: `dueDate` pairs with `items`, `shippedAt` and `status` freely (4 + 4 +
6 = 14) but with `payment` only through `invoice` (2), because `(dueDate = present, payment = card)`
is not a variant. The selection is **11** variants — 9 for the covering array, 2 for the boundaries —
covering all 46. Five of them are `card` variants in which `dueDate` is inactive and therefore has no
point at all in the assignment: not `null`, not `"absent"`, absent from the list (spec §2.1).

Both boundary variants are among those five, which is the `one-of` limit in practice. `dueDate` is
gated on `invoice`, `alternative` is unordered, so the maximal variant takes `card` and never opens
the alternative that contains the extra optional. The largest payload this shape admits is an invoice
with a `dueDate` and two items, and no boundary variant is it.

## 7. When the budget bites

Take a different response: two `anyOf` enumerations of 30 currencies each, one for the order and one
for the settlement, and nothing else. The space is 900, and so is the pairwise sample: every one of the 900 pairs needs a variant
of its own, because no variant can contain two points of the same dimension. That exceeds
`max-variants`, and the engine fails rather than returning the first 50:

```
✗ variant-budget-exceeded — interaction 'get an order'
  space 900, selection would be 900, budget 50
  largest contributors:
    response.body.currency#value            30 points
    response.body.settlementCurrency#value  30 points
  Narrow the declaration, exclude a region, pin the variants that matter, or raise
  max-variants for this interaction.
```

The message is the design (spec §3.6). Two 30-value enumerations multiplied together is not a contract
about a currency field; it is a data set being tested through a mock server, and the four ways out are
all decisions the author should be making rather than the sampler making them silently.

## 8. What reaches the engine

The host loops over the selection in order (spec §4.1–4.3):

```json sketch
[ { "op": "consumer-session/variants", "body": { "session": "s-1", "handle": "h-1" } },
  { "op": "consumer-session/serve-variant",
    "body": { "session": "s-1", "handle": "h-1", "variant": "base" } },
  { "op": "consumer-session/serve-variant",
    "body": { "session": "s-1", "handle": "h-1",
              "variant": "response.body.payment#alternative=invoice;response.body.status#value=SHIPPED" } } ]
```

`serve-variant` takes the id, never the label. If the host runs seven of the eight and calls
`finalise`, the eighth is reported `not-exercised` and no pact comes back (protocol spec §8.2, spec §4.2) — the
declared width was a promise, and it was not kept.
