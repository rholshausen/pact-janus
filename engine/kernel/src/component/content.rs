//! The content interface's request/result documents and native-binding trait
//! (component-interfaces spec §6, `schemas/v1/content.schema.json`): octets to document and back.
//! "This is the entire content of the rule that the kernel knows nothing about JSON — a shape
//! applies to what a content component decoded, never to a guess."
//!
//! `decode`/`encode`'s document is `plan::RuntimeValue` directly: the wire form tags it
//! (`x-tagged-by: "encoded"`) because JSON text needs a marker to carry bytes losslessly, but the
//! native binding passes in-memory values, and `RuntimeValue` already *is* "the shape language's
//! value model — JSON values plus bytes" (spec §6.1) with no ambiguity to tag.

use super::ComponentError;
use super::parts::SlotValue;
use crate::plan::RuntimeValue;
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct Decode {
  pub content_type: String,
  pub value: SlotValue,
  pub options: Option<Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DecodeResult {
  pub document: RuntimeValue,
  /// Value-dependent losses this decode could not preserve (spec §6.4); empty when there are
  /// none, which is every case this task's components produce.
  pub degradations: Vec<Value>,
}

#[derive(Debug, Clone)]
pub struct Encode {
  pub content_type: String,
  pub document: RuntimeValue,
  pub options: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct EncodeResult {
  pub value: SlotValue,
}

#[derive(Debug, Clone)]
pub struct Compile {
  pub content_type: String,
  pub shape: Value,
  pub path: String,
}

#[derive(Debug, Clone)]
pub struct CompileResult {
  /// A plan fragment (design 2.4's grammar) spliced at `Compile::path`. `None` is legitimate —
  /// spec §6.3: "the kernel then compiles the slot generically and calls decode at execution
  /// time."
  pub fragment: Option<Value>,
  pub grammar_version: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Detect {
  pub value: SlotValue,
  pub hint: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DetectResult {
  pub media_type: Option<String>,
  pub confidence: Option<f64>,
}

/// Native binding (spec §9.1), same shape as [`super::transport::TransportComponent`].
pub trait ContentComponent {
  fn decode(&self, req: Decode) -> Result<DecodeResult, ComponentError>;
  fn encode(&self, req: Encode) -> Result<EncodeResult, ComponentError>;
  fn compile(&self, req: Compile) -> Result<CompileResult, ComponentError>;
  fn detect(&self, req: Detect) -> Result<DetectResult, ComponentError>;
}
