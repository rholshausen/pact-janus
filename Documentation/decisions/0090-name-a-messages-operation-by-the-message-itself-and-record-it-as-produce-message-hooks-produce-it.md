# 0090 — Name a message's operation by the message itself, and record it as `produce-message` hooks produce it

- **Status**: accepted
- **Date**: 2026-11-13
- **Plan tasks**: S4.6; the identity half of S4.8 (amends engine-protocol spec §8.5 and design 2.8 §2.2)
- **Evidence**: [ADR 0025](0025-a-provider-shape-entry-may-select-interactions-by-operation.md) and its
  tripwire, [ADR 0086](0086-record-a-provider-shape-from-the-providers-own-tests-by-the-operation-each-test-names.md)
  and its tripwire, [ADR 0067](0067-record-a-message-as-an-emissive-interaction-and-hand-it-to-the-test-when-nothing-delivers-it.md),
  [ADR 0068](0068-build-a-message-with-its-own-builder-and-hand-the-closure-the-message-in-place-of-a-mock.md),
  S1.1's message fixtures (`corpora/verifier/messages/`), pact-js's message example, whose consumer
  names its queue in the message's metadata

## Context

`janus check` has never run on a contract with messages (S2.12). Run on pact-js's message example, it
already decides one: the walk is direction-agnostic, the `message` part's `metadata` and `body` are
slots like any other, and the HTTP component's operators compare a `contentType` as S4.7 made them.
What a provider cannot do is produce the shape to check against.

- **Recording.** A recording session reads what the transport for its kind declares (ADR 0086
  decision 3), and no transport serves `message`: a message has no transport until S6.2. So
  `provider-shape-session/create` with `kind: message` is `component-unavailable`.
- **Identity.** An observation names its operation as a selector over the initiating part's slots,
  for HTTP a method and a path template. An asynchronous message answers nothing: no request
  initiates it. What identifies its operation is where it was published and which of that channel's
  messages it is. A Janus contract records neither as such (ADR 0068 left a message's transport out
  until S6.2), and a v1–v4 message pact names its message by description. ADR 0025's tripwire names
  this case: a provider that cannot identify an operation from its request slots argues for a
  transport-owned operation.
- **One recorder.** A provider that answers HTTP requests and publishes messages has one shape, and a
  session has one kind.

What consumers write is the evidence. pact-js's message example puts its queue in the message's
metadata (`queue: orders.placed`), as pact-jvm's Kafka and Spring Cloud Stream users put a topic or
destination there, and an event's kind is usually a member of its body (`type: OrderPlaced`) or a header.
Today's message providers key their producers by the consumer's description (`@PactVerifyProvider`,
pact-js's `messageProviders`), which a provider's own tests do not know.

## Decision

1. **A message's operation is a selector over the part that starts its exchange.** A synchronous
   message's is its `request`, as an HTTP operation's is its request. An asynchronous message starts
   its own exchange, so its selector is over the `message` part itself: a metadata member that names
   where it was published, a member of its body that names its kind, or both. A selector is matched
   against examples and never compared (ADR 0025 decision 5), so selecting by the part the provider
   produces makes no claim about it. A consumer whose recorded message carries nothing an entry
   selects by is matched by description, or not at all, and the report says which (`matched-by`).
2. **A selector reads a content slot as its document.** `janus check` matches a selector against
   each recorded example decoded by its declared type, through the engine's content components, as
   a verification reads it, so a selector over a body member sees the member and not the body's text.
   The walk itself is unchanged.
3. **The kind `message` is what `produce-message` hooks produce.** With no transport serving it, a
   recording session reads it as lifecycle-hooks spec §3.4 defines it: a `message` part answering a
   synchronous message's `request` (ADR 0067 decision 2), with no outcome slots and no credential
   members. A body with no label is read by the type a verification reads a produced message as
   (ADR 0067 decision 3): its metadata's `contentType`, else what its octets are detected as. Only
   the `message` part is recorded. A transport that serves the kind, when S6.2 adds one, is read
   instead.
4. **An observation may name its kind.** `observe` takes `kind`, the session's when absent, and each
   observation is read through its own kind's declarations, so one session records a provider's HTTP
   answers and its messages into one shape, under one source. Each kind's credential members are
   withheld from its own parts.
5. **The SDKs record a message as a message producer returns one.** Both recorders gain
   `observe-message(operation, message, options)`: `operation` is `metadata` and `body`, written as
   `message-contents`' slots are; `message` is the SDK's provider `Message` (ADR 0056), written as a
   producer's answer is; `request`, a second `Message`, makes it a synchronous message's answer and the
   operation the request's. `description` is required, since there is no method and path to name the
   entry by. A test that knows the consumer's description may give it, and its entry is then matched
   by description as today's message providers' are.
6. **`janus check` decides a message contract as any other.** No new finding or report member. Its
   reason for decided findings says the provider "may produce what this consumer has not tested",
   not "responses".
7. **An importer writes a selector only from what a message carries.** For S4.8: a value an AsyncAPI
   document fixes on a header or a payload member is a selector member. A channel address is one only
   where the message itself carries it. An address the message does not carry selects nothing, and
   is reported as a mapping that loses something. Which AsyncAPI constructs fix a value is S4.8's.

## Alternatives considered

- **A transport-owned operation: the contract's `transport` names its channel.** ADR 0025's option a.
  Nothing writes one: a native consumer's message names no transport (ADR 0068), and a v1–v4 message
  names no channel. It waits on S6.2 giving a consumer test a transport to name, and every contract
  written before then would still select nothing.
- **Description only, as today's message providers match.** It works where a provider verifies
  against a consumer's pact, and fails in its own tests, which is the problem ADR 0086 solved for
  HTTP. It stays, as decision 5's alternative.
- **A built-in `message` transport component that only declares its parts.** A consumer session could
  then start it, and ADR 0067 decision 5 answers a message's parts to the test only when no transport
  of its kind is started.
- **A session per kind.** Two sessions writing one shape under one source would replace each other's
  evidence (ADR 0086 decision 7), and two sources would put an SDK's detail into the evidence file.
- **Outcome slots for a message.** A message's kind is its operation, not its outcome: an event whose
  kind is a header and a body member together is one selector over both slots, so ADR 0086's tripwire
  on decision 4 does not fire.

## Consequences

Easier: a provider records the messages its own tests produce with the recorder it records HTTP with,
and a message contract is checked against them, matched by what the consumer's message carries. A
synchronous message is selected as an HTTP request is.

Harder: a consumer that writes no queue, topic or kind into its message selects no entry, and its
provider must record with the consumer's descriptions to be checked. A selector over a body member is
matched against a decoded document, so it costs a decode per example.

Committed to: `observe`'s `kind`, behind `provider-shape-recording`; the kind `message` with no
transport; `observe-message` in the behavioural spec.

**Tripwire.** Revisit decision 1 if consumers' messages routinely carry nothing that identifies their
operation, so message entries are matched only by description, or when S6.2's transports carry a
channel address in a slot of their own, which a selector should name instead of a metadata member.
