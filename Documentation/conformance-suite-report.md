# Plan task 6.4: the conformance suite seed, and what it found

**Date:** 2026-09-20. **Subject:** [`conformance/`](../conformance), the two drivers, and the
checker. **Claim under test:** ADR 0017's — that "conformant" can be a claim a build makes. Task
6.3's report closed on "only half testable": the bar is "pass every conformance-suite case its
`conformance` list names", and there were 53 ids and no cases, so every judgement call the JVM
implementer made passed the scenario they invented for it, because they wrote both.

## 1. What was built

| | |
|---|---|
| Corpus | 27 cases under `conformance/cases`, covering all **53** conformance ids in `behavioural-spec.json`: 15 translation, 6 lifecycle, 3 variants, 3 live. Each is one JSON document naming the ids it covers and quoting the sentence it pins. |
| Case format | `conformance/schemas/v1/conformance-case.schema.json`. A case is data — a DSL chain written as JSON, a scripted engine, and what must come of it — not a Gherkin sentence each language re-reads. §3 says why. |
| Drivers | `sdks/typescript/test/conformance/` (Vitest) and `sdks/jvm/sdk/src/test/java/io/pact/janus/sdk/conformance/` (JUnit `@TestFactory`). Each knows only how its language *spells* a primitive; about 400 lines each, test-scope, outside the published surface. |
| Checker | `tools/conformance` (`pact_janus_conformance`). `lint` holds the corpus to its schema and to the behavioural specification's ids — an id with no case fails the build. `check` reads each language's report: every case accounted for, and passed. |
| CI | The corpus lints in the `rust` job; both SDKs run it in their own test runs and write reports; `check` gates the `sdks` job on both. |

**Both SDKs pass all 27 cases.** `cargo run -p pact_janus_conformance -- check` reports
`pact-janus-typescript: 27 of 27` and `pact-janus-jvm: 27 of 27`.

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
They are still unstated, so there is no case for them: a case that decided them would make the suite
normative beyond the document every implementer reads (`conformance/README.md` §7).

| | TypeScript | JVM |
|---|---|---|
| `headers: { Accept: "first", accept: "second" }` | the last wins, silently: one `accept` slot with `["second"]` | refused at the call: "header 'accept' is already declared" |
| a non-string, non-list, non-helper value (`header("X-Count", 3)`) | builds `each-like` over `equality 3` — a shape the transport's string values can never match | refused at the call |

Both SDKs pass every conformance id they name. The first silently drops a declaration the author
wrote; the second builds a document that cannot match and hands it to the engine (defensible under
SDK spec §2.1 — rejecting is the engine's call — except that the engine accepts it, so nobody
reports anything and the variant simply never verifies).

**Recommendation** (behavioural spec `request`, restating 6.3's recommendation 2, still open): say
what happens when two header names collide after lower-casing, and what happens to a value that is
none of the three forms. Then add the two cases.

### 4.3 Confirmed and bounded: contract bytes are not a conformance bar

`session.finalise.contract-bytes-unaltered` is the id whose name most invites a byte-for-byte test.
Its case checks what the specification actually requires — compact JSON, one trailing LF, members in
the engine's order, nothing re-indented or re-escaped, with non-ASCII and quote characters in the
payload — and deliberately does not test number spellings or integer-like member names, where an SDK
that parses the frame can only reconstruct what the engine wrote (Phase 9 finding 6). ADR 0017 puts
bytes outside conformance, so the case says so in its `note` rather than leaving the next reader to
wonder whether the gap was an oversight.

## 5. What 27 cases do not cover

Stated plainly, because ADR 0017's own "Harder" says an under-specified category here is a silent
hole in what "conformant" guarantees:

- **Per-language refusals**: `regex` flags that a pattern text cannot carry (behavioural spec
  `regex`), what counts as a scalar or a map in a typed language, ordered-map literals. Each SDK
  tests its own; the corpus cannot.
- **Protocol failures during a run**: `variant-budget-exceeded`, `session-not-found`, and an engine
  that dies mid-loop. The scripted engine can express the first two (`engine.errors`) and no case
  uses them yet.
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
agree, and the two that do not (§4.2) are edges the specification never described.

**Recommended next**:

1. Settle §4.2 in `behavioural-spec.json`, then add the two cases.
2. Add cases for the protocol failures in §5 as the scripted engine already supports them.
3. Task 6.5 measures against this corpus: a DSL change lands in the behavioural specification, an
   agent regenerates both idiomatic layers, and *this* is what says whether the result is correct.
