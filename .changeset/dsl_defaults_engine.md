---
engine: minor
---

# An empty list is opt-in and covers nothing inside it, an interaction carries its own sampling policy, and a provider object is split by a consumer's union

These are staged plan S3.1's decisions (gate D, ADRs 0079–0081).

- **`janus upgrade` writes a bare `type` rule on an array as `each-like` with `min: 0`.** A v1–v4 pact
  that names no length admits an empty list, and the contract now does too. The `rule-narrowed`
  finding for it is gone. pact-jvm's `eachLike` and `eachArrayLike` keep their meaning through the
  facade, so the `facade/pact-jvm-matchers-collections` conformance case now pins `min: 0` for both.
  A pact that states a `min` or a `max` converts as before.
- **A list whose `min` is 0 is exercised with an element first.** Its cardinality dimension's default
  is `min+1`, and the dimensions inside it are gated by every point with an element. The variant-space
  document writes one gate per point. Gates that name one dimension are alternatives, which no earlier
  engine wrote. The empty list is the minimal boundary variant, and it assigns nothing inside the list.
  The selection for such a list changes. A contract already recorded verifies as before, because a
  verifier replays what the contract recorded. Selections of spaces with no `min: 0` list do not change.
- **An interaction specification may carry `policy`**, a sampling-policy document, applied between the
  session's policy and the `variants` call's. Its pins' and exclusions' references are resolved at
  `add-interaction`. A reference that names nothing, a point the dimension does not have, or an
  exclusion with an empty reason is `interaction-invalid` there. The contract records `policy` on the
  interaction with full dimension ids.
- **An exclusion is reported under `exhaustive`**, with the variants it removed. Before, it was reported
  only under `t-wise`.
- **The subsumption walk splits a provider object against a consumer `one-of`.** When the provider's
  discriminator is an `equality` or an `any-of`, the object is compared once per consumer alternative
  its literals reach. A literal no alternative binds is `wider-values`. Before, any provider that bound
  more than one literal was `unknown`.
