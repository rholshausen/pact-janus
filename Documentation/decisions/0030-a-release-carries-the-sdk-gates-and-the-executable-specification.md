# 0030 — A release carries the SDK gates and the executable specification they run against

- **Status**: accepted
- **Date**: 2026-10-16
- **Plan tasks**: staged plan S0.11; S2.9 (graduation), which this makes cheap
- **Evidence**: `.github/workflows/release.yml` (the `binaries`, `assemble` and `sdk-gates` jobs);
  `.github/scripts/janus-spec.sh`; `tools/sdk-gates`;
  [ADR 0027](0027-build-in-the-prototype-repository-and-graduate-sdks-into-their-existing-repositories.md)
  decision 4; [ADR 0028](0028-change-files-decide-a-releases-version-and-merging-their-pull-request-releases.md)
  decision 9, which this extends

## Context

An SDK here passes three gates in CI: the conformance suite (plan task 6.4, ADR 0017), the thinness
audit (task 6.5) and bindings that match their schemas (task 6.1). Each is a Rust tool under `tools/`,
run with `cargo run` from a checkout of this repository, reading files from that checkout. ADR 0027
graduates the SDKs into pact-js and pact-jvm at stage 2's exit, and decision 4 says the split is made
cheap now: the corpora and the conformance suite ship with every engine release, and the checkers ship
as binaries, so a graduated SDK runs the same gates from another repository against the engine version
it pins.

Three things stood in the way. Each tool found its inputs through the path of its own source, compiled
in, so even a built binary could only be run where it was built. The thinness manifest described both
SDKs from the repository root, outside either SDK. And ADR 0028 decision 9 lists what a release
carries, as bare files: an SDK's resolver is a download and a hash.

## Decision

1. **Every release carries `janus-sdk-gates-<triple>`** (`.exe` on Windows) for the five triples
   `janus-engine` is built for: one statically linked binary whose subcommands are the three gates —
   `conformance`, `thinness`, `bindings` — and `version`. Each subcommand is the `tools/` crate of the
   same name, whose command now lives in its library so that its own binary and this one run the same
   code. It is built, checked for static linking and run in the same matrix job as the engine.
2. **Every release carries `janus-spec.tar.gz`**: the executable specification an SDK's gates read —
   `corpora/`, `conformance/`, the behavioural specification, `sdks/bindings.json` and every schema set
   it names — **at the paths they have in this repository**. It is made by `git archive` from the
   commit being released (`.github/scripts/janus-spec.sh`), so the same commit makes the same bytes.
   It is an archive, against decision 9's bare files, because it is a tree read by tools, not a binary
   an SDK resolver runs; `tar` reads it on all three operating systems.
3. **A gate is told where the specification is, and never assumes it.** `--spec <dir>` names this
   repository's root or an unpacked `janus-spec.tar.gz`, which read the same because their paths are
   the same; it defaults to the working directory. An SDK's own test drivers read `JANUS_SPEC` with the
   same meaning, and fall back to the repository they are in.
4. **An SDK's thinness manifest lives at its root** (`sdks/<sdk>/thinness.json`), with paths relative
   to it, so the audit runs wherever the SDK's source is. With no manifest named, the tool measures
   every `sdks/*/thinness.json` under the working directory, side by side, or else the
   `thinness.json` in it.
5. **The claim is a job, run before anything is published.** `release.yml`'s `sdk-gates` job checks
   out one SDK directory and nothing else, takes the engine, the gates and the specification from the
   release's own files, and runs all three gates; `publish` needs it. On every rehearsal the files are
   the ones a release would publish.
6. **What ships is what a change file covers.** The gates' sources and the specification's files
   join the paths CI's `Change file` workflow counts as shipped.

Decision 9's list becomes: `janus-engine-<triple>`, `janus-<triple>` and `janus-sdk-gates-<triple>`
for the five triples; `janus-engine.wasm`; `janus-spec.tar.gz`; and `SHA256SUMS` — 17 files. Its
other terms, and every other decision of ADR 0028, stand.

## Alternatives considered

- **Three binaries, one per tool.** Fifteen files where five do, and three downloads in every SDK's CI
  for what is one step.
- **Subcommands of `janus`.** The CLI is the user's, and drives the engine through the protocol;
  SDK-maintenance tooling would be the second exception to that, after `component push|pull`, and the
  first a user never needs.
- **The archive's own layout** (`cases/`, `schemas/<set>/`, …). Every tool and driver would need a
  second set of paths, and a mistake in the mapping would show up only in another repository.
- **Ship the files individually.** Hundreds of release assets, each in `SHA256SUMS` and attested,
  for no reader that wants one at a time.
- **Each graduated SDK vendors the corpus.** The copy drifts from the engine it pins, which is the
  failure the conformance suite exists to prevent.

## Consequences

Easier: an SDK checked out on its own — and, after S2.9, an SDK in its own repository — runs its three
gates from one release with no Rust toolchain and no checkout of this one. The gates and the cases
it is held to are the pinned engine's, so moving a pin moves all three together.

Harder: the gates are now part of what a release promises. A change to their command lines, or to
the paths inside `janus-spec.tar.gz`, is a change SDKs see, and is sized in a change file like any
other. Every release is 17 files, and `assemble` refuses any other count.

Committed to: the two new asset names, beside decision 9's, are what SDK pipelines pin against;
changing one is a superseding ADR. The specification's paths are this repository's, so moving one of
those directories moves it inside the archive too.

Tripwires:

- **An SDK needs a gate the binary does not have** (a fourth check, a language whose bindings are made
  differently): it becomes a subcommand here first, so every SDK keeps getting its gates from one place.
- **A graduated SDK cannot use the paths** — its own repository already has a `conformance/` or a
  `corpora/`: unpacking into a directory of its own is the answer `--spec` already gives, and the
  layout stays.
- **The archive grows past what a CI step should download** (the corpora are well under a megabyte
  today): split it, and say which part each gate needs.
