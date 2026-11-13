# 0091 — Derive a message producer's shape from AsyncAPI, selected by the values its messages fix

- **Status**: accepted
- **Date**: 2026-11-13
- **Plan tasks**: S4.8 (extends ADR 0088 to AsyncAPI; makes ADR 0090 decision 7 concrete)
- **Evidence**: [ADR 0088](0088-derive-a-provider-shape-from-openapi-with-janus-derive-and-report-every-mapping-that-loses-something.md),
  [ADR 0090](0090-name-a-messages-operation-by-the-message-itself-and-record-it-as-produce-message-hooks-produce-it.md),
  [ADR 0067](0067-record-a-message-as-an-emissive-interaction-and-hand-it-to-the-test-when-nothing-delivers-it.md),
  [ADR 0025](0025-a-provider-shape-entry-may-select-interactions-by-operation.md), the AsyncAPI 2.6 and
  3.0 specifications, pact-js's message example (`corpora/verifier/messages/pact-js-order-placed`)

## Context

S4.2 derives an HTTP provider's shape from its OpenAPI document (ADR 0088). A message producer
documents itself in AsyncAPI, and nothing derives its shape. AsyncAPI's Schema Object is a superset of
JSON Schema, as OpenAPI's is, so ADR 0088's schema mapping and its loss reporting apply to a message's
payload and headers unchanged. What AsyncAPI adds is around the schema:

- **Who sends what.** A document describes one application. AsyncAPI 3.0 says so with an operation's
  `action`, `send` or `receive`. 2.x says it with a channel's `subscribe` (what the application
  publishes, for others to subscribe to) and `publish` (what it receives), verbs written from the
  reader's side. A 3.0 operation may also have a `reply`: with `receive`, the application answers a
  request; with `send`, it makes one.
- **Identity.** ADR 0090 decided that a message's operation is a selector over the message itself, or
  over a synchronous message's request, and that an importer writes a selector only from what a message
  carries (decision 7). It left to this item which AsyncAPI constructs fix a value. A channel's
  address is the obvious identity, and a message does not carry it: it is where the message was
  published, which a consumer's contract records only if the consumer wrote it into the metadata, as
  pact-js's example writes `queue`.
- **Content type and format.** A message names its `contentType`, or the document a
  `defaultContentType`. Its payload may be a schema in another format (Avro, protobuf, RAML), which
  2.x names on the message (`schemaFormat`) and 3.0 on the payload (a Multi Format Schema Object).
- **Traits.** A message may list traits merged into it. In 2.x each trait is merged over the message
  in turn, so a trait wins; in 3.0 the message's own members win.

## Decision

1. **`janus derive` reads AsyncAPI 2.x and 3.0**, as JSON or YAML, by the document's `asyncapi`
   member, through the same command, options and loss report as OpenAPI (ADR 0088 decisions 1 and 6).
   Any other version is refused by name, as Swagger 2.0 is. `--base-path` is OpenAPI's and is refused
   with an AsyncAPI document. `--assume-non-empty` applies as it does to OpenAPI.
2. **Only what the documented application produces is derived.** That is a message it sends (3.0
   `action: send` with no `reply`, 2.x `subscribe`), and a reply it answers a request with (3.0
   `action: receive` with a `reply`). What it receives (`receive` with no reply, 2.x `publish`) is
   its consumers' output. A request it makes (`send` with a `reply`) is the requester's half of a
   synchronous message, whose contract is the requester's own. Neither is derived, and neither is a
   loss: the shape claims nothing about it.
3. **One entry per message an operation sends, and per request and reply it answers.** An operation's
   messages are those it lists, else every message of its channel. In 2.x a message may be a `oneOf`
   of messages. A reply's messages are those it lists, else its channel's. A reply that names neither
   is `reply-unresolved`. Traits are merged as each version says. An entry's `message` part is:
   - **`metadata`**: the message's `headers` schema, through the schema mapping, and a `contentType`
     member, `http:media-type` of the message's content type, else the document's default. A
     consumer's contract records the type there (ADR 0067 decision 3), and `janus upgrade` writes it
     the same way. Where the headers name `contentType` themselves, theirs is kept.
   - **`body`**: the payload schema, through the schema mapping, when the content type is JSON (as
     ADR 0088 decision 4 reads one). A message that names no content type, in a document with no
     default, is read as JSON and reported as `content-type-unstated`. Any other content type claims
     no body (`body-not-json`). So does a payload in a schema format the mapping does not read
     (`schema-format-unread`), until a content component decodes it: protobuf for S5.2, Avro for S6.9.
     AsyncAPI's, JSON Schema's and OpenAPI 3.0's formats are read. A message with no payload claims no
     body (`untyped-schema`).
   - A synchronous message's request is not a part. A provider shape carries what the provider
     produces (subsumption-check spec §2.1), and the request only selects the entry.
