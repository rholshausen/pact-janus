---
engine: minor
---

# A component's declaration can name the signer it trusts, and its Sigstore signature is verified

Staged plan S5.1c, ADR 0099. An `oci` source takes a `signature`: an `issuer` and exactly one of
`identity`, compared exactly, or `identity-pattern`, a regular expression matching the whole identity,
and optionally a `trusted-root` file. The loader then finds the Sigstore bundles attached to the
component's manifest as OCI referrers, as `cosign sign` (v3) attaches them, through the referrers API or
the referrers tag schema. It verifies them offline against the Sigstore public-good instance's trusted
root, which the engine carries, or the declared one, and loads the component only when one is from that
signer. This happens after the digest is checked and before a byte is compiled. A declaration with no
`signature` loads exactly as before.

- A load with no trusted signature fails as `component-unavailable` with `signature-missing` (nothing
  attached) or `signature-untrusted` (nothing attached is from that signer, each named with who signed
  it or why it does not verify).
- A recorded component's `source` (a contract's `metadata.components`, a verification summary's
  `components`) has `signer: { identity, issuer }` when the loader verified one.
- `janus component pull` takes `--certificate-identity`, `--certificate-identity-regexp`,
  `--certificate-oidc-issuer` and `--trusted-root`. It reports who signed the component, and prints a
  declaration naming the signer when exactly one verifies. It exits 1 when a signer it names did not
  sign.
- A pinned component whose signature verified once loads again from the cache with no request, its
  bundle verified again.
- The loader resolves a signature's `trusted-root` against the configuration file, as it resolves a
  `file` source.
- `pact_janus_kernel::component::Source` has `signature` (`SignatureTrust`), `Fetched` and
  `RecordedSource` have `signer` (`Signer`), and `WasmLoader::pull` takes a
  `pact_janus_component_host::signature::Trust` and returns the signers it verified.
