---
jvm-sdk: minor
---

# The verifier declares janus-plugin-bridge for pacts that name a pact plugin

`Verifier.withPluginBridge(program)` names `janus-plugin-bridge`, and without it the verifier reads
`JANUS_PLUGIN_BRIDGE` (ADR 0112). When either names a bridge, `verification/verify`'s options carry
`plugin-bridge` as `janus verify` sends it, so a provider test verifies a protobuf, gRPC or CSV pact
through the bridge where it was refused before. A `plugin-bridge` in `withOptions` is sent as given.
`JanusProviderExtension` gets the bridge from the variable. The bridge ships in each engine release:
use the one from the release of the engine the SDK runs.
