//! Computing a shape's variant space (shape-language spec §6, `schemas/v1/variant-space.schema.json`).
//!
//! Selecting, sampling and naming whole variants is design 2.3's business (its own plan task);
//! this module only answers "which dimensions does this shape tree contribute, and what points
//! does each have" — the document design 2.3 consumes (spec §6.6). It is computed here, alongside
//! the plan compiler ([`crate::plan`]), because both walk exactly the same tree by exactly the
//! same rules (spec §6.2's determinism requirement), and [`super::path`] is what keeps the two
//! walks agreeing on a dimension id.

use super::model::{CoreShape, ShapeKind, ShapeNode};
use super::path;
use serde_json::Value;

/// The dimensions a shape tree contributes, in declaration order (spec §6.6): parents before the
/// dimensions they gate.
#[derive(Debug, Clone, PartialEq)]
pub struct VariantSpace {
  pub dimensions: Vec<Dimension>,
}

/// One axis along which a shape's admitted set is deliberately wider than one case (spec §6.1).
#[derive(Debug, Clone, PartialEq)]
pub struct Dimension {
  pub id: String,
  pub path: String,
  pub facet: &'static str,
  pub operator: &'static str,
  pub points: Vec<Point>,
  pub default: String,
  pub gated_by: Vec<Gate>,
}

/// One point of a dimension.
#[derive(Debug, Clone, PartialEq)]
pub struct Point {
  pub name: String,
  /// The literal this point selects, for `value` facets (spec §6.6).
  pub value: Option<Value>,
  /// The cardinality this point selects, for `cardinality` facets (spec §6.6).
  pub size: Option<u64>,
}

impl Point {
  fn named(name: &str) -> Point {
    Point {
      name: name.to_string(),
      value: None,
      size: None,
    }
  }
}

/// A point of another dimension that must be selected for this dimension to be active (spec §6.3).
#[derive(Debug, Clone, PartialEq)]
pub struct Gate {
  pub dimension: String,
  pub point: String,
}

/// One point of an `each-like`/`each-entry` cardinality dimension (spec §6.4).
#[derive(Debug, Clone, PartialEq)]
pub struct CardinalityPoint {
  pub name: String,
  pub size: u64,
}

/// The canonical rendering of an `any-of` option, which is also its point name (spec §6.4): a
/// string option is its own text; a number, boolean or `null` option is its JSON text.
pub fn canonical_point_name(value: &Value) -> String {
  match value {
    Value::String(s) => s.clone(),
    other => other.to_string(),
  }
}

/// The cardinality dimension's points for `min`/`max` (spec §6.4): `min`; `min+1` when the space
/// is wider than one case; `max` when it is finite and distinct from both.
pub fn cardinality_points(min: u64, max: Option<u64>) -> Vec<CardinalityPoint> {
  let mut points = vec![CardinalityPoint {
    name: "min".to_string(),
    size: min,
  }];
  let has_min_plus_1 = match max {
    None => true,
    Some(m) => m > min,
  };
  if has_min_plus_1 {
    points.push(CardinalityPoint {
      name: "min+1".to_string(),
      size: min + 1,
    });
  }
  if let Some(m) = max {
    let distinct_from_min = m != min;
    let distinct_from_min_plus_1 = !has_min_plus_1 || m != min + 1;
    if distinct_from_min && distinct_from_min_plus_1 {
      points.push(CardinalityPoint {
        name: "max".to_string(),
        size: m,
      });
    }
  }
  points
}

/// Compute the variant space of one shape tree, rooted at `at` (a dimension path, spec §6.2 —
/// typically [`path::root`]'s `"<part>.<slot>"`).
pub fn compute(shape: &ShapeNode, at: &str) -> VariantSpace {
  let mut dimensions = Vec::new();
  walk(shape, at, &[], &mut dimensions);
  VariantSpace { dimensions }
}

/// Compute and concatenate the variant space of several shape trees — typically every slot of an
/// interaction's parts — in the order given (spec §6.2's determinism requirement extends across
/// them: callers should visit parts and slots in a stable order, which a `BTreeMap` gives for
/// free).
pub fn compute_many<'a>(shapes: impl IntoIterator<Item = (String, &'a ShapeNode)>) -> VariantSpace {
  let mut dimensions = Vec::new();
  for (at, shape) in shapes {
    dimensions.extend(compute(shape, &at).dimensions);
  }
  VariantSpace { dimensions }
}

