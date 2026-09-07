//! The shape parser and well-formedness validator (shape-language spec §3, §5).
//!
//! This is deliberately not JSON-Schema validation: `shape.schema.json` fixes the document's
//! *shape*, but "the rules in spec §5 fix what a well-formed shape *means*, and no JSON Schema
//! keyword could carry the second job. The engine, not the validator, is the authority on
//! well-formedness" (shape-language spec, examples/composition-edges.md §6). Every rule checked
//! here is one that keyword-based schema validation structurally cannot express.

use super::model::{CORE_OPERATORS, CoreShape, Example, ShapeKind, ShapeNode, TemporalKind, ValueKind};
use crate::error::{Problem, push_pointer_segment};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

/// Where a recursive call sits in the tree, for the absence rule (spec §5.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Position {
  /// A member of an `object` node — the one structural position a `⊥`-admitting shape may
  /// occupy.
  ObjectMember,
  /// Anywhere else: a part's slot root, `each-like` items, `each-entry` keys/values, an
  /// `array`/`contains` entry, a `one-of` alternative, or the `of` of `optional`/`nullable`.
  NotASlot,
}

/// Node-count and depth limits an engine "MAY impose" (spec §5.6). A shape tree is authored by a
/// DSL, not generated adversarially, but a document arriving over a pipe still gets a named
/// error instead of a stack overflow.
const MAX_DEPTH: usize = 64;
const MAX_NODES: u32 = 20_000;

struct Budget {
  nodes: u32,
  limit_reported: bool,
}

impl Budget {
  fn new() -> Self {
    Budget {
      nodes: 0,
      limit_reported: false,
    }
  }

  /// `true` iff parsing may proceed at `depth`. Reports the limit at most once: once a tree is
  /// this large, further violations are noise, not new information.
  fn enter(&mut self, depth: usize, path: &str, problems: &mut Vec<Problem>) -> bool {
    if self.limit_reported {
      return false;
    }
    self.nodes += 1;
    if depth > MAX_DEPTH || self.nodes > MAX_NODES {
      self.limit_reported = true;
      problems.push(Problem {
        pointer: path.to_string(),
        message: format!(
          "shape tree exceeds this engine's limits (depth <= {MAX_DEPTH}, nodes <= {MAX_NODES})"
        ),
      });
      return false;
    }
    true
  }
}

/// Parse and validate one shape document, rooted at `path` (an RFC 6901 JSON pointer prefix,
/// e.g. `/parts/response/body`). Returns every well-formedness violation found (spec §5), not
/// just the first — a document worth reporting on is worth reporting on completely.
pub fn parse(value: &Value, path: &str) -> Result<ShapeNode, Vec<Problem>> {
  let mut problems = Vec::new();
  let mut budget = Budget::new();
  let node = parse_child(
    value,
    Position::NotASlot,
    path.to_string(),
    0,
    &mut budget,
    &mut problems,
  );
  match (node, problems.is_empty()) {
    (Some(node), true) => Ok(node),
    _ => Err(problems),
  }
}

/// Parse a child node, then apply the one rule that depends on *where* a node sits rather than
/// what it is (spec §5.1): a shape admitting absence may appear only as an object member.
fn parse_child(
  value: &Value,
  position: Position,
  path: String,
  depth: usize,
  budget: &mut Budget,
  problems: &mut Vec<Problem>,
) -> Option<ShapeNode> {
  if !budget.enter(depth, &path, problems) {
    return None;
  }
  let node = parse_node(value, path.clone(), depth, budget, problems)?;
  if position == Position::NotASlot && node.admits_absent() {
    problems.push(Problem {
      pointer: path,
      message: format!(
        "'{}' admits absence, which may only appear as an object member (spec §5.1)",
        node.operator()
      ),
    });
  }
  Some(node)
}

