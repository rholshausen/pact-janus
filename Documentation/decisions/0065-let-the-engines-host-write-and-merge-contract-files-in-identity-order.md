# 0065 — Let the engine's host write and merge contract files, in identity order

- **Status**: accepted
- **Date**: 2026-11-02
- **Plan tasks**: S2.11; ADR 0063 decision 7
- **Evidence**: pact-reference's `write_pact` (`pact_models/src/pact.rs`) and its file lock
  (`file_utils.rs`): one exclusive lock from the read of the existing file to the write, with a
  conflicting v1–v3 interaction refused and a v4 one silently replaced by the newer; pact-jvm's
  `DefaultPactWriter` and `PactMerge`: the same lock, a conflict refused, `pact.writer.overwrite` to
  opt out; `sdks/typescript/src/janus.ts` and `sdks/jvm/sdk`'s `Janus.finalise`, which overwrite;
  contract-file spec §4.2, which already says what identifies an interaction, and §2.4, which writes
  `interactions` as submitted

## Context

Jest, Vitest and Gradle run test files in separate workers or processes. pact-js and pact-jvm merge
every file's interactions into one pact file, so a consumer with several test files gets one pact.
A Janus SDK writes its session's contract and overwrites, so the last file to finish wins and the
others' interactions are lost without an error. S2.5's example suites have several test files, so
the facade cannot pass without merging.

Merging decides which interactions are the same and what a conflict is. That is logic about
contracts, which an SDK must not own, and every SDK would otherwise implement it again. It also needs
a file lock that holds across processes. So the engine has to do it, but the kernel reads no file: a
filesystem is the embedding's to give, as the configuration loader is (ADR 0055) and the broker client
is (ADR 0032).

Two facts about the format bound the answer:

- **Identity is already decided.** Contract-file spec §4.2: an interaction is its `description`
  together with its `states`, and a contract holds no two with the same pair.
- **Order is not.** §2.4 writes `interactions` as submitted. A merged file's order would then depend on
  which worker finished first, so the same suite run twice could write two different files, and §2.4's
  own rule, that the same content is the same bytes, would break. A broker deduplicates on those bytes.

## Decision

1. **`finalise` takes a `write`, and the engine's host writes the file.**
   `consumer-session/finalise` takes `write: { path, mode? }`. `path` is absolute, as `config/load`'s
   is (a relative one is `malformed-frame`), and its directory is created. When the session produces a
   contract, the engine writes it there and answers `written: <path>` in place of `contract` and
   `contract-text`. When it withholds one, it writes nothing and leaves the file as it was, so a failed
   test file never removes another file's interactions. Without `write`, `finalise` answers as before.
2. **Behind a capability, `contract-writing`.** The engine declares it when its embedding registered
   a writer: the native embeddings do, and the offline WASM artifact does not. An engine without one
   answers a `write` with `component-unavailable` naming `contract-writer`. The host's part is only the
   file: it opens, locks, reads, writes and unlocks. Reading, merging and the bytes are the kernel's.
3. **Merging is by identity.** `mode` is `merge` by default. The file's interactions and the session's
   are joined by §4.2's identity. Two with the same identity whose canonical bytes are the same are
   one. Two with the same identity and any difference is a conflict, and so is a file that names
   another consumer or provider. A conflict is `contract-conflict`, a `document` error carrying the
   `path` and each conflicting interaction's `description` and `states`; the file is left as it was.
   A file that is empty, or absent, is written as new. A file that is not a contract fails as reading
   one does (`contract-invalid`, `contract-version-unsupported`), with the `path`.
4. **Overwriting stays an option.** `mode: "overwrite"` writes the session's contract whatever the file
   held, as pact-jvm's `pact.writer.overwrite` does. Each SDK offers it as an option.
5. **Interactions are written in identity order, in every contract.** §2.4 changes: `interactions`
   are sorted by `description`, in code-point order, then by the canonical bytes of `states`, a
   missing `states` first. Every writer sorts: a session's contract, a merge, and `upgrade/pact`. So a
   merge's bytes do not depend on which writer came first, and two test files that merge into one file
   write the bytes one session recording both would have.
6. **The lock is an exclusive lock on the file**, held from reading what it holds until the new bytes
   are written, and waited for as long as it takes, as pact-reference's is. The file is rewritten in
   place: replacing it with another file would move the lock off the file the next writer is waiting on.
7. **The SDKs write no file.** Both send `write` with the absolute path they used to write themselves,
   `<contract directory>/<consumer>-<provider>.janus.json`, and `merge` unless their overwrite option is
   set, and return the path the engine answered. A result with `contract-text` and no `written` is from
   an engine that predates this ADR, and `finalise` fails, saying so, as for ADR 0063.

## Alternatives considered

- **The SDK merges.** It would parse a contract, which ADR 0063 stopped it doing, and two SDKs would
  implement identity and conflicts twice.
- **A `contract/write` operation after `finalise`.** The SDK would carry the contract's text from one
  call to the next for no gain, and a host could write a withheld session's file by mistake.
- **The newest record wins a conflict**, as pact-reference does for a v4 pact. Two test files recording
  one interaction differently is a mistake in a suite, and silently keeping one hides it.
- **Keep submitted order, and append what a merge adds.** The file's bytes would depend on which worker
  finished first.
- **Write to a temporary file and rename it over the contract.** The rename is atomic, but a writer
  waiting on the old file's lock would then read a file no one writes any more.
- **A lock file beside the contract.** It is one more file to leave behind when a process is killed,
  and an OS lock is released when its process ends.

## Consequences

Easier: several test files record one contract, in every SDK at once, with no SDK code beyond a path.
Two SDKs write the same file because neither writes it. A suite that records an interaction two ways
fails, naming it, instead of keeping whichever ran last.

Harder: a contract with several interactions changes its bytes once, when its writer moves to this
engine, since its interactions are sorted. A broker sees one new version. `janus explain --index`
counts in the sorted order.

What merges is whatever the file held, including what an earlier run left. An interaction a later run
changed is a conflict, and one it removed stays in the file. A contract directory is a run's output and
is cleared by the build before a run, as pact-js and pact-jvm users clear their pact directory today.

Committed to: `finalise`'s `write` and `written`, the `contract-writing` capability, the
`contract-conflict` code, and identity order in §2.4.

Tripwires:

- **A run identity reaches the engine**, such as a test runner's run id. Then a file from an earlier run
  could be replaced rather than merged into, and the stale-file consequence above goes away.
- **A host that cannot lock**, such as a network filesystem that ignores locks. Merging there loses
  writes as overwriting did, and the host should say so rather than claim the capability.
