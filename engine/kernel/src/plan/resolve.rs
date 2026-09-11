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
}

/// Where `match:content-type` (shape spec §4.2) actually gets its answer: a content component's
/// `detect` operation (component-interfaces spec §6.5), narrowed to what the interpreter needs.
/// "The kernel does not know what a header is; it knows how to ask" applies here too — the kernel
/// does not know what JSON looks like, it knows how to ask a content component.
///
/// A single slot, not a registry: every caller today hands in one hardcoded component (there is
/// only one, `engine/component-json`'s `JsonContent`). No resolution mechanism exists yet for
/// component-interfaces spec §2.3's "collect requirements across interactions, resolve to loaded
/// components" — tracked in `Documentation/kernel-boundary-review.md`'s finding-1 resolution as a
/// gap for whoever adds a second content component (Phase 8 task 8.1).
pub trait ContentDetector {
  /// `None` means this detector doesn't recognise `value` as any type it handles — not "no
  /// detector was available", which [`super::interpret::execute_with_content`] reports on its own
  /// when `content` itself is `None`.
  fn detect(&self, value: &RuntimeValue) -> Option<String>;
}

/// A resolver over a flat map of absolute path -> already-decoded value, keyed at whatever depth
/// the caller captured it (a whole part, `"$.response.body"`, or a single field,
/// `"$.response.status"`). A requested path resolves against the *longest* captured key that is a
/// prefix of it, and [`navigate`] walks the remainder — so capturing `"$.response.body"` once
/// answers every path reaching under it.
#[derive(Debug, Default)]
pub struct CapturedValues {
  captured: BTreeMap<String, RuntimeValue>,
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
}
