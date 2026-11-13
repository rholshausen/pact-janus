# 0089 — Scope a subsumption policy and its exemptions by the provenance of the shape a finding came from

- **Status**: accepted
- **Date**: 2026-11-13
- **Plan tasks**: S4.4 (amends subsumption-check spec §6.1, §7.1–§7.3; finding 9B)
- **Evidence**: [spike 7.3 findings](../../spikes/7.3-type-derived-shapes/FINDINGS.md) §4.3 and §6,
  [ADR 0016](0016-subsumption-defaults-to-warn-with-mandatory-reason-exemptions.md),
  [ADR 0088](0088-derive-a-provider-shape-from-openapi-with-janus-derive-and-report-every-mapping-that-loses-something.md)

## Context

A provider shape says how it came to exist, its `provenance` (subsumption-check spec §2.3): `recorded`
from the provider's own tests (S4.1), `derived` from a type document (S4.2), `authored`, `observed`. The
walk ignores it, by rule. Policy is where it was always meant to matter, and spec §7.3 deferred the
selector until spike 7.3 measured how noisy a derived shape is.

It measured. A generator-emitted OpenAPI document gave 4.5 times the findings of a hand-authored one,
O(fields), and §4.3 found that nothing separates the noise from the signal but provenance: the true
`wider-values` on `$.status` and the generated `weaker-presence` on `$.status` share an interaction and
a path, so an exemption scoped by path silences both. The configuration the evidence supports is `block`
on `recorded` and `warn` on `derived`, which no policy can say today: `on-finding` is one action for
every finding.

A policy that reads provenance needs the report to carry it. Today it does not: a subsumption report
says which entry an interaction matched only by how (`matched-by`), not what kind of evidence that entry
was.

## Decision

1. **The report carries each matched interaction's provenance.** A subsumption report's interaction
   result has `provenance`: the matched entry's own, or else its document's (§2.3). An unmatched
   interaction has none. A report written before this has none either, and a policy reads its
   interactions as having no provenance.
2. **A policy may name actions per provenance.** Beside `on-finding` and `on-review`, a policy has
   `provenance`: a map from a provenance to `{ on-finding?, on-review? }`. A finding from an interaction
   whose provenance the map names, for a severity that entry sets, takes that action. Every other finding
   takes `on-finding` or `on-review`.

   ```yaml
   on-finding: block
   provenance:
     derived: { on-finding: warn }
   ```

3. **The more specific setting wins, whatever layer set it.** Layers resolve as §7.1 says, each
   overriding what it sets, with one rule on top: a provenance's own action beats the plain one, so a
   later layer's `on-finding` does not override an earlier layer's `provenance.derived.on-finding`.
   `janus check --on-finding block` over a project policy that warns on derived findings blocks every
   finding but a derived one, which is what the team that wrote "derived warns" meant. Between two
   settings alike in specificity, the later layer wins, as today. Within the map, layers merge by
   provenance and by severity.
4. **An exemption may select by provenance.** `provenance` joins `consumer`, `interaction` and `path` as
   a selector (§7.2): set, it matches a finding only from an interaction of that provenance, compared
   exactly. A report with no provenance matches no exemption that sets one.
5. **The compatibility report says it.** Its echoed `policy` carries the `provenance` map when one was
   resolved, each finding entry carries its interaction's `provenance`, and a pair's reasons are split by
   action: `subsumption-findings` (and `-reviews`) once for each action its live findings resolved to,
   each counting its own and, when the policy names any provenance, naming the provenances it counted.
   A pair blocks when any of its reasons does, as now. The text page's policy line names each
   provenance's actions after the plain ones.
6. **`janus check` reads it from `--policy`.** Its `--on-finding` and `--on-review` keep their one form,
   the per-run override of the plain actions. A flag form for provenance is additive, and waits until
   someone asks.
7. **No default changes.** Every action still defaults to `warn` (ADR 0016), for every provenance.
   Whether `block` becomes a default for any of them is gate P, judged on pilot data.

## Alternatives considered

- **`on-finding` as an action or a map** (`on-finding: { recorded: block, derived: warn }`). One member
  fewer, and a member whose type changes, which a reader of today's documents cannot read (§9's
  open-world rules).
- **A list of rules, each selectors and actions**, generalising exemptions to actions. It would let a
  policy block one consumer's findings and warn on another's, which nobody has asked for, and its
  precedence between overlapping rules is a design of its own. Provenance is the one axis the evidence
  names.
- **The later layer always wins**, as `on-finding` does today. Then a CI script's `--on-finding block`
  resurrects every derived finding the project's policy had routed to `warn`, and the only way to keep
  that routing is to repeat it on the command line.
- **Weighting the walk by provenance.** §2.3 forbids it, and spike 7.3 §6.3 found every derived finding
  true: the question is which true things are worth a person's attention, which is policy's.

## Consequences

Easier: a team can block on what its provider's own tests produced and still see what its type schemas
claim, without exempting the derived findings away.

Harder: a pair's decision now depends on which entry each interaction matched, so a provider that
republishes a recorded shape as a derived one can change a pair from `block` to `warn` with no change to
either side's behaviour. That is what the policy asked for, and the report names the provenance beside
every finding so a reader can see it happen.

Committed to: `provenance` is a member of the policy, of an exemption, of a subsumption report's
interaction and of a compatibility report's finding entry and policy. The specificity rule is part of
§7.1's layering.

**Tripwire.** Revisit decision 3 if teams set provenance actions on the command line by repeating them
in a policy file they generate per run, which is the flag form decision 6 left out; and decision 2's
shape if a second axis (consumer, operation) turns out to need actions of its own, which is the rules
list this ADR declined.
