# 0033 — Count a run's variants per source document in its summary

- **Status**: accepted
- **Date**: 2026-10-19
- **Plan tasks**: S1.3 (the build); task 7.4 (`subsumption/decide`, which reads it)
- **Evidence**: [Phase 9 finding 10](../prototype/phase-9-findings.md) (a summary cannot say how
  many variants one pair ran); the [broker integration notes](../prototype/broker-integration-notes.md)
  §11 (option A); [ADR 0032](0032-speak-to-the-broker-through-protocol-operations-backed-by-a-native-host-capability.md)
  (fetching and publishing inside the run, per pact)

## Context

A verification run verifies several documents, and its summary has one set of variant counts for
all of them. `subsumption/decide` answers per pair, so for a run over more than one pair it has had
to leave the counts out: a page for a four-contract run reads `verified` with no numbers (finding
10). The finding listed three options, and the broker notes chose the first, per-pair
sub-summaries in the run summary, because a broker keys results to a matrix row and a run-level
tally is no use to it.

What neither settled is what an entry is *for*. Since ADR 0032 a run can fetch from a broker, and a
broker run often fetches two pacts between the same consumer and provider: say, the one on `main`
and the one deployed to production, each a different consumer version and so a different row of
the broker's matrix. The run already treats them separately: each has its own pending status, and
each gets its own published result.

## Decision

1. **The summary gains `pairs`: one entry per source document**, positionally parallel to
   `consumers` and `providers`, as `pending` and `publication` already are. Each entry has the
   document's `consumer` and `provider`, its `status`, and its own `variants`, with the same members
   as the run-level counts, which they add up to.
2. **An entry's `status` is `verified`, `failed`, or `incomplete`** (the run stopped before it
   finished the document: a hook aborted it, or never reached it). A pending document that failed
   is `failed`. That pending failures don't fail the run is the run-level `status`'s business, not
   the entry's.
3. **`subsumption/decide` reads the entries.** A pair's status is the worst of its entries; its
   counts are its entries' counts added up, and are present when one run covered the pair, however
   many other pairs that run covered. A summary without `pairs`, written by an engine from before
   this, is still read the old way: status from `failures`, counts only when the run covered the pair
   alone.
4. **Additive within protocol v1** (spec §11.2): a new member of an open document.

## Alternatives considered

- **One entry per (consumer, provider), the documents between them added up.** It is what the name
  suggests and what `decide` wants. But it throws away the distinction the broker keeps: a consumer's
  `main` pact passing and its deployed pact failing would read as one pair that failed by some count,
  with nothing to say which. A reader who wants the sum can add entries up; one who wants the
  separate counts cannot take a sum apart.
- **Leave it, and have hosts verify one contract per run** (finding 10's B). A reporting limitation
  turned into folklore in every CI script.
- **Have hosts tally the event stream** (finding 10's C). It leaves the summary, the document a
  report is written from, unable to answer the question its readers ask.

## Consequences

- `janus check` reports counts per pair for multi-contract runs, which it could not before.
- The summary, the protocol's largest regular document, grows by one entry per source. A run over a
  handful of pacts adds a few hundred bytes.
- Anything else that wants per-document numbers (the differential harness, an SDK's report, a
  broker's own embedding) reads `pairs` and never re-derives them from `failures`.
- **Tripwire**: a run over so many sources that the summary's size matters (hundreds of pacts from
  one broker) reopens this, with a per-source event as the alternative.
