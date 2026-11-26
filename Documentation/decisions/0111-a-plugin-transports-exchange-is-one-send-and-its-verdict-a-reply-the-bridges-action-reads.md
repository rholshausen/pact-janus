# 0111 — A plugin transport's exchange is one send, and its verdict a reply the bridge's action reads

- **Status**: accepted
- **Date**: 2026-11-26
- **Plan tasks**: S5.2e (plugin transports in verification); S5.2f, which serves a plugin transport's
  mock; S5.2h, which releases the bridge; S5.2i, a plugin transport's request filter and output
- **Evidence**: [ADR 0046](0046-ask-a-provider-for-a-message-through-produce-message-and-let-the-transport-name-its-parts.md),
  [ADR 0106](0106-bridge-pact-plugins-through-a-child-process-that-speaks-the-subprocess-binding.md),
  [ADR 0108](0108-a-subprocess-component-is-a-program-and-its-arguments-and-its-process-is-the-unit-of-recreation.md),
  [ADR 0109](0109-a-bridged-plugins-content-matcher-is-one-compare-contents-action-and-its-catalogue-a-handshake-capability.md),
  [ADR 0110](0110-a-v1-v4-plugin-body-compiles-to-its-bridges-compare-contents-and-one-node-reports-each-mismatch-it-found.md);
  component-interfaces spec §5 and §9.3.1; engine-protocol spec §8.3; `pact_verifier` 1.5.0's
  `verify_interaction_using_transport` and `pact_verifier_cli`'s `configure_provider`; pact-plugin-driver
  1.2.7's `prepare_validation_for_interaction` and `verify_interaction`; pact-plugins'
  `examples/gRPC/area_calculator`

## Context

A v4 interaction can name a `transport`, and when a plugin registered a transport of that name
(`grpc`, from the protobuf plugin), today's verifier does not send anything itself. It asks the plugin
twice: `PrepareInteractionForVerification`, handed the pact, the interaction's key and a context of the
provider's `host`, its `port` and the `providerState` values, answers the request it would send; and
`VerifyInteraction`, handed that request back with the same context, sends it, reads the answer,
compares it with the pact and answers a verdict: `ok`, a list of mismatches and errors, and lines of
output. The comparison is the plugin's. Today's verifier turns the mismatches into its own, drops the
errors (`TODO: matching crate does not support storing an error against an item`), and fails the
interaction when `ok` is false.

ADR 0110 refuses such an interaction by name (`details.why: transport`). ADR 0106 said a plugin transport
would become a Janus transport kind, and left S5.2e one question: how a verdict the plugin reached
becomes a verification result without the engine comparing the same parts again. A Janus transport only
moves parts, and the engine decides (spec §5.4): `transport/send` returns a reply, and the plan for the
transport's `answering` part is scored against it (§5.7).

Three more questions follow from driving it at all:

- **What is sent.** The plugin builds the request from the pact, not from parts the engine holds. It
  needs the interaction as the pact wrote it, the pact's metadata (the protobuf plugin's descriptors are
  there), and the provider-state values, which exist only once the states are set up.
- **Where it is sent.** Today's verifier reaches a plugin transport at the provider's host and at the
  port of the `--transports` entry named for it, else at `--port`. pact-plugins' own examples verify the
  area calculator with `-p <port>` alone. A Janus verification's target binds each kind it drives
  (`target.transports`), started before the run, and a binding no loaded component provides fails the
  call. But a `grpc` transport exists only once a bridge is loaded, which is when a pact names its plugin,
  and a broker's pact names it only once it is fetched, inside the run.
- **What a host sees.** A plugin transport's request and reply are the plugin's, in the plugin's terms.

## Decision

1. **A plugin's transport is a transport kind the bridge contributes, in the drive role.** For each
   `transport` entry in the plugin's catalogue the bridge's handshake implements `transport` and
   contributes the kind by the entry's key (`grpc`), with `roles: ["drive"]` and exchange parts
   `request` and `response`. Its catalogue entry under the `pact-plugin` capability names the `kind` and
   the `action` that reads its verdict (decision 3). Serving a plugin transport's mock is S5.2f's.
