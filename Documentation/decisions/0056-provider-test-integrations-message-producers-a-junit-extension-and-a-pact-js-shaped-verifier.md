# 0056 — Provider test integrations: message producers as loopback hooks, a JUnit 5 extension that reports each interaction, and a pact-js-shaped `Verifier`

- **Status**: accepted
- **Date**: 2026-10-30
- **Plan tasks**: S1.15, which this decides; [ADR 0055](0055-verify-a-provider-in-a-test-through-the-run-with-handlers-as-loopback-hooks.md),
  whose run and loopback hooks this builds on; [ADR 0046](0046-ask-a-provider-for-a-message-through-produce-message-and-let-the-transport-name-its-parts.md),
  whose `produce-message` part this answers; [ADR 0049](0049-match-a-content-type-against-the-arrivals-own-octets-and-labels.md),
  whose detection reads an unlabelled message; [ADR 0053](0053-take-todays-verifier-options-where-they-mean-what-they-did.md),
  whose table the facade's option mapping follows; S2.5, whose consumer facade gets the same layer
- **Evidence**: pact-js's provider examples (`examples/*/provider.test.ts`: `Verifier`,
  `MessageProviderPact`, `providerWithMetadata`) and their option types (`src/dsl/verifier/proxy/types.ts`,
  pact-js-core's `VerifierOptions`); pact-jvm's JUnit 5 provider tests (`provider/junit5/src/test`:
  `@State`, `@PactVerifyProvider`, `MessageAndMetadata`, `@TestTemplate` with
  `PactVerificationInvocationContextProvider`); the run's per-interaction events (engine-protocol spec §9)

## Context

S1.14 gave each SDK a provider API: one `verification/verify`, with the test's state handlers and
request filters served as loopback `http` hooks. Two things stop a provider's existing test suite
moving onto it.

- **Messages.** pact-js's `messageProviders` and pact-jvm's `@PactVerifyProvider` methods produce a
  message in the test process. The engine asks for one through a `produce-message` hook, whose answer
  is a whole `message` part (ADR 0046). ADR 0055 left them to this item.
- **The shape of a test.** A pact-js user writes `new Verifier({ providerBaseUrl, pactUrls,
  stateHandlers }).verifyProvider()`. A pact-jvm user writes a `@TestTemplate` that JUnit runs once
  per interaction, with `@State` methods on the test class. Neither spelling is S1.14's.

Today's integrations are also where a run's results become test results. pact-jvm reports one test
per interaction; pact-js reports one test for the whole verification. Janus's run reports one event
per interaction as it verifies it, so either is available without asking the engine for more.

## Decision

1. **A message producer is a loopback `produce-message` hook.** The *n*-th producer declared on a
   verifier is named `message-<n>`. Its entry is `{ name, when: { interaction: <description> },
   changes: [ "parts" ], run }`, with `run` as a state handler's. It follows the configuration's
   `produce-message` entries, as every handler does.
   - The producer receives what today's verifier would POST for the message: the context's
     `interaction.pact-message.request`, else an empty object. That is the description, the provider
     states and a synchronous message's request.
   - It returns a message: a body, and optionally a content type and metadata. The SDK answers
     `{ outcome: "ok", changes: { parts: { message: { metadata, body } } } }`.
     - `body` is `{ content: <base64>, encoded: "base64" }`, plus `content-type` when the producer
       gave one. Bytes are taken as they are, a string as its UTF-8 octets, and anything else as its
       JSON text.
     - `metadata` is `{ content: <object> }`: `contentType` set to the content type when one was
       given, then the producer's metadata over it.
   - A producer that gives no content type leaves the body unlabelled. The engine then reads the
     type as it reads a provider's reply: the metadata's `contentType`, else what the octets are
     detected as (ADR 0049). The SDK sniffs nothing.
   - A message whose metadata has no `contentType` member at all is read as carrying that same type
     there. Today's verifier never sees one: it reads a message from a reply and writes the reply's
     `Content-Type`, or an empty string, into the metadata. A producer in a test has no reply, and
     pact-jvm, whose producers these are, does not hold a message to a `contentType` it was never
     given. Without this, every pact-jvm message pact, which records `contentType` in its metadata,
     would fail on it. The legacy layer captures it (`message_captures`); the `pact-message` format,
     which always writes one, is unaffected.
   - A thrown failure is `failed` with `handler-failed`, as for a state handler.
   - The SDK writes no default producer. A message no producer answers fails as
     `message-not-produced` (ADR 0046), because the provider was never asked. `janus verify` does add
     a default: it has a provider URL to ask, which a test's own producers replace.
2. **A verifier takes the HTTP transport's other options.** `transport` holds the drive options
   besides `base-url`: `headers` and `client` (component-interfaces spec §5.6). They are passed as
   given, beside `base-url`. This is where pact-js's `customProviderHeaders` and
   `disableSslVerification` go. The SDK checks nothing in them; the transport does.
3. **The JUnit 5 integration is an extension that reports each interaction as a test.**
   `JanusProviderExtension` is registered in a static field. A `@TestFactory` returns
   `janus.interactions(this)`.
   - The extension finds the test class's `@State` and `@MessageProducer` methods, on the class, its
     superclasses and its interfaces, and declares each as a handler of the verifier on that
     instance.
     - A `@State` method takes no parameter or the state's parameters as a `Map`. A setup may return
       a `Map` of provider-state values. `action = TEARDOWN` makes it the state's teardown.
     - A `@MessageProducer` method takes no parameter or the request as a `Map`. It returns a
       `Message`, bytes, a string, or a value written as JSON.
   - It starts one run (`Verifier.start()`) and returns a stream of dynamic tests, one per
     `verification/interaction-result` as it arrives.
     - A failed result fails its test with the result's mismatches.
     - A pending failure is aborted: reported, failing nothing.
     - When the run ends `failed` with no failed result to show it, a last test fails with the
       summary. When it ends `filtered`, a last test is aborted, so the narrowing is visible.
   - The run is one run, as ADR 0055 decides, not one per interaction. The stream closes the run
     when JUnit closes it, and the extension closes any run left open after the class.
   - A state with no `@State` method has no hook, so its exchange runs without setup. pact-jvm fails
     such an interaction and pact-js warns. Janus does what the engine does with any state no hook
     handles. The migration note says so.
4. **The JavaScript integration is pact-js's `Verifier`, over S1.14's.** `pact-janus/verifier`
   exports `Verifier`, `MessageProviderPact` and `providerWithMetadata`. They take pact-js's
   options and map each to the protocol's spelling, as S1.13 maps today's CLI options.
   - `stateHandlers`, `messageProviders` and the source, broker and publishing options are adapted.
   - `requestFilter`, an Express middleware, is called with a request object built from the
     request's slots. A middleware may change only the headers, the one slot its entry declares.
   - `beforeEach`, `afterEach`, `providerStatesSetupUrl` and `transports` are refused by name.
     Janus has no point before a state's setup at which to run them, and `verifier.janus.yaml`
     (`config`) states the rest.
   - Options that tuned pact-js's proxy or its logging take effect nowhere, and the migration note
     says so: `logLevel`, `logFile`, `timeout`, `changeOrigin`, `proxyHost`.
   - It is the `facade` layer of the TypeScript SDK's thinness audit, with its own budget. It is the
     layer most likely to grow logic, as S2.5 says of the consumer facade, so it is measured apart.
   - It works under Jest and Vitest alike: it is a promise, and imports neither.
5. **The exit is shown by the examples, ported, in this repository.**
   - Each runnable provider example of pact-js has its pact and a provider answering as the
     example's does in S1.1's corpus (`corpora/verifier/examples/`), so today's verifier and
     `janus verify` are compared on it. The `plugins` example is left out: ADR 0045 refuses a pact
     plugin.
   - So does a set of pact-jvm's JUnit 5 provider tests: HTTP with states and values, parameters,
     annotations on interfaces, and asynchronous, synchronous and metadata-carrying messages.
   - The same pacts are verified by the ported suites in each SDK's own tests, on the integration
     above, against the ported providers, and every interaction must reach the verdict S1.1 records.
   - The examples' providers are rewritten on the language's own HTTP server, so the SDKs gain no
     dependency.

## Alternatives considered

- **pact-jvm's annotations, one run per interaction.** A `@TestTemplate` per interaction, each a run
  filtered to it, is closest to today. But it starts an engine per interaction, and a filter selects
  by description pattern, not identity. It also contradicts ADR 0055 decision 1, which makes a
  verify one run. Streaming the one run's events gives the same report.
- **Keep S1.14's spelling for JavaScript and only document the mapping.** That is cheaper, but every
  pact-js provider test would be rewritten by hand. The plan asks for a `Verifier`-shaped API, and
  the mapping is mechanical: each option is a member of a protocol document.
- **Label every message body.** The SDK could call a string `text/plain` and an object
  `application/json`. A JSON string from `@PactVerifyProvider` would then be text, and today it
  verifies as JSON. Reading an unlabelled body is the engine's job already.
- **A default producer in the SDK, as `janus verify` has.** A test that verifies messages has no
  provider URL to ask; its producers are its own code.

## Consequences

Easier: a pact-js provider test moves by changing its import. A pact-jvm one keeps its `@State`
methods and changes its template for a factory. Both report per interaction where they did before.
A test can produce a message in its own code, as both did.

Found by the examples, and recorded rather than decided here:

- pact-js's `matchers` example uses v4's `statusCode` matcher, which the legacy compiler refuses.
  `janus upgrade` already writes it as `http:status-class`, but no component contributes that
  operator. The disagreement is in `corpora/verifier/disagreements.json` against S1.35, which
  closes it.
- pact-jvm's `ProviderStateInjectedTest` passes there because pact-jvm generates the expected
  response's `LOCATION` header from the provider-state values. Today's `pact_verifier_cli` compares
  the recorded header and fails it, and Janus agrees with today's. The migration note says so.

Harder: the TypeScript SDK has a facade layer to keep thin, and the JVM SDK's integration layer
grows past its budget. A middleware that rewrote a path or a body in pact-js must declare it with
S1.14's `requestFilter` instead. pact-jvm's failure on a state with no method becomes an exchange
that runs without setup, which usually fails on its own.
