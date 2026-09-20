# Plan task 6.4: the conformance suite seed, and what it found

**Date:** 2026-09-20, with §7 added the same day, after the recommendations were implemented.
**Subject:** [`conformance/`](../conformance), the two drivers, and the checker.
**Claim under test:** ADR 0017's — that "conformant" can be a claim a build makes. Task 6.3's
report closed on "only half testable": the bar is "pass every conformance-suite case its
`conformance` list names", and there were 53 ids and no cases, so every judgement call the JVM
implementer made passed the scenario they invented for it, because they wrote both.

## 1. What was built

| | |
|---|---|
| Corpus | 32 cases under `conformance/cases`, covering all **58** conformance ids in `behavioural-spec.json`: 18 translation, 8 lifecycle, 3 variants, 3 live. Each is one JSON document naming the ids it covers and quoting the sentence it pins. |
| Case format | `conformance/schemas/v1/conformance-case.schema.json`. A case is data — a DSL chain written as JSON, a scripted engine, and what must come of it — not a Gherkin sentence each language re-reads. §3 says why. |
| Drivers | `sdks/typescript/test/conformance/` (Vitest) and `sdks/jvm/sdk/src/test/java/io/pact/janus/sdk/conformance/` (JUnit `@TestFactory`). Each knows only how its language *spells* a primitive; about 400 lines each, test-scope, outside the published surface. |
| Checker | `tools/conformance` (`pact_janus_conformance`). `lint` holds the corpus to its schema and to the behavioural specification's ids — an id with no case fails the build. `check` reads each language's report: every case accounted for, and passed. |
| CI | The corpus lints in the `rust` job; both SDKs run it in their own test runs and write reports; `check` gates the `sdks` job on both. |

**Both SDKs pass every case.** `cargo run -p pact_janus_conformance -- check` reports
`pact-janus-typescript: 32 of 32` and `pact-janus-jvm: 32 of 32`. The counts are from after §7;
the first run, which §2–§6 report, had 27 cases and 53 ids.

## 2. The headline result: the two SDKs agree, including on contract content

`live/rfc-order-contract` runs the RFC's consumer example end to end against the real engine and
compares the **interaction content** the contract records — `states`, `parts` and the whole
`selection`, all 8 selected variants with their assignments and the exercised values — against a
reviewed copy of the engine's own record. `metadata` and the parties are not compared (ADR 0017
commitment 2).

The JVM SDK was written from the specification alone, by an implementer who never read the
TypeScript source (6.3). It records **the same content, member for member**, including variant ids
and the order of the selection. That is the strongest available evidence for B1's second half, and
it is evidence 6.3 could not produce: agreement measured against the engine's answer, not against
the other SDK's output.

Three further live results worth naming, because each is a place an SDK could have quietly
reimplemented the engine and did not:

- `live/shape-dimensions`: the presence, value, alternative and cardinality dimensions come back
  from the engine, derived from the document each DSL emitted — `response.body.shippedAt#presence`
  and friends, not anything the SDK computed.
- `live/one-of-engine-validates`: a `one-of` whose alternatives bind no discriminator is accepted by
  both DSLs and rejected by the engine, in the engine's own words. Neither SDK checks it, which is
  what the `one-of` entry requires.
- `lifecycle/finalise-writes-contract`: both write the engine's contract with members in the engine's
  order and exactly one LF, though they get there differently — the JVM slices the bytes out of the
  response frame, TypeScript re-serialises the parsed document (Phase 9 finding 6).

## 3. Why cases are data, not Gherkin

The plan says to grow from `pact-compatibility-suite`, and the arrangement is kept: one shared
corpus, a thin per-language driver, one fixture set. The medium is not. In a Gherkin suite the
scenario is prose and the *step definitions* are per language — so "the header name is lower-cased"
is re-read once per language, and two implementations can satisfy the same sentence differently,
which is the drift the suite exists to remove. Here the expectation is the document itself:

```json
"expect": { "at": { "/parts/request/headers/members/accept":
  { "shape": "equality", "example": ["application/json"] } } }
```

