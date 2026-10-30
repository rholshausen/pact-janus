# 0059 — The engine writes a verification run's report, and every client prints it

- **Status**: accepted
- **Date**: 2026-10-30
- **Plan tasks**: none; found verifying PactFlow's `example-provider` through `pact-janus/verifier`,
  where a passing run printed nothing and pact-js prints the verifier's report.
  [ADR 0054](0054-carry-an-errors-causes-as-a-chain-and-mark-display-text-with-segments.md), whose segments
  the report is marked with and whose error chain it says; supersedes in part
  [ADR 0058](0058-a-provider-state-no-hook-sets-up-is-a-warning-or-a-failure-by-choice.md) decision 3,
  whose SDKs printed each warning themselves
- **Evidence**: pact-js's provider test output, the Rust verifier's report printed through its FFI;
  `janus verify`'s report, laid out by the CLI from the run's events (`cli/src/verify.rs` before this
  change); both SDKs, which printed nothing for a run that passed

## Context

A pact-js provider test prints what it verified: each interaction, its states, and whether its
response matched. The Rust verifier writes that report, and pact-js prints it. A Janus SDK printed
nothing on a pass, and on a failure only the failed interactions, in its own words.

`janus verify` has a report, but the CLI laid it out itself from the run's events. Giving each SDK the
same report the same way means the layout in the CLI, in the TypeScript SDK, in the JVM SDK and in
every SDK after them: presentation drifting apart in each language, and code the thinness audit exists
to keep out of an SDK.

## Decision

1. **A run asks for its report, and the engine writes it.** `verification/verify`'s
   `options.report` (an object) asks; its `error-chain` member says every layer of each error's
   chain rather than what failed and its root cause (ADR 0054). The run's stream then carries a
   `verification/report` event after each event worth a line: the start, each result, warnings,
   fetches, publications, a hook's failure, executed plans, and the summary's line before the
   terminal event. Each carries `text`, a block of whole lines a host prints as it arrives followed by
   a line break, and `text-segments` for a host that declared `segments`.
2. **The layout is `janus verify`'s, moved into the engine** (`engine/kernel/src/protocol/report.rs`),
   with two changes. A result says each of its interaction's states under it (`given …`), as today's
   verifiers do, which tells two interactions with one description apart. The note for a run stopped
   at its first failure no longer names the CLI's flag, since an SDK set it with an option.
3. **`janus verify` prints the engine's report**, coloured from its segments. It lays out nothing.
4. **Every SDK asks for the report and prints it.** `verify` adds `report: {}` to the options unless
   the caller gave `report`, and writes each report event's text to standard output as it arrives.
   Warnings are in the report, so an SDK no longer prints them itself; they are still collected on
   the result (behavioural spec `verify`, `verification.verify.report-printed`).

## Alternatives considered

- **Each SDK lays out the report from the results it collects.** No protocol change, but one layout
  per language, each free to drift from the CLI's.
- **A `text` member on each existing event.** A host would still decide which events to print and
  where a failed hook's line goes; a report event says it all, in order.

## Consequences

Easier: a passing provider test says what it verified, in the same words in the CLI and every SDK, and
a new SDK prints a report by printing what it is sent.

Harder: the report's wording is the engine's, so changing it changes every client's output at once,
and the CLI's snapshots pin it. A host that wants no report does not ask; an SDK asks unless the
caller gives its own `report`.
