# Moving a consumer test to Janus

How a consumer test written with pact-js or pact-jvm moves to Janus. Staged plan S2.5a built
pact-js's, and
[ADR 0066](decisions/0066-let-a-compatibility-facade-submit-todays-builder-calls-and-convert-them-as-upgrade-does.md)
decides how; S2.5c added pact-js's message consumer API to it. S2.5b built pact-jvm's, for its Java
DSL under JUnit 5, as
[ADR 0070](decisions/0070-port-pact-jvms-java-dsl-into-the-facade-and-submit-what-it-builds-as-ffi-calls.md)
decides. A provider test has its own note: [`provider-test-migration.md`](provider-test-migration.md).

Today's builders hand the engine the calls `pact_ffi` would make to build their interaction:
pact-js's record the calls they make, and pact-jvm's, which call no FFI, build what pact-jvm builds
and submit it as those calls. The engine makes the calls itself, then converts the interaction they
build as `janus upgrade` converts a pact's
([contract-file spec §8](specs/contract-file/spec.md#8-converting-v1v4-pacts)). So a test
records the contract `janus upgrade` would write from the pact the library writes today, and
[S1.1's harness](../corpora/verifier/README.md) already holds those to the pact's verdicts. The test
runs once, against the interaction's example, as it does today. A test that wants variants (an
`optional` field shown both present and absent) moves to the Janus DSL.

One thing a recorded contract does differently from a pact: it never holds a credential
([ADR 0074](decisions/0074-redact-credentials-from-what-the-engine-records-and-reports.md)). An
`Authorization`, `Proxy-Authorization` or `Cookie` request header, or a `Set-Cookie` response
header, is recorded as `"[redacted]"`, and the contract lists it under `redacted`
([contract-file spec §5.6](specs/contract-file/spec.md#56-redacted-members)). The test itself runs as
before: the mock still matches the header the test declared. A provider then verifies with a token
of its own, through a request filter or `--header`, as pact's advice has always been. A test whose
provider accepts one fixed fake token keeps it: the Janus DSL's `redaction: { keep:
["request.headers.authorization"] }`, or `withKept` in the JVM SDK. `add` names other members to
redact, an `x-api-key` header or a body's `password`. The facades take no option: they redact the
defaults, as `janus upgrade` does.

Each row is one of three things, as ADR 0066 decision 5 decides them:

- **kept**: spelled and behaving as today. A matcher's rule converts with nothing lost.
- **adapted**: the same thing, done differently, as the row says. A matcher's rule converts with a
  judgement the row states.
- **dropped**: refused by name, or a matcher whose rule converts with something lost, with what to do
  instead.

Every **kept** row is a case of the SDK conformance suite: a test written as a user of the facade
writes one, under [`conformance/cases/facade/`](../conformance/cases/facade), which the SDK that ships the
facade runs in its build. The suite's lint reads the rows here, and fails a `kept` row no case names
Each case pins the contract its test recorded, read against the row, so a
rule that stops converting with nothing lost fails it ([ADR 0076](decisions/0076-write-a-facades-conformance-cases-as-tests-in-its-own-language.md)).

pact-js's own consumer examples run on its facade in this repository's tests
(`sdks/typescript/test/examples`), each beside its provider test. Each records what `janus upgrade`
makes of the pact pact-js wrote, and each provider verifies the contract its consumer test recorded.
pact-jvm's own JUnit 5 consumer tests run on its facade (`sdks/jvm/facade/src/test`), their imports
changed, and each contract they record is compared with what `janus upgrade` makes of the pact
pact-jvm 4.7.8 wrote running the same test.

## pact-js

Change the import, and nothing else for most tests:

```ts
// before
import { Matchers, Pact } from "@pact-foundation/pact";
// after
import { Matchers, Pact } from "pact-janus/pact";

const { like, integer, eachLike } = Matchers;

const provider = new Pact({ consumer: "UserConsumer", provider: "UserProvider", dir: "pacts" });

it("fetches a user by ID", () =>
  provider
    .addInteraction()
    .given("a user with ID 1 exists")
    .uponReceiving("a GET request for user 1")
    .withRequest("GET", "/users/1", (builder) => builder.headers({ Accept: "application/json" }))
    .willRespondWith(200, (builder) => {
      builder.headers({ "Content-Type": "application/json" });
      builder.jsonBody(like({ id: integer(1), name: like("Alice") }));
    })
    .executeTest(async (mockserver) => {
      const user = await new UserServiceClient(mockserver.url).getUser(1);
      expect(user.id).toBe(1);
    }));
```

`pact-janus/pact` exports pact-js's names: `Pact`, `PactV4`, `PactV3`, `Matchers` (`MatchersV3`),
`SpecificationVersion`, `XmlBuilder`, and the provider side's `Verifier`, `MessageProviderPact` and
`providerWithMetadata`, which `pact-janus/verifier` exports too. It runs under Jest and Vitest alike,
and `require("pact-janus/pact")` works as `import` does. Each `executeTest` starts the engine
(`JANUS_ENGINE`, else the one the SDK was released with) and ends it. When a conversion loses
something, the test prints the engine's finding as a warning, prefixed `pact-janus:`, unless
`logLevel` is `error`.

A `Content-Type` the test wrote without a matcher is compared as a media type, as pact-js compared it:
a provider, or a consumer's client, that adds `; charset=utf-8` passes (S2.16, [ADR
0071](decisions/0071-compare-a-content-type-as-a-media-type-with-the-http-components-http-media-type.md)).
The contract says so with the HTTP component's `http:media-type`. Any other header the test wrote
without a matcher is compared as written.

### The builders and their options

| pact-js | Janus | Verdict |
|---|---|---|
| `new Pact(options)`, `new PactV4(options)`, `new PactV3(options)` | The same | kept |
| `consumer`, `provider` | The contract's consumer and provider | kept |
| `dir` | Where the contract is written, as `<consumer>-<provider>.janus.json`: a Janus contract, not a pact. Default `./pacts`. Test files recording one pair merge into one file, as pact-js's do ([ADR 0065](decisions/0065-let-the-engines-host-write-and-merge-contract-files-in-identity-order.md)). A run should clear the directory first | adapted |
| `spec` | The pact version the calls build to. From V3, a binary body gets a content-type rule, as pact-js's does | kept |
| `logLevel` | How much the engine logs, as `LOG_LEVEL` sets it too; `RUST_LOG` overrides both | kept |
| `host`, `port` | The mock's address | kept |
| `tls: true` | Refused: the mock serves plain HTTP | dropped |
| `cors` | Accepted, no effect. The mock answers no CORS preflight, which only a browser sends | dropped |
| `addInteraction()` … `.executeTest(fn)` | The same. The test runs once, against the interaction's example, and `executeTest` resolves with what it returned | kept |
| A test that throws | Its error is rethrown, what the mock found is printed beside it, and no contract is written | kept |
| A request that was missing or did not match | `executeTest` rejects, "Test failed for the following reasons", naming the interaction or each mismatch, and no contract is written | kept |
| `given(state, params?)`, `uponReceiving(description)` | The same | kept |
| `withRequest(method, path, builder)`: `query`, `headers`, `jsonBody`, `xmlBody`, `binaryFile`, `body`, `matchingRules` | The same calls | kept |
| `multipartBody(contentType, file, part, boundary?)`, and `PactV3`'s `withRequestMultipartFileUpload`, `withResponseMultipartFileUpload` | The same calls, and the same bytes on every run: with no boundary the engine derives one from the part rather than drawing one at random. The part's `Content-Type` is the one given, where pact-js guessed one from the file's extension. The contract asserts nothing about the body's parts (`body-not-parsed`), since `pact_ffi` writes the body's type without its boundary, which only the `Content-Type` header names. The test runs as before; until S2.20 reads the boundary from the header, a provider is held to the header's pattern alone | dropped |
| `willRespondWith(status, builder)`, with `matchStatus(…)` as the status | The same | kept |
| `addGraphQLInteraction()`: `withOperation`, `withVariables`, `withRequest`, `withQuery`, `withMutation` | The same, the query a whitespace-tolerant pattern as pact-js writes it. A query is not parsed: an invalid one is not refused at the call. A parsed document is read from its source text (`loc.source.body`, which `gql` sets) | adapted |
| `pending()` | Accepted, no effect: a contract has no pending interactions, and the broker decides ([ADR 0051](decisions/0051-leave-pending-out-of-the-contract-and-let-upgrade-exclude-pending-interactions.md)) | dropped |
| `comment()`, `testName()`, `reference()` | Accepted, no effect: a contract records none of them | dropped |
| `withCompleteRequest()` | Throws, as pact-js's does | kept |
| `usingPlugin()`, on an interaction or a message, and so `withPluginContents` and `startTransport` | Refused by name: Janus cannot load pact plugins ([ADR 0045](decisions/0045-refuse-an-interaction-that-needs-a-pact-plugin-by-name-and-verify-the-rest.md)) | dropped |
| `PactV3`: `given`, `uponReceiving`, `withRequest`, `willRespondWith`, `addInteraction`, `withRequestBinaryFile`, `withResponseBinaryFile`, `withRequestMatchingRules`, `withResponseMatchingRules`, `executeTest` | The same calls, in pact-js's order | kept |
| Several interactions before one `executeTest`, or one interaction called twice | The same: `PactV3`'s `executeTest` serves every interaction added since the last, and `Pact`'s every one added since its last `executeTest`, each as often as the test calls it ([ADR 0069](decisions/0069-keep-an-arming-until-its-variant-is-reported-and-give-an-arrival-to-the-arming-it-matches-best.md)). A request two interactions match goes to the more specific | kept |
| `addAsynchronousInteraction()`: `given`, `expectsToReceive(description, builder)`, `withJSONContent`, `withMetadata`, `withMatchingRules`, `executeTest` | The same calls. `executeTest`'s function is handed the message, `{ contents: { content }, metadata }`, its JSON contents as their document and any other as text, and no mock is started ([ADR 0067](decisions/0067-record-a-message-as-an-emissive-interaction-and-hand-it-to-the-test-when-nothing-delivers-it.md)) | kept |
| A message's `contentType` metadata | Compared as a media type, as pact-js compared it (ADR 0071). A message that names none has its contents' type checked as today's verifier checks it | kept |
| `withContent(contentType, buffer)` on a message, `MessageConsumerPact`'s `withBinaryContent` | The same calls: the test is handed the octets, as a `Buffer` | kept |
| `addSynchronousInteraction(description)`: `given`, `withRequest(builder)`, `withResponse(builder)`, `withJSONContent`, `withContent`, `withMatchingRules`, `executeTest` | The same calls. `executeTest`'s function is handed `{ Request: { content }, Response: [{ content }] }`, each a `Buffer`, with each generator's fresh value, where pact-js handed the contents as written | adapted |
| A synchronous message's response `withMetadata` | Accepted, no effect, as pact-js's: `pact_ffi` writes no synchronous message's metadata | kept |
| A synchronous message with several responses | The contract keeps the first, reported as `responses-dropped`, since today's verifier fails a message expecting more than one | dropped |
| `MessageConsumerPact`: `given`, `expectsToReceive`, `withContent`, `withTextContent`, `withMetadata`, `verify(handler)` | The same calls, a V3 message pact's unless `spec` says otherwise. The handler is handed `{ description, providerStates, contents, metadata }`, and the contract is written when it resolves. `pactfileWriteMode` is accepted: contracts are always merged ([ADR 0065](decisions/0065-let-the-engines-host-write-and-merge-contract-files-in-identity-order.md)) | kept |
| `MessageConsumerPact.json()` | Not provided: it returned pact-js's own state | dropped |
| `synchronousBodyHandler`, `asynchronousBodyHandler`, `v4SynchronousBodyHandler`, `v4AsynchronousBodyHandler` | The same | kept |
| `PactV2`, `MatchersV2` | Not provided: they are pact-js's older API. Move to `PactV3` or `Pact` first | dropped |

### The matchers

Each matcher returns the data pact-js's returns, so `reify`, `extractPayload` and `isMatcher` work as
they do. Its verdict is what the conversion does with the rule it writes.

| pact-js | The contract | Verdict |
|---|---|---|
| `like(v)` on a value, `string`, `boolean` | `type` | kept |
| `like(v)` on an object | The object written out, `type` at each position the rule reaches, a nested list an `each-like` of its first element with `min: 0` | kept |
| `like(v)` on a list | `each-like` of its first element with `min: 1`, where pact-js admitted an empty list. Use `atLeastLike(v, 0)` if an empty list is a case the consumer handles | adapted |
| `eachLike(v, min)`, `atLeastOneLike`, `atLeastLike`, `constrainedArrayLike` | `each-like` with `min`, and `max` where given | kept |
| `atMostLike(v, max)` | `each-like` with `min: 0` and `max` | kept |
| `integer`, `decimal`, `number` | The kind. With no example, a random value on every reply, as pact-js's mock serves one | kept |
| `regex`, `email`, `ipv4Address`, `ipv6Address`, `iso8601Date`, `iso8601DateTime`, `iso8601DateTimeWithMillis`, `iso8601Time`, `rfc1123Timestamp`, `hexadecimal`, `uuid` | `regex`, anchored, since pact-js's patterns match the whole value (S1.18) | kept |
| `datetime`, `timestamp`, `date`, `time` | `datetime`, `date`, `time`, with the format | kept |
| `includes`, `nullValue`, `equal`, `contentType` | `include`, `null`, `equality`, `content-type` | kept |
| `eachKeyLike`, `eachValueMatches` | `each-entry` | kept |
| `eachKeyMatches(example, rules)` | `each-entry` with the keys held to the rules and the values open: pact-js also compared each member the example names, which `each-entry` cannot say. Nothing checks the values yet | dropped |
| `arrayContaining(…)` | `contains`, which needs a distinct element for each variant, where pact-js let one element satisfy two | adapted |
| `matchStatus(status, class or list)` | `http:status-class` | kept |
| `fromProviderState(expression, example)` | `type`, with a `ProviderState` generator a verifier fills from the state's values | kept |
| `url`, `url2` | `regex`, as pact-js's. The mock serves the example URL, not its own address, since no generator writes the mock's URL yet. A `RegExp` fragment is refused, since pact-js generated its example with `randexp`: pass `regex(pattern, example)` | dropped |
| `matchingRules(rules)` and PactV3's `with*MatchingRules` | Converted as a pact's rules at those paths are | kept |
| `reify`, `extractPayload`, `isMatcher` | The same | kept |

## pact-jvm (JUnit 5)

Add `io.pact.janus:janus-facade` beside `janus-sdk`, from the same release (ADR 0060), and replace
`au.com.dius.pact.` with `io.pact.janus.facade.` in the suite's imports. Nothing else changes for a
test written in the Java DSL:

```java
// before
import au.com.dius.pact.consumer.MockServer;
import au.com.dius.pact.consumer.dsl.PactDslJsonBody;
import au.com.dius.pact.consumer.dsl.PactDslWithProvider;
import au.com.dius.pact.consumer.junit5.PactConsumerTestExt;
import au.com.dius.pact.consumer.junit5.PactTestFor;
import au.com.dius.pact.core.model.RequestResponsePact;
import au.com.dius.pact.core.model.annotations.Pact;
// after
import io.pact.janus.facade.consumer.MockServer;
import io.pact.janus.facade.consumer.dsl.PactDslJsonBody;
import io.pact.janus.facade.consumer.dsl.PactDslWithProvider;
import io.pact.janus.facade.consumer.junit5.PactConsumerTestExt;
import io.pact.janus.facade.consumer.junit5.PactTestFor;
import io.pact.janus.facade.core.model.RequestResponsePact;
import io.pact.janus.facade.core.model.annotations.Pact;

@ExtendWith(PactConsumerTestExt.class)
@PactTestFor(providerName = "UserProvider")
class UserClientTest {
  @Pact(consumer = "UserConsumer")
  RequestResponsePact aUser(PactDslWithProvider builder) {
    return builder
        .given("a user with ID 1 exists")
        .uponReceiving("a GET request for user 1")
        .path("/users/1")
        .method("GET")
        .willRespondWith()
        .status(200)
        .body(new PactDslJsonBody().integerType("id", 1).stringType("name", "Alice"))
        .toPact();
  }

  @Test
  void fetchesAUser(MockServer mockServer) throws IOException {
    User user = new UserServiceClient(mockServer.getUrl()).getUser(1);
    assertEquals(1, user.id());
  }
}
```

The facade's classes are pact-jvm's Java DSL, ported with their names (ADR 0070 decision 4): each
builds the example, rules and generators pact-jvm's builds, and the facade depends on nothing of
pact-jvm's. Each test starts the engine (`JANUS_ENGINE`, else the one the SDK was released with)
and ends it. When a conversion loses something, the test prints the engine's finding to stderr,
prefixed `pact-janus:` and naming the interaction.

A `Content-Type` the test wrote without a matcher is compared as a media type, as pact-jvm compared
it: the same type, in any case, with each parameter it names, and others allowed. So a client's
`application/json; charset=UTF-8` meets a pact's `application/json`, and a client's `application/json`
does not meet the `application/json; charset=UTF-8` pact-jvm's builders write for a JSON body, in
pact-jvm's mock or the facade's (S2.16,
[ADR 0071](decisions/0071-compare-a-content-type-as-a-media-type-with-the-http-components-http-media-type.md)).
Any other header the test wrote without a matcher is compared as written.

### The extension and its annotations

| pact-jvm | Janus | Verdict |
|---|---|---|
| `@ExtendWith(PactConsumerTestExt.class)`, `@PactConsumerTest` | The same | kept |
| `@Pact(consumer, provider)` on a method taking `PactDslWithProvider` or `MessagePactBuilder`, returning `RequestResponsePact`, `MessagePact` or `V4Pact` | The same | kept |
| A `@Pact` method taking pact-jvm's V4 `PactBuilder` or `SynchronousMessagePactBuilder` | Refused by name: those builders are staged plan S2.5d | dropped |
| `@PactTestFor(providerName, pactMethod, pactMethods, pactVersion, providerType)`, on the class or the method | The same. `pactVersion` is the pact version the builders build to, V4 unless named | kept |
| `@PactTestFor(hostInterface, port)`, `@MockServerConfig(hostInterface, port, providerName)`, `@MockServers` | The mock's address, `${name}` read from a system property, else the environment, as pact-jvm reads it. An IPv6 mock's URL has its brackets | kept |
| Several providers in one test: `pactMethods` naming each provider's methods, `@ForProvider` on each parameter | The same: one mock for each provider, each recording its own contract | kept |
| `@PactTestFor(https = true)`, its `keyStore*` options, `@MockServerConfig(tls, keyStore*)` | Refused by name: the mock serves plain HTTP | dropped |
| `@PactTestFor(mockServerImplementation)`, `@MockServerConfig(implementation)` | Refused by name: the engine runs its own mock | dropped |
| `@MockServerConfig(registryEntry, transportConfig)` | Accepted, no effect: they configure a pact plugin's transport, and Janus cannot load pact plugins ([ADR 0045](decisions/0045-refuse-an-interaction-that-needs-a-pact-plugin-by-name-and-verify-the-rest.md)) | dropped |
| The injected `MockServer` | `getUrl()` and `getPort()`. The members pact-jvm's extension calls itself, such as `validateMockServerState`, are not provided | adapted |
| The injected `RequestResponsePact` or `V4Pact` | The same | kept |
| A message test's `List<Message>` (a V4 pact's `List<V4Interaction.AsynchronousMessage>`), `MessagePact` or `V4Pact` | The same: each message as its builder built it, its metadata keeping its values' own types. No mock is started ([ADR 0067](decisions/0067-record-a-message-as-an-emissive-interaction-and-hand-it-to-the-test-when-nothing-delivers-it.md)) | kept |
| The test runs once, against each interaction's example | The same. Every interaction of the test's pact is served, each as often as the test calls it, and a request two interactions match goes to the more specific ([ADR 0069](decisions/0069-keep-an-arming-until-its-variant-is-reported-and-give-an-arrival-to-the-arming-it-matches-best.md)) | kept |
| A request that did not match, or an interaction never called | The test fails with pact-jvm's message, "The following mismatched requests occurred" or "The following requests were not received", and no contract is written | kept |
| A test that throws | Its error is the test's failure, what the mock found is printed beside it, and no contract is written ([ADR 0064](decisions/0064-report-each-variants-test-verdict-to-the-engine-and-answer-with-its-result.md)) | kept |
| `@Pact` methods no test ran | The class fails, naming them, unless the method or its test is `@Disabled` | kept |
| `@PactIgnore` | The same: the extension leaves the test alone | kept |
| `@PactDirectory`, `pact.rootDir`, else `build/pacts` under Gradle and `target/pacts` otherwise | Where the contract is written, as `<consumer>-<provider>.janus.json`: a Janus contract, not a pact. Test classes recording one pair merge into one file, as pact-jvm's do, and an interaction recorded two ways is a `contract-conflict` ([ADR 0065](decisions/0065-let-the-engines-host-write-and-merge-contract-files-in-identity-order.md)). `pact.writer.overwrite` is not read. A run should clear the directory first | adapted |
| JUnit 4: `PactProviderRule`, `PactProviderRuleMk2`, `@PactVerification`, `ConsumerPactTest` | Not provided: JUnit 4 is not supported (S2.4). Move the test to JUnit 5's `PactConsumerTestExt` first | dropped |
| `ConsumerPactRunnerKt.runConsumerTest` | Not provided: run the test under `PactConsumerTestExt` | dropped |

### The builders

| pact-jvm | Janus | Verdict |
|---|---|---|
| `ConsumerPactBuilder.consumer(name).hasPactWith(provider)`, `pactSpecVersion` | The same | kept |
| `given(state)`, `given(state, params)`, several states | The same | kept |
| `uponReceiving`, `path`, `method`, `query`, `encodedQuery`, `headers`, `setDefaultRequestValues`, `setDefaultResponseValues` | The same | kept |
| `matchPath`, `matchHeader`, `matchQuery`, `queryMatchingDate`, `queryMatchingTime`, `queryMatchingDatetime` and their ISO forms, given an example | `regex`, anchored, and the date and time rules with their format | kept |
| The same with no example, and `stringMatcher(name, regex)` with none | The example is drawn from the pattern as pact-jvm draws it, but seeded by the pattern, so every run records the same example where pact-jvm drew a new one (ADR 0070 decision 3) | adapted |
| `body(...)`: a `DslPart`, a `String` with a content type or without, an XML `Document`; `bodyWithSingleQuotes` | The same. A body no generator fills is served as the octets pact-jvm wrote, so a test comparing its text sees pact-jvm's | kept |
| `body(JSONObject)`, `withContent(JSONObject)`, and a content type given as HttpCore's `ContentType` | Not taken: the facade depends on neither org.json nor HttpCore. Pass the object's `toString()`, and the content type as a `String` | dropped |
| `body(FormPostBuilder)` | A form body, its rules and generators by parameter ([ADR 0038](decisions/0038-read-form-bodies-as-a-query-string-through-a-built-in-form-component.md)) | kept |
| `body(PactXmlBuilder)` | An XML body, with its attributes, `eachLike` and namespaces ([ADR 0039](decisions/0039-read-xml-bodies-by-element-name-through-a-built-in-xml-component.md)) | kept |
| `withBinaryData(bytes, contentType)` | The same: served and recorded as octets | kept |
| `withFileUpload(partName, fileName, contentType, bytes)` | The same, but with a boundary derived from the part rather than drawn at random, so a test records the same bytes on every run | adapted |
| `bodyMatchingContentType(contentType, example)` | `content-type`, checked against the octets ([ADR 0049](decisions/0049-match-a-content-type-against-the-arrivals-own-octets-and-labels.md)). Over a JSON example the rule is pact-jvm's at `$`, which reaches each value, as a v1–v4 rule at `$` does | kept |
| `headerFromProviderState`, `pathFromProviderState`, `queryParameterFromProviderState` | The example, with a `ProviderState` generator a verifier fills from the state's values | kept |
| `willRespondWith()`: `status`, `headers`, `matchHeader`, `matchSetCookie`, `body` and the rest as for the request | The same | kept |
| `successStatus()`, `clientErrorStatus()`, `serverErrorStatus()`, `nonErrorStatus()`, `errorStatus()`, `informationStatus()`, `redirectStatus()`, `statusCodes(list)` | `http:status-class` | kept |
| Several interactions in one pact, chained by `given` or `uponReceiving` after `willRespondWith` | The same | kept |
| `comment()`, `addMetadataValue()` | Accepted, no effect: a contract records neither | dropped |
| `MessagePactBuilder`: `consumer`, `hasPactWith`, `given`, `expectsToReceive`, `withContent` (a `DslPart`, a `PactXmlBuilder`, or a `String` with a content type), `withMetadata` (a `Map` or a `MetadataBuilder`) | The same calls, a V3 message pact's unless `pactVersion` says otherwise | kept |
| A message's `contentType` metadata | Compared as a media type, as pact-jvm compared it (ADR 0071) | kept |
| `PactDslJsonBody.pluginValue()` | Not provided: Janus cannot load pact plugins (ADR 0045) | dropped |
| pact-jvm's V4 `PactBuilder`, `MultipartBuilder`, and its Groovy, Spock and Kotlin DSLs | Not yet: staged plan S2.5d | dropped |

### The matchers

`PactDslJsonBody`, `PactDslJsonArray`, `PactDslJsonRootValue`, `PactDslRootValue`, the `LambdaDsl`
over them and `MetadataBuilder` write pact-jvm's rules and generators. A row's verdict is what the
conversion does with them.

| pact-jvm | The contract | Verdict |
|---|---|---|
| `stringType`, `like`, `numberType`, `booleanType`, and their plural forms | `type`, or `number` and `boolean` | kept |
| `integerType`, `decimalType` | `integer` and `decimal`, a number's kind read from how it is written, as pact-jvm reads it ([ADR 0072](decisions/0072-read-a-numbers-kind-from-how-it-is-written.md)) | kept |
| Any of these with no example | pact-jvm's fixed example, as `100` for `integerType`, with its random generator, which the mock fills fresh on every reply ([ADR 0048](decisions/0048-carry-generators-on-shapes-and-fill-fresh-values-where-a-value-is-produced.md)) | kept |
| `stringValue`, `numberValue`, `booleanValue`, `equalTo`, and a member with no rule | `equality` | kept |
| `stringMatcher`, `uuid`, `hexValue`, `ipAddress` | `regex`, anchored, since pact-jvm's patterns match the whole value (S1.18) | kept |
| `integerMatching`, `decimalMatching`, `numberMatching` | The number's kind and the anchored `regex`, both held | kept |
| `date`, `time`, `datetime`, `timestamp`, `localDate`, and `dateExpression`, `timeExpression`, `datetimeExpression` | `date`, `time`, `datetime`, with the format. Generators, an expression's too, are filled as pact-jvm fills them | kept |
| `includesStr`, `nullValue` | `include`, and an `equality` with `null`: pact-jvm writes no rule for a null | kept |
| `valueFromProviderState` | `type`, with a `ProviderState` generator a verifier fills from the state's values | kept |
| `eachLike`, `minArrayLike`, `maxArrayLike`, `minMaxArrayLike`, `eachArrayLike` and its bounded forms | `each-like`, with `min` and `max` where given | kept |
| `array()` with no rule | An exact `array`, as pact-jvm compared it | kept |
| `arrayContaining(…)` | `contains`, which needs a distinct element for each variant, where pact-jvm let one element satisfy two | adapted |
| `eachKeyLike`, `eachValueLike`, `eachKeyMappedToAnArrayLike` | `each-entry` | kept |
| `eachValueMatching` | The example's entry, with the rules under its `*` path: pact-jvm and today's verifier apply them to the keys the example names, and allow others | kept |
| `eachKeyMatching(matcher)` | `each-entry` with the keys held to the rule and the values open: pact-jvm also compared each member the example names, which `each-entry` cannot say | dropped |
| `and(name, value, matchers…)` whose rules one subsumes, as a `regex` does a string's `type` | That rule, which admits only what all of them do | kept |
| `and(name, value, matchers…)` whose rules none subsumes | Its first rule, reported `rule-unmapped`: a shape has one operator, and none says what both do ([contract-file spec §8.2](specs/contract-file/spec.md#82-matching-rules-become-shapes)). Write the one rule that says it all, or check the rest in the test | dropped |
| `or(name, value, matchers…)` | The first rule alone, reported as `rule-combination-or`: a shape has no union of rules yet | dropped |
| `unorderedArray`, `unorderedMinArray`, `unorderedMaxArray`, `unorderedMinMaxArray` and their `newUnordered…` forms | `pact:ignore-order`, an operator nothing provides, reported as `rule-unmapped`. Today's Rust verifier fails pact-jvm's `ignore-order` rule as a plugin it does not have, so the contract fails it by name too: the mock refuses a request that uses it, and a provider's verification fails a response that does ([ADR 0073](decisions/0073-name-a-rule-nobody-implements-as-the-pact-components-operator-and-fail-by-it.md)). Rewrite it with `eachLike`, or `arrayContaining` if each element must be there | dropped |
| `matchUrl`, `matchUrl2` | `regex`, as pact-jvm's. The mock serves the example URL, not its own address, since no generator writes the mock's URL yet: the finding says so | dropped |
| `PactDslRootValue` (a text body) under a text `Content-Type`, `PactDslJsonRootValue` (a JSON value at the root) | The same rules at the root | kept |
| `PactDslRootValue` with no `Content-Type` | pact-jvm labels the text JSON, which it is not, so the contract asserts nothing about it (`body-not-parsed`). Name its type with `headers` | dropped |
