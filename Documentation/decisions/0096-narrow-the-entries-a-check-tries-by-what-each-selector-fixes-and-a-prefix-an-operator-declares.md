# 0096 — Narrow the entries a check tries by what each selector fixes, and a prefix an operator declares

- **Status**: accepted
- **Date**: 2026-11-18
- **Plan tasks**: S4.11
- **Evidence**: [ADR 0095](0095-recompute-a-missing-report-for-the-pair-a-broker-is-asked-about-in-its-wasm-engine-and-never-a-whole-fleet-at-once.md),
  `benchmarks/janus/src/fleet.rs` (`subsumption-pair-50`, `subsumption-pair-50-wide-shape`),
  [ADR 0025](0025-a-provider-shape-entry-may-select-interactions-by-operation.md),
  [ADR 0086](0086-record-a-provider-shape-from-the-providers-own-tests-by-the-operation-each-test-names.md)
  decision 2

## Context

A provider-shape entry is matched to a consumer interaction by its selector when their descriptions
differ (ADR 0025, subsumption-check spec §2.2). The kernel knows nothing of HTTP, so it decides
whether a selector admits an interaction by running the selector's plan against the values the
interaction recorded. That is the one thing it already does for every interaction. It costs a few
microseconds a run, and an interaction ran every entry's plan. S4.5 found that a check therefore
costs interactions × entries (ADR 0095): over an 80-entry shape selection was 60% of the walk, and
five times the entries made a 50-interaction check 3.4 times slower.

Most entries could be ruled out with a comparison. A selector's `method` is an `equality`, and so is
the status a recorder writes into it (ADR 0086 decision 4). Its `path` is the HTTP component's
`http:path` (ADR 0086 decision 2), whose template the kernel cannot read. But every path a template
admits begins with the template's text before its first parameter, and only the component knows that.

## Decision

1. **Each entry's selector fixes constraints, compiled with its plan once per check.** An `equality`
   whose example is a string, a number or a boolean fixes that value at its slot, or at a member
   under it through `object` shapes, as a message's metadata names its queue. A component operator
   fixes a prefix if its component declares one. No other shape fixes anything.
2. **A constraint passes over an entry only where its plan would certainly refuse.** A recorded value
   of a different kind, or a member that is absent, is left to the plan. Numbers are compared as
   numbers, since equality holds `100` and `100.0` to be one value (ADR 0072). So the entries that
   select an interaction are the ones that did before, and so are the reports. The plans still decide.
3. **An interaction's recorded values are decoded once per check.** That is once per slot and
   variant, however many entries read them. Before, every entry decoded them again.
4. **A matcher component may declare a prefix, through an optional `matcher/prefix`.** Given one of
   its operators and a shape document that uses it, the component answers the text every value the
   document admits begins with, or none (component-interfaces spec §7.6). A component that does not
   implement it has no prefix, which costs only time. A prefix an admitted value lacks would change
   which entry is selected, so a component must not answer one. The native binding answers it in
   code. Forwarding it to an out-of-tree component goes with `matcher/compare` in S5.7, since
   `janus check` loads no declared component until then. The HTTP component answers it for
   `http:path`: the template's text before its first parameter.
5. **A provider shape carried in a frame is read once.** `subsumption/check` read the shape by writing
   it out as text and parsing it twice. It now reads the document the frame already holds. That makes
   a check against a large shape a fifth cheaper, and only an error's line and column change, which
   pointed into text no one wrote.
6. **S4.11's exit measures each interaction's share of a check.** The plan's exit asked that a check
   against five times the entries cost within 1.5 times as much. Reading a document five times the
   size costs five times as much, which no index removes. `benchmarks/janus` now records the check
   of the same documents with no interactions (`read_median_us`), and each interaction's share of the
   rest (`per_interaction_us`). The exit holds the latter to 1.5 times.

## Alternatives considered

- **An index keyed by literal values.** With the constraints a comparison each, a linear pass over
  400 entries costs microseconds, which an index would save at the price of a structure to keep right.
  Revisit at thousands of entries.
- **The kernel reading `http:path`'s template.** It is HTTP knowledge, which the kernel does not
  have (AGENTS.md's first architecture rule).
- **Narrowing by `equality` alone.** Method and status split this fleet's entries in two at most,
  since every operation is a `GET` answering 200 or 404. The path is what tells operations apart.
- **A faster interpreter for selectors.** Every run still costs something, and interactions × entries
  of them is still the shape of the cost.

## Consequences

Measured natively on the fleet ADR 0095 measured, against the kernel before and after:

| 50 interactions | Before | After |
|---|---|---|
| each interaction, against 80 entries | 427 µs | 203 µs |
| each interaction, against 400 entries | 1,358 µs | 269 µs |
| reading the documents, 80 and 400 entries | 4.4 ms, 23.4 ms | 3.5 ms, 18.2 ms |

In WASM each interaction costs 268 µs against 80 entries and 337 µs against 400, 1.26 times. A
500-interaction pair checks in half the time, and a fleet's 500 pairs in 7.1 s natively and 8.9 s in
WASM on one thread, where they took 12.2 s and 15.9 s.

- A component author who wants their operator to narrow selection implements `matcher/prefix`. One
  who does not loses nothing but speed.
- Reading the documents now costs more than selection against a large shape. A check of a contract
  against a 400-entry shape spends half its time reading them. That cost is proportional to their
  size, and a broker pays it once per pair.
- *Tripwire:* revisit if a provider's selectors fix nothing a comparison can read (no `equality`, no
  operator with a prefix), or if shapes reach thousands of entries.
