//! A Sigstore small enough to read (S5.1c's tests): a certificate authority playing Fulcio, a CT log
//! and a Rekor log, each a key made for the test, and the trusted root that names them. It signs the
//! way `cosign sign` (v3) does — a DSSE envelope over an in-toto statement whose subject is a manifest
//! digest, with a short-lived certificate naming an identity and an OIDC issuer, an embedded SCT, and
//! a Rekor `dsse` entry with its inclusion proof, checkpoint and signed entry timestamp — so the
//! loader verifies every bundle here exactly as it verifies one from the public-good instance, under
//! no relaxed policy, with the trusted root declared as the file this writes.
//!
//! The public-good instance is the other half: `tests/signature.rs` signs keylessly with `cosign`
//! itself when CI can (`JANUS_COSIGN`), because a Sigstore that agrees with the verifier by
//! construction proves only that they agree.

#![allow(dead_code)]

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use ring::rand::SystemRandom;
use ring::signature::{ECDSA_P256_SHA256_ASN1_SIGNING, EcdsaKeyPair, KeyPair};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::str::FromStr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use x509_cert::Certificate;
use x509_cert::der::asn1::{BitString, Ia5String, ObjectIdentifier, OctetString, UtcTime, Utf8StringRef};
use x509_cert::der::{Any, Encode};
use x509_cert::ext::pkix::name::GeneralName;
use x509_cert::ext::pkix::sct::{
  DigitallySigned, HashAlgorithm, LogId, SerializedSct, SignatureAlgorithm, SignatureAndHashAlgorithm,
  SignedCertificateTimestamp, SignedCertificateTimestampList, Version as SctVersion,
};
use x509_cert::ext::pkix::{BasicConstraints, ExtendedKeyUsage, KeyUsage, KeyUsages, SubjectAltName};
use x509_cert::ext::{AsExtension, Extension};
use x509_cert::name::Name;
use x509_cert::serial_number::SerialNumber;
use x509_cert::spki::{AlgorithmIdentifierOwned, SubjectPublicKeyInfoOwned};
use x509_cert::time::{Time, Validity};
use x509_cert::{TbsCertificate, Version};

const EC_PUBLIC_KEY: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.10045.2.1");
const PRIME256V1: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.10045.3.1.7");
const ECDSA_WITH_SHA256: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.10045.4.3.2");
const CODE_SIGNING: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.3.6.1.5.5.7.3.3");
/// Fulcio's OIDC issuer extension, a DER UTF8String.
const FULCIO_ISSUER: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.3.6.1.4.1.57264.1.8");
/// The in-toto payload type a DSSE envelope carries a statement as.
pub const IN_TOTO: &str = "application/vnd.in-toto+json";
/// `cosign sign`'s statement about an image.
pub const COSIGN_SIGN: &str = "https://sigstore.dev/cosign/sign/v1";
/// The OCI media type of a Sigstore bundle, as cosign v3 attaches one.
pub const BUNDLE_MEDIA_TYPE: &str = "application/vnd.dev.sigstore.bundle.v0.3+json";

/// A P-256 key, as each of the three services has.
pub struct Key {
  pair: EcdsaKeyPair,
}

impl Key {
  fn generate() -> Key {
    let rng = SystemRandom::new();
    let pkcs8 = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, &rng).unwrap();
    Key {
      pair: EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, pkcs8.as_ref(), &rng).unwrap(),
    }
  }

  fn sign(&self, message: &[u8]) -> Vec<u8> {
    self
      .pair
      .sign(&SystemRandom::new(), message)
      .unwrap()
      .as_ref()
      .to_vec()
  }

  fn spki(&self) -> SubjectPublicKeyInfoOwned {
    SubjectPublicKeyInfoOwned {
      algorithm: AlgorithmIdentifierOwned {
        oid: EC_PUBLIC_KEY,
        parameters: Some(Any::from(&PRIME256V1)),
      },
      subject_public_key: BitString::from_bytes(self.pair.public_key().as_ref()).unwrap(),
    }
  }

  fn spki_der(&self) -> Vec<u8> {
    self.spki().to_der().unwrap()
  }

  /// RFC 6962's log ID, and Rekor's: the SHA-256 of the key's SubjectPublicKeyInfo.
  fn log_id(&self) -> [u8; 32] {
    Sha256::digest(self.spki_der()).into()
  }
}

