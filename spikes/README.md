# Spikes

Time-boxed experiments (plan Phase 1 and beyond). Rules:

- Each spike lives in its own directory named after its plan task, e.g. `1.1-idl-bakeoff/`.
- **Every spike directory must contain a `FINDINGS.md`** — the finding is the deliverable; the code is
  disposable and may rot.
- Spike code is not held to workspace conventions, is excluded from the Cargo workspace (each spike is
  its own project), and is never depended on from `engine/`, `cli/` or `sdks/`.
- ADRs reference findings by path; deleting spike code is fine, deleting findings is not.
