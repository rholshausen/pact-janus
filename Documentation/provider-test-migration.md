# Moving a provider test to Janus

How a provider that verifies inside its test suite moves to Janus: pact-js's `Verifier` and
`MessageProviderPact`, and pact-jvm's JUnit 5 provider runner. Staged plan S1.15 built both
integrations, and [ADR 0056](decisions/0056-provider-test-integrations-message-producers-a-junit-extension-and-a-pact-js-shaped-verifier.md)
decides them. A provider that verifies with the CLI has its own table:
[`verifier-migration.md`](verifier-migration.md).

Each row is one of three things:

- **kept**: spelled and behaving as today.
- **adapted**: the same thing, spelled or done differently, as the row says.
- **dropped**: refused by name, with what to do instead.

pact-js's and pact-jvm's own provider examples run on these integrations in this repository's tests
(`sdks/typescript/test/examples`, `sdks/jvm/.../PactJvmExamplesTest.java`). Their pacts are
fixtures in [the verifier corpus](../corpora/verifier/README.md), where today's verifier and
`janus verify` reach the same verdicts, apart from the differences recorded below.

## pact-js

Change the import, and nothing else for most tests:

```ts
// before
import { Verifier } from "@pact-foundation/pact";
// after
import { Verifier } from "pact-janus/verifier";

await new Verifier({
  providerBaseUrl: "http://localhost:3001",
  pactUrls: [path.resolve("pacts/UserConsumer-UserProvider.json")],
  stateHandlers: {
    "a user with ID 1 exists": async () => seedUser({ id: 1, name: "Alice" }),
  },
}).verifyProvider();
```