/// The three services and the trusted root naming them.
pub struct Sigstore {
  ca: Key,
  ca_name: Name,
  ca_der: Vec<u8>,
  ct: Key,
  rekor: Key,
}

/// What [`Sigstore::sign`] signs, beyond the digest: who, vouched for by whom, and what the statement
/// says about the digest.
pub struct Signing<'a> {
  pub identity: &'a str,
  pub issuer: &'a str,
  pub predicate: &'a str,
}

impl Default for Signing<'_> {
  fn default() -> Self {
    Signing {
      identity: "https://github.com/acme/janus-csv/.github/workflows/release.yml@refs/heads/main",
      issuer: "https://token.actions.githubusercontent.com",
      predicate: COSIGN_SIGN,
    }
  }
}

fn utc(at: SystemTime) -> Time {
  Time::UtcTime(UtcTime::from_unix_duration(at.duration_since(UNIX_EPOCH).unwrap()).unwrap())
}

fn extension(value: &impl AsExtension, subject: &Name) -> Extension {
  value.to_extension(subject, &[]).unwrap()
}

fn ecdsa_with_sha256() -> AlgorithmIdentifierOwned {
  AlgorithmIdentifierOwned {
    oid: ECDSA_WITH_SHA256,
    parameters: None,
  }
}

impl Sigstore {
  pub fn new() -> Sigstore {
    let ca = Key::generate();
    let ca_name = Name::from_str("CN=sigstore-intermediate,O=janus-test").unwrap();
    let now = SystemTime::now();
    let tbs = TbsCertificate {
      version: Version::V3,
      serial_number: SerialNumber::new(&[1]).unwrap(),
      signature: ecdsa_with_sha256(),
      issuer: ca_name.clone(),
      validity: Validity {
        not_before: utc(now - Duration::from_secs(86_400)),
        not_after: utc(now + Duration::from_secs(86_400)),
      },
      subject: ca_name.clone(),
      subject_public_key_info: ca.spki(),
      issuer_unique_id: None,
      subject_unique_id: None,
      extensions: Some(vec![
        extension(
          &BasicConstraints {
            ca: true,
            path_len_constraint: None,
          },
          &ca_name,
        ),
        extension(&KeyUsage(KeyUsages::KeyCertSign | KeyUsages::CRLSign), &ca_name),
      ]),
    };
    let ca_der = sign_certificate(&ca, tbs);
    Sigstore {
      ca,
      ca_name,
      ca_der,
      ct: Key::generate(),
      rekor: Key::generate(),
    }
  }

  /// The trusted root a declaration names as its `trusted-root`.
  pub fn trusted_root(&self) -> String {
    let start = "2020-01-01T00:00:00Z";
    let log = |key: &Key, url: &str| {
      json!({ "baseUrl": url, "hashAlgorithm": "SHA2_256",
              "publicKey": { "rawBytes": BASE64.encode(key.spki_der()), "keyDetails": "PKIX_ECDSA_P256_SHA_256",
                             "validFor": { "start": start } },
              "logId": { "keyId": BASE64.encode(key.log_id()) } })
    };
    serde_json::to_string_pretty(&json!({
      "mediaType": "application/vnd.dev.sigstore.trustedroot+json;version=0.1",
      "tlogs": [ log(&self.rekor, "https://rekor.janus.test") ],
      "certificateAuthorities": [ {
        "subject": { "organization": "janus-test", "commonName": "sigstore-intermediate" },
        "uri": "https://fulcio.janus.test",
        "certChain": { "certificates": [ { "rawBytes": BASE64.encode(&self.ca_der) } ] },
        "validFor": { "start": start } } ],
      "ctlogs": [ log(&self.ct, "https://ctfe.janus.test") ],
      "timestampAuthorities": [],
    }))
    .unwrap()
  }

  /// A bundle signing `digest` (`sha256:<hex>`, a manifest's), as `cosign sign` would with `signing`'s
  /// identity: its JSON bytes, which is what an OCI referrer's layer carries.
  pub fn sign(&self, digest: &str, signing: &Signing<'_>) -> Vec<u8> {
    let hex = digest.strip_prefix("sha256:").unwrap();
    let statement = serde_json::to_vec(&json!({
      "_type": "https://in-toto.io/Statement/v1",
      "subject": [ { "digest": { "sha256": hex } } ],
      "predicateType": signing.predicate,
      "predicate": {},
    }))
    .unwrap();
    self.sign_statement(&statement, signing)
  }

