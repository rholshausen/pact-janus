# Worked example — a third-party CSV content component

Companion to [`../spec.md`](../spec.md). Where the
[HTTP transport example](http-transport-both-bindings.md) tests the in-tree half of
[ADR 0012](../../../decisions/0012-one-interface-two-bindings.md), this one walks the out-of-tree half:
a component nobody in this repo wrote, declared in a project's config, fetched by digest, sandboxed,
and contributing a content type, an action, an operator, a variant dimension and a comparability
procedure. It is deliberately the shape of plan task 8.1, so that task starts from a document rather
than from a conversation.

CSV is a good subject precisely because it is *lossy*: it has no numeric type, no null, no nesting.
A content component that pretends otherwise makes contracts that read stronger than they are, and §6.4
is the machinery that stops it.

## 1. Declaring it

```json component-config
{ "components": [
    { "name": "csv",
      "source": { "kind": "oci",
                  "reference": "ghcr.io/pact-foundation/janus-csv:1.2.0",
                  "digest": "sha256:9f2c1de4a1b0c37e5f6a2d8b41c09e7a3f5b6c8d9e0a1b2c3d4e5f60718293a4" },
      "grants": { "env": [], "network": false },
      "limits": { "deadline-ms": 5000, "instances": "per-session" } } ] }
```

Everything a reviewer needs is on this page: which component, which bytes (the digest, not the tag),
what it may touch (nothing), and how long it may take. The engine verifies the digest **before**
instantiating anything (§10.3), and a component whose imports exceed its grants fails to load rather
than being denied at first use (§9.2) — a failure at load names the component, a failure at first use
names a body that would not parse.

## 2. The handshake

```json component-hello
{
  "component-protocol-versions": [1],
  "engine": { "name": "janus-engine", "version": "0.1.0" },
  "grants": { "env": [], "network": false },
  "capabilities": { }
}
```

```json component-hello-result
{
  "component-protocol-version": 1,
  "component": { "name": "csv", "version": "1.2.0" },
  "interfaces": ["content", "matcher"],
  "contributes": {
    "content-types": [
      { "media-type": "text/csv",
        "degradations": [
          { "code": "numeric-lexical",
            "message": "CSV carries no types: every field decodes as a string, so integer, decimal and boolean are indistinguishable from the text that spells them." },
          { "code": "no-null",
            "message": "An empty field decodes as the empty string; CSV cannot distinguish it from an absent value." } ] } ],
    "actions": [ { "name": "csv:parse" } ],
    "operators": [
      { "name": "csv:columns", "comparability": "exact" },
      { "name": "csv:header-style", "comparability": "exact", "variant-facet": "header-style" } ],
    "generators": [ { "name": "csv:row-id", "produces": "a unique row identifier per run" } ]
  },
  "capabilities": { "batch-apply": { } }
}
```

Every contributed name is namespaced `csv:` — the component's own name, checked at load (§2.4). A
component declaring `column` unnamespaced, or `xml:column`, fails to load here and not later; the
namespace is a partition the engine enforces, not a convention authors are asked to respect.

The two degradations are the honest part. `numeric-lexical` is the case shape spec §4.2 names, and
declaring it is what lets `explain` tell a user that `{ "shape": "integer" }` over a CSV column is
checking the *spelling* of an integer, which is all CSV can offer.

## 3. Decoding

```json decode
{ "content-type": "text/csv",
  "value": { "content": "aWQsc3RhdHVzLHRvdGFsCm8tMSxzaGlwcGVkLDEyLjUwCm8tMixuZXcsMC4wMAo=",
             "encoded": "base64", "content-type": "text/csv" } }
```

```json decode-result
{ "document": [ { "id": "o-1", "status": "shipped", "total": "12.50" },
                { "id": "o-2", "status": "new",     "total": "0.00" } ],
  "degradations": [
    { "code": "numeric-lexical", "path": "$[*].total",
      "message": "column 'total' decoded as strings; CSV has no numeric type" } ] }
```

Octets in, document out — and the shape applies to *that* document, never to the octets and never to a
guess (§6.1). The per-decode degradation names the column, which the static declaration cannot: the
static one says the type is lossy, this one says where the loss actually bit.

`encode` is required to invert this (§6.2), and that requirement has teeth here: a component that
decodes `"0.00"` to a string but encodes numbers back as `0` would record contract examples its own
matcher rejects.

## 4. The plan fragment it contributes

`content/compile` is offered the body slot's shape and returns a fragment the kernel splices in, so the
component's decoding is visible in `explain` rather than hidden inside a match:

```json compile-result
{ "grammar-version": "v0",
  "fragment": {
    "kind": "pipeline",
    "children": [
      { "kind": "resolve", "path": "$.response.body" },
      { "kind": "action", "name": "csv:parse",
        "children": [ { "kind": "value", "value": { "of": "object",
                                                    "value": { "header": "present", "delimiter": "," } } } ] } ] } }
```

which renders as:

```text
(
  $.response.body
  | %csv:parse (
      {"header": "present", "delimiter": ","}
    )
)
```

The fragment uses one core node kind and one namespaced action, which is the whole permitted alphabet
(§6.3). It declares the grammar version it targets: an engine on a later grammar accepts it if the
grammar grew additively and fails naming the skew if it did not, and never silently reinterprets it
(§12.3). Task 8.4 is that scenario run deliberately.

## 5. The operator, compiled and executed

```json matcher-compile
{ "operator": "csv:columns", "path": "$.response.body",
  "node": { "shape": "csv:columns", "columns": ["id", "status", "total"], "example": "id,status,total" } }
```

```json matcher-apply
{ "action": "csv:columns",
  "config": { "columns": ["id", "status", "total"] },
  "values": [ { "content": ["id", "status", "total"], "path": "$.response.body" },
              { "content": ["id", "status"], "path": "$.response.body" } ] }
```

```json matcher-apply-result
{ "results": [
    { "status": "ok" },
    { "status": "error", "path": "$.response.body",
      "message": "expected columns [id, status, total], got [id, status]" } ] }
```

Two values in one call, because the component declared `batch-apply` (§7.2). The failure comes back as
a **plan result**, not as an error frame: a component mismatch is a mismatch, reaching the user through
the same executed plan a kernel mismatch does. The distinction the interface insists on is between "the
value is wrong" (a result) and "I could not do my job" (an error), and only the second one is a
`ComponentError`:

```json component-error
{ "code": "decode-failed", "category": "document",
  "message": "unterminated quoted field on line 3",
  "details": { "content-type": "text/csv", "path": "$.response.body", "line": 3 } }
```

## 6. A contributed variant dimension

`csv:header-style` declares a facet, so the operator widens the variant space rather than just
asserting against it:

```json variant-space-result
{ "dimensions": [
    { "id": "$.response.body#header-style",
      "path": "$.response.body",
      "facet": "header-style",
      "operator": "csv:header-style",
      "points": [ { "name": "absent" }, { "name": "present" } ],
      "default": "present" } ] }
```

Point order is the declaration (§7.4): `absent` first makes it the minimal point and `present` the
maximal, so the boundary variants pick them up with no extra member and no special case in the sampler.
Variant semantics §3.1 asks a component facet to declare its extremes; an ordered list is that
declaration.

## 7. Comparability

The operator declared `exact`, so it owes a containment procedure (§7.5):

```json matcher-compare
{ "operator": "csv:columns",
  "provider": { "shape": "csv:columns", "columns": ["id", "status", "total", "carrier"] },
  "consumer": { "shape": "csv:columns", "columns": ["id", "status", "total"] } }
```

```json matcher-compare-result
{ "verdict": "yes",
  "reason": "provider's columns are a superset; extra columns are ignored unless forbidden" }
```

And the answer a component is most tempted to get wrong:

```json matcher-compare-result
{ "verdict": "unknown",
  "reason": "column order differs and this component cannot decide whether the consumer's reader is positional" }
```

`unknown` is a first-class answer (shape spec §8). A component that guessed `yes` here would be the
reason a team stopped trusting `can-i-deploy`, and no amount of correctness elsewhere buys that back.

## 8. When it cannot be loaded

The contract says what it needs:

```json sketch
{ "requires": [ { "component": "content/csv", "min-version": 1 } ] }
```

On an engine embedded as a WASM component — which can host nothing but in-tree components
([ADR 0013](../../../decisions/0013-component-hosting-is-an-embedding-capability.md)) — the run stops
before it starts, at the union check of §2.3:

```json engine-error
{ "code": "component-unavailable", "category": "component",
  "message": "no loader for component 'csv': this embedding loads in-tree components only",
  "details": { "component": "content/csv", "min-version": 1, "loaders": ["in-tree"] } }
```

The host could have known this from the handshake:

```json capability
{ "components": { "loaders": ["in-tree"] } }
```

That is the whole value of making hosting a capability rather than a surprise: the failure names what
is missing, what is present, and which interaction needed it — and it happens at interaction 0, not at
interaction 40 of 50.
