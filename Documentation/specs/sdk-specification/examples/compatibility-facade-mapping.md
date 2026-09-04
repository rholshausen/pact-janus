# Worked example — classifying today's classic DSL against the facade vocabulary

Spec.md §6.1 fixes three honest outcomes for a classic-DSL primitive — `kept`, `adapted`, `dropped` — and
the discipline that a `kept` name must not gain new behaviour. This file works a representative slice of
today's classic consumer DSL (the `like`/`eachLike`/`term`-family matchers most SDKs share, in spirit if
not in exact spelling) against that vocabulary, so tasks 6.2/6.3 have a concrete starting table for the
facade module rather than a blank page.

Fence marker: ```` ```json primitive ```` validates against
[`behavioural-spec.schema.json`](../schemas/v1/behavioural-spec.schema.json)'s `Primitive` definition, the
same as [order-example-mapping.md](order-example-mapping.md).

## 1. `like` has two faces, and only one maps cleanly

Classic `like(v)` means two different things depending on what `v` is, and the facade has to say so
rather than pick one and hope:

- **`like(scalar)`** — "accept any value of the same kind as this example" — is exactly what the new
  `type` operator says (shape spec §4.2), with nothing lost: `kept`.
- **`like(objectLiteral)` or `like(arrayLiteral)`** relied on the old matcher's *recursive* descent into
  members — "accept any object shaped like this, recursively" — which shape spec §4.2 calls out by name
  as "a deliberate narrowing" the new `type` operator does not do, precisely because that recursive
  behaviour was "the single largest source of 'why did this match?' questions." There is no honest
  `type`-based translation for the composite case: `dropped`, migrating to writing the structure
  explicitly with `object`/`each-like` (which the DSL's own object-literal-to-shape mapping already does
  for free — see [order-example-mapping.md](order-example-mapping.md) §2 — so the migration is usually
  deleting the `like(...)` wrapper, not writing new code).

```json primitive
{ "id": "type",
  "category": "shape",
  "summary": "Accepts any value of the same kind as the example, non-recursively.",
  "produces": [ { "kind": "shape-operator", "operator": "type" } ],
  "signature": [ { "name": "example", "role": "example-value" } ],
  "semantics": "Compiles to a shape-language 'type' node (shape spec §4.2). On a composite example, constrains the kind only ('object', 'array') and does not recurse into members — structure is declared with 'object'/'each-like', never inferred from a type example. Contributes no variant dimension.",
  "errors": [], "conformance": [ "shape.type.kind-only", "shape.type.no-recursion" ],
  "facade": [
    { "from": "like(scalar)", "status": "kept",
      "reason": "identical behaviour: accept any value of the example's kind, no recursion involved because there is nothing to recurse into." },
    { "from": "like(objectLiteral) or like(arrayLiteral)", "status": "dropped",
      "reason": "relied on old type's recursive descent into members (shape spec §4.2's own 'deliberate narrowing'), which the new type operator does not do and shapes do not restore elsewhere. Migrates to declaring the structure directly with object/each-like — usually deleting the like(...) wrapper entirely, since a plain object literal already compiles to an object shape (order-example-mapping.md §2)." } ] }
```

## 2. Kind matchers that already map cleanly

```json primitive
{ "id": "boolean",
  "category": "shape",
  "summary": "A boolean value.",
  "produces": [ { "kind": "shape-operator", "operator": "boolean" } ],
  "signature": [ { "name": "example", "role": "example-value" } ],
  "semantics": "Compiles to a shape-language 'boolean' kind predicate (shape spec §4.2). Contributes no variant dimension.",
  "errors": [], "conformance": [ "shape.boolean.kind-predicate" ],
  "facade": [ { "from": "boolean(v)", "status": "kept",
                "reason": "identical: any boolean, no width or precision claim either version made." } ] }
