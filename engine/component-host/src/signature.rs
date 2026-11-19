//! Sigstore signatures on component artifacts (component-interfaces spec §10.3, ADR 0099).
//!
//! A signature is a Sigstore bundle attached to the component's manifest as an OCI referrer, which is
//! what `cosign sign` (v3) writes: a DSSE envelope over an in-toto statement whose subject is the
//! manifest digest, signed with a short-lived Fulcio certificate naming the signer's identity, and
//! logged in Rekor. GitHub's `actions/attest-build-provenance`, with `push-to-registry`, attaches the
//! same kind of bundle over SLSA provenance. [`oci::Oci::bundles`](crate::oci::Oci::bundles) finds
//! them; this module decides whether one is from the signer a declaration trusts.
//!
//! Verification is offline. The trusted root — Fulcio's certificate authorities, Rekor's and the CT
//! logs' keys, the timestamp authorities — is the public-good instance's, as the `sigstore-verify`
//! release this engine was built with carries it, or one the declaration names as a file. Nothing is
//! fetched from Sigstore itself: the bundle carries its inclusion proof and signed timestamps, so a
//! run reaches the registry and nothing else.
//!
//! The digest stays the integrity check. A signature says *who* published those bytes; the pin says
//! *which* bytes, and is checked first.

use pact_janus_kernel::component::{SignatureTrust, Signer};
use ring::digest;
use ring::signature::{self as ring_signature, UnparsedPublicKey, VerificationAlgorithm};
use serde_json::Value;
use sigstore_verify::crypto::{CryptoProvider, HashContext, SigningScheme};
use sigstore_verify::trust_root::{SIGSTORE_PRODUCTION_TRUSTED_ROOT, TrustedRoot};
use sigstore_verify::types::{Bundle, HashAlgorithm, Sha256Hash, SignatureContent};
use sigstore_verify::{VerificationPolicy, Verifier};
use std::fmt;
use std::sync::OnceLock;

/// What an OCI referrer carrying a Sigstore bundle says it is: its `artifactType`, and its layer's
/// media type. Every version's starts with this (`…bundle.v0.3+json`, `…bundle+json;version=0.2`).
pub const BUNDLE_MEDIA_TYPE_PREFIX: &str = "application/vnd.dev.sigstore.bundle";

/// The statements a bundle's in-toto payload may make about a component for it to count as a
/// signature (ADR 0099 decision 2): cosign's `sign`, and SLSA provenance, which GitHub's attestation
/// action writes. A statement about anything else — a vulnerability scan, a test result — is signed
/// by its signer too, and says nothing about whether they published the component.
pub const SIGNING_PREDICATES: &[&str] = &[
  "https://sigstore.dev/cosign/sign/v1",
  "https://slsa.dev/provenance/v1",
];

/// Who a signature must come from, and what it is verified against. From a declaration, both the
/// identity and the issuer are required; `janus component pull` may name neither, and then reports
/// whoever signed.
pub struct Trust {
  identity: Option<Identity>,
  issuer: Option<String>,
  /// What the trusted root is, as a message says it: the public-good instance, or a file.
  root: String,
  verifier: Verifier,
}

enum Identity {
  Exact(String),
  /// Anchored at both ends, so it matches the whole identity, as a pact's regex matches a whole value
  /// (staged plan S1.18): an unanchored pattern would let `acme/janus-csv` trust `evil/acme/janus-csv-fork`.
  Pattern(regex::Regex, String),
}

impl Identity {
  fn matches(&self, identity: &str) -> bool {
    match self {
      Identity::Exact(expected) => identity == expected,
      Identity::Pattern(pattern, _) => pattern.is_match(identity),
    }
  }
}

impl fmt::Display for Identity {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Identity::Exact(identity) => write!(f, "'{identity}'"),
      Identity::Pattern(_, pattern) => write!(f, "an identity matching '{pattern}'"),
    }
  }
}

