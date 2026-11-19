# 0099 — Verify a component's Sigstore signature, found as an OCI referrer, against the signer its declaration names

- **Status**: accepted
- **Date**: 2026-11-19
- **Plan tasks**: S5.1c
- **Evidence**: the staged plan's cross-cutting X.6, component-interfaces spec §10.3,
  `cosign` v3.1.4's `sign` (`cmd/cosign/cli/sign/sign.go`, `pkg/oci/remote/write.go`),
  `sigstore-verify` 0.16.0, `engine/component-host/tests/signature.rs`,
  [ADR 0021](0021-a-component-artifact-is-its-own-type-and-a-pin-is-fetched-by-digest.md),
  [ADR 0097](0097-record-each-declared-component-a-session-or-run-loaded-by-the-digest-a-declaration-would-pin.md),
  [ADR 0098](0098-accept-a-component-published-in-the-cncf-wasm-layout-recognised-by-the-pipe-its-config-says-it-exports.md)

## Context

A declaration pins a component by digest (ADR 0021), so a run loads the bytes its author chose. Nothing
says who published those bytes. The first time anyone pins a component, they pin whatever the tag
resolved to that day, and `janus component pull` prints that digest for them to paste. Spec §10.3 said
signature verification was not specified in v1, and that it would sit on top of digest pinning rather
than replace it. S5.1c is that layer.

The plan named the scheme: Sigstore signatures attached to an artifact through OCI referrers. The
ecosystem has settled on it. `cosign` v3 signs an image by writing a DSSE envelope over an in-toto
statement whose subject is the manifest digest, with predicate type `https://sigstore.dev/cosign/sign/v1`.
It signs that with a short-lived Fulcio certificate naming the signer's OIDC identity, logs it in Rekor,
and attaches the resulting Sigstore bundle to the manifest as an OCI 1.1 referrer. The referrer's
`artifactType` and its one layer are `application/vnd.dev.sigstore.bundle.v0.3+json`. A registry without
the referrers API, such as `registry:2`, gets the referrers tag schema's index (`sha256-<hex>`) instead.
GitHub's `actions/attest-build-provenance` with `push-to-registry` attaches the same kind of bundle over
SLSA provenance. In CI, a GitHub workflow is signed with keylessly: its certificate's identity is
`https://github.com/<owner>/<repo>/.github/workflows/<file>@<ref>`, vouched for by
`https://token.actions.githubusercontent.com`.

The plan left three questions to this ADR: the verifier, where the trust root comes from offline, and
what an unsigned component does by default.

## Decision

1. **A signature is a Sigstore bundle attached to the manifest as an OCI referrer.** The loader lists
   the manifest's referrers through the referrers API (`GET /v2/<name>/referrers/<digest>`). From a
   registry that answers 404, 400 or 405 there, it reads the referrers tag schema's index instead. A
   referrer counts when its `artifactType` begins `application/vnd.dev.sigstore.bundle`, and it is used
   only when its manifest's `subject` is the component's manifest digest: a tag is anyone's to write.
   Each referrer manifest and bundle is fetched by digest and checked as every blob is (spec §10.3
   step 2). At most 16 such referrers are considered, and a bundle is at most 1 MiB.
2. **What counts as a signature is a statement that the signer published those bytes.** The bundle is
   a DSSE envelope over an in-toto statement. Its subject is the manifest digest, and its predicate is
   `cosign sign`'s (`https://sigstore.dev/cosign/sign/v1`) or SLSA provenance
   (`https://slsa.dev/provenance/v1`). A statement with another predicate is signed by its signer too,
   but a vulnerability scan says nothing about who published the component. A bundle that signs a
   message (a `hashedrekord`) is not accepted, since `cosign` v3 does not write one for an image. The
   bundle is verified in full, with nothing skipped: the certificate chains to the trusted root's
   Fulcio and carries an SCT from one of its CT logs, the log entry's inclusion proof, checkpoint and
   signed entry timestamp verify against its Rekor keys, the times agree, and the envelope's signature
   verifies.
3. **A declaration names the signer it trusts.** An `oci` source takes a `signature`:
   `{ identity | identity-pattern, issuer, trusted-root? }`. `issuer` is required, with exactly one of
   `identity` (the certificate's identity, exactly) and `identity-pattern` (a regular expression, which
   must match the whole identity, as a v1–v4 regex matches a whole value since S1.18). An unanchored
   pattern would let `acme/janus-csv` trust `evil/acme/janus-csv-fork`. A member the loader does not
   know is refused, not ignored: a trust constraint that is ignored is not enforced. The schema stays
   open; this is the loader's refusal, like a credential `kind` it cannot present. A `file` source
   takes no `signature`, since there is nowhere beside a file to find one and its digest pins it. A
   declaration the loader cannot use fails the load before anything is fetched.