It runs under Jest and Vitest alike: it is a promise, and imports neither. As it runs, it prints the
run's report to standard output, as pact-js prints the verifier's: each interaction, its states and
whether it verified, then a summary. The engine writes it, so it reads as `janus verify`'s does
([ADR 0059](decisions/0059-the-engine-writes-a-verification-runs-report.md)). `require("pact-janus/verifier")`
works as `import` does, so a test suite written in CommonJS changes its `require` the same way. The
engine is the `janus-engine` executable that `JANUS_ENGINE` names, else the one the SDK was released
with, which it fetches on first use ([ADR 0060](decisions/0060-release-each-sdk-as-files-on-a-github-release-of-its-own-and-fetch-its-pinned-engine.md)).
The SDK itself is installed from its [release](https://github.com/rholshausen/pact-janus/releases),
as [`sdks/README.md`](../sdks/README.md#installing-a-release) says.
Underneath it is the SDK's
own `Verifier` (`import { Verifier } from "pact-janus"`), which takes the protocol's spellings directly.

| pact-js | Janus | Verdict |
|---|---|---|
| `new Verifier(options).verifyProvider()` | The same; resolves with the run's result, and rejects with `VerificationFailedError` naming each failed interaction | kept |
| `new MessageProviderPact(options).verify()` | The same | kept |
| `providerBaseUrl` | The HTTP transport's `base-url` | kept |
| `pactUrls` (files) | Read by the SDK and sent inline | kept |
| `pactUrls` (`http`/`https` URLs) | A `urls` source, fetched with the broker credentials below. Files and URLs together are refused: one kind of source per run | adapted |
| `pactBrokerUrl`, `provider` | A `broker` source. With `pactUrls` too, refused: one kind of source per run | adapted |
| `pactBrokerToken`, `pactBrokerUsername`, `pactBrokerPassword` | The source's `auth`. Nothing is read from `PACT_BROKER_*`: a test passes what it wants sent | adapted |
| `consumerVersionSelectors` | Passed as the broker reads them | kept |
| `consumerVersionTags` | A selector `{ tag, latest: true }` per tag, as today's verifier writes them | kept |
| `enablePending`, `includeWipPactsSince` | Asked for only with selectors, which is when today's verifier asks | kept |
| `providerVersionBranch`, `providerBranch` | The source's `provider-branch` and the published result's `branch` | kept |
| `providerVersionTags` | The source's `provider-tags` and the published result's `tags` | kept |
| `failIfNoPactsFound: false` | The source's `require-pacts: false` | kept |
| `publishVerificationResult`, `providerVersion`, `buildUrl` | The run's `publish` option. Publishing without `providerVersion` is refused | kept |
| `consumerFilters` | The run's `filter.consumers` | kept |
| `customProviderHeaders` | The HTTP transport's `headers`, as a map or as `"Name: value"` lines | kept |
| `disableSslVerification`, `validateSSL: false` | The HTTP transport's `client.tls.insecure` | kept |
| `stateHandlers` | A state handler per name, a function or `{ setup, teardown }`. What a setup returns is the provider-state values a `ProviderState` generator reads | kept |
| A state with no handler | The exchange runs with no setup, with a `state-unhandled` warning on standard error and on the result's `warnings`, as pact-js warns ([ADR 0058](decisions/0058-a-provider-state-no-hook-sets-up-is-a-warning-or-a-failure-by-choice.md)). The SDK's own `Verifier` with `options: { "unhandled-states": "fail" }` fails it instead | kept |
| `requestFilter` | An Express-style middleware `(req, res, next)`, called with the request's method, path, URL, headers, query and body. Only the headers it leaves on `req` reach the provider, because a filter must say what it changes before the run (lifecycle-hooks spec §4.3). A middleware that answers the request itself fails the exchange | adapted |
| `messageProviders` | A message producer per description. It receives today's request for the message: its description, provider states, and a synchronous message's request | kept |
| `providerWithMetadata(fn, metadata)` | The message's metadata. A `contentType` in it labels the body. A body with none is JSON, as pact-js's proxy writes it | kept |
| `beforeEach`, `afterEach` | Refused. They ran on the edges of today's state-change calls, and no hook point comes before a state's setup. Do the work in a state handler's setup or teardown | dropped |
| `providerStatesSetupUrl`, `providerStatesSetupTeardown`, `providerStatesSetupBody` | Refused. Use `stateHandlers`, or a `state-setup` hook in `verifier.janus.yaml` (the `config` option), written as [`janus verify -s`](verifier-migration.md) writes one | dropped |
| `transports` | Refused. A message transport is a `produce-message` hook in `verifier.janus.yaml` | dropped |
| `tlsClientOptions` | Not taken. Use the SDK's own `Verifier` with `transport: { client: { tls: { "client-certificate": … } } }` | dropped |
| `logLevel` | How much the engine logs on its standard error, as `LOG_LEVEL` sets it too; `RUST_LOG` overrides both ([ADR 0061](decisions/0061-the-engines-logs-are-the-runtimes-own-set-and-shown-by-its-logging-configuration.md)) | kept |
| `logFile` | Accepted, no effect. The engine logs to its standard error | dropped |
| `timeout` | Accepted, no effect. The test framework's own timeout bounds the test | dropped |
| `changeOrigin`, `proxyHost` | Accepted, no effect. They configured pact-js's proxy, and Janus has none: requests go to the provider directly | dropped |
| `config`, `components`, `engine` | Janus's own: a `verifier.janus.yaml`, declared components, and how the engine is reached | — |

## pact-jvm (JUnit 5)

The verification is one run, reported as one test per interaction as the engine verifies it. A
`@TestFactory` replaces the `@TestTemplate`, and the target is the verifier's provider URL:

```java
// before
@Provider("myAwesomeService")
@PactFolder("pacts")
class ContractTest {
  @TestTemplate
  @ExtendWith(PactVerificationInvocationContextProvider.class)
  void testTemplate(PactVerificationContext context, HttpRequest request) {
    request.addHeader("X-ContractTest", "true");
    context.verifyInteraction();
  }

  @BeforeEach
  void before(PactVerificationContext context) {
    context.setTarget(new HttpTestTarget("localhost", 8080));
  }

  @State("default")
  Map<String, Object> toDefaultState() { return Map.of("ticketId", "1234"); }
}

// after
class ContractTest {
  @RegisterExtension
  static final JanusProviderExtension janus = JanusProviderExtension.of(() -> Verifier
      .forProvider("http://localhost:8080")
      .withPacts(List.of(Path.of("pacts/contract.json")))
      .requestFilter(request -> {
        Map<String, Object> headers = new HashMap<>((Map<String, Object>) request.get("headers"));
        headers.put("X-ContractTest", List.of("true"));
        return Map.of("headers", headers);
      }));

  @TestFactory
  Stream<DynamicTest> testTemplate() {
    return janus.interactions(this);
  }

  @State("default")
  Map<String, Object> toDefaultState() { return Map.of("ticketId", "1234"); }
}
```

| pact-jvm | Janus | Verdict |
|---|---|---|
| `@TestTemplate` with `PactVerificationInvocationContextProvider`, `context.verifyInteraction()` | A `@TestFactory` returning `janus.interactions(this)`. It is one run, and each interaction result becomes a dynamic test as it arrives, named `<consumer>: <description>`, with the variant when it is not the base | adapted |
| A failed interaction | Its test fails with the result's mismatches, or its error | kept |
| A pending pact's failure | Its test is aborted: reported, and failing nothing | adapted |
| A filtered run | A last test, `verification`, is aborted: not every variant was verified (ADR 0041) | adapted |
| `@Provider` | Not needed: the pacts the verifier names are the ones verified. A broker source names its provider | adapted |
| `@PactFolder`, `@PactSource` of files | `withPacts`, naming each file | adapted |
| `@PactBroker`, `@PactBrokerAuth`, `@VersionSelector`, `pactbroker.*` properties | `withBroker`, as engine-protocol spec §8.3 spells a broker source. Nothing is read from system properties or the environment | adapted |
| `pact.verifier.publishResults`, `pact.provider.version`, `pact.provider.branch` | `withOptions(Map.of("publish", …))` | adapted |
| `@PactFilter`, `pact.filter.*` | `withOptions(Map.of("filter", …))`, as `janus verify`'s filters | adapted |
| `context.setTarget(new HttpTestTarget(host, port))` | `Verifier.forProvider(url)` | adapted |
| `HttpsTestTarget(…, insecure)` | `withTransport(Map.of("client", Map.of("tls", Map.of("insecure", true))))` | adapted |
| `MessageTestTarget` | Not needed: a message is asked of the `@ProvidesMessage` methods. The verifier still takes a provider URL, which a messages-only run never reaches | adapted |
| `@State(value, action)` | `io.pact.janus.sdk.State`, on the test class, its superclasses or its interfaces' default methods. A method takes no parameter or the state's parameters as a `Map`, and a setup may return a `Map` of provider-state values. `action = TEARDOWN` is the teardown | kept |
| `context.withStateHandlers(...)`, state classes | `Verifier.state(name, setup, teardown)` on the verifier the extension runs | adapted |
| A state with no `@State` method | The exchange runs with no setup and a `state-unhandled` warning, where pact-jvm fails the interaction ([ADR 0058](decisions/0058-a-provider-state-no-hook-sets-up-is-a-warning-or-a-failure-by-choice.md)). `withOptions(Map.of("unhandled-states", "fail"))` fails it as pact-jvm does | adapted |
| `@PactVerifyProvider(description)` | `@ProvidesMessage(description)`. A method takes no parameter or today's request for the message as a `Map`, which carries a synchronous message's request | kept |
| A `String`, `byte[]` or `MessageAndMetadata` returned | A `String`, `byte[]`, or `Message.of(bytes).withMetadata(map)`. Any other value is written as JSON | adapted |
| A message body with no content type | Read by the engine from its metadata's `contentType`, else from its octets. pact-jvm labels it with the type the pact expects. A JSON body reads as JSON either way | adapted |
| A request changed in the template (`request.addHeader`), `@TargetRequestFilter` | A request filter on the verifier: `requestFilter(request -> …)` receives the request's slots and returns the ones it changes, the headers by default | adapted |
| `@IgnoreNoPactsToVerify` | A broker source's `require-pacts: false` | adapted |
| pact-jvm's logs, through SLF4J | The engine's, through SLF4J too, to the logger `io.pact.janus.engine`, whose level is how much the engine logs: `logging.level.io.pact.janus.engine=debug` under Spring Boot ([ADR 0061](decisions/0061-the-engines-logs-are-the-runtimes-own-set-and-shown-by-its-logging-configuration.md)) | kept |

## Differences from today's verdicts

- **A `ProviderState` generator on the expected response.** pact-jvm generates the expected response
  from the provider-state values too, so a `Location` header recorded as `/users/666` with a
  generator from `${userId}` verifies against `/users/666554433`. Today's `pact_verifier_cli`, which
  pact-js uses, compares the recorded value and fails it, and Janus agrees with today's
  (`corpora/verifier/examples/pact-jvm/provider-state-injected`).
