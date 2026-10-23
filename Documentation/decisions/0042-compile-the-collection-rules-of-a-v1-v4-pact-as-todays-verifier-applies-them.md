# 0042 — Compile the collection rules of a v1–v4 pact as today's verifier applies them

- **Status**: accepted
- **Date**: 2026-10-23
- **Plan tasks**: S1.24 (the build); S1.25 (`janus upgrade`, which this leaves as it was)
- **Evidence**: pact-reference's `pact_matching` at the version `pact_verifier_cli` 1.3.5 uses
  (`compare_maps`, `compare_lists`, `CoreMatchingContext::match_keys`,
  `matchingrules::compare_maps_with_matchingrule`, `compare_lists_with_matchingrule`, the
  `DoMatch` implementations for JSON values, slices and maps); `pact_models`' `is_values_matcher`,
  `can_cascade` and its reading of `arrayContains` variants; S1.1's `matching-rules/` fixtures

## Context

Four v1–v4 matching rules are about a collection's entries rather than one value: `Values` (v3),
and `EachKey`, `EachValue` and `ArrayContains` (v4). The legacy compiler compiled each to a
`flow:error` node, so a pact that used one failed every interaction it was in, where today's
verifier passes it. pact-js writes `eachKeyMatches`, `eachValueMatches` and `arrayContaining` with
them, and pact-jvm's `eachKeyLike` writes `values`. No pact-specification case covers them, which is
why the gap never showed in the 741 cases the legacy harness runs.

Today's verifier applies them in ways no shape operator does:

- On an object's own path, `Values`, `EachValue` and `ArrayContains` (`pact_models`' "values
  matchers") compare **every entry of the reply**: its value against the pact's value for that key,
  or the pact's first value, by key order, for a key the pact does not give. The keys themselves are
  not compared, so a key the pact gives and the reply lacks is not a mismatch. Rules are looked up at
  the reply's key. `EachValue` first replaces the whole matching context with its own rules, at
  `<path>.*`.
- `EachKey` applies its rules to every key of the reply, skips the check that the pact's keys are
  there, and compares the pact's own members only where the reply has them.
- On an array, `ArrayContains` requires each variant (the pact's element at the variant's index,
  under the variant's own rules alone) to match **some** element of the reply. Elements are not
  claimed: one element can satisfy two variants. A variant with no `rules` is its element compared
  by equality. `EachValue` holds every element to its rules, and checks nothing when it cascaded.
- Cascaded down to an object, none of them changes the walk. On a single value, `Values` is
  equality and the other three accept anything in a JSON document.

## Decision

1. **The legacy compiler compiles the four as today applies them**, from the functions above, for
   a body's objects and arrays: JSON, and any document a content component decodes into one.
2. **The values matchers on an object are a `flow:for-each` over its entries**, whose template
   dispatches on `~>.key` with `flow:if`: the pact's value for each of its keys, then the first value
   for each key a rule names below the object, then the first value for any other key. That last
   branch looks rules up at a key no rule names, so only a `*` rule reaches it, as only a `*` rule
   reaches a key today. `EachValue`'s branches are compiled under its own rules only. A branch for a
   key is labelled as that member (`$.tags.red`), and the last as any entry's value (`$.tags{*}`), the
   shape language's `each-entry` segment.
3. **`EachKey` is a `flow:for-each` holding each key to its rules** (labelled `{key}`, as an
   `each-entry`'s keys are), beside the object's members compared under `flow:if(check:exists(…))`.
4. **`legacy:array-contains` is one variant: "some element satisfies this template".** The action
   was in the plan grammar from the start and nothing ever emitted it; it took a value and compared
   by equality. It now takes the variant's template, compiled under the variant's rules, and
   optionally the variant's index and element, to fail with today's message. It reports only the
   variant when none matched, not what each element that is not the variant failed on. The template
   is labelled `[?i]` after the array's path (`$.items[?0]`): whichever element satisfies variant `i`.
   The shape language's segments have no word for that, `[i]` naming the reply's `i`th element and
   `[*]` every element, so plan-grammar §4.4 specifies it, as a legacy-only label.
5. **Each rule in a list is applied on its own**, as today: on an object, every rule is required
   whatever the list's logic says; on an array, the list's `AND` or `OR` combines them.

## Alternatives considered

- **Compile `ArrayContains` to the shape language's `match:contains`.** It exists, and it is
  greedy and claims a distinct element per entry, which is right for a shape. Today does not
  claim, so a reply with one element that satisfies two variants passes today and would fail.
- **Compile the map rules to the shapes `each-entry` compiles to.** `each-entry` holds every value
  to one shape. Today compares each value with the pact's value for its key, falling back to the
  first, and looks rules up at the actual key; one template cannot say that.
- **Leave them refused, and say why.** A provider moving over would find its pacts failing for no
  reason it could fix, which is the cost stage 1 exists to remove.

## Consequences

- A pact using any of the four in a body verifies as it does today. S1.1's four `matching-rules/`
  fixtures, a pass and a fail for each rule and 150 mutations of their replies, agree with today's
  verifier on every verdict, and the failure messages for `ArrayContains` are today's word for word.
- Declared on text (a path, a header, a query or form value, an XML element), they are still refused
  as a documented gap: today treats each of those differently (a query parameter's values are a
  list, a header's a string), and no fixture shows what a pact there expects.
- `janus upgrade` still converts the map rules into an `each-entry` shape and `ArrayContains` into a
  `contains` shape, which are approximations: the contract it writes may not reach the pact's
  verdict. Two-path agreement for them belongs with S1.25's other conversions.
- A mismatch inside an entry or an element template is reported at a relative path (`~>.value.id`),
  as one inside any rule-driven array already is. Which key or index it was is not said; the CLI
  output review after stage 1a takes that up.
- **Tripwire**: if a pilot provider's pact uses one of the four on text, the text slots get the same
  treatment, from a fixture of that pact.
