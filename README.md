# Pact Janus: published pages

This branch is written by CI. Do not edit it by hand.

`bench/` is the benchmark trend (staged plan item S0.3): the 1.7 scenarios run against Janus in each
embedding, one point per merge to `main`, recorded by the `Benchmarks` workflow on that branch
(`.github/workflows/benchmarks.yml`). The chart is at
<https://rholshausen.github.io/pact-janus/bench/>, and `bench/data.js` is the data behind it.

It is a trend, not a gate: the numbers come from shared runners and move from run to run. What
they mean, and what each scenario measures, is in `benchmarks/README.md` on `main`.
