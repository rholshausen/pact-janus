---
typescript-sdk: minor
---

# The verifier declares janus-plugin-bridge for pacts that name a pact plugin

`Verifier` takes `pluginBridge`, the path of `janus-plugin-bridge`, and without it reads
`JANUS_PLUGIN_BRIDGE` (ADR 0112). When either names a bridge, `verification/verify`'s options carry
`plugin-bridge` as `janus verify` sends it, so a provider test verifies a protobuf, gRPC or CSV pact
through the bridge where it was refused before. A `plugin-bridge` in `options` is sent as given. The
pact-js provider API (`pact-janus/verifier`) gets the bridge from the variable. The bridge ships in each
engine release: use the one from the release of the engine the SDK runs.
