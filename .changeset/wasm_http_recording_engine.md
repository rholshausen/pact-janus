---
engine: minor
---

# `janus-engine.wasm` records an HTTP provider's shape, and redacts HTTP's credentials

Staged plan S4.9, ADR 0094. `provider-shape-session/observe` reads an observation through what its
kind declares: which part answers, the outcome slots that keep a 404 from widening a 200, and the
credential members recorded only as present. The engine found those by asking the transport for the
kind, and the WASM component has none, so an `http` recording there was `component-unavailable`. For
the same reason, a v1–v4 pact upgraded there kept its `Authorization` and `Set-Cookie` values, which
the native engine records as `[redacted]`.

- The kernel holds what a kind declares as `KindDeclarations`, and an embedding with no transport
  for a kind can register them with `Engine::register_kind`. A transport registered for the kind
  still declares it. The kernel registers `message` itself.
- `pact_janus_component_http::kind` is the HTTP kind's declarations, built without the `transport`
  feature. `HttpTransport` answers from it, and the WASM component registers it for `http`.
- A registered kind is not a transport. `engine/hello` is unchanged, and `start-transport` for it is
  still `component-unavailable`.
- The release checks that the component records the sample provider's answers, frame for frame, as
  the native engine does, and upgrades a pact with the same redactions.