impl Trust {
  /// A declaration's `signature` (ADR 0099 decision 3): an issuer, and exactly one of `identity` and
  /// `identity-pattern`.
  pub fn from_declared(declared: &SignatureTrust) -> Result<Trust, String> {
    match (&declared.identity, &declared.identity_pattern) {
      (Some(_), Some(_)) => {
        return Err("a signature names 'identity' or 'identity-pattern', not both".to_string());
      }
      (None, None) => {
        return Err(
          "a signature names the identity it trusts, as 'identity' or 'identity-pattern'".to_string(),
        );
      }
      _ => {}
    }
    if declared.issuer.is_empty() {
      return Err("a signature names the OIDC 'issuer' that vouched for its identity".to_string());
    }
    Trust::new(
      declared.identity.as_deref(),
      declared.identity_pattern.as_deref(),
      Some(&declared.issuer),
      declared.trusted_root.as_deref(),
    )
  }

  /// Any combination, for `janus component pull`: with no identity and no issuer, every signer whose
  /// bundle verifies is reported.
  pub fn new(
    identity: Option<&str>,
    identity_pattern: Option<&str>,
    issuer: Option<&str>,
    trusted_root: Option<&str>,
  ) -> Result<Trust, String> {
    install_provider();
    let identity = match (identity, identity_pattern) {
      (Some(_), Some(_)) => {
        return Err("name an identity or an identity pattern, not both".to_string());
      }
      (Some(identity), None) => Some(Identity::Exact(identity.to_string())),
      (None, Some(pattern)) => Some(Identity::Pattern(
        regex::Regex::new(&format!("^(?:{pattern})$"))
          .map_err(|err| format!("'{pattern}' is not a regular expression: {err}"))?,
        pattern.to_string(),
      )),
      (None, None) => None,
    };
    let (root, json) = match trusted_root {
      Some(path) => (
        format!("the trusted root '{path}'"),
        std::fs::read_to_string(path)
          .map_err(|err| format!("the trusted root '{path}' could not be read: {err}"))?,
      ),
      None => (
        "the Sigstore public-good instance's trusted root".to_string(),
        SIGSTORE_PRODUCTION_TRUSTED_ROOT.to_string(),
      ),
    };
    let verifier = TrustedRoot::from_json(&json)
      .map_err(|err| err.to_string())
      .and_then(|root| Verifier::new(&root).map_err(|err| err.to_string()))
      .map_err(|err| format!("{root} cannot be used: {err}"))?;
    Ok(Trust {
      identity,
      issuer: issuer.map(str::to_string),
      root,
      verifier,
    })
  }

  /// Whether this trust names a signer, rather than reporting whoever signed.
  pub fn names_a_signer(&self) -> bool {
    self.identity.is_some() || self.issuer.is_some()
  }

  /// Who this trust accepts, as a message says it.
  pub fn describe(&self) -> String {
    match (&self.identity, &self.issuer) {
      (Some(identity), Some(issuer)) => format!("{identity} vouched for by '{issuer}'"),
      (Some(identity), None) => identity.to_string(),
      (None, Some(issuer)) => format!("an identity vouched for by '{issuer}'"),
      (None, None) => "any signer".to_string(),
    }
  }

  /// One bundle, against the manifest digest it must be about. `Ok` is the signer it verified, when
  /// that is a signer this trust accepts; `Err` says why it is not, in a sentence.
  pub fn check(&self, digest: &str, bundle: &[u8]) -> Result<Signer, String> {
    let signer = self.verify(digest, bundle)?;
    let identity_ok = self
      .identity
      .as_ref()
      .is_none_or(|identity| identity.matches(&signer.identity));
    let issuer_ok = self.issuer.as_ref().is_none_or(|issuer| *issuer == signer.issuer);
    if identity_ok && issuer_ok {
      Ok(signer)
    } else {
      Err(format!(
        "signed by '{}', vouched for by '{}'",
        signer.identity, signer.issuer
      ))
    }
  }

