## 0.2.4 (2026-11-13)

### Features

#### `ShapeRecorder` records a provider shape from the provider's own tests

A provider's tests record the shape of what the provider answers, by the operation each test names,
and the engine writes it for `janus check` (staged plan S4.1, ADR 0086):

```ts
import { pathTemplate } from "pact-janus";
import { useShapeRecorder } from "pact-janus/vitest"; // or pact-janus/jest

const recorder = useShapeRecorder({ provider: "order-service" });

test("an order", async () => {
  const response = await fetch(`${url}/orders/66`);
  await recorder.observe({ method: "GET", path: pathTemplate("/orders/{id}") }, response);
  // ...the test's own assertions
});
```

`useShapeRecorder` names the test file as the recording's source and finishes it after the file's
tests: the engine writes `shapes/order-service.provider-shape.json`, merged with every other test
file's, and keeps the counts in `.janus/evidence`. `new ShapeRecorder(config)` and `finish()` are the
same without a test framework. A `fetch` `Response` is read from a clone, so hand it over before
reading its body; `{ status, headers, body }` is the other form. `pathTemplate(template, example?)` is
the HTTP component's path template, which a consumer's request may use too, with an example.

#### `ShapeRecorder.observeMessage` records the messages a provider produces

A provider's tests record the messages it publishes into the same shape as its HTTP answers, by what
identifies each message's operation (staged plan S4.6, ADR 0090):

```ts
const message = createOrderPlacedEvent("order-1"); // a `Message`, as a message producer returns one
await recorder.observeMessage({ metadata: { queue: "orders.placed" } }, message, {
  description: "publish orders.placed",
});
```

`operation` names the message's own `metadata` and `body`, or with `request`, the message a synchronous
message answers, the request's. `description` is required. `janus check` then matches a consumer's
message contract to the entry by what its message carries, here the queue.

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

`InteractionBuilder` and `MessageBuilder` have `exclude(when, reason)` (staged plan S3.2, ADR 0080):

```ts
janus.interaction("get an order")
  .exclude([["status", "SHIPPED"], ["shippedAt", "absent"]], "the order service dates a shipped order")
```

Each call is one exclusion, and calls accumulate. No variant the engine selects is inside an exclusion,
and the contract records each one with its reason and its references resolved to full ids. The
contract still admits the combination, so a provider that sends it passes. A reference that names no
dimension, a point the dimension does not have, and a blank reason fail `execute` with the engine's
`interaction-invalid`. An interaction that does not call `exclude` builds the document it built before.

#### `generated` names a generator on a value

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

#### `headerList` declares a header that is a list

`headerList(...elements)` is a header's value whose elements match however the client or provider
frames them, one field line or several, with any spacing around the commas (staged plan S3.3,
ADR 0082):

```ts
janus.interaction("list orders")
  .request({ method: "GET", path: "/orders", headers: { Accept: headerList("application/json", "text/plain") } })
  .response({ status: 200, headers: { Vary: headerList("Accept", "Origin") } })
```

A mock serves it as one field line, `Accept, Origin`. It is accepted only as a request or response
header's value; a query value, a body or another helper refuses it. A plain string or list is still
compared as the header's field lines, exactly. `HeaderList` and the `HeaderValue` type are exported.

#### Each variant a test of its own under Vitest and Jest, and what the mock did logged as the test runs

Staged plan S3.5, ADR 0084.

- **`await janus.variants(interaction, closure)`**, on the `Janus` that `pact-janus/vitest`'s or
  `pact-janus/jest`'s `useJanus` returns, registers one test per variant the engine selects, named
  `label [id]` (`base` for the base variant). Await it at a test file's top level (Vitest also in an
  async `describe`); Jest needs the file to be an ES module. A failing variant fails only its own test,
  with its closure's error or the engine's verdict. `variantTests` on any `Janus` answers the same tests
  as `{ name, run }`, for another framework.
