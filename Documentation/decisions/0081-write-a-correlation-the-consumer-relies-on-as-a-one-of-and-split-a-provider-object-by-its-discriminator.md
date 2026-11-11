# 0081 — Write a correlation the consumer relies on as a `one-of`, and split a provider object by its discriminator

- **Status**: accepted
- **Date**: 2026-11-11
- **Plan tasks**: S3.1 (gate D), phase-9 finding 35 option B; feeds S3.2, S4.1
- **Evidence**: [phase-9 finding 35](../prototype/phase-9-findings.md#35-the-dsl-cannot-exclude-a-combination-the-provider-cannot-produce);
  shape spec [§4.4](../specs/shape-language/spec.md#44-presence-and-choice-operators) and
  [§5.4](../specs/shape-language/spec.md#54-one-of); subsumption spec
  [§3.2](../specs/subsumption-check/spec.md#32-the-walk-is-admits-containment-decomposed) and
  [§3.3](../specs/subsumption-check/spec.md#33-one-of); `Walk::inside_alternative`
  (`engine/kernel/src/subsumption/compare.rs`); the sample provider's recorded shape
  (`samples/order-service/shapes/`); the RFC's order payload, measured below

## Context

A shipped order has a shipping date and a pending one does not. A shape can say this in two ways.
Written independently, `status` is an `any-of` and `shippedAt` an `optional`, and an exclusion stops
the sampler pairing `SHIPPED` with an absent date (ADR 0080). Written as a `one-of` keyed by `status`,
the alternatives are a `pending` object without `shippedAt` and a `shipped` object, whose `status` is
`anyOf('SHIPPED', 'DELIVERED')`, with it.

The two spellings admit different sets. An exclusion governs only what is exercised (variant
semantics §3.5), so the independent shape still admits a `SHIPPED` order with no date. If the
consumer reads `shippedAt` whenever it sees `SHIPPED`, the contract accepts a response the consumer
breaks on, and a provider that stops sending the date passes its verification. The `one-of` admits
only what the consumer handles, so that provider fails.

The `one-of` spelling is not free. An alternative is a whole object, so the members common to every
alternative are written in each. Each copy is then its own dimension, gated by its alternative:
`response.body@pending.payment#alternative` and `response.body@shipped.payment#alternative`. Measured on
the RFC's payload, the `one-of` spelling selects 7 variants from a space of 12. The independent
spelling selects 8 from 24.

Finding 35 asked a second question: whether the subsumption walk compares a provider's object
against a consumer's `one-of` usefully. It does not. Rule 3 (§3.2) looks up the alternative named by
the provider's discriminator, and `inside_alternative` requires the provider to bind it to exactly
one literal. A recorded provider shape binds `status` to the `any-of` of every value it observed
(`CANCELLED`, `PENDING`, `SHIPPED` for the sample provider), so the walk answers `unknown`
("unreviewable") for every recorded provider against every consumer `one-of`.

## Decision

1. **A correlation the consumer relies on is written as a `one-of`.** It is keyed by the member that
   decides the correlation, and each alternative names the members that come with it. The SDK
   specification's guidance, and S3.2's worked reference, write the RFC's `status` and `shippedAt` this
   way when the consumer reads one because of the other. The contract then fails the provider that
   breaks the correlation.
2. **An exclusion is for a combination the consumer handles but the provider cannot produce.** It is
   also for a correlation no member discriminates, such as two optional members that are present
   together. An exclusion keeps the consumer's width and drops a test the provider could not satisfy.
   It is not a way to make a correlation the consumer depends on look checked.
3. **The shape language gains no construct for shared members.** An alternative is a complete object
   (§5.4), so a common member is written in each alternative. The DSL makes that cheap where the
   language does: TypeScript spreads a common object into each alternative, and S3.2's style guides
   record each language's way. A reference to a member inside alternatives names one copy, so
   `whenVariant('payment', ...)` must qualify it (`'@shipped.payment'`) once there are two, and §6.3's
   uniqueness rule already says so.
4. **The subsumption walk splits a provider node by its discriminator's literals.** When the
   consumer's node is a `one-of` and the provider's is not, and the provider's shape at the
   discriminator member admits a finite set of literals (an `equality` or an `any-of`), the walk
   compares once per consumer alternative those literals reach. It compares the provider's node, with
   the discriminator narrowed to the literals that alternative binds, against the alternative, and
   combines the results by Kleene conjunction, as §3.3 does for two `one-of`s. Two literals that reach
   one alternative (`SHIPPED` and `DELIVERED`) are compared together, so a finding inside it is
   reported once. Literals the consumer's alternatives do not bind are `no`, `wider-values`, as §3.3
   already says. A provider whose discriminator is open (a `string`, or no
   such member) stays `unknown`: which alternative it is in is not decidable from the shape.
5. **A provider whose correlation matters says so in its own shape.** A per-field provider shape
   answers per field (§5), so against the consumer's `shipped` alternative, a recorded `shippedAt`
   that is sometimes absent is `no`. That is correct for the shape as recorded, and it may not reflect
   what the provider actually does. A provider shape can be a `one-of` too, and S4.1 and S4.2 decide
   whether recorders and importers write one.

## Alternatives considered

- **Always write correlated members independently, with exclusions** (finding 35 option A alone).
  The contract then admits what the consumer breaks on, and an exclusion governs only what is
  exercised (ADR 0008 decision 6). That is the false pass the shape language exists to remove.
- **Always use `one-of`, and exclusions only to stay inside a budget.** Some correlations have no
  discriminating member, and some combinations the consumer handles are ones the provider cannot set
  up. Forcing those into a `one-of` invents a tag.
- **Add shared members to `one-of`** (a `common` member the alternatives extend). It is a second way to
  write every `one-of`, with an inheritance rule for the alternatives to follow. The DSL removes the
  repetition with no change to the shape language, which is frozen at v1.
- **Leave the walk at `unknown`.** Recorded provider shapes always bind the discriminator to several
  literals, so every consumer `one-of` stays unreviewable, including the RFC's `payment`.

## Consequences

- A consumer that relies on a correlation gets a contract that fails a provider for breaking it. The
  variant count is usually no larger, since one alternative's interior is not paired with another's.
- The DSL repeats shared members, so a long common part makes a `one-of` verbose in languages without
  spreading. S3.2's style guides must give each language a way to write it once.
- `janus check` decides a recorded provider against a consumer `one-of` literal by literal. A tag the
  consumer never named is `wider-values` and a missing correlated member is `no`, in place of one
  `unknown`. The sample provider's report against a consumer that writes `status` as a `one-of` names
  `CANCELLED` as a status the consumer does not handle.
- A provider's recorded shape is held to correlations it cannot record. This is the per-field limit
  §5 already names, now visible as `no` on a correlated member, not hidden behind `unknown`.
- **Tripwire**: pilot providers routinely exempting correlated-member findings against recorded
  shapes. That is the signal for S4.1 to record a `one-of` by the consumer's discriminator, or for a
  superseding ADR on how the walk reads a per-field provider against a union.
