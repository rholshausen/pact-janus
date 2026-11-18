# 0094 — Hold what a transport kind declares apart from the transport, so an embedding with none can record it

- **Status**: accepted
- **Date**: 2026-11-18
- **Plan tasks**: S4.9 (for S4.10a, the Drift plugin that records through `janus-engine.wasm`)
- **Evidence**: [the provider-shape plugin proposal](../provider-shape-plugin.md) §4,
  [ADR 0086](0086-record-a-provider-shape-from-the-providers-own-tests-by-the-operation-each-test-names.md)
  decisions 3 and 4, [ADR 0074](0074-redact-credentials-from-what-the-engine-records-and-reports.md)
  decision 2, [ADR 0090](0090-name-a-messages-operation-by-the-message-itself-and-record-it-as-produce-message-hooks-produce-it.md)
  decision 3, [ADR 0092](0092-decide-in-a-broker-through-the-engines-wasm-component-and-refuse-to-start-without-it.md)
  decision 3, `engine/component-host/tests/engine_wasm.rs`

## Context

`provider-shape-session/observe` reads an observation through what the transport for its kind
declares (ADR 0086 decision 3): which part answers an exchange, so only the answer is recorded
(component-interfaces spec §5.7); the outcome slots that keep a 404 from widening a 200 (§5.10); and
the credential members recorded only as present (§5.8, ADR 0074). The engine found them by looking the
kind up among its registered transports. `janus-engine.wasm` registers none: it has no thread for an
exchange loop, and the HTTP transport is not built for its target (ADR 0062, ADR 0092). So inside it
an `http` observation was `component-unavailable`, and only the kind `message` recorded, because ADR
0090 decision 3 gave it declarations the kernel writes itself.

S4.10's Drift plugin embeds `janus-engine.wasm` and records HTTP answers, so this is the gap between
the artifact and its first host that is not a broker. The declarations are not I/O. They are three
constant answers, written as methods of `HttpTransport`, which is behind the `transport` feature the
WASM build leaves off.

The same lookup also decides what a consumer session redacts (ADR 0074): the engine asks each
registered transport for its credential members. Inside the WASM engine there were none to ask, so a
contract finalised there would record an `Authorization` header's value, which the native engine
records as `"[redacted]"`.

## Decision

1. **What a transport kind declares without doing I/O is a value of the kind.** The kernel holds it as
   `KindDeclarations` (`engine/kernel/src/component/transport.rs`): the exchange parts, the outcome
   slots and the credential members. `KindDeclarations::of` reads them from a transport, and that
   transport's trait methods stay the only way an out-of-tree transport contributes them
   (`TransportContribution`).
2. **An embedding may register a kind's declarations with no transport**, with
   `Engine::register_kind`. The engine reads a kind's declarations from the transport registered for
   it when there is one, and otherwise from the kind registered. A transport is the authority over what
   it presents. The kernel registers the kind `message` itself, with ADR 0090 decision 3's
   declarations, so the built-in kind is no longer a special case in the lookup.
3. **Every reading of a kind's declarations that is not I/O reads a registered kind too.** That is
   recording (`provider-shape-session/create` and `observe`) and redaction (what a consumer session
   records and reports). Starting a transport and verifying still need a transport, and fail by name
   as before.
4. **The HTTP component's declarations are one module that builds without its transport**
   (`pact_janus_component_http::kind`), as its matchers do (ADR 0092 decision 3). `HttpTransport`
   answers from it, and the WASM engine registers it for `http`. There is one source, so the two
   embeddings cannot declare different credentials.
5. **A registered kind is not a transport.** It is not declared in-tree, so a requirement naming a
   transport is not satisfied by it, `engine/hello` lists nothing new (it lists no transports), and
   `start-transport` for it is `component-unavailable` as before. A host learns that an embedding can
   record a kind from `provider-shape-session/create`, which answers a session or
   `component-unavailable`.
6. **A test holds the artifact to the native engine.** `engine_wasm.rs` records the sample provider's
   answers, fetched through the HTTP transport's drive role, with a `Set-Cookie` on one of them,
   through the native engine as `janus-engine` registers it and through `janus-engine.wasm` under the
   `wasmtime` crate. Every frame of the conversation must be the same, the shape `finalise` answers
   included.

## Alternatives considered

- **A transport registered with no serve or drive role, whose `start-transport` fails by name.** No
  lookup changes, but the WASM engine would declare an in-tree transport it cannot start. A contract
  requiring the transport would resolve and then fail at `start-transport`, with a different code from
  the one an embedding with no transport gives. Each of the transport's six operations would be a stub
  that answers `operation-unsupported`.
- **The host sends the declarations in `create`.** The engine would need nothing, but every host would
  spell HTTP's credential members. A host that left out `set-cookie` would publish a provider's session
  cookies in its shape, and a host spelling transport knowledge is what the thinness rule exists to
  stop.
- **Building the HTTP transport for `wasm32-wasip2`.** A guest has sockets but no thread for the
  exchange loop (ADR 0062), and recording needs none of the transport's I/O.

## Consequences

- `janus-engine.wasm` records an HTTP provider's shape as `janus-engine` does, so S4.10a's plugin can
  be built against it.
- A contract finalised in the WASM engine redacts HTTP credentials as the native engine's does.
- A transport kind added in-tree later puts its declarations in a module that builds without its I/O,
  so an embedding without the transport can still read them.
- *Tripwire:* revisit if a kind's declarations come to depend on how its transport was started (its
  options), since a registered kind has no options to read them from.