  /// The cryptographic half: the bundle's certificate chains to the trusted root's Fulcio, its SCT and
  /// its log entry verify, its envelope's signature verifies, and its statement names `digest` as its
  /// subject and is one of [`SIGNING_PREDICATES`]. Whoever it names is the signer.
  fn verify(&self, digest: &str, bytes: &[u8]) -> Result<Signer, String> {
    let text = std::str::from_utf8(bytes).map_err(|_| "the bundle is not UTF-8 text".to_string())?;
    let bundle = Bundle::from_json(text).map_err(|err| format!("the bundle cannot be read: {err}"))?;
    let SignatureContent::DsseEnvelope(envelope) = &bundle.content else {
      return Err(
        "the bundle signs a message, not an in-toto statement about a manifest (cosign v3's `sign` writes one)"
          .to_string(),
      );
    };
    let statement: Value = serde_json::from_slice(envelope.payload.as_bytes())
      .map_err(|err| format!("the bundle's statement is not JSON: {err}"))?;
    let predicate = statement
      .get("predicateType")
      .and_then(Value::as_str)
      .unwrap_or_default();
    if !SIGNING_PREDICATES.contains(&predicate) {
      return Err(format!(
        "the bundle's statement is '{predicate}', which says nothing about who published the component"
      ));
    }
    let hex = digest.strip_prefix("sha256:").unwrap_or(digest);
    let artifact =
      Sha256Hash::from_hex(hex).map_err(|err| format!("'{digest}' is not a SHA-256 digest: {err}"))?;
    let result = self
      .verifier
      .verify(artifact, &bundle, &VerificationPolicy::any_identity())
      .map_err(|err| format!("the signature does not verify against {}: {err}", self.root))?;
    let identity = result
      .identity()
      .map(|identity| identity.as_str().to_string())
      .ok_or_else(|| "the signing certificate names no identity".to_string())?;
    let issuer = result
      .issuer()
      .map(str::to_string)
      .ok_or_else(|| "the signing certificate names no OIDC issuer".to_string())?;
    Ok(Signer { identity, issuer })
  }
}

/// `sigstore-crypto` asks a process-wide provider for its hashing and signature checks. Its own is
/// aws-lc, which this workspace does not build: the TLS stack is on `ring` (engine/http-client), so the
/// provider is `ring` too, and one C cryptography library ships in the binaries, not two.
fn install_provider() {
  static INSTALLED: OnceLock<()> = OnceLock::new();
  INSTALLED.get_or_init(|| {
    // Another caller in the process may have installed one first; that one is then used, which is
    // the library's rule, and is no less able to verify.
    let _ = sigstore_verify::crypto::install_default_provider(&RING);
  });
}

static RING: Ring = Ring;

#[derive(Debug)]
struct Ring;

struct RingHash(digest::Context);

impl HashContext for RingHash {
  fn update(&mut self, data: &[u8]) {
    self.0.update(data);
  }

  fn finalize(self: Box<Self>) -> Vec<u8> {
    self.0.finish().as_ref().to_vec()
  }
}

impl Ring {
  fn algorithm(scheme: SigningScheme) -> Option<&'static dyn VerificationAlgorithm> {
    Some(match scheme {
      SigningScheme::EcdsaP256Sha256 => &ring_signature::ECDSA_P256_SHA256_ASN1,
      SigningScheme::EcdsaP256Sha384 => &ring_signature::ECDSA_P256_SHA384_ASN1,
      SigningScheme::EcdsaP384Sha256 => &ring_signature::ECDSA_P384_SHA256_ASN1,
      SigningScheme::EcdsaP384Sha384 => &ring_signature::ECDSA_P384_SHA384_ASN1,
      SigningScheme::Ed25519 => &ring_signature::ED25519,
      SigningScheme::RsaPkcs1Sha256 => &ring_signature::RSA_PKCS1_2048_8192_SHA256,
      SigningScheme::RsaPkcs1Sha384 => &ring_signature::RSA_PKCS1_2048_8192_SHA384,
      SigningScheme::RsaPkcs1Sha512 => &ring_signature::RSA_PKCS1_2048_8192_SHA512,
      SigningScheme::RsaPssSha256 => &ring_signature::RSA_PSS_2048_8192_SHA256,
      SigningScheme::RsaPssSha384 => &ring_signature::RSA_PSS_2048_8192_SHA384,
      SigningScheme::RsaPssSha512 => &ring_signature::RSA_PSS_2048_8192_SHA512,
      _ => return None,
    })
  }
}

impl CryptoProvider for Ring {
  fn hash(&self, algorithm: HashAlgorithm) -> sigstore_verify::crypto::Result<Box<dyn HashContext>> {
    let algorithm = match algorithm {
      HashAlgorithm::Sha2256 => &digest::SHA256,
      HashAlgorithm::Sha2384 => &digest::SHA384,
      HashAlgorithm::Sha2512 => &digest::SHA512,
      other => {
        return Err(sigstore_verify::crypto::Error::UnsupportedAlgorithm(format!(
          "{other:?} is not one ring hashes"
        )));
      }
    };
    Ok(Box::new(RingHash(digest::Context::new(algorithm))))
  }

