# 0097 — Record each declared component a session or run loaded, by the digest a declaration would pin

- **Status**: accepted
- **Date**: 2026-11-19
- **Plan tasks**: S5.1a
- **Evidence**: [Phase 9 finding 15](../prototype/phase-9-findings.md#15-the-engine-records-no-component-it-resolved--not-its-version-not-its-digest),
  component-interfaces spec §10.3, contract-file spec §2.4 and §3.2,
  [ADR 0021](0021-a-component-artifact-is-its-own-type-and-a-pin-is-fetched-by-digest.md),
  [ADR 0065](0065-let-the-engines-host-write-and-merge-contract-files-in-identity-order.md)

## Context

Component-interfaces spec §10.3 says the engine records the name and version of every component that
took part in a run, and that a contract's `metadata.writer` map is where that goes on the consumer side.
Nothing has ever done it (finding 15). A consumer session writes its contract with no `metadata` at
all, and a verification summary says nothing about components. The scope that resolves a session's
components had a `components()` for this, which nothing called.

Since OCI distribution (ADR 0021), the value most worth recording is the digest. It is the one thing
that says *which bytes* decoded a body, and it reached only an `info` log line. Signing (S5.1c) will add
the identity that signed those bytes, which needs a record to go in.

The finding gave three options: `component:<name>` entries in `writer` (A), a `components` member
beside it (B), and carrying where a loader found the bytes through to the record (C). C is needed by
either of the others.

## Decision

1. **A contract's `metadata.components` and a verification summary's `components` list each declared
   component the session or run loaded.** Each entry has the `name` and `version` its handshake gave,
   and its `source`: the declaration's `kind`, an `oci` source's `reference`, and the `digest`. Entries
   are in name order. The member is absent when nothing was declared, so a contract written with no
   components has the bytes it had before.
2. **Loaded, not called.** A session's components are loaded before its first interaction, so what it
   loaded follows from its configuration. Which component decoded a body depends on routing, and a
   component that decoded nothing in one test may have contributed an operator or a fragment.
   Recording what was loaded is deterministic, and a component that was loaded and did nothing is
   still worth knowing about. The embedding's own in-tree components are not listed: they are the
   engine's, and change only with it.
3. **The digest is the one a declaration would pin.** For an `oci` source it is the manifest digest
   the loader resolved, a tag's included. For a `file` source it is the SHA-256 of the file's bytes,
   computed if the declaration pins none. So an entry can be pasted into a declaration as its pin. A
   `file` source's path is not recorded: it is absolute by the time the engine has it (design 2.7
   §7.1), and a developer's checkout and CI's would write different contracts. An `oci` reference is
   recorded without its digest, which the entry already gives, and without the credential, which is
   never part of a reference (ADR 0052).
4. **The loader says where the bytes came from.** `Loaded` gains `fetched`: an `oci` source's
   reference and digest, or a file's digest. A loader that cannot say leaves it out, and the entry
   then has a `source` with its `kind` alone. The kernel builds the entry from the declaration and
   the handshake, so every loader's entries are the same shape.
5. **The engine's own version is not recorded.** Every release would change every contract's bytes.
   A broker that hashes the whole file would see a new contract version and trigger every provider's
   verification, which contract-file spec §3.2 calls a design error. And `corpora/contracts/` pins
   contract bytes across releases. A verification result a broker receives already carries the
   verifier's version. `writer` stays in the schema, open, for a host that wants to write it.
6. **A merged contract lists what each writer loaded.** When the engine's host merges a test file's
   contract into one already written (ADR 0065), the two lists are joined, identical entries once.
   Two entries with one name and different digests both stay, since the file holds interactions
   recorded with each. The list is sorted by name, then version, then its source's canonical bytes, so a merged
   file's bytes do not depend on which test file came first.
7. **The record is metadata.** It changes nothing a verifier does: a verifier loads the components its
   own configuration declares, and an interaction's `requires` says what it needs (contract-file spec
   §3.2's placement rule).

## Alternatives considered

- **`writer` entries, `"component:csv": "1.0.0 sha256:…"`** (option A). Rejected because a string that
  packs a version and a digest is a format inside a format, which every reader would parse. The S5.1c
  signer identity would have to fit in it too.
- **Only the components that were called.** Rejected in decision 2. It makes the record depend on which
  bodies a test happened to send, and every content call would have to report back to the scope.
- **The `file` path, made relative to the project.** Rejected because the engine does not know the
  project's root, and a path says nothing the digest does not.
- **Writing the engine's version too**, as pact-js writes `pactJs.version`. Rejected in decision 5.

## Consequences

- A contract written with a declared component says which bytes took part, so a broker or a reviewer
  can tell two contracts recorded with different builds of a component apart.
- The summary `--output json` writes carries the same list, so a provider's CI can see what decoded
  its bodies.
- A component's build that is not reproducible changes the contract's bytes on every build. That is
  true, since different bytes took part, and an author pinning a digest sees it first.
- S5.1c adds the verified signer to `source`, which is open.
- Tripwire: if brokers start treating metadata as part of a contract's identity and a component's new
  build churns contract versions, move the record to the broker's publish request instead.
