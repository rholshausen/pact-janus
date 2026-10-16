---
default: minor
---

# A release carries the gates an SDK's CI runs, and the specification they run against

Two kinds of file join every release:

- `janus-sdk-gates-<triple>` (`.exe` on Windows) for the same five targets as `janus-engine`: one
  statically linked binary holding the conformance check (`janus-sdk-gates conformance check
  <report.json>`), the thinness audit (`janus-sdk-gates thinness check`, which reads the
  `thinness.json` at the SDK's root) and the binding generator (`janus-sdk-gates bindings generate
  --only <language> --sdk <dir>`).
- `janus-spec.tar.gz`: the conformance suite, the corpora, the behavioural specification, and the
  schemas the bindings are generated from, at the paths they have in the repository. Point the gates
  at it with `--spec <dir>`, and an SDK's conformance driver with `JANUS_SPEC`.

An SDK that pins this version can run its conformance, thinness and bindings gates from the release
alone, with no checkout of this repository and no Rust toolchain. Both are listed in `SHA256SUMS`
and attested like every other file.
