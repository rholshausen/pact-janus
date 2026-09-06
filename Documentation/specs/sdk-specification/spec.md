# SDK specification template (v1, draft)

Plan task: **2.9**. Status: **draft — under review**.

Every other Phase 2 design fixes what crosses the engine boundary. This one fixes what crosses a
different boundary: the one between "an SDK maintainer knows what a Janus SDK must do" and "a Janus SDK
does it, in TypeScript, in Java, and in whatever the ecosystem adds after this prototype." Plan task 6.3
is this design's own conformance test before task 6.4's suite is even built: the JVM SDK is written
*from this specification*, deliberately not by porting the TypeScript SDK, specifically to find out
whether a behavioural spec plus a style guide actually transmits behaviour, or whether — as with today's
Pact — two implementations quietly diverge the moment nobody is looking at both at once.

This document is therefore not prose about good SDK design. It is a **document format**: the canonical
behavioural specification (§3) that names every DSL primitive precisely enough to implement without
reading another language's source, the per-language style-guide skeleton (§5) that carries what the
format deliberately leaves out, and the compatibility-facade guidance (§6) that decides how much of
today's DSL survives. [ADR 0017](../../decisions/0017-sdk-conformance-is-suite-passing-not-prose-matching.md)
settles what "conformant" means — the RFC's own claim ("An SDK is conformant when it passes the suite
against a pinned engine version") made precise enough for task 6.4 to build against and task 6.5 to
measure against.

## Contents