fn parse_node(
  value: &Value,
  path: String,
  depth: usize,
  budget: &mut Budget,
  problems: &mut Vec<Problem>,
) -> Option<ShapeNode> {
  let Some(obj) = value.as_object() else {
    problems.push(Problem {
      pointer: path,
      message: "a shape node must be a JSON object".to_string(),
    });
    return None;
  };
  let Some(shape_value) = obj.get("shape") else {
    problems.push(Problem {
      pointer: path,
      message: "a shape node requires a 'shape' member naming its operator".to_string(),
    });
    return None;
  };
  let Some(operator) = shape_value.as_str() else {
    problems.push(Problem {
      pointer: path,
      message: "'shape' must be a string naming the operator".to_string(),
    });
    return None;
  };

  let example = parse_example(obj, &path, problems).unwrap_or_default();
  let generator = obj.get("generator").cloned();

  if let Some((component, name)) = operator.split_once(':') {
    if component.is_empty() || name.is_empty() {
      problems.push(Problem {
        pointer: path,
        message: format!("'{operator}' is not a valid namespaced operator (spec §3.5)"),
      });
      return None;
    }
    let raw = obj
      .iter()
      .filter(|(key, _)| !matches!(key.as_str(), "shape" | "example" | "encoded" | "generator"))
      .map(|(key, value)| (key.clone(), value.clone()))
      .collect();
    return Some(ShapeNode {
      example,
      generator,
      kind: ShapeKind::Component {
        operator: operator.to_string(),
        raw,
      },
    });
  }

  if !CORE_OPERATORS.contains(&operator) {
    problems.push(Problem {
      pointer: path,
      message: format!("unknown operator '{operator}' (spec §3.7)"),
    });
    return None;
  }

  let core = parse_core(operator, obj, example.as_ref(), &path, depth, budget, problems)?;
  Some(ShapeNode {
    example,
    generator,
    kind: ShapeKind::Core(core),
  })
}

/// `example`, tagged per the protocol's tagged-content convention (protocol spec §2.5).
fn parse_example(
  obj: &Map<String, Value>,
  path: &str,
  problems: &mut Vec<Problem>,
) -> Result<Option<Example>, ()> {
  let Some(value) = obj.get("example") else {
    return Ok(None);
  };
  let encoded = match obj.get("encoded") {
    None => None,
    Some(Value::String(tag)) => Some(tag.clone()),
    Some(_) => {
      problems.push(Problem {
        pointer: format!("{path}/encoded"),
        message: "'encoded' must be a string".to_string(),
      });
      return Err(());
    }
  };
  Ok(Some(Example {
    value: value.clone(),
    encoded,
  }))
}

