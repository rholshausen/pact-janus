## 0.2.4 (2026-11-13)

The SDK runs janus-engine 0.7.0, where it ran 0.6.0: [its release](https://github.com/rholshausen/pact-janus/releases/tag/v0.7.0) says what changed.

### Features

#### `ShapeRecorder` records a provider shape from the provider's own tests

A provider's tests record the shape of what the provider answers, by the operation each test names,
and the engine writes it for `janus check` (staged plan S4.1, ADR 0086):

```java
static final ShapeRecorder recorder = ShapeRecorder.forProvider("order-service");

@Test
void anOrder() throws Exception {
  HttpResponse<byte[]> response = client.send(request("/orders/66"), BodyHandlers.ofByteArray());
  recorder.observe(Map.of("method", "GET", "path", Shapes.pathTemplate("/orders/{id}")),
      ShapeRecorder.Response.of(response));
  // ...the test's own assertions
}

@AfterAll
static void finish() {
  recorder.finish();
}
```

The recording's source is the class that called `forProvider`. `finish()` has the engine write
`shapes/order-service.provider-shape.json`, merged with every other class's, and keep the counts in
`target/janus/evidence` beside a `pom.xml`, else `build/janus/evidence`. `Response.of` sends the
request the response answered, which the engine checks the operation admits; `new Response(status,
headers, body)` is the other form. `Shapes.pathTemplate(template)` and `pathTemplate(template,
example)` are the HTTP component's path template, which a consumer's request may use too, with an
example.

#### `ShapeRecorder.observeMessage` records the messages a provider produces

A provider's tests record the messages it publishes into the same shape as its HTTP answers, by what
identifies each message's operation (staged plan S4.6, ADR 0090):

```java
recorder.observeMessage(Map.of("metadata", Map.of("queue", "orders.placed")), orderPlaced(),
    ShapeRecorder.MessageOptions.described("publish orders.placed"));
```

The message is the `Message` a `@ProvidesMessage` method returns. `operation` names its `metadata` and
`body`, or with `MessageOptions.withRequest(message)`, a synchronous message's request. `janus check`
then matches a consumer's message contract to the entry by what its message carries, here the queue.

### Fixes

#### The generated bindings carry a compatibility report's provenance

The protocol bindings' `PolicyView` has an optional `provenance`, a map to the new
`ProvenanceActions`, and `FindingEntry` and `ExemptionResult` an optional `provenance` (ADR 0089).
Nothing in the SDK reads them.

#### The generated bindings carry a provider-shape recording's operation and write

The protocol bindings' `StartRecording` has an optional `kind`, `ObserveResponse` an optional
`operation`, `ObserveResponseResult` optional `warnings`, `FinaliseRecording` an optional `write`
(`ShapeWrite`) and `FinaliseRecordingResult` an optional `written` (ADR 0086). The SDK records no
shape yet.

#### The generated bindings carry an observation's kind

The protocol bindings' `ObserveResponse` has an optional `kind` (ADR 0090), which `observeMessage`
sends.

## 0.2.3 (2026-11-12)

The SDK runs janus-engine 0.6.0, where it ran 0.5.0: [its release](https://github.com/rholshausen/pact-janus/releases/tag/v0.6.0) says what changed.

### Features

#### `exclude` leaves out a combination the provider cannot produce

`Interaction` and `MessageInteraction` have `exclude(when, reason)` (staged plan S3.2, ADR 0080), the
pairs a `List` of `Map.Entry<String, String>`, dimension to point:

```java
janus.interaction("get an order")
    .exclude(List.of(entry("status", "SHIPPED"), entry("shippedAt", "absent")),
        "the order service dates a shipped order")
```

Each call is one exclusion, and calls accumulate. No variant the engine selects is inside an exclusion,
and the contract records each one with its reason and its references resolved to full ids. The
contract still admits the combination, so a provider that sends it passes. A reference that names no
dimension, a point the dimension does not have, and a blank reason fail `execute` with the engine's
`interaction-invalid`. An interaction that does not call `exclude` builds the document it built before.

#### `Shapes.generated` names a generator on a value

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

#### `Shapes.headerList` declares a header that is a list

`Shapes.headerList(Object... elements)` is a header's value whose elements match however the client or
provider frames them, one field line or several, with any spacing around the commas (staged plan
S3.3, ADR 0082):

```java
janus.interaction("list orders")
    .request(r -> r.method("GET").path("/orders").header("Accept", headerList("application/json", "text/plain")))
    .response(r -> r.status(200).header("Vary", headerList("Accept", "Origin")));
```

A mock serves it as one field line, `Accept, Origin`. It is accepted only as a request or response
header's value; a query value or the literal rules refuse it with an `IllegalArgumentException`. A
`String` or `List` is still compared as the header's field lines, exactly.

#### Each variant a row of its own under Spock, and what the mock did logged as the test runs

Staged plan S3.5, ADR 0084.

- **Spock**: `janus.variants(interaction)` in a feature's `where:` block answers one `VariantRun` per
  selected variant, and `janus.run(variant) { mock, v -> }` runs it, failing with the closure's own
  exception or the engine's verdict. A feature named `"#variant"` unrolls into one iteration per variant,
  named `label [id]`. A message's rows are `MessageVariantRun`s, run with a `MessageTest`.
- **What the mock made of each request is logged while the test runs**, to the SLF4J logger
  `io.pact.janus.engine`: a request it rejected at `warn` (`'get an order' [base]: the request did not
  match`, then each mismatch), one it matched at `debug`. The SDK asks for the session's events only
  when an SLF4J provider is bound; with none, the engine's own warning on its stderr says it, as before.

### Fixes

#### The generated bindings carry an interaction's sampling policy

The contract bindings' `InteractionSpec` and `Interaction` have an optional `policy`: the
interaction's own sampling policy (ADR 0080), which the engine resolves when the interaction is added
and records in the contract. The SDK writes none yet; S3.2's `exclude` will.

## 0.2.2 (2026-11-09)

The SDK runs janus-engine 0.5.0, where it ran 0.4.5: [its release](https://github.com/rholshausen/pact-janus/releases/tag/v0.5.0) says what changed.

### Features

#### `JanusOptions` takes the members to redact and to keep

`JanusOptions.withRedacted(paths)` names, by path, members the contract records redacted beyond the
credentials the transport declares (`request.headers.x-api-key`), and `withKept(paths)` ones it records
as they are (`request.headers.authorization`, for a fixed token the provider's test environment
accepts). They are passed to the engine as written, as `config.redaction` (ADR 0074). With an engine
that has S2.8, a contract records `Authorization`, `Proxy-Authorization`, `Cookie` and `Set-Cookie` as
`"[redacted]"` whether or not they are given.

### Fixes

#### A pact-jvm test that throws prints the request it never sent

`janus-facade`'s JUnit 5 extension, when a test throws without calling the mock, now prints "The
following requests were not received" with the interaction beside the test's own failure, as pact-jvm
does. It reads the engine's `exercised: false` (staged plan S2.6), so it needs an engine that writes it.

## 0.2.1 (2026-11-05)

The SDK runs janus-engine 0.4.5, where it ran 0.4.4: [its release](https://github.com/rholshausen/pact-janus/releases/tag/v0.4.5) says what changed.

### Features

#### `janus-facade`: pact-jvm's Java DSL and JUnit 5 consumer extension

A new jar, `janus-facade`, released beside `janus-sdk`, is pact-jvm's consumer DSL over the engine
(ADR 0070, staged plan S2.5b): the `PactDslWithProvider` chain, `PactDslJsonBody`, `PactDslJsonArray`,
both root values, `LambdaDsl`, `PactXmlBuilder`, `FormPostBuilder`, `MessagePactBuilder` and
`PactConsumerTestExt`, under `io.pact.janus.facade.` with pact-jvm's names. A suite moves by adding
`io.pact.janus:janus-facade` and replacing `au.com.dius.pact.` with `io.pact.janus.facade.` in its
imports. Each builder builds the example, rules and generators pact-jvm's builds and submits them
through `add-pact-interaction`, so a test records the contract `janus upgrade` writes from the pact
pact-jvm writes, as `<consumer>-<provider>.janus.json` where pact-jvm writes its pacts. pact-jvm 4.7.8's
Java `consumer/junit5` tests run on it with only their imports changed.
`Documentation/consumer-test-migration.md` gives each primitive's verdict. A TLS mock server or a
named mock server implementation is refused by name, JUnit 4's `PactProviderRule` is not provided,
and pact-jvm's V4 `PactBuilder`, Groovy and Kotlin DSLs are staged plan S2.5d.

It needs an engine whose `add-pact-interaction` takes `with_binary_body` (with its
`content_type_header`) and `with_generators`, which v0.4.4 does not, so this release waits for the
engine release after it. The SDK's `Engine` sends frames for a layer that speaks the protocol itself.

### Fixes

#### Bindings know `with_binary_body` and `with_generators`

The generated protocol bindings' vocabulary lists the two `pact_ffi` calls
`consumer-session/add-pact-interaction` now makes (ADR 0070). Nothing in either SDK calls them yet;
pact-jvm's facade will (S2.5b).

## 0.2.0 (2026-11-04)

The SDK runs janus-engine 0.4.4, where it ran 0.4.3: [its release](https://github.com/rholshausen/pact-janus/releases/tag/v0.4.4) says what changed.

### Breaking Changes

#### Test files recording one consumer and provider write one contract

`finalise` no longer writes the contract file: it names it, and the engine writes it, merging into
what the file holds (ADR 0065). Several test files recording one consumer and provider, in separate
workers or processes, write one contract with all of their interactions, where the last to finish
used to overwrite the others. An interaction two of them record differently fails the second to
finalise with the engine's `contract-conflict`, a `JanusEngineException` naming it. `JanusOptions.withContractOverwrite(true)` replaces the file
instead. Because what merges is whatever the file held, a run should clear its contract directory
first. This needs an engine with `contract-writing`; an older one named by `JANUS_ENGINE` fails
`finalise`, saying so.

#### A variant fails in its own test, and the engine withholds the contract

`execute`, and each dynamic test `variants` returns, reports the test's verdict to the engine and
fails on each variant the engine did not verify (ADR 0064): the test threw, the mock rejected its
request, or nothing called the mock. These used to fail only `finalise`, after the class's last
test. A test that never calls the mock now fails. `VariantFailure` is `(variant, result, cause)`,
whose `cause` is the test's throwable or an `AssertionError` saying the engine's verdict.

The SDK no longer withholds a contract itself: the engine does. `ContractWithheldException` loses
`failedInteractions()` and `engineWithheld()`, and gains `interactions()` (those that did not verify,
then those the engine refused) and `refused()`. This needs an engine with
`consumer-session/report-variant`, which an older engine named by `JANUS_ENGINE` answers
`operation-unsupported` at the first `execute`.

### Features

#### A consumer test can record a message

`janus.message(description)` builds a `MessageInteraction` (ADR 0068): `given`,
`contents(m -> m.metadata(..).body(..))` for the message and `request(m -> ...)` for the request a
synchronous message answers. A body written with `Shapes.content(type, …)` declares that type, and any
other is JSON. `janus.execute(message, (received, variant) -> ...)` starts no mock, and hands the test
each variant's `ReceivedMessage`: `body()`, `contentType()`, `metadata()`, `text()`, `json()`, and a
synchronous message's `request()`. `JanusExtension` has `message`, `execute` and `variants` for it.
It needs an engine with ADR 0067's `serve-variant` parts.

### Fixes

#### Bindings for `add-pact-interaction`

The generated protocol bindings have the types of the engine's new
`consumer-session/add-pact-interaction` (ADR 0066): `AddPactInteraction`, `PactSteps`, `PactStep`
and `AddPactInteractionResult`. Nothing in the SDK calls it yet; pact-jvm's facade will (S2.5b).

#### Bindings know the message calls of `add-pact-interaction`

The generated protocol bindings' vocabulary lists the `pact_ffi` message calls
`consumer-session/add-pact-interaction` now makes (S2.5c). Nothing in the SDK calls them yet;
pact-jvm's facade will (S2.5b).

#### Bindings for a delivered message

The generated protocol bindings have `ServeVariantResult.parts` and `DeliveredSlotValue`, the message
`consumer-session/serve-variant` answers for an emissive interaction (ADR 0067), and the upgrade
findings `body-type-on-metadata` and `responses-dropped`. Nothing in the SDK builds a message
interaction yet (S2.14).

#### The contract written is the engine's text, and pacts are sent as read

`finalise` writes the contract text the engine answers, where it copied the contract's bytes out of
the response frame, which only worked while frames were JSON. The `Verifier` sends each pact file's
text unparsed (ADR 0063). Both need an engine with the `contract-text` capability, which the SDK's
pinned engine has. An older engine named by `JANUS_ENGINE` fails `finalise`, saying so, rather than
reporting a withheld contract.

## 0.1.1 (2026-10-31)

The SDK runs janus-engine 0.4.3, where it ran 0.4.2: [its release](https://github.com/rholshausen/pact-janus/releases/tag/v0.4.3) says what changed.

### Features

#### The engine's logs are set and shown as the test's own

How much the engine logs is now set the way the test's language sets any logging, and a `RUST_LOG`
that is set still wins (ADR 0061).

- **TypeScript:** the `logLevel` option of `Janus` and `Verifier`, else `LOG_LEVEL`, else `warn`, as
  in pact-js. The `pact-janus/verifier` facade passes pact-js's `logLevel` through, where it had no
  effect.
- **JVM:** the engine's events are logged through SLF4J to the logger `io.pact.janus.engine`, at their
  own levels, and the engine logs at the most detailed level that logger shows. Under Spring Boot,
  `logging.level.io.pact.janus.engine=debug` in `application.properties` shows them in the test's own
  log and report. The SDK now depends on `slf4j-api`. With no SLF4J provider bound, the engine's
  stderr is inherited, at `warn`.

## 0.1.0 (2026-10-31)

### Features

#### The SDK is released, and fetches the engine it was released with

The SDK is released as files on a GitHub release of its own: `pact-janus-<version>.tgz`, installed by
its URL, or the JVM SDK's `io.pact.janus:janus-sdk` and `janus-bindings` jars, read by Gradle through
an `ivy` repository (ADR 0060). The JVM modules were `io.pact.janus:sdk` and `io.pact.janus:bindings`.

Told no engine to run, the SDK runs the `janus-engine` release it pins. It fetches the engine on
first use into `JANUS_ENGINE_CACHE`, or by default `pact-janus/engines` in the user's cache directory.
It fetches from `JANUS_ENGINE_MIRROR` when that is set, and refuses any file whose SHA-256 is not the
pinned one. `JANUS_ENGINE` still names another engine. The SDK no longer runs a `janus-engine` from
the `PATH`; name it in `JANUS_ENGINE` instead.

The engine it pins is janus-engine 0.4.2, the first release with everything the SDK asks of it: the
v4 `statusCode` matcher, and message producers served from the test.
