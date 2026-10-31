# JVM (Java 17) SDK style guide

Filled in from the skeleton at
[`Documentation/specs/sdk-specification/examples/style-guide-skeleton.md`](../../Documentation/specs/sdk-specification/examples/style-guide-skeleton.md),
per SDK spec §5. This file records how the
[behavioural specification](../../Documentation/specs/sdk-specification/behavioural-spec.json) is
*expressed* in Java; it never changes what a primitive means. Where the two disagree, the behavioural
specification wins. The SDK was written from the specification alone, without reading the TypeScript
SDK (plan task 6.3); what that turned up is in
[`Documentation/jvm-sdk-from-spec-report.md`](../../Documentation/prototype/jvm-sdk-from-spec-report.md).

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
  | `when-variant`, `variant-cases` | static methods on `io.pact.janus.sdk.VariantBinding`, written as a `given` parameter's value: `given("an order exists", map("shipped", whenVariant("shippedAt", "present")))`. Not on `Shapes`, because a binding is not a shape and is refused wherever a shape is expected |
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
- **A failed provider run** is `VerificationFailedException`, unchecked and not an
  `AssertionError`, carrying `result()` and `failures()`: the failed results that are not pending.
  A verifier naming no source, or two, is `IllegalArgumentException` from `verify()`, before
  anything is sent.
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

## Provider verification

- `Verifier.forProvider(url)`, then `withPacts(paths)`, `withBroker(map)` or `withUrls(map)`, and
  optionally `withOptions(map)`, `withTransport(map)`, `withConfig(path)`, `withComponents(list)` and
  `withEmbedding(embedding)`; then `state(name, setup)` or `state(name, setup, teardown)`,
  `requestFilter(filter)` or `requestFilter(filter, slots)`, and `message(description, producer)`;
  then `verify()` (ADR 0055, ADR 0056). `withTransport` is the HTTP transport's options beside
  `base-url` (`headers`, `client`), passed as given. The
  verifier is mutable and each method returns it, so a test configures it in one chain. Nothing is
  sent before `verify()`.
- The maps passed through — `withBroker`, `withUrls`, `withOptions` — are the protocol's documents,
  keyed as it spells them (`"consumer-version-selectors"`, `"provider-version"`).
- `StateSetup` returns a `Map` of provider-state values, or `null` for none. A handler that cannot
  reach its state throws `StateUnsupportedException` with the reason; any other exception it throws
  is a failed setup. The handlers are functional interfaces that may throw checked exceptions.
- A `RequestFilter` receives a `Map` with a member per request slot, each slot's content as the
  engine sent it, and returns the request; `null` changes nothing.
- A `MessageProducer` receives today's request for the message as a `Map` and returns a `Message`:
  `Message.of(body)`, then `withContentType` and `withMetadata`. The body is a `byte[]`, a `String`,
  or any other value, written as JSON by Jackson. With no content type it is unlabelled, and the
  engine reads its type from the metadata, else its octets.
- `verify()` returns the `VerificationResult` for `verified` and `filtered` runs; a test that must
  not pass a filtered run checks `status()`. Its `warnings()` are the run's `verification/warning`
  payloads, such as a provider state no handler sets up (ADR 0058). The run's report, which the engine
  writes, is printed to standard output as it arrives, as `janus verify` prints it: each interaction,
  its states and its outcome, then the summary (ADR 0059). `withOptions` may give its own `report`,
  such as `Map.of("error-chain", true)`; the SDK lays nothing out itself. Handlers are called on the endpoint's own threads, not
  the test's.
- `start()` starts the same run and returns a `Verifier.VerificationRun`, whose `next()` returns each
  interaction result as the engine reports it, and `null` once the run has ended; `result()` is the
  run's result. Close it, however it ends: that stops the handlers and the engine. `verify()` is
  `start()`, read to the end.

## Provider test-framework integration

- **`JanusProviderExtension`** (ADR 0056 decision 3), registered in a static field with
  `JanusProviderExtension.of(verifier)` or `of(() -> verifier)`, the supplier called when the factory
  runs, so a provider URL chosen in `@BeforeAll` is known. A `@TestFactory` returns
  `janus.interactions(this)`.
