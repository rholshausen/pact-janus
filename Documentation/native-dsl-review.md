# The native DSL, for review before it freezes

Stage 3 ends by freezing the native DSL as the major version's surface (staged plan, stage 3's "done
when"). After that, a new primitive or option is a minor change, and changing or removing one is a
major one. S3.2 asks for that surface to be designed with the community. This note is what the review
is asked about: the whole native consumer surface as both SDKs spell it today, what is already
decided and where, and the questions still open. Each open question gives the current answer, so
silence is consent to it.

The surface's meaning is the [behavioural specification](specs/sdk-specification/behavioural-spec.json).
Each language's spelling is its style guide, [TypeScript](../sdks/typescript/STYLE.md) and
[JVM](../sdks/jvm/STYLE.md). The compatibility facades, which spell pact-js's and pact-jvm's DSLs over
it, are outside this review: their contract is to behave as today's DSL does
([consumer-test-migration.md](consumer-test-migration.md)).

## 1. The surface

The RFC's consumer example, as written in TypeScript:

```ts
const getOrder = janus
  .interaction("get an order")
  .given("an order exists", { id: "42", shipped: whenVariant("shippedAt", "present") })
  .exclude([["status", "SHIPPED"], ["shippedAt", "absent"]], "the order service dates a shipped order")
  .request({ method: "GET", path: "/orders/42" })
  .response({
    status: 200,
    body: {
      id: string("42"),
      status: anyOf("PENDING", "SHIPPED", "DELIVERED"),
      shippedAt: optional(datetime("2026-07-30T10:00:00Z")),
      payment: oneOf("type", {
        card: { type: "card", last4: string("4242") },
        invoice: { type: "invoice", due: date("2026-08-30") },
      }),
      items: eachLike({ sku: string("SKU-1"), qty: integer(1) }),
    },
  });

await janus.execute(getOrder, async (mock, variant) => {
  const order = await client(mock.url).getOrder("42");
  expect(render(order)).toBeDefined();
});
```

| Primitive | TypeScript | Java | What it does |
|---|---|---|---|
| `janus` | `new Janus({ consumer, provider, ... })`, `useJanus(...)` | `Janus.of(consumer, provider[, options])`, `JanusExtension` | One suite's session and contract |
| `interaction` | `janus.interaction(d)` | `janus.interaction(d)` | An HTTP interaction |
| `message` | `janus.message(d)` | `janus.message(d)` | A message the provider emits |
| `given` | `.given(name, params?)` | `.given(name[, params])` | A provider state |
| `when-variant`, `variant-cases` | `whenVariant(dim, point)`, `variantCases(dim, cases, default?)` | `VariantBinding.whenVariant(..)`, `.variantCases(..)` | A state parameter that follows the running variant |
| `exclude` | `.exclude([[dim, point], ...], reason)` | `.exclude(List.of(entry(dim, point), ...), reason)` | A combination the provider cannot produce, left out of what is exercised |
| `request`, `response` | `.request({ method, path, query, headers, body })` | `.request(r -> r.method(..).path(..))` | An HTTP exchange's parts |
| `message-contents`, `message-request` | `.contents({ metadata, body })`, `.request(..)` | `.contents(m -> ..)`, `.request(m -> ..)` | A message's parts |
| `execute` | `janus.execute(i, async (mock, variant) => ..)` | `janus.execute(i, (mock, variant) -> ..)` | Runs the test once per selected variant |
| `finalise` | `janus.finalise()`, or the integration | `janus.finalise()`, or the extension | Writes the contract |
| `literal` | a plain value | a plain value | Exactly this value |
| `json`, `content` | `json(doc)`, `content(type, doc)` | `Shapes.json(..)`, `Shapes.content(..)` | A body, and its media type |
| `integer`, `number`, `decimal`, `string`, `boolean` | `integer(1)`, ... `boolean(true)` | `Shapes.integer(1)`, ... `Shapes.bool(true)` | Any value of that kind, with an example |
| `datetime`, `date`, `time`, `regex` | `datetime(ex, format?)`, `regex(re, ex)` | `Shapes.datetime(..)`, `Shapes.regex(..)` | A string of that form |
| `any-of` | `anyOf(a, b, ..)` | `Shapes.anyOf(..)` | One of these values: a variant each |
| `one-of` | `oneOf(discriminator, { name: alt })` | `Shapes.oneOf(..)` | A tagged union: a variant per alternative |
| `optional`, `nullable` | `optional(of)`, `nullable(of)` | `Shapes.optional(..)`, `Shapes.nullable(..)` | May be absent, may be null: a variant each way |
| `forbidden` | `forbidden()` | `Shapes.forbidden()` | Must be absent |
| `each-like` | `eachLike(items, { min?, max? })` | `Shapes.eachLike(items[, Cardinality.min(0)])` | A list of like elements: variants at its bounds |

