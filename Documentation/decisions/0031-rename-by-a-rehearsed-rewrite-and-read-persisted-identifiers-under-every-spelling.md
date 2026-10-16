# 0031 — Rename by a rehearsed rewrite, and read persisted identifiers under every spelling

- **Status**: accepted
- **Date**: 2026-10-16
- **Plan tasks**: staged plan S0.5; S0.9 and S2.7 (the name, and the rename), which this prepares
- **Evidence**: [`Documentation/codename.md`](../codename.md) (the inventory);
  `.github/scripts/rename.sh`; `.github/workflows/rename-rehearsal.yml`;
  `engine/kernel/src/persisted.rs`; [ADR 0011](0011-contracts-as-self-identifying-json-documents.md)
  and [ADR 0021](0021-a-component-artifact-is-its-own-type-and-a-pin-is-fetched-by-digest.md), whose
  identifiers carry the codename

## Context

The project is built under a codename, "Janus", until the community names it (§7.1), and that is
expected late: after stage 1, once people can run their own pipelines on it. By then the codename is
in every crate, binary, package, environment variable, release asset and specification, and in some
documents the engine has already written out into brokers and registries, which a rename cannot
reach.

S0.5 asks that the rename be made mechanical before it is needed, and proved, so that it is a reviewed
change and not a search. Two forces pull against each other. Most surfaces want the simplest possible
rename, because the more special cases it has, the more it can miss. A few identifiers must survive
it: a contract recorded under the old name has to stay readable after it.

## Decision

1. **The rename is a rewrite of everything, by one script.** `.github/scripts/rename.sh <name>`
   rewrites `janus`, `Janus` and `JANUS` in every text file in the repository, and moves every path
   that contains it. This includes the ADRs and the prototype's record, which cite paths that move;
   git keeps the original wording. There is no list of surfaces for the script to follow and fall out
   of date. The inventory, `Documentation/codename.md`, records what the rename *means*, for
   reviewers and for the change file. It is not something the rename depends on.
2. **Completeness is a check, not a hope.** After rewriting, the script fails if the codename appears
   in any file or path but three exemptions: `CHANGELOG.md`, which is what was released; the list of
   earlier spellings below; and the repository's own name, which is S0.7's decision. The script also
   refuses a new name that already appears in the repository, because completeness could not then be
   shown.
3. **A persisted identifier is read under every spelling it has had.** One the engine writes into a
   document that outlives it and reads back from that document is defined once, in
   `engine/kernel/src/persisted.rs`, as what it writes. Its readers accept that spelling and every
   earlier one, listed in `persisted/earlier.rs`, which the script fills before it rewrites and never
   rewrites itself. So the renamed engine writes the new spelling and reads both. An earlier spelling
   is dropped one major version after the rename that added it. The identifiers today are a contract's
   and a provider shape's `$format`, an OCI component's artifact and config types, the interface a
   component exports, the scripted-hook library's global, and an http hook's body format. The broker's
   `specification` value joins them when the code writes it.
4. **What is written and never read back is renamed outright**: reports, the sampling algorithm's
   name, schema `$id`s, conformance documents passed between an SDK and the gates of the same release.
   So is everything a user configures but the engine does not read back from a stored document:
   binaries, packages, environment variables, the cache directory, file-name conventions. The rename's
   change file is `major`, and says which.
5. **The new name is one lowercase ASCII word.** Anything else needs a different spelling on each
   surface, and is a different rename.
6. **The rename is rehearsed every week.** The `Rename rehearsal` workflow runs the script on a
   throwaway checkout, with a placeholder made from the run's number, and then requires everything to
   pass renamed: the workspace's clippy and tests, the third-party component, both SDK suites and their
   conformance, the thinness audit, the binding pipeline, the protocol client and the demo. It builds
   a component before renaming and loads it after, so the dual-spelling rule is tested against a real
   artifact, and the kernel's `tests/persisted.rs` reads documents under each earlier spelling, which
   only a rename provides. The workflow also runs on any pull request that changes the script, the
   persisted identifiers or the inventory.

## Alternatives considered

- **A codename constant that every surface reads.** No such constant exists for a crate name, a Java
  package, a binary or a schema `$id`, so it would cover the surfaces a rewrite handles anyway and
  none of the hard ones.
- **A list of surfaces the script follows.** It is right on the day it is written. A rewrite with a
  completeness check cannot miss a surface somebody forgot to list.
- **Leave the records as written.** Their links into moved paths would break the link check, or need
  a second, partial rewrite. History is in git.
- **Never rename persisted identifiers.** Every document the renamed project writes would carry a name
  it no longer has, forever.
- **Dual spelling for everything.** Environment variables, binaries and packages are each a dozen
  special cases. A user changes them once, with a changelog entry telling them to, and they are not
  in anybody's stored documents.

## Consequences

Easier: the rename, when it comes, is one command and a review, and the weekly rehearsal says whether
it would work today. A new surface needs nothing: the script finds it. A new persisted identifier
needs one constant in `persisted.rs`, and its readers calling `reads()`.

Harder: anything that *reads back* the codename from a stored document must go through `persisted.rs`,
or the rename would break the documents written before it. The rehearsal does not catch that on its
own: a reader comparing against a literal is renamed with the literal, and passes. Reviewing a new
reader is what catches it, which is why the inventory and CLAUDE.md say so.

Committed to: renaming every surface but the three exemptions; dual spelling for one major version.

Tripwires:

- **The name is not one lowercase word.** The inventory grows a column of spellings per surface, and
  the script learns them, before the rename.
- **A persisted identifier is found outside `persisted.rs`.** It moves there, with a test that reads
  it under an earlier spelling.
- **The repository moves (S0.7).** The repository's name stops being exempt in the same change.
