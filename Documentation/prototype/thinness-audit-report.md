# Plan task 6.5: the thinness audit, and the AI-regeneration trial

**Date:** 2026-09-20. **Subject:** `sdks/typescript`, `sdks/jvm`, `tools/thinness`, and one
specification change made to be regenerated from.
**Claims under test:** the RFC's, twice. That **SDKs are thin** — no matching logic, no
orchestration beyond the protocol — and that **"one team, eight SDKs"** is achievable because a
behavioural-specification change can be carried into every language by an agent under a
deterministic judge (SDK spec §8).

Closes Phase 6 (Milestone M4). This is an `[explore]` task: it reports evidence, including the
evidence against.

## 1. What was built

| | |
|---|---|
| Measurement | `tools/thinness` (`pact_janus_thinness`), reading `sdks/thinness.json`. Counts every SDK source file by layer, reports total *and* code lines, and `check` fails CI if a hand-written layer exceeds its budget or if any file belongs to no layer. |
| Spec change | One primitive added to `behavioural-spec.json`: **`forbidden`** — "this member must be absent". Plus three conformance cases (`translation/forbidden-bare-node`, `live/forbidden-no-dimension`, `live/forbidden-engine-rejects-misplacement`), written *before* any agent ran. |
| Trial | Two regenerating agents, one per language, each given the changed primitive, its style guide and its own existing source — and each forbidden from reading the other language's SDK. |
| Result | Both SDKs implement `forbidden`; both pass all **35** conformance cases; `cargo run -p pact_janus_thinness -- check` is green. |

## 2. The thinness audit: the numbers

```
pact-janus-typescript (TypeScript)        pact-janus-jvm (Java)
  layer         files  lines   code         layer         files  lines   code
  generated         9   1584    746         generated        82  13965   8486
  idiomatic         5    582    426         idiomatic        19   1958   1386
  embedding         3    193    148         embedding         3    266    190
  integration       1     15      9         integration       2    101     54
  tests            13   1503   1268         tests            14   2443   2048
```

| | generated | hand-written (shipped) | hand-written % |
|---|---:|---:|---:|
| pact-janus-typescript | 746 | **583** | 44% |
| pact-janus-jvm | 8486 | **1630** | 16% |

**The percentage column is the least useful thing in this report, and it is in the tool so that
nobody quotes it without the next sentence.** TypeScript looks three times "less thin" than Java
for a reason that has nothing to do with thinness: the same four schema sets generate 746 code
lines of TypeScript type aliases and 8,486 of Java classes. The honest comparison is the absolute
one — **583 and 1,630 hand-written code lines** for a complete SDK — and even that is inflated for
Java by boilerplate (`Cardinality`, four exception classes, `RequestParts`/`ResponseParts`) that a
language with records-as-you-mean-them would not need.

Layer definitions are in `sdks/thinness.json`, and the tool refuses to run if a source file is not
classified — so the next file somebody adds is a decision, not a silent reclassification.

## 3. What is hand-written, and whether it should exist

The plan asks specifically: what had to be hand-written that the RFC claims shouldn't exist —
matching logic, or orchestration?

**Matching logic: none, in either language.** No helper decides what a value admits. `shapes.ts`
and `Shapes.java` each map a primitive to a shape node and stop; the "literal rules" (`compile`,
`Literals`) are a translation from a language's native map/list/scalar into `object`/`array`/
`equality` nodes, which is the one thing a *language* must do and the engine cannot. Nothing
computes a variant dimension: the SDKs never see one until the engine hands it back, which
`live/shape-dimensions` pins.

