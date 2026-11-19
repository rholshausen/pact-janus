# Worked example — a third-party CSV content component

Companion to [`../spec.md`](../spec.md). Where the
[HTTP transport example](http-transport-both-bindings.md) tests the in-tree half of
[ADR 0012](../../../decisions/0012-one-interface-two-bindings.md), this one walks the out-of-tree half:
a component nobody in this repo wrote, declared in a project's config, fetched by digest, sandboxed,
and contributing a content type, what value operators mean for it, an operator, a variant dimension
and a comparability procedure. It is deliberately the shape of plan task 8.1, so that task starts from a document rather
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
        "substitutions": { "integer": "csv:integer", "number": "csv:number", "decimal": "csv:number",
                           "boolean": "csv:boolean", "equality": "csv:equality" },
        "degradations": [
          { "code": "numeric-lexical",
            "message": "CSV carries no types: every field decodes as a string, so integer, decimal and boolean are indistinguishable from the text that spells them." },
          { "code": "no-null",
            "message": "An empty field decodes as the empty string; CSV cannot distinguish it from an absent value." } ] } ],
    "actions": [ { "name": "csv:integer" }, { "name": "csv:number" }, { "name": "csv:boolean" },
                 { "name": "csv:equality" } ],
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

The two degradations are the honest part. `numeric-lexical` is the case shape spec §4.2 names: the
column decodes to strings (§3 below), so an integer and the text that spells one cannot be told
apart. The `substitutions` are what the component does about it: under `text/csv`, `integer` is
`csv:integer`, text that spells an integer, and so on for the other operators whose meaning CSV
changes (§4 below). Without them `{ "shape": "integer" }` over a CSV column would reject every value,
as it did when plan task 8.1 first built this component from this page
([third-party component report](../../../prototype/third-party-component-report.md), finding 4).

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

## 4. What a value operator means for CSV

CSV has no types, so a CSV body decodes to strings: `items` is `"1"`, never `1`. A consumer who writes
the shape they mean — `items: integer` — would get the engine's `match:integer`, and a string is not an
integer. So the handshake's `substitutions` say what `integer` means for this content type (§6.3):
`csv:integer`, run by `matcher/apply`. The engine compiles the body slot as it compiles any other,
every structure and every variant's pinning its own, and emits `csv:integer` where it would emit
`match:integer`. The maximal variant of an `each-like` of 1–3 orders renders as:

```text
:body (
  #{'decoded as text/csv by csv: boolean is csv:boolean, decimal is csv:number, equality is csv:equality, integer is csv:integer, number is csv:number'},
  %expect:array ( $.response.body ),
  %expect:count ( $.response.body, 3 ),
  %flow:for-each (
    ** ( $.response.body ),
    :"$.response.body[*]" (
      %expect:object ( ~> ),
      :"$.response.body[*].id" ( %match:string ( ~>.id ) ),
      :"$.response.body[*].items" ( %csv:integer ( ~>.items ) )
    )
  )
)
```

Every operator CSV does not change is the core action the engine would have used anyway, `match:string`
here. The annotation says why `csv:integer` is where `match:integer` would be. `csv:equality` is given
`equality`'s example as its argument and compares it as the text a field holds, so `66` matches `"66"`
(§7.2).

`content/compile` is still asked for the slot's shape when the interaction arrives. This component can
carry any shape a CSV document decodes to, so it answers with success, which is all an answer can be:

```json compile-result
{ }
```

A component that cannot carry a shape answers with an error instead, as the built-in `text` component
does for rows of objects, and the interaction is refused as `compile-failed`.

An earlier version of this example had the component return a **plan fragment** that replaced the
slot's plan. That made the component a shape compiler, and since it was given no variant, its
`expect:size 1..3` checked every variant, so the three-row one passed against a provider with one row
(Phase 9 finding 21). Fragments are withdrawn
([ADR 0100](../../../decisions/0100-a-content-component-says-what-a-value-operator-means-for-its-type-and-the-engine-compiles-every-slot.md)).

## 5. The operator, compiled and executed

A shape that uses the component's operator, `{ "shape": "csv:columns", "columns": ["id", "status",
"total"] }`, compiles to the action of the same name, with the value and the operator's members as
its two children (§7.1). The members arrive as the action's one argument:

```json matcher-apply
{ "action": "csv:columns",
  "config": { "arguments": [ { "columns": ["id", "status", "total"] } ] },
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
