---
engine: minor
---

# A `Content-Type` with no rule is compared as a media type in an upgraded contract

The built-in HTTP component runs `http:media-type` (ADR 0071, staged plan S2.16): a value of the
same type and subtype as its `type` member, in any case, with each of its parameters, other
parameters allowed. `janus upgrade` and `add-pact-interaction` write a `Content-Type` with no matching
rule as it, in a request, a response and a message's metadata, where they wrote `equality`, so a
reply or request that adds a charset the pact does not name passes, as it does today. Such an
interaction now requires the `http` component, which the native engines register; its finding is a
`rule-narrowed` note rather than a judgement. A pact verified as a pact is unchanged.
