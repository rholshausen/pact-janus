# Moving a consumer test to Janus

How a consumer test written with pact-js moves to Janus. Staged plan S2.5a built it, and
[ADR 0066](decisions/0066-let-a-compatibility-facade-submit-todays-builder-calls-and-convert-them-as-upgrade-does.md)
decides how, and S2.5c added pact-js's message consumer API to it. pact-jvm's is S2.5b. A provider
test has its own note: [`provider-test-migration.md`](provider-test-migration.md).

Today's builders record the calls they would have made to `pact_ffi`, and the engine makes those
calls itself. It then converts the interaction they build as `janus upgrade` converts a pact's
([contract-file spec §8](specs/contract-file/spec.md#8-converting-v1v4-pacts)). So a test
records the contract `janus upgrade` would write from the pact pact-js writes today, and
[S1.1's harness](../corpora/verifier/README.md) already holds those to the pact's verdicts. The test
runs once, against the interaction's example, as it does today. A test that wants variants (an
`optional` field shown both present and absent) moves to the Janus DSL.

Each row is one of three things, as ADR 0066 decision 5 decides them:

- **kept**: spelled and behaving as today. A matcher's rule converts with nothing lost.
- **adapted**: the same thing, done differently, as the row says. A matcher's rule converts with a
  judgement the row states.
- **dropped**: refused by name, or a matcher whose rule converts with something lost, with what to do
  instead.

pact-js's own consumer examples run on this facade in this repository's tests
(`sdks/typescript/test/examples`), each beside its provider test. Each records what `janus upgrade`
makes of the pact pact-js wrote, and each provider verifies the contract its consumer test recorded.

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

One difference catches most suites, until S3.4. A header the test wrote without a matcher is compared
as written. pact-js compared a `Content-Type` by its media type and parameters, so a provider that
adds `; charset=utf-8` passed. Against the contract it fails. Write the header the provider sends, or
a `regex` matcher for it. `janus upgrade` converts a pact's header the same way, and S1.1's upgrade
ledger lists each example this affects.

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
| `multipartBody(contentType, file, part, boundary?)` | The same, but with no boundary the engine derives one from the part rather than drawing one at random, so a test records the same bytes on every run. The part's `Content-Type` is the one given, where pact-js guessed one from the file's extension | adapted |
| `willRespondWith(status, builder)`, with `matchStatus(…)` as the status | The same | kept |
| `addGraphQLInteraction()`: `withOperation`, `withVariables`, `withRequest`, `withQuery`, `withMutation` | The same, the query a whitespace-tolerant pattern as pact-js writes it. A query is not parsed: an invalid one is not refused at the call. A parsed document is read from its source text (`loc.source.body`, which `gql` sets) | adapted |
| `pending()` | Accepted, no effect: a contract has no pending interactions, and the broker decides ([ADR 0051](decisions/0051-leave-pending-out-of-the-contract-and-let-upgrade-exclude-pending-interactions.md)) | dropped |
| `comment()`, `testName()`, `reference()` | Accepted, no effect: a contract records none of them | dropped |
| `withCompleteRequest()` | Throws, as pact-js's does | kept |
| `usingPlugin()`, on an interaction or a message, and so `withPluginContents` and `startTransport` | Refused by name: Janus cannot load pact plugins ([ADR 0045](decisions/0045-refuse-an-interaction-that-needs-a-pact-plugin-by-name-and-verify-the-rest.md)) | dropped |
| `PactV3`: `given`, `uponReceiving`, `withRequest`, `willRespondWith`, `addInteraction`, `withRequestBinaryFile`, `withResponseBinaryFile`, `withRequestMultipartFileUpload`, `withResponseMultipartFileUpload`, `withRequestMatchingRules`, `withResponseMatchingRules`, `executeTest` | The same calls, in pact-js's order | kept |
| Several interactions before one `executeTest`, or one interaction called twice | The same: `PactV3`'s `executeTest` serves every interaction added since the last, and `Pact`'s every one added since its last `executeTest`, each as often as the test calls it ([ADR 0069](decisions/0069-keep-an-arming-until-its-variant-is-reported-and-give-an-arrival-to-the-arming-it-matches-best.md)). A request two interactions match goes to the more specific | kept |
| `addAsynchronousInteraction()`: `given`, `expectsToReceive(description, builder)`, `withJSONContent`, `withMetadata`, `withMatchingRules`, `executeTest` | The same calls. `executeTest`'s function is handed the message, `{ contents: { content }, metadata }`, its JSON contents as their document and any other as text, and no mock is started ([ADR 0067](decisions/0067-record-a-message-as-an-emissive-interaction-and-hand-it-to-the-test-when-nothing-delivers-it.md)) | kept |
| A message's `contentType` metadata | Compared as written, where pact-js compared it as a media type, until S3.4. A message that names none has its contents' type checked as today's verifier checks it | adapted |
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
