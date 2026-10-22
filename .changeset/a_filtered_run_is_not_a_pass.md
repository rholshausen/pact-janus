---
default: major
---

# A filtered run is reported as `filtered`, never `verified`, and is not published

`verification/verify`'s `options.variants` (or `options.variant`) narrows a run to some recorded variants.
Until now, such a run reported `status: verified` whenever nothing failed, even when the filter named an id
no document records and nothing was replayed at all. With `options.publish`, every pact it fetched was then
published to the broker as a success (ADR 0041).

- **A filter naming an id no document records is refused.** It fails with `variant-not-found` (category
  `session`). `details.variant` is the first unknown id and `details.recorded` lists the ids the documents
  do record. For an inline source, `verify` answers with the error. For a `broker` source, the run ends
  right after `verification/fetched` with `status: failed` and the error, and nothing runs or is
  published. An empty `variants` list is `malformed-frame`.
- **A run is filtered when its filter left out at least one recorded variant**, not merely when it was
  given one. A filter that admits everything (for example `base`, over v1–v4 pacts) leaves a whole run.
- **A filtered run that nothing failed has `status: filtered`.** Each `pairs` entry can now be `filtered`
  too, after `failed` and `incomplete`. A host that reads a run's status must treat `filtered` as "not
  verified, not failed". The summary's and `verification/started`'s `filtered` member keeps its place,
  with this meaning.
- **A pact the filter narrowed is not published.** Its `verification/published` event and `publication`
  entry have `outcome: skipped` and `reason: filtered`. A pact the filter did not narrow is published as
  before.
- **`subsumption/decide` raises `verification-filtered` per pair**, from the pair's `filtered` status. It
  no longer raises it for every pair of a run that narrowed only one of them.

`janus verify --variant` follows. An id no document records exits 2. A filtered run prints `FILTERED:` and
exits 0, since it did what it was asked. `pact_verifier_cli` passes a filter that matches nothing and
publishes a narrowed pact, and pact-jvm skips publishing it; Janus refuses the first and skips the second.
