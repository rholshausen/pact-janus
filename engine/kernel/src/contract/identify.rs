//! Identification (contract-file spec §2.3): "which schema to validate against," never a
//! substitute for validating. Both modes degrade to a clear "this is not a Janus contract,"
//! never to a misparse.

/// The exact byte prefix a canonically-written contract begins with.
const STRICT_PREFIX: &[u8] = b"{\"$format\":";

/// How far a tolerant reader scans looking for `"$format"` (contract-file spec §2.3).
const TOLERANT_WINDOW: usize = 8 * 1024;

/// Identification mode a reader chooses (contract-file spec §2.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentifyMode {
  /// Compare the eleven-byte prefix. For pipelines that control their writers, and for rejecting
  /// non-contracts cheaply.
  Strict,
  /// Scan a bounded window for the member name `"$format"`. For files that have round-tripped
  /// through a broker, a formatter or a script.
  Tolerant,
}

/// `true` iff `bytes` begins with the canonical `{"$format":` prefix.
pub fn identify_strict(bytes: &[u8]) -> bool {
  bytes.starts_with(STRICT_PREFIX)
}

/// `true` iff `"$format"` appears anywhere in the first [`TOLERANT_WINDOW`] bytes.
pub fn identify_tolerant(bytes: &[u8]) -> bool {
  let window = &bytes[..bytes.len().min(TOLERANT_WINDOW)];
  window.windows(b"\"$format\"".len()).any(|w| w == b"\"$format\"")
}

/// Identify `bytes` under the given mode.
pub fn identify(bytes: &[u8], mode: IdentifyMode) -> bool {
  match mode {
    IdentifyMode::Strict => identify_strict(bytes),
    IdentifyMode::Tolerant => identify_tolerant(bytes),
  }
}
