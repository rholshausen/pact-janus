---
jvm-sdk: minor
---

# `janus-facade`: pact-jvm's Java DSL and JUnit 5 consumer extension

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
