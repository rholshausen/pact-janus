---
typescript-sdk: minor
---

# `generated` names a generator on a value

`generated(of, generator, config?)` is a value the engine produces fresh wherever it is produced, a
mock's reply each time it serves one and a verifier's request, while the contract records `of`'s
example (staged plan S3.6, ADR 0085):

```ts
janus.interaction("get an order")
  .given("an order exists", { id: 42 })
  .request({ method: "GET", path: generated("/orders/42", "pact:ProviderState", { expression: "/orders/${id}" }) })
  .response({ status: 200, body: { id: integer(42), token: generated(string("a1b2"), "pact:Uuid") } })
```

The generator is named by its component, `pact:` for the generators pact's DSLs have. `of` is a plain
value or a value helper's result; a map, a list, a structural helper or one whose value a variant
chooses throws a `TypeError` (write `optional(generated(...))`). A generator nothing provides is the
engine's `interaction-invalid`, from `execute`.
