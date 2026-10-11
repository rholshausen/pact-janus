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
- Matching-behaviour changes must update `corpora/` in the same commit, and a change to what an SDK's
  DSL does belongs in the behavioural specification and `conformance/` first.
- Every relative link in the repository's Markdown must resolve: `cargo run -p pact_janus_doc_links`
  (CI enforces it). Moving or renaming a document means fixing what cites it in the same commit.
- `spikes/` holds the findings of the prototype's experiments and no code. The ADRs cite those
  findings by path, so they are not moved or edited; see `spikes/README.md`.
- Commit messages follow Conventional Changelog style: `feat:`, `fix:`, `chore:`, `docs:`,
  `refactor:`, `test:`.

Conventions for AI-assisted work are in `CLAUDE.md` / `.github/copilot-instructions.md` — humans may
find them a useful summary too.
