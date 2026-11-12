# 0087 — Compare the built-in components' operators by their own matchers, and a `compare` each declares

- **Status**: accepted
- **Date**: 2026-11-12
- **Plan tasks**: S4.7 (builds component-interfaces spec §7.5's native binding; amends shape spec §8's
  enumerable-provider row)
- **Evidence**: [ADR 0086](0086-record-a-provider-shape-from-the-providers-own-tests-by-the-operation-each-test-names.md),
  `engine/kernel/tests/provider_shape_by_operation.rs` (a recorded `Content-Type` was a review),
  [phase 9 finding 9](../prototype/phase-9-findings.md#9-a-converted-pacts-frozen-examples-are-the-consumer-side-of-73s-noise-question)
  (task 9.2's option A)

## Context

Stages 1–3 gave the built-in HTTP component four operators: `http:status-class` (ADR 0057),
`http:media-type` (ADR 0071), `http:list` (ADR 0082) and `http:path` (ADR 0086). An upgraded pact writes
the first three wherever a v1–v4 pact compares a status class, a `Content-Type` or a list header. The
subsumption walk treats every component operator as opaque (shape spec §8): `yes` when the two nodes are
identical, `unknown` otherwise. So the `Content-Type` a provider's tests recorded, `application/json`,
against a pact's `http:media-type` of `application/json`, was a review on every interaction. A review
nobody can act on is the noise ADR 0016 defaults to `warn` to avoid.

Component-interfaces spec §7.5 already says how this is meant to work: an operator declares a
comparability class, and for an `exact` one the walk asks the component, `matcher/compare`. None of it
was built. Task 9.2 also decided a finite provider against a conservative core operator by running the
consumer's own matcher on each value, which is membership, not containment, and needs nothing from the
operator but its matcher.

## Decision

1. **The native matcher binding answers `comparability` and `compare`.** `MatcherComponent` gains both,
   defaulting to `opaque` and `operation-unsupported`, so a component that says nothing stays opaque.
   The walk reaches them through the engine's contributions, the same ones a selector's actions run
   through.
2. **A finite provider is decided by the consumer's operator's own matcher**, when its component
   declared it `exact` or `conservative`: an `equality`, an `any-of`, `null` or `boolean` against it is
   `yes` when the matcher admits every value, and `no`, naming the values, when it does not. This is
   shape spec §8's enumerable-provider row, extended from the conservative core operators to declared
   component operators. An operator declared `opaque`, or one no component runs, is not asked.
3. **Anything wider is the component's `compare`**, asked for the consumer's operator first, then the
   provider's. Its `no` is a `wider-values` finding carrying the component's reason; its `unknown` is a
   review carrying it.
4. **The HTTP component declares** `http:status-class`, `http:media-type` and `http:list` `exact`, and
   `http:path` `conservative`. Its `compare` decides:
   - two status classes, or a class against statuses, by the statuses each admits;
   - two media types by whether the consumer's admits the provider's own type, since the provider's
     values are that type with parameters added, which the consumer allows;
   - two lists by their elements;
   - a path template with no parameter as the one path it admits, and one with a parameter as admitting
     paths without end, so no set of values contains it;
   - any of the four against `any`, and the three string-valued ones against `string`, as `yes`; a
     media type, list or template against an `equality` or `any-of` as `no`.
   Two templates that differ, a template against a regex, and anything else are `unknown`.
5. **`pact` operators stay opaque.** ADR 0073's operators exist so an interaction nobody can verify fails
   by name; no component runs them, so none is compared.

## Alternatives considered

- **Teach the kernel what each HTTP operator admits.** Faster to write, and exactly the HTTP knowledge
  the kernel boundary keeps out (shape spec §3.5).
- **Declare all four `exact`, and decide template containment.** Template containment is decidable, and
  this checker would gain little from it now: a recorded shape's selector names a template, and a
  consumer's request a path. Left `unknown`, which §8 permits, until someone needs it.
- **Membership for every component operator, declared or not.** Running a matcher is safe, but shape
  spec §8 lets only a declared class narrow `unknown`, so a third party's operator stays opaque until
  its component says otherwise.

## Consequences

Easier: a shape recorded from a real provider compares its headers and status with an upgraded pact's,
so `janus check` reports the provider's variance and nothing about how HTTP spells a media type.

Harder: a verdict that was `unknown` can now be `no`. A team with `on-finding: block` and
`on-review: warn` can see a check that warned now block, where a provider's shape really does admit
what the consumer's does not, such as a status class wider than the statuses a consumer tested.

Committed to: a component's answers are as trustworthy as the walk's own, so each `compare` decision is
pinned by `engine/kernel/tests/subsumption_components.rs`. A declared out-of-tree component's operators
stay opaque in `janus check`, which loads no declared component; S5.7 tracks loading them and
forwarding `matcher/compare` over the component pipe.

**Tripwire.** Revisit decision 4's template rows if consumers write `http:path` in requests that a
recorded template is then compared against, which would make template containment worth deciding.