4. **Verified at load and at pull, after the digest and before a byte is compiled.** At load, the
   bundles the cache remembers for the digest are tried first, and the registry is asked only when none
   of them is from the trusted signer: a signature added since, or a declaration that now trusts someone
   else. A pinned component whose signature verified once still loads with no request (spec §10.3
   step 3). The cache decides nothing: every bundle is verified again on every load. `janus component
   pull` takes cosign's spellings: `--certificate-identity`, `--certificate-identity-regexp`,
   `--certificate-oidc-issuer` and `--trusted-root`. With a signer named, it is held to it. With none,
   it reports every signer whose bundle verifies against the trusted root. When there is exactly one,
   the declaration it prints names it, as it prints the digest to pin.
5. **A load with no trusted signature fails as the subject.** `signature-missing` when no signature is
   attached, and `signature-untrusted` when something is attached and none of it verifies as from the
   trusted signer. The message names who each one is from or why it does not verify. Both carry
   `source: engine` inside the `component-unavailable` of spec §11.3, and `pull` exits 1 with them, as
   with `digest-mismatch`.
6. **The verifier is the Sigstore project's own Rust verifier, `sigstore-verify`, with `ring` as its
   cryptography.** It is synchronous and offline, and verifies bundles v0.1 to v0.3, Rekor v1 and v2
   entries, and RFC 3161 timestamps. Its default provider is aws-lc, which this workspace does not build.
   The TLS stack is on `ring` (engine/http-client), so the loader installs a `ring` provider and the
   binaries ship one C cryptography library, not two. `ring` verifies a message and never a digest, so
   that provider refuses a prehashed check. Decision 2 never asks for one.
7. **The trust root is carried, not fetched.** Unless a declaration names one, it is the Sigstore
   public-good instance's trusted root as the `sigstore-verify` release this engine was built with
   carries it. A run reaches the registry and nothing else. There is no TUF client and no request to
   Sigstore at run time, so CI behind a firewall, or offline from a warm cache, verifies the same way.
   That root changes when the dependency does, which is a change file like any other. `trusted-root`
   names a trusted root file instead: a private Sigstore instance, GitHub's instance for private
   repositories, or a fresher copy of the public one. The host's loader resolves it against the
   configuration file, as it resolves a `file` source (lifecycle-hooks spec §7.1). An SDK passes it
   as written.
8. **Unsigned is the default.** A declaration with no `signature` loads as it did, unverified, and its
   digest is its integrity. Requiring signatures by default would refuse every component declared today
   and every `file` source. Trust is per component, because signers are, and a reviewer sees a
   declaration's `signature`, or its absence, beside its digest. There is no project-wide switch.
9. **The record carries the signer.** A recorded component's `source` (ADR 0097) gains
   `signer: { identity, issuer }`, the certificate's own identity and issuer, when the declaration named
   a signer and the loader verified one. A contract's `metadata.components` and a verification summary's
   `components` both carry it. A component loaded with no `signature` records none, whether or not one
   was attached, because nothing was verified.
10. **Janus does not sign.** `cosign sign` signs a component as it signs an image, and a release
    workflow's attestation step works too. `janus component push` is unchanged.

## Alternatives considered

- **`sigstore-rs` (the `sigstore` crate).** Rejected because it calls itself experimental, is async
  throughout, and brings an OCI client, `reqwest`, OpenSSL and a TUF client. `sigstore-verify`, from the
  Sigstore project's newer `sigstore-rust` repository, does one job and does it offline.
- **Notation (Notary Project) signatures.** Rejected for now because they rest on an X.509 PKI each
  publisher runs, where Sigstore's keyless signing needs nothing beyond the CI the publisher already
  has. The referrer machinery in decision 1 is the same, so this could be added beside Sigstore.
- **Fetching the trusted root through TUF on each run.** Rejected because it is a request to
  `tuf-repo-cdn.sigstore.dev` from every run, which fails offline and behind a firewall, and makes a run
  depend on the network for something a release can carry. The trusted root changes rarely, and a
  declaration can name a fresher one.
- **Accepting any in-toto statement about the digest.** Rejected in decision 2: a signed scan result or
  test report is not a statement of publication.
- **An unanchored `identity-pattern`, as some tools read one.** Rejected in decision 3.
- **aws-lc as the provider.** Rejected in decision 6. It is a second C library to build on five release
  targets, and every check this needs, `ring` already does.
- **Requiring a signature unless a declaration opts out.** Rejected in decision 8.

## Consequences

- A team can pin a component to the CI workflow that publishes it. A digest pinned later by someone who
  never checked where it came from still has to come from that workflow to load.
- `janus component pull` tells an author who signed what they are about to pin, verified against the
  trusted root.
- The `component` loader's failures gain two codes, and recorded sources gain `signer`, both additive.
- The engine carries a snapshot of the public-good trusted root. A key the instance adds after the
  snapshot (a new Rekor log, a new Fulcio certificate) fails verification of bundles that use it until
  the dependency moves. CI's `signed_keylessly_in_ci` signs with today's `cosign` against today's
  instance on every push to `main`, so it fails first.
- Tripwire: if components start shipping signatures that are not in-toto statements (a `hashedrekord`
  over the manifest, say), decision 2 widens. Accepting one needs a prehashed check, which means a
  provider other than `ring` for that path.