  /// As [`Sigstore::sign`], over a statement the test wrote.
  pub fn sign_statement(&self, statement: &[u8], signing: &Signing<'_>) -> Vec<u8> {
    let now = SystemTime::now();
    let leaf_key = Key::generate();
    let leaf = self.certificate(&leaf_key, signing, now);
    let pae = [
      format!("DSSEv1 {} {IN_TOTO} {} ", IN_TOTO.len(), statement.len()).into_bytes(),
      statement.to_vec(),
    ]
    .concat();
    let signature = leaf_key.sign(&pae);

    // Rekor's `dsse` v0.0.1 entry, as a log of size one: its leaf is the root.
    let pem = format!(
      "-----BEGIN CERTIFICATE-----\n{}\n-----END CERTIFICATE-----\n",
      BASE64.encode(&leaf)
    );
    let envelope = json!({ "payload": BASE64.encode(statement), "payloadType": IN_TOTO,
                           "signatures": [ { "sig": BASE64.encode(&signature), "keyid": "" } ] });
    let body = serde_json::to_vec(&json!({
      "apiVersion": "0.0.1", "kind": "dsse",
      "spec": {
        "envelopeHash": { "algorithm": "sha256", "value": hex(&Sha256::digest(envelope.to_string())) },
        "payloadHash": { "algorithm": "sha256", "value": hex(&Sha256::digest(statement)) },
        "signatures": [ { "signature": BASE64.encode(&signature), "verifier": BASE64.encode(&pem) } ],
      },
    }))
    .unwrap();
    let root: [u8; 32] = Sha256::digest([&[0u8][..], &body].concat()).into();
    let integrated = now.duration_since(UNIX_EPOCH).unwrap().as_secs() - 5;
    let log_id = self.rekor.log_id();
    let set = self.rekor.sign(
      format!(
        r#"{{"body":"{}","integratedTime":{integrated},"logID":"{}","logIndex":0}}"#,
        BASE64.encode(&body),
        hex(&log_id)
      )
      .as_bytes(),
    );
    let origin = "rekor.janus.test - 1";
    let note = format!("{origin}\n1\n{}\n", BASE64.encode(root));
    let note_signature = [&log_id[..4], &self.rekor.sign(note.as_bytes())].concat();
    // The signature line names the log by its host, as Rekor does, not by the whole origin.
    let checkpoint = format!(
      "{note}\n\u{2014} rekor.janus.test {}",
      BASE64.encode(note_signature)
    );

    serde_json::to_vec(&json!({
      "mediaType": BUNDLE_MEDIA_TYPE,
      "verificationMaterial": {
        "certificate": { "rawBytes": BASE64.encode(&leaf) },
        "tlogEntries": [ {
          "logIndex": "0",
          "logId": { "keyId": BASE64.encode(log_id) },
          "kindVersion": { "kind": "dsse", "version": "0.0.1" },
          "integratedTime": integrated.to_string(),
          "inclusionPromise": { "signedEntryTimestamp": BASE64.encode(set) },
          "inclusionProof": { "logIndex": "0", "rootHash": BASE64.encode(root), "treeSize": "1", "hashes": [],
                              "checkpoint": { "envelope": checkpoint } },
          "canonicalizedBody": BASE64.encode(&body),
        } ],
      },
      "dsseEnvelope": envelope,
    }))
    .unwrap()
  }

