# JVM (Java 17) SDK style guide

Filled in from the skeleton at
[`Documentation/specs/sdk-specification/examples/style-guide-skeleton.md`](../../Documentation/specs/sdk-specification/examples/style-guide-skeleton.md),
per SDK spec §5. This file records how the
[behavioural specification](../../Documentation/specs/sdk-specification/behavioural-spec.json) is
*expressed* in Java; it never changes what a primitive means. Where the two disagree, the behavioural
specification wins. The SDK was written from the specification alone, without reading the TypeScript
SDK (plan task 6.3); what that turned up is in
[`Documentation/jvm-sdk-from-spec-report.md`](../../Documentation/jvm-sdk-from-spec-report.md).

## Naming and module layout

- **Case.** Primitive ids become lowerCamelCase Java methods: `each-like` → `eachLike`, `any-of` →
  `anyOf`, `one-of` → `oneOf`. `boolean` is a Java keyword, so it is `bool` (see Deviations).
- **A primitive with an empty `signature`** is a no-argument static method, not a constant —
  `Shapes.forbidden()`, never a `Shapes.FORBIDDEN` field. SDK spec §3.2 requires the style guide to
  say which, because no conformance case can see the difference. Two reasons for the method here:
  every shape helper then reads the same way at a call site, and a call returns a fresh `ShapeNode`,
  where a shared constant would be an aliasing hazard — `ShapeNode` is documented as immutable but
  is not defensively copied.
- **Where each primitive lives.**

  | Primitive(s) | Java |
  |---|---|
  | `janus` | `Janus.of(consumer, provider[, JanusOptions])` |
  | `interaction` | `janus.interaction(description)` → `Interaction` |
  | `given`, `request`, `response` | `Interaction.given(...)`, `.request(r -> ...)`, `.response(r -> ...)` |
  | `execute` | `janus.execute(interaction, (mock, variant) -> ...)` |
  | `finalise` | `janus.finalise()`; run for you by `JanusExtension` |
  | `literal` | no call site: any plain value where a shape is expected |
  | `json`, `integer`, `number`, `decimal`, `string`, `boolean`, `datetime`, `date`, `time`, `regex`, `any-of`, `one-of`, `optional`, `nullable`, `forbidden`, `each-like` | static methods on `io.pact.janus.sdk.Shapes`, for `import static io.pact.janus.sdk.Shapes.*` |

- **Modules.** Gradle project `:bindings` (package `io.pact.janus.bindings.<set>.v1`) is generated and
  never edited. Project `:sdk` is the idiomatic layer: package `io.pact.janus.sdk` (the DSL, `Janus`,
  the JUnit extension) and `io.pact.janus.sdk.engine` (the embedding seam: `Embedding`, `FramePipe`,
  `SubprocessEmbedding`).
- **Generated types stay inside.** The SDK builds and reads protocol documents only through the
  bindings (`InteractionSpec`, `Shape`, `Create`, `VariantsResult`, `FinaliseResult`, …) and never
  hand-writes a frame. Its public surface shows the user its own small types (`ShapeNode`, `Mock`,
  `Variant`, `JanusEngineException`) instead. The two exceptions are deliberate:
  `Interaction.toSpec()` returns the generated `InteractionSpec`, so a test can inspect exactly the
  document `execute` submits, and `ContractWithheldException.results()` returns the engine's
  `InteractionResult`s unchanged.

## The async model

- **Blocking.** Every call that talks to the engine blocks the calling thread until the engine has
  answered. That is what JUnit tests, and most JVM test code, expect, and the subprocess pipe is serial
  anyway. No `CompletableFuture` appears in the API.
- **The closure** (`VariantTest`: `void run(Mock mock, Variant variant) throws Exception`) is
  synchronous too: `execute` calls `serve-variant`, then the closure, and waits for it to return or
  throw before arming the next variant. Anything the closure throws (a checked exception, an
  `AssertionError`, any `RuntimeException`) fails that variant. It is caught, recorded and rethrown
  inside one `ExecuteFailedException` after every variant has run. The SDK never swallows it.
  `VirtualMachineError`s (`OutOfMemoryError` and the like) are not caught. A closure that wants to
  test async client code joins its own futures before it returns.
- `Janus` synchronises its own methods, and it does not hold the lock while the closure runs.

## The error surface