#[allow(clippy::too_many_arguments)]
fn parse_core(
  operator: &str,
  obj: &Map<String, Value>,
  example: Option<&Example>,
  path: &str,
  depth: usize,
  budget: &mut Budget,
  problems: &mut Vec<Problem>,
) -> Option<CoreShape> {
  match operator {
    "any" => Some(CoreShape::Any),
    "equality" => {
      require_example(operator, example, path, problems);
      Some(CoreShape::Equality)
    }
    "type" => {
      require_example(operator, example, path, problems);
      Some(CoreShape::Type)
    }
    "string" => Some(CoreShape::Kind(ValueKind::String)),
    "number" => Some(CoreShape::Kind(ValueKind::Number)),
    "integer" => Some(CoreShape::Kind(ValueKind::Integer)),
    "decimal" => Some(CoreShape::Kind(ValueKind::Decimal)),
    "boolean" => Some(CoreShape::Kind(ValueKind::Boolean)),
    "null" => Some(CoreShape::Kind(ValueKind::Null)),
    "not-empty" => Some(CoreShape::NotEmpty),
    "regex" => {
      let pattern = require_string(obj, "pattern", operator, path, problems)?;
      Some(CoreShape::Regex { pattern })
    }
    "datetime" | "date" | "time" => {
      let kind = match operator {
        "datetime" => TemporalKind::DateTime,
        "date" => TemporalKind::Date,
        _ => TemporalKind::Time,
      };
      // `format` is optional (spec §4.2: absent means "any ISO-8601 string of this kind"),
      // despite `shape.schema.json` currently marking it required — a schema/prose disagreement
      // worth filing (spec header: "a disagreement between them is a bug to file").
      let format = match obj.get("format") {
        None => None,
        Some(Value::String(format)) => Some(format.clone()),
        Some(_) => {
          problems.push(Problem {
            pointer: format!("{path}/format"),
            message: "'format' must be a string".to_string(),
          });
          None
        }
      };
      Some(CoreShape::Temporal { kind, format })
    }
    "include" => {
      let substring = require_string(obj, "substring", operator, path, problems)?;
      Some(CoreShape::Include { substring })
    }
    "content-type" => {
      let content_type = require_string(obj, "content-type", operator, path, problems)?;
      Some(CoreShape::ContentType { content_type })
    }
    "semver" => Some(CoreShape::Semver),
    "object" => {
      let members_value = require(obj, "members", operator, path, problems)?;
      let Some(members_obj) = members_value.as_object() else {
        problems.push(Problem {
          pointer: format!("{path}/members"),
          message: "'members' must be an object".to_string(),
        });
        return None;
      };
      let mut members = BTreeMap::new();
      for (name, member_value) in members_obj {
        let mut member_path = path.to_string();
        member_path.push_str("/members");
        push_pointer_segment(&mut member_path, name);
        if let Some(node) = parse_child(
          member_value,
          Position::ObjectMember,
          member_path,
          depth + 1,
          budget,
          problems,
        ) {
          members.insert(name.clone(), node);
        }
      }
      Some(CoreShape::Object { members })
    }
    "array" => {
      let entries = parse_entries(obj, operator, "entries", path, depth, budget, problems)?;
      Some(CoreShape::Array { entries })
    }
    "contains" => {
      let entries = parse_entries(obj, operator, "entries", path, depth, budget, problems)?;
      Some(CoreShape::Contains { entries })
    }
    "each-like" => {
      let items_value = require(obj, "items", operator, path, problems)?;
      let items = parse_child(
        items_value,
        Position::NotASlot,
        format!("{path}/items"),
        depth + 1,
        budget,
        problems,
      )?;
      let (min, max) = parse_cardinality(obj, path, problems)?;
      Some(CoreShape::EachLike {
        items: Box::new(items),
        min,
        max,
      })
    }
    "each-entry" => {
      let values_value = require(obj, "values", operator, path, problems)?;
      let values = parse_child(
        values_value,
        Position::NotASlot,
        format!("{path}/values"),
        depth + 1,
        budget,
        problems,
      )?;
      let keys = match obj.get("keys") {
        None => None,
        Some(keys_value) => parse_child(
          keys_value,
          Position::NotASlot,
          format!("{path}/keys"),
          depth + 1,
          budget,
          problems,
        )
        .map(Box::new),
      };
      let (min, max) = parse_cardinality(obj, path, problems)?;
      Some(CoreShape::EachEntry {
        keys,
        values: Box::new(values),
        min,
        max,
      })
    }
    "optional" => {
      let of_value = require(obj, "of", operator, path, problems)?;
      let of = parse_child(
        of_value,
        Position::NotASlot,
        format!("{path}/of"),
        depth + 1,
        budget,
        problems,
      )?;
      Some(CoreShape::Optional { of: Box::new(of) })
    }
    "forbidden" => Some(CoreShape::Forbidden),
    "nullable" => {
      let of_value = require(obj, "of", operator, path, problems)?;
      let of = parse_child(
        of_value,
        Position::NotASlot,
        format!("{path}/of"),
        depth + 1,
        budget,
        problems,
      )?;
      if matches!(of.kind, ShapeKind::Core(CoreShape::Nullable { .. })) {
        problems.push(Problem {
          pointer: format!("{path}/of"),
          message: "'nullable' must not wrap 'nullable' (spec §5.2)".to_string(),
        });
      }
      Some(CoreShape::Nullable { of: Box::new(of) })
    }
    "any-of" => parse_any_of(obj, example, path, problems),
    "one-of" => parse_one_of(obj, path, depth, budget, problems),
    _ => unreachable!("operator '{operator}' is in CORE_OPERATORS but not handled"),
  }
}

fn require<'a>(
  obj: &'a Map<String, Value>,
  member: &str,
  operator: &str,
  path: &str,
  problems: &mut Vec<Problem>,
) -> Option<&'a Value> {
  match obj.get(member) {
    Some(value) => Some(value),
    None => {
      problems.push(Problem {
        pointer: path.to_string(),
        message: format!("'{operator}' requires '{member}'"),
      });
      None
    }
  }
}

fn require_string(
  obj: &Map<String, Value>,
  member: &str,
  operator: &str,
  path: &str,
  problems: &mut Vec<Problem>,
) -> Option<String> {
  match require(obj, member, operator, path, problems)? {
    Value::String(value) => Some(value.clone()),
    _ => {
      problems.push(Problem {
        pointer: format!("{path}/{member}"),
        message: format!("'{member}' must be a string"),
      });
      None
    }
  }
}

fn require_example(operator: &str, example: Option<&Example>, path: &str, problems: &mut Vec<Problem>) {
  if example.is_none() {
    problems.push(Problem {
      pointer: path.to_string(),
      message: format!("'{operator}' requires 'example'"),
    });
  }
}

