# Variant semantics and sampling specification (v1, final)

Plan task: **2.3**. Status: **final**.

The [shape language](../shape-language/spec.md) says where a declaration is deliberately wider than
one case; this document says what is done about it. It specifies what a variant *is*, how variants are
named, which of them an engine selects and by what algorithm, what a budget does when it bites, how a
variant is exercised on each side of the contract, and how a variant drives the provider state a
verification needs — the RFC's `whenVariant`, which the RFC marks as a sketch.

It answers the RFC's unresolved question *"variant sampling defaults: is pairwise the right default?
What are the caps and overrides?"* and designs the provider-state linkage the RFC leaves open. The
decisions behind it are [ADR 0008](../../decisions/0008-deterministic-pairwise-variant-sampling.md)
(selection) and [ADR 0009](../../decisions/0009-variant-bound-provider-state-parameters.md) (state
linkage).

The schemas under [`schemas/v1/`](schemas/v1/) are the **specified surface**, on the same terms as the
Engine Protocol's and the shape language's: this prose defines their semantics, the schemas define
their shapes, and a disagreement between them is a bug to file rather than a precedence question.
These documents cross the engine boundary inside protocol frames, so they inherit the Engine
Protocol's open-world authoring rules
([protocol spec §2.2](../engine-protocol/spec.md#22-open-world-authoring-rules)).

## Contents

1. [Scope and conformance](#1-scope-and-conformance)
2. [Variants](#2-variants)
3. [Selection](#3-selection)
4. [The consumer loop](#4-the-consumer-loop)
5. [The provider side](#5-the-provider-side)
6. [Variant-bound provider state](#6-variant-bound-provider-state)
7. [Diagnostics](#7-diagnostics)
8. [Errors](#8-errors)
9. [Evolution and compatibility](#9-evolution-and-compatibility)

Worked examples — the RFC's order payload sampled end to end, and the provider-state linkage from
declaration to state setup — live under [`examples/`](examples/). Every selection, policy and
variant-params document in them, and in this specification, is validated against the schemas by
`cargo test -p pact_janus_schema_compat`, so the examples cannot drift from the specified surface.

---

## 1. Scope and conformance

**MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT** and **MAY** are to be interpreted as described in
RFC 2119.

Conformance roles:

- An **engine** computes a variant space from a compiled interaction, selects a sample from it,
  produces and matches values under a selected variant, records what was exercised, and resolves
  variant-bound provider-state parameters.
- A **host** — an SDK, the CLI, a test harness — iterates the selection the engine hands it and
  reports back which variants it actually exercised.
- An **author** writes the shapes that create the space, and optionally a sampling policy.

In scope: variant identity, the space and its size, the selection algorithm and its defaults, budgets,
exclusions, the consumer and provider loops, and variant-bound provider state.

Out of scope, with owners:

| Question | Owner |
|---|---|
| which operators contribute dimensions, their points, defaults and gates | design [2.2](../shape-language/spec.md) |
| the operations that carry a selection across the boundary | design [2.1](../engine-protocol/spec.md) |
| how a variant's value is actually produced from a plan; generators | designs 2.4, 2.6 |
| how exercised variants and their examples are written into a pact file | design 2.5 |
| the provider-state document itself — its name, typed parameters, encoding | design 2.5 |
| the `state-setup` hook: how it is configured, invoked and reported | design 2.7 |
| what an unexercised region of a shape means for `can-i-deploy` | design 2.8 |
| the CLI surface (`--variant`, `--exhaustive`, …) | design 5.5 |

## 2. Variants

### 2.1 Assignments and activation

The input is a variant-space document
([shape spec §6.6](../shape-language/spec.md#66-the-variant-space-document)): dimensions in a
deterministic order, parents before the dimensions they gate, each with ordered points, a default
point, and zero or more gates.

An **assignment** is a partial map from dimension id to point name. A dimension `d` is **active**
under an assignment `α` when every gate `(g, p)` of `d` has `α(g) = p`. A **variant** is an assignment
that is *exactly total over its active dimensions*: every active dimension has a point, and no
inactive dimension has one. That double condition is the whole well-formedness rule, and both halves
matter — a missing point leaves a value undetermined, and a point on an inactive dimension names a
choice that no produced value can show.

Two derived facts this specification relies on:

- **Gates form a forest.** A dimension's gates are the gating operators on its path from the root of
  the part, so they are a chain ordered by nesting, and the gating relation over all dimensions is a
  forest. An engine MUST reject a variant-space document whose gates are not a forest
  (`interaction-invalid`); no shape can produce one, but a component operator declaring its own
  dimensions could.
- **Parents come first.** Because dimensions are emitted parents-before-gated, an assignment can be
  built in a single forward pass: walking the dimensions in order, each dimension's activation is
  already decided by the points chosen before it.

**`complete(α)`** is that forward pass: walk the dimensions in order; skip the inactive ones; keep the
point `α` gives, or take the dimension's default. It turns any consistent partial assignment into a
variant, deterministically. The **base variant** is `complete({})` — every dimension at the default,
which is the value the author's own examples describe.

### 2.2 Variant identity

A variant's identity **is its assignment**. The `id` is a name derived from it, and where the two ever
disagree the assignment wins: a selection carries both, and a pact file records both (§4.4).

The id is built from the dimensions whose point differs from their default, in variant-space order,
rendered `<dimension-id>=<point>` and joined with `;`. Inactive dimensions contribute nothing. When
every active dimension is at its default the id is the reserved name **`base`**.

```
base
response.body.shippedAt#presence=absent
response.body.items#cardinality=min+1;response.body.payment#alternative=invoice
```

Three properties follow, and each of them is why the id is derived this way rather than being an index
or a digest:

- **Adding a dimension renames nothing.** A new `optional` elsewhere in the tree sits at its default in
  every existing id, so recorded pacts and pinned variants keep meaning what they meant. An ordinal
  (`variant-3`) fails this immediately; so does a digest over the full assignment.
- **An id is readable and diffable.** A pact file diff shows which variants were exercised and how they
  differ, without a lookup table.
- **An id parses back to an assignment** against any variant space that has those dimensions and
  points, which is what lets a user pin one by name and a verifier find one in a pact.

Ids are long. A selection therefore also carries a **label** — the same rendering with each dimension
id shortened to the shortest trailing path segments that are unique within *this* space, plus the
facet where the path alone is ambiguous. Labels are for humans only: an engine MUST NOT accept a label
where an id is expected, and MUST NOT record one in a pact file. That split is deliberate — the thing
that is stable is unreadable, the thing that is readable is not stable, and pretending otherwise is
how identifiers rot.

One caveat, stated because it will otherwise be discovered as a bug: `base` names the *defaults*, and
a default is a property of the shape (the `example` an author wrote). Changing an example changes what
`base` denotes. The assignment recorded alongside the id is what makes that visible rather than
silent.

### 2.3 Counting the space

The size of the space is needed before any variant is built, because it decides the strategy (§3.2).
With gates it is not `∏|points|`. Define, over the forest of §2.1:

```
weight(d) = Σ over points p of d:  ∏ over dimensions d′ gated by (d, p):  weight(d′)
            (an empty product is 1)
size      = ∏ over ungated dimensions d:  weight(d)
```

So a `one-of` with two alternatives, one of which contains a single `optional`, weighs `1 + 2 = 3`,
not 2 — the sum over alternatives of the product of what each alternative contains. The order payload
extended with an `optional` inside its `invoice` alternative has a space of 36, not 48.

An engine MUST compute the size with saturating arithmetic against the largest budget in force (§3.6)
and MAY report it as "at least *n*" once saturated. A space of 10^40 is a fact about the shape, not a
number anyone needs exactly, and computing it must never be the thing that fails.

## 3. Selection

### 3.1 What a selection must contain

A **selection** is an ordered list of variants drawn from the space, together with a report of how it
was chosen (§3.9). Every selection MUST contain, in this order and deduplicated by assignment:

1. **the base variant**, always, first;
2. **the minimal and maximal variants**, unless `boundaries` is turned off (§3.8);
3. **one variant extending each pin**, in policy order (§3.8);
4. **enough further variants to cover every reachable coverage target** at the configured strength
   (§3.3), or the honest report of which targets were not covered and why.

The base variant is unconditional because it is the interaction the author actually wrote, because it
is the one variant every upgraded v1–v4 pact has (§9), and because a run whose *first* failure is an
exotic corner is a run whose failure nobody trusts.

**The boundary variants.** The **minimal** and **maximal** variants sit at the two ends of the width
the shape declares — the least and the most the contract permits a provider to send. Each dimension
takes its extreme point and `complete` (§2.1) settles the rest; what "extreme" means is fixed per
facet:

| Facet | Minimal … maximal | What the minimal point collapses |
|---|---|---|
| `presence` | `absent` … `present` | the member, and everything under it |
| `nullability` | `null` … `non-null` | the subtree under the null |
| `cardinality` | smallest … largest size | the elements |
| `alternative` | fewest … most dimensions in the alternative | every alternative not taken |
| `value` | — takes its default | nothing; no option is smaller than another |
| a component facet | as the component declares | — |

The unifying idea is **collapse**: at its minimal point a dimension produces the least structure it
can, at its maximal point the most. For `presence` and `nullability` that is literally the gate —
`absent` and `null` are the points that deactivate a subtree (§2.1) — so those two need no order
declared beyond the gate that already exists. The other two do not gate, and their extremes are stated
because a rule that only counted collapsed *dimensions* would get them wrong:

- **`cardinality` does not gate at all** (shape spec §6.5): a collection's interior dimensions are
  shared across elements and stay active whatever the count, so no point of it collapses a dimension.
  The same blindness would hit `optional` over a scalar — nothing lives under
  `optional(datetime(…))`, so neither of its points collapses anything, and yet `absent` is the
  boundary the RFC's entire example turns on. Collapse is a property of the value produced, not a
  count of dimensions.
- **`alternative` collapses every alternative not taken**, so the minimal variant takes the
  alternative contributing the fewest dimensions and the maximal the one contributing the most, ties
  broken by point order. Taking the *default* alternative instead would be arbitrary — a default is
  the author's chosen example, not an end of anything — and it would leave the maximal variant unable
  to open the alternative that holds the extra structure, which is usually the whole reason that
  alternative is interesting.

They are in the selection for the same kind of reason the base variant is, and it is **not** coverage.
Pairwise already covers every point and every pair of points, so `absent` and `max` each appear
somewhere regardless; the extremes add only conjunctions of three or more, and an engine that wants
those has `strength: 3`. What the extremes are for is that they are the two ends of the declared
width. A pact that records six samples from the middle of its space and neither of its boundaries has
recorded the wrong six.

Three limits, stated rather than papered over:

- **No variant opens every alternative at once.** A `one-of` is exclusive by construction, so when
  dimensions live in two of its alternatives the maximal variant activates as many as any variant can
  and no more. Maximal is a maximum, not a totality, and the report does not claim otherwise.
- **The maximal variant depends on subtree shape.** Adding a dimension deep inside one alternative can
  change which alternative contributes the most, and so change the maximal variant and its id. Ids
  remain derived from assignments and recorded beside them (§2.2), so nothing becomes uninterpretable
  — but this is the one place where an edit in one part of a tree moves a seed somewhere else.
- **A boundary variant is often already there**, and then it costs nothing: the maximal variant *is*
  the base whenever every `optional` defaults to `present`, every collection to its largest point and
  every `one-of` to its widest alternative.

The cost is bounded and small, because they are seeds — the covering step works around them exactly as
it does a pin:

| Shape | Covering array alone | With the base and boundary seeds |
|---|---|---|
| the RFC's order payload (2×2×2×3) | 6 | 8 |
| the same with a gated `optional` inside `invoice` | 9 | 10 |
| eight `optional` members | 9 | 9 |
| five optionals, three enumerations, one list | 12 | 13 |

Between zero and two, and smallest where the space is largest.

### 3.2 Strategies and the default ladder

`strategy` is an open vocabulary. v1 defines:

| Strategy | Selection |
|---|---|
| `auto` | **the default**: `exhaustive` when the space is at most the exhaustive threshold, otherwise `t-wise` |
| `exhaustive` | every variant in the space |
| `t-wise` | a covering array of strength `strength` (default 2 — pairwise) |
| `base-only` | the base variant and the pins, nothing else |

The defaults are **`auto`**, **strength 2**, and an **exhaustive threshold of 8**.

Pairwise is the right default for the reason the pairwise literature gives and the RFC assumes: the
overwhelming majority of real defects are triggered by one parameter or by an interaction between two,
so strength 2 buys most of the available confidence for a sample that grows logarithmically rather
than exponentially. What that means at this specification's scale, measured with the algorithm of §3.4
rather than asserted:

| Dimensions | Space | Pairwise sample |
|---|---|---|
| 2×2 | 4 | 4 |
| 2×2×2 | 8 | 4 |
| 2×2×3 | 12 | 6 |
| 2×2×2×3 (the RFC's order payload) | 24 | 6 |
| 2^10 | 1 024 | 11 |
| 2^20 | 1 048 576 | 14 |

Those are covering-array sizes. A selection also carries the seeds of §3.1 — the base variant and,
unless turned off, the two boundaries — which add between zero and two on top.

The threshold of 8 comes out of the same table. Below it the sample and the space are close enough
that exhaustive costs at most four extra runs and removes the only awkward question a sampled matrix
raises — *which combination did we not try?*. At 16 the gap is ten runs and at 24 it is eighteen, which
is where "we tested every pair" stops being a compromise and starts being the point. An author who
wants the whole space anyway says `exhaustive` and gets it, subject to the budget.

`base-only` exists for two real cases: an upgraded v1–v4 pact, which demonstrates exactly one example
whatever size its converted shapes give the space (§9) — where the space has no dimensions at all,
`base-only` and `exhaustive` coincide; and a suite being migrated, where a team wants shapes recorded
before it is ready to pay for variant runs. It is a deliberate, visible reduction in what the
contract demonstrates, and §7 requires it to be reported as such.

### 3.3 Coverage targets, reachability and gating

At strength `t`, a **coverage target** is a set of `t` `(dimension, point)` pairs drawn from `t`
distinct dimensions. A target is **reachable** when some variant contains all of it; it is a target
only if it is reachable.

Reachability is decided structurally, not by search. The target's own pairs, together with the gates of
each dimension it names (transitively), form a set of requirements; the target is reachable iff that
set is **consistent** — no dimension required at two different points. Two dimensions in different
alternatives of the same `one-of` are therefore never paired, and a dimension inside an `optional` is
paired only with that `optional`'s `present` point. Exclusions (§3.5) remove targets in addition.

An engine MUST report the count of reachable targets and the count actually covered (§3.9). It MUST NOT
report a coverage figure it did not compute; "pairwise" is a claim about a specific set of pairs, and a
sampler that cannot say which pairs it covered has not made the claim.

**Vacuous coverage.** A collection's cardinality dimension does not gate the dimensions inside it
(shape spec §6.5) — the interior dimensions are shared across elements and take a point whatever the
cardinality is. When the selected cardinality point is size 0 there are no elements, so the interior
points are chosen but never shown. A variant whose cardinality point for a collection is 0 therefore
MUST NOT be counted as covering any target that names a dimension inside that collection. This is the
one seam where shape spec §6.5's no-gating rule shows, and closing it here is cheaper than gating a
dimension on a numeric point.

### 3.4 The selection algorithm

The algorithm is **IPOG** (in-parameter-order, general) with the seeds, gates and exclusions this
specification adds. It is specified here, normatively and in full, because a sampler that is only
described produces a different sample in every implementation, and the sample is recorded in a
contract.

Let `D` be the dimensions and `t` the strength.

**Step 0 — seeds.** `S := [complete({})]`; then the minimal and maximal variants (§3.1) unless
`boundaries` is off; then `complete(pin)` for each pin in policy order. Duplicates are skipped, so a
boundary that coincides with the base or a pin costs nothing. Seeds are fixed: later steps read them
for coverage and never modify them.

**Step 1 — targets.** `T :=` every reachable target (§3.3) at strength `t`, minus those removed by
exclusions, minus those already covered by `S`.

**Step 2 — dimension order.** Order `D` by descending point count, ties broken by variant-space order,
subject to a dimension never preceding one that gates it. Ties are broken by declaration order and
nothing else: no randomness, no hashing, no map iteration order.

Taking the widest dimensions first is what makes the sample small. The first `t` dimensions are
enumerated exhaustively, so putting the widest there forces the largest unavoidable block of the
covering array to the front, and everything after it grows horizontally into that block.

**Step 3 — initial block.** `TS :=` every consistent, unexcluded combination of points over the first
`t` dimensions in that order, as partial assignments, dropping any already covered by a seed.

**Step 4 — growth.** For each remaining dimension `d`, in order:

- *Horizontal.* For each partial assignment `τ` in `TS` under which `d` is active: choose the point of
  `d` that covers the most targets in `T` involving `d` and dimensions already assigned in `τ`,
  skipping points that would make `τ` match an exclusion. Ties go to the earlier point in the
  dimension's point order. Extend `τ`, and remove the targets it now covers.
- *Vertical.* For each still-uncovered target involving `d`: extend an existing `τ` that is compatible
  with it (the first such, in order) if one exists; otherwise append a new partial assignment holding
  the target's pairs and their gates. Remove what is now covered.

**Step 5 — completion.** Complete each partial assignment with `complete` (§2.1) and append it to `S`,
skipping duplicates. Where a default would make the completed variant match an exclusion, replace it
with the first point of the first free dimension, in order, that does not; if no such replacement
exists the assignment is dropped and reported (§3.5).

**Step 6 — budget.** Apply §3.6.

The selection is `S`, in that order.

Two properties worth stating because they are the reason for the shape of the algorithm. It
**terminates**: every iteration of step 4 either covers a target or removes it as unreachable, and `T`
is finite. It **makes progress on every variant it adds**: each added assignment exists to cover a
specific target. A simpler greedy — build a variant by walking the dimensions and taking the locally
best point each time — has neither property: it can produce a variant covering nothing new and loop
forever, and on the RFC's order payload it needs eight variants where this one needs six.

An engine MAY substitute a different algorithm only under a different name (§3.7).

### 3.5 Exclusions

Some combinations of points cannot occur together in the system being described. The shape language
has no way to say so — its dimensions are independent by construction, and the RFC's own example shows
the gap: nothing in the order shape stops the sampler pairing `status = SHIPPED` with `shippedAt =
absent`, though the provider will never produce it.

An **exclusion** removes a region of the space: a conjunction of `(dimension, point)` pairs, plus a
**required `reason`**. A variant matching every pair of an exclusion is not in the space, is not a
coverage target, and is never selected.

Three rules keep exclusions from becoming a way to make a contract quietly weaker:

- **The reason is required**, and it is recorded — in the selection report (§3.9) and in the pact file
  (§4.4). An excluded region is width the consumer declared and deliberately never demonstrated, and
  the contract has to say so out loud.
- **An exclusion does not narrow `admits`.** The shape still admits the excluded values, so a provider
  producing one still matches. Exclusions govern *what is exercised*, never *what is accepted*. Design
  2.8 decides what an excluded region means for subsumption; this specification's contribution is to
  make it visible and attributable rather than inferable from a gap in a list.
- **No expression language.** An exclusion is a list of pairs. Predicates over values, comparisons
  between fields and arithmetic are all out — the same reasoning as ADR 0007's refusal of literal
  shorthand: a document that has to be *evaluated* to be understood cannot be reviewed in a pact file.

When an exclusion makes a target unreachable, the target is removed and counted as `removed` in the
report. When the sampler cannot complete a variant for a target without matching an exclusion, it does
**not** backtrack: it drops the target and reports it as `target-dropped` with the exclusion cited.
Reporting an uncovered pair is honest; searching harder for it is a cost with no bound, and the author
who sees the report can pin the variant they wanted.

### 3.6 Budgets, and what happens when one bites

Two numbers bound the work, both per interaction:

| Budget | Default | Meaning |
|---|---|---|
| `exhaustive-threshold` | 8 | above this space size, `auto` selects `t-wise` rather than `exhaustive` |
| `max-variants` | 50 | the largest selection the engine will hand back |

Exceeding `max-variants` is an **error**, not a truncation: the engine fails the operation with
`variant-budget-exceeded` (§8), naming the space size, the selection size, the budget, and the
dimensions contributing the most points. It does not return the first 50.

This is the contested choice, and it is deliberate. A truncated selection is a contract that silently
demonstrates less than it claims — precisely the failure mode variant testing exists to remove — and
it does it at the moment the shape is most complicated, which is the moment nobody re-reads the
report. Failing puts a decision in front of the author, and every way out is a real one: narrow the
shape, exclude a region with a reason, pin the variants that matter and drop to `base-only`, or raise
the budget explicitly for this interaction. All four are visible in review. Truncation is not.

The budget bites almost only on wide `any-of` sets: pairwise over two 30-option enumerations needs 900
variants, because every one of the 900 pairs must appear somewhere. That is the signal that an
enumeration is being tested as data rather than as a contract, and an author who means it can say so.

`max-variants` bounds one interaction. A session with two hundred interactions can still be slow;
that is a suite-design question, and the report (§3.9) gives a host what it needs to sum the cost.

### 3.7 Determinism, and why the algorithm is named

**Selection is a pure function of the variant space, the policy, and nothing else.** No random number
generator, no clock, no hash seed, no iteration order of an unordered map. The same inputs MUST produce
the same selection, in the same order, on every engine that implements this specification.

Determinism is not a nicety here. The selection is recorded in a pact file; a verifier replays what was
recorded; a failing run has to be reproducible from the pact and nothing else. Randomised covering-array
generators (AETG and its descendants) produce smaller arrays on average by restarting from random
seeds, and that is exactly the trade this design refuses: a sample that changes between runs turns
every pact file into a diff and every flaky verification into an archaeology exercise.

The consequence is that improving the algorithm changes samples. So the algorithm is **named**, the
name is part of the policy, and the name is recorded in the selection report and the pact file. v1
defines `janus-ipog-v1` (§3.4), and a better sampler ships as `janus-ipog-v2` — an addition to an open
vocabulary, never a redefinition of an existing name. This is the same commitment ADR 0007 makes for
operators, for the same reason: recorded artifacts outlive the code that wrote them.

### 3.8 The policy document

Schema: [`schemas/v1/sampling-policy.schema.json`](schemas/v1/sampling-policy.schema.json).

```json policy
{ "strategy": "auto",
  "strength": 2,
  "exhaustive-threshold": 8,
  "max-variants": 50,
  "boundaries": true,
  "algorithm": "janus-ipog-v1",
  "pin": [
    { "assignment": [ { "dimension": "status", "point": "SHIPPED" },
                      { "dimension": "shippedAt", "point": "present" } ],
      "reason": "the shipped-order case the UI reads" } ],
  "exclude": [
    { "when": [ { "dimension": "status", "point": "SHIPPED" },
                { "dimension": "shippedAt", "point": "absent" } ],
      "reason": "the order service sets shippedAt when it sets SHIPPED" } ] }
```

A policy resolves in layers, each overriding the one before it member by member:

1. the defaults of this specification;
2. the session configuration (`consumer-session/create`'s open `config`);
3. the interaction specification's own policy (designs 2.5, 3.2), where an author says "this
   interaction is exhaustive";
4. the `policy` member of the `consumer-session/variants` request, for a host that is offering a
   `--exhaustive` switch.

`boundaries` turns the minimal and maximal seeds off (§3.1). It is the one reduction in what a
selection demonstrates that costs nothing to reverse, which is why it is a boolean rather than a
strategy: a suite that cannot afford two more runs per interaction says so once, in the session
configuration, and the report says so on every interaction.

`pin` and `exclude` are the exception: they **accumulate** across layers rather than overriding, so a
session-wide exclusion is not silently lost when an interaction sets its strength. A pin is a *partial*
assignment — pin `status = SHIPPED` and the sampler completes the rest, which is almost always what an
author means by "make sure we test the shipped case".

### 3.9 The selection document

Schema: [`schemas/v1/variant-selection.schema.json`](schemas/v1/variant-selection.schema.json). This is
the document `consumer-session/variants` returns, and the input to a host's test loop:

```json selection
{ "variants": [
    { "id": "base", "label": "base", "origin": "base",
      "assignment": [ { "dimension": "response.body.items#cardinality", "point": "min" },
                      { "dimension": "response.body.payment#alternative", "point": "card" },
                      { "dimension": "response.body.shippedAt#presence", "point": "present" },
                      { "dimension": "response.body.status#value", "point": "PENDING" } ] },
    { "id": "response.body.shippedAt#presence=absent",
      "label": "shippedAt=absent", "origin": "boundary",
      "assignment": [ { "dimension": "response.body.items#cardinality", "point": "min" },
                      { "dimension": "response.body.payment#alternative", "point": "card" },
                      { "dimension": "response.body.shippedAt#presence", "point": "absent" },
                      { "dimension": "response.body.status#value", "point": "PENDING" } ] } ],
  "report": {
    "space": { "size": 24, "exact": true, "dimensions": 4 },
    "strategy": "t-wise", "strength": 2, "algorithm": "janus-ipog-v1",
    "selected": 8,
    "coverage": { "targets": 30, "covered": 30, "removed": 0, "dropped": 0 },
    "budgets": { "exhaustive-threshold": 8, "max-variants": 50 },
    "boundaries": true } }
```

`origin` says why each variant is in the list — `base`, `boundary`, `pinned` or `covering` — which is
what lets a host explain a run, and what lets an author see that their pin survived. The report is not optional
decoration: §3.3 requires the coverage figures to be computed, and this is where they are stated.

## 4. The consumer loop

### 4.1 One variant armed at a time

A host calls `consumer-session/variants` for a handle, then loops: `serve-variant` with a variant id,
drive the application under test, repeat. Per protocol spec §8.2, re-arming a handle replaces the
previous arming, and this specification adds the reason: **at most one variant of an interaction may
be armed at a time.** Two variants of the same interaction can differ only in the response, leaving two
armings that the same inbound request would match, and an engine choosing between them would be
guessing which variant the test meant. Different interactions may of course be armed concurrently —
that is ordinary parallel-interaction mocking, and their request shapes are what distinguish them.

Dimensions may live in any part. A dimension in the *request* changes what the mock will accept and
what the verifier will replay; a dimension in the *response* changes what is produced and what is
matched. The slot prefix in the dimension id is what tells them apart, and nothing else in this
specification treats them differently.

### 4.2 Exercised

A variant is **exercised** when the interaction actually ran under it: for a passive interaction, the
armed variant matched inbound traffic and the exchange completed; for an emissive one, the message was
produced and delivered to the consumer's handler. Arming a variant and never sending the request is
not exercising it.

Every variant in the selection is **required**. Protocol spec §8.2 makes the consequence structural: an
unexercised required variant has status `not-exercised`, and `finalise` withholds the pact exactly as
it does for a failure. That is the mechanism that turns a declared width into a promise — a host that
takes the first variant and stops gets no pact file, and gets told which variants it skipped.

Statuses: `verified`, `failed`, `not-exercised`. Nothing distinguishes "the host chose to skip it" from
"the host crashed before reaching it", and nothing should: both mean the same thing about the contract.

### 4.3 Run order

The selection is ordered and hosts SHOULD run it in order: base first, then the boundary variants,
then pins, then covering variants in the order the algorithm produced them. The first variant to run is therefore the one the
author wrote by hand, so the first failure a user sees is the simplest one available. A host MAY run
variants in parallel where its transports allow it (each variant is an independent exchange), but MUST
report every variant's outcome.

### 4.4 What is recorded

For each exercised variant a pact file records (design 2.5 owns the encoding):

- the variant `id` **and** its full `assignment` — the id is the name, the assignment is the truth
  (§2.2);
- the concrete values produced for that variant, per part;
- the resolved provider-state parameters for that variant (§6).

For the interaction as a whole it records the shape once, the selection report (§3.9), and any
exclusions with their reasons. Recording the report is what makes a pact file self-describing about its
own coverage: a reader can see that a 24-variant space was covered pairwise under `janus-ipog-v1` by
six variants, with two more at its boundaries, rather than inferring it from the length of a list.

Variants that were not exercised are not recorded, in any form other than the report's counts. This is
the RFC's honesty rule, and it is why a pact file cannot claim a variant that never ran.

## 5. The provider side

### 5.1 The pact is the sample

**Sampling happens once, on the consumer side.** A verifier MUST NOT re-sample: it replays exactly the
variants the pact records, all of them, in recorded order.

The alternative — sampling again at verification time from the recorded shape — is tempting because the
verifier has everything it needs, and it is wrong on both counts that matter. It would test
combinations the consumer never demonstrated, which is the same false confidence subsumption exists to
remove; and it would make a verification run's content depend on the verifier's policy, so a green run
would not be reproducible from the pact. The consumer chooses the sample; the provider answers for it.

A verifier MAY filter the recorded variants down (a `--variant` switch for debugging, design 5.5), and
MUST report a filtered run as filtered — a partial run is not a pass.

### 5.2 Replaying a variant

For each recorded variant, in order:

1. resolve the interaction's provider states for that variant (§6) and run `state-setup`;
2. send the **recorded request example** for that variant — replay is by example, so the provider sees
   the bytes the consumer actually sent, not a re-derivation;
3. match the response against the interaction's shape **pinned to that variant** (shape spec §7.1): an
   `optional` pinned to `absent` admits only `⊥`, an `any-of` pinned to `SHIPPED` admits only that.

Pinning at step 3 is what makes a variant mean something on this side. Matching the response against
the whole shape would accept `PENDING` where the consumer demonstrated `SHIPPED`, and the variant would
have proved nothing about the provider.

### 5.3 Outcomes

Per interaction and variant, the verification stream reports `verified`, `failed`, or
`state-unavailable` (§6.7). `state-unavailable` is a failure of the run; it is a distinct status
because its remedy is a contract change, not a code change, and a summary that cannot distinguish "the
provider returned the wrong thing" from "the provider cannot be put into this state" sends the reader
to the wrong team.

## 6. Variant-bound provider state

### 6.1 What the RFC sketches, and what breaks

The RFC writes:

```typescript
given('an order exists', { shipped: whenVariant('shippedAt', 'present') })
```

The intent is exact and worth keeping: a provider state parameter whose value depends on which variant
is running, so the verifier can put the provider into the state each variant needs. Three things have
to be decided before it is implementable.

**Where the binding lives.** A state parameter's value is user data, so a wrapper object — a value that
happens to have a `when-variant` member meaning "this is a binding" — is ambiguous against a parameter
whose value legitimately has that member. This is the same trap ADR 0007 refused for shapes, and the
answer is the same: bindings live in **their own member**, never in-band inside a value.

**What it evaluates to.** `whenVariant(dimension, point)` yields a boolean, but the useful cases are
not all boolean — a state parameter often wants a *different value* per point ("the order's status is
`SHIPPED`"). So the canonical form maps points to values, and the RFC's boolean predicate is the DSL
sugar for the two-case form. One canonical document, sugar in the SDKs.

**How the dimension is named.** The RFC's `'shippedAt'` is not a dimension id — ids are
`response.body.shippedAt#presence` (shape spec §6.2). The SDK cannot expand it, because a thin SDK does
not compute variant spaces. So the engine resolves the reference (§6.3), which is also where a typo
becomes a good error message instead of a silent mismatch.

### 6.2 The binding document

Schema: [`schemas/v1/variant-params.schema.json`](schemas/v1/variant-params.schema.json). A provider
state carries its literal parameters in the member it already has, and its variant-bound parameters in
a sibling `variant-params` member (design 2.5 ratifies the encoding in the pact file and interaction
spec; the semantics are this specification's):

```json params
{ "variant-params": [
    { "name": "shipped",
      "dimension": "shippedAt",
      "cases": [ { "point": "present", "value": true },
                 { "point": "absent", "value": false } ] },
    { "name": "status",
      "dimension": "response.body.status#value",
      "cases": [ { "point": "SHIPPED", "value": "SHIPPED" },
                 { "point": "DELIVERED", "value": "DELIVERED" } ],
      "default": "PENDING" } ] }
```

A parameter MUST NOT appear in both the literal parameters and `variant-params` — a value that is
sometimes a literal and sometimes computed is a debugging problem nobody needs (`interaction-invalid`).

### 6.3 Dimension references

A `dimension` member is a **reference**, resolved against the interaction's variant space. It matches a
dimension when it is the dimension's full id, or its full path, or a trailing run of whole path
segments of it. `shippedAt`, `body.shippedAt`, `response.body.shippedAt` and
`response.body.shippedAt#presence` all resolve to the same dimension.

Resolution MUST be unique. A reference matching no dimension, or more than one, is `interaction-invalid`
at `add-interaction` time, with a `problems[]` entry naming the reference and listing the candidate
dimension ids. Failing at submission — while the author is looking at the DSL that produced it — is the
whole reason resolution belongs in the engine, and a reference that resolves to two dimensions because
a shape grew a second `shippedAt` elsewhere is a genuine ambiguity that the author should settle.

**What is recorded is the resolved id.** The pact file carries `response.body.shippedAt#presence`, never
`shippedAt`, so a verifier reading the pact resolves nothing and cannot resolve it differently.

### 6.4 Resolution

For a variant `v` and a binding `b`:

- if `b`'s dimension is **active** in `v` and some case names `v`'s point for it, the parameter takes
  that case's value;
- otherwise the parameter takes `b`'s `default`;
- if there is no `default`, the parameter is **absent** for this variant — not null, not empty. A state
  handler that treats a missing parameter as a default gets to keep doing so.

Resolution happens on both sides and MUST agree: the consumer resolves when recording an exercised
variant, the verifier when replaying it. Since resolution is a total function of the assignment and the
binding, agreement is structural rather than a matter of trust — and the recorded resolved values make
a disagreement visible immediately.

### 6.5 Validation

At `add-interaction`, an engine MUST reject (`interaction-invalid`, with positions):

| Problem | Why it is caught here |
|---|---|
| a reference that resolves to no dimension | the author's typo, at the moment they can fix it |
| a reference that resolves to several | a real ambiguity; the message lists the candidates |
| a `point` that the referenced dimension does not have | `whenVariant('shippedAt', 'set')` silently never fires otherwise |
| no `default` on a binding whose dimension is **gated** | a gated dimension is inactive in some variants by construction, so the fallback is not hypothetical |
| the same `name` in both literal params and `variant-params` | §6.2 |

The fourth row is the one that only a design that took gating seriously would catch: a binding on
`payment@invoice.dueDate#presence` has no case to apply in any `card` variant, and without a `default`
the state parameter would vanish for half the run.

### 6.6 State setup runs per variant

Distinct variants often resolve to identical state parameters — in the order payload, every variant
with `shippedAt = present` resolves `shipped` to `true` regardless of the other three dimensions. It
is tempting to run `state-setup` once for a run of consecutive variants whose resolved states are
equal.

**A verifier MUST NOT.** `state-setup` runs for every variant, including consecutive variants whose
resolved parameters are identical, and a verifier MUST NOT reorder variants to create such runs:
recorded order is the order (§5.1).

The precondition such an optimisation needs is one the verifier cannot check. "Skip the setup, the
state is already established" assumes nothing has disturbed it since — and the thing most likely to
have disturbed it is the variant that just ran. An interaction is a real request against a real
provider; it may create, mutate or consume the very data the state describes, and whether it does is
invisible from this side of the boundary. The other half of the assumption is no better: the verifier
cannot know whether a state handler is idempotent, only the author of the handler can.

The trade is bad in both directions. What skipping buys is small — measured on the worked example,
grouping consecutive equal states saves one setup call in eight when one parameter is bound and none
at all when two are, because pairwise selection exists to vary dimensions *together*, so consecutive
variants rarely share a state. What it costs is the worst failure mode a contract test has: a variant
that fails only because an earlier variant moved the data underneath it fails *depending on what ran
before it* — intermittent, order-dependent, and attributed to the wrong interaction.

Re-running is also the semantics the ecosystem already has. Today's verifier invokes the state change
for every interaction, so state handlers are written to be re-runnable; a handler that breaks when run
twice with the same parameters is already broken, and nothing here should reward it.

If setup cost ever does dominate a verification run, the thing to design is a **declaration** — a
state its own author marks variant-independent, in hook configuration (design 2.7) — not an inference
the verifier makes about code it cannot see. ADR 0009 carries that as a tripwire rather than a
feature.

### 6.7 When the provider cannot produce the state

A variant can need a state the provider cannot produce: the consumer declared `status = SHIPPED` with
`shippedAt = absent`, and no amount of setup will make the order service produce it.

The `state-setup` hook (design 2.7) may therefore answer three ways, and this specification fixes what
each means:

| Hook outcome | Variant status | Reading |
|---|---|---|
| success | run continues | the provider is in the state |
| **`unsupported`**, with a reason | `state-unavailable` | the provider cannot reach this state |
| error / failure | `failed` | the state setup itself is broken |

`state-unavailable` **fails the verification run by default.** The reasoning is the one the RFC builds
this whole mechanism for: the consumer demonstrated it can handle that variant, and the provider has
not demonstrated it can produce it, so the variant is *not verified*, and reporting it as a pass would
be exactly the false confidence variant testing exists to remove. A provider that cannot produce a
combination is evidence that the consumer's declaration is wider than reality — and the fix is a
contract change: exclude the region with a reason (§3.5), or narrow the shape.

That answer is right and it is also, on its own, a way to block a provider team on a consumer team's
release. So the waiver is specified rather than left to be invented, and it is scoped: an
`allow-state-unavailable` entry names the interaction and the point (or the exact variant), carries a
required reason, and is reported in the summary as a waived variant rather than a passing one. Global
"ignore state failures" is deliberately not offered — it is the switch that would be turned on once and
never turned off.

## 7. Diagnostics

`verification/explain` and the CLI (design 5.5) MUST be able to show, for an interaction:

- the dimensions, their points, their defaults and their gates;
- the space size, the strategy chosen and why (which threshold or budget decided it);
- the selection with each variant's id, label and origin;
- coverage: targets reachable, covered, removed by exclusions, dropped;
- every exclusion with its reason, and every waiver with its reason.

```text
interaction 'get an order' — 4 dimensions, space 24, pairwise (space > threshold 8)
  selected 8 variants, covering 30 of 30 reachable pairs   [janus-ipog-v1]
    1. base                                        base
    2. shippedAt=absent                            boundary (minimal)
    3. items=min+1                                 boundary (maximal)
    4. payment=invoice;status=SHIPPED              covering
    …
  excluded 1 region: status=SHIPPED & shippedAt=absent
    "the order service sets shippedAt when it sets SHIPPED"
```

The point of the RFC's `explain` is that matching is never a black box; a sampled matrix is the other
place a user is asked to trust a decision they did not make, and it gets the same treatment. Plan task
4.6 is where this stops being a claim: it deliberately mishandles a variant and reports whether the
failure was identifiable.

## 8. Errors

Codes are the Engine Protocol's (protocol spec §10.2); this specification adds one and constrains the
details of two:

| Code | Category | When | `details` |
|---|---|---|---|
| `variant-budget-exceeded` | `document` | the selection would exceed `max-variants` (§3.6) | `space`, `selected`, `budget`, `dimensions` (the largest contributors, by id) |
| `interaction-invalid` | `document` | a malformed policy, an unresolvable or ambiguous dimension reference, an unknown point, a missing `default` on a gated binding, a non-forest gate structure | `problems: [{ path, message }]`, with candidate ids where a reference was ambiguous |
| `variant-not-found` | `session` | `serve-variant` with an id that is not in this interaction's selection | the id, and the selection's ids |

`variant-not-found` covers the case worth calling out: an id that names a *valid* variant of the space
which was not selected. It is not found because the selection is what the contract will record, and
serving an unselected variant would exercise something the report does not account for. A host that
wants it pins it.

## 9. Evolution and compatibility

**Upgraded pacts are the degenerate case, with no special rule.** A v1–v4 pact converted to a Janus
contract (design [2.5](../contract-file/spec.md#8-converting-v1v4-pacts); it is not a "pact v5" — [ADR
0011](../../decisions/0011-contracts-as-self-identifying-json-documents.md)) has matching rules turned
into shapes and its single example as the sole variant. Where none of the converted rules contributes a
dimension, the space is fully degenerate: `size = 1`, the base variant is the only variant, every
strategy agrees, and the selection is `[base]`.

**The invariant is `selected = 1`, not `size = 1`.** Some rules do contribute dimensions — a `min` on
an array becomes an `each-like` with a minimum, and shape spec §6.4 gives that operator a `cardinality`
dimension with a `min+1` point whenever `max > min`, which an absent `max` satisfies — so a converted
contract can have a space larger than its evidence. The pact demonstrated one
example and §4.4's honesty rule permits recording exactly that, so the selection is `base` alone under
strategy `base-only`, and the report says `selected: 1` against a `space.size` above 1. The contract is
then honestly under-covered and its own report says by how much; running the consumer's suite under
Janus is what closes the gap, which is the incentive the migration path wants.

Neither case is a branch in this specification. `base-only` is a strategy the vocabulary already has
(§3.2), the report's arithmetic is unchanged, and the degenerate space falls out of §2.3's product
rather than being special-cased around it — which is the test of whether the arithmetic was right.
Design 2.5 §8.3 states the same invariant from the conversion side.

**The algorithm name is frozen; the vocabulary grows.** `janus-ipog-v1` denotes the algorithm of §3.4
forever. Strategies, origins, algorithms and statuses are open string vocabularies that grow without a
version bump (protocol spec §11.2), and the schemas here follow the same additive-evolution rules,
enforced by the same CI checker ([`tools/schema-compat`](../../../tools/schema-compat/README.md)).

**Ids and labels have different guarantees.** Dimension ids and point names are stable by shape spec
§6.2, so variant ids built from them are stable too, and both are recorded. Labels are derived from the
space they appear in and may change when the shape changes; they are never recorded and never accepted
as input (§2.2).

**A recorded variant outlives its selection.** A pact records the variants that were exercised; a later
run of the same shape under a different policy — a raised budget, an added pin — selects a different
set. That is intended: the pact is evidence of what was demonstrated, not a description of what the
sampler would do today. It is also the reason the assignment is recorded next to the id, so that a
variant from an older pact can still be interpreted against a shape that has since grown a dimension.
