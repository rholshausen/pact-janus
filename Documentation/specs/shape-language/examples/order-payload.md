# Worked example — the RFC's order payload

The [RFC's consumer test](https://github.com/pact-foundation/roadmap/pull/146) declares one response
body with four sources of width. This file takes it from DSL to canonical shape to variant space to
produced values, and ends with what a subsumption check makes of a provider shape for the same
payload.

Every ```` ```json shape ```` block below is a complete shape document validated against
[`shape.schema.json`](../schemas/v1/shape.schema.json), and every ```` ```json variants ```` block
against [`variant-space.schema.json`](../schemas/v1/variant-space.schema.json), by
`cargo test -p pact_janus_schema_compat`. Blocks marked `value` are payloads, not shapes; blocks
marked `sketch` show documents owned by other designs.

## 1. The DSL the user writes

```typescript
.response({
  status: 200,
  body: json({
    id: integer(42),
    status: anyOf('PENDING', 'SHIPPED', 'DELIVERED'),
    shippedAt: optional(datetime('2026-07-30T10:00:00Z')),
    payment: oneOf('type', {
      card:    { type: 'card', last4: regex(/\d{4}/, '1234') },
      invoice: { type: 'invoice', dueDate: date('2026-08-30') },
    }),
    items: eachLike({ sku: string('SKU-1'), qty: integer(1) }, { min: 1 }),
  }),
});
```

## 2. The canonical shape the SDK sends

The DSL's job is sugar; what crosses the pipe is node objects (spec §3.2). The SDK adds no matching
logic — it maps `eachLike` to `each-like`, `anyOf` to `any-of`, and stops.

```json shape
{ "shape": "object",
  "members": {
    "id": { "shape": "integer", "example": 42 },
    "status": { "shape": "any-of",
                "options": ["PENDING", "SHIPPED", "DELIVERED"],
                "example": "PENDING" },
    "shippedAt": { "shape": "optional",
                   "of": { "shape": "datetime",
                           "format": "yyyy-MM-dd'T'HH:mm:ssX",
                           "example": "2026-07-30T10:00:00Z" } },
    "payment": { "shape": "one-of",
                 "discriminator": "type",
                 "default": "card",
                 "alternatives": {
                   "card": { "shape": "object",
                             "members": {
                               "type": { "shape": "equality", "example": "card" },
                               "last4": { "shape": "regex", "pattern": "\\d{4}", "example": "1234" } } },
                   "invoice": { "shape": "object",
                                "members": {
                                  "type": { "shape": "equality", "example": "invoice" },
                                  "dueDate": { "shape": "date", "format": "yyyy-MM-dd",
                                               "example": "2026-08-30" } } } } },
    "items": { "shape": "each-like",
               "min": 1,
               "items": { "shape": "object",
                          "members": {
                            "sku": { "shape": "string", "example": "SKU-1" },
                            "qty": { "shape": "integer", "example": 1 } } } } } }
```

Points worth noticing:

- `payment`'s alternatives each bind the discriminator `type` to a distinct literal, which is what
  makes the union tagged and its subsumption exact (spec §5.4).
- `last4`'s pattern is unanchored, like every v1–v4 regex (spec §4.2): it admits `"91234"`. An author
  who means four digits and nothing else writes `^\\d{4}$`.
- Nothing says "the body is an object with exactly these five members". Anything else the provider
  sends is admitted and ignored (spec §4.3).

## 3. The variant space

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
  { "id": "response.body.status#value", "path": "response.body.status",
    "facet": "value", "operator": "any-of", "default": "PENDING",
    "points": [ { "name": "PENDING", "value": "PENDING" },
                { "name": "SHIPPED", "value": "SHIPPED" },
                { "name": "DELIVERED", "value": "DELIVERED" } ] } ] }
```

Dimensions are emitted in the deterministic order of spec §6.2 — depth-first, object members
lexicographic — which is why `items` comes first and `status` last.

No dimension is gated: the `one-of` alternatives contain only `equality`, `regex` and `date` nodes,
none of which carry a dimension, and nothing lives under the `optional`. The space is therefore a
plain product:

| Dimension | Points |
|---|---|
| `response.body.items#cardinality` | 2 |
| `response.body.payment#alternative` | 2 |
| `response.body.shippedAt#presence` | 2 |
| `response.body.status#value` | 3 |
| **Variant space** | **24** |

**The RFC counts 12 here** — "3 statuses × `shippedAt` present/absent × 2 payment alternatives" — by
not counting the cardinality dimension that its own operator table declares for `eachLike`. The
arithmetic under this specification is 24. Which number is right is a design question and this is the
task that has to answer it: a declared `min: 1` with no exercised second element is an untested claim
about list handling, and consumer code that breaks on a two-element list is exactly the failure
variant testing exists to catch, so the dimension counts. The cost lands on design 2.3, not here —
pairwise sampling over these four dimensions needs 6 variants, not 24, and 2.3 owns the thresholds and
caps that keep it there.

## 4. Values produced for two variants

Base variant — every dimension at its default (`items=min`, `payment=card`, `shippedAt=present`,
`status=PENDING`):

```json value
{ "id": 42,
  "status": "PENDING",
  "shippedAt": "2026-07-30T10:00:00Z",
  "payment": { "type": "card", "last4": "1234" },
  "items": [ { "sku": "SKU-1", "qty": 1 } ] }
```

A variant at the other corner (`items=min+1`, `payment=invoice`, `shippedAt=absent`,
`status=DELIVERED`):

```json value
{ "id": 42,
  "status": "DELIVERED",
  "payment": { "type": "invoice", "dueDate": "2026-08-30" },
  "items": [ { "sku": "SKU-1", "qty": 1 }, { "sku": "SKU-1", "qty": 1 } ] }
```