```

```json primitive
{ "id": "decimal",
  "category": "shape",
  "summary": "A number written with a fractional part.",
  "produces": [ { "kind": "shape-operator", "operator": "decimal" } ],
  "signature": [ { "name": "example", "role": "example-value" } ],
  "semantics": "Compiles to a shape-language 'decimal' kind predicate (shape spec §4.2). Where a content component cannot distinguish lexical forms, this degrades to 'number' at the component boundary (shape spec §4.2) — the SDK still emits 'decimal'; the degradation is the content component's business, not the DSL's.",
  "errors": [], "conformance": [ "shape.decimal.kind-predicate" ],
  "facade": [ { "from": "decimal(v)", "status": "kept",
                "reason": "identical claim: a number written with a fractional part." } ] }
```

## 3. A matcher that needs a supplied default: `uuid`

There is no dedicated `uuid` operator in the shape language's core vocabulary (shape spec §4.1's operator
table) — a project-specific format did not earn a core slot the way `datetime`/`date`/`time` did, since
those already existed in v1-v4. `uuid()` therefore needs `regex` plus a stated default pattern to keep
meaning the same thing it always did:

```json primitive
{ "id": "regex",
  "category": "shape",
  "summary": "A string matching a pattern.",
  "produces": [ { "kind": "shape-operator", "operator": "regex" } ],
  "signature": [ { "name": "pattern", "role": "label" }, { "name": "example", "role": "example-value" } ],
  "semantics": "See order-example-mapping.md §3.2 for this primitive's own semantics — repeated here only as the anchor for the facade entry below, per spec.md §6.2's pattern of attaching a facade mapping to the primitive it targets.",
  "errors": [], "conformance": [ "shape.regex.unanchored" ],
  "facade": [
    { "from": "term(generate, matcher)", "status": "kept",
      "reason": "term paired one example value with one regex matcher, which is exactly regex(pattern, example)'s own shape — a rename, not a behaviour change." },
    { "from": "uuid(v)", "status": "adapted",
      "reason": "adapts to regex(pattern, v) with a supplied default pattern for RFC 4122 (any version, any variant) UUID syntax, case-insensitive hex. Stated here because the old primitive never exposed a pattern for a caller to check, so the default is now a fact this document, not the call site, is responsible for." } ] }
```

## 4. A matcher that keeps its meaning by name change alone

```json primitive
{ "id": "contains",
  "category": "shape",
  "summary": "An array containing a distinct match for each of the given entry shapes; other elements ignored.",
  "produces": [ { "kind": "shape-operator", "operator": "contains" } ],
  "signature": [ { "name": "entries", "role": "options-bag" } ],
  "semantics": "Compiles to a shape-language 'contains' node (shape spec §4.1), kept specifically so that v3/v4 arrayContains-style declarations survive conversion (shape spec §4.3). Subsumption for this operator is opaque (shape spec §8) — a provider shape using it can only ever be compared to an identical consumer shape (shape-language spec example composition-edges.md §3) — which this primitive's documentation SHOULD surface to a user reaching for it fresh, steering them to each-like where the elements are otherwise alike.",
  "errors": [], "conformance": [ "shape.contains.distinct-match-per-entry" ],
  "facade": [ { "from": "arrayContainingMatcher(entries) / an eachLike used only to assert \"somewhere in this array\"",
                "status": "kept",
                "reason": "identical intent — a distinct element admitting each named entry shape, everything else ignored — under a name that now matches what the operator actually does rather than overloading eachLike's cardinality-driven name for a different purpose." } ] }
```

## 5. What has no per-primitive facade entry at all

The raw matching-rule-path escape hatch — specifying a rule by JSONPath string, independent of the DSL
body's own structure (`matchingRules: { "$.body.foo": { ... } }` and its per-language equivalents) — does
not get a facade entry on any one primitive, because it does not map to one: shape spec §7.1 fixes "no
cascading, no precedence... a path has exactly one shape," so there is no successor primitive whose
`facade` array this belongs in. It is `dropped` at the level of the escape hatch itself, and its migration
path is the same one composite `like` gets in §1: declare the shape structurally, at the position it
belongs, in the DSL body — which is also the only way the new model can compute a variant space for it at
all (shape-language spec §6 derives dimensions from the tree the DSL builds, not from a side-channel rule
map).
