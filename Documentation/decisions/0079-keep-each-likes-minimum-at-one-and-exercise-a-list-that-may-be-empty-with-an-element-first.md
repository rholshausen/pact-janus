# 0079 — Keep `each-like`'s minimum at one, and exercise a list that may be empty with an element first

- **Status**: accepted
- **Date**: 2026-11-11
- **Plan tasks**: S3.1 (gate D), phase-9 finding 2
- **Evidence**: [phase-9 finding 2](../prototype/phase-9-findings.md#2-does-an-array-matcher-admit-the-empty-array-by-default);
  pact-js's `eachLike` (`src/v3/matchers.ts`, which writes `min: 1`, and `src/dsl/matchers.ts`, which
  refuses a `min` below 1) and pact-jvm's (`PactDslJsonBody.eachLike`, which writes a bare
  `TypeMatcher`, beside `minArrayLike` for a bound); the shape spec's
  [§6.1](../specs/shape-language/spec.md#61-dimensions-and-points),
  [§6.3](../specs/shape-language/spec.md#63-gating) and
  [§6.4](../specs/shape-language/spec.md#64-per-operator-dimensions); ADR 0024 decision 4, which left
  the default open; the variant selections measured below, from `consumer-session/variants`

## Context

Finding 2 asks whether an array matcher that states no length admits `[]`. The two largest Pact DSLs
answer it oppositely under one name: pact-js's `eachLike` writes `min: 1`, and pact-jvm's writes a bare
`type`, which v1–v4 read as any length. The shape language chose `min: 1` (§4.3, §5.5), and `janus
upgrade` maps a bare `type` to that default, reporting the narrowing as `rule-narrowed` since task 9.2.
ADR 0024 kept the default and left the question to the DSL, which stage 3 now freezes.

Finding 2 argued the cost in variants: a default of 0 would put an empty case into every list's
dimension. Measured, the count does not change. A cardinality dimension has the points `min` and
`min+1` either way, and the RFC's order payload selects 8 variants with `min: 0` as with `min: 1`. What
changes is which lists are served. The base variant is every dimension at its default point (variant
semantics §2.1), and `each-like`'s default is `min`. With `min: 0`, the base variant serves `[]`, though
the author wrote one element and §2.1 says the base variant is "the value the author's own examples
describe".

The same measurement found a defect that exists today for any author who writes `min: 0`. A list
whose items carry an `optional` and a three-option `any-of` selects 7 variants, and 4 of them serve
`[]`. Cardinality does not gate its items (§6.4), so those 4 still assign points to the item
dimensions, and the sampler counts the pairs they make as covered. `gift = present` with `kind = B`, and
with `kind = C`, appear only in empty variants. Nothing was produced for them, but the selection report
says every pair was. Variant semantics §3.3 already forbade this: a variant whose cardinality point is 0
"MUST NOT be counted as covering" a target inside the collection, a rule it chose as "cheaper than
gating a dimension on a numeric point". No engine implemented it.

## Decision

1. **`each-like`'s `min` stays 1 when it is not given**, and so does `each-entry`'s. A list the author
   wrote with an element is one they have shown has an element. `min: 0` says it may be empty, which
   adds the empty list to what is exercised as well as to what is admitted. That is the shape
   language's rule for every width.
2. **The DSL has one array operator.** `eachLike(items)` is at least one element, and
   `eachLike(items, { min: 0 })` may be empty. A second operator that differs only in its default
   would make an author learn two names for one node. pact-jvm's users already have two, and one of
   them means the opposite of pact-js's.
3. **`janus upgrade` maps a bare `type` on an array to `each-like` with `min: 0`**, with no finding. A
   pact that names no bound admitted any length, so the upgraded contract does too. pact-jvm's
   `eachLike` keeps its meaning through the facade, and pact-js's, which writes `min: 1`, keeps its
   own. The `rule-narrowed` finding task 9.2 added for this case goes, and plan-grammar §4.4's two paths
   agree on an empty array again. A stated `min` or `max` is converted as before.
4. **A list that may be empty is exercised with an element first.** When `min` is 0, the cardinality
   dimension's default point is `min+1`, the first point that produces an element. The base variant
   then serves the author's example, and the empty list is the minimal boundary variant (ADR 0008
   decision 2). When `min` is 0 and `max` is 0, there is no dimension, as now.
5. **An empty list has no interior.** The dimensions inside an `each-like`'s `items`, or an
   `each-entry`'s `keys` and `values`, are gated by every cardinality point whose size is above 0. When
   `min` is above 0, every point is, and nothing changes. When `min` is 0, the interior is inactive
   under the `min` point, as an `optional`'s is under `absent`. The empty variant assigns its interior
   nothing, and no pair is counted for it. This replaces §3.3's vacuous-coverage rule.
6. **Gates that name one dimension are alternatives.** A dimension is active when, for each dimension
   its gates name, the assignment gives that dimension one of the points they name. Gates on different
   dimensions must all hold, as now. Until now no engine emitted two gates on one dimension, because
   each gating operator gates its subtree once. So every variant-space document already written
   means what it meant, and the schema does not change. §2.3's count is unchanged too: a dimension
   gated by several points of its parent is counted under each of them.

## Alternatives considered

- **Default `min` to 0** (the "explicit bounds only" school). The base variant of every list becomes
  the empty list, so the first failure a consumer sees is a case they never wrote. Decision 4 could be
  applied to fix that, but then no list would show the default it claims: the base variant serves one
  element and the contract admits none.
- **Two operators, as pact-jvm has** (`eachLike` and `minArrayLike`, or a new `listOf`). This replaces
  reliance on a default with a choice between two names, and the facade shows the names already mean
  different things in different SDKs. `{ min: 0 }` is visible in review, where a choice between names
  is not.
- **Keep the upgrade's `min: 1` and its finding.** This is the narrowing finding 2 recorded. It fails
  pact-jvm users' providers that answer `[]` to pacts they pass today, and facade users' mocks that
  serve it. The upgrade's job is to keep what the pact admitted.
- **Implement §3.3's vacuous-coverage rule instead.** It leaves the empty variant assigning points that
  no value shows, which §2.1's well-formedness rule calls "a choice that no produced value can show",
  and it needs its own check wherever coverage is counted. Gating uses the machinery every other
  empty interior already uses.
- **Gate the interior with a new facet** (an `emptiness` dimension over a `min: 1` list). This states
  the same thing, but it renames every `min: 0` list's dimension, and dimension ids are recorded
  (shape spec §6.2).

## Consequences

- A native list still means at least one element, which is what pact-js's users write today and what
  the RFC's example writes explicitly. An author who handles an empty list says so, and their test
  then runs with one.
- An upgraded pact-jvm `eachLike` admits `[]` in the contract, as it does in the pact. Facade contracts
  that `compareContracts` pins change from `min: 1` to `min: 0`.
- A `min: 0` list's selection changes: its base variant has an element, and the empty variant
  assigns nothing inside it. The verifier replays what a contract recorded (ADR 0008), so a contract
  already written verifies as before. Re-recording one changes its selection.
- The sampler's activation, reachability and counting read a gate list as one group of alternative
  points per dimension.
- The subsumption check is unaffected. A provider shape with `min: 0` against a consumer's default
  `min: 1` is still the `wider-cardinality` finding the demo shows ("the provider may produce an empty
  list"), and that finding is the point of the default.
- **Tripwire**: pilot consumers routinely writing `{ min: 0 }`, or providers' `wider-cardinality`
  findings on lists being exempted as noise more often than they are fixed. Either is evidence for a
  superseding ADR on the default.
