# 0004 — Fork the pact-reference v2 matching engine as the kernel starting point

- **Status**: accepted
- **Date**: 2026-08-23
- **Plan tasks**: 0.4, 1.8 (G1)
- **Evidence**: [reuse inventory](../reuse-inventory.md) (task 0.4 — this ADR ratifies its
  recommendation)

## Context

Bet B2 says the plan compiler/interpreter can express both v1–v4 matching-rule semantics and the
new shape semantics. The RFC's "already prototyped" evidence for that bet is the v2 matching
engine in pact-reference (`pact_matching/src/engine`): ~13.5k lines including tests — node
grammar and builders, interpreter, value resolvers, body plan builders, pretty/`--executed`
rendering. The kernel must start somewhere, and the choice (adopt as dependency, fork, or
rewrite) shapes every Phase 3 task. Janus needs structural changes to that code's spine: plan
construction driven by compiled *shapes* (2.2) with variant dimensions (2.3) rather than v1–v4
matching rules; the node grammar promoted to a public, versioned surface (2.4); body builders
moved behind content components (2.6, bet B3).

## Decision

Fork `pact_matching/src/engine` into `engine/kernel` as the kernel starting point, and take
`pact_models` as a crates.io dependency (the v1–v4 door; Janus's v5 model is added beside it in
3.1). Specifically:

- Bring the `engine/tests` directory across with the fork; grow corpus coverage (3.7) **before**
  restructuring the spine.
- Record provenance in the forking commit — source repo, commit SHA, per `CLAUDE.md` — and
  preserve the MIT notice inside the Apache-2.0 repo.
- Bring along or reimplement the crate-internal helpers the engine leans on (e.g.
  `headers::parse_charset_parameters`); the `PACT_MATCHING_ENGINE=v2` toggle and the v1 engine do
  not come across.
- Keep terminal rendering (`ansi_term`) out of the kernel proper (feature-gated or moved to the
  CLI) to protect the `wasm32-wasip2` build and ADR-0003's zero-import discipline.
- The v1 matching code and the 803 spec test-case files stay in pact-reference as the
  **behavioural oracle and data** for verdict-diffing (3.5); they are consumed, not forked into
  the kernel.

## Alternatives considered

- **Depend on `pact_matching`**: killed because the required changes are structural, not
  additive — shapes replacing matching rules as the compiler input, and content handling moving
  behind component interfaces, rewrite the crate's core; a dependency would also drag the v1
  engine and its toggle along.
- **Rewrite from scratch**: killed because it discards the only working prototype of B2 and the
  head start the RFC's feasibility argument rests on; the verdict-diffing plan (3.5) also wants
  maximal behavioural continuity with what it is diffing against.
- **Fork the engine but not depend on `pact_models`**: killed — the engine imports it
  pervasively, so this is a disguised rewrite of both at once.

## Consequences

Easier: Phase 3 starts from a grammar and interpreter that already pass the spec suite via the v2
toggle; v1–v4 pact reading is a solved dependency; the 3.5 oracle diff has a shared model on both
sides.

Harder: the fork point freezes — upstream fixes to the v2 engine after the fork must be
cherry-picked consciously (provenance per commit); the kernel inherits code written for matching
rules and carries that shape until 2.2/2.4 restructure it; corpus coverage (3.7) becomes a
prerequisite gate for any restructuring work.

Tripwire: if compiling shapes (2.2) to the forked node grammar proves fundamentally awkward —
grammar changes so deep the fork stops paying for itself — supersede this ADR with a
partial-rewrite decision scoped to the compiler front-end, keeping the interpreter.
