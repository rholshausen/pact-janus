---
engine: minor
---

# An interaction carried over a pact plugin's transport is verified through janus-plugin-bridge

A v1–v4 interaction carried over a plugin's transport (the protobuf plugin's `grpc`) is no longer refused
(ADR 0111). The bridge contributes the transport as a kind it drives, and one `transport/send` is the whole
exchange: the plugin builds the request from the pact, calls the provider and compares its answer, and
the reply is its verdict, which the run reads through the bridge's `<component>:verdict` action and
nothing else. Each of the plugin's mismatches is reported with its path as `at`; a plugin that fails the
exchange is the send's `transport-failed`. `details.why: transport` now means a bridge that does not drive
the transport.

A `target.transports` binding may say `optional: true`. When no loaded component provides its kind it is
held rather than failing the call, and started the first time an interaction of its kind is verified if a
component loaded by then provides it, such as a fetched pact's plugin bridge. A plugin transport with no
binding of its own is started with the first binding's options, as today's verifier reaches it at the
provider's host and port. `janus verify` binds each `--transports` entry but `http`, and a `--transport`
other than `http` or `https`, optionally, so an entry for a transport no pact uses is harmless, as today.
