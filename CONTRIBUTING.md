# Contributing to Pact Janus

Janus is a prototype of the [Pact MkII RFC](https://github.com/pact-foundation/roadmap/pull/146); its
findings are the deliverable as much as its code. Start with:

- `Documentation/charter.md` — what the prototype must prove, and what is out of scope.
- `Documentation/project-plan.md` — the phased plan; work items reference its task numbers.
- `Documentation/decisions/` — ADRs. Accepted decisions are superseded, not re-argued.

## Practicalities

- Rust workspace at the repo root: `cargo build`, `cargo test`, `cargo clippy --all-targets -- -D warnings`,
  `cargo fmt --all -- --check` must all pass (CI enforces them). Style is 2-space indentation via
  `rustfmt.toml`, matching pact-reference.
- Engine kernel crates must also build for `wasm32-wasip2` (CI enforces for the kernel).
- Spikes go under `spikes/` with a `FINDINGS.md`; see `spikes/README.md`.
- Matching-behaviour changes must update `corpora/` in the same commit (from Phase 3).
- Commit messages follow Conventional Changelog style: `feat:`, `fix:`, `chore:`, `docs:`,
  `refactor:`, `test:`.

Conventions for AI-assisted work are in `CLAUDE.md` / `.github/copilot-instructions.md` — humans may
find them a useful summary too.
