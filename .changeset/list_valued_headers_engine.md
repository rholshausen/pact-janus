---
engine: major
---

# A header is a list where the contract says so, and a v1–v4 header's rule applies to each value

Staged plan S3.3, ADR 0082.

- **`http:list`.** The built-in HTTP component runs a new operator, `http:list`, whose member
  `elements` is a list's elements in order. It admits a header whose field lines, joined with commas
  and split at each comma outside a quoted string, are those elements: `a, b`, `a,b` and two field
  lines `a` and `b` all match `elements: ["a", "b"]`. An embedding that registers the HTTP component's
  matcher actions (`ACTIONS`) gets it with no change.
- **Headers are field lines.** The HTTP transport never splits a header. A served HTTP/2 or HTTP/3
  request's `Cookie` crumbs are joined into one line with `"; "`, as HTTP/1.1 sends it.
- **`janus upgrade`** writes a header v1–v4 compare as a list, with no rule, as `http:list`: a header
  `pact_models` splits (`Accept`, `Vary`, `Cache-Control`, …) or a value with a comma. A reply that
  frames the list another way now passes the contract, as it passes the pact. A header with a rule is
  an `each-like` of the rule's shape with `min: 1`, so its recorded variant space has a cardinality
  dimension, though a converted interaction still runs base-only. The `rule-narrowed` judgement on
  headers is left only for a value with parameters.
- **A v1–v4 header's rule applies to each of its values,** as today's verifier applies it, where it
  was applied to their join. `janus verify` captures a reply's header as its values, split as
  `pact_models` splits a list header, and `legacy:header-value` and `legacy:body-type` read several
  values as their join with `", "`. A plan compiled from a pact with a header rule is a
  `flow:for-each` over the header's values, behind an `expect:not-empty`.
- **A contract writes a component operator's members by name,** and each value as it arrived. An
  existing contract with an `http:media-type` or `http:status-class` node is written with that node's
  members in another order the next time it is written.
- **Conformance.** The behavioural specification has a `header-list` primitive, and three ids:
  `shape.header-list.compiled-on-the-header`, `shape.header-list.headers-only` and
  `shape.header-list.any-framing`, covered by `translation/header-list`,
  `translation/header-list-in-a-query`, `translation/header-list-in-a-body` and
  `live/header-list-any-framing`. `janus-sdk-gates conformance check` expects them in every report,
  so an SDK must implement `header-list` to move to this release. Four pinned facade contracts record
  a header rule as an `each-like`.
