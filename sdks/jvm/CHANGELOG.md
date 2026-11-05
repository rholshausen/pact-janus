## 0.2.1 (2026-11-05)

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
