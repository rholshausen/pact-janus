//! Resolvers (plan-grammar spec §2.4): where a `resolve` node's value actually comes from. "The
//! kernel does not know what a header is; it knows how to ask" — a [`Resolver`] is the answer to
//! that ask.
//!
//! [`CapturedValues`] is deliberately the *only* resolver here, serving both roles plan task 3.4
//! names: a golden-corpus case's captured-value form (plan-grammar spec §6.1's `case.json`
//! `values`), used as-is, and an interaction's already-decoded request/response parts, built by
//! capturing each part's value under its own path (`"$.request.method"`, `"$.response.body"`,
//! ...). Both are the same shape — an absolute path mapped to an already-decoded value — so one
//! resolver serves both rather than two parallel implementations of the same lookup.
//!
//! What is deliberately **not** here: decoding wire bytes or JSON text into a document. That is a
//! transport/content component's job (component-interfaces spec, design 2.6, built in plan task
//! 4.2) — this resolver is only ever handed values that are already decoded, which is what keeps
//! it from being the kernel-knows-HTTP-and-JSON leak it might otherwise look like.

use super::value::{RuntimeValue, navigate};
use std::collections::BTreeMap;

pub trait Resolver {
  /// Resolve an absolute path (`$....`) against the interaction context. Never fails: a path
  /// this resolver has nothing for is [`RuntimeValue::Absent`], because "not there" is a fact a
  /// plan discovers via `check:exists`, not a reason to abort execution (protocol spec §10.1's
  /// errors-are-values rule applies here too — a missing capture is not even an error).
  fn resolve(&self, path: &str) -> RuntimeValue;

  /// Why the slot `path` is at or under has no document, when its octets could not be decoded
  /// (component-interfaces spec §6.1, ADR 0101): the message the interpreter reports at the slot. A
  /// node whose value under test is read from such a slot is not executed, since a shape never applies
  /// to a guess. `None`, the default, is a value that resolves as [`Resolver::resolve`] says.
  fn undecodable(&self, _path: &str) -> Option<Undecodable> {
    None
  }

  /// Whether any slot could not be decoded, so an interpreter need not ask of every node.
  fn any_undecodable(&self) -> bool {
    false
  }
}

/// A slot whose octets the component that claims them could not decode (ADR 0101): where it is, and
/// what to say about it.
#[derive(Debug, Clone, PartialEq)]
pub struct Undecodable {
  /// The slot's absolute path: `$.response.body`.
  pub path: String,
  pub message: String,
}

/// Where `match:content-type` (shape spec §4.2) actually gets its answer: a content component's
/// `detect` operation (component-interfaces spec §6.5), narrowed to what the interpreter needs.
/// "The kernel does not know what a header is; it knows how to ask" applies here too — the kernel
/// does not know what JSON looks like, it knows how to ask a content component.
///
/// The live exchange and verification paths answer it over the session's content registry, with
/// the arrival's own parts ([`crate::component::ContentDetection`], staged plan S1.10).
pub trait ContentDetector {
  /// The content type of `value`, which was resolved from `at` when a path resolved it: a whole
  /// slot (`$.response.body`) is the arrival's octets, which a detector holding them reads instead.
  /// `None` means no type is known for it, not "no detector was available", which
  /// [`super::interpret::execute_with_content`] reports on its own when `content` itself is `None`.
  fn detect(&self, value: &RuntimeValue, at: Option<&str>) -> Option<String>;
}

/// A resolver over a flat map of absolute path -> already-decoded value, keyed at whatever depth
/// the caller captured it (a whole part, `"$.response.body"`, or a single field,
/// `"$.response.status"`). A requested path resolves against the *longest* captured key that is a
/// prefix of it, and [`navigate`] walks the remainder — so capturing `"$.response.body"` once
/// answers every path reaching under it.
#[derive(Debug, Default)]
pub struct CapturedValues {
  captured: BTreeMap<String, RuntimeValue>,
  /// The slots that could not be decoded, by path, with what the interpreter says at each.
  undecodable: BTreeMap<String, String>,
}

impl CapturedValues {
  pub fn new() -> CapturedValues {
    CapturedValues::default()
  }

  /// Capture an already-decoded value at `path` (an absolute path, e.g. `"$.response.body"`).
  pub fn capture(mut self, path: impl Into<String>, value: RuntimeValue) -> CapturedValues {
    self.captured.insert(path.into(), value);
    self
  }

  /// A slot at `path` whose octets could not be decoded, and why (ADR 0101): it resolves as absent,
  /// and a node reading it is not executed ([`Resolver::undecodable`]).
  pub fn capture_undecodable(
    mut self,
    path: impl Into<String>,
    message: impl Into<String>,
  ) -> CapturedValues {
    let path = path.into();
    self.captured.insert(path.clone(), RuntimeValue::Absent);
    self.undecodable.insert(path, message.into());
    self
  }

  /// From a golden-corpus case's `values` map (plan-grammar spec §6.1): JSON values keyed by the
  /// path a resolver would supply.
  pub fn from_json(values: &BTreeMap<String, serde_json::Value>) -> CapturedValues {
    let mut resolver = CapturedValues::new();
    for (path, value) in values {
      resolver = resolver.capture(path.clone(), RuntimeValue::from_json(value));
    }
    resolver
  }
}

impl Resolver for CapturedValues {
  fn resolve(&self, path: &str) -> RuntimeValue {
    let mut candidate = path;
    loop {
      if let Some(value) = self.captured.get(candidate) {
        return navigate(value, &path[candidate.len()..]);
      }
      match candidate.rfind(['.', '[']) {
        Some(cut) => candidate = &candidate[..cut],
        None => return RuntimeValue::Absent,
      }
    }
  }

  fn any_undecodable(&self) -> bool {
    !self.undecodable.is_empty()
  }

  /// The slot a path is at or under, when that slot could not be decoded. A value captured at a
  /// longer path is its own, decoded or not: the v1–v4 resolver captures each header beside the
  /// slot holding them all.
  fn undecodable(&self, path: &str) -> Option<Undecodable> {
    let mut candidate = path;
    loop {
      if let Some(message) = self.undecodable.get(candidate) {
        return Some(Undecodable {
          path: candidate.to_string(),
          message: message.clone(),
        });
      }
      if self.captured.contains_key(candidate) {
        return None;
      }
      candidate = &candidate[..candidate.rfind(['.', '['])?];
    }
  }
}