  /// A Fulcio-like certificate: ten minutes long, code signing, the identity as its one SAN, the
  /// issuer in Fulcio's extension, and an SCT from the CT log over the precertificate.
  fn certificate(&self, key: &Key, signing: &Signing<'_>, now: SystemTime) -> Vec<u8> {
    let subject = Name::default();
    let issuer_value = Utf8StringRef::new(signing.issuer).unwrap().to_der().unwrap();
    let san = SubjectAltName(vec![GeneralName::UniformResourceIdentifier(
      Ia5String::new(signing.identity).unwrap(),
    )]);
    let mut tbs = TbsCertificate {
      version: Version::V3,
      serial_number: SerialNumber::new(&[2, 1]).unwrap(),
      signature: ecdsa_with_sha256(),
      issuer: self.ca_name.clone(),
      validity: Validity {
        not_before: utc(now - Duration::from_secs(60)),
        not_after: utc(now + Duration::from_secs(540)),
      },
      subject: subject.clone(),
      subject_public_key_info: key.spki(),
      issuer_unique_id: None,
      subject_unique_id: None,
      extensions: Some(vec![
        extension(&KeyUsage(KeyUsages::DigitalSignature.into()), &subject),
        extension(&ExtendedKeyUsage(vec![CODE_SIGNING]), &subject),
        extension(&san, &subject),
        Extension {
          extn_id: FULCIO_ISSUER,
          critical: false,
          extn_value: OctetString::new(issuer_value).unwrap(),
        },
      ]),
    };

    // RFC 6962 §3.2: the SCT signs the precertificate, which is the TBS without the SCT extension.
    let timestamp = now.duration_since(UNIX_EPOCH).unwrap().as_millis() as u64 - 30_000;
    let precert = tbs.to_der().unwrap();
    let issuer_key_hash: [u8; 32] = Sha256::digest(self.ca.spki_der()).into();
    let mut signed = vec![0u8, 0u8];
    signed.extend(timestamp.to_be_bytes());
    signed.extend(1u16.to_be_bytes());
    signed.extend(issuer_key_hash);
    signed.extend(&(precert.len() as u32).to_be_bytes()[1..]);
    signed.extend(&precert);
    signed.extend(0u16.to_be_bytes());
    let sct = SignedCertificateTimestamp {
      version: SctVersion::V1,
      log_id: LogId {
        key_id: self.ct.log_id(),
      },
      timestamp,
      extensions: Vec::new().into(),
      signature: DigitallySigned {
        algorithm: SignatureAndHashAlgorithm {
          hash: HashAlgorithm::Sha256,
          signature: SignatureAlgorithm::Ecdsa,
        },
        signature: self.ct.sign(&signed).into(),
      },
    };
    let scts = SignedCertificateTimestampList::new(&[SerializedSct::new(sct).unwrap()]).unwrap();
    tbs.extensions.as_mut().unwrap().push(extension(&scts, &subject));
    sign_certificate(&self.ca, tbs)
  }
}

fn sign_certificate(issuer: &Key, tbs: TbsCertificate) -> Vec<u8> {
  let signature = issuer.sign(&tbs.to_der().unwrap());
  Certificate {
    tbs_certificate: tbs,
    signature_algorithm: ecdsa_with_sha256(),
    signature: BitString::from_bytes(&signature).unwrap(),
  }
  .to_der()
  .unwrap()
}

fn hex(bytes: &[u8]) -> String {
  bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// The referrer manifest `cosign sign` (v3) writes to attach `bundle` to `subject`: an OCI 1.1 image
/// manifest whose `artifactType` and one layer are the bundle's, with the empty config. Its bytes,
/// and the blobs it names.
pub fn referrer(subject: &str, subject_size: usize, bundle: &[u8]) -> (Vec<u8>, Vec<Vec<u8>>) {
  let empty = b"{}".to_vec();
  let digest = |bytes: &[u8]| format!("sha256:{}", hex(&Sha256::digest(bytes)));
  let manifest = serde_json::to_vec(&json!({
    "schemaVersion": 2,
    "mediaType": "application/vnd.oci.image.manifest.v1+json",
    "artifactType": BUNDLE_MEDIA_TYPE,
    "config": { "mediaType": "application/vnd.oci.empty.v1+json", "digest": digest(&empty), "size": empty.len() },
    "layers": [ { "mediaType": BUNDLE_MEDIA_TYPE, "digest": digest(bundle), "size": bundle.len(),
                  "annotations": { "dev.sigstore.bundle.content": "dsse-envelope",
                                   "dev.sigstore.bundle.predicateType": COSIGN_SIGN } } ],
    "subject": { "mediaType": "application/vnd.oci.image.manifest.v1+json", "digest": subject, "size": subject_size },
    "annotations": { "org.opencontainers.image.created": "2026-11-19T00:00:00Z" },
  }))
  .unwrap();
  (manifest, vec![empty, bundle.to_vec()])
}

/// A bundle with one byte of its signature changed: well formed, and signed by nobody.
pub fn tampered(bundle: &[u8]) -> Vec<u8> {
  let mut bundle: Value = serde_json::from_slice(bundle).unwrap();
  let sig = bundle["dsseEnvelope"]["signatures"][0]["sig"].as_str().unwrap();
  let mut bytes = BASE64.decode(sig).unwrap();
  let last = bytes.len() - 1;
  bytes[last] ^= 1;
  bundle["dsseEnvelope"]["signatures"][0]["sig"] = json!(BASE64.encode(bytes));
  serde_json::to_vec(&bundle).unwrap()
}
