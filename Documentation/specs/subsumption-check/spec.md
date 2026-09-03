# Subsumption check specification (v1, draft)

Plan task: **2.8**. Status: **draft — under review**.

The [shape language](../shape-language/spec.md) defines `admits(S)` and, in its §8, exactly which
comparisons a checker can decide about two *shapes of the same operator family* — the identity floor,
the three comparability classes, and the rule that `unknown` is never a guess. That is the checker's
alphabet. This document is the checker: the walk that applies §8's per-operator rules across a whole
interaction tree rather than one node at a time, the two composition rules that are easy to get
backwards, the provider-published artifact the walk's other side reads from, the asymmetric findings the
RFC lists, the output format a human or a CLI renders, and the warn/block policy surface with its
exemption scoping — the RFC's unresolved question this design answers ([ADR
0016](../../decisions/0016-subsumption-defaults-to-warn-with-mandatory-reason-exemptions.md)).

What this specification does **not** do is redefine anything shape-language spec §8 already fixed. The
per-operator comparability classes, the identity floor, and "no guessing" are that specification's
contract with every consumer of `admits`, and design 7.1's property tests check the checker's verdicts
against them directly. Restating them here would create two places that could disagree.

## Contents

1. [Scope and conformance](#1-scope-and-conformance)
2. [Provider shapes](#2-provider-shapes)
3. [The subsumption walk](#3-the-subsumption-walk)
4. [Findings](#4-findings)
5. [Coverage and unexercised regions](#5-coverage-and-unexercised-regions)
6. [The subsumption report](#6-the-subsumption-report)
7. [Policy: warn, block and exemptions](#7-policy-warn-block-and-exemptions)
8. [Errors](#8-errors)
9. [Evolution and compatibility](#9-evolution-and-compatibility)

Worked examples — the shape-language order payload carried through to a full report and rendering, and
policy/exemption scoping — live under [`examples/`](examples/), validated against the schemas below by
`cargo test -p pact_janus_schema_compat`.

---

## 1. Scope and conformance

**MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT** and **MAY** are to be interpreted as described in RFC
2119.

Conformance roles:

- A **checker** — the engine, or a broker with the same rules implemented — reads a consumer contract
  (design 2.5) and a provider shape (§2) for the same provider, walks matched interactions (§3), and
  produces a subsumption report (§6).
- A **publisher** produces a provider shape, by one of the RFC's four provenance routes (§2.3); how it is
  produced is task 7.1–7.3's business, not this specification's.
- A **policy** (§7) turns a report into a warn/block decision and a set of exemptions a team has
  accepted.

In scope: the provider-shape document, the walk's composition rules over shape-language's per-operator
comparisons, the finding vocabulary, the report document and its text rendering, and the policy/exemption
document.

Out of scope, with owners:

| Question | Owner |
|---|---|
| what `admits` means, and the per-operator comparability classes the walk calls at each node | design [2.2](../shape-language/spec.md) §8 |
| how a provider shape is produced — recorded from the provider's own tests, derived from types, authored, observed | tasks 7.1–7.3 |
| combining a subsumption report with verification results into one `can-i-deploy` report, and the `pact check` CLI surface | task 7.4 |
| broker storage and rendering of provider shapes and subsumption reports | task 7.5 |
| property-testing the walk's verdicts against brute-force sampling | task 7.1 |

## 2. Provider shapes

### 2.1 What it is, and what it is not

A **provider shape** publishes, per interaction, the shape of the parts the provider produces — the
response a consumer receives, or the message a consumer receiving role consumes. It is not a contract:
it carries no evidence, no variants, and no consumer name, because one provider shape is compared against
every consumer that has a pact with that provider (RFC: "Providers that publish no shape simply get
today's semantics"). It is also not symmetric with a Janus contract's own `parts` — a provider shape
never carries request-direction slots, because the reverse direction is already covered by replaying the
consumer's recorded request variants at the provider (shape-language spec's own worked example, §7, and
the RFC's own scoping): there is nothing for subsumption to add there.

The kernel does not know which slots of a part are "provider-produced" — that is the transport
component's business, per the architecture rule that keeps HTTP out of the kernel. This specification
therefore does not enforce direction: a checker compares whatever part/slot pairs exist in **both**
documents for a matched interaction (§3.1), and it is publishing guidance, not an engine rule, that a
provider shape should carry only the slots it actually produces. A slot present in one document and
absent from the other contributes no finding — the checker has nothing on one side to compare against,
and reporting a verdict it cannot support would be exactly the guess §8's `unknown` discipline forbids.

### 2.2 Matching interactions

An interaction in a provider shape is matched against a consumer contract's interaction the same way two
interactions within one contract are told apart (contract spec §4.2): **`description` together with the
`states` names**. A provider shape's `states` carry names only, never `params` — a provider does not
generally know the parameter values a given consumer's states bind, only the state vocabulary its own
`state-setup` hooks answer to.

An interaction the checker cannot match to any provider-shape entry gets **no** subsumption result — not
`unknown`, a distinct report state, `not-published` (§6.3) — because "the provider published nothing for
this interaction" is a different fact from "the provider published something the checker could not
decide about."

### 2.3 Provenance

Every provider shape carries a **provenance**, an open string vocabulary naming how it came to exist, in
the RFC's own decreasing order of fidelity: `recorded` (the union of response shapes the provider's own
tests produced — task 7.2), `derived` (imported from types via a content component — task 7.3),
`authored` (written by hand), `observed` (accumulated from verification traffic). Provenance is recorded
for the reader, not for the walk: **the structural comparison in §3 does not vary by provenance, and a
checker MUST NOT decide a different verdict for the same two shapes because one arrived with a different
provenance.** What provenance is for is policy (§7.3) and the human reading a finding — a `derived` shape
overstating the real response space (the RFC's own "everything nullable in the ORM" worry) is a reason to
route its findings to a looser default, never a reason to compute `admits` differently. Keeping the two
separate is what keeps the walk itself honest: a checker that quietly trusted `authored` shapes more than
`derived` ones would be guessing with extra steps.

`provenance` may be set once for the whole document and overridden per interaction, for a publisher whose
own pipeline mixes routes (some interactions self-recorded, others still hand-authored while the recorder
is rolled out).

### 2.4 The document

```json provider-shape
{ "$format": "janus-provider-shape/1",
  "provider": { "name": "orders-api" },
  "provenance": "recorded",
  "interactions": [
    { "description": "get an order",
      "states": [ { "name": "an order exists" } ],
      "parts": { "response": { "body": { "shape": "object", "members": { } } } } } ] }
```

Schema: [`schemas/v1/provider-shape.schema.json`](schemas/v1/provider-shape.schema.json). Members not
named there — `shape`, and everything under it — are design 2.2's document, opaque to this schema, the
same treatment contract spec §5.1 gives the same nesting.

## 3. The subsumption walk

### 3.1 Inputs and the top-level loop

For every interaction in the consumer contract that has a matching provider-shape entry (§2.2), and for
every part/slot pair present in both (§2.1), the checker computes

```
compare(P, C) -> yes | no | unknown
```

where `P` and `C` are the shape nodes at that slot, published and declared respectively. This is exactly
`admits(P) ⊆ admits(C)` at that node (shape spec §2.3, §8). The result at the slot's root, plus every
finding recorded along the way (§4), is that slot's contribution to the report.

### 3.2 The walk is admits containment, decomposed

`compare` is not a bespoke algorithm layered on top of the shape language; it is shape spec §4.1's own
`admits` definitions, read as a recursive containment check, because every composite operator's `admits`
is already defined compositionally there (`optional(of)` is `admits(of) ∪ {⊥}`, `each-like` is an
interval times `admits(items)`, and so on). Two rules govern how containment factors through that
composition, and shape spec §8 states them for scalars but not for the composite case; they are this
design's contribution:

**Rule 1 — leaves are compared by their declared comparability class, composites by recursion.** A node
using an operator from the **exact** or **conservative** or **opaque** class (shape spec §8's table) that
carries no sub-shapes (`regex`, `datetime`, `any-of`, kind predicates, …) is compared directly by that
class's procedure. A node whose operator carries sub-shapes (`object`, `array`, `each-like`,
`each-entry`, `optional`, `nullable`, `one-of`) is compared by recursing into them and combining the
results with **Kleene conjunction**: the result is `no` if any child is `no`; otherwise `unknown` if any
child is `unknown`; otherwise `yes`. This is the ordinary three-valued semantics for "all of these
sub-containments must hold" — object members, array/each-like/each-entry elements plus their cardinality
interval, and `one-of` alternatives are each an AND of sub-comparisons, so the same combinator applies
uniformly and needs no per-operator special case beyond identifying the children:

| Operator | Children combined by Kleene conjunction |
|---|---|
| `object` | one per member name in `C.members` (Rule 2 below governs a name absent from `P.members`) |
| `array` | one per position; a length mismatch is `no` outright (positions the shorter side lacks have no comparison to make) |
| `each-like` | the cardinality intervals' containment, plus `compare(P.items, C.items)` |
| `each-entry` | the cardinality intervals' containment, plus `compare(P.keys, C.keys)` and `compare(P.values, C.values)` |
| `optional` / `nullable` | `⊥ ∈ admits(P) ⟹ ⊥ ∈ admits(C)` (respectively `null`), plus `compare(P.of, C.of)` |
| `one-of` | one per discriminator literal appearing in `P.alternatives` (Rule 3 below governs one `C.alternatives` lacks) |

Cardinality-interval and literal-set containment are themselves plain set/interval containment, already
promised decidable by the exact class (shape spec §8) — they are leaves of this recursion, not further
composites.

**Rule 2 — a member absent from `P.members` is not silence about a narrow claim; it is the widest
possible claim.** `object`'s `admits` checks only the members it names (shape spec §4.3); a name it does
not name is unconstrained — present with any value, or absent — which is wider than every operator except
an equivalent "any value or absent." Concretely: for a member name in `C.members` with no counterpart in
`P.members`, the check is **`no`** unless `C`'s shape at that name already admits everything `⊥` included
(an `optional` wrapping `any`, in practice never written). This is the direction easiest to get backwards
because the *matching* rule for the opposite case — a member `P` names that `C` does not — is must-ignore
and decides `yes` (§4.1); the two are not mirror images. A provider shape that under-declares a member the
consumer's contract constrains will show as `no` far more often than intuition expects, and the fix on the
provider side is to name the member — with `any` if nothing narrower is yet knowable — not to leave it
out, because leaving it out is the widest thing a shape can say.

**Rule 3 — cross-operator comparison is ordinary, not special-cased.** Nothing requires `P` and `C` at a
node to use the same operator: `compare(equality(42), type-integer)` is `yes` by set containment
(`{42} ⊆` every integer) the same way `compare(type-integer, any-of[1, 2, 3])` is `no` (an infinite set is
never inside three literals), and a node under an `object` in `P` compared against a `one-of` in `C`
resolves by testing whether `P`'s admits set — whatever operator produced it — is inside the alternative
`C.discriminator` picks out. The exact class's promise ("decide yes/no for every pair of operators in
this class", shape spec §8) already covers this; this rule exists only to say plainly that the walk
relies on it rather than requiring same-operator pairs, since the RFC's own examples (`number` against
`integer`) could be misread as implying otherwise.

### 3.3 `one-of`

A discriminated union's alternatives are keyed by distinct literal values of `discriminator` (shape spec
§5.4), which makes the lookup exact rather than a search: for each literal `d` that `P.alternatives`
binds, if `C.alternatives` binds the same `d`, recurse `object`-wise into the two alternatives; if it does
not, the result is `no` — the provider can produce a shape of the payload the consumer's contract never
even names an alternative for, which is not narrower than anything the consumer declared. `C` binding
more discriminator values than `P` uses is not a finding: `P` simply exercises a subset of the union, and
subsetting a union is what narrower means.

## 4. Findings

### 4.1 What gets reported

A **finding** is recorded at the *shallowest* node whose verdict is not `yes` — a deeper explanation
(the specific member, the specific array position) is where the reason lives, not every ancestor node on
the path to it, the same discipline `explain`'s executed form uses for match failures (plan-grammar spec
§3.2). A `yes` verdict is never a finding, including the must-ignore case — an extra provider member is
the RFC's own example of the asymmetry Postel's law preserves, and reporting it would teach a team to
ignore the report — **except** the one case §5 defines, where a `yes` carries a coverage caveat worth a
human's attention despite being structurally correct.

### 4.2 The RFC's asymmetric rules, in this walk's terms

| RFC finding | Kind | Where it comes from in §3 |
|---|---|---|
| provider enum/union wider than consumer's | `wider-values` | `any-of` literal-set containment, or `one-of` §3.3's unmatched discriminator |
| provider type broader (`number` where consumer tested `integer`) | `broader-type` | the kind lattice, exact class |
| provider `nullable`/optional where consumer tested only non-null/present | `weaker-presence` | Rule 1's `⊥`/`null` containment on `optional`/`nullable` |
| a member the provider's object shape does not name, that the consumer's does | `undeclared-member` | Rule 2 (§3.2) |
| wider cardinality (e.g. `min: 0` where the consumer declared `min: 1`) | `wider-cardinality` | the interval-containment child of `each-like`/`each-entry` |
| two conservative-class nodes that are not identical and not exactly-nested (two regexes, two datetime formats) | `unreviewable` | shape spec §8's conservative class |
| an opaque-class node (`contains`, an undeclared component operator) that is not identical | `unreviewable` | shape spec §8's opaque class |
| extra provider member; provider using fewer `one-of` alternatives or `any-of` options than the consumer declared | *(not a finding — `yes`)* | must-ignore (§4.1), subsetting a union or enum |

A seventh kind, `excluded-combination`, is not a shape difference at all — it is §5's `yes`-plus-caveat
case, kept out of this table because it does not come from the walk deciding anything, only from a
cross-reference to the consumer's recorded exclusions.

`kind` is an open string vocabulary (§9): a component operator that declares a comparability class (
component-interfaces spec §7.5) and answers `no`/`unknown` from its own `compare` contributes a finding
too, and MAY name its own `kind` rather than being forced into this table's seven.

### 4.3 Severity

Every finding carries a **severity**, computed once from its verdict and stored rather than re-derived,
so §7's policy dispatch never has to re-walk the tree: `finding` for a `no` verdict — a decided
incompatibility — `review` for an `unknown` verdict — an honest "a person must look at this" — and
`advisory` for the §5 case, a `yes` verdict reported only because it carries an `excluded-by` caveat. A
`review` finding is never silently escalated to `finding`, and never quietly dropped: shape spec §8's
whole point is that `unknown` is a first-class answer, not a placeholder for one the checker was too lazy
to compute. `advisory` is policy-inert by construction (§7.1 dispatches only on `finding` and `review`) —
it exists to be read, not to gate anything.

### 4.4 What a finding says

Every finding names its `path`, in shape spec §6.2's own dimension-path grammar (`response.body.status`,
`response.body.items[*].qty`, `response.body.payment@invoice.dueDate`) — reused rather than reinvented,
because a finding and a variant dimension are two different questions about the same address and a
reader should not have to learn two addressing schemes. It carries a short human `summary` for each side
— "one of `PENDING` | `SHIPPED` | `DELIVERED` | `CANCELLED`" for the provider, "one of `PENDING` |
`SHIPPED` | `DELIVERED`" for the consumer — generated deterministically from the operator and its
parameters, the same discipline that keeps `explain`'s pretty form a rendering rather than free prose
(plan-grammar spec §3.1): two checkers comparing the same two shapes MUST produce the same summary text,
so a diff between two report runs is a diff of substance, not of phrasing. §6.2 fixes the phrase table.

Schema: [`schemas/v1/finding.schema.json`](schemas/v1/finding.schema.json).

## 5. Coverage and unexercised regions

The walk in §3 is purely structural: it compares what two shapes *declare*, and says nothing about
whether the consumer's tests actually produced, together, the specific combination a provider's evidence
demonstrates. That gap is real and bounded, not a bug to be designed away: ADR 0008 lets a sampler
**exclude** a cross-dimension combination from the exercised space, with a mandatory reason, "without
narrowing `admits`" (ADR 0008 decision 6) — which means a `yes` verdict at a node can legitimately sit
over a region the consumer's own selection report (variant semantics §4.4) records as never jointly
exercised.

This is the question variant semantics spec §1 defers here: **what does an unexercised region mean for
`can-i-deploy`?** The answer this design gives is narrow on purpose. A checker MAY cross-reference the
dimensions a finding's `path` touches against the consumer contract's recorded exclusions
(`selection.report`, sampling-policy spec's `Exclusion` document) and, where an exclusion's `when` set
covers the dimensions in play, attach it to the node as `excluded-by` — a **caveat, carrying the
exclusion's own reason, that never changes what the walk decided.** Shape spec §8 fixes the walk's output
at exactly `yes` / `no` / `unknown`; this design does not widen it, because a provider shape is a
per-field claim, not a per-combination one, and inferring which *joint* combination the provider's own
evidence actually produced from a per-field shape would be exactly the guess §8 forbids.

A node whose verdict is already `no` or `unknown` simply carries `excluded-by` on the finding §4 already
reports. A node whose verdict is `yes` — ordinarily not a finding at all — is reported anyway, as an
`advisory`-severity finding with `kind: "excluded-combination"` (§4.3), specifically so that a passing
field sitting over unexercised joint coverage does not vanish from the report the way every other `yes`
does. `excluded-by` never manufactures a fourth verdict value; it manufactures a reason to show a `yes`
that would otherwise, correctly, be silent.

## 6. The subsumption report

### 6.1 The document

One report covers one (consumer, provider) pair: every matched interaction, its findings, and an
aggregate verdict per interaction and overall.

```json report
{ "$format": "janus-subsumption-report/1",
  "consumer": { "name": "orders-ui" },
  "provider": { "name": "orders-api" },
  "interactions": [
    { "description": "get an order",
      "states": [ { "name": "an order exists" } ],
      "matched": true,
      "verdict": "no",
      "findings": [
        { "path": "response.body.status", "verdict": "no", "severity": "finding", "kind": "wider-values",
          "provider": { "summary": "one of 'PENDING' | 'SHIPPED' | 'DELIVERED' | 'CANCELLED'" },
          "consumer": { "summary": "one of 'PENDING' | 'SHIPPED' | 'DELIVERED'" },
          "reason": "4 options vs 3" } ] } ],
  "summary": { "interactions": 1, "matched": 1, "findings": 1, "reviews": 0 } }
```

Schema: [`schemas/v1/subsumption-report.schema.json`](schemas/v1/subsumption-report.schema.json).

### 6.2 Aggregation

An interaction's `verdict` is the Kleene conjunction (§3.2, Rule 1) of every part/slot root comparison it
ran. This is the same combinator the walk itself uses internally, applied one level higher, for the same
reason: a report that called an interaction `yes` while one of its slots was `unknown` would be exactly
the false confidence §8 exists to prevent.

### 6.3 `not-published`

An interaction with no matching provider-shape entry (§2.2) gets `"matched": false` and
`"verdict": "not-published"` — a report-level state, distinct from the walk's own three values, that
means *no check ran*, not *the check passed*. This is what keeps the mechanism adoptable per-provider
(RFC): a provider that has published nothing produces a report that says so plainly, rather than a report
full of silent `yes`es a reader could mistake for coverage.

### 6.4 Text rendering

A checker or a reader renders findings grouped by interaction, then by `path`, one line per side, in the
RFC's own style:

```
✗ order-consumer is not compatible with order-service
  interaction 'get an order', response body $.status:
    provider may produce: 'PENDING' | 'SHIPPED' | 'DELIVERED' | 'CANCELLED'
    consumer has only tested: 'PENDING' | 'SHIPPED' | 'DELIVERED'
  interaction 'get an order', response body $.items:
    provider may produce an empty list
    consumer has only tested at least one item
```

The header line's verdict (`✗` for any `finding`-severity result, a distinct marker task 7.4 defines for
`review`-only reports) and the per-finding two-line body are this specification's rendering contract; the
`$.status`-style path is the same address as `path` (§4.4), rendered with a leading `$.` per part/slot
convention rather than shown raw, because this text is read by people who know JSONPath and not
necessarily this project's dimension-id grammar. Combining this block with verification-result lines into
one `can-i-deploy` report is task 7.4's job; this specification fixes the block, not the page it appears
on.

## 7. Policy: warn, block and exemptions

### 7.1 The surface

A **subsumption policy** resolves two independent questions — what happens on a `finding`-severity
result, and what happens on a `review`-severity one — plus the exemptions a team has already decided not
to re-litigate every run.

```json policy
{ "on-finding": "warn", "on-review": "warn",
  "exemptions": [
    { "interaction": { "description": "get an order" },
      "path": "response.body.status",
      "reason": "CANCELLED is a known future state; consumer ticket ORD-451 tracks widening the shape",
      "expires": "2026-12-01" } ] }
```

Schema: [`schemas/v1/subsumption-policy.schema.json`](schemas/v1/subsumption-policy.schema.json). Both
`on-finding` and `on-review` default to **`warn`** ([ADR
0016](../../decisions/0016-subsumption-defaults-to-warn-with-mandatory-reason-exemptions.md)); resolution
layers the same way sampling policy does (variant semantics spec §3.8) — specification default, then
project config, then a per-run override — with `exemptions` accumulating across layers rather than the
last layer replacing the list, the same treatment ADR 0008 gives `exclude`.

### 7.2 Exemption scoping

An exemption's selector members are all optional; an unset selector matches every value on that axis, and
an exemption applies to a finding only when **every selector it sets** matches:

| Selector | Scope |
|---|---|
| `consumer` | every finding for the named consumer |
| `interaction` (`description` + optional `states`) | every finding within that interaction, for any consumer if `consumer` is unset |
| `path` | one field, within whatever `interaction`/`consumer` also narrow |

This is the granularity the plan names directly — per field, per interaction, per consumer — as three
selectors that compose rather than three separate mechanisms, following the same shape ADR 0008's
`Exclusion` already established for a different axis (dimension points instead of report findings).

`reason` is **required**, for the reason ADR 0008's `Exclusion.reason` is required: an exemption is a
team accepting a gap between what the provider may do and what the consumer has tested, and a policy
document that could accept that silently would make "why is this exempted" archaeology instead of a
one-line answer. `expires` is optional but its absence is a smell a report or dashboard SHOULD surface
(task 7.4/7.5) rather than this specification enforcing it structurally — an unconditionally-required
expiry would force a nonsensical date onto a genuinely permanent exemption (a field the provider will
never narrow, by design), and the honesty problem is teams accumulating exemptions and never revisiting
them, not the schema shape.

### 7.3 Provenance and policy defaults, not verdicts

§2.3 fixed that provenance never changes a verdict. It is exactly the input policy is *for*: a team
importing `derived` provider shapes and drowning in `wider-values`/`weaker-presence` findings from an
everything-nullable ORM schema (the RFC's own drawback, and task 7.3's evidence-gathering target) does
not fix that by asking the checker to compute a different `admits` for `derived` shapes — it fixes it by
scoping policy, or exemptions, to provenance. This specification does not add `provenance` as a policy
selector in v1, deliberately: task 7.3 has not yet produced the evidence for how noisy `derived` findings
actually get on a realistic spec, and adding a selector ahead of that evidence risks shaping the surface
around a guess. The `path`/`interaction`/`consumer` selectors already available are sufficient to hand-
scope the same outcome while that evidence is gathered; a `provenance` selector is the natural next
addition once 7.3 reports back, and would be additive (§9).

## 8. Errors

Codes are the protocol's (protocol spec §10, `engine-error.schema.json`).

| Condition | Code |
|---|---|
| not a provider shape, or `$format` names a major this checker does not implement | `pact-invalid`, respectively `pact-version-unsupported` |
| structurally invalid provider shape or policy document | `pact-invalid`, with `problems[]` positions |
| a provider-shape interaction names a part/slot the checker cannot parse as a shape | `interaction-invalid`, naming the path |
| a component operator declaring a comparability class without an implementation `compare` requires (component-interfaces spec §7.5) | `component-failed`, naming the component |

`problems[]` entries carry an RFC 6901 pointer, the same convention contract spec §11 and the protocol
use throughout — a document a person cannot be told the location of a fault in has not been diagnosed.

## 9. Evolution and compatibility

`$format` is `janus-provider-shape/1`, respectively `janus-subsumption-report/1`: one version line each,
governed by the same additive-evolution rules as every other design's schemas, enforced by
[`tools/schema-compat`](../../../tools/schema-compat/README.md).

**Finding `kind` is additive.** A new structural rule, or a component contributing its own kind (§4.2),
is a new value in an open vocabulary; an existing `kind` is never redefined, because a report is read by
whatever policy and dashboard tooling was current when a team decided an exemption, and both need to keep
meaning what they meant.

**The walk is not versioned separately from the shape language it reads.** There is no `walk` algorithm
name the way variant selection has `janus-ipog-v1` (ADR 0008), because the walk has no free parameters —
it is shape spec §8's comparability classes, applied by §3's two composition rules, and improving it means
either shape spec §8 narrowing a class (an additive change there, e.g. deciding more of the conservative
class by policy rather than mathematics, per shape spec §8's own note) or this specification fixing a
composition bug, versioned like any other prose correction. A checker's *conclusions* can therefore get
better over time — narrower `unknown`s — without the recorded provider shape or contract changing at all.