- **Engine errors** (engine-protocol spec §10), whatever the code or category, are one type:
  `JanusEngineException` (unchecked), with `operation()`, `code()`, `category()`, `engineMessage()`,
  `details()` (the engine's map, unchanged), and two typed accessors for the conventions §10.2
  fixes: `problems()` (`interaction-invalid`/`contract-invalid`: `Problem(pointer, message)`,
  verbatim) and `supported()` (`protocol-version-unsupported`). One type, not a subclass per code,
  because the code vocabulary is open: a code this SDK has never heard of still arrives named, and
  a `switch` on `code()` degrades better than a `catch` on a class that does not exist yet. `details`
  stays a generic map for the same reason. Only the two conventions the protocol itself fixes get
  typed accessors.
- **Pipe failures** (engine missing, crashed, wrote something that is not a frame, pipe-level
  `malformed-frame` with an empty id) are `JanusEmbeddingException`. They are not engine errors,
  because the engine could not answer.
- **Test outcomes are `AssertionError`s.** JUnit reports those as *failures*, not *errors*.
  `ExecuteFailedException` names every failed variant by label and id, and chains each cause (first
  as cause, the rest suppressed). `ContractWithheldException` names every interaction and variant that
  did not verify, with the engine's mismatches, and every interaction whose `execute` failed.
- **DSL misuse caught at the call** is `IllegalArgumentException`: a value with no JSON reading, a
  `Pattern` with out-of-band flags, a header declared twice. This is only about Java values the
  SDK cannot turn into a document at all. Shape validity is the engine's (`interaction-invalid`).

## Builder ergonomics

- A **fluent chain** for the interaction, as in the RFC: `janus.interaction(..).given(..).request(..)
  .response(..)`. The `request`/`response` options bags are **lambda-configured part builders**
  (`r -> r.method("GET").path("/orders/42").header("Accept", "application/json")`). A member not
  called produces no slot. Calling `request` (or `response`) again replaces that part.
- **Plain values are the literal rule.** Anything that is not a `ShapeNode` is compiled by the
  behavioural spec's `literal` rules: `String`/`Number`/`Boolean`/`null` → `equality`; `Map` →
  `object`, member by member; `List` → `array`, entry by entry. The SDK tells a helper's result from a
  plain map **by Java type** (`instanceof ShapeNode`), never by looking inside the map. A value with no
  JSON reading (an `Instant`, an enum, a POJO) is refused, not guessed.
- **Map literals.** `Map.of` has no encounter order, rejects `null` values, and its iteration
  order changes from one JVM run to the next. `Shapes.map("k1", v1, "k2", v2, …)` is the SDK's ordered
  map literal. It returns a plain `LinkedHashMap`, so it is data compiled by the literal rules, not a
  shape helper. Maps with no defined order are written in key order (see Deviations).
- Headers and query parameters use the name-to-list rule exactly as specified: a `String` →
  `equality` over `[value]`, a `List<String>` → `equality` over the list, a `ShapeNode` → `each-like`
  of it, bounded at exactly one value (`min` 1, `max` 1). Header names are lower-cased with `Locale.ROOT`. Query names are kept as written.
- `Shapes.content(mediaType, document)` returns a `Content`, not a `ShapeNode`: it names a whole slot,
  so `body(Object)` is the only member that accepts one, and the literal rules and the name-to-list
  rule refuse it with an `IllegalArgumentException` naming where it was written (behavioural spec
  `content`, ADR 0019). It cannot be refused at compile time without overloading `body`, and one
  `Object` parameter is what every other slot takes.
- Components are `JanusOptions.withComponents(List<Map<String, ?>>)`: the project configuration's
  own declarations, as plain maps (checked for JSON-ness, not for meaning). The SDK does not read
  `consumer.janus.yaml` — see the TypeScript style guide's reason, which is the same one.

```java
Interaction getOrder = janus.interaction("get an order")
    .given("an order exists", map("id", "42"))
    .request(r -> r.method("GET").path("/orders/42"))
    .response(r -> r
        .status(200)
        .body(json(map(
            "id", integer(42),
            "status", anyOf("PENDING", "SHIPPED", "DELIVERED"),
            "shippedAt", optional(datetime("2026-07-30T10:00:00Z")),
            "payment", oneOf("type", map(
                "card", map("type", "card", "last4", regex("\\d{4}", "1234")),
                "invoice", map("type", "invoice", "dueDate", date("2026-08-30")))),
            "items", eachLike(map("sku", string("SKU-1"), "qty", integer(1)), Cardinality.min(1))))));
```

## Test-framework integration

- **JUnit Jupiter** (the JUnit 6 BOM the bindings project already uses, same `org.junit.jupiter`
  API). Register one `JanusExtension` per test class, in a static field:
  `@RegisterExtension static final JanusExtension janus = JanusExtension.of("web-app", "orders-api");`.
  It implements `AfterAllCallback` and runs `finalise` after the class's last test, **including when a
  test failed**. A withheld contract then fails the class. A static extension field is inherited by
  `@Nested` classes, so the extension skips their `afterAll` and only the declaring class's run ends
  the session.
- **Two ways to run a closure.** `janus.execute(interaction, closure)` inside a `@Test` is the RFC's
  shape: every variant runs within one test, and the test fails once naming each failed variant.
  `janus.variants(interaction, closure)` returned from a `@TestFactory` makes **each selected variant
  its own dynamic test**, named `label [id]` (just `base` for the base variant). A failing variant
  fails only its own test, with the closure's own exception, and still withholds the contract. Both
  share one implementation (`Janus.begin` / `Janus.run`), so they make the same protocol calls in the
  same order.
- The **engine** is started lazily by the first `execute`, and ended by `finalise`. After `finalise`
  the same `Janus` may be used again; the next `execute` starts a fresh engine and session.

## Packaging and distribution

- Maven coordinates `io.pact.janus:sdk` and `io.pact.janus:bindings`, version `0.0.0` (a prototype,
  unpublished). Dependencies: `jackson-databind` (frames are JSON, and the bindings carry Jackson 2
  annotations) and, `compileOnly`, `junit-jupiter-api` for the extension. Nothing else.
- **The engine is not bundled.** The subprocess embedding runs the executable `JANUS_ENGINE` names,
  or `janus-engine` from `PATH`. `JanusOptions.withEmbedding(SubprocessEmbedding.of(path))` names one
  explicitly. The SDK's own build runs `cargo build -p pact_janus_cli --bin janus-engine` before its
  tests (Gradle task `:sdk:buildEngine`) and points `JANUS_ENGINE` at the result, so tests never run
  against a stale engine.
- **Embedding.** Subprocess only. ADR 0003 names Chicory as the JVM's primary embedding, but a WASM
  guest cannot host a consumer test's mock server (Phase 9 finding 3). `Embedding`/`FramePipe` is
  the seam a Chicory embedding would implement: `FramePipe.call(byte[]) -> byte[]` is exactly the
  WASM call pipe's shape (engine-protocol spec §3.1).
- **Protocol version.** Pinned to protocol 1. `engine/hello` offers `[1]` and declares no
  capabilities, so frames are always JSON and events are never pushed.
- **HTTP clients against the mock.** Any HTTP/1.1 client works, including the JDK's
  `HttpClient.newHttpClient()`: the mock declines its HTTP/2 cleartext upgrade offer and answers
  over HTTP/1.1. (It did not answer at all until the fix following the 6.3 report.)

## Deviations from the behavioural specification

| Primitive | Deviation | Reason |
|---|---|---|
| `boolean` | spelled `bool(example)` | `boolean` is a reserved word in Java; no other spelling reads as the primitive |
| `literal`, `one-of`, `given`, `request` (headers/query maps) | a `Map` with no defined encounter order (`Map.of`, `HashMap`, …) is written in **key order**, not "the order written"; a `LinkedHashMap`, a `SortedMap`, or `Shapes.map(...)` keeps its own order | Java has no ordered map literal. `Map.of` discards the written order, and its iteration order is salted per JVM run, so honouring it would make the same DSL produce different documents run to run. Key order is the only deterministic choice left. It only changes member order, never content. (The contract's canonical writer keys open maps — parts, shapes' members, params — in sorted order anyway, for deterministic bytes.) |
| `each-like` | the options bag is a `Cardinality` value: `Cardinality.min(1)`, `.max(5)`, `.between(1, 5)`, or no argument | a Java method cannot take `{ min: 1 }`; `Cardinality` carries "given" vs "not given" so absent bounds are still not written |
| `regex` | also accepts a `java.util.regex.Pattern`, and refuses one compiled with flags its text does not carry | the spec requires refusing out-of-band options; `Pattern.flags()` folds inline flags in, so the SDK compares against the flags the bare pattern text compiles to |
| `any-of` | `anyOf(null)` (a null varargs array) means one option, `null` | Java passes a lone `null` argument as a null array; that is what the author meant |
| `request`, `response` | a header or query value may also be any `Number` that is a whole number within ±(2^53 − 1) — including a `double` like `3.0` — or a `Boolean`, written as the one string that spells it | no longer a deviation: ADR 0019 made this the rule for every SDK, and the entry stays only to name what Java accepts. A `float`/`double` with a fractional part, a `long` past the bound, an `Instant`, `null` and everything else are refused at the call |
| `execute` | also offered as `JanusExtension.variants(...)` for a `@TestFactory`, where each variant fails as its own test instead of `execute` failing once | the style skeleton asks for per-variant test results. The protocol sequence and the `finalise` verdict are identical |
| `janus` | an engine that *agrees* a protocol version other than 1 fails the first `execute` with `JanusEmbeddingException`, not an engine error | the spec says "fails with the engine's error", but in that case the engine sent none. The protocol has no code for it |