The provider side (`verifier`, `state`, `requestFilter`, `message`, `verify`) is specified in the same
document and is outside this review, because it carries today's provider APIs through their facades.

## 2. Already decided

Each of these is an accepted ADR. Re-opening one means a superseding ADR, which the review may well
ask for.

- **A list has an element unless the author says otherwise** ([ADR 0079](decisions/0079-keep-each-likes-minimum-at-one-and-exercise-a-list-that-may-be-empty-with-an-element-first.md)).
  `eachLike(items)` is at least one element. `{ min: 0 }` may be empty, and is still exercised with an
  element first. There is one array operator.
- **A combination the provider cannot produce is excluded on the interaction, with a reason** ([ADR 0080](decisions/0080-write-an-exclusion-on-the-interaction-as-its-own-sampling-policy.md)).
  The contract records it, and still admits the combination.
- **A correlation the consumer relies on is a `oneOf`** ([ADR 0081](decisions/0081-write-a-correlation-the-consumer-relies-on-as-a-one-of-and-split-a-provider-object-by-its-discriminator.md)),
  not two independent members and an exclusion.
- **Nothing is inferred from a plain value**: it is matched exactly (the `literal` rule). Kinds,
  patterns and optionality each need their helper.
- **A recorded example is never random** ([ADR 0008](decisions/0008-deterministic-pairwise-variant-sampling.md)).
  A generated value is the engine's, declared on the shape (S3.6, not yet in the DSL).

## 3. Open questions

1. **Is `exclude` the only sampling control an author needs?** The sampling policy also has pins
   (`pin`: "make sure we test the shipped case"), a strategy (exhaustive, pairwise, base only), a
   budget (`max-variants`) and the boundary variants' switch. The DSL spells none of them, on an
   interaction or for a suite (S3.2's decision): any can be added later as a minor change, and none
   can be removed once frozen. A host can still pass a policy on each `variants` call. *Would you
   reach for any of them, and where: on one interaction, or for the whole suite?*
2. **Are `{ min: 0 }` and one array operator right?** pact-js's `eachLike` has a minimum of one and
   pact-jvm's admits an empty list, under one name. ADR 0079 keeps one operator with the minimum
   stated. Its tripwire is consumers routinely writing `{ min: 0 }`.
3. **Should there be a native `like`?** Today's DSLs match a whole object by type with one `like`.
   Natively, each value says its kind (`string("42")`), and a plain value is exact. The shape
   language's `type` operator would carry a native `like(v)`, but a `like` over an object would have
   to decide each member's shape for the author, which the `literal` rule refuses to do anywhere
   else. The facades keep pact-js's and pact-jvm's `like`.
4. **Are dimension and point names readable?** `whenVariant` and `exclude` name a dimension by
   whatever trailing part of its path is unambiguous (`"shippedAt"`, or
   `"response.body.shippedAt#presence"` in full), and its points by name: `present` and `absent`,
   `non-null` and `null`, an `anyOf`'s values, a `oneOf`'s alternatives, and `min`, `min+1` and `max` for
   a list. The engine resolves them and rejects an ambiguous one at `execute`.
5. **A `oneOf`'s default alternative is the first by name, not the first written** (shape spec §5.4),
   because the order of an object's members does not survive every JSON parser. The base variant of
   `oneOf("type", { invoice, card })` is `card`. The shape language has a `default` member, which the
   DSL does not spell.
6. **Is each language's spelling natural there?** For example, Java's `exclude` takes
   `List<Map.Entry<String, String>>`, Java's parts are configured in lambdas, and TypeScript's
   `variantCases` takes its default as an optional third argument.

## 4. After the review

The answers go into the behavioural specification first, and into the conformance suite, which both
SDKs run (ADR 0017). A question that changes an accepted decision is a superseding ADR. When the
answers are in, the surface is frozen and this note is a record.
