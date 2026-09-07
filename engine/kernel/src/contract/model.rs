//! The Janus contract model (contract-file spec §2–§7, `schemas/v1/contract.schema.json`).
//!
//! Every struct here mirrors one of the schema's `$defs`, field for field, **in the schema's own
//! property order** — `serde_json` serializes a struct's fields in declaration order (unlike a
//! map, which it would sort or leave insertion-ordered), so this is what makes ADR 0011 §2.4's
//! canonical member order fall out of the type instead of a hand-rolled serializer.
//!
//! Anything the schema calls opaque — shape nodes, the selection report, a variant's assignment,
//! state parameters and variant-param bindings — stays a `BTreeMap`/`Value` here. Shapes are
//! design 2.2's business, the selection report and variant bindings are design 2.3's; re-typing
//! them in the contract model would mean re-deriving their semantics in a second place.
//! `BTreeMap` rather than `serde_json::Map` for these: it sorts keys, which makes the canonical
//! writer's determinism (§2.4) independent of whatever order the code that built the value
//! happened to insert members in, rather than a property that depends on construction order.

use crate::common::{Requirement, State, Transport};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// One part's shapes: slot name -> shape node. Each value is design 2.2's shape document, opaque
/// here (contract-file spec's `ShapePart`).
pub type ShapePart = BTreeMap<String, Value>;

/// One part's recorded values: slot name -> slot value (contract-file spec's `ValuePart`).
pub type ValuePart = BTreeMap<String, SlotValue>;

/// A Janus contract (`contract.schema.json`'s root). `$format` is first by construction: it is the
/// first field declared here, and `serde_json` never reorders struct fields.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Contract {
  #[serde(rename = "$format")]
  pub format: String,
  #[serde(rename = "$schema", skip_serializing_if = "Option::is_none")]
  pub schema: Option<String>,
  pub consumer: Party,
  pub provider: Party,
  pub interactions: Vec<Interaction>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub metadata: Option<Metadata>,
}

/// The format token this crate writes and reads (contract-file spec §2.3).
pub const FORMAT: &str = "janus-contract/1";

/// A named party — consumer or provider. Member name and the path to `name` are fixed by
/// contract-file spec §3.1 so brokers already deployed read them correctly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Party {
  pub name: String,
}

/// One interaction: its shape recorded once, and the evidence of every variant exercised
/// (contract-file spec §4). Identity is `description` plus `states` (§4.2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Interaction {
  pub description: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub transport: Option<Transport>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub states: Option<Vec<State>>,
  /// Part name -> slot name -> shape (opaque; design 2.2 owns the shape's own interior).
  pub parts: BTreeMap<String, ShapePart>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub requires: Option<Vec<Requirement>>,
  pub selection: RecordedSelection,
}

/// Design 2.3's variant-selection document plus this design's evidence
/// (contract-file spec §5.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecordedSelection {
  pub variants: Vec<RecordedVariant>,
  /// Design 2.3's selection report, opaque here.
  pub report: BTreeMap<String, Value>,
}

/// One exercised variant: its identity from design 2.3, and the concrete values it produced.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecordedVariant {
  pub id: String,
  /// The assignment is the truth and `id` is its name (variant-semantics spec §2.2); opaque here.
  pub assignment: Vec<Value>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub states: Option<Vec<ResolvedState>>,
  /// Part name -> slot name -> the concrete value produced.
  pub parts: BTreeMap<String, ValuePart>,
}

/// A state as one variant needs it: resolved values only, never bindings (variant-semantics
/// spec §6.4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolvedState {
  pub name: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub params: Option<BTreeMap<String, Value>>,
}

/// The value a slot carried, always wrapped (contract-file spec §5.3): a bare value with a
/// sibling tag would be ambiguous against user data that happens to have that member.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SlotValue {
  /// `null` means the slot carried a null; the slot being empty is expressed by omitting its key
  /// from the enclosing `ValuePart`, never by a special `content` value (contract-file spec §5.3).
  pub content: Value,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub encoded: Option<String>,
  #[serde(rename = "content-type", skip_serializing_if = "Option::is_none")]
  pub content_type: Option<String>,
}

/// How the file came to exist. Deliberately outside the contract's content-addressed identity
/// (contract-file spec §3.2) — never `pactSpecification` (ADR 0011).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Metadata {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub writer: Option<BTreeMap<String, String>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub created: Option<String>,
  #[serde(rename = "build-url", skip_serializing_if = "Option::is_none")]
  pub build_url: Option<String>,
}
