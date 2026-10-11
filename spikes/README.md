# Spikes

The prototype's time-boxed experiments (project plan Phase 1 and beyond). Each lived in a directory
named after its plan task, and each had one deliverable: a `FINDINGS.md`. The code was disposable.

**The code is gone.** Staged plan item S0.12 deleted it when the real build started, because none of it
is a starting point for anything and its findings are already in the ADRs. What is left, in place:

- each spike's `FINDINGS.md`, at the path the [decision records](../Documentation/decisions/README.md)
  and the specifications cite it by. These are not moved;
- each spike's `README.md` — the question it asked and how it went about answering it — and 1.1's
  `evolution-gauntlet.md`, the change catalog its findings score against. Both describe files that
  are no longer here.

The last commit with the code in it is `b8d5010`. To read a spike's code, or run it again:

```sh
git ls-tree -r --name-only b8d5010 -- spikes/1.1-idl-bakeoff     # what was there
git show b8d5010:spikes/8.3-subprocess-transport/src/lib.rs      # one file
git worktree add ../janus-spikes b8d5010                         # all of it, checked out beside this repo
```

| Spike | Question | Findings |
|---|---|---|
| 1.1 | Which IDL defines the protocol and the component interfaces, judged by compatibility under evolution | [FINDINGS](1.1-idl-bakeoff/FINDINGS.md) |
| 1.2 | Whether the WASM component model works from each SDK's host language, and what it costs | [FINDINGS](1.2-wasm-embedding/FINDINGS.md) |
| 1.3 | Whether the engine as a subprocess, driven over stdio, works and dies cleanly in each SDK's language | [FINDINGS](1.3-subprocess-embedding/FINDINGS.md) |
| 1.4 | Whether a native engine can host WASM components as plugins: sandboxed, contained, and fast enough | [FINDINGS](1.4-engine-hosting-plugins/FINDINGS.md) |
| 1.5 | Whether the transport interface fits an asynchronous message transport | [FINDINGS](1.5-message-transport-shape/FINDINGS.md) |
| 1.6 | Which script runtime runs hooks in every engine embedding | [FINDINGS](1.6-script-hook-bakeoff/FINDINGS.md) |
| 7.3 | Whether provider shapes derived from types overstate, and whether provenance should select policy | [FINDINGS](7.3-type-derived-shapes/FINDINGS.md) |
| 8.3 | Whether a transport component can run out of process, and what its grants are worth there | [FINDINGS](8.3-subprocess-transport/FINDINGS.md) |

CI checks that this directory holds Markdown only, which is S0.12's exit ("no code under `spikes/`").
