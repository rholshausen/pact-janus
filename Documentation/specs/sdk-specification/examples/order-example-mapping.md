# Worked example — mapping the RFC's consumer test

The RFC's own consumer example — the one CLAUDE.md names as this project's DSL reference surface — call
by call, from DSL text to the behavioural-specification primitive it exercises to what that primitive
produces. This file does not re-derive shape semantics: [shape-language's order-payload
example](../../shape-language/examples/order-payload.md) already carries the same payload from DSL to
canonical shape to variant space, and is the authority for that. What is new here is the layer above it —
which primitive each DSL call *is*, and what it produces structurally (spec.md §3.2) — which is the
question an SDK maintainer, not a shape-language reader, needs answered.

Fence marker: ```` ```json primitive ```` validates against
[`behavioural-spec.schema.json`](../schemas/v1/behavioural-spec.schema.json)'s `Primitive` definition.

## 1. The DSL

```typescript
const getOrder = pact.interaction('get an order')
  .given('an order exists', { id: '42' })
  .request({ method: 'GET', path: '/orders/42' })
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

await pact.execute(getOrder, async (mock, variant) => {
  const client = new OrderClient(mock.url);
  const order = await client.getOrder('42');
  expect(order.lineCount).toBeGreaterThan(0);
});
```

## 2. Call to primitive

| DSL call | Primitive `id` | `produces` |
|---|---|---|
| `pact.interaction('get an order')` | `interaction` | spec-member `description` |
| `.given('an order exists', {...})` | `given` | spec-member `states` |
| `.request({...})` | `request` | spec-member `parts.request` |
| `.response({...})` | `response` | spec-member `parts.response` |
| `integer(42)` | `integer` | shape-operator `integer` |
| `anyOf(...)` | `any-of` | shape-operator `any-of` |
| `optional(...)` | `optional` | shape-operator `optional` |
| `datetime(...)` | `datetime` | shape-operator `datetime` |
| `oneOf('type', {...})` | `one-of` | shape-operator `one-of` |
| `regex(/\d{4}/, '1234')` | `regex` | shape-operator `regex` |
| `date('2026-08-30')` | `date` | shape-operator `date` |
| `eachLike({...}, { min: 1 })` | `each-like` | shape-operator `each-like` |
| `string('SKU-1')` | `string` | shape-operator `string` |
| `pact.execute(interaction, closure)` | `execute` | protocol-operations (§4) |

An object literal (`{ type: 'card', last4: ... }`, the body of `json({...})` itself) needs no primitive of
its own: the DSL's job is sugar, and an object literal maps directly to an `object` shape node with one
member per key, contributing no behaviour beyond that mapping (shape-language example order-payload.md
§2). A literal string or number appearing where a shape is expected — `type: 'card'` inside a `one-of`
alternative — compiles to an `equality` node the same way; neither needs a named DSL primitive because
neither is a DSL call.

## 3. The primitive entries

### 3.1 Interaction and session primitives

```json primitive
{ "id": "interaction",
  "category": "interaction",
  "summary": "Begins a new interaction specification.",
  "produces": [ { "kind": "spec-member", "path": "description" } ],
  "signature": [ { "name": "description", "role": "label" } ],
  "semantics": "Opens a new, empty interaction-spec document (plan task 3.2) with 'description' set. No protocol call happens yet — the document is built up by subsequent chained calls and only submitted at 'execute' (this file §3.3). The returned builder is opaque to the protocol; it exists only in the idiomatic layer.",
  "errors": [],
  "conformance": [ "session.interaction.description-set", "session.interaction.no-call-until-execute" ] }
```

```json primitive
{ "id": "given",
  "category": "interaction",
  "summary": "Declares a provider state this interaction requires.",
  "produces": [ { "kind": "spec-member", "path": "states" } ],
  "signature": [
    { "name": "name", "role": "label" },
    { "name": "params", "role": "options-bag",
      "description": "Literal state parameters. Variant-bound parameters (variant semantics spec §6) are a later addition to this primitive's signature, not yet part of v1's DSL surface." } ],
  "semantics": "Appends one entry to the interaction-spec document's 'states' array: { name, params }. Calling 'given' more than once appends further states rather than replacing the first — an interaction may require several. Contributes nothing to the variant space by itself (variant-bound state parameters, when the DSL grows them, will be signalled through the shape they bind to, not through this primitive).",
  "errors": [],
  "conformance": [ "session.given.appends-state", "session.given.multiple-states" ] }