fn walk(shape: &ShapeNode, at: &str, gates: &[Gate], out: &mut Vec<Dimension>) {
  let ShapeKind::Core(core) = &shape.kind else {
    // A component operator's `admits` — and therefore its dimension, if any — is opaque to the
    // kernel, which "MUST NOT guess" (shape spec §3.5). A component that wants variant coverage
    // declares its own dimension through the component interface (design 2.6), not through this
    // walk.
    return;
  };
  match core {
    CoreShape::Optional { of } => {
      let id = path::dimension_id(at, "presence");
      out.push(Dimension {
        id: id.clone(),
        path: at.to_string(),
        facet: "presence",
        operator: "optional",
        points: vec![Point::named("present"), Point::named("absent")],
        default: "present".to_string(),
        gated_by: gates.to_vec(),
      });
      let mut inner = gates.to_vec();
      inner.push(Gate {
        dimension: id,
        point: "present".to_string(),
      });
      walk(of, at, &inner, out);
    }
    CoreShape::Nullable { of } => {
      let id = path::dimension_id(at, "nullability");
      out.push(Dimension {
        id: id.clone(),
        path: at.to_string(),
        facet: "nullability",
        operator: "nullable",
        points: vec![Point::named("non-null"), Point::named("null")],
        default: "non-null".to_string(),
        gated_by: gates.to_vec(),
      });
      let mut inner = gates.to_vec();
      inner.push(Gate {
        dimension: id,
        point: "non-null".to_string(),
      });
      walk(of, at, &inner, out);
    }
    CoreShape::AnyOf { options } => {
      if options.len() < 2 {
        // A one-option `any-of` contributes no dimension (spec §5.3, §6.1).
        return;
      }
      let default = shape
        .example
        .as_ref()
        .map(|e| canonical_point_name(&e.value))
        .unwrap_or_else(|| canonical_point_name(&options[0]));
      out.push(Dimension {
        id: path::dimension_id(at, "value"),
        path: at.to_string(),
        facet: "value",
        operator: "any-of",
        points: options
          .iter()
          .map(|v| Point {
            name: canonical_point_name(v),
            value: Some(v.clone()),
            size: None,
          })
          .collect(),
        default,
        gated_by: gates.to_vec(),
      });
    }
    CoreShape::OneOf {
      alternatives,
      default,
      ..
    } => {
      // Well-formedness (spec §5.4) requires at least two alternatives, but a defensive check
      // costs nothing and keeps this function total over any tree, not just well-formed ones.
      if alternatives.len() < 2 {
        return;
      }
      let id = path::dimension_id(at, "alternative");
      let default_name = default.clone().unwrap_or_else(|| {
        alternatives
          .keys()
          .next()
          .expect("checked non-empty above")
          .clone()
      });
      out.push(Dimension {
        id: id.clone(),
        path: at.to_string(),
        facet: "alternative",
        operator: "one-of",
        points: alternatives.keys().map(|name| Point::named(name)).collect(),
        default: default_name,
        gated_by: gates.to_vec(),
      });
      for (name, alt) in alternatives {
        let alt_path = path::alternative(at, name);
        let mut inner = gates.to_vec();
        inner.push(Gate {
          dimension: id.clone(),
          point: name.clone(),
        });
        walk(alt, &alt_path, &inner, out);
      }
    }
    CoreShape::EachLike { items, min, max } => {
      let points = cardinality_points(*min, *max);
      if points.len() >= 2 {
        out.push(Dimension {
          id: path::dimension_id(at, "cardinality"),
          path: at.to_string(),
          facet: "cardinality",
          operator: "each-like",
          points: points
            .iter()
            .map(|p| Point {
              name: p.name.clone(),
              value: None,
              size: Some(p.size),
            })
            .collect(),
          default: "min".to_string(),
          gated_by: gates.to_vec(),
        });
      }
      // Cardinality does not gate its subtree (spec §6.4): interior dimensions vary regardless
      // of which cardinality point is selected.
      walk(items, &path::each_like_item(at), gates, out);
    }
    CoreShape::EachEntry {
      keys,
      values,
      min,
      max,
    } => {
      let points = cardinality_points(*min, *max);
      if points.len() >= 2 {
        out.push(Dimension {
          id: path::dimension_id(at, "cardinality"),
          path: at.to_string(),
          facet: "cardinality",
          operator: "each-entry",
          points: points
            .iter()
            .map(|p| Point {
              name: p.name.clone(),
              value: None,
              size: Some(p.size),
            })
            .collect(),
          default: "min".to_string(),
          gated_by: gates.to_vec(),
        });
      }
      if let Some(keys) = keys {
        walk(keys, &path::each_entry_key(at), gates, out);
      }
      walk(values, &path::each_entry_value(at), gates, out);
    }
    CoreShape::Object { members } => {
      // A `BTreeMap`'s iteration order is already the lexicographic order spec §6.2 requires.
      for (name, member) in members {
        walk(member, &path::member(at, name), gates, out);
      }
    }
    CoreShape::Array { entries } => {
      for (index, entry) in entries.iter().enumerate() {
        walk(entry, &path::array_index(at, index), gates, out);
      }
    }
    CoreShape::Contains { entries } => {
      // Spec §6.2 names no path segment for `contains` — an existential match against candidate
      // elements, not an addressed position. Array-index syntax is reused as the least surprising
      // stable id; `contains`'s comparability is already opaque (spec §8), so this walk does not
      // need to be more precise than that.
      for (index, entry) in entries.iter().enumerate() {
        walk(entry, &path::array_index(at, index), gates, out);
      }
    }
    CoreShape::Any
    | CoreShape::Equality
    | CoreShape::Type
    | CoreShape::Kind(_)
    | CoreShape::NotEmpty
    | CoreShape::Regex { .. }
    | CoreShape::Temporal { .. }
    | CoreShape::Include { .. }
    | CoreShape::ContentType { .. }
    | CoreShape::Semver
    | CoreShape::Forbidden => {
      // Leaves: no width, no dimension (spec §6.1).
    }
  }
}
