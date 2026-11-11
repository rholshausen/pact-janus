# 0080 — Write an exclusion on the interaction, as its own sampling policy

- **Status**: accepted
- **Date**: 2026-11-11
- **Plan tasks**: S3.1 (gate D), phase-9 finding 35; feeds S3.2
- **Evidence**: [phase-9 finding 35](../prototype/phase-9-findings.md#35-the-dsl-cannot-exclude-a-combination-the-provider-cannot-produce);
  ADR 0008 decision 6; variant semantics [§3.5](../specs/variant-semantics/spec.md#35-exclusions),
  [§3.8](../specs/variant-semantics/spec.md#38-the-policy-document) and
  [§6.3](../specs/variant-semantics/spec.md#63-dimension-references); `Session::variants`
  (`engine/kernel/src/protocol/session.rs`), which resolves the session's and the call's policy and
  no other; the RFC's order payload, measured below

## Context

The RFC's consumer example declares `status: anyOf('PENDING', 'SHIPPED', 'DELIVERED')` and
`shippedAt: optional(...)` independently. Pairwise sampling therefore pairs `SHIPPED` with an absent
`shippedAt`, which the sample provider cannot produce, so its verification reports
`state-unavailable` (variant semantics §6.7). ADR 0008's remedy is an exclusion with a required
reason. The engine already applies one. Given two exclusions on the `variants` call, the order payload
still selects 8 variants, and the report records each exclusion, its reason and the one target it
removed. No SDK primitive writes one, so an SDK user can only narrow the shape.

Variant semantics §3.8 resolves a policy in four layers: the specification's defaults, the session
configuration, the interaction specification's own policy, and the `variants` call. The engine reads
the second and the fourth. The interaction specification has no member for the third, so nothing
reads it.

## Decision

1. **An interaction's exclusions are written on the interaction.** The DSL gains an `exclude`
   primitive on the interaction builder. It takes a conjunction of (dimension, point) pairs and a
   reason, for example `.exclude([['status', 'SHIPPED'], ['shippedAt', 'absent']], 'the order service
   sets shippedAt when it ships')`. Each call is one exclusion, and calls accumulate in the order
   written. A dimension is named the way `whenVariant` names one: a reference that resolves to one
   dimension (§6.3). S3.2 specifies its spelling in the behavioural specification, and each style guide
   records how its language writes the pairs.
2. **The interaction specification gains a `policy` member, and it is §3.8's third layer.** It is a
   sampling-policy document (`sampling-policy.schema.json`). An SDK's `exclude` writes its `exclude`
   list, and S3.2 decides whether the DSL exposes the policy's other members. `pin` and `exclude`
   accumulate across layers, and every other member overrides, as §3.8 already says. The engine applies
   the interaction's policy at `consumer-session/variants`, between the session's and the call's.
3. **References resolve when the interaction is added.** At `add-interaction`, the engine validates
   the policy and resolves each pin's and exclusion's references against the interaction's variant
   space, as it does a variant binding. A reference that matches no dimension, or more than one, and a
   point the dimension does not have, are `interaction-invalid`, each with a `problems[]` entry naming
   it and listing the candidates. The author sees the mistake at `execute`, beside the DSL that made
   it.
4. **The contract records the policy with its references resolved.** An interaction record carries
   `policy` when its specification did, with every `dimension` written as the full id. A verifier
   therefore resolves nothing, as §6.3 already requires for bindings. The selection report records
   what each exclusion removed, as it does today. The policy is part of the interaction record, so two
   test files that record one interaction with different exclusions are a `contract-conflict` (ADR
   0065).
5. **A reason is required by the schema, and the engine enforces it.** An SDK does not check for one,
   since the engine does. A missing or empty reason is `interaction-invalid`.
6. **A converted interaction takes no policy.** `add-pact-interaction` records a base-only selection
   (ADR 0066 decision 3), so there is nothing to exclude. A facade spells today's DSL, which has no
   exclusions.

## Alternatives considered

- **Pass the exclusion on each `variants` call** (the fourth layer). This needs no schema change, but
  the exclusion stays outside the document the author submitted. A host that replays an interaction
  specification would get different variants from the same document, and an exclusion would be
  resolved late, when `variants` is called, not when the interaction is added.
- **Exclusions in the session configuration only.** That layer is for suite-wide policy. An exclusion
  names one interaction's dimensions, so a session-wide one would be `interaction-invalid` for every
  other interaction that lacks the dimension.
- **No primitive: correlations are written as `one-of` only.** ADR 0081 makes `one-of` the answer when
  the consumer relies on a correlation. An exclusion is still needed when the consumer handles a
  combination but the provider cannot produce it, and when no member discriminates the case.

## Consequences

- The RFC's example can be written as it is, plus one exclusion, and its contract says out loud which
  combination was not demonstrated and why.
- The contract-file schema's `InteractionSpec` and `Interaction` gain an optional `policy`. Both SDKs'
  bindings are regenerated, and a contract with no exclusions is written byte for byte as before.
- `add-interaction` does work it did not do before: it validates the interaction's policy and
  resolves its references. The session's policy and the call's are still checked at `variants`.
- An exclusion governs what is exercised, never what is admitted (§3.5). A provider that sends the
  excluded combination still passes, and `janus check` reports the region as `excluded-combination`
  (subsumption spec §5).
- **Tripwire**: pilot contracts that carry more exclusions than dimensions, or exclusions written to
  stay inside `max-variants` rather than because the provider cannot produce a combination. Either
  means authors need a narrower shape or a budget, not more exclusions.
