# 0070 — Port pact-jvm's Java DSL into the facade, and submit what it builds as `pact_ffi` calls

- **Status**: accepted
- **Date**: 2026-11-05
- **Plan tasks**: S2.5b; S2.5d builds on it
- **Evidence**: pact-jvm at `1928f2247` (4.7.8): its consumer DSL (`consumer/src/main/kotlin/au/com/dius/pact/consumer/dsl/`,
  `consumer/src/main/java/au/com/dius/pact/consumer/dsl/LambdaDsl*.java`, `MessagePactBuilder.kt`,
  `xml/PactXmlBuilder.kt`), its JUnit 5 extension (`consumer/junit5/src/main/`) and that module's
  tests, which are S2.5b's exit; the `consumer` artifact's dependencies (`consumer/build.gradle`);
  pact-reference's `pact_ffi` at `1f1c2e61` (`src/mock_server/handles.rs`: `pactffi_with_binary_body`,
  `pactffi_with_generators`); [ADR 0066](0066-let-a-compatibility-facade-submit-todays-builder-calls-and-convert-them-as-upgrade-does.md)

## Context

S2.5b is pact-jvm's compatibility facade, for the Java DSL under JUnit 5. ADR 0066 decided that a
facade's conversion is `janus upgrade`'s: the facade hands the engine a v1–v4 interaction as a
consumer library builds it, as `add-pact-interaction`'s `pact_ffi` calls, and the engine converts it.
pact-js builds its interactions by calling the FFI, so its facade records those calls.

pact-jvm calls no FFI. Its DSL builds pact-jvm's own model: each method puts an example value in the
body, and adds a matching rule and perhaps a generator at a path, by category, as the pact file
writes them. `PactDslJsonBody.integerType("id", 100)` puts `100` at `id` and adds
`{ "match": "integer" }` at `$.id`. The rules and generators are what pact-jvm writes into the pact.
So they are what `janus upgrade` reads when it converts that pact, and what S1.1's harness holds to
the pact's verdicts.

There are three ways to get them to the engine. The facade can port the DSL and send the rules it
builds. It can port the DSL but have each method write `pact_ffi`'s integration JSON, which the
engine's port of the FFI already turns into rules. Or it can depend on pact-jvm's `consumer` artifact
for the DSL itself.

## Decision

1. **The facade ports pact-jvm's Java DSL, and builds what it builds.** `PactDslWithProvider` and
   the chain under it, `PactDslJsonBody`, `PactDslJsonArray`, `PactDslJsonRootValue`,
   `PactDslRootValue`, the `LambdaDsl` over them, `PactXmlBuilder` and `MessagePactBuilder` are
   written in Java in the JVM SDK's `facade` layer, from pact-jvm at the commit above, with its
   provenance recorded. Each builds the same three things pact-jvm's builds: the example, and the
   matching rules and generators by category and path, as pact-jvm writes them into a pact of the
   test's specification. The facade depends on nothing of pact-jvm's.
2. **They reach the engine as `add-pact-interaction` calls.** An interaction is the calls `pact_ffi`
   would need to build what pact-jvm built: `upon_receiving`, `given`, `given_with_params`,
   `with_request`, `with_query_parameter`, `with_header_v2` and `response_status_v2` as pact-js's
   facade makes them, and for messages, the message calls S2.5c added. Two calls are added to the
   vocabulary, ported from `pact_ffi` as ADR 0066 decision 2 ports the rest:
   - `with_binary_body`, `{ part, content_type, body }`, `body` base64: the example's octets as
     pact-jvm writes them, with no integration JSON read from them and no rule added. pact-jvm's
     examples are plain values, so `with_body`'s processing would only misread a body that happens to
     contain a `pact:matcher:type` member.
   - `with_generators`, `{ part, generators }`: the generators by category, as a pact writes them,
     merged into the part's as `with_matching_rules` merges rules.

   The rules go as `with_matching_rules`, by category: `body`, `header`, `query`, `path`, `status` and
   `metadata`, as pact-jvm's DSL fills them.
3. **The facade draws nothing at random.** Where pact-jvm draws an example at random, as
   `stringMatcher(name, regex)`, `matchPath(regex)`, `matchHeader(name, regex)` and
   `matchQuery(name, regex)` do when given no example, the call is refused by name and asks for one.
   A random example would make every run record different bytes, and two runs merging into one
   contract (ADR 0065) would conflict, as ADR 0066 decision 7 found for a multipart boundary. Where
   pact-jvm puts a fixed example beside a random generator, as `integerType(name)` does with `100`, the
   facade does the same: the generator is a generator, and a mock fills it fresh (ADR 0048).
