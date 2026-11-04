# 0067 — Record a message as an emissive interaction, and hand it to the test when nothing delivers it

- **Status**: accepted
- **Date**: 2026-11-04
- **Plan tasks**: S2.12; S6.2's core brought forward
- **Evidence**: [ADR 0046](0046-ask-a-provider-for-a-message-through-produce-message-and-let-the-transport-name-its-parts.md),
  which verifies a v1–v4 message and left `upgrade` dropping it; [ADR 0064](0064-report-each-variants-test-verdict-to-the-engine-and-answer-with-its-result.md);
  engine-protocol spec §7.4 and §8.2 (`serve-variant` for an emissive interaction); lifecycle-hooks
  spec §3.4–§3.5; spike 1.5 findings 3 and 6; S1.1's `messages/` fixtures and the compatibility suite's
  V3 and V4 message scenarios; pact-js's `MessageConsumerPact` and pact-jvm's message consumer tests,
  which hand the test the message rather than deliver it

## Context

A contract has no message form. `janus upgrade` drops every message interaction with an
`interaction-dropped` finding, `janus verify` can replay no message from a contract, and a consumer
session cannot hand one to a test. pact-js's and pact-jvm's message users cannot move until it can,
and S2.9 graduates the SDKs into exactly those libraries. S6.2 had all of it, behind real brokers.

Most of the design is already there. The contract's `transport` has `mode`, `passive` or `emissive`,
and `parts` names no part (contract-file spec §4.3, §5.1). The engine-protocol spec says what
`serve-variant` does for an emissive interaction: deliver the message now, through the bound
transport or a hook. ADR 0046 settled what a message *is* when a provider produces one: a `message`
part with a `metadata` slot, one object, and a `body` slot, its contents as octets with their type.
What it left open is the consumer's side, where nothing delivers a message yet: no in-tree transport
sends one, and the `consume-message` hook that would put it where an application reads it is
specified and not built. Today's consumer libraries do not deliver one either. pact-js's
`MessageConsumerPact.verify(handler)` and pact-jvm's message tests hand the message to the test, and
the test calls its own handler with it.

A synchronous message is a request the provider answers with a message. Today's verifier POSTs the
request, as the pact holds it, matching rules and all, to ask for the answer. A pact-js or pact-jvm
consumer test of one is handed both halves.

## Decision

1. **A message interaction is emissive, and its message is its `message` part.** Its `transport` is
   `{ kind, mode: "emissive" }`, `kind` being the transport the pact names, else `message`. Its
   `message` part has the two slots `produce-message` hooks produce: `metadata`, an `object` shape, and
   `body`. The body's type is declared in `content-types` as an HTTP body's is (contract-file spec
   §5.5). `message` is the hook system's word for the part a provider produces (lifecycle-hooks spec
   §3.4), not a transport's, so the kernel may name it.
2. **A synchronous message records its request too.** Its `request` part holds the message the consumer
   sends, with the same two slots. An emissive interaction with any part beside `message` is a request
   and its answer: no new member says so, because the parts already do.
3. **`upgrade` converts each message.** A v3 message and a v4 asynchronous message become a `message`
   part; a v4 synchronous message a `request` from its request and a `message` from its first response.
   - The metadata's members are the pact's keys, each by its rule, else as `equality`. A `contentType`
     with no rule is `equality`, where v1–v4 compare it as a MIME type, as they compare a header: it
     is `rule-narrowed` (`judgement`), as a header is, until S3.4 stops the narrowing with S3.3's
     `http:media-type`.
   - The body is converted as an HTTP body of its type is (contract-file spec §8.2), and its octets are
     the variant's evidence.
   - Today's check that the produced message's type agrees with the pact's (`legacy:message-type`) is
     on the metadata. A pact whose metadata names a `contentType` has it there already. One that names
     none gets an `optional` pattern on `contentType` admitting what the check admits, reported as
     `body-type-on-metadata` (`judgement`), as a reply's type is checked on its header (ADR 0047).
     A verifier reads a produced message's metadata `contentType` as today's verifier reads a
     message's type: its body's label, else the metadata's own, else what its octets are detected as.
     The label comes first because an `http` hook in `pact-message` format labels the body with the
     reply's `Content-Type`, and a `Pact-Message-Metadata` header can say otherwise in the metadata; a
     producer in a test that gave no type is read from its octets, as ADR 0056 decision 1 reads one.
   - Generators go where a response's go, and a `metadata` generator on its member.
   - A synchronous message with more than one response is converted with its first, reported as
     `responses-dropped` (`lossy`). Today's verifier compares the first and fails the interaction for
     expecting more, so the contract verifies what the pact could not.

   No message is `interaction-dropped` any more. A message that needs a pact plugin still is (ADR 0045).
