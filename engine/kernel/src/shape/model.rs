//! The shape document model (shape-language spec §3–§4, `schemas/v1/shape.schema.json`).
//!
//! A [`ShapeNode`] is one node of a shape tree. Core operators (spec §4) are fully typed as
//! [`CoreShape`]; a namespaced, component-contributed operator (spec §3.5) stays opaque — the
//! kernel "treats them opaquely and MUST NOT guess" (spec §3.5), so a `Component` node keeps its
//! operator-specific members as raw JSON for a later stage (a loaded component, design 2.6) to
//! interpret.

use serde_json::Value;
use std::collections::BTreeMap;

/// A value a shape admits, tagged per the protocol's tagged-content convention (protocol spec
/// §2.5, ADR 0006): `encoded` absent means `value` is its natural JSON form, `"base64"` means
/// `value` is a string carrying the base64 of an octet sequence.
#[derive(Debug, Clone, PartialEq)]
pub struct Example {
  pub value: Value,
  pub encoded: Option<String>,
}

/// One shape node: an operator plus the cross-cutting members every node may carry (spec §3.1,
/// §3.3, §3.4) regardless of which operator it names.
#[derive(Debug, Clone, PartialEq)]
pub struct ShapeNode {
  /// A value this shape admits (spec §3.3). Required for `equality` and `type`; optional
  /// elsewhere, including as an explicit override on a structural operator.
  pub example: Option<Example>,
  /// Generator component reference and configuration (spec §3.4), opaque here: design 2.6 owns
  /// its shape.
  pub generator: Option<Value>,
  pub kind: ShapeKind,
}

impl ShapeNode {
  /// The operator this node names, as authored (`shape.schema.json`'s discriminator).
  pub fn operator(&self) -> &str {
    match &self.kind {
      ShapeKind::Core(core) => core.operator_name(),
      ShapeKind::Component { operator, .. } => operator,
    }
  }

  /// `true` iff this node's `admits` set contains absence (`⊥`) — `optional` and `forbidden`
  /// are the only core operators that do (spec §4.1). A component operator's `admits` is opaque
  /// to the kernel (spec §3.5), so this is conservatively `false` for one regardless of what it
  /// might declare — the composition checks that rely on this method (spec §5.1, §5.2) apply to
  /// the core vocabulary only.
  pub fn admits_absent(&self) -> bool {
    matches!(
      self.kind,
      ShapeKind::Core(CoreShape::Optional { .. } | CoreShape::Forbidden)
    )
  }
}

/// What operator a node names, and the members that are specific to it.
#[derive(Debug, Clone, PartialEq)]
pub enum ShapeKind {
  Core(CoreShape),
  /// A component-contributed operator, namespaced `<component>:<name>` (spec §3.5). `raw` holds
  /// every member other than `shape`, `example`, `encoded` and `generator`, unexamined.
  Component {
    operator: String,
    raw: BTreeMap<String, Value>,
  },
}

/// The kind predicates (spec §4.2): `string`, `number`, `integer`, `decimal`, `boolean`, `null`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueKind {
  String,
  Number,
  Integer,
  Decimal,
  Boolean,
  Null,
}

impl ValueKind {
  pub fn operator_name(self) -> &'static str {
    match self {
      ValueKind::String => "string",
      ValueKind::Number => "number",
      ValueKind::Integer => "integer",
      ValueKind::Decimal => "decimal",
      ValueKind::Boolean => "boolean",
      ValueKind::Null => "null",
    }
  }
}

/// `datetime`, `date` and `time` (spec §4.2): the same shape, one per format family.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemporalKind {
  DateTime,
  Date,
  Time,
}

impl TemporalKind {
  pub fn operator_name(self) -> &'static str {
    match self {
      TemporalKind::DateTime => "datetime",
      TemporalKind::Date => "date",
      TemporalKind::Time => "time",
    }
  }
}

