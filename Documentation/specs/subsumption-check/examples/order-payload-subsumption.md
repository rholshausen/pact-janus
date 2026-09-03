# Worked example — subsuming the RFC's order payload

This file picks up exactly where [shape-language's order-payload
example](../../shape-language/examples/order-payload.md) leaves off: §2 there is the consumer's declared
shape, §7 there is a provider shape for the same operation with four differences. This file turns that
prose table into the documents a checker actually produces — a `ProviderShape`, the walk's per-node
verdicts, a `SubsumptionReport`, and the text rendering a person reads — and then adds two small,
self-contained comparisons the order payload does not happen to exercise: a member the provider's shape
does not name, and a presence widening.

Fence markers: ```` ```json provider-shape ```` validates against
[`provider-shape.schema.json`](../schemas/v1/provider-shape.schema.json), ```` ```json report ```` against
[`subsumption-report.schema.json`](../schemas/v1/subsumption-report.schema.json), ```` ```json finding ````
against [`finding.schema.json`](../schemas/v1/finding.schema.json); ```` ```json sketch ```` blocks are
shapes owned by design 2.2, reproduced for context and not revalidated here.

## 1. What the consumer declared, and what the provider publishes

The consumer's shape (shape-language example §2) and the provider's published shape (shape-language
example §7) are not repeated in full here — follow the links. The provider wraps its shape in the
artifact this design defines:

```json provider-shape
{ "$format": "janus-provider-shape/1",
  "provider": { "name": "orders-api" },
  "provenance": "recorded",
  "interactions": [
    { "description": "get an order",
      "states": [ { "name": "an order exists" } ],
      "source": { "test-run": "orders-api#4821" },
      "parts": {
        "response": {
          "body": { "shape": "object",
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
                                         "last4": { "shape": "regex", "pattern": "[0-9]{4}",
                                                    "example": "1234" } } },
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
                                      "backordered": { "shape": "boolean", "example": false } } } } } } } } } ] }
```

`source` is this document's open provenance detail (spec.md §2.4): here, which of the provider's own test
runs the recorder unioned to produce this shape (task 7.2).

## 2. The walk, node by node

This is shape-language example §7's table, unchanged in its verdicts — this design does not decide
anything shape spec §8 didn't already fix — but now labelled with the composition rule and finding `kind`
each one exercises:

| Node | Verdict | Rule (§3.2) | `kind` (§4.2) |
|---|---|---|---|
| `status` | `no` | leaf, exact class, literal-set containment | `wider-values` |
| `items` cardinality | `no` | `each-like` child: interval containment | `wider-cardinality` |
| `items[*].qty` | `no` | leaf, exact class, kind lattice | `broader-type` |
| `items[*].backordered` | `yes` | must-ignore — not a finding | — |
| `payment@card.last4` | `unknown` | leaf, conservative class | `unreviewable` |
| everything else (`id`, `payment@invoice.*`, `shippedAt`) | `yes` | identity floor | — |

`items`' own verdict is the Kleene conjunction of its cardinality child and its `compare(items.items, …)`
child (Rule 1, §3.2): cardinality alone is already `no`, which makes `items` `no` regardless of the
element-wise comparison it also runs — the report still surfaces `items[*].qty` as its own finding
because it is a distinct, shallower-than-nothing-deeper reason, not a restatement of the cardinality one
(§4.1).

## 3. The report

