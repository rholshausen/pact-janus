# 0008 — Select variants by a named, deterministic pairwise algorithm, and fail rather than truncate

- **Status**: proposed
- **Date**: 2026-08-27
- **Plan tasks**: 2.3 (feeds 4.3, 4.6, 5.2, 2.5, 2.8)
- **Evidence**: [Pact MkII RFC](https://github.com/pact-foundation/roadmap/pull/146) ("A consumer test
  in Pact MkII", unresolved question "variant sampling defaults"), [ADR
  0007](0007-shapes-denote-value-sets.md) (dimensions, gating, id stability),
  [shape language spec §6](../specs/shape-language/spec.md#6-variant-dimensions), measured sample sizes
  in [the worked example](../specs/variant-semantics/examples/order-payload-sampling.md)

## Context

The shape language makes a declared width a promise to exercise it, which is only affordable if
something chooses a small covering sample. The RFC says "pairwise by default, exhaustive below a
threshold, and always including any variant the user pins explicitly", and then lists the numbers as an
unresolved question: is pairwise right, what are the caps, what are the overrides.

Three constraints bound the answer, and they are not the ones a covering-array library is designed for.
The selection is **recorded in a pact file** and replayed by a verifier months later, so it has to be
reproducible from the artifact. Gating (shape spec §6.3) makes this a *constrained* covering array —
dimensions inside a `one-of` alternative or an `optional` do not exist in every variant. And the sample
is read by humans in a failure report, so "why is this variant here" needs an answer.

## Decision

**Selection is a pure function of the variant space and the policy, computed by a named algorithm, and
a budget that would be exceeded is an error rather than a smaller sample.**

Six commitments, and they are the contested ones:

1. **Pairwise by default, exhaustive at or below a space of 8.** Strength 2 is the default because most
   defects are single-parameter or two-parameter; the threshold is 8 because below it exhaustive costs
   at most four extra runs and removes the only awkward question a sampled matrix raises — which
   combination did we not try. At 16 the gap is ten runs and at 24 it is eighteen, which is where
   sampling starts earning its keep. Both numbers come from running the algorithm, not from taste.
2. **The base variant and the two boundaries are seeded, not sampled.** The minimal and maximal
   variants put every *ordered*-facet dimension (`presence`, `nullability`, `cardinality`) at its
   lowest and highest point and everything else at its default. The justification is deliberately not
   coverage — pairwise already covers every point and every pair, so the extremes add only three-way
   conjunctions, and an engine that wants those has `strength: 3`. It is that these two variants are
   the least and the most a provider is permitted to send, and a contract that records six samples
   from the middle of its space and neither of its boundaries has recorded the wrong six. Measured
   cost: zero to two variants, smallest where the space is largest, because a seed is something the
   covering step works around. Two limits are accepted rather than hidden: with a `one-of` in the tree
   there is no maximal variant at all (alternatives are mutually exclusive), and `value` and
   `alternative` facets have no order, so the extremes are silent about them.
3. **No randomness, anywhere.** Every tie is broken by declaration order. Randomised generators (AETG
   and descendants) produce smaller arrays on average, and that is precisely the trade being refused: a
   sample that changes between runs turns every pact file into a diff and every flaky verification into
   archaeology.
4. **The algorithm is named and frozen, not described.** `janus-ipog-v1` denotes the specified IPOG
   variant forever, and appears in the policy, the selection report and the pact file. A better sampler
   ships as `janus-ipog-v2` — an addition to an open vocabulary, never a redefinition. Same commitment
   ADR 0007 makes for operators, for the same reason: recorded artifacts outlive the code that wrote
   them.
5. **Exceeding `max-variants` (default 50) fails the operation.** A truncated selection is a contract
   that silently demonstrates less than it claims, at the moment the shape is most complicated and the
   report least likely to be re-read. Failing puts the decision in front of the author, and all four
   ways out — narrow the shape, exclude a region, pin and drop to `base-only`, raise the budget — are
   visible in review.
6. **Cross-dimension impossibility is an exclusion with a required reason, not a shape construct.** The
   shape language's dimensions are independent by construction, and the RFC's own example needs
   `status = SHIPPED` tied to `shippedAt = present`. An exclusion removes a region from the *space*
   without narrowing `admits`, carries a mandatory reason, and is recorded in the pact — so an
   undemonstrated region is attributable rather than inferable from a gap in a list. It is a list of
   pairs, never an expression: a document that must be evaluated to be understood cannot be reviewed in
   a pact file.

Two consequences of the same reasoning, recorded because they will be re-argued: the **base variant is
always selected and always first**, so the first failure a user sees is the ordinary case the author
wrote, with the boundaries immediately behind it; and **the verifier never re-samples** — it replays exactly what the pact records, because
sampling again would test combinations the consumer never demonstrated and would make a green run
depend on the verifier's policy rather than on the contract.

Spec text: [Variant semantics and sampling specification](../specs/variant-semantics/spec.md) §2–§5;
schemas `variant-selection.schema.json` and `sampling-policy.schema.json`.

## Alternatives considered

- **Adopt an existing covering-array tool** (PICT, ACTS/jenny, a Rust crate). None of them is a
  library in the language and target set this engine needs (WASM included), the ones with real
  constraint support are a JVM or a Windows binary, and every one of them optimises for array size —
  the axis this design cares least about — while none guarantees the reproducibility it cares most
  about. Specifying ~40 lines of algorithm costs less than shipping a subprocess.
- **A simple greedy** (build each variant by walking the dimensions and taking the locally best point).
  It was the starting point and it fails twice: it can emit a variant that covers nothing new and loop
  forever, and on the RFC's order payload it needs eight variants where seeded IPOG needs six.
- **Truncate at the cap and warn.** Keeps every run green, which is the problem: it converts "this
  contract is too wide to test" into a line of log output nobody reads.
- **Random sampling with a fixed seed.** Reproducible, and it gives up the coverage *guarantee* — the
  claim "every pair was exercised" is the thing that makes a sampled matrix defensible at all.
- **Re-sample at verification time.** Tempting because the verifier has the shape; wrong because it
  would verify combinations the consumer never demonstrated.
- **Per-element dimensions and per-variant exclusion expressions.** Both rejected upstream in ADR 0007
  and here for the same reason: they make a recorded artifact something you have to execute to read.

## Consequences

Easier: eight variants cover the RFC's 24-variant order payload — six for every pair, two for its
boundaries — and 14 cover a space of a million, so declaring the real response space stays affordable; a pact file is self-describing about its own
coverage (space, strategy, algorithm, pairs covered), so a reader can see what was demonstrated without
re-running anything; failures are reproducible from the artifact alone.

Harder: improving the sampler means shipping a new algorithm name and accepting that old pacts keep the
old sample; editing a shape re-samples, so a pact diff after adding an `anyOf` option is larger than the
edit suggests; and authors of very wide enumerations will meet `variant-budget-exceeded` and have to
make a decision the old model let them avoid.

Committed to: `janus-ipog-v1` means what §3.4 says forever; the defaults 2, 8 and 50 are policy members
and may be tuned, but the *shape* of the ladder (auto → exhaustive or t-wise, budget as an error) is the
decision; selection stays a pure function with no I/O, no clock and no RNG, which is also what keeps it
inside the WASM kernel.

**Tripwire** — revisit if any of these show up in Phase 4/5: `boundaries` gets turned off routinely
(two variants per interaction is too expensive, or the maximal variant is too obviously not maximal to
be worth running); teams routinely raise `max-variants`
instead of narrowing shapes (the budget is teaching the wrong lesson); exclusions accumulate faster than
dimensions (the shape language, not the sampler, is missing the ability to express dependence); pairwise
misses defects that a strength-3 default would have caught (4.6 is where that would surface); or the
re-sample-on-edit churn makes pact diffs unreviewable, which would argue for a stability-preserving
sampler rather than a size-minimising one.
