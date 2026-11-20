---
engine: major
---

# A check loads the components a project declares, and compares their operators as their handshakes declare

Staged plan S5.7, ADR 0102. A third party's comparable operator is decided by a subsumption check,
where it was always a review.

- **`subsumption/check` and `broker/check-contracts` take `components`**, loaded as
  `verification/verify`'s `target.components` are. The walk runs a declared operator's own action
  against a finite provider and asks its `matcher/compare` for anything wider. A component that cannot
  be loaded is `component-unavailable`.
- **`janus check --config <file>`** reads a verifier configuration's `components`, as `janus verify` and
  `janus explain` do.
- **A declared component's comparability is read from its handshake** by the engine, whatever binding
  loaded it, and the WASM binding forwards `matcher/compare` and `matcher/prefix`.
- **Breaking for a component that declares an operator `exact` or `conservative` without contributing an
  action of the operator's name, or without implementing `matcher`**: it no longer loads, as
  `component-invalid`, since nothing could decide the operator.
