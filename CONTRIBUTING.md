# Contributing to Pact Janus

Janus is the build of the [Pact MkII RFC](https://github.com/pact-foundation/roadmap/pull/146), in the
repository that prototyped it. Start with:

- `Documentation/staged-implementation-plan.md` — the live plan. Work items reference its item ids
  (`S0.1`, `S1.4`, …), and each has an exit that says when it is done.
- `Documentation/specs/` — the specifications. Normative: a behaviour change is made there first.
- `Documentation/decisions/` — ADRs. Accepted decisions are superseded, not re-argued.
- `Documentation/prototype/` — the prototype's record: the plan it was built to (bare task numbers
  like `1.6` or `G1` are that plan's) and the report each task left. Kept as written, not maintained.

## Practicalities

- Rust workspace at the repo root: `cargo build`, `cargo test`, `cargo clippy --all-targets -- -D warnings`,
  `cargo fmt --all -- --check` must all pass (CI enforces them). Style is 2-space indentation via
  `rustfmt.toml`, matching pact-reference.
- Engine kernel crates must also build for `wasm32-wasip2` (CI enforces for the kernel).
- CI builds and tests on Linux, macOS and Windows. If you are on Linux with the
  `x86_64-pc-windows-gnu` target, MinGW and wine, most Windows failures show up before you push:
  `CARGO_TARGET_X86_64_PC_WINDOWS_GNU_RUNNER=wine cargo test --target x86_64-pc-windows-gnu --no-fail-fast`.
  Tests that spawn `cargo` or a shell fail there and are not evidence of anything.
- Matching-behaviour changes must update `corpora/` in the same commit, and a change to what an SDK's
  DSL does belongs in the behavioural specification and `conformance/` first.
- Every relative link in the repository's Markdown must resolve: `cargo run -p pact_janus_doc_links`
  (CI enforces it). Moving or renaming a document means fixing what cites it in the same commit.
- `spikes/` holds the findings of the prototype's experiments and no code. The ADRs cite those
  findings by path, so they are not moved or edited; see `spikes/README.md`.
- Commit messages follow Conventional Changelog style: `feat:`, `fix:`, `chore:`, `docs:`,
  `refactor:`, `test:`.

## Change files and releases

A release's version and its changelog come from **change files**, not from commit subjects
([ADR 0028](Documentation/decisions/0028-change-files-decide-a-releases-version-and-merging-their-pull-request-releases.md)).
If your pull request changes what a release ships — the engine's or the CLI's sources or manifests,
`Cargo.lock`, `corpora/` or `conformance/` — add one. [knope](https://knope.tech) writes
it for you (`knope document-change`), or write `.changeset/<any_name>.md` by hand:

```markdown
---
default: minor
---

# `janus check` reads provider shapes from a directory

`--provider-shape` takes a directory as well as a file, and reads every shape in it.
```

- The size is what the change costs someone who pins the engine. `major`: they must change something
  to move to it (a protocol frame, a document they hold, a CLI flag or exit code, a verdict that was
  `matched`). `minor`: they can do something they could not. `patch`: something wrong is now right.
  Until 1.0, `major` moves the minor version and the other two move the patch.
- The body is the changelog entry. Write it for an SDK maintainer deciding whether to move their pin:
  what changed for them, not which plan item it was. A `#` heading is optional; without one, a short
  body becomes a single bullet.
- If nobody who pins the engine could notice the change (a refactor, a comment), say so instead, with
  a line in the pull request's description: `No-Changeset: <why>`.

CI checks all of this (the `Change file` workflow), and `knope prepare-release --dry-run` shows what
the change files gathered so far add up to.

**Releasing** is two steps, for anyone with write access, and needs no secret:

1. Run the **Prepare release** workflow (give it a version only when the version is a decision, as
   `0.1.0` and `1.0.0` are). It pushes a `release` branch with the new version, `CHANGELOG.md` and
   the consumed change files. Open the pull request it links to. CI runs on it, and so does a full
   rehearsal of the release build.
  ```
  gh workflow run prepare-release.yml
  # to set the version if required
  gh workflow run prepare-release.yml -f version=0.1.0
  ```
2. Merge it. The **Release** workflow builds `janus-engine`, `janus` and `janus-sdk-gates` for five
   target triples, the engine as a WASM component and `janus-spec.tar.gz` (ADR 0030), runs each SDK's
   gates from those files alone, attests every file, and then creates the `v<version>` tag and the
   GitHub release together. Do not push the tag yourself.
3. Open the pull request that moves the SDKs' pin. After publishing, the same workflow writes
   `sdks/engine-release.json` for the new release (`engine-pin`, below), runs both SDK suites against
   the release on every operating system with no Rust toolchain reachable, and pushes the result as
   the `engine-pin` branch. Its summary links to the pull request. To write a pin by hand, with `gh`
   signed in:
  ```
  cargo run -p pact_janus_engine_pin -- 0.1.0
  ```
   It refuses to write anything unless `gh attestation verify` passes on the release's `SHA256SUMS`.

What a release's signature is, and how to check a download, is
[ADR 0029](Documentation/decisions/0029-attest-what-a-release-built-and-let-an-sdk-verify-the-digest-it-pins.md):
`sha256sum --check SHA256SUMS`, and `gh attestation verify <file> --repo rholshausen/pact-janus`.

Conventions for AI-assisted work are in `AGENTS.md`, which Claude Code (through `CLAUDE.md`), GitHub
Copilot and other agents read. Humans may find it a useful summary too.
