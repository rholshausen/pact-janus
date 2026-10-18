---
default: patch
---

# The verifier fixture corpus gains broker fixtures

`corpora/verifier/` in `janus-spec.tar.gz` now holds fixtures verified from a Pact Broker. Their
pacts are published to a real broker first, and both verifiers fetch them from it and publish their
results back. They cover the sample provider's own pact, pending and not-pending pacts, a
work-in-progress pact, a pact the consumer version selectors do not fetch, and the
pact-compatibility-suite's four broker scenarios, which were skipped until now. A fixture's `broker`
member says how its pacts are published, what was verified before the run, and the options the
verifiers get. `disagreements.json` gains keys `<pact>/published` for what a run published, and a
verdict `absent`.

Running them takes the Pact Broker pinned in `tools/verifier-diff/pact-broker.json`, which is not
in the archive: anything reading the corpus from the archive alone should skip a fixture with a
`broker` member. Nothing an SDK runs today reads the corpus.

What the ledger says about this release: `janus verify` cannot fetch from a broker or publish
results yet (S1.3). Every broker fixture is a disagreement until it can.
