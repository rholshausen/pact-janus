//! Routing content by media type (component-interfaces spec §6, contract-file spec §5.5): the
//! registry that replaced the engine's single content slot when plan task 8.1 put a second content
//! component behind it.
//!
//! A [`ContentRegistry`] is itself a [`ContentComponent`], so everything downstream of it — the
//! exchange loop, the verifier, the wire helpers — still holds one `dyn ContentComponent` and never
//! learns how many there are. It asks each registered component whether it [`handles`] a media type,
//! in registration order, and hands the call to the first that does. The kernel still knows no media
//! type: it compares strings the components declared.
//!
//! [`handles`]: ContentComponent::handles

use super::content::{
  Compile, CompileResult, ContentComponent, Decode, DecodeResult, Detect, DetectResult, Encode, EncodeResult,
};
use super::error::ComponentError;
use std::sync::Arc;

/// Content components in precedence order. Earlier wins: a session's declared components are
/// registered before the embedding's in-tree ones, so a project that declares its own handler for
/// a type the engine also handles gets the one it declared.
#[derive(Clone, Default)]
pub struct ContentRegistry {
  components: Vec<Arc<dyn ContentComponent>>,
}

impl ContentRegistry {
  pub fn new() -> Self {
    Self::default()
  }

  /// Register `component` after every component already here.
  pub fn with(mut self, component: Arc<dyn ContentComponent>) -> Self {
    self.components.push(component);
    self
  }

  /// This registry's components followed by `fallback`'s: a session's own components ahead of the
  /// engine's in-tree ones.
  pub fn then(mut self, fallback: &ContentRegistry) -> Self {
    self.components.extend(fallback.components.iter().cloned());
    self
  }

  pub fn is_empty(&self) -> bool {
    self.components.is_empty()
  }

  /// The component that handles `content_type`, if any does.
  pub fn route(&self, content_type: &str) -> Option<&Arc<dyn ContentComponent>> {
    self
      .components
      .iter()
      .find(|component| component.handles(content_type))
  }

  fn routed(&self, content_type: &str) -> Result<&Arc<dyn ContentComponent>, ComponentError> {
    self
      .route(content_type)
      .ok_or_else(|| ComponentError::unsupported_content_type(content_type))
  }
}

impl ContentComponent for ContentRegistry {
  fn handles(&self, content_type: &str) -> bool {
    self.route(content_type).is_some()
  }

  fn decode(&self, req: Decode) -> Result<DecodeResult, ComponentError> {
    self.routed(&req.content_type)?.decode(req)
  }

  fn encode(&self, req: Encode) -> Result<EncodeResult, ComponentError> {
    self.routed(&req.content_type)?.encode(req)
  }

  fn compile(&self, req: Compile) -> Result<CompileResult, ComponentError> {
    self.routed(&req.content_type)?.compile(req)
  }

  /// Detection resolves an *absent* type (spec §6.5), so there is nothing to route on: the first
  /// component that recognises the octets answers. A component that does not implement detection
  /// answers `operation-unsupported`, which here means "not me".
  fn detect(&self, req: Detect) -> Result<DetectResult, ComponentError> {
    for component in &self.components {
      if let Ok(result) = component.detect(req.clone())
        && result.media_type.is_some()
      {
        return Ok(result);
      }
    }
    Ok(DetectResult {
      media_type: None,
      confidence: None,
    })
  }
}

/// Whether `content_type` (a header value: `text/csv; charset=utf-8`) is one `pattern` declares
/// (a handshake's `media-type`: `text/csv`, `text/*` or `*/*`). Parameters are ignored and case is
/// not significant (RFC 9110 §8.3.1) — what a content type *is* is its essence.
pub fn media_type_matches(pattern: &str, content_type: &str) -> bool {
  let essence = |value: &str| {
    value
      .split(';')
      .next()
      .unwrap_or_default()
      .trim()
      .to_ascii_lowercase()
  };
  let (pattern, content_type) = (essence(pattern), essence(content_type));
  let (Some((pattern_type, pattern_subtype)), Some((main_type, subtype))) =
    (pattern.split_once('/'), content_type.split_once('/'))
  else {
    return false;
  };
  (pattern_type == "*" || pattern_type == main_type) && (pattern_subtype == "*" || pattern_subtype == subtype)
}
