---
engine: patch
---

# A v1–v4 `eachValue` rule applies the rules below its values, as pact_verifier_cli 1.5.0 does

S1.1's harness now pins `pact_verifier_cli` 1.5.0, which fixes pact-reference#586 (ADR 0107,
rholshausen/pact-janus#16). A v4 `eachValue` rule on an object or an array used to hold the values to
its own rules alone. Rules written below the values were ignored: `$.chips.*.amount`,
`$.items[*].amount`, or a rule for one key such as `$.chips.main.amount`. That is how pact-js and
`pact_ffi` write a matcher nested in an `eachValueMatches` example. `janus verify` now applies them,
as today's verifier and pact-jvm do. The `eachValue` rule's own rules are merged into any rule already
written for the values (`$.items[*]`), or replace one whose `combine` is `OR`. **A pact that passed can
now fail**, at the nested rule, and its plan names the rules it now runs.

`janus upgrade` converts with the same rules, so the upgraded contract reaches the pact's verdict. A
map's `each-entry` has one shape for every value, so a rule for one key's value is converted from the
pact's first member. A rule for that member's key then holds every value, reported as `rule-narrowed`.
A rule for any other key is dropped, reported as `rule-unmapped`. A `values` map is converted the same
way.