  /// `ring` takes the key in the form the provider is handed it: an uncompressed SEC1 point, a raw
  /// Ed25519 key, or a PKCS#1 `RSAPublicKey`.
  fn verify_raw(
    &self,
    scheme: SigningScheme,
    spki_pubkey_bytes: &[u8],
    data: &[u8],
    signature: &[u8],
  ) -> sigstore_verify::crypto::Result<()> {
    let algorithm = Ring::algorithm(scheme).ok_or_else(|| {
      sigstore_verify::crypto::Error::UnsupportedAlgorithm(format!("{scheme:?} is not one ring verifies"))
    })?;
    UnparsedPublicKey::new(algorithm, spki_pubkey_bytes)
      .verify(data, signature)
      .map_err(|_| sigstore_verify::crypto::Error::Verification("the signature does not verify".to_string()))
  }

  /// `ring` verifies a message, never a digest. A prehashed signature is a `hashedrekord` message
  /// signature over an artifact's bytes, and a component's is a DSSE statement about its manifest,
  /// whose signature is over the envelope: so this is never asked, and refuses if it is.
  fn verify_prehashed(
    &self,
    scheme: SigningScheme,
    _spki_pubkey_bytes: &[u8],
    _digest: &[u8],
    _signature: &[u8],
  ) -> sigstore_verify::crypto::Result<()> {
    Err(sigstore_verify::crypto::Error::UnsupportedAlgorithm(format!(
      "{scheme:?} over a digest: a component's signature is a DSSE statement, verified over its envelope"
    )))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn declared(identity: Option<&str>, pattern: Option<&str>, issuer: &str) -> SignatureTrust {
    SignatureTrust {
      identity: identity.map(str::to_string),
      identity_pattern: pattern.map(str::to_string),
      issuer: issuer.to_string(),
      trusted_root: None,
    }
  }

  #[test]
  fn a_declaration_names_one_identity_and_an_issuer() {
    let issuer = "https://token.actions.githubusercontent.com";
    assert!(Trust::from_declared(&declared(Some("a"), None, issuer)).is_ok());
    assert!(Trust::from_declared(&declared(None, Some("a.*"), issuer)).is_ok());
    let both = Trust::from_declared(&declared(Some("a"), Some("a.*"), issuer))
      .err()
      .unwrap();
    assert!(both.contains("not both"), "{both}");
    let neither = Trust::from_declared(&declared(None, None, issuer)).err().unwrap();
    assert!(neither.contains("'identity' or 'identity-pattern'"), "{neither}");
    let no_issuer = Trust::from_declared(&declared(Some("a"), None, ""))
      .err()
      .unwrap();
    assert!(no_issuer.contains("'issuer'"), "{no_issuer}");
    let bad = Trust::from_declared(&declared(None, Some("("), issuer))
      .err()
      .unwrap();
    assert!(bad.contains("not a regular expression"), "{bad}");
    let mut missing = declared(Some("a"), None, issuer);
    missing.trusted_root = Some("/no/such/trusted_root.json".to_string());
    let missing = Trust::from_declared(&missing).err().unwrap();
    assert!(missing.contains("could not be read"), "{missing}");
  }

  /// A pattern matches the whole identity, as a v1–v4 regex matches a whole value (S1.18).
  #[test]
  fn an_identity_pattern_matches_the_whole_identity() {
    let workflow = "https://github.com/acme/janus-csv/.github/workflows/release.yml@refs/tags/v1.2.0";
    let pattern = r"https://github\.com/acme/janus-csv/\.github/workflows/release\.yml@refs/tags/v.*";
    let trust = Trust::new(None, Some(pattern), None, None).unwrap();
    let Some(identity) = &trust.identity else {
      panic!("an identity")
    };
    assert!(identity.matches(workflow));
    assert!(!identity.matches(&format!("https://evil.example/?{workflow}")));
    let partial = Trust::new(None, Some("acme/janus-csv"), None, None).unwrap();
    assert!(!partial.identity.unwrap().matches(workflow));
  }

  #[test]
  fn the_public_good_trusted_root_loads_under_ring() {
    let trust = Trust::new(None, None, None, None).expect("the embedded root");
    assert!(!trust.names_a_signer());
    assert_eq!(trust.describe(), "any signer");
  }
}