4. **Migrating is one textual replacement.** The facade's classes keep pact-jvm's names and sub-packages
   under the prefix `io.pact.janus.facade.`: `au.com.dius.pact.consumer.dsl.PactDslJsonBody` is
   `io.pact.janus.facade.consumer.dsl.PactDslJsonBody`, and `au.com.dius.pact.core.model.annotations.Pact`
   is `io.pact.janus.facade.core.model.annotations.Pact`. A suite moves by replacing
   `au.com.dius.pact.` with `io.pact.janus.facade.` in its imports and changing one dependency. The
   facade is its own jar, so a user of the Janus DSL does not get it. S2.9 decides the packages pact-jvm
   ships it under.
5. **The JUnit 5 extension runs a test as pact-jvm's does, through the engine.** `PactConsumerTestExt`
   finds the `@Pact` methods `@PactTestFor` names, calls each with its builder, and runs the test in
   one engine session. It adds each interaction, arms its one variant (ADR 0066 decision 4), starts
   one HTTP transport per provider for an HTTP test, and hands the test what pact-jvm hands it: a
   `MockServer` for each provider, or the messages, `List<Message>` or the `MessagePact`, served with no
   transport (ADR 0067 decision 5). Then it reports the test's verdict for every variant (ADR 0064). A
   test passes when pact-jvm's would: every interaction was called, and nothing the mock received was
   wanting. Otherwise it fails with pact-jvm's message. The contract is written with `finalise`'s
   `write` in merge mode (ADR 0065), to `<consumer>-<provider>.janus.json` in the directory pact-jvm
   writes its pacts to.
6. **What has no conversion is refused by name, at the call**, as ADR 0066 decision 6 says. This
   includes `@PactTestFor(https = true)`, `@MockServerConfig`'s TLS and keystore options, and naming a
   `mockServerImplementation`. JUnit 4's `PactProviderRule` and `@PactVerification` are `dropped`, since
   JUnit 4 is not supported (S2.4). The verdict of each primitive is ADR 0066 decision 5's, and the
   pact-jvm section of `consumer-test-migration.md` publishes it.

## Alternatives considered

- **Port the DSL, but write integration JSON.** Each method would put `{ "pact:matcher:type": … }`
  in the body and let the engine's port of the FFI build the rules, as pact-js's matchers do. The
  facade would be smaller. But the rules would be the FFI's reading of what the facade wrote, not
  pact-jvm's own, and integration JSON has no form for some of them: a list of rules is always
  combined with `AND`, so pact-jvm's `or` cannot be written. Where they differ, a contract would
  differ from what `janus upgrade` writes from pact-jvm's pact, which is what S2.5b's exit compares
  against.
- **Depend on pact-jvm's DSL.** The facade would be a JUnit 5 extension and a converter, and every
  class would be pact-jvm's own. But the `consumer` artifact brings pact-jvm's core, its matching
  engine, Kotlin, Ktor and Netty for its mock servers, the plugin driver and Tika. The SDK would pull
  in a second matching engine it never runs, and move whenever pact-jvm releases.
- **Keep pact-jvm's own package names.** Moving would be only a dependency change. But both jars on one
  classpath would split packages, and naming another project's packages is that project's decision.
  S2.9 makes it, when the SDK becomes pact-jvm's next major version.
- **Draw a deterministic example from the regex.** The engine, or the facade, could derive an example
  from the pattern. That is a regex generator the facade would own, or a call the FFI does not have.
  Refusing asks the test for one line it should arguably have had already.

## Consequences

Easier: no matching logic enters the SDK. The facade builds data, the engine converts it as `upgrade`
does, and S1.1's harness covers what it records. A pact-jvm suite moves by one replacement in its
imports, unless it uses a refused call.

Harder: the facade is large, the size of the pact-jvm DSL's data building, and its layer's budget
says so. It has to follow pact-jvm's DSL as that changes. The kernel tests replay pact-jvm's
`consumer/junit5` interactions as calls and compare the result with the pacts pact-jvm writes, so a
drift shows up as a test failure.

Committed to: the `io.pact.janus.facade.` prefix until S2.9, `with_binary_body` and `with_generators`
in `add-pact-interaction`'s vocabulary, and refusing a random example.

Not decided here: pact-jvm's Groovy, Spock and Kotlin DSLs, and its V4 `PactBuilder`, which are
S2.5d's. They translate to the same calls.

Tripwire: if pilot suites lean on random regex examples so much that the refusal costs more than one
line per test, the facade derives a deterministic example from the pattern, and this ADR is amended.
