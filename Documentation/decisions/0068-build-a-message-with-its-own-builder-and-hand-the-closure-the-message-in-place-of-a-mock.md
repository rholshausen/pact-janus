# 0068 — Build a message with its own builder, and hand the closure the message in place of a mock

- **Status**: accepted
- **Date**: 2026-11-04
- **Plan tasks**: S2.14
- **Evidence**: [ADR 0067](0067-record-a-message-as-an-emissive-interaction-and-hand-it-to-the-test-when-nothing-delivers-it.md)
  decisions 1, 2, 5 and 6; behavioural spec `interaction`, `request`, `response`, `content` and
  `execute`; pact-js's `MessageConsumerPact` (`expectsToReceive`, `withContent`, `withMetadata`,
  `verify(handler)`) and V4 `addSynchronousInteraction`; pact-jvm's `MessageInteractionBuilder` and the
  `V4Interaction.AsynchronousMessage` a JUnit 5 message test is handed; the thinness audit's budgets

## Context

ADR 0067 gave a contract its message form and made `consumer-session/serve-variant` answer an emissive
interaction's message as `{ parts }` when no transport of its kind is started. Neither SDK can use it:
the DSL builds HTTP interactions only, its `transport` fixed at `{ kind: http, mode: passive }`, and
`execute` starts the HTTP transport and hands the closure a mock.

Three things have to be decided in the specification before either SDK builds them: how a message is
spelled, what decides its body's type, and what the closure receives.

- **The type.** An HTTP body with no declared type is produced as `application/json`, because the HTTP
  transport says its `body` is a content slot (component-interfaces spec §5.5). Nothing says that of a
  message: with no transport, the engine encodes a declared slot and hands every other slot over as its
  value (ADR 0067 decision 5). An undeclared JSON body would reach the test as a document, and a test
  written against today's libraries holds octets or text.
- **The closure.** Today's message tests are handed the message, and call their own handler with it.
  pact-js's gets `contents` and `metadata`; pact-jvm's gets the contents as bytes or a string.
- **The budget.** Both SDKs' idiomatic layers are within a few lines of their thinness budget.

## Decision

1. **A message has its own builder, `message`, beside `interaction`.** It opens an interaction
   specification whose `transport` is `{ kind: message, mode: emissive }`. `given` is the one
   `interaction` has, unchanged. `message-contents` declares the message, `parts.message`, and
   `message-request` the request a synchronous message answers, `parts.request` (ADR 0067
   decision 2). Each takes `metadata` and `body`. A builder that is only a message cannot be given an
   HTTP `request` or `response`, so no chain mixes the two and nothing has to refuse one.
2. **A message's body always declares its type.** A body written with `content` declares its media
   type, as an HTTP body's does. Any other body declares `application/json`. The SDK writes the type
   because nothing else would: the declaration is what the engine encodes the body by, and it is what
   a verifier decodes the produced message under. `metadata` is a map of name to template, compiled
   as an `object` with one member per name by the `literal` rules. A member not given is no slot.
3. **`execute` of a message starts no transport, and hands the closure the message.** It sends what it
   sends for an interaction, without `start-transport`, and calls the closure with `(message,
   variant)`, `message` read from `serve-variant`'s `parts`: its `body` as octets, its `content-type`
   when it has one, and its `metadata`, an empty map when it has none, with `text` and `json` reading
   the octets as UTF-8 and as JSON. A synchronous message's `request` is one more message of the same
   form. The report and the variant's verdict are `execute`'s, unchanged. An answer with no `parts`
   is an engine older than ADR 0067, and `execute` fails at once rather than hand the closure
   nothing.
4. **The type is `ReceivedMessage` in both SDKs.** The JVM SDK's `Message` is what a provider's
   message producer returns (ADR 0056), and the consumer's view is what it was handed.
5. **The budgets rise by what the builder costs**, and the commit that raises them says by how much.

## Alternatives considered

- **One builder for both, with a `message` part beside `request` and `response`.** The chain would
  decide the transport, and an HTTP request with a message beside it would need a refusal no
  specification sentence asks for. pact-js and pact-jvm keep message builders apart too.
- **Leave the body undeclared and have the engine encode a structured slot as JSON.** The engine would
  encode a message's `metadata` too, which is a structured value and not content: which slot is
  content is a transport's to say (component-interfaces spec §5.5), and no transport is there.
- **Hand the closure the `parts` document.** Every test would decode base64 to read its message, and
  would read a protocol document a later engine may grow.
- **Decode the body for the test by its content type.** That is content knowledge in the SDK, which
  is the engine's. `json()` is the test asking for JSON, as `fetch`'s `Response.json()` is.
- **Let a message name its transport kind** (`kafka`). Today's verifier routes a message by it, and
  `upgrade` keeps it, but a native test has nothing to route yet. It is an option to add when S6.2's
  transports give a kind a meaning on the consumer's side.

## Consequences

- A native consumer test can record an asynchronous or a synchronous message, and the contract it
  writes is the one `janus upgrade` writes for the pact that test would have written, which a provider
  verifies through its message producers.
- The behavioural spec gains `message`, `message-contents` and `message-request`, and `execute` a
  second form. The conformance suite gains translation, scripted and live cases for them, and its
  case format a message chain and what a closure was handed.
- S2.5b and S2.5c spell pact-jvm's and pact-js's message builders on this one.
- **Tripwire**: a message transport (S6.2) that a consumer test must name. Then a message takes a
  `transport` option, and decision 3's "starts no transport" applies only when it names none.
