# 0025 — Let a provider-shape entry select the interactions it is about by operation

- **Status**: accepted
- **Date**: 2026-09-24
- **Plan tasks**: 9.2 (amends design 2.8 §2.2 and §6; feeds 9.4 and the broker notes)
- **Evidence**: [spike 7.3 findings](../../spikes/7.3-type-derived-shapes/FINDINGS.md) §2 (the blocking
  finding), [broker integration notes](../broker-integration-notes.md) §2,
  [ADR 0016](0016-subsumption-defaults-to-warn-with-mandatory-reason-exemptions.md)

## Context

Design 2.8 §2.2 matches a provider-shape entry to a consumer interaction by the consumer's
**description** and state names — the identity a contract uses internally (contract spec §4.2). That works
for a shape recorded from the provider's own tests only if those tests happen to use the consumer's words,
and task 7.2's demonstration did because it was written to. It cannot work for a shape derived from types:
an OpenAPI document has paths, methods and `operationId`s, and no generator will ever emit the prose a
consumer team typed. Spike 7.3 found every derived interaction reported `not-published` — "no check ran" —
rendered as a polite "no shapes published", which reads like reassurance. The only workaround was a
hand-written map from `operationId` to description, written per consumer and failing silently when stale.

So the mechanism the RFC ranks second in fidelity, and the one that costs a provider nothing to adopt,
produced no findings at all. The RFC itself says a provider publishes a shape "for each *operation*".

The constraint on any fix: the kernel knows nothing about HTTP. It cannot parse `/orders/{id}` or know that
`method` and `path` identify an HTTP operation.

## Decision

1. **A provider-shape entry MAY carry a `selector`**: shapes over slots, in the same part → slot → shape
   form as `parts`, usually over the request-direction slots that identify an operation. An OpenAPI
   importer writes one per operation — `method` as `equality`, the path template as an anchored `regex`.
2. **The checker matches in two steps.** First by description and state names, exactly as before. Failing
   that, by selector: an entry is selected when its shapes admit the recorded values of **every** variant
   the contract records for the interaction. An entry that names states must also match them; one that
   names none is about the operation in any state.
3. **A tie is not broken.** More than one selected entry leaves the interaction `not-published`, with a
   `reason` naming the entries — the same "no guessing" discipline the walk applies to `unknown`.
4. **The report says how each interaction matched** (`matched-by`: `description` | `selector`), and a
   report in which nothing matched says that no published shape matched, not that none was published.
5. **A selector is matched against examples and never compared for subsumption.** It is evaluated by
   compiling its shapes into a plan and running it on recorded values — the thing the kernel does on every
   exchange — so no HTTP knowledge enters the kernel, and it makes no claim about what the provider
   accepts, which keeps design 2.8 §2.1's "no request-direction slots" true of what is *compared*.

## Alternatives considered

- **An open `operation` document the transport component interprets** (spike 7.3 option a). Correct in
  spirit, and it needs a new transport-interface operation to decide "is this request an instance of this
  operation". A selector gets the same answer from machinery that already exists, and a component that
  wants a richer notion of operation can still contribute an operator to write it with.
- **States plus a `source` cross-reference** (option b). States are not unique per operation.
- **Keep §2.2 and document the mapping file** (option c). A per-consumer file that fails silently when
  stale, pushed onto exactly the teams the mechanism is meant to help.

## Consequences

Easier: a derived shape is useful the day it is imported, for every consumer, with no mapping to keep;
`recorded` shapes keep matching by description, unchanged.

Harder: a selector that is too loose matches the wrong interaction, and one that is too tight matches
nothing. The first is caught by decision 3 when two entries collide, and neither can be caught when only
one entry exists — which is why the report records `matched-by`, so a reader can see which pairs never
agreed on a name.

Committed to: identity between the two documents is a *question about examples*, answered by the plan
interpreter. A future `observed` provenance can select the same way, since verification traffic has
examples and no descriptions.

**Tripwire.** Revisit if importers routinely produce selectors that tie (the path-template-as-regex mapping
is too coarse, and a specificity rule — literal segments over templated ones, as every HTTP router has —
is worth specifying); or if a non-HTTP provider cannot identify an operation from its request slots at
all, which would argue for option a's transport-owned operation after all.
