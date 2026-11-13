---
engine: minor
---

# A subsumption policy can act on where a provider shape came from

Staged plan S4.4, ADR 0089. A provider shape says how it came to exist, its `provenance` (`recorded`,
`derived`, ...), and a policy can now act on it:

- **A policy's `provenance`** names actions per provenance:
  `{ "on-finding": "block", "provenance": { "derived": { "on-finding": "warn" } } }` blocks on what the
  provider's own tests recorded and warns on what its OpenAPI document says. A provenance's own action
  beats the plain one whatever layer set either, so `janus check --on-finding block` leaves a project
  policy's derived `warn` standing.
- **An exemption's `provenance`** selects findings from shapes of that provenance only, beside
  `consumer`, `interaction` and `path`.
- **A subsumption report** records each matched interaction's `provenance`; a compatibility report
  echoes the policy's `provenance` map, gives each finding entry its `provenance`, and splits a pair's
  `subsumption-findings` and `subsumption-reviews` reasons by the action their findings resolved to,
  naming the provenances when the policy names any.

Every action still defaults to `warn`. A report written before this records no provenance, and its
findings take the plain actions and match no exemption that names one.