4. **A selector is the values a message fixes.** A header or payload member fixes a value when its
   schema allows exactly one: `const`, or an `enum` of one. The selector is every such value reached
   from the root through objects' `properties` (`$ref` and `allOf` followed), written as `equality`
   inside the objects that lead to it. Its part is `message` for a message the application sends, and
   `request` for a reply, whose request's values select it (ADR 0090 decision 1). Arrays, unions and
   `additionalProperties` are not searched, since which element or alternative a consumer's message
   carries is not fixed. A member need not be `required` to select: a selector makes no claim (ADR
   0025 decision 5), and a consumer whose message lacks the member is simply not selected by it.
5. **The address is a selector member only where a header fixes it.** That is decision 4 applied to
   ADR 0090 decision 7: a document that says its message carries its queue writes the queue as a
   header with one value, as the pact-js example's provider does. The address is otherwise in the
   entry's description and source, and selects nothing. A message that fixes nothing has no selector,
   is matched only by its description, and is reported as `selector-empty`.
6. **Two entries one message could select are reported.** `janus check` chooses neither and reports
   the interaction `not-published` (subsumption-check spec §2.2). Two selectors tie when no member
   they both fix has different values. The common case is one message sent on two channels. Each
   entry of such a pair is reported as `selector-ambiguous`, naming the other.
7. **An entry's description and source name the operation as a reader would.** The description is
   `send <address> <message>`, or `reply to <request> on <address> with <reply>`. The source is
   `{ "asyncapi": "send <address>" | "receive <address>", "operation-id", "message", "reply" }`.
   The address is a 3.0 channel's `address`, else its id when the address is only known at run time,
   and a 2.x channel's key. A message's name is its `name`, else its `messageId`, else the last
   reference followed to it, else its key. So a 2.x document and a 3.0 one for one producer, with
   the 2.x `operationId` the 3.0 operation's key, derive the same shape.
8. **Bindings and `correlationId` are not read.** A Kafka key or an AMQP routing key has no slot until
   S6.2's transports give a message one. The shape claims nothing there, which the check compares
   only where both documents have a slot.

## Alternatives considered

- **The channel address as a selector member under a fixed metadata name** (`queue`, `topic`, or one
  `--address-member` names). It would match pact-js's convention without the document saying the
  message carries it. But it is a claim about the consumer's metadata, not the provider's message, and
  ADR 0090 decision 7 and its tripwire reserve that for S6.2's transports, whose own slot will carry an
  address. A provider whose consumers write the queue documents it as a header, as decision 5 says.
- **Only required members select.** That is stricter, and it loses the common case: AsyncAPI documents
  often leave `required` off headers, and a selector over an optional member never selects a message
  without it, so it costs nothing.
- **An `enum` of several values as an `any-of` selector member.** It would select a message carrying
  any of them, which identifies nothing: the same member on a sibling message usually admits the same
  values.
- **Derive the request of a `send` with a `reply`** as an asynchronous message the application
  produces. Nobody checks a contract against it: a synchronous message's consumer is its requester,
  and the request is the consumer's own.
- **A separate front end per AsyncAPI major version.** The two differ around operations and traits,
  not schemas, so one module reads both, branching where they do.

## Consequences

Easier: a message producer with an AsyncAPI document has a shape with one command. It is matched to
every consumer whose message carries a value the document fixes, and checked as a recorded message
shape is (S4.6). A request/reply operation is checked against a synchronous message.

Harder: a producer whose messages fix nothing gets a shape that `janus check` matches only by
description, which a derived entry rarely shares with a consumer. The report says so per message, and
the remedy is in the document (a `const` on the header or member that names the event's kind). A
document that sends one message on two channels gets two entries that tie, and is told.

Committed to: the AsyncAPI loss codes (`content-type-unstated`, `reply-unresolved`,
`schema-format-unread`, `selector-ambiguous`, `selector-empty`) are CLI surface beside ADR 0088's.
A message's `contentType` is written into the metadata as `http:media-type`, as `janus upgrade`
writes a v3 message's.

**Tripwire.** Revisit decision 5 when S6.2's transports carry a channel address in a slot of their
own, which an importer should then write. Revisit decision 4 if pilot producers' documents fix
nothing on their messages, so derived message shapes are matched only by description.