fn parse_entries(
  obj: &Map<String, Value>,
  operator: &str,
  member: &str,
  path: &str,
  depth: usize,
  budget: &mut Budget,
  problems: &mut Vec<Problem>,
) -> Option<Vec<ShapeNode>> {
  let entries_value = require(obj, member, operator, path, problems)?;
  let Some(array) = entries_value.as_array() else {
    problems.push(Problem {
      pointer: format!("{path}/{member}"),
      message: format!("'{member}' must be an array"),
    });
    return None;
  };
  let mut entries = Vec::new();
  for (index, entry_value) in array.iter().enumerate() {
    if let Some(node) = parse_child(
      entry_value,
      Position::NotASlot,
      format!("{path}/{member}/{index}"),
      depth + 1,
      budget,
      problems,
    ) {
      entries.push(node);
    }
  }
  Some(entries)
}

/// `min`/`max` cardinality (spec §5.5): `min` defaults to 1, `max` absent means unbounded, and
/// `min <= max` when both are present.
fn parse_cardinality(
  obj: &Map<String, Value>,
  path: &str,
  problems: &mut Vec<Problem>,
) -> Option<(u64, Option<u64>)> {
  let min = match obj.get("min") {
    None => 1,
    Some(value) => match value.as_u64() {
      Some(min) => min,
      None => {
        problems.push(Problem {
          pointer: format!("{path}/min"),
          message: "'min' must be a non-negative integer".to_string(),
        });
        return None;
      }
    },
  };
  let max = match obj.get("max") {
    None => None,
    Some(value) => match value.as_u64() {
      Some(max) => Some(max),
      None => {
        problems.push(Problem {
          pointer: format!("{path}/max"),
          message: "'max' must be a non-negative integer".to_string(),
        });
        return None;
      }
    },
  };
  if let Some(max) = max
    && min > max
  {
    problems.push(Problem {
      pointer: path.to_string(),
      message: format!("'min' ({min}) must not be greater than 'max' ({max}) (spec §5.5)"),
    });
    return None;
  }
  Some((min, max))
}

/// `any-of` (spec §5.3): `options` non-empty and distinct, `example` (if present) one of them.
fn parse_any_of(
  obj: &Map<String, Value>,
  example: Option<&Example>,
  path: &str,
  problems: &mut Vec<Problem>,
) -> Option<CoreShape> {
  let options_value = require(obj, "options", "any-of", path, problems)?;
  let Some(array) = options_value.as_array() else {
    problems.push(Problem {
      pointer: format!("{path}/options"),
      message: "'options' must be an array".to_string(),
    });
    return None;
  };
  if array.is_empty() {
    problems.push(Problem {
      pointer: format!("{path}/options"),
      message: "'options' must be non-empty (spec §5.3)".to_string(),
    });
    return None;
  }
  for (index, option) in array.iter().enumerate() {
    for other in &array[..index] {
      if values_equal(option, other) {
        problems.push(Problem {
          pointer: format!("{path}/options/{index}"),
          message: "'options' must be distinct (spec §5.3)".to_string(),
        });
        break;
      }
    }
  }
  if let Some(example) = example
    && !array.iter().any(|option| values_equal(option, &example.value))
  {
    problems.push(Problem {
      pointer: format!("{path}/example"),
      message: "'example' is not among 'options' (spec §5.3)".to_string(),
    });
  }
  Some(CoreShape::AnyOf {
    options: array.clone(),
  })
}

