---
default: minor
---

# A run's summary counts its variants per source document

The summary `verification/finished` carries gains `pairs` (ADR 0033). It is positionally parallel
to `consumers` and `providers`, with one entry per source document: its `consumer` and
`provider`, its `status` (`verified`, `failed`, or `incomplete` when the run stopped before
finishing it), and its own `variants` counts. The run-level `variants` is their sum. A pending
document that failed is `failed` in its entry; whether that fails the run is still the run's
`status`.

`subsumption/decide`, and so `janus check`, now reports a pair's own variant counts for a run over
several contracts. When a run verified two documents between the same consumer and provider, for
example a consumer's `main` and deployed pacts fetched from a broker, their counts are added up.
Before, the counts were left out for any run over more than one pair. A summary from before `pairs`
is read as it was.
