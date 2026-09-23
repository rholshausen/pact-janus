# Plan-fragment stress test

Plan task **8.4** (`[explore]`): "have the 8.1 component contribute plan fragments and a custom action;
check the 2.4 versioning policy holds up — what happens when the component targets plan grammar v0 and
the engine moves to v0.1?" Decisions it forced are [ADR 0022](decisions/0022-a-fragment-replaces-its-slots-plan-and-declares-a-grammar-the-engine-says-it-reads.md);
open questions are [Phase 9 findings](phase-9-findings.md) 21–24.

## Verdict

**The policy holds, once it is stated.** Plan grammar §7.1 had the right principle: fragments are the one
plan-shaped document that crosses a version boundary, so the grammar is a compatibility surface and
action semantics are frozen. But the engine had no way to act on it. It had no version ordering, never
told a component which grammar it reads, and its plan reader ignored the `grammar` member entirely. It
had never asked a component for a fragment, and had no way to run a component's action. With those
built, every skew scenario fails by name, before anything runs, and the designed-for case (an old
fragment on a newer engine) is readable by rule.

**The fragment model is where it does not hold.** Contributing a fragment turns out to mean *replacing*
the slot's plan. So a content component that contributes one becomes a shape compiler for every shape
it accepts, and does it without knowing the variant. The stress test demonstrates the cost: a variant
checked against a fragment passed where the engine's own plan correctly failed. ADR 0022 records the
likely fix, operator-level substitution, as its tripwire.

## Method

**The contributor is the real 8.1 component.** `third-party/janus-csv` gained a `content/compile` that
returns a fragment, and the `matcher` interface's `apply` for three actions: `csv:integer`, `csv:number`
and `csv:boolean`. The fragment is the one a CSV component has a genuine reason to contribute. CSV decodes
every field as a string, so the engine's generic `match:integer` fails every CSV body whose consumer
wrote `integer`. That is why 8.1's own test had to use a regex. The component is still written from the
docs: it follows plan grammar §2, §4 and §5.2 and the plan schema, and mirrors the engine's plan
structure so `explain` reads the same either way. Its fragment compiler is
[`src/fragment.rs`](../third-party/janus-csv/src/fragment.rs), about 150 lines.

**Skew is exercised with a stub.** The real engine reads `v0` only, so a `v0.1` engine cannot be run.
The version rule is unit-tested over synthetic versions (`plan/fragment.rs`). The end-to-end refusals
use a stub content component that answers `compile` with any fragment a test gives it
(`engine/kernel/tests/fragments.rs`).

**The kernel half is durable code.** It asks `content/compile` for each declared slot, validates and
splices the answer, dispatches namespaced actions to `matcher/apply`, and offers `plan-grammar-versions`
in `component/hello`. All of it follows ADR 0022, and it is what any real contributor needs.

## What was tested

| Scenario | Result |
|---|---|
| CSV contract with `items: integer`, consumer run → verification against the sample provider | **verified**; the executed plan shows `csv:integer` where the generic one said `match:integer` |
| a readable fragment's own action, run through `apply` in a verification | verified on a matching value, failed on another; the value arrives as a `MatchValue` with its path |
| fragment declares **no** grammar | refused at `add-interaction`: `grammar-skew`, "does not say which plan grammar" |
| fragment declares **`v0.1`**, engine reads `v0` | refused: `grammar-skew`, naming `v0.1` and what the engine reads |
| fragment declares `v1` | refused: `grammar-skew` |
| `v0` fragment on a `v0.1` engine | **readable by rule** (unit test; no `v0.1` engine exists to run) |
| `v0` fragment using `expect:unique`, a core action `v0` does not have | refused: `fragment-invalid`, "not a core action of the grammar the fragment declared" |
| fragment using another component's action (`csv:integer` from `stub`) | refused: `fragment-invalid` |
| fragment using its own action that its handshake did not contribute | refused: `fragment-invalid` |
| fragment resolving a path outside its slot | refused: `fragment-invalid` |
| fragment that is not a plan node | refused: `fragment-invalid`, "unknown plan node kind" |
| a verification whose fragment is skewed | refused before the run starts, as `component-unavailable` |
| a component named `expect` (or `match`, `check`, `convert`) | refused at load: `component-invalid` |
| an engine that sends no `plan-grammar-versions`, or only ones the component does not write | the CSV component contributes no fragment; the generic plan runs |

## Answering the question: the engine moves to v0.1

Suppose `v0.1` adds a core action, `expect:unique`. That is exactly what a CSV component might want for
a key column, and until then it would contribute its own `csv:unique`.