```json report
{ "$format": "janus-subsumption-report/1",
  "consumer": { "name": "order-consumer" },
  "provider": { "name": "orders-api" },
  "interactions": [
    { "description": "get an order",
      "states": [ { "name": "an order exists" } ],
      "matched": true,
      "verdict": "no",
      "findings": [
        { "path": "response.body.status", "verdict": "no", "severity": "finding", "kind": "wider-values",
          "provider": { "summary": "one of 'PENDING' | 'SHIPPED' | 'DELIVERED' | 'CANCELLED'" },
          "consumer": { "summary": "one of 'PENDING' | 'SHIPPED' | 'DELIVERED'" },
          "reason": "4 options vs 3; finite sets, exact" },
        { "path": "response.body.items", "verdict": "no", "severity": "finding", "kind": "wider-cardinality",
          "provider": { "summary": "0 to unbounded elements" },
          "consumer": { "summary": "1 to unbounded elements" },
          "reason": "the empty array is admitted by the provider, not by the consumer" },
        { "path": "response.body.items[*].qty", "verdict": "no", "severity": "finding", "kind": "broader-type",
          "provider": { "summary": "any number" },
          "consumer": { "summary": "a whole number" },
          "reason": "kind lattice: integer subset of number, exact" },
        { "path": "response.body.payment@card.last4", "verdict": "unknown", "severity": "review",
          "kind": "unreviewable",
          "provider": { "summary": "strings matching '[0-9]{4}'" },
          "consumer": { "summary": "strings matching '\\d{4}'" },
          "reason": "two different regexes: conservative class, no guessing" } ] } ],
  "summary": { "interactions": 1, "matched": 1, "findings": 3, "reviews": 1 } }
```

Three decided findings, one honest review, no false confidence — the same count shape-language's example
§7 already promised in prose; this is that promise as the artifact a policy (spec.md §7) actually reads.

## 4. The text rendering

```
✗ order-consumer is not compatible with orders-api
  interaction 'get an order', response body $.status:
    provider may produce: 'PENDING' | 'SHIPPED' | 'DELIVERED' | 'CANCELLED'
    consumer has only tested: 'PENDING' | 'SHIPPED' | 'DELIVERED'
  interaction 'get an order', response body $.items:
    provider may produce an empty list
    consumer has only tested at least one item
  interaction 'get an order', response body $.items[*].qty:
    provider may produce any number
    consumer has only tested a whole number
  ? interaction 'get an order', response body $.payment.last4 (card):
    provider pattern '[0-9]{4}' cannot be compared against consumer pattern '\d{4}' — review manually
```

The `?`-prefixed line is a `review`-severity finding, rendered distinctly from the three decided `✗`
lines (spec.md §6.4) — task 7.4's combined `can-i-deploy` page reuses this block verbatim alongside
verification-result lines.

## 5. Two comparisons the order payload does not exercise

**A member the provider's shape does not name (Rule 2, §3.2).** Suppose the consumer additionally
requires a top-level `region` member:

```json sketch
{ "shape": "object",
  "members": { "region": { "shape": "any-of", "options": ["AU", "NZ"], "example": "AU" } } }
```

and the provider's published shape's `members` has no `region` key at all — not `any`, not `optional`,
simply absent. `object`'s `admits` only checks named members (shape spec §4.3), so the provider's shape
places no constraint there: it admits a `region` of any value, or none. That is wider than
`any-of("AU", "NZ")`, so the verdict is `no`:

```json finding
{ "path": "response.body.region", "verdict": "no", "severity": "finding", "kind": "undeclared-member",
  "provider": { "summary": "unconstrained: any value, or absent" },
  "consumer": { "summary": "one of 'AU' | 'NZ'" },
  "reason": "the provider's shape does not name this member; an unnamed member is the widest possible claim, not a narrow one (spec.md §3.2 Rule 2)" }
```

The fix is not to leave the field out — that is already the widest thing a shape can say — but to name
it, with `any` if nothing narrower is yet known, or with the real enum once it is.

**Weaker presence.** Suppose instead the consumer's shape has `shippedAt` as a plain
`{ "shape": "datetime", … }` — always present, never absent — because every variant the consumer's test
happened to pin left it present. The provider's `optional(datetime(…))` (§1 above) admits `⊥`, which the
consumer's plain `datetime` does not:

```json finding
{ "path": "response.body.shippedAt", "verdict": "no", "severity": "finding", "kind": "weaker-presence",
  "provider": { "summary": "a datetime, or absent" },
  "consumer": { "summary": "a datetime, always present" },
  "reason": "the provider admits absence (optional); the consumer's shape does not" }
```

This is the RFC's own "nullable column" scenario, decided by ordinary set containment on `⊥` rather than
a bespoke presence rule (shape spec §8) — and, as it happens, the order payload's *actual* consumer shape
already declares `shippedAt` as `optional` (shape-language example §2), which is exactly why this
comparison does not appear in §3 above: the consumer already tests the width the provider needs.
