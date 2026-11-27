---
engine: minor
---

# A pact plugin's field-level matching rule and generator are verified through janus-plugin-bridge

A v1–v4 interaction with a matching rule or request generator that a pact plugin provides at the field
level, such as pact-plugins' `creditcard` rule on `$.card.number`, is no longer refused (ADR 0113). For a
plugin of interface version 2, the bridge contributes `<component>:match-field` for each `matcher`
catalogue entry and `<component>:generate-field` for each `generator` entry, and names each as the entry's
`action` under the `pact-plugin` capability. The engine compiles a rule that a bridge provides to
`match-field` on the value at the rule's path. The action has two arguments: the pact's example, and the
rule's `name`, `values`, `mismatch-type` and slot `root`. The engine fills a plugin request generator
through `generate-field` before the request is sent. The plugin's mismatch is reported in its own words at
the value's path, as today's verifier reports it.

`details.why: rules` now refuses only an interaction whose rule or generator none of its plugins'
bridges provides, and names those rules and generators.
