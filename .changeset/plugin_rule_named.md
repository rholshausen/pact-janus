---
engine: patch
---

# A rule nobody implements upgrades to an operator that fails by name

`janus upgrade` and `add-pact-interaction` write a v1–v4 rule that `pact_models` reads as a plugin's,
in a pact that names no plugin, as the `pact` component's operator of the same name, with the rule's
configuration as members: pact-jvm's `ignore-order` becomes `{ "shape": "pact:ignore-order", "min": 1,
"example": [...] }` (ADR 0073, staged plan S2.17). Nothing provides it, and no requirement is written,
so the interaction fails by name, as today's verifier and `janus verify` fail the pact, and the rest of
the contract verifies. It is reported once, as `rule-unmapped`, where the rule is. On an array it was an
`each-like` of the first element with nothing reported, which admitted replies today fails and refused
the pact's own example; on a scalar it was `equality` on the example. As one alternative of an `OR`
beside others, it is left out, since it never matches. A consumer test whose request uses pact-jvm's
`unorderedArray` is now refused by name at the mock. A pact verified as a pact is unchanged.