4. **A contract's message is asked of the provider as a pact's is.** An emissive interaction drives no
   transport in verification. Its states are set up, its `produce-message` hooks produce the `message`
   part, and the plan's `message` subtree, pinned to the variant, scores it, as ADR 0046 decision 3
   scores a pact's. The hooks are told what today's verifier would POST for it, as
   `interaction.pact-message`: its description, the variant's resolved states and, for a synchronous
   message, its recorded `request` written as a v4 message's contents and metadata. So `janus verify`'s
   default producer, `--transports` and an SDK's message producers work unchanged. The request carries
   no `matchingRules`, since a contract has none; a provider that reads them is a deliberate
   difference, in S1.1's upgrade ledger.
5. **With nothing to deliver it, `serve-variant` answers with the message.** When no transport of the
   interaction's kind is started in the session, `serve-variant` for an emissive interaction answers
   `{ parts }`: every part of the variant, with each generator's fresh value filled as a mock fills a
   reply's (ADR 0048), in the form a transport carries. A declared slot is encoded through its content
   component, and every other slot is its plain value. The variant is then exercised, and its test's
   verdict (`report-variant`, ADR 0064) decides it: the host hands the parts to the test and reports
   what the test did. A synchronous message's answer holds its `request` and its `message`, and the
   engine matches no request, since nothing sent one, as today's consumer libraries match none.
   Delivering through a started transport of the kind is S6.2's. Until it is built, `serve-variant`
   refuses that case as `operation-unsupported` rather than claim a delivery it did not make.
6. **The SDKs are S2.14's.** Neither SDK can build a message interaction: their DSL is HTTP's. S2.14
   adds a message builder and an `execute` that, for a message, starts no transport and hands the
   closure the message instead of a mock, to both SDKs, under the behavioural spec and the conformance
   suite. S2.5b and S2.5c build their facades' message calls on it.

## Alternatives considered

- **Passive synchronous messages.** A synchronous message is initiated by the consumer, which is a
  passive interaction by engine-protocol spec §7.4. But with nothing to send it through, the consumer
  sends nothing, the variant is never exercised, and the contract is withheld. A transport that carries
  one (S6.2) may serve it passively; what it records then is S6.2's to decide.
- **A `synchronous` member on the transport descriptor.** It would say what the parts already say, and
  could disagree with them.
- **Hand the message to the test as documents, not as the wire form.** It saves an SDK parsing a JSON
  body, and gives a test a value no transport would ever deliver: a test is where a transport would be,
  and S6.2's transports will `send` exactly these parts.
- **Build `consume-message` on the consumer side first.** A hook delivers a message to where an
  application reads it, which a test that calls its own handler does not need, and today's tests are
  written that way.
- **Reconstruct `matchingRules` for a synchronous request from its shapes.** Shapes and rules are not in
  bijection (contract-file spec §8.1), so some would be wrong, and no provider in the pilots reads them.
- **Keep the pact's `legacy:header-value` on `contentType`.** A contract has no legacy actions;
  S3.3's `http:media-type` is the operator for it, and a header with no rule is narrowed the same way.

## Consequences

- `janus upgrade` converts every message pact S1.1 holds, so the harness now verifies their contracts
  too. Its upgrade ledger gains the synchronous request's `matchingRules`, and whatever the narrowed
  `contentType` disagrees on, each against this ADR or S3.4.
- A converted message contract verifies through the same `produce-message` path as the pact it came
  from. A pact with HTTP interactions and messages upgrades whole.
- `consumer-session/serve-variant`'s result gains `parts`. It is additive, so no capability guards it:
  a host that ignores it hands its test nothing, and its variants are still exercised, so the contract
  is withheld only if the test reports a failure.
- Committed to: the `message` and `request` parts of an emissive interaction, `serve-variant`'s
  `parts`, and the findings `body-type-on-metadata` and `responses-dropped`.
- **Tripwire**: a message transport (S6.2) that must record the consumer's request of a synchronous
  message as matched rather than given. Decision 5 then needs a passive form for it, and decision 2's
  "parts say so" may need a member after all.
