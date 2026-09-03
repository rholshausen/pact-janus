# Worked example — policy, exemption scoping, and an excluded combination

Continues from [order-payload-subsumption.md](order-payload-subsumption.md). `orders-ui`'s team has seen
the report there and wants to keep `on-finding` at the default while it fixes what it can, accept one
finding permanently, and understands what an `excluded-by` cross-reference on a passing field would mean.

Fence markers: ```` ```json policy ```` validates against
[`subsumption-policy.schema.json`](../schemas/v1/subsumption-policy.schema.json), ```` ```json finding ````
against [`finding.schema.json`](../schemas/v1/finding.schema.json); ```` ```json sketch ```` is design
2.3's `Exclusion` document, reproduced for context.

## 1. Three exemptions, three scopes

```json policy
{ "on-finding": "warn", "on-review": "warn",
  "exemptions": [
    { "path": "response.body.status",
      "reason": "CANCELLED is a real future state; widening the consumer's anyOf is tracked as ORD-451",
      "expires": "2026-12-01" },
    { "interaction": { "description": "get an order", "states": ["an order exists"] },
      "path": "response.body.payment@card.last4",
      "reason": "two card-number regexes that are provably equivalent (both exactly four ASCII digits); accepted until the shape language gets a documented regex-equivalence procedure" },
    { "consumer": "legacy-billing-sync",
      "reason": "legacy-billing-sync only reads response.body.id and ignores the rest of the payload; every other finding against it is noise until that consumer is retired" } ]}
```

Read as scoping, narrowest first:

| Exemption | Applies to | Selectors set |
|---|---|---|
| 1 | every consumer, every interaction, only `response.body.status` | `path` |
| 2 | every consumer, only `get an order`'s `last4`, no other field | `interaction`, `path` |
| 3 | every field, every interaction, only `legacy-billing-sync` | `consumer` |

Exemption 1 has no `interaction` selector, so it silences the `wider-values` finding from
[§3 of the previous example](order-payload-subsumption.md#3-the-report) for `orders-ui` too, wherever
`response.body.status` appears — which today is the one interaction. Exemption 1 also carries `expires`;
exemption 2 does not, because it records a belief that the two regexes are equivalent forever, not a
temporary gap — the kind of permanent exemption spec.md §7.2 says should not be forced into carrying a
meaningless date. A dashboard (task 7.5) is where "no `expires`" gets surfaced for review, not this
document.

## 2. What `on-finding: "block"` would change

Nothing in the documents above — `on-finding` and `on-review` are read at report time, by task 7.4's `pact
check`, not baked into the report itself (spec.md §6.4). Raising `on-finding` to `"block"` after exemptions
1 and 3 are in place would still let `get an order` pass `can-i-deploy`: the `wider-values` finding is
exempted, the `unreviewable` payment finding is a `review`-severity result governed by `on-review`, and
`wider-cardinality`/`broader-type` on `items` remain live findings that a `block` policy would stop the
deploy on — exactly the two the consumer has not yet decided to accept or fix.

## 3. An excluded combination, cross-referenced

Suppose the consumer's own sampler recorded an exclusion (ADR 0008) between two dimensions of a different
interaction, `place an order`:

```json sketch
{ "when": [ { "dimension": "request.body.expedited#presence", "point": "present" },
            { "dimension": "request.body.giftWrap#presence", "point": "present" } ],
  "reason": "expedited + gift-wrapped orders are not offered together by the checkout UI" }
```

If a provider shape's `response.body.handlingFee` node happens to be admitted (`yes`) by the consumer's
shape only because the consumer's declared width covers both `expedited` and `giftWrap` independently —
and never jointly, per the exclusion above — the checker MAY attach the exclusion to that field's
(passing) comparison as a caveat, not a finding:

```json finding
{ "path": "response.body.handlingFee", "verdict": "yes", "severity": "advisory", "kind": "excluded-combination",
  "provider": { "summary": "a surcharge amount" },
  "consumer": { "summary": "a surcharge amount" },
  "reason": "structurally identical to the consumer's shape (identity floor) — reported only because an exercised-coverage caveat applies",
  "excluded-by": [
    { "when": [ { "dimension": "request.body.expedited#presence", "point": "present" },
                { "dimension": "request.body.giftWrap#presence", "point": "present" } ],
      "reason": "expedited + gift-wrapped orders are not offered together by the checkout UI" } ] }
```

The walk itself still decided `yes` — the identity floor, unchanged by anything in this section (spec.md
§5) — so `verdict` says exactly that. What makes this worth reporting at all is `severity: "advisory"`:
spec.md §4.1's rule that a `yes` is never a finding has exactly one exception, this one, and it exists so
that a passing field sitting over unexercised joint coverage does not simply vanish the way every other
`yes` does. `on-finding`/`on-review` policy (§1 above) never sees this entry — `advisory` is policy-inert
by construction (spec.md §4.3) — a human reads it or does not.
