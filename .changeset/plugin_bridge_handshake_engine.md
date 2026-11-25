---
engine: patch
---

# The handshake schema defines the `pact-plugin` capability a plugin bridge declares

The component-interfaces handshake schema now defines `PactPluginCapability`, the `pact-plugin` member of
a handshake's `capabilities` (component-interfaces spec §9.3.1, ADR 0109). It is what
`janus-plugin-bridge`, the new bridge for today's pact-plugins (`plugin-bridge/`, staged plan S5.2c),
declares about the plugin it serves: the plugin's name, version and interface version, and every entry of
its catalogue, with the content types a content matcher claims and the action a served entry is reached
through. Its content matcher is the action `pact-plugin-<name>:compare-contents`, whose argument the spec
section describes.

Nothing the engine does changes: no engine reads the capability or declares a bridge yet (S5.2d). The
bridge is not part of a release yet (S5.2h).