```

```json primitive
{ "id": "request",
  "category": "interaction",
  "summary": "Declares the request part of an HTTP interaction.",
  "produces": [ { "kind": "spec-member", "path": "parts.request" } ],
  "signature": [ { "name": "parts", "role": "options-bag",
                   "description": "method, path, headers, query and body — each a shape or a plain value the idiomatic layer wraps as 'equality' (this file §2)." } ],
  "semantics": "Populates 'parts.request' on the interaction-spec document. A bare value for a member (e.g. method: 'GET') compiles to an 'equality' shape node over that value, not to a shape the DSL leaves the user to write out — this is what keeps the common case one line (shape-language example order-payload.md §8's own sketch of a request part shows the same equality-by-default treatment). A member given a shape helper directly (e.g. a path built with 'regex') is used as authored.",
  "errors": [ { "code": "interaction-invalid",
                "surfaced-as": "thrown at 'execute' time (this primitive itself makes no protocol call), carrying 'problems' positions rewritten from the interaction-spec path to the DSL call site that produced it, where the idiomatic layer can determine one" } ],
  "conformance": [ "session.request.bare-value-is-equality", "session.request.shape-used-as-authored" ] }
```

```json primitive
{ "id": "response",
  "category": "interaction",
  "summary": "Declares the response part of an HTTP interaction.",
  "produces": [ { "kind": "spec-member", "path": "parts.response" } ],
  "signature": [ { "name": "parts", "role": "options-bag" } ],
  "semantics": "Identical treatment to 'request' (this file), populating 'parts.response' instead. 'status' follows the same bare-value-is-equality rule as any other member.",
  "errors": [ { "code": "interaction-invalid", "surfaced-as": "thrown at 'execute' time, as for 'request'" } ],
  "conformance": [ "session.response.bare-value-is-equality" ] }
```

```json primitive
{ "id": "execute",
  "category": "session",
  "summary": "Submits the interaction, iterates every selected variant against the user's test closure, and finalises the session.",
  "produces": [
    { "kind": "protocol-operation", "operation": "consumer-session/create" },
    { "kind": "protocol-operation", "operation": "consumer-session/add-interaction" },
    { "kind": "protocol-operation", "operation": "consumer-session/start-transport" },
    { "kind": "protocol-operation", "operation": "consumer-session/variants" },
    { "kind": "protocol-operation", "operation": "consumer-session/serve-variant" },
    { "kind": "protocol-operation", "operation": "consumer-session/finalise" } ],
  "signature": [
    { "name": "interaction", "role": "options-bag", "description": "The built interaction-spec document." },
    { "name": "closure", "role": "closure", "description": "Runs once per selected variant, receiving (mock, variant)." } ],
  "semantics": "The full call sequence spec.md §4 traces: 'create' once per session (reused across interactions in the same test file — the idiomatic layer SHOULD open one session per test suite, not one per interaction, so 'given' state and transport setup amortise); 'add-interaction' for this interaction; 'start-transport' for its transport kind if not already started on this session; 'variants' to obtain the engine's selection; then, for each variant in the order returned, 'serve-variant' followed by running 'closure(mock, variant)' — where 'mock' exposes the started transport's endpoint (e.g. 'mock.url' for HTTP) and 'variant' is the descriptor 'variants' returned for that entry, opaque beyond its 'id' (engine-protocol spec §8.2); finally 'finalise', which MUST run even if a variant's closure throws (engine-protocol spec §7.1: sessions are the only resource, and finalise is the only way to release one). A closure that throws fails that variant; 'execute' itself then rejects/fails the test after 'finalise' has still run.",
  "errors": [
    { "code": "interaction-invalid", "surfaced-as": "thrown before any variant runs, from add-interaction" },
    { "code": "variant-budget-exceeded", "surfaced-as": "thrown before any variant runs, from variants" },
    { "code": "session-not-found", "surfaced-as": "an idiomatic-layer bug if it ever surfaces to a user — the SDK owns the session handle's lifetime end to end" } ],
  "conformance": [
    "session.execute.full-call-sequence", "session.execute.finalise-always-runs",
    "session.execute.closure-per-selected-variant", "session.execute.failing-variant-fails-build" ] }
