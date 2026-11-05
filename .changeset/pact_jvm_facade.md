---
jvm-sdk: minor
---

# `janus-facade`: pact-jvm's Java DSL and JUnit 5 consumer extension, in progress

A new jar, `janus-facade`, is pact-jvm's consumer DSL over the engine (ADR 0070, staged plan S2.5b):
the `PactDslWithProvider` chain, `PactDslJsonBody`, `PactDslJsonArray`, the root values,
`LambdaDsl`, `PactXmlBuilder`, `FormPostBuilder`, `MessagePactBuilder` and `PactConsumerTestExt`,
under `io.pact.janus.facade.` with pact-jvm's names. A suite moves by replacing `au.com.dius.pact.`
with `io.pact.janus.facade.` in its imports. Each builder builds the example, rules and generators
pact-jvm's builds and submits them through `add-pact-interaction`, which needs an engine that takes
`with_binary_body` (with its `content_type_header`) and `with_generators`. The SDK's `Engine` sends
frames for a layer that speaks the protocol itself.