1. [Scope and conformance](#1-scope-and-conformance)
2. [What a Janus SDK is](#2-what-a-janus-sdk-is)
3. [The behavioural specification](#3-the-behavioural-specification)
4. [Worked reference: the RFC's consumer example](#4-worked-reference-the-rfcs-consumer-example)
5. [The per-language style-guide skeleton](#5-the-per-language-style-guide-skeleton)
6. [The compatibility facade](#6-the-compatibility-facade)
7. [Conformance](#7-conformance)
8. [The AI-assisted regeneration loop](#8-the-ai-assisted-regeneration-loop)
9. [Errors](#9-errors)
10. [Evolution and compatibility](#10-evolution-and-compatibility)

Worked examples — the RFC's consumer example mapped primitive by primitive, a style-guide skeleton, and a
compatibility-facade mapping for a representative slice of today's classic DSL — live under
[`examples/`](examples/), validated against [`schemas/v1/behavioural-spec.schema.json`](schemas/v1/behavioural-spec.schema.json)
by `cargo test -p pact_janus_schema_compat`.

---

## 1. Scope and conformance

**MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT** and **MAY** are to be interpreted as described in RFC
2119.

Conformance roles:

- A **spec author** — this project's core team — maintains the canonical behavioural specification, one
  document shared by every language.
- An **SDK maintainer** implements the idiomatic layer (§2) for one language from the behavioural
  specification plus that language's style guide (§5), and keeps the conformance suite (task 6.4) green
  against a pinned engine version.
- A **regenerating agent** (§8, task 6.5) proposes idiomatic-layer changes when the behavioural
  specification changes; it never decides correctness.

In scope: the behavioural-specification document format, the discipline that keeps a primitive's entry
precise enough to implement blind (§3–§4), the style-guide skeleton (§5), compatibility-facade principles
(§6), and what "conformant" means (§7, ADR 0017).

Out of scope, with owners:

| Question | Owner |
|---|---|
| the protocol operations and documents a primitive's entry names | designs [2.1](../engine-protocol/spec.md), [2.2](../shape-language/spec.md), [2.3](../variant-semantics/spec.md) |
| the binding-generation pipeline that produces layer (a) | task 6.1 |
| the actual TypeScript and JVM idiomatic layers | tasks 6.2, 6.3 |
| the conformance suite's actual scenarios, growing from `pact-compatibility-suite` | task 6.4 |
| the thinness audit and AI-regeneration trial's measurement methodology | task 6.5 |
| positioning AI-assisted diagnosis/verification beyond SDK maintenance | design 2.10 |

## 2. What a Janus SDK is

Three layers, per the RFC, and each has exactly one obligation:

1. **Generated bindings** — typed views of the protocol's own JSON Schemas (engine-protocol spec §2.3):
   request/result/event documents, shape and variant documents, error documents. Produced by the binding
   pipeline (task 6.1) from `Documentation/specs/*/schemas/`, the same schemas every other design ships,
   and **never hand-edited** (CLAUDE.md's rule, restated because it is this layer's entire reason to
   exist: a hand-edited generated file is a second source of truth the pipeline cannot know about).
2. **The idiomatic layer** — DSL, test-framework integration, docs. This is the only hand-written
   production code an SDK contains, and its role is exactly this and nothing more: **translate DSL calls
   into protocol operations and shape/plan documents, and translate protocol results and errors into
   language-native constructs.** No matching logic, no orchestration beyond the protocol's own operations
   (CLAUDE.md architecture rule; RFC: "New engine features surface in an SDK by regenerating bindings and
   adding DSL sugar").
3. **The conformance run** — executing task 6.4's suite, in this language, against a pinned engine
   version, in CI. This is what makes "conformant" (§7) a claim a build can fail, not a claim a maintainer
   asserts.

### 2.1 The thinness test

For any function under consideration for the idiomatic layer, ask: **does this decide anything the engine
does not already decide, or does it only carry a decision across the boundary?** If the former, it does
not belong in an SDK. Three concrete failures this test catches, because each has happened in today's
per-language Pact implementations:

- Computing a variant space, or a covering sample of it, client-side instead of calling
  `consumer-session/variants` (variant semantics spec owns the algorithm; an SDK that reimplements even a
  simplified version of it is exactly the divergence B1 exists to remove).
- Deciding a match's pass/fail from raw request/response values instead of from the result
  `consumer-session/serve-variant` and `finalise` already return.
- Retrying, coercing, or "helpfully" repairing a value before handing it to the engine — the engine's
  `interaction-invalid`/`contract-invalid` errors exist precisely so a malformed document is reported, not
  silently patched (engine-protocol spec §10).

An idiomatic layer that passes this test end to end has no code path that could disagree with the engine
about whether a test passed, which is the property B1 is actually about.

## 3. The behavioural specification

### 3.1 What it is

The canonical behavioural specification is **one document, one per DSL surface (not per language)**,
naming every user-facing primitive — a shape helper (`optional`, `anyOf`), an interaction builder
(`.given`, `.request`), a session/lifecycle call (`janus.execute`) — as a **primitive entry**:

```json primitive
{ "id": "optional",
  "category": "shape",
  "summary": "Marks a value as may-be-absent.",
  "produces": [ { "kind": "shape-operator", "operator": "optional" } ],
  "signature": [ { "name": "of", "role": "nested-shape",
                   "description": "The shape admitted when the value is present." } ],
  "semantics": "Compiles to a shape-language 'optional' node (shape spec §4.4) wrapping the compiled form of 'of', with no further transformation. The DSL contributes no matching logic here: 'of' is compiled by the same rules that would apply if it appeared unwrapped, and 'optional' only adds the wrapper (shape-language example order-payload.md §2's own description of a DSL's job as 'sugar'). Contributes one variant dimension (presence) at this node — the SDK does not compute variant dimensions itself; it emits the node and the engine derives dimensions from it (shape spec §6.4).",
  "errors": [ { "code": "interaction-invalid",
                "surfaced-as": "thrown/raised exception carrying the engine's 'problems' positions verbatim; never a boolean or a silently-skipped assertion" } ],
  "conformance": [ "shape.optional.presence-dimension", "shape.optional.wraps-any-operator" ] }
```

Schema: [`schemas/v1/behavioural-spec.schema.json`](schemas/v1/behavioural-spec.schema.json).

### 3.2 Fields, and why each exists

- **`produces`** names what protocol/document construct the primitive compiles to — a shape operator
  (design 2.2), a protocol operation (design 2.1), or a member path within the interaction-spec document
  (plan task 3.2). It is an array because a primitive like `execute` produces a *sequence* of protocol
  calls, not one document. Naming it structurally, rather than only in prose, is what lets a regenerating
  agent (§8) find every primitive touching a given protocol member when that member's own design changes.
- **`signature`** names parameters by **role**, not by per-language type: `example-value`,
  `nested-shape`, `options-bag`, `label`. A role is deliberately coarser than a type, because the concrete
  type (a `string` in TypeScript, a `String` in Java, a builder object in either) is the idiomatic layer's
  and the style guide's business (§5), not this document's — fixing types here would make the format
  prescribe an API shape rather than a behaviour.
- **`semantics`** is the load-bearing field, and §3.3 states the bar it must clear.
- **`errors`** names which of engine-protocol spec §10's structured error codes this primitive's
  translation must surface, and how it becomes a language-native failure. This exists because "no
  matching logic" (§2.1) has a mirror-image failure mode: an idiomatic layer that swallows a structured
  error into a generic exception, or a `false` return, has thrown away exactly the information
  engine-protocol spec §10 designed `details`/`problems` to carry, and a user debugging a failing test
  loses it a second time for no reason.
- **`conformance`** is a list of scenario ids — strings, open, owned by task 6.4 — naming which suite
  cases exercise this primitive. It is traceability, not a scenario definition: this design does not
  specify test scenarios, only that every primitive names at least one, so "does the suite actually cover
  this?" is answerable by reading the spec rather than by reading the suite's source.
- **`facade`** is optional and covered in §6.

### 3.3 The bar `semantics` must clear

Task 6.3 exists to test exactly one falsifiable claim about this document, and this design states the
claim plainly so 6.3 can hold it to account: **a `semantics` paragraph passes if a competent engineer,
fluent in the target language but who has never read another language's idiomatic-layer source, can
implement the primitive from it alone and pass every conformance-suite case §3.2's `conformance` list
names for it.** A paragraph that requires "look at how the TypeScript SDK does it" to resolve an ambiguity
has failed this bar, whatever else it gets right — and 6.3's whole purpose is to surface exactly that
failure mode while it is still cheap to fix, before a third and fourth SDK are relying on the same
document.

This is also why `semantics` cites other designs by section rather than restating them: a primitive's
behaviour is precisely the composition of what the shape language, the protocol, and variant semantics
already define, and restating any of it here would create a second place it could quietly disagree with
the source of truth.

## 4. Worked reference: the RFC's consumer example

[`examples/order-example-mapping.md`](examples/order-example-mapping.md) walks the RFC's own consumer
test — the one CLAUDE.md's TypeScript conventions already name as the DSL's reference surface
(`optional`, `anyOf`, `oneOf`, `eachLike`, `janus.execute(interaction, async (mock, variant) => …)`) — call
by call, from DSL text to the primitive entries it exercises to the protocol operations those entries
resolve to at `execute` time:

```
janus.interaction('get an order')    -- begins an interaction-spec document; no protocol call yet
  .given('an order exists', {...})   -- appends a `states` entry
  .request({...})                    -- populates parts.request, mapping shape helpers 1:1
  .response({...})                   -- populates parts.response, mapping shape helpers 1:1

janus.execute(interaction, closure)
  -> consumer-session/create          (once per session, not per interaction)
  -> consumer-session/add-interaction
  -> consumer-session/start-transport
  -> consumer-session/variants
  -> for each selected variant:
       consumer-session/serve-variant
       run closure(mock, variant)
  -> consumer-session/finalise
```

Every arrow is engine-protocol spec §8.2's own operation table; this design adds nothing to it, only
attaches DSL call sites to it so an implementer has one place that shows the whole path from keystroke to
wire call.

## 5. The per-language style-guide skeleton

The behavioural specification deliberately does not fix: naming case, module layout, the async
model (`Promise`/`async`-`await`, `CompletableFuture`, a coroutine), builder ergonomics (fluent chain vs.
data class plus function), how a structured protocol error becomes a language-native failure, or
test-framework integration mechanics (`test.each`, a JUnit 5 `@TestFactory`). Those are exactly the
decisions that make an SDK feel native to its language, and prescribing them centrally would produce the
"wrapper-of-wrappers" awkwardness the RFC explicitly throws away.

[`examples/style-guide-skeleton.md`](examples/style-guide-skeleton.md) is the skeleton every language
copies and fills in once, kept in that SDK's own tree (`sdks/typescript/STYLE.md`,
`sdks/jvm/STYLE.md`) — the sections it contains: naming and module layout, the async model, error surface,
builder ergonomics, test-framework integration points, packaging/distribution, and a **deviations**
section: any place a primitive's `signature` role does not map cleanly to an idiomatic parameter in this
language, named with a one-line reason. The deviations section is not optional decoration — a silent
deviation is precisely the kind of divergence B1 exists to remove, and naming it here makes it a reviewed
decision instead of an implementation detail nobody wrote down.

## 6. The compatibility facade

### 6.1 Principle

The RFC's own words bound this: "the old DSL surface is kept where it maps cleanly... so most consumer
tests need mechanical changes only." "Maps cleanly" is not a feeling; it is a claim about information
loss, and this design fixes what it means:

- **`kept`** — the old primitive's behaviour is a strict, lossless projection of a new primitive's
  behaviour. Calling the old name today and calling it under the facade produce the same observable
  outcome for every input that was legal before. `like(v)`'s classic type-matching behaviour, for
  instance, is a strict subset of what a `type`-shape does (shape spec §4.2) — nothing about `like` relied
  on anything a `type` shape doesn't also provide.
- **`adapted`** — kept, but the facade must supply a default the old call site never had to state, because
  the old primitive's semantics under-specified something the new model requires an answer to. Every
  `adapted` entry names the default and the one-line reason it is safe (§6.2). An `adapted` mapping that
  cannot state its default in one line is not actually a clean mapping and should be `dropped` instead.
- **`dropped`** — no honest translation exists. The commonest reason is that the old primitive assumed
  cascading or precedence semantics the shape language deliberately does not have ("no cascading, no
  precedence... a path has exactly one shape", shape spec §7.1), or reached into raw matching-rule paths
  as an escape hatch shapes intentionally do not expose. A `dropped` entry names what a user migrates to
  and why the old behaviour cannot be preserved, because "dropped" without a migration path is exactly
  the mechanical-changes-only promise broken.

**A `kept` name MUST NOT be given new behaviour.** If satisfying the new model under an old name would
change what that name does for any input that used to be legal, the honest classification is `adapted`
(with the default stated) or `dropped` (with the migration path stated) — never `kept` with an asterisk.
This is the same discipline ADR 0007 applies to shape operators ("never redefined") and ADR 0008 applies
to sampling algorithms, extended to a third kind of recorded meaning: what a consumer's *existing test
suite* already depends on.

### 6.2 Where the mapping lives

A primitive's optional `facade` array names the old-DSL names it replaces:

```json primitive
{ "id": "any-of",
  "category": "shape",
  "summary": "Enumerates the literal values a field may take.",
  "produces": [ { "kind": "shape-operator", "operator": "any-of" } ],
  "signature": [ { "name": "options", "role": "example-value-list" } ],
  "semantics": "Compiles to a shape-language 'any-of' node (shape spec §4.1, §5.3) with 'options' as its literal set and the first option as its example. Contributes one 'value' variant dimension, one point per option, in declaration order (shape spec §6.4).",
  "errors": [],
  "conformance": [ "shape.any-of.literal-containment", "shape.any-of.value-dimension" ],
  "facade": [
    { "from": "like(v) with a custom generator function returning one of several fixed literals at random",
      "status": "dropped",
      "reason": "a random per-run generator defeats deterministic recording (ADR 0008's 'no randomness, anywhere' applies just as much to a consumer's own generators as to the sampler) and never declared its alternative set anywhere the engine could see. Migrates to anyOf(...) naming the same literals, now sampled deterministically per variant instead of chosen at random once." } ] }
```

[`examples/compatibility-facade-mapping.md`](examples/compatibility-facade-mapping.md) works through a
representative slice of today's classic consumer DSL (`like`, `eachLike`, `term`, `regex`, `uuid`,
`iso8601DateTime`, boolean/number/decimal type matchers, and the JVM builder's equivalents) against this
vocabulary, so 6.2/6.3 have a concrete table to build the actual facade module from rather than starting
the classification from nothing.

## 7. Conformance

[ADR 0017](../../decisions/0017-sdk-conformance-is-suite-passing-not-prose-matching.md) is the decision
record; this section states the resulting definition plainly. **An SDK is conformant, for a pinned engine
version, when every behavioural-specification primitive it implements passes every conformance-suite
(task 6.4) case its `conformance` list names, run in that language, against that engine.** The suite, not
this document's prose, is the arbiter — a `semantics` paragraph is necessary context for an implementer
and irrelevant to whether a build is green.

Task 6.4's suite MUST cover, at minimum, the four categories the plan already commits to:

1. **DSL → interaction-spec translation** — a primitive's `produces` and `signature` are honoured for
   representative inputs, including the shape-operator mappings §4's worked reference fixes.
2. **Session lifecycle** — the protocol call sequence §4 traces (`create` → `add-interaction` →
   `start-transport` → `variants` → `serve-variant`* → `finalise`) happens in that order, once per
   session, with `finalise` always reached (including on a failing test — sessions are the only resource,
   engine-protocol spec §7.1, and an SDK that leaks a session on failure has already failed conformance
   regardless of what the test asserted).
3. **Variant iteration** — the closure runs once per variant the engine selected (not once per
   *declared* variant, and never zero times because the SDK "optimised" a single-variant space), and a
   variant the closure cannot handle fails the build rather than being silently skipped.
4. **Contract-output equivalence** — two SDKs given equivalent DSL usage produce contracts the engine
   considers the **same interaction content**: identical `parts`, `states` and `selection` per contract
   spec §5–§6, under its canonical-writing rules (contract spec §2.4). This is deliberately *not*
   byte-identical files — `metadata.writer` legitimately differs by SDK name and version (contract spec
   §3.2 puts exactly this kind of fact in `metadata`, outside content identity) — and a suite that
   demanded byte-identical output would be grading a fact this design never asked any SDK to agree on.

Conformance says nothing about idiomatic quality — a conformant SDK can still have an awkward, unpleasant
API — and nothing about thinness by itself: a suite that only samples scenarios cannot, by construction,
rule out a hand-rolled reimplementation that happens to agree with the engine on every sampled case. That
gap is exactly what task 6.5's thinness audit exists to close by a different method (code inspection, LOC
by layer), and this design does not pretend the suite alone makes the audit unnecessary.

## 8. The AI-assisted regeneration loop

The RFC's "Generated, AI-assisted SDKs" idea needs one thing from this design to be more than an
aspiration: a diffable, machine-readable source of truth for what changed. The behavioural specification
is that source. A regeneration task, concretely, is:

```
{ changed: [ primitive ids added/removed/whose semantics changed ],
  language: "jvm",
  style-guide: sdks/jvm/STYLE.md,
  existing-source: sdks/jvm/src/... }
  -> proposed idiomatic-layer patch
  -> task 6.4's conformance suite, run against the pinned engine
  -> human review of the diff
```

Two things this design fixes about that loop, both restating the RFC's own boundary one layer up: **the
suite is the sole judge of correctness**, never the agent's own claim, and never a diff that merely
"looks like" the TypeScript equivalent; and **a passing suite run does not authorise merging** — human
review stays in the loop for exactly the reason plan task 6.5 measures review burden rather than assuming
it away. This is the same judgment/orchestration split the RFC draws for agentic verification (§2.10):
the deterministic artifact (here, the conformance suite; there, the engine's pass/fail) keeps judgment,
and the agent performs mechanical work under it.

## 9. Errors

The behavioural-specification document is authored and reviewed content, not a document that crosses the
engine boundary at runtime, so it carries no operational error taxonomy of its own. Two checks apply:

| Condition | Check |
|---|---|
| structurally invalid document (missing required member, malformed `produces`/`facade`) | `tools/schema-compat lint`, the same open-world rules every other design's schemas follow |
| a primitive with an empty `conformance` list | not a schema violation — task 6.4's own CI SHOULD flag it as a coverage gap, the same way an exclusion with no reason would be flagged elsewhere, but this design does not require a non-empty list structurally, because a freshly authored primitive legitimately has no scenario yet on the day it is added |

A structured protocol error a primitive's translation must surface (its `errors` field) is
engine-protocol spec §10's own taxonomy; this design adds no codes.

## 10. Evolution and compatibility

**Primitives are additive; semantics is not silently redefined.** A new DSL primitive is a new entry; an
existing primitive's `semantics` changing incompatibly is exactly the event that must re-trigger both
SDKs' conformance runs and is exactly what the regeneration loop's diff (§8) is defined against — the
discipline is the same one ADR 0007 fixes for shape operators and ADR 0008 fixes for the sampling
algorithm, applied to a document two SDKs, not one engine and one reader, depend on staying meant the same
way.

`$format` is `janus-sdk-behavioural-spec/1`, one version line, governed by the same additive-evolution
rules — enforced by [`tools/schema-compat`](../../../tools/schema-compat/README.md) — as every other
design's schemas: `category`, `produces.kind`, and `facade.status` are open vocabularies that only grow.

The style-guide skeleton and the compatibility-facade table are not schema-governed documents — they are
templates and worked guidance, filled in once per language and revised by ordinary review — and this
design does not version them, for the same reason engine-protocol spec §2.2 does not version prose:
nothing reads them except the people who write them.
