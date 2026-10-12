# 0029 — Attest what a release built, and let an SDK verify the digest it pins

- **Status**: accepted
- **Date**: 2026-10-12
- **Plan tasks**: staged plan S0.4 (chooses the signing scheme), X.1 (the subprocess binary's
  provenance), S0.10 (the SDKs' resolvers), S0.7 (owners, which this must not wait for)
- **Evidence**: `.github/workflows/release.yml` (the `assemble` and `publish` jobs);
  [ADR 0023](0023-the-subprocess-is-the-primary-embedding-and-wasm-serves-offline-operations.md)
  decision 4 (an SDK resolves `janus-engine` at a version it pins);
  [ADR 0028](0028-change-files-decide-a-releases-version-and-merging-their-pull-request-releases.md)
  (what a release is and what it carries); [ADR 0021](0021-a-component-artifact-is-its-own-type-and-a-pin-is-fetched-by-digest.md)
  (the same rule for components: a pin is a digest)

## Context

ADR 0023 made a native binary every SDK's primary embedding, so every user of every SDK runs a
program this repository built. S0.4 says its releases are signed and that an SDK verifies what it
downloads; X.1 says how an SDK finds the binary: a pinned version, a verified checksum, never a bare
`PATH` lookup.

Two different questions are inside "signed". One is the SDK's, asked on every machine, in every SDK
language, with no tool installed: *are these the bytes I was built to run?* The other is a person's,
asked once, when a pin is written or an incident is investigated: *where did these bytes come from?*

Two constraints bound the answer. An SDK is thin, so whatever it verifies it verifies with its
language's standard library. And there are no owners yet (S0.7): a signing key needs someone to hold
it, rotate it and answer for it, and there is nobody to name.

## Decision

1. **Every release has a `SHA256SUMS`**, listing the SHA-256 of every other file in it, in the format
   `sha256sum --check` reads.
2. **Every file in a release, `SHA256SUMS` included, has a build provenance attestation**: a
   [GitHub artifact attestation](https://docs.github.com/en/actions/security-for-github-actions/using-artifact-attestations),
   which is a Sigstore signature over the file's digest and a SLSA provenance statement naming the
   repository, the workflow and the commit that built it. It is signed with the identity of the
   workflow run itself, proved to Sigstore by the run's OIDC token, and recorded in a public
   transparency log. **Nobody holds a key.** Only `release.yml`'s `publish` job can make one, and
   that job runs only for a release.
3. **An SDK pins the engine by version and by digest**: the SHA-256 of `janus-engine` for each target
   triple, written in the SDK's own source beside the version. To resolve the engine it downloads the
   file for its platform, hashes it, and refuses to run anything whose digest is not the pinned one.
   The trust root is the SDK package the user already installed: no network answer can change which
   bytes are acceptable. This is the rule ADR 0021 set for components, applied to the engine.
4. **An SDK does not verify the attestation.** That needs a Sigstore client, in every SDK language,
   inside the layer that is supposed to be a few hundred lines. The digest pin answers the SDK's
   question completely; the attestation answers the other one.
5. **The attestation is checked where a pin is written**, with `gh attestation verify <file> --repo
   <this repository>`: by the maintainer or tool that copies a release's digests into an SDK, and by
   anyone who wants to. The `publish` job does it to its own release, downloaded back, before it
   finishes, so a release that does not verify is known at once.
6. **The override stays and is loud.** `JANUS_ENGINE` names an executable and is used as given, with
   no digest to check it against; the SDK logs that it was used (X.1).
7. **No operating-system code signing yet.** The binaries are fetched by an SDK or by `curl`, not by
   a browser, so they carry no quarantine mark for Gatekeeper or SmartScreen to act on, and the ad-hoc
   signature Apple silicon requires is applied by the linker. Authenticode and notarisation need
   certificates, which need owners.

## Alternatives considered

- **cosign keyless signatures per file.** The same Sigstore trust, with a signature and a certificate
  file beside every artifact and a verifier nobody has installed. The attestation carries the
  provenance statement as well, and `gh` verifies it.
- **A minisign (Ed25519) key over `SHA256SUMS`.** An SDK could verify the signature with its standard
  library and no network. But a pin by digest is already stronger than a signature check, since it
  names the exact bytes, and the key needs the custodian that does not exist.
- **GPG.** The same custodian, and tooling nobody enjoys.
- **Checksums alone.** They say the download is the file the release holds, and nothing about how the
  release came to hold it: anyone who can edit a release can replace a file and its line in
  `SHA256SUMS` together.
- **Have the SDK verify provenance.** Decision 4.

## Consequences

Easier: there is no secret to leak, rotate or hand over; verification by an SDK is a hash, in any
language; and a release's origin is a public record.

Harder: writing a pin is five digests per version, per SDK, so it is done by a tool that reads
`SHA256SUMS` (S0.10), not by hand. And the attestation is only as trustworthy as write access to
this repository and its `release.yml`: a change to that file is a change to what "built here" means,
and is reviewed as one.

Committed to: the attested identity is **this repository and this workflow**. A rename or a transfer
under S0.7 changes the name a verifier gives. What becomes of the attestations made before it has to
be checked before the move, not assumed; the digest pins are unaffected either way.

Tripwires:

- **Verification is needed with no network and no `gh`** (an air-gapped build, a distribution's
  packaging): add a key-based signature over `SHA256SUMS`, once there are owners to hold the key.
- **The CLI is distributed to be installed by people** (Homebrew, winget, a download page): decision
  7 is revisited, with the certificates that needs.
- **An SDK's own packaging requires signatures** (Maven Central does): that is the SDK package's
  signature, made where the SDK is released (S2.9), and does not change what this repository signs.