```

### 3.2 Shape-helper primitives

Each maps 1:1 to shape-language's own operator table (shape spec §4.1); the DSL contributes only the
mapping, never matching logic (shape-language example order-payload.md §2).

```json primitive
{ "id": "integer",
  "category": "shape",
  "summary": "A whole-number value.",
  "produces": [ { "kind": "shape-operator", "operator": "integer" } ],
  "signature": [ { "name": "example", "role": "example-value" } ],
  "semantics": "Compiles to a shape-language 'integer' kind predicate (shape spec §4.2) with 'example' as its example. Contributes no variant dimension.",
  "errors": [], "conformance": [ "shape.integer.kind-predicate" ] }
```

```json primitive
{ "id": "string",
  "category": "shape",
  "summary": "A string value.",
  "produces": [ { "kind": "shape-operator", "operator": "string" } ],
  "signature": [ { "name": "example", "role": "example-value" } ],
  "semantics": "Compiles to a shape-language 'string' kind predicate (shape spec §4.2) with 'example' as its example. Contributes no variant dimension.",
  "errors": [], "conformance": [ "shape.string.kind-predicate" ] }
```

```json primitive
{ "id": "datetime",
  "category": "shape",
  "summary": "A string parsing as a datetime under a format.",
  "produces": [ { "kind": "shape-operator", "operator": "datetime" } ],
  "signature": [
    { "name": "example", "role": "example-value" },
    { "name": "format", "role": "label",
      "description": "Optional. Omitted means ISO-8601 (shape spec §4.2)." } ],
  "semantics": "Compiles to a shape-language 'datetime' node (shape spec §4.2). When the DSL infers 'format' from 'example' rather than requiring it explicitly, the inference rule MUST be named in the style guide (spec.md §5) as a deviation, because two SDKs inferring differently would silently disagree about what the same DSL call means.",
  "errors": [], "conformance": [ "shape.datetime.format-parses", "shape.datetime.default-is-iso8601" ] }
```

```json primitive
{ "id": "date",
  "category": "shape",
  "summary": "A string parsing as a calendar date under a format.",
  "produces": [ { "kind": "shape-operator", "operator": "date" } ],
  "signature": [ { "name": "example", "role": "example-value" }, { "name": "format", "role": "label" } ],
  "semantics": "Identical treatment to 'datetime' (this file), for the 'date' operator (shape spec §4.2).",
  "errors": [], "conformance": [ "shape.date.format-parses" ] }
```

```json primitive
{ "id": "regex",
  "category": "shape",
  "summary": "A string matching a pattern.",
  "produces": [ { "kind": "shape-operator", "operator": "regex" } ],
  "signature": [
    { "name": "pattern", "role": "label", "description": "RE2-compatible subset (shape spec §4.2)." },
    { "name": "example", "role": "example-value" } ],
  "semantics": "Compiles to a shape-language 'regex' node. Matching is unanchored — a pattern matches if it matches anywhere in the string, per every v1-v4 pact (shape spec §4.2) — so the idiomatic layer MUST NOT silently anchor the pattern on the user's behalf; an author who means a full match writes '^...$' themselves.",
  "errors": [], "conformance": [ "shape.regex.unanchored", "shape.regex.re2-subset" ] }
```

```json primitive
{ "id": "each-like",
  "category": "shape",
  "summary": "An array of like elements, with a cardinality.",
  "produces": [ { "kind": "shape-operator", "operator": "each-like" } ],
  "signature": [
    { "name": "items", "role": "nested-shape" },
    { "name": "options", "role": "options-bag", "description": "{ min, max } — min defaults to 1." } ],
  "semantics": "Compiles to a shape-language 'each-like' node (shape spec §4.1). Contributes one 'cardinality' variant dimension with points 'min' and, when 'max' is finite and greater than 'min', 'max' (shape spec §6.4) — the SDK emits the node; it does not compute the dimension itself (shape spec §6).",
  "errors": [], "conformance": [ "shape.each-like.cardinality-dimension", "shape.each-like.min-default-1" ] }