`shippedAt` is not present with a null — it is not there (spec §2.1). The consumer test runs against
this response too, and if `order.shippedAt.getTime()` throws, the build fails. That is the whole point
of `optional` being honest.

Both elements of `items` are alike, because dimensions inside `each-like` are shared across elements
(spec §6.5): the second element is another `SKU-1`, not a second chance to vary.

## 5. What the provider may send

Matching is membership (spec §7.1). Against the **whole** shape:

```json value
{ "id": 7,
  "status": "SHIPPED",
  "shippedAt": "2026-08-01T09:30:00Z",
  "payment": { "type": "card", "last4": "0000", "network": "visa" },
  "items": [ { "sku": "SKU-9", "qty": 3 }, { "sku": "SKU-2", "qty": 1 } ],
  "warehouse": "AKL-1" }
```

Admitted: `network` and `warehouse` are unnamed members, so they are ignored; `items` has two
elements, inside `[1, ∞)`; the elements differ, which the shape permits — element *values* are free,
it is the *dimension points* that are shared.

Rejected, one reason each:

| Value | Verdict |
|---|---|
| `"status": "REFUNDED"` | not among the `any-of` options |
| `"shippedAt": null` | `optional` admits `⊥` or a datetime, not `null` — `nullable` is a different declaration |
| `"payment": { "type": "cheque", "chequeNo": "0007" }` | no alternative binds `type` to `cheque`; the checker names the discriminator value it read |
| `"items": []` | below `min: 1` |
| `"items": { "SKU-1": 3 }` | an object where the shape admits arrays |

Under a variant the shape is narrower still: served or verified with `status` pinned to `SHIPPED`,
only `"SHIPPED"` is admitted, not the other two options.

## 6. `forbidden`, and why there is no closed object

The must-ignore default is not negotiable, but a specific field can be asserted absent:

```json shape
{ "shape": "object",
  "members": {
    "id": { "shape": "integer", "example": 42 },
    "ssn": { "shape": "forbidden" },
    "internalNotes": { "shape": "forbidden" } } }
```

This says what a PII assertion actually means — *these* fields must not appear — while leaving the
provider free to add fields nobody has an opinion about. A "closed object" operator would say the
unusable thing instead (spec §4.3), and would make the RFC's "extra fields are fine" subsumption rule
impossible to state.

## 7. What a subsumption check makes of a provider shape

The provider publishes its own response shape (RFC; design 2.8 owns the walk). Suppose it is the same
tree with four differences:

```json shape
{ "shape": "object",
  "members": {
    "id": { "shape": "integer", "example": 42 },
    "status": { "shape": "any-of",
                "options": ["PENDING", "SHIPPED", "DELIVERED", "CANCELLED"],
                "example": "PENDING" },
    "shippedAt": { "shape": "optional",
                   "of": { "shape": "datetime", "format": "yyyy-MM-dd'T'HH:mm:ssX",
                           "example": "2026-07-30T10:00:00Z" } },
    "payment": { "shape": "one-of",
                 "discriminator": "type",
                 "alternatives": {
                   "card": { "shape": "object",
                             "members": {
                               "type": { "shape": "equality", "example": "card" },
                               "last4": { "shape": "regex", "pattern": "[0-9]{4}", "example": "1234" } } },
                   "invoice": { "shape": "object",
                                "members": {
                                  "type": { "shape": "equality", "example": "invoice" },
                                  "dueDate": { "shape": "date", "format": "yyyy-MM-dd",
                                               "example": "2026-08-30" } } } } },
    "items": { "shape": "each-like",
               "min": 0,
               "items": { "shape": "object",
                          "members": {
                            "sku": { "shape": "string", "example": "SKU-1" },
                            "qty": { "shape": "number", "example": 1 },
                            "backordered": { "shape": "boolean", "example": false } } } } } }
```

`admits(provider) ⊆ admits(consumer)`, node by node (spec §8):

| Node | Provider vs consumer | Answer | Why |
|---|---|---|---|
| `status` | 4 options vs 3 | **no** | `CANCELLED ∉ admits(C)`; finite sets, exact |
| `items` cardinality | `[0, ∞)` vs `[1, ∞)` | **no** | the empty array is admitted by P, not by C — interval containment, exact |
| `items[*].qty` | `number` vs `integer` | **no** | kind lattice, exact: `integer ⊂ number` |
| `items[*].backordered` | extra member | **yes** | must-ignore: the consumer named no such member |
| `payment@card.last4` | `[0-9]{4}` vs `\d{4}` | **unknown** | two different regexes: conservative class, no guessing |
| everything else | identical nodes | **yes** | the identity floor |

Three decided findings, one honest "review this", no false confidence. The consumer's fix for the
first three is to widen the declaration — `anyOf(..., 'CANCELLED')`, `{ min: 0 }`, `number` — which
variant testing then forces them to actually exercise. That loop, subsumption pushing declarations
wider and variant testing keeping them demonstrated, is the pair of mechanisms the RFC builds this
language for.

## 8. How this reaches the engine

The shape travels inside an interaction specification on `consumer-session/add-interaction`, and the
variant space comes back — as descriptors carrying at least `id` (protocol spec §8.2) — from
`consumer-session/variants`. Interiors below belong to designs 2.5/3.2 and 2.3:

```json sketch
{ "description": "a request for an order",
  "transport": { "kind": "http", "mode": "passive" },
  "request": { "method": "GET", "path": "/orders/42" },
  "response": { "status": 200, "body": { "content-type": "application/json" } } }
```