- **What the mock made of each request is logged while the closure runs**: a request it rejected
  through `console.warn` (`pact-janus: 'get an order' [base]: the request did not match`, then each
  mismatch), one it matched through `console.debug`, as `logLevel` (else `LOG_LEVEL`, else `warn`)
  allows. The SDK asks for the session's events and polls them during each closure, which needs an
  engine that answers `create` with a `stream`; with an older one it polls nothing.

### Fixes

#### The generated bindings carry an interaction's sampling policy

The contract bindings' `InteractionSpec` and `Interaction` have an optional `policy`: the
interaction's own sampling policy (ADR 0080), which the engine resolves when the interaction is added
and records in the contract. The SDK writes none yet; S3.2's `exclude` will.

## 0.2.2 (2026-11-09)

The SDK runs janus-engine 0.5.0, where it ran 0.4.5: [its release](https://github.com/rholshausen/pact-janus/releases/tag/v0.5.0) says what changed.

### Features

#### `Janus` takes a `redaction` option

`new Janus({ ..., redaction: { add, keep } })` names, by path, members the contract records redacted
beyond the credentials the transport declares (`add: ["request.headers.x-api-key"]`), and ones it
records as they are (`keep: ["request.headers.authorization"]`, for a fixed token the provider's test
environment accepts). It is passed to the engine as written, as `config.redaction` (ADR 0074). With
an engine that has S2.8, a contract records `Authorization`, `Proxy-Authorization`, `Cookie` and
`Set-Cookie` as `"[redacted]"` whether or not the option is given.

### Fixes

#### A pact-js test that throws prints the request it never sent

`pact-janus/pact`'s `executeTest`, when the test throws without calling the mock, now prints
"Missing request: the interaction '…' was expected, but not received" beside rethrowing the test's
error, as pact-js does. It reads the engine's `exercised: false` (staged plan S2.6), so it needs an
engine that writes it.

## 0.2.1 (2026-11-05)

The SDK runs janus-engine 0.4.5, where it ran 0.4.4: [its release](https://github.com/rholshausen/pact-janus/releases/tag/v0.4.5) says what changed.

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
finalise with the engine's `contract-conflict`, a `JanusError` naming it. `overwriteContract: true` replaces the file
instead. Because what merges is whatever the file held, a run should clear its contract directory
first. This needs an engine with `contract-writing`; an older one named by `JANUS_ENGINE` fails
`finalise`, saying so.

#### A variant fails in its own test, and the engine withholds the contract

`execute` reports each closure's verdict to the engine and fails on each variant the engine did not
verify (ADR 0064): the closure threw, the mock rejected its request, or nothing called the mock.
These used to fail only `finalise`, in the suite's `afterAll`. A closure that never calls the mock
now fails its own `execute`. `VariantFailure` carries the engine's `result`, and `cause` only when
the closure threw.

The SDK no longer withholds a contract itself: the engine does. `ContractWithheldError` loses
`failedTests` and `engineWithheld`, and gains `interactions` (those that did not verify, then those
the engine refused) and `refused`. This needs an engine with `consumer-session/report-variant`,
which an older engine named by `JANUS_ENGINE` answers `operation-unsupported` at the first `execute`.

### Features

#### `pact-janus/jest`, the Jest integration

`useJanus(config)` from `pact-janus/jest` does under Jest what it does under Vitest: it creates the
suite's `Janus` and runs `finalise` in an `afterAll`, so the contract is written after the file's
last test and a withheld contract fails the file. It is CommonJS and ES modules alike, takes
`afterAll` from `@jest/globals`, so it works with `injectGlobals: false`, and is tested on Jest 30,
which it names as an optional peer dependency. The provider side already ran under Jest; it is now
tested there too, from CommonJS and as ES modules.

#### `pact-janus/pact`: pact-js's consumer API

`pact-janus/pact` exports pact-js's names: `Pact` (`PactV4`), `PactV3`, `Matchers` (`MatchersV3`),
`XmlBuilder`, `SpecificationVersion`, and the provider side's `Verifier`, `MessageProviderPact` and
`providerWithMetadata`, so a pact-js suite moves by changing its import (ADR 0066). The matchers are
pact-js's own data; the builders, HTTP and GraphQL, record the `pact_ffi` calls pact-js's would make,
and `executeTest` hands them to the engine's `consumer-session/add-pact-interaction`, which needs an
engine with that operation. A test runs once, against its example, and records the Janus contract
`janus upgrade` would write from the pact pact-js writes, in `<dir>/<consumer>-<provider>.janus.json`,
merged with other test files'. `usingPlugin`, `tls` and, until S2.5c, the message API are refused by
name; `pending`, `comment`, `testName` and `reference` are accepted and do nothing. Several
interactions in one `executeTest` are refused until S2.13. `Documentation/consumer-test-migration.md`
gives each primitive's verdict.

#### `pact-janus/pact`: pact-js's message consumer API

`Pact`'s `addAsynchronousInteraction` and `addSynchronousInteraction`, `MessageConsumerPact`, and
`synchronousBodyHandler`, `asynchronousBodyHandler`, `v4SynchronousBodyHandler` and
`v4AsynchronousBodyHandler` are pact-js's, so a message consumer suite moves by changing its import
(S2.5c). The builders record the `pact_ffi` calls pact-js's make, which needs an engine whose
`add-pact-interaction` builds messages. No mock is started: the test is handed the message the
engine answers, in the form pact-js hands it, and the contract is the one `janus upgrade` writes from
the pact pact-js writes. A synchronous message's response metadata is accepted and does nothing, as
pact-js's does, and `usingPlugin` is refused by name. Until S2.15, a message given as octets is
handed back as their base64 text. `Documentation/consumer-test-migration.md` gives each primitive's
verdict.

#### `pact-janus/pact`: several interactions in one `executeTest`

`PactV3`'s `executeTest` serves every interaction added since the last one, and `Pact`'s every
interaction added since its last `executeTest`, as pact-js's do, each as often as the test calls it.
A test that never calls one of them fails, naming its missing request, and writes no contract. It
needs an engine that arms several interactions at once (ADR 0069).

#### A consumer test can record a message

`janus.message(description)` builds a message interaction (ADR 0068): `given`, `contents({ metadata,
body })` for the message and `request({ metadata, body })` for the request a synchronous message
answers. A body written with `content(type, …)` declares that type, and any other is JSON.
`janus.execute(message, (message, variant) => …)` starts no mock, and hands the closure each variant's
`ReceivedMessage`: `body` as a `Uint8Array`, `contentType`, `metadata`, `text()`, `json()`, and a
synchronous message's `request`. The contract records it, and a provider's message producers verify
it. It needs an engine with ADR 0067's `serve-variant` parts.

### Fixes

#### The contract written is the engine's text, and pacts are sent as read

`finalise` writes the contract text the engine answers, byte for byte, where it wrote
`JSON.stringify` of the document. A JavaScript parse had respelled numbers (`1.0` as `1`, `1e16` as
`10000000000000000`) and rounded an integer above 2^53, so a contract with one could be written
changed. The `Verifier` sends each pact file's text unparsed, so a provider is verified against what
the file says (ADR 0063). Both need an engine with the `contract-text` capability, which the SDK's
pinned engine has. An older engine named by `JANUS_ENGINE` fails `finalise`, saying so, rather than
reporting a withheld contract.

#### Bindings for a delivered message

The generated protocol bindings have `ServeVariantResult.parts` and `DeliveredSlotValue`, the message
`consumer-session/serve-variant` answers for an emissive interaction (ADR 0067), and the upgrade
findings `body-type-on-metadata` and `responses-dropped`. Nothing in the SDK builds a message
interaction yet (S2.14).

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
