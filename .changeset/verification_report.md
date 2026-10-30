---
default: minor
---

# The engine writes a verification run's report

A run that asks with `options.report` (ADR 0059) gets its report on its stream: a
`verification/report` event after each event worth a line, carrying `text` to print as it arrives,
and `text-segments` for a host that declared `segments`. It is the report `janus verify` printed,
laid out by the engine instead of the CLI: the start, each result with its mismatches and the hook
that failed it, warnings, broker fetches and publications, executed plans, and the summary's line.
`options.report.error-chain` says every layer of each error's chain.

Two changes to what `janus verify` prints: each result names its interaction's provider states under
it (`given …`), and a run stopped at its first failure no longer names `--exit-on-first-error`.

For an SDK maintainer: the behavioural specification's `verify` adds `report: {}` to the options
unless the caller gave `report`, and writes each report event's text to standard output, as pact-js
prints the verifier's report (`verification.verify.report-printed`, conformance case
`provider/report-printed`). An SDK no longer prints warnings itself: the report says them. The
bindings gain `Report` and the `verification/report` event kind.
