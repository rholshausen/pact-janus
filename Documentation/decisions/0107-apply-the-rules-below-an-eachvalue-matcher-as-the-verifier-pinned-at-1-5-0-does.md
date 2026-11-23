# 0107 — Apply the rules below an `eachValue` matcher, as the verifier pinned at 1.5.0 does

- **Status**: accepted
- **Date**: 2026-11-23
- **Plan tasks**: S1.24 (the legacy compiler's collection rules), S1.25 (`janus upgrade`), S1.1 (the
  differential harness, whose pin moves); rholshausen/pact-janus#16
- **Evidence**: pact-reference's `pact_matching` 2.2.0, which `pact_verifier_cli` 1.5.0 builds:
  `matchingrules::each_value_context` and its two callers, `compare_maps_with_matchingrule` and
  `compare_lists_with_matchingrule`, from commits `1f1c2e61` (apply the matchers nested under an
  `eachValue` matcher, pact-reference#586), `2e47687e` (apply an `eachValue` rule once when the
  values already have it) and `17aa4910` (merge the `eachValue` rules into any existing rules for the
  values; pact-jvm `56003751`); S1.1's `matching-rules/each-value-nested` fixture and its mutations

## Context

[ADR 0042](0042-compile-the-collection-rules-of-a-v1-v4-pact-as-todays-verifier-applies-them.md)
compiled a v4 `eachValue` rule as `pact_verifier_cli` 1.3.5 applied it: "`EachValue` first replaces
the whole matching context with its own rules, at `<path>.*`". A rule written below the values
(`$.chips.*.amount`, `$.items[*].amount`), which pact-js and `pact_ffi` write for a matcher nested in
an `eachValueMatches` example, was never applied. pact-reference#586 called that a bug, and
`pact_verifier_cli` 1.5.0, released on 2026-11-13, is the first release with the fix. S1.1's harness
pins the verifier it agrees with, so moving the pin to 1.5.0 made the `each-value-nested` fixture,
kept as a tripwire for this, disagree on six interactions and the run.

The fixed verifier builds the values' context in `each_value_context`:

- it keeps every rule whose path is longer than the collection's and whose first tokens match the
  collection's path exactly, so a rule for the values, for a member of every value, or for one key's
  value (`$.chips.main.amount`) still applies; a rule at the collection's own path or above it does
  not, as before;
- the `eachValue` rule's own rules are merged into each group already defined for the values, one
  token below the collection and ending in `*` or `[*]`, its rules first and then the group's, none
  twice. Only one group is chosen for a path, so a separate group at `<path>.*` would lose to a
  `<path>[*]` one. A group whose logic is `OR` is replaced, because merging would make the
  `eachValue` rules optional;
- with no such group, its rules are a group at `<path>.*`, as before.

When it is applied is unchanged: on an object only when the rule did not cascade, and on an array
not at all when it cascaded.

## Decision

1. **The legacy compiler builds an `eachValue` rule's context as `each_value_context` does**
   (`each_value_category` in `engine/kernel/src/legacy/compile.rs`), for both an object's entries
   and an array's elements. This supersedes ADR 0042's reading of `EachValue` in its context and its
   decision 2 ("`EachValue`'s branches are compiled under its own rules only"). The rest of ADR 0042
   stands.
2. **A key a rule names below the values gets its own branch**, as ADR 0042 decision 2 already
   compiles for `Values`: the merged context includes rules like `$.chips.main.amount`, so the
   entry template dispatches on that key.
3. **`janus upgrade` converts with the same context**, for a map's `each-entry` values and a list's
   `each-like` items. `each-entry` has one shape for every value, so a rule for one key's value
   cannot be said. When a rule below a map's values names a key, the values are converted at the
   pact's first member's key, from which they are already converted: a rule for that key then holds
   every value, a false failure rather than a false pass, and is reported as `rule-narrowed`; a rule
   for any other key is dropped and reported as `rule-unmapped`. Without one, the values are
   converted at any key, as before. This holds for `values` maps too, which share the conversion.

## Alternatives considered

- **Port the V2 engine's fix (`e441f98c`) instead.** Janus needs no interpreter change, since
  `flow:for-each` already walks an object's entries by key, and the V2 engine still does not apply a
  rule for one key below the values, which the default engine, and so the verifier, does.
- **Keep 1.3.5's reading and record the six interactions as deliberate differences.** The reading
  was a bug upstream fixed, pact-js already writes the rules expecting them to apply, and pact-jvm
  applies them too. Keeping it would pass replies both of today's verifiers fail.
- **Drop a rule for one key in the upgrade, and list the verdict.** Sound, but it passes a reply
  the pact fails. Applying the first member's rule to every value fails it, as the pact does, at
  the cost of failing a reply the pact would pass, which is the direction the upgrade already takes
  for `eachKey` and `arrayContains`.

## Consequences

- A pact whose `eachValue` rule has rules below its values verifies as `pact_verifier_cli` 1.5.0
  and pact-jvm verify it: a reply that passed can now fail, at the nested rule. The
  `each-value-nested` fixture and its 155 mutation verdicts agree with today's verifier, for the
  pact and for its upgraded contract, and its ledger entries are gone.
- `corpora/legacy/v4-each-value-nested-map` and `-list` (with their mismatches) pin the plans: the
  list case's items group merges the `eachValue` rule's `type` into a `$.items[*]` rule of its own.
- **Tripwire**: an upstream change to `each_value_context`, or to when its callers apply it, shows
  in the harness when the pin moves; this ADR is then superseded the same way.
