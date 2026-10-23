# 0046 — Ask a provider for a message through `produce-message`, and let the transport name its parts

- **Status**: accepted
- **Date**: 2026-10-23
- **Plan tasks**: S1.7
- **Evidence**: kernel-boundary review findings 8 and 9; spike 1.5 finding 6; pact-reference's
  `pact_verifier` (`messages.rs`, `verify_message_from_provider` and `verify_sync_message_from_provider`)
  and `pact_matching` (`match_message`, `match_message_contents`, `match_message_metadata`);
  `pact_verifier_cli --transports`; the pact-compatibility-suite's V3 and V4 message provider features;
  pact-jvm's and pact-js's message provider examples, in S1.1's corpus

## Context

`janus verify` skipped every v1–v4 message interaction without a word. A v3 message pact verified
nothing, and its run reported `verified`. pact-jvm users have such pacts in numbers.

Today's verifier asks the provider for each message over HTTP. It POSTs the message's `description`
and `providerStates`, and a synchronous message's `request`, to the provider. The reply's body is the
message. Its `Content-Type` and the base64 JSON in a `Pact-Message-Metadata` header are the message's
metadata. Which URL it POSTs to is `--transports`' business: the transport the interaction names, else
one named `message` (or `async-message`, `sync-message`), else the provider's own URL. A pact that mixes
HTTP interactions and messages can therefore keep its message producer on another port.

The comparison is not a response's. The metadata keys the pact names are compared by their rules, else
as a MIME type for `contentType`, else by equality. The contents' content type must be *equivalent* to
the pact's, which is stricter than the check before a response body. Then the contents are compared as a
response body is.

Two findings of the kernel-boundary review stood in the way. The kernel spelled HTTP's part names: the
verifier sent a variant's `request` and scored the reply's `response`, and the consumer's passive loop
matched an arrival's `request` and replied with `response` (finding 8). And a request that did not match
was answered with a `status: 500` the kernel wrote itself (finding 9). A message has neither a `request`
nor a `response`, and a message transport has no status to write.

The design already had the way in. Lifecycle-hooks spec §3.4 specified `produce-message`, the point at
which a provider's own producer is asked for the message. Spike 1.5 found that a hook and a wire
transport are interchangeable sources of the same parts.

## Decision

1. **A transport names its own parts.** Its contribution declares `exchange-parts`: `initiating`, the
   part an exchange starts with, and `answering`, the part that answers it (component-interfaces spec
   §5.7). HTTP's are `request` and `response`. The kernel reads them and spells none. Driving, it sends
   the `initiating` part and scores the plan's `answering` subtree. Serving, it matches the
   `initiating` subtree and replies with the `answering` part.
2. **A request that does not match is rejected, not answered.** The kernel disposes of the arrival with
   `reject`, and the `details` carry the mismatches. How a kind says no is the transport's: the HTTP
   transport answers the same 500 naming every mismatch that the kernel used to write.
3. **A v1–v4 message is produced by `produce-message` hooks.** A v3 message, and a v4 asynchronous or
   synchronous message without a plugin, is one replay that drives no transport. Its states are set up,
   its `produce-message` hooks produce a `message` part, and the part is compared with the message the
   pact expects (a synchronous message's first response), as today's verifier compares one. The part
   has two slots: `metadata`, one object, and `body`, the contents as octets labelled with their type.
   A new legacy action, `legacy:message-type`, is today's equivalence check on the content type. When
   no hook produces the part, the variant fails as `component-unavailable` naming
   `hook/produce-message`, with `reason: message-not-produced`. The provider was never asked, so
   nothing was verified, and the result says how to ask it.
4. **The `http` hook speaks today's message protocol.** `format: pact-message` POSTs today's request
   for the message and reads the reply as today's verifier reads it. `transports`, a list of names and
   URLs, routes it as `--transports` does. The engine hands the hook what it needs as
   `interaction.pact-message`: the request, whether the message is synchronous, and the transport the
   pact names.
5. **`janus verify` asks the provider as today when nothing else is configured.** It takes today's
   `--transports NAME:PORT[/PATH]`. When the configuration names no `produce-message` hook, it adds
   one, `provider-messages`, in `pact-message` format. The hook's URL is the provider URL, its
   `transports` are `--transports`, and it takes the `--header`s and the client options the provider
   is reached with. An `http` entry in `--transports` is where HTTP interactions go, as today. A
   configuration's own `produce-message` hooks replace the default whole, since hooks at a point chain
   and a second producer would overwrite the first.
6. **A reply that is not a 2xx fails, for every message.** Today's verifier requires a success only
   for a synchronous message, and reads an asynchronous one from the body of any reply. Janus fails
   the `produce-message` hook, as an `http` hook's non-2xx always does. A producer that answered 500
   has not produced the message, whatever its body says.

## Alternatives considered

- **Require a hook, with no default.** That is more explicit, and the hook is in the verifier's
  configuration where the reader can see it. But every provider that verifies messages today would
  need a configuration file to move, where an HTTP-only provider needs none.
- **A default producer at `--provider-url` only.** Simpler, and right for a provider that serves both
  on one port, as pact-js's message proxy does. It is wrong for a pact that mixes HTTP interactions and
  messages and a provider that keeps its producer on another port, which is what `--transports` exists
  for.
- **Route by `when: { transport }` selectors.** A hook entry per `--transports` entry, selected by the
  interaction's transport, would need no `transports` member. Today's rule has a fallback ("else the
  provider URL") that selectors cannot express: an absent `when` admits everything, and the hooks at a
  point chain.
- **A message transport over HTTP.** The parts a transport sends are the interaction's, and today's
  request is the interaction's description and states, not a part of it. A hook is handed exactly
  those, and lifecycle-hooks spec §3.4 already specified the point.
- **Keep the kernel's part names, and map a message onto `response`.** That would have verified
  messages without touching the kernel, and left findings 8 and 9 open for the first transport that is
  not HTTP-shaped.
- **Read an asynchronous message from any reply, as today.** That is exact, but it verifies a message
  a producer returned with an error, and a mutation that only changes the status would pass.

## Consequences

- A v1–v4 message pact is verified. Before, it was skipped and its run reported `verified`. A message
  pact whose provider does not answer messages now fails, so the change file is `major`.
- S1.1's corpus has `messages/`: pact-js's and pact-jvm's message examples (asynchronous, with
  metadata, and synchronous), a pact that mixes HTTP and a message, with mutations, and the
  compatibility suite's V3 and V4 message provider scenarios. Every verdict agrees but one deliberate
  difference: `messages/error-status`, decision 6, is in the ledger.
- `before-request` does not run for a message, because no request is sent (lifecycle-hooks spec §4.1).
  Today's request filter and `-H` headers reach the message POST. The default producer takes the `-H`
  headers, and a provider whose message endpoint needs a credential a request filter added names it in
  its `produce-message` hook's `headers`.
- `janus upgrade` still drops a message interaction with a finding: a contract has no message form
  until the consumer side has one (S6.2). `janus explain` shows a message's plan.
- `consume-message` on the provider side, which verifies a provider that consumes messages, is
  specified and not built: today's verifier has no such thing. A transport that delivers messages from
  a real broker is S6.3's.
- The `TransportComponent` trait has a new required method, `exchange_parts`, and `Dispose` a new
  member, `details`. Every in-tree transport declares its parts in code; the handshake's
  `TransportContribution` has `exchange-parts` for an out-of-tree one, which nothing loads yet (S5.5).
- **Tripwire**: if pilot providers rely on an asynchronous message endpoint that answers with a non-2xx
  status, decision 6 is the one to reopen.
