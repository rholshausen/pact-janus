## 0.2.2 (2026-11-09)

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
