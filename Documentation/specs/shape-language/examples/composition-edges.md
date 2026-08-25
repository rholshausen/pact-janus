# Worked example — composition edges

The order payload exercises the common path. This file covers the cases where authors, converters and
the engine most often disagree: presence versus nullability, maps versus objects, the three array
operators, bytes bodies, gated dimensions, and the well-formedness rules — each with the error an
engine must produce when it is broken.

Fence markers as in [`order-payload.md`](order-payload.md): `shape` blocks validate against
[`shape.schema.json`](../schemas/v1/shape.schema.json), `variants` blocks against
[`variant-space.schema.json`](../schemas/v1/variant-space.schema.json), and `value` blocks are
payloads, not shapes.

## 1. Absent, null, or both

Three declarations that today's matching rules cannot tell apart:

```json shape
{ "shape": "object",
  "members": {
    "a": { "shape": "optional", "of": { "shape": "string", "example": "x" } },
    "b": { "shape": "nullable", "of": { "shape": "string", "example": "x" } },
    "c": { "shape": "optional", "of": { "shape": "nullable",
                                        "of": { "shape": "string", "example": "x" } } } } }
```

| Member | `admits` | Dimensions |
|---|---|---|
| `a` | strings, `⊥` | `body.a#presence` |
| `b` | strings, `null` | `body.b#nullability` |
| `c` | strings, `null`, `⊥` | `body.c#presence`, `body.c#nullability` (gated by `present`) |

```json value
{ "b": null, "c": null }
```

Admitted: `a` is absent, which `optional` allows; `b` is null, which `nullable` allows; `c` is present
and null, which the stack allows. Swap the values — `{ "a": null, "b": null }` — and `a` fails, because
`optional` never learned to admit `null`.

`c`'s two dimensions and the gate between them:

```json variants
{ "dimensions": [
  { "id": "body.c#presence", "path": "body.c", "facet": "presence",
    "operator": "optional", "default": "present",
    "points": [ { "name": "present" }, { "name": "absent" } ] },
  { "id": "body.c#nullability", "path": "body.c", "facet": "nullability",
    "operator": "nullable", "default": "non-null",
    "points": [ { "name": "non-null" }, { "name": "null" } ],
    "gated-by": [ { "dimension": "body.c#presence", "point": "present" } ] } ] }
```

Three variants, not four: when `c` is absent there is no nullability to choose (spec §6.3). A sampler
that multiplied the dimensions would try to serve an "absent and null" response, which is not a value.

`nullable(optional(S))` is rejected outright (spec §5.2) — one spelling per meaning, so that
`body.c#presence` means the same thing in every pact file that has one.

## 2. An object with named members, or a map of anything

```json shape
{ "shape": "object",
  "members": {
    "id": { "shape": "integer", "example": 42 },
    "labels": { "shape": "each-entry",
                "min": 1,
                "keys": { "shape": "regex", "pattern": "^[a-z.]+$", "example": "env" },
                "values": { "shape": "string", "example": "prod" } } } }
```

```json value
{ "id": 42, "labels": { "env": "prod", "tier": "gold" }, "audit": { "by": "svc" } }
```

Admitted. `audit` is an unnamed member of the outer object, so it is ignored; both members of `labels`
are checked, because `each-entry` speaks about every entry (spec §4.3). `{ "labels": { "Env": "prod" } }`
fails on the key shape, and `{ "labels": {} }` fails on `min`.

That is the whole difference between the two operators: `object` names the members it has an opinion
about and ignores the rest; `each-entry` has an opinion about all of them and names none.

## 3. Three ways to say "array"

```json shape
{ "shape": "object",
  "members": {
    "coordinates": { "shape": "array",
                     "entries": [ { "shape": "decimal", "example": -36.85 },
                                  { "shape": "decimal", "example": 174.76 } ] },
    "items": { "shape": "each-like",
               "min": 1, "max": 3,
               "items": { "shape": "string", "example": "SKU-1" } },
    "audit": { "shape": "contains",
               "entries": [ { "shape": "object",
                              "members": { "event": { "shape": "equality", "example": "created" } } } ] } } }
```

| Operator | Length | Extra elements | Dimension |
|---|---|---|---|
| `array` | exactly 2 | rejected — position is the address (spec §4.3) | — |
| `each-like` | 1 to 3 | n/a — every element is checked | `cardinality`: `min` (1), `min+1` (2), `max` (3) |
| `contains` | any | ignored — that is the operator's purpose | — |

```json value
{ "coordinates": [-36.85, 174.76],
  "items": ["SKU-1", "SKU-9"],
  "audit": [ { "event": "queued" }, { "event": "created", "at": "2026-08-25T00:00:00Z" } ] }
```