A driver cannot read that two ways. It also costs both SDKs nothing in dependencies — no Cucumber
runtime in a prototype whose whole claim is that SDKs are thin.

The cost is honest: a case can only exercise what its template language can say. Flags on a compiled
regex object, ordered-map literals, typed-language refusals — none are expressible, and they stay in
each SDK's own tests. §5 lists what that leaves uncovered.

## 4. What the suite found

### 4.1 Fixed: TypeScript could not say who withheld the contract

`finalise` withholds a contract for two different reasons — the engine returned none, or the engine
returned one and the SDK refused to write it because a test failed (Phase 9 finding 4). The JVM's
`ContractWithheldException` distinguishes them (`engineWithheld()`); TypeScript's
`ContractWithheldError` did not, so a user could not tell "your consumer failed" from "the engine
found a mismatch" without parsing the message.

Both are cases here — `lifecycle/finalise-withholds-contract` and
`lifecycle/failed-test-withholds-contract` — and the suite could not tell them apart in TypeScript.
Fixed by adding `engineWithheld` to the error. This is a small thing, but it is exactly the class of
gap ADR 0017 predicted: a distinction the specification draws in prose, which one implementation had
and the other did not, found by a case rather than by a reader.

### 4.2 Not decided: two real divergences the specification does not settle

Both are in `request`'s name-to-list rule, and both were named in 6.3's report §2.3 as unstated.
At the time of the first run they were still unstated, so there was no case for them: a case that
decided them would have made the suite normative beyond the document every implementer reads
(`conformance/README.md` §7). **§7 below settles both, and both now have cases.**

| | TypeScript | JVM |
|---|---|---|
| `headers: { Accept: "first", accept: "second" }` | the last wins, silently: one `accept` slot with `["second"]` | refused at the call: "header 'accept' is already declared" |
| a non-string, non-list, non-helper value (`header("X-Count", 3)`) | builds `each-like` over `equality 3` — a shape the transport's string values can never match | refused at the call |

Both SDKs pass every conformance id they name. The first silently drops a declaration the author
wrote; the second builds a document that cannot match and hands it to the engine (defensible under
SDK spec §2.1 — rejecting is the engine's call — except that the engine accepts it, so nobody
reports anything and the variant simply never verifies).