**Orchestration: the minimum the protocol requires, and it is visible.** `Janus#execute` is about
25 lines: `add-interaction`, `start-transport` once, `variants`, then a loop that calls
`serve-variant` and the closure and collects failures. There is no decision in it that the engine
did not make — not which variants to run, not whether one passed. It exists because somebody has
to drive a request/response protocol, and the RFC's own `janus.execute(interaction, (mock, variant)
=> …)` shape requires the SDK to hold the loop.

Three things are hand-written that arguably shouldn't be, and all three are honest costs:

1. **A JSON writer in the JVM SDK** (`Json.java`, 91 lines). Contract spec §2.4 wants deterministic
   bytes, so the SDK must control member order; `Map.of` has no encounter order and salts its
   iteration per JVM run. TypeScript pays nothing here because `JSON.stringify` and object literals
   already preserve insertion order. This is a per-language tax the specification cannot remove —
   and Phase 9 finding 6 argues the real fix is for the contract to cross the boundary as canonical
   text, which would delete this file.
2. **Four exception classes and a `Cardinality` enum on the JVM** — ~190 lines that are pure
   language ceremony.
3. **The embedding** (148 and 190 code lines): `Content-Length` framing, subprocess lifecycle,
   stdin-EOF shutdown. Written twice, identical in behaviour. The RFC does not claim this away, but
   it is the clearest candidate for something generated rather than written, and neither SDK has a
   WASM embedding yet — which is where ADR 0003's second binding would double it again.

## 4. The regeneration trial: method

The loop SDK spec §8 defines, run as written. The **specification changed first**: a `forbidden`
entry in `behavioural-spec.json`, and three conformance cases. The cases were written, linted and
**verified against the real engine before either agent started** — `forbidden` at a body root is
refused with `interaction-invalid`, and an interaction whose only operator is `forbidden` yields
exactly one variant where the same interaction with `optional` yields two.

Two deliberate controls on the method:

- **The entry was calibrated, not polished.** A spec author who writes an unusually thorough entry
  for the primitive being measured rigs the trial. The 23 existing `semantics` fields have a median
  of 46 words (33 in the shape category); `forbidden`'s is 107, comparable to `one-of`'s 98 and
  longer than `regex`'s 70 — the upper end of the existing band, because it has three things to say,
  but written in the house style and citing the shape spec rather than restating it.
- **Each agent was blind to the other language**, as in task 6.3, and was told so. Without that,
  "the two SDKs agree" measures nothing.

`forbidden` was chosen as the probe because it *looks* like the mirror of `optional` and is not:
it takes no nested shape, it contributes **no** variant dimension, and it may appear only in a slot.
An agent pattern-matching on `optional` gets all three wrong.

## 5. What the agents produced

Both succeeded on the first attempt, with no intervention. The entire trial diff:

| | files | insertions | idiomatic-layer **code** lines added |
|---|---:|---:|---:|
| TypeScript | 4 + STYLE.md | 23 | **2** (`shapes.ts` +1, `index.ts` +1) |
| JVM | 2 + STYLE.md | 15 | **3** (`Shapes.java`) |

Both got every trap right: a bare node with no `of` and no `example`; no dimension code (neither SDK
computes dimensions at all, so the correct action was none); and — the one that mattered — **neither
added a placement check**, which the entry explicitly forbids and which would have failed
`live/forbidden-engine-rejects-misplacement`. The JVM agent said it would have added that check had
the sentence not been there. That sentence earned its place.

Both independently chose a nullary function over a constant. Only the TypeScript agent wrote that
choice into its style guide unprompted; the JVM agent recorded the new primitive in its style
guide's table but not the reasoning behind its spelling, which is the gap §7(a) closes.

## 6. Review burden

The measurement the plan asks for. **No agent output was rejected or rewritten.** Reviewing meant
re-running both suites myself (35/35 each, confirmed, not taken on report) and reading 38 lines of
diff. What review *added*, beyond the agents' work:

| Review action | Lines | Why the agents could not have done it |
|---|---:|---|
| Spec §3.2: an empty `signature` is normative, and the style guide MUST record the spelling | +5 | Agents are forbidden to edit `Documentation/` — correctly, since this is a change to the format all languages share |
| JVM `STYLE.md`: the nullary-spelling rule the new §3.2 now requires | +6 | Follows from the spec change above |
| Phase 9 finding 7 (below) | +30 | A cross-primitive design question, not a patch |

That is the honest shape of the burden: **the code review was near-zero; the specification review
was where the work was.** Both agents' reports were substantive — each raised the `signature: []`
ambiguity independently, and one found a real composition gap — and reading those reports took
longer than reading their diffs, which is the correct allocation.

## 7. What the trial found

**(a) `signature: []` was ambiguous, and both agents guessed.** Nothing said whether "no arguments"
is a nullary call or a constant. Both chose the call; both justified it well; **neither was
following the specification, because the specification did not say** — and no conformance case can
see the difference, so two SDKs could diverge here permanently with a green suite. Fixed in §3.2:
an empty `signature` is normative, and the style guide must record the spelling. That two agents
agreed is not evidence the gap was harmless; it is evidence the gap is invisible.

**(b) "This header must not be sent" is unreachable from the DSL** — found by the TypeScript agent
reading the `request` and `forbidden` entries against each other, and **confirmed against the
engine**. `request` compiles a header map's shape-helper value to `each-like { items: …, min: 1,
max: 1 }`; shape spec §5.1 forbids a node admitting absence as an `each-like`'s `items`. So
`headers: { "x-trace": forbidden() }` builds a document the engine refuses, pointing at a node the
author never wrote — while the document the author meant (`forbidden` as the member itself) is
already legal and has no spelling. Both SDKs have the gap; neither is wrong, because the
specification does not say what the composition means. Recorded as **Phase 9 finding 7**, and then
decided: option (a), the `request` rule gains an exception for a helper whose node admits absence.
Implementing it showed the defect was wider than the agent's report — `optional` in a header is
broken identically, and more commonly written — so the rule is stated generally (absence applies to
the member; the list treatment goes inside the modifier) rather than as a `forbidden` special case.
Both SDKs implement it, under `session.request.absence-applies-to-the-member` and three new cases.

Finding (b) is the trial's best result. It is not a bug an agent introduced — it is a specification
gap an agent *found*, in the seam between two entries, which is exactly where a one-document-per-
language process loses things. It is worth noting what the agent did *not* find: that the same seam
had already swallowed `optional`, which has been in the DSL since task 6.2 and whose header form has
never built a valid document. The agent found the gap by reading the entry it had just been given
against its neighbour; nobody had reason to read `optional` against that neighbour, and no case
existed to notice. That is an argument for conformance cases at every composition point, not only at
every primitive.

## 8. What this does not show

- **Two instances of the same model are not two implementers.** They agreed on the nullary spelling
  and on everything else, but correlated agreement between two runs of one model is much weaker
  evidence than two humans converging. Task 6.3's result — a JVM SDK written blind, recording
  content identical to TypeScript's — remains the stronger evidence for the specification format,
  and this trial does not supersede it.
- **n = 1, and a small 1.** `forbidden` is a leaf with no arguments. A primitive with real internal
  structure (`each-entry`, or a change to `execute`'s semantics) would test far more, and a *changed*
  or *removed* primitive — where the agent must find every affected call site rather than add one —
  is the case the loop will actually face most often and was not tested at all.
- **The conformance cases existed before the agents ran**, so an agent could in principle have fitted
  the cases rather than read the spec. Mitigating: neither of the two findings above is visible to
  any case, and both agents reasoned explicitly from the entry's prose in their reports. But the
  loop as specified does hand the agent the judge, and a spec author who writes weak cases will get
  code that passes them.
- **These SDKs are small and new.** Nothing here shows what the loop does to a mature SDK with a
  compatibility facade (spec §6) and real users.

## 9. Verdict

**Thinness: supported, with the percentage discarded.** 583 and 1,630 hand-written code lines for a
complete SDK, no matching logic in either, and orchestration confined to ~25 lines that contain no
decision the engine did not make. `cargo run -p pact_janus_thinness -- check` now makes that a
build failure rather than a paragraph, and the budgets it enforces are the ones measured here.

**"One team, eight SDKs": supported in its mechanics, and the economics are clearer than the slogan.**
The spec-side cost of this feature — one entry (10 lines) plus three conformance cases (79 lines) —
was 89 lines — roughly **eighteen times** the code-side cost of 5 code lines across two languages. That ratio is
the whole argument, and it cuts both ways: at two languages the specification work dominates and the
loop barely pays for itself; at eight it is paid once and the per-language cost stays near zero.
The claim is therefore not "SDKs become free" but "**the cost moves into the specification and the
corpus, and stops scaling with languages**" — which is worth saying that way in the RFC, because it
is defensible and the slogan is not.

The loop's real value showed up somewhere the RFC does not claim it: an agent reading one entry
against another found a composition gap two human implementers had already walked past.
