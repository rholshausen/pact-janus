# 0001 — Record decisions as ADRs

- **Status**: accepted
- **Date**: 2026-08-12
- **Plan tasks**: 0.2

## Context

The prototype exists to produce evidence for the Pact MkII RFC. Its decisions — and the reasons and
evidence behind them — are therefore deliverables in their own right, and will be read by the Pact
community when the RFC is revised. They need a durable, reviewable home that outlives chat logs, PR
threads and spike code.

## Decision

All gate decisions and contested design choices are recorded as ADRs in `Documentation/decisions/`,
following `template.md` and the process in `README.md`. The README carries the index and a backlog of
known pending decisions. Accepted ADRs are only changed by superseding them.

## Alternatives considered

- Decisions in the project plan itself: the plan is a sequencing document; burying rationale there makes
  both harder to read and edit.
- GitHub issues/discussions only: fine for debate, poor as a permanent, versioned record; the repo
  should be self-contained for RFC reviewers.

## Consequences

Small writing overhead per decision; in exchange, Phase 9's unresolved-questions resolution (task 9.2)
becomes largely a compilation exercise. Tripwire: if ADRs start restating whole designs, they are being
misused — designs live in specs, ADRs record choices.