**Recommendation** (behavioural spec `request`, restating 6.3's recommendation 2): say what happens
when two header names collide after lower-casing, and what happens to a value that is none of the
three forms. Then add the two cases. **Done in §7.**

### 4.3 Confirmed and bounded: contract bytes are not a conformance bar

`session.finalise.contract-bytes-unaltered` is the id whose name most invites a byte-for-byte test.
Its case checks what the specification actually requires — compact JSON, one trailing LF, members in
the engine's order, nothing re-indented or re-escaped, with non-ASCII and quote characters in the
payload — and deliberately does not test number spellings or integer-like member names, where an SDK
that parses the frame can only reconstruct what the engine wrote (Phase 9 finding 6). ADR 0017 puts
bytes outside conformance, so the case says so in its `note` rather than leaving the next reader to
wonder whether the gap was an oversight.

## 5. What the first 27 cases did not cover

Stated plainly, because ADR 0017's own "Harder" says an under-specified category here is a silent
hole in what "conformant" guarantees:

- **Per-language refusals**: `regex` flags that a pattern text cannot carry (behavioural spec
  `regex`), what counts as a scalar or a map in a typed language, ordered-map literals. Each SDK
  tests its own; the corpus cannot.
- ~~**Protocol failures during a run**: `variant-budget-exceeded`, `session-not-found`, and an engine
  that dies mid-loop.~~ Covered as of §7 — and the third turned out to be a fourth divergence, not a
  coverage gap.
- **The JUnit/Vitest integrations**: that `finalise` runs after the suite's last test is a case
  (`session.finalise.always-runs`), but that the *framework hook* runs it is each SDK's own test.
- **Thinness**: unchanged from ADR 0017 commitment 3 — a suite that samples scenarios cannot rule out
  a hand-rolled reimplementation that agrees on every sampled case. That is task 6.5's audit, by
  code inspection, and conformance passing is not evidence of thinness.

## 6. Verdict on ADR 0017

**The claim holds, and the exercise was worth doing for 6.3's sake alone.** "Conformant" is now a
command that exits non-zero: the corpus lints against the behavioural specification's ids, each
language reports, and the checker fails on a missing or failed case. The interesting result is not
that the suite failed something — it failed one thing, §4.1 — but that two independently written
SDKs, one of them written blind from prose, agree on 27 scenarios and on the full recorded content
of the RFC example. 6.3's verdict was "the format transmits the happy path reliably and leaves the
edges to the implementer". The suite now says the same thing more precisely: the edges it can reach
agree, and the two that did not (§4.2) were edges the specification never described — §7 is what
the specification says about them now.

**Recommended next**:

1. ~~Settle §4.2 in `behavioural-spec.json`, then add the two cases.~~ Done — §7.
2. ~~Add cases for the protocol failures in §5 as the scripted engine already supports them.~~
   Done — §7.
3. Task 6.5 measures against this corpus: a DSL change lands in the behavioural specification, an
   agent regenerates both idiomatic layers, and *this* is what says whether the result is correct.

## 7. The recommendations, implemented

[ADR 0019](decisions/0019-an-sdk-refuses-what-it-cannot-spell-and-stops-when-the-engine-does.md)
settles §4.2's two questions and a third the protocol-failure cases turned up while being written.
All three are one question — **what may an idiomatic layer decide by itself?** — and the answers are
now in `behavioural-spec.json`, which is what an implementer reads, with five new conformance ids
and their cases. Both SDKs changed; neither was simply declared right.

| Question | Settled as | Who changed |
|---|---|---|
| Two header names colliding once lower-cased | refused at the call: only the SDK can report a collision it created by lower-casing, and the engine never sees the dropped declaration | TypeScript (it kept the last, silently) |
| A value that is none of the three forms | written as the one string that spells it, where every language spells it the same way — a whole number up to 2^53 − 1, a boolean — and refused otherwise | both: TypeScript gained the refusals, the JVM gained the conversion |
| An engine error inside the variant loop | ends the run at once with the engine's error; the every-variant-runs guarantee is about what a *test* decided | the JVM (it recorded the engine's error as a variant's failure and carried on) |

The middle row is the one worth dwelling on, because neither SDK's behaviour survived. `header("X-Count", 3)`
means `X-Count: 3` to the author, so refusing it (the JVM) is unhelpful and building an unmatchable
shape from it (TypeScript) is worse than unhelpful — it fails silently, since the engine accepts the
document and the variant just never verifies. But the obvious fix, "call the language's `toString`",
is how two conformant SDKs come to send different bytes for the same test: `1.0` is `"1"` in
JavaScript and `"1.0"` in Java, `1e21` is `"1e+21"` and `"1.0E21"`, and a date is a locale-dependent
sentence in one and ISO-8601 in the other. So the conversion is a closed set with one spelling each,
and everything else is refused — the reversible direction, since widening it later is additive while
a spelling already written into contracts is not.

### 7.1 The third divergence, found by writing the coverage cases

§5 listed "an engine that dies mid-loop" as *uncovered*. Writing the case showed it was not a gap in
the suite but a fourth disagreement between the SDKs, and the sharpest one to read: with three
variants and an engine that stops answering after the first, TypeScript reported one engine error
while the JVM reported "2 of 3 variants failed" — the same run, described as a machinery failure by
one SDK and as two test failures by the other. `lifecycle/engine-error-aborts-the-loop` pins it, and
`lifecycle/variant-budget-exceeded` covers the other protocol failure §5 named, including that no
closure runs and the contract is withheld.

This is the pattern worth carrying into 6.5: the cases written to *close a coverage gap* were the
ones that found a behaviour nobody had decided. A gap in the suite and a gap in the specification
look identical from the outside, and the only way to tell them apart is to write the case.

### 7.2 What §5 still leaves uncovered

Unchanged from §5's first three bullets: per-language refusals (regex flags, what counts as a scalar
in a typed language), the test-framework integrations themselves, and thinness — which remains ADR
0017 commitment 3's problem, for 6.5's audit to answer by code inspection rather than by sampling.
