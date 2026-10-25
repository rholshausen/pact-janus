---
default: minor
---

# `verification/explain` compiles what a run compiles, and runs captured values

`verification/explain` now prints the plan a run executes, and can run it (ADR 0050).

- **`components`**: the project's declared components, resolved as `verification/verify`'s
  `target.components` are. A component's plan fragment is now in the plan `explain` prints. A contract's
  `content-types` reach the compiler, which they did not before.
- **An interaction a run would refuse is refused**: one whose requirement, declared content type or
  contributed fragment no loaded component satisfies now fails with `component-unavailable`, as
  `verify` fails it, where `explain` printed a generic plan.
- **`values`**, with optional `encoded`: captured values by path, as a golden corpus case gives them. The
  result then carries `executed`, with `status` (`matched` or `mismatched`), `mismatches` and the executed
  `text`. They are run with the run's content detection and the actions its components contribute.
- The subject's `variant` is now in the schema. The engine already read it.

`janus explain` gains `--config <file>`, which loads a verifier configuration's components, and
`--executed` now runs its values in the engine.
