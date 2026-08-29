# Worked example — the RFC's order interaction, recorded

[Design 2.2's example](../../shape-language/examples/order-payload.md) takes the RFC's order response
from DSL to shape to a 24-variant space. [Design 2.3's](../../variant-semantics/examples/order-payload-sampling.md)
samples that space down to eight variants. This file is where those eight become a file on disk: what
the consumer actually writes when `finalise` succeeds, and what a verifier later reads and nothing
else.

The ```` ```json contract ```` block is a complete contract validated against
[`contract.schema.json`](../schemas/v1/contract.schema.json). Its `selection` member is *also*
validated against design 2.3's [`variant-selection.schema.json`](../../variant-semantics/schemas/v1/variant-selection.schema.json),
because it is that document with this design's evidence added (spec §5.2) — if the two schemas ever
disagreed about it, the test would say so. `shape` and `params` blocks go to their owners' schemas.
All by `cargo test -p pact_janus_schema_compat`.

## 1. The shape, recorded once

The interaction's `response.body` slot holds design 2.2's shape unchanged — a contract stores it, it
does not restate it:

```json shape
{ "shape": "object",
  "members": {
    "id": { "shape": "integer", "example": 42 },
    "status": { "shape": "any-of", "options": ["PENDING", "SHIPPED", "DELIVERED"], "example": "PENDING" },
    "shippedAt": { "shape": "optional",
                   "of": { "shape": "datetime", "format": "yyyy-MM-dd'T'HH:mm:ssX",
                           "example": "2026-07-30T10:00:00Z" } },
    "payment": { "shape": "one-of", "discriminator": "type", "default": "card",
                 "alternatives": {
                   "card": { "shape": "object",
                             "members": { "type": { "shape": "equality", "example": "card" },
                                          "last4": { "shape": "regex", "pattern": "\\d{4}", "example": "1234" } } },
                   "invoice": { "shape": "object",
                                "members": { "type": { "shape": "equality", "example": "invoice" },
                                             "dueDate": { "shape": "date", "format": "yyyy-MM-dd",
                                                          "example": "2026-08-30" } } } } },
    "items": { "shape": "each-like", "min": 1,
               "items": { "shape": "object",
                          "members": { "sku": { "shape": "string", "example": "SKU-1" },
                                       "qty": { "shape": "integer", "example": 1 } } } } } }
```

## 2. The state bindings, recorded once

Bindings live on the interaction; their resolved values live on each variant (spec §6). The
`dimension` members are resolved ids, never the `shippedAt` shorthand the author wrote:

```json params
{ "variant-params": [
  { "name": "shipped",
    "dimension": "response.body.shippedAt#presence",
    "cases": [ { "point": "present", "value": true },
               { "point": "absent", "value": false } ] },
  { "name": "status",
    "dimension": "response.body.status#value",
    "cases": [ { "point": "SHIPPED", "value": "SHIPPED" },
               { "point": "DELIVERED", "value": "DELIVERED" } ],
    "default": "PENDING" } ] }
```

## 3. The contract

Shapes elided with `…` below are the ones printed in full above; everything else is the file verbatim.

```json contract
{ "$format": "janus-contract/1",
  "consumer": { "name": "orders-ui" },
  "provider": { "name": "orders-api" },
  "interactions": [
    { "description": "get an order",
      "transport": { "kind": "http", "mode": "passive" },
      "states": [
        { "name": "an order exists",
          "params": { "id": "42" },
          "variant-params": [
            { "name": "shipped", "dimension": "response.body.shippedAt#presence",
              "cases": [ { "point": "present", "value": true }, { "point": "absent", "value": false } ] },
            { "name": "status", "dimension": "response.body.status#value",
              "cases": [ { "point": "SHIPPED", "value": "SHIPPED" }, { "point": "DELIVERED", "value": "DELIVERED" } ],
              "default": "PENDING" } ] } ],
      "parts": {
        "request": {
          "method": { "shape": "equality", "example": "GET" },
          "path": { "shape": "equality", "example": "/orders/42" } },
        "response": {
          "status": { "shape": "equality", "example": 200 },
          "body": { "shape": "object", "members": {} } } },
      "selection": {
        "variants": [
          { "id": "base", "origin": "base",
            "assignment": [ { "dimension": "response.body.items#cardinality", "point": "min" },
                            { "dimension": "response.body.payment#alternative", "point": "card" },
                            { "dimension": "response.body.shippedAt#presence", "point": "present" },
                            { "dimension": "response.body.status#value", "point": "PENDING" } ],
            "states": [ { "name": "an order exists",
                          "params": { "id": "42", "shipped": true, "status": "PENDING" } } ],
            "parts": {
              "request": { "method": { "content": "GET" }, "path": { "content": "/orders/42" } },
              "response": { "status": { "content": 200 },
                            "body": { "content": { "id": 42, "status": "PENDING",
                                                   "shippedAt": "2026-07-30T10:00:00Z",
                                                   "payment": { "type": "card", "last4": "1234" },
                                                   "items": [ { "sku": "SKU-1", "qty": 1 } ] },
                                      "encoded": "json", "content-type": "application/json" } } } },
          { "id": "response.body.shippedAt#presence=absent", "origin": "boundary",
            "assignment": [ { "dimension": "response.body.items#cardinality", "point": "min" },
                            { "dimension": "response.body.payment#alternative", "point": "card" },
                            { "dimension": "response.body.shippedAt#presence", "point": "absent" },
                            { "dimension": "response.body.status#value", "point": "PENDING" } ],
            "states": [ { "name": "an order exists",
                          "params": { "id": "42", "shipped": false, "status": "PENDING" } } ],
            "parts": {
              "request": { "method": { "content": "GET" }, "path": { "content": "/orders/42" } },
              "response": { "status": { "content": 200 },
                            "body": { "content": { "id": 42, "status": "PENDING",
                                                   "payment": { "type": "card", "last4": "1234" },
                                                   "items": [ { "sku": "SKU-1", "qty": 1 } ] },
                                      "encoded": "json", "content-type": "application/json" } } } },
          { "id": "response.body.items#cardinality=min+1", "origin": "boundary",
            "assignment": [ { "dimension": "response.body.items#cardinality", "point": "min+1" },
                            { "dimension": "response.body.payment#alternative", "point": "card" },
                            { "dimension": "response.body.shippedAt#presence", "point": "present" },
                            { "dimension": "response.body.status#value", "point": "PENDING" } ],
            "states": [ { "name": "an order exists",
                          "params": { "id": "42", "shipped": true, "status": "PENDING" } } ],
            "parts": {
              "request": { "method": { "content": "GET" }, "path": { "content": "/orders/42" } },
              "response": { "status": { "content": 200 },
                            "body": { "content": { "id": 42, "status": "PENDING",
                                                   "shippedAt": "2026-07-30T10:00:00Z",
                                                   "payment": { "type": "card", "last4": "1234" },
                                                   "items": [ { "sku": "SKU-1", "qty": 1 },
                                                              { "sku": "SKU-2", "qty": 2 } ] },
                                      "encoded": "json", "content-type": "application/json" } } } },
          { "id": "response.body.payment#alternative=invoice;response.body.status#value=SHIPPED",
            "origin": "covering",
            "assignment": [ { "dimension": "response.body.items#cardinality", "point": "min" },
                            { "dimension": "response.body.payment#alternative", "point": "invoice" },
                            { "dimension": "response.body.shippedAt#presence", "point": "present" },
                            { "dimension": "response.body.status#value", "point": "SHIPPED" } ],
            "states": [ { "name": "an order exists",
                          "params": { "id": "42", "shipped": true, "status": "SHIPPED" } } ],
            "parts": {
              "request": { "method": { "content": "GET" }, "path": { "content": "/orders/42" } },
              "response": { "status": { "content": 200 },
                            "body": { "content": { "id": 42, "status": "SHIPPED",
                                                   "shippedAt": "2026-07-30T10:00:00Z",
                                                   "payment": { "type": "invoice", "dueDate": "2026-08-30" },
                                                   "items": [ { "sku": "SKU-1", "qty": 1 } ] },
                                      "encoded": "json", "content-type": "application/json" } } } },
          { "id": "response.body.items#cardinality=min+1;response.body.shippedAt#presence=absent;response.body.status#value=SHIPPED",
            "origin": "covering",
            "assignment": [ { "dimension": "response.body.items#cardinality", "point": "min+1" },
                            { "dimension": "response.body.payment#alternative", "point": "card" },
                            { "dimension": "response.body.shippedAt#presence", "point": "absent" },
                            { "dimension": "response.body.status#value", "point": "SHIPPED" } ],
            "states": [ { "name": "an order exists",
                          "params": { "id": "42", "shipped": false, "status": "SHIPPED" } } ],
            "parts": {
              "request": { "method": { "content": "GET" }, "path": { "content": "/orders/42" } },
              "response": { "status": { "content": 200 },
                            "body": { "content": { "id": 42, "status": "SHIPPED",
                                                   "payment": { "type": "card", "last4": "1234" },
                                                   "items": [ { "sku": "SKU-1", "qty": 1 },
                                                              { "sku": "SKU-2", "qty": 2 } ] },
                                      "encoded": "json", "content-type": "application/json" } } } },
          { "id": "response.body.status#value=DELIVERED", "origin": "covering",
            "assignment": [ { "dimension": "response.body.items#cardinality", "point": "min" },
                            { "dimension": "response.body.payment#alternative", "point": "card" },
                            { "dimension": "response.body.shippedAt#presence", "point": "present" },
                            { "dimension": "response.body.status#value", "point": "DELIVERED" } ],
            "states": [ { "name": "an order exists",
                          "params": { "id": "42", "shipped": true, "status": "DELIVERED" } } ],
            "parts": {
              "request": { "method": { "content": "GET" }, "path": { "content": "/orders/42" } },
              "response": { "status": { "content": 200 },
                            "body": { "content": { "id": 42, "status": "DELIVERED",
                                                   "shippedAt": "2026-07-30T10:00:00Z",
                                                   "payment": { "type": "card", "last4": "1234" },
                                                   "items": [ { "sku": "SKU-1", "qty": 1 } ] },
                                      "encoded": "json", "content-type": "application/json" } } } },
          { "id": "response.body.items#cardinality=min+1;response.body.payment#alternative=invoice;response.body.shippedAt#presence=absent;response.body.status#value=DELIVERED",
            "origin": "covering",
            "assignment": [ { "dimension": "response.body.items#cardinality", "point": "min+1" },
                            { "dimension": "response.body.payment#alternative", "point": "invoice" },
                            { "dimension": "response.body.shippedAt#presence", "point": "absent" },
                            { "dimension": "response.body.status#value", "point": "DELIVERED" } ],
            "states": [ { "name": "an order exists",
                          "params": { "id": "42", "shipped": false, "status": "DELIVERED" } } ],
            "parts": {
              "request": { "method": { "content": "GET" }, "path": { "content": "/orders/42" } },
              "response": { "status": { "content": 200 },
                            "body": { "content": { "id": 42, "status": "DELIVERED",
                                                   "payment": { "type": "invoice", "dueDate": "2026-08-30" },
                                                   "items": [ { "sku": "SKU-1", "qty": 1 },
                                                              { "sku": "SKU-2", "qty": 2 } ] },
                                      "encoded": "json", "content-type": "application/json" } } } },
          { "id": "response.body.payment#alternative=invoice", "origin": "covering",
            "assignment": [ { "dimension": "response.body.items#cardinality", "point": "min" },
                            { "dimension": "response.body.payment#alternative", "point": "invoice" },
                            { "dimension": "response.body.shippedAt#presence", "point": "present" },
                            { "dimension": "response.body.status#value", "point": "PENDING" } ],
            "states": [ { "name": "an order exists",
                          "params": { "id": "42", "shipped": true, "status": "PENDING" } } ],
            "parts": {
              "request": { "method": { "content": "GET" }, "path": { "content": "/orders/42" } },
              "response": { "status": { "content": 200 },
                            "body": { "content": { "id": 42, "status": "PENDING",
                                                   "shippedAt": "2026-07-30T10:00:00Z",
                                                   "payment": { "type": "invoice", "dueDate": "2026-08-30" },
                                                   "items": [ { "sku": "SKU-1", "qty": 1 } ] },
                                      "encoded": "json", "content-type": "application/json" } } } } ],
        "report": {
          "space": { "size": 24, "exact": true, "dimensions": 4 },
          "strategy": "t-wise", "strength": 2, "algorithm": "janus-ipog-v1", "selected": 8,
          "coverage": { "targets": 30, "covered": 30, "removed": 0, "dropped": 0 },
          "budgets": { "exhaustive-threshold": 8, "max-variants": 50 },
          "boundaries": true } } } ],
  "metadata": { "writer": { "pact-janus": "0.1.0" } } }
```

## 4. What to notice

**The shape appears once and the evidence eight times.** That asymmetry is the format: the shape is
what the provider is held to, and each variant is a demonstration that the consumer really can handle
one point of it. A verifier replays the eight recorded requests in this order and matches each response
against the shape **pinned** to that variant's assignment — so variant 2, pinned to
`shippedAt = absent`, rejects a provider that helpfully sends a timestamp (spec §5.4).

**No labels.** Design 2.3 renders `shippedAt=absent` for humans, but a label depends on the space it was
derived in and would silently change meaning when the shape grows a dimension. Only ids and assignments
are recorded (spec §5.2), and the ids here are verbose on purpose.

**No plan, anywhere.** A plan is how *this* engine executes this shape today (plan grammar §7). Freezing
one into the file would make a five-year-old contract depend on a five-year-old compiler.

**The request parts repeat, identically, eight times.** That is honest — the request really was the same
each time — and it is also the clearest illustration of where a contract's size comes from: the variant
multiplier, not base64. The [format review](../../../contract-file-format-review.md) §8.2 puts a
tripwire on it rather than optimising it away before there is anything to measure.

**`metadata` has no timestamp.** This contract is reproducible: run the same consumer build twice and
get the same bytes, so a broker sees one contract version and a repository sees no diff (spec §3.2).