/// `one-of` (spec §5.4): at least two alternatives, each an `object` shape (optionally wrapped
/// in `nullable`) binding `discriminator` to an `equality` or `any-of`, with pairwise-disjoint
/// discriminator value sets.
fn parse_one_of(
  obj: &Map<String, Value>,
  path: &str,
  depth: usize,
  budget: &mut Budget,
  problems: &mut Vec<Problem>,
) -> Option<CoreShape> {
  let discriminator = require_string(obj, "discriminator", "one-of", path, problems)?;
  let alternatives_value = require(obj, "alternatives", "one-of", path, problems)?;
  let Some(alternatives_obj) = alternatives_value.as_object() else {
    problems.push(Problem {
      pointer: format!("{path}/alternatives"),
      message: "'alternatives' must be an object".to_string(),
    });
    return None;
  };
  if alternatives_obj.len() < 2 {
    problems.push(Problem {
      pointer: format!("{path}/alternatives"),
      message: "'one-of' requires at least two alternatives (spec §5.4)".to_string(),
    });
  }

  let mut alternatives = BTreeMap::new();
  let mut discriminator_sets: Vec<(String, Vec<Value>)> = Vec::new();
  for (name, alt_value) in alternatives_obj {
    let mut alt_path = path.to_string();
    alt_path.push_str("/alternatives");
    push_pointer_segment(&mut alt_path, name);
    let Some(node) = parse_child(
      alt_value,
      Position::NotASlot,
      alt_path.clone(),
      depth + 1,
      budget,
      problems,
    ) else {
      continue;
    };
    if let Some(options) = discriminator_options(&node, &discriminator, &alt_path, name, problems) {
      discriminator_sets.push((name.clone(), options));
    }
    alternatives.insert(name.clone(), node);
  }

  for i in 1..discriminator_sets.len() {
    let (name_i, options_i) = &discriminator_sets[i];
    for (name_j, options_j) in &discriminator_sets[..i] {
      if options_i
        .iter()
        .any(|a| options_j.iter().any(|b| values_equal(a, b)))
      {
        problems.push(Problem {
          pointer: format!("{path}/alternatives"),
          message: format!(
            "alternatives '{name_j}' and '{name_i}' do not have disjoint discriminator values (spec §5.4)"
          ),
        });
      }
    }
  }

  let default = match obj.get("default") {
    None => None,
    Some(Value::String(name)) => {
      if !alternatives.contains_key(name) {
        problems.push(Problem {
          pointer: format!("{path}/default"),
          message: format!("'default' names an alternative that does not exist: '{name}'"),
        });
      }
      Some(name.clone())
    }
    Some(_) => {
      problems.push(Problem {
        pointer: format!("{path}/default"),
        message: "'default' must be a string".to_string(),
      });
      None
    }
  };

  Some(CoreShape::OneOf {
    discriminator,
    alternatives,
    default,
  })
}

/// The literal value set an alternative's `discriminator` member binds to, or `None` (with a
/// problem already recorded) if the alternative is not shaped the way spec §5.4 requires.
fn discriminator_options(
  node: &ShapeNode,
  discriminator: &str,
  alt_path: &str,
  alt_name: &str,
  problems: &mut Vec<Problem>,
) -> Option<Vec<Value>> {
  let Some(members) = unwrap_object_members(node) else {
    problems.push(Problem {
      pointer: alt_path.to_string(),
      message: format!(
        "alternative '{alt_name}' must be an 'object' shape, optionally wrapped in 'nullable' (spec §5.4)"
      ),
    });
    return None;
  };
  let Some(discriminator_node) = members.get(discriminator) else {
    problems.push(Problem {
      pointer: alt_path.to_string(),
      message: format!(
        "alternative '{alt_name}' does not declare the discriminator member '{discriminator}' (spec §5.4)"
      ),
    });
    return None;
  };
  match &discriminator_node.kind {
    ShapeKind::Core(CoreShape::Equality) => discriminator_node
      .example
      .as_ref()
      .map(|example| vec![example.value.clone()]),
    ShapeKind::Core(CoreShape::AnyOf { options }) => Some(options.clone()),
    _ => {
      let mut member_path = alt_path.to_string();
      member_path.push_str("/members");
      push_pointer_segment(&mut member_path, discriminator);
      problems.push(Problem {
        pointer: member_path,
        message: format!(
          "the discriminator member of alternative '{alt_name}' must be 'equality' or 'any-of', not '{}' (spec §5.4)",
          discriminator_node.operator()
        ),
      });
      None
    }
  }
}

/// The members of an `object` node, or of the `object` a `nullable` wraps — the shapes spec
/// §5.4 accepts as a `one-of` alternative — or `None` for anything else.
fn unwrap_object_members(node: &ShapeNode) -> Option<&BTreeMap<String, ShapeNode>> {
  match &node.kind {
    ShapeKind::Core(CoreShape::Object { members }) => Some(members),
    ShapeKind::Core(CoreShape::Nullable { of }) => match &of.kind {
      ShapeKind::Core(CoreShape::Object { members }) => Some(members),
      _ => None,
    },
    _ => None,
  }
}

/// Structural equality per shape-language spec §4.2's `equality` semantics: numbers compare
/// numerically (`1` and `1.0` are the same value), not by JSON lexical form.
pub(crate) fn values_equal(a: &Value, b: &Value) -> bool {
  match (a, b) {
    (Value::Number(a), Value::Number(b)) => a.as_f64() == b.as_f64(),
    (Value::Array(a), Value::Array(b)) => {
      a.len() == b.len() && a.iter().zip(b).all(|(a, b)| values_equal(a, b))
    }
    (Value::Object(a), Value::Object(b)) => {
      a.len() == b.len()
        && a
          .iter()
          .all(|(key, value)| b.get(key).is_some_and(|other| values_equal(value, other)))
    }
    _ => a == b,
  }
}