2. **One `send` is the whole exchange.** Its `request` part has three slots, all plain JSON: `pact`, the
   v4 pact holding this interaction alone, with its metadata; `key`, the interaction's key in it; and
   `provider-state`, the exchange's provider-state values, which the engine writes after the states are
   set up, every state's `params` and then what the `state-setup` hooks returned (ADR 0034's precedence).
   The bridge calls `PrepareInteractionForVerification` and then `VerifyInteraction`, each with the
   context today's verifier builds (`host`, `port`, `providerState`) and the drive instance's `headers`
   added to the request's metadata as today's `--header`s are. Its reply's `response` part has one slot,
   `verdict`: the plugin's answer, `ok`, its `mismatches` (each `message`, `at` the plugin's path,
   `expected` and `actual`), its `errors` and its `output`. A plugin the bridge cannot reach is
   `unavailable` and the bridge exits (ADR 0109 decision 6); a plugin that answers either call with an
   error is the send's error, `transport-failed` with the plugin's message, as a provider HTTP cannot
   reach is the HTTP transport's.
3. **The verdict is scored as any reply is, by one action of the bridge.** The bridge contributes
   `<component>:verdict`, whose value under test is the `verdict` slot: `ok` is `pass`, and otherwise the
   result is an `error` whose `mismatches` are each of the plugin's mismatches, located `at` its path, and
   each of its errors, unlocated (ADR 0110 decision 1). The legacy compiler compiles an interaction carried
   over a bridged plugin transport to that one node on `$.response.verdict`, and to nothing else: the
   plugin compared the request, the reply and their bodies, and nothing is compared twice. So the verdict
   is in the plan, an executed plan shows it, and `janus explain --executed` replays it from captured values,
   as for any other interaction.
4. **A binding may be `optional`, and a plugin transport is reached as today's verifier reaches it.** A
   `target.transports` binding gains `optional`: when no component in the run's scope provides its kind it
   is held rather than refused, and started the first time an interaction of its kind is verified, if a
   component loaded by then provides it (a fetched pact's bridge); never started, it is no error. A binding
   whose kind is provided is started before the run whether optional or not. An interaction carried over a
   bridged plugin transport with no binding of its own kind is driven through an instance started with the
   run's first binding's options, the provider's, as today's verifier falls back to the provider's host and
   port. The bridge reads where to send from a drive instance's options: `host` and `port`, else
   `base-url`'s host and port, and `headers`. `janus verify` binds each `--transports` entry but `http`
   optionally, at the provider's host and the entry's port, and binds `--transport`'s kind optionally when
   it is neither `http` nor `https`.
5. **What a plugin transport exchanges is not rewritten by hooks in its terms.** `before-request` and
   `after-response` see the parts decision 2 names, so a request filter cannot change the request the
   plugin builds, as today's `--request-filter` can (`call_non_http`). The plugin's `output` reaches the
   verdict and no report. Both are staged plan S5.2i.

## Alternatives considered

- **A verdict on `transport/send`'s result,** beside or instead of `reply`. Every transport and every
  binding would carry a member only a bridge fills, the engine would score an exchange without its plan,
  and an executed plan and `explain --executed` would have nothing to show for it.
- **The whole interaction as one action** (`<component>:verify-interaction` on no value), sending from a
  matcher. A plan's values reach it through resolvers over the parts an exchange produced (plan-grammar
  §2.4), and this action would produce them itself: I/O inside a plan, with no transport instance to route
  it, no hook points around it, and nothing for `explain --executed` to replay.
- **Prepare as an action and send as a transport,** so the engine holds the request the plugin builds and
  a `before-request` hook can rewrite it. `VerifyInteraction` needs the pact and the interaction again, so
  the request part would carry them anyway, and the plugin would be asked to compare an exchange the engine
  split in two. Request filters on plugin transports are rare, and decision 5 says so rather than building
  for them now.
- **Compile the reply's bodies to `compare-contents`** as well. The plugin already compared them, with the
  rules and configuration it chose; a second comparison could only disagree with it.
- **Refuse a binding no component provides, as now, and have hosts bind `grpc` only when a pact needs it.**
  A host cannot know: a broker's pacts arrive in the run. And today's `--transports grpc:<port>` is
  harmless for a pact that needs no gRPC.

## Consequences

- S5.2e's exit is an S1.1 fixture for the area calculator, its pact as pact-plugins' Go consumer wrote it,
  verified against a gRPC provider passing and failing, agreeing with `pact_verifier_cli` 1.5.0.
- The component interface's bridge section (§9.3.1) gains the transport and the verdict action, and the
  engine protocol's `TransportBinding` gains `optional`: additive, so no version moves.
- A run's targets can grow during it; every instance it started is stopped when it ends.
- Request filters on a plugin transport, and the plugin's output in a report, are differences from today's
  verifier, recorded in `Documentation/verifier-migration.md` until S5.2i.
- **Tripwire**: a plugin transport whose verdict depends on something the engine holds and decision 2 does
  not send. The request part is then missing a slot, and a slot belongs to the bridge's contract, not the
  kernel's.