```

```json primitive
{ "id": "any-of",
  "category": "shape",
  "summary": "Enumerates the literal values a field may take.",
  "produces": [ { "kind": "shape-operator", "operator": "any-of" } ],
  "signature": [ { "name": "options", "role": "example-value-list" } ],
  "semantics": "Compiles to a shape-language 'any-of' node (shape spec §4.1, §5.3) with 'options' as its literal set and the first option as its example. Contributes one 'value' variant dimension, one point per option, in declaration order (shape spec §6.4).",
  "errors": [],
  "conformance": [ "shape.any-of.literal-containment", "shape.any-of.value-dimension" ],
  "facade": [
    { "from": "like(v) with a custom generator function returning one of several fixed literals at random",
      "status": "dropped",
      "reason": "a random per-run generator defeats deterministic recording (ADR 0008's 'no randomness, anywhere' applies just as much to a consumer's own generators as to the sampler) and never declared its alternative set anywhere the engine could see. Migrates to anyOf(...) naming the same literals, now sampled deterministically per variant instead of chosen at random once." } ] }
```

```json primitive
{ "id": "optional",
  "category": "shape",
  "summary": "Marks a value as may-be-absent.",
  "produces": [ { "kind": "shape-operator", "operator": "optional" } ],
  "signature": [ { "name": "of", "role": "nested-shape",
                   "description": "The shape admitted when the value is present." } ],
  "semantics": "Compiles to a shape-language 'optional' node (shape spec §4.4) wrapping the compiled form of 'of', with no further transformation. The DSL contributes no matching logic here: 'of' is compiled by the same rules that would apply if it appeared unwrapped, and 'optional' only adds the wrapper (shape-language example order-payload.md §2's own description of a DSL's job as 'sugar'). Contributes one variant dimension (presence) at this node — the SDK does not compute variant dimensions itself; it emits the node and the engine derives dimensions from it (shape spec §6.4).",
  "errors": [ { "code": "interaction-invalid",
                "surfaced-as": "thrown/raised exception carrying the engine's 'problems' positions verbatim; never a boolean or a silently-skipped assertion" } ],
  "conformance": [ "shape.optional.presence-dimension", "shape.optional.wraps-any-operator" ] }
```

```json primitive
{ "id": "one-of",
  "category": "shape",
  "summary": "A discriminated union of alternatives.",
  "produces": [ { "kind": "shape-operator", "operator": "one-of" } ],
  "signature": [
    { "name": "discriminator", "role": "label" },
    { "name": "alternatives", "role": "options-bag",
      "description": "Name -> object literal; each MUST bind 'discriminator' to a distinct literal (shape spec §5.4)." } ],
  "semantics": "Compiles to a shape-language 'one-of' node with one alternative per key of 'alternatives', each compiled as an object shape whose 'discriminator' member is bound to an 'equality' node over that key's literal binding — the DSL does not require the user to write the discriminator's equality node explicitly; it derives it from which literal the alternative's own object binds at that member (mirroring shape-language example order-payload.md §2's DSL-to-shape mapping). Contributes one 'alternative' variant dimension, one point per alternative, in declaration order (shape spec §6.4). It is a compile-time error, not a runtime one, if two alternatives bind 'discriminator' to the same literal or if an alternative omits it.",
  "errors": [ { "code": "interaction-invalid",
                "surfaced-as": "thrown at 'execute' time naming the duplicate or missing discriminator literal" } ],
  "conformance": [ "shape.one-of.discriminator-binding", "shape.one-of.alternative-dimension" ] }
```

## 4. What `execute` produces, concretely

Filling in `execute`'s trace (this file §3.1, spec.md §4) with this example's actual values:

```
consumer-session/create        { consumer: { name: <this test file's consumer> }, provider: { name: 'orders-api' } }
consumer-session/add-interaction   { session, interaction: <the compiled document, per §2 above> }
consumer-session/start-transport   { session, transport: 'http' }
consumer-session/variants          { session, handle }  -> the engine's selection (variant semantics spec §3)
-- for each selected variant --
consumer-session/serve-variant     { session, handle, variant }
                                    closure(mock, variant) runs; `mock.url` serves the armed variant
consumer-session/finalise          { session }  -> { results, pact? }
```

No step here is a decision the idiomatic layer makes — every one is a direct call with this example's own
values substituted in, which is the whole point of naming primitives structurally (spec.md §3.2): an
implementer can trace this table without needing to have seen it done in another language first.