- **A `v0` fragment on the `v0.1` engine keeps working**, unchanged. `v0.1` reads `v0` by rule, and
  `csv:unique` is namespaced, so the new core action cannot collide with it.
- **A component that upgrades to `expect:unique`** declares `v0.1`. On a `v0.1` engine that is fine. On
  a `v0` engine it is refused at the first interaction, naming the skew. But it should never get that
  far: `component/hello` lists `v0`, so the component writes its `v0` fragment with `csv:unique`
  instead. One component serves both engines.
- **What would have broken before 8.4**, and no longer does:
  1. The plan reader ignored `grammar`, so a `v0.1` document was read as `v0`, which is exactly the
     silent reinterpretation §12.3 forbids.
  2. A component had no way to learn the engine's grammar, so it could only guess and be refused.
  3. A component *named* `expect` could have contributed `expect:unique` in `v0` and then collided with
     `v0.1`'s core action of the same name. That is the one hole in "namespaced names never collide
     with core ones".
  4. A fragment with no declared grammar was indistinguishable from a current one.

## Findings

1. **A fragment replaces the slot's plan, and cannot join it** (ADR 0022 decision 1). This is the finding
   that shapes the rest. The spec said "spliced at path". The only reason a *content* component
   contributes a fragment is that its content type changes an operator's meaning, and a generic check
   left beside the component's own still fails. So the component compiles the slot, and inherits plan
   grammar §5.1's obligation for every shape it accepts. The CSV component covers one structure and
   eight operators, declines everything else, and even that took ~150 lines mirroring the engine's
   compiler.

2. **`compile` is not told the variant, and it shows** (Phase 9 finding 21, demonstrated). The engine
   compiles one plan per variant, pinning each dimensional operator to its variant's point:
   `each-like 1..3`'s maximal variant becomes `expect:count 3`. `content/compile` receives the slot's
   authored shape only, so the fragment checks `expect:size 1..3` for every variant. Against a provider
   with one order, the three-row variant fails under the generic plan, as it should, and *passes* under
   the fragment. The variant proved nothing, and the run says verified
   (`a_fragment_compiled_without_the_variant_widens_a_pinned_variant`). Variant-semantics §5.2 is
   precisely the rule this breaks.

3. **A fragment cannot see the octets** (finding 22). The spec's and worked example's picture was a
   fragment that *decodes*: `pipeline($.response.body, csv:parse)`, so the component's decoding shows
   in `explain`. The engine decodes a declared slot in the resolver, before the plan runs. No plan has
   ever contained `json:parse`, and a fragment only ever resolves the decoded document. The worked
   example is corrected, and the design question is Phase 9's: either resolvers stop decoding and
   `<component>:parse` becomes the plan step the grammar spec describes, or the spec stops promising
   it.

4. **`explain` loads no components** (finding 23). `verification/explain` takes an interaction and no
   `components`, so for a CSV contract it prints the generic plan, the one with `match:integer`, which
   is not the plan `verify` runs. The executed plans `verify` emits are right. The RFC's inspectability
   claim rests on `explain`, and for any contract with a fragment it now describes the wrong plan.

5. **`Apply.config` has no source in the grammar** (finding 24). `matcher/apply` takes `values` and a
   `config` "as the compiled plan node carries it", and a plan node carries no config. An action's
   children are its arguments. ADR 0022 maps the first to the value under test and the rest to
   `config.arguments`. The existing corpus case `unknown-component-action` shows that is what the shape
   compiler already emits for a component *operator*: its extra members become a second, object-valued
   child.

6. **A component action that fails is reported as a mismatch.** Spec §11.3 says a component error is
   `component-failed` at the engine boundary. A plan node has nowhere to put one, so it becomes the
   node's error, labelled with the component and code. This is Phase 9 finding 14 again, reached from
   the other side.

7. **Only declared components are asked for fragments.** The in-tree JSON component contributes none,
   so asking it changes nothing today. But a built-in that did contribute one would have to be asked
   too, and would be the first case where spec §9.1's "no privileged path" cut the other way: a
   built-in *under*-privileged.

## Corpus

One golden case changed, in the same commit as the behaviour, per CLAUDE.md:
`corpora/shapes/unknown-component-action`. The verdict is unchanged (mismatched). The message now says
no component in scope contributes the action, where it used to say the action was unknown, and the
mismatch now carries the path the value was resolved from. No other case moved. The shape compiler
emits the same plans, and splicing happens only for slots a declared component compiles.