- `interactions(this)` declares the instance's `@State` and `@ProvidesMessage` methods on the
  verifier: those of its class, its superclasses and its interfaces' default methods, the most
  specific declaration of a signature winning. A `@State` names states (`value`, several allowed) and
  an `action`, `SETUP` by default. A method takes no parameter or one `Map`, and a setup returns a
  `Map` of values or nothing. A `@ProvidesMessage` method returns a `Message`, bytes, a string, or a
  value written as JSON. Methods may be private and package-private; they are called reflectively, on
  the endpoint's threads, and an exception they throw is the handler's.
- The stream it returns is one run. Each interaction result is a dynamic test named
  `<consumer>: <description>`, with ` [<variant>]` when the variant is not `base`, made when the
  engine reports the result. `verified` passes. A pending failure, or a `skipped` result, is aborted
  (`Assumptions.abort`): reported, failing nothing. Anything else fails, with the result's mismatches
  or error. After the last result, a test named `verification` fails when the run failed with no
  failed result to show it, and is aborted when the run was filtered.
- Closing the stream, which JUnit does, closes the run, and `afterAll` closes any run still open.
- pact-jvm's annotations map to these as
  [`provider-test-migration.md`](../../Documentation/provider-test-migration.md) records.

## Logging

- The engine's logs are the JVM's own (ADR 0061): its events go to the SLF4J logger
  `io.pact.janus.engine`, each at its own level, as `target: message key=value …` (`EngineLog`). The
  engine logs at the most detailed level that logger has enabled, so a project sets it where it sets
  any logger: `logging.level.io.pact.janus.engine=debug` under Spring Boot, or a Logback or Log4j2
  logger entry. A `RUST_LOG` in the environment is passed as it is, and the events it lets through are
  still logged at their own levels.
- The SDK starts the engine with `JANUS_LOG_FORMAT=json` and reads its stderr on a daemon thread,
  reading each event's members, never its text. A line that is not an event, such as a crash, is
  logged whole as an error.
- With no SLF4J provider on the class path, the engine's stderr is inherited, at `warn` unless
  `RUST_LOG` says otherwise.

## Packaging and distribution

- Coordinates `io.pact.janus:janus-sdk` and `io.pact.janus:janus-bindings`, at the version in
  `gradle.properties`. Released as files on a GitHub release tagged `jvm-sdk/v<version>` (ADR 0060):
  each module's jar, sources jar, Ivy descriptor and POM (`./gradlew releaseFiles`). Gradle reads
  them in place through an `ivy` repository; Maven installs them with `install:install-file`.
  `release-check/` is a project outside the SDK that resolves them as a user would, which
  `release-sdks.yml` runs before publishing. Dependencies: `slf4j-api` (the engine's logs, ADR 0061), `jackson-databind` (frames are JSON, and the bindings carry Jackson 2
  annotations) and, `compileOnly`, `junit-jupiter-api` for the extension. Nothing else.
- **The engine is not bundled; it is pinned** (ADR 0060 decision 6). The build copies
  `sdks/engine-release.json` into the jar beside `EngineRelease`, which reads it. The subprocess
  embedding runs the executable `JANUS_ENGINE` names, else that release's engine, from the cache
  (`JANUS_ENGINE_CACHE`, else `pact-janus/engines` in the user's cache directory) or fetched into it
  from the release, or from `JANUS_ENGINE_MIRROR`, and never a file with another digest. It never
  looks on the `PATH`. `JanusOptions.withEmbedding(SubprocessEmbedding.of(path))` names one
  explicitly. When it cannot fetch or start one, an `EngineNotStartedException` (an `IOException`)
  says why (the fetch failed, a path that does not exist, a file that is not executable), then each
  way to get one: a mirror, this platform's release asset, or a build. It carries no stack trace or cause, so a report
  shows the message alone, wrapped to `COLUMNS` when that is set. The SDK's own build runs `cargo build -p pact_janus_cli --bin janus-engine` before its
  tests (Gradle task `:sdk:buildEngine`) and points `JANUS_ENGINE` at the result, so tests never run
  against a stale engine.
- **Embedding.** Subprocess only, which ADR 0023 makes every SDK's primary: a WASM guest cannot host
  a consumer test's mock server (Phase 9 finding 3), so Chicory, the superseded ADR 0003's choice, is not. `Embedding`/`FramePipe` is
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
