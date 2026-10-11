# 0027 — Build in the prototype's repository, and graduate SDKs into their existing repositories

- **Status**: accepted
- **Date**: 2026-10-11
- **Plan tasks**: staged plan S0.6 (decides it), S0.10–S0.12 (make the split cheap), S2.9 (the
  graduation), S6.1
- **Evidence**: `.github/workflows/ci.yml` (what one commit has to touch today), `Cargo.toml` (the
  excluded workspaces), [ADR 0004](0004-fork-v2-engine-as-kernel.md) (provenance of the forked engine),
  [ADR 0023](0023-the-subprocess-is-the-primary-embedding-and-wasm-serves-offline-operations.md)
  (the SDKs' primary embedding is a released binary), the staged plan's stage 2 exit (SDKs ship MkII
  as a new major version of today's packages)

## Context

The staged plan is now the live plan, and before its first item lands the build needs a home. Three
layouts were weighed: keep this mono-repo and build on it; start a new mono-repo; or one repository per
concern (engine and specs, each SDK, the conformance suite), which is the Pact organisation's shape.

What decides it is how much has to move in one commit, and when. Today a specification change is one
commit that touches the schemas, the corpora, the engine, both SDKs' generated bindings and the
conformance suite, and CI fails on any of them drifting. That atomicity is what principle 3 of the plan
("the specification grows with the code") rests on. The coupling is highest in stages 0–2, when the
protocol gains operations, the contract format changes and the plan grammar is renamed, and it drops
after stage 3 freezes the DSL and the format.

The SDKs are a special case. Stage 2's deliverable is that SDKs ship MkII as a new major version of
the packages people already use, with a compatibility facade for today's DSL. The JavaScript idiomatic
layer therefore ends up in pact-js and the JVM one in pact-jvm, as their next major, not in new
repositories. Whatever the layout, the SDKs leave this repository at stage 2's exit.

## Decision

1. **The real build proceeds in this repository**, on `main`, hardening the prototype in place. The
   history stays: it is the provenance ADR 0004 requires for the forked engine, and the prototype's
   findings are cited by the ADRs. The repository is renamed when the name is decided (S0.5's rehearsal
   covers the rename; GitHub redirects the old name).
2. **The engine, the specifications, the CLI, the corpora, the conformance suite and the checkers stay
   together, permanently.** They are one artifact, the executable specification, and the same-commit
   rule needs them atomic. If the community's answer to "where it lives" is pact-reference, that is a
   subtree move of these directories, which a mono-repo makes no harder.
3. **`sdks/` is an incubator.** The TypeScript and JVM SDKs are built here through stage 2 and
   **graduate into pact-js and pact-jvm** at stage 2's exit (S2.9), as those packages' next major
   version. Stage 6's SDKs graduate the same way, into their language's existing repository where one
   exists and into a repository of their own where none does. After graduation an SDK's drivers for the
   conformance suite live with the SDK, and this repository keeps no SDK source.
4. **The split is made cheap in stage 0, not at the moment of graduation:**
   - an SDK's test setup resolves the engine as a pinned, released `janus-engine` from S0.4's pipeline,
     with building it from source as the developer fallback it already has, so an SDK contributor
     needs no Rust toolchain (S0.10);
   - the corpora and the conformance suite ship as a versioned artifact with every engine release, and
     the checkers an SDK's CI runs (conformance, thinness, bindings) ship as binaries, so a graduated SDK
     runs the same gates from another repository (S0.11);
   - the prototype is pruned: spike code is deleted and each `FINDINGS.md` kept, and the phase reports
     and project plan move under `Documentation/prototype/`, so a newcomer reads the real build and
     not its scaffolding (S0.12).
5. **The demo** stays here as the end-to-end regression test, and after S2.9 consumes the graduated
   SDKs as published packages, which is what a user does.

## Alternatives considered

- **A new mono-repo.** It is this repository with a rename and a pruning pass, both of which can be
  done in place, and it costs the history unless it is carried over anyway.
- **Separate repositories from day one.** The right end state for the SDKs, and the wrong time: during
  stages 1–2 every specification change would become a release train (schema, bindings package,
  corpus artifact, a pin bump in each SDK, then conformance) for a change one person is making at once.
  Decision 4 builds the pieces that train needs, so the move happens when the churn has stopped.
- **Keep the SDKs here permanently.** Stage 2's own exit contradicts it: a new major of pact-js is
  released from pact-js. Keeping a parallel SDK here would be a second implementation to drift from
  the first, which is the thing the RFC set out to end.

## Consequences

Easier: S0.1 starts today, with every cross-cutting check already enforced. The rename, the
AI-assisted regeneration across SDKs and the same-commit rule all work in one place through the stages
that need them most.

Harder: one CI pipeline carries Rust, Node, the JDK, WASM, a registry and the demo until S2.9, and one
tag namespace serves artifacts that will want their own cadences. S0.10 and S0.11 are new work whose
only purpose is a split that happens two stages later; skipping them would make S2.9 a rewrite of
every SDK's test setup under deadline.

Committed to: the executable specification as one repository; SDKs as incubated here and owned
elsewhere; the released binary, not a source build, as the SDK's contract with the engine.

**Tripwire.** Revisit if a second maintainer group wants to own an SDK before stage 2's exit — then
S2.9 happens early for that SDK, which decision 4 should have made cheap. Revisit too if pact-js or
pact-jvm's maintainers decline the graduation: an SDK then gets a repository of its own under the
Pact organisation, and the compatibility facade becomes a migration guide rather than a shared package.
