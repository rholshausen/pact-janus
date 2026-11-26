---
engine: minor
---

# A v1–v4 pact's plugin content is verified through janus-plugin-bridge

An interaction that needs a pact plugin is no longer always refused (ADR 0110). Given
`options.plugin-bridge` on `verification/verify` or `verification/explain` (a component declaration but
its name, saying how to start `janus-plugin-bridge`), the engine declares `pact-plugin-<name>` for each
plugin a pact's metadata names, unless `target.components` declares that name; a pact fetched from a
broker or by URL is declared for when it arrives. A body a bridged content matcher claims compiles to
its bridge's `compare-contents`, and a request body with generators is filled by its `generate-content`,
as today's verifier fills it. `janus verify` and `janus explain` set the option from `JANUS_PLUGIN_BRIDGE`,
else a `janus-plugin-bridge` beside `janus`.

An interaction no bridge can verify is refused as before, `component-unavailable` with `details.reason`
`pact-plugin`, and now `details.why`: `no-bridge`, `transport`, `rules` or `kind`. Its message says where
the bridge was looked for.

One plan node can report several mismatches: plan-grammar §2.3's `error` result may carry `mismatches`,
each a `message` and an optional `at`, and a run reports each as a mismatch of its own. The engine
protocol's `Mismatch` gains `at`, where the node located it inside the value, in its own words, and a
text report writes `<path> (<at>): <message>`. A component's result `value` tagged `base64` is now read as
octets.
