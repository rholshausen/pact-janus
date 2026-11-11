---
jvm-sdk: minor
---

# `Shapes.generated` names a generator on a value

`Shapes.generated(of, generator)` and `Shapes.generated(of, generator, config)` are a value the engine
produces fresh wherever it is produced, a mock's reply each time it serves one and a verifier's
request, while the contract records `of`'s example (staged plan S3.6, ADR 0085):

```java
janus.interaction("get an order")
    .given("an order exists", map("id", 42))
    .request(r -> r.method("GET").path(generated("/orders/42", "pact:ProviderState", map("expression", "/orders/${id}"))))
    .response(r -> r.status(200).body(json(map("id", integer(42), "token", generated(string("a1b2"), "pact:Uuid")))));
```

The generator is named by its component, `pact:` for the generators pact's DSLs have. `of` is a plain
value or a value helper's result; a map, a list, a structural helper or one whose value a variant
chooses throws an `IllegalArgumentException` (write `optional(generated(...))`). A generator nothing
provides is the engine's `interaction-invalid`, from `execute`.