/// The core operator set (shape-language spec §4.1), fully typed.
#[derive(Debug, Clone, PartialEq)]
pub enum CoreShape {
  Any,
  /// Admits exactly `ShapeNode::example` (required; spec §4.2).
  Equality,
  /// Admits every value of `ShapeNode::example`'s kind (example required; spec §4.2).
  Type,
  Kind(ValueKind),
  NotEmpty,
  Regex {
    pattern: String,
  },
  Temporal {
    kind: TemporalKind,
    /// Absent means "any string parsing as ISO-8601" for this kind (spec §4.2).
    format: Option<String>,
  },
  Include {
    substring: String,
  },
  ContentType {
    content_type: String,
  },
  Semver,
  /// Members not named here are admitted and ignored (spec §4.3's must-ignore default).
  Object {
    members: BTreeMap<String, ShapeNode>,
  },
  /// Exactly `entries.len()` elements, positionally matched (spec §4.3).
  Array {
    entries: Vec<ShapeNode>,
  },
  EachLike {
    items: Box<ShapeNode>,
    min: u64,
    max: Option<u64>,
  },
  EachEntry {
    /// Absent means any key admitted (spec §4.3).
    keys: Option<Box<ShapeNode>>,
    values: Box<ShapeNode>,
    min: u64,
    max: Option<u64>,
  },
  /// Each entry shape must find a distinct admitting element (spec §4.3); the one core operator
  /// with opaque comparability (spec §8).
  Contains {
    entries: Vec<ShapeNode>,
  },
  Optional {
    of: Box<ShapeNode>,
  },
  /// Admits only absence (spec §4.4); takes no `of`.
  Forbidden,
  Nullable {
    of: Box<ShapeNode>,
  },
  /// Admits the literal values in `options` (spec §4.4). `ShapeNode::example`, if present, names
  /// the dimension's default point and must be one of `options` (spec §5.3).
  AnyOf {
    options: Vec<Value>,
  },
  /// A tagged union (spec §4.4, §5.4): each alternative is an `object` shape (optionally wrapped
  /// in `nullable`) binding `discriminator` to a distinct literal.
  OneOf {
    discriminator: String,
    alternatives: BTreeMap<String, ShapeNode>,
    /// Absent means the lexicographically first alternative name (spec §5.4).
    default: Option<String>,
  },
}

impl CoreShape {
  pub fn operator_name(&self) -> &'static str {
    match self {
      CoreShape::Any => "any",
      CoreShape::Equality => "equality",
      CoreShape::Type => "type",
      CoreShape::Kind(kind) => kind.operator_name(),
      CoreShape::NotEmpty => "not-empty",
      CoreShape::Regex { .. } => "regex",
      CoreShape::Temporal { kind, .. } => kind.operator_name(),
      CoreShape::Include { .. } => "include",
      CoreShape::ContentType { .. } => "content-type",
      CoreShape::Semver => "semver",
      CoreShape::Object { .. } => "object",
      CoreShape::Array { .. } => "array",
      CoreShape::EachLike { .. } => "each-like",
      CoreShape::EachEntry { .. } => "each-entry",
      CoreShape::Contains { .. } => "contains",
      CoreShape::Optional { .. } => "optional",
      CoreShape::Forbidden => "forbidden",
      CoreShape::Nullable { .. } => "nullable",
      CoreShape::AnyOf { .. } => "any-of",
      CoreShape::OneOf { .. } => "one-of",
    }
  }
}

/// The unnamespaced operator vocabulary this specification defines (spec §3.1's
/// `x-known-values`). An unnamespaced name outside this list is a named failure, never a
/// silently-ignored one (spec §3.7).
pub const CORE_OPERATORS: &[&str] = &[
  "any",
  "equality",
  "type",
  "string",
  "number",
  "integer",
  "decimal",
  "boolean",
  "null",
  "regex",
  "datetime",
  "date",
  "time",
  "include",
  "content-type",
  "semver",
  "not-empty",
  "object",
  "array",
  "each-like",
  "each-entry",
  "contains",
  "optional",
  "nullable",
  "forbidden",
  "any-of",
  "one-of",
];