Admitted. `audit`'s single entry shape found a distinct element that admits it; the other element and
the extra `at` member are ignored. Note the price: `contains` is the one core operator whose
subsumption is opaque (spec §8), so a provider shape using it can only ever be compared to an
identical consumer shape. Prefer `each-like` when authoring fresh; `contains` is here so that v3/v4
`arrayContains` survives conversion.

A finite `max` gives the cardinality dimension a third point, and with it a promise to exercise the
upper bound:

```json variants
{ "dimensions": [
  { "id": "body.items#cardinality", "path": "body.items", "facet": "cardinality",
    "operator": "each-like", "default": "min",
    "points": [ { "name": "min", "size": 1 },
                { "name": "min+1", "size": 2 },
                { "name": "max", "size": 3 } ] } ] }
```

## 4. Bodies that are octets

A shape over content the engine cannot decode works on the octet sequence, and its example is written
in the protocol's tagged form (spec §2.2, protocol spec §2.5):

```json shape
{ "shape": "content-type",
  "content-type": "image/png",
  "example": "iVBORw0KGgoAAAANSUhEUg==",
  "encoded": "base64" }
```

The `encoded` tag is what makes this lossless in both directions, and it is why a contract test can
assert on a payload that is *deliberately* malformed — the case protocol §2.4 says decides the
document model. A shape that addresses into structure (`object`, `each-like`, …) cannot be used here:
without a content component to decode the octets there is nothing to address into, and the engine
reports a component error rather than applying the shape to a guess.

## 5. Gated dimensions under a `one-of`

```json shape
{ "shape": "one-of",
  "discriminator": "kind",
  "default": "email",
  "alternatives": {
    "email": { "shape": "object",
               "members": {
                 "kind": { "shape": "equality", "example": "email" },
                 "address": { "shape": "regex", "pattern": "@", "example": "a@example.com" },
                 "verified": { "shape": "optional", "of": { "shape": "boolean", "example": true } } } },
    "sms": { "shape": "object",
             "members": {
               "kind": { "shape": "equality", "example": "sms" },
               "number": { "shape": "regex", "pattern": "^\\+", "example": "+6421000000" } } } } }
```

```json variants
{ "dimensions": [
  { "id": "body#alternative", "path": "body", "facet": "alternative",
    "operator": "one-of", "default": "email",
    "points": [ { "name": "email" }, { "name": "sms" } ] },
  { "id": "body@email.verified#presence", "path": "body@email.verified", "facet": "presence",
    "operator": "optional", "default": "present",
    "points": [ { "name": "present" }, { "name": "absent" } ],
    "gated-by": [ { "dimension": "body#alternative", "point": "email" } ] } ] }
```

Three variants: `email` with `verified`, `email` without, and `sms`. The `@email` segment in the
dimension id is what keeps the presence dimension attributable to one alternative — two alternatives
with a member of the same name get different ids, and a pinned variant still means one thing after a
round trip through a pact file (spec §6.2).

## 6. Ill-formed shapes, and what the engine says

Each of these is rejected before compilation, as `interaction-invalid` with a `problems[]` entry
naming the path (spec §5, protocol spec §10.2). Every one of them is **schema-valid** — and validated
as such by the same CI test as the shapes above. That is the point: the schema fixes the document's
shape, the rules in spec §5 fix what a well-formed shape *means*, and no JSON Schema keyword could
carry the second job. The engine, not the validator, is the authority on well-formedness.

```json shape
{ "shape": "each-like",
  "items": { "shape": "optional", "of": { "shape": "string", "example": "x" } } }
```

`optional` outside a slot (spec §5.1). An array whose second element is "absent" is an array of two
elements, so the declaration has no meaning to give.

```json shape
{ "shape": "nullable",
  "of": { "shape": "optional", "of": { "shape": "string", "example": "x" } } }
```

Presence modifiers stack one way only (spec §5.2). The author means `optional(nullable(string))`.

```json shape
{ "shape": "one-of",
  "discriminator": "type",
  "alternatives": {
    "a": { "shape": "object", "members": { "type": { "shape": "string", "example": "a" } } },
    "b": { "shape": "object", "members": { "type": { "shape": "string", "example": "b" } } } } }
```

The discriminator is a `string` matcher, not a literal, so both alternatives admit every value of
`type` and the union is untagged (spec §5.4). Reported at authoring time, which is where an ambiguous
union is cheap to fix.

```json shape
{ "shape": "any-of", "options": ["PENDING", "SHIPPED"], "example": "DELIVERED" }
```

The example is not among the options (spec §5.3) — the default point would name a value the shape does
not admit.

```json shape
{ "shape": "object",
  "members": { "total": { "shape": "sum-of", "example": 42 } } }
```

Unknown unnamespaced operator: `interaction-invalid`, naming `sum-of` and the path (spec §3.7). Not
ignored — dropping an operator would weaken the contract while still reporting success. Had it been
`stats:sum-of`, the answer would be `component-unavailable` naming the missing component instead.
