//! The plan compiler (plan task 3.3, plan-grammar spec §5): shapes compile to plan structure,
//! operator by operator, per spec §5.2's table. Every core operator of the shape language
//! (design 2.2) is handled; a namespaced component operator compiles to a namespaced action and
//! is otherwise opaque, per shape spec §3.5's "the kernel treats them opaquely and MUST NOT
//! guess".
//!
//! **Pinning.** An [`Assignment`] is a partial map from dimension id (shape spec §6.2) to the
//! name of the point selected for it. A dimension the assignment does not mention compiles to its
//! general, branching form (spec §5.2's table); a dimension it does mention compiles to the
//! narrower form spec §5.1 and §7.1 describe. A point name that does not match any point the
//! operator actually has is treated the same as an absent entry — the general form — rather than
//! as an error: validating that an assignment names real dimensions and points is design 2.3's
//! business (the layer that actually enumerates them), not this compiler's, and a lenient
//! fallback here means a bad variant id degrades to "compiled unpinned" instead of a panic.

use super::model::{Literal, Node};
use crate::interaction_spec::InteractionSpec;
use crate::shape::variant_space::{VariantSpace, canonical_point_name, cardinality_points, compute_many};
use crate::shape::{CoreShape, Example, ShapeKind, ShapeNode, path};
use serde_json::Value;
use std::collections::BTreeMap;

/// Dimension id (shape spec §6.2) -> the name of the point pinned for it. Design 2.3 owns the
/// full variant-selection document (naming, sampling, provider-state linkage); this is the
/// minimal slice the compiler needs to narrow a shape (plan-grammar spec §5.1).
pub type Assignment = BTreeMap<String, String>;

/// Compile a whole interaction specification to a plan (plan-grammar spec §5), optionally under a
/// variant assignment. `variant_id` is stamped onto the result verbatim (design 2.3 names it);
/// pass `None` to compile the shape unpinned.
pub fn compile(
  spec: &InteractionSpec,
  assignment: &Assignment,
  variant_id: Option<&str>,
) -> super::model::Plan {
  let mut part_nodes = Vec::new();
  for (part_name, part) in &spec.parts {
    let mut slot_nodes = Vec::new();
    for (slot_name, shape) in part {
      let root = path::root(part_name, slot_name);
      let ctx = Ctx {
        cursor: Cursor::Absolute(format!("$.{root}")),
        at: root,
        assignment,
      };
      let children = compile_shape(shape, &ctx);
      slot_nodes.push(Node::container(Some(slot_name.clone()), children));
    }
    part_nodes.push(Node::container(Some(part_name.clone()), slot_nodes));
  }
  super::model::Plan {
    grammar: super::model::GRAMMAR_VERSION,
    root: Node::container(Some(spec.description.clone()), part_nodes),
    variant: variant_id.map(str::to_string),
  }
}

/// The variant space of a whole interaction specification: every part's every slot, in name
/// order, concatenated (shape spec §6.6) — the document design 2.3 (and, downstream, phase 4's
/// variant machinery) enumerates to select and sample variants.
pub fn variant_space(spec: &InteractionSpec) -> VariantSpace {
  let shapes = spec.parts.iter().flat_map(|(part, slots)| {
    slots
      .iter()
      .map(move |(slot, shape)| (path::root(part, slot), shape))
  });
  compute_many(shapes)
}

/// Where a compiled node resolves its value from: the interaction context (absolute), or the
/// current item on the iteration stack (relative — spec §2.1's `resolve-current`). Both variants
/// hold the path text ready to print (plan-grammar spec §3.1's sigils are part of the text, not a
/// separate rendering concern here).
#[derive(Debug, Clone)]
enum Cursor {
  Absolute(String),
  Relative(String),
}

impl Cursor {
  fn member(&self, name: &str) -> Cursor {
    match self {
      Cursor::Absolute(p) => Cursor::Absolute(format!("{p}.{name}")),
      Cursor::Relative(p) => Cursor::Relative(format!("{p}.{name}")),
    }
  }

  fn array_index(&self, index: usize) -> Cursor {
    match self {
      Cursor::Absolute(p) => Cursor::Absolute(format!("{p}[{index}]")),
      Cursor::Relative(p) => Cursor::Relative(format!("{p}[{index}]")),
    }
  }

  fn resolve(&self) -> Node {
    match self {
      Cursor::Absolute(p) => Node::resolve(p.clone()),
      Cursor::Relative(p) => Node::resolve_current(p.clone()),
    }
  }
}

/// The compiler's running position: where to resolve a value from ([`Cursor`]), where the tree is
/// dimensionally ([`path`]'s syntax, for dimension ids and container labels) and the assignment
/// pinning this compilation. Bundled together because every structural descent updates all three
/// in lockstep.
struct Ctx<'a> {
  cursor: Cursor,
  at: String,
  assignment: &'a Assignment,
}

impl<'a> Ctx<'a> {
  fn member(&self, name: &str) -> Ctx<'a> {
    Ctx {
      cursor: self.cursor.member(name),
      at: path::member(&self.at, name),
      assignment: self.assignment,
    }
  }

  fn array_index(&self, index: usize) -> Ctx<'a> {
    Ctx {
      cursor: self.cursor.array_index(index),
      at: path::array_index(&self.at, index),
      assignment: self.assignment,
    }
  }

  /// The item context inside an `each-like`/`contains`: a fresh relative cursor (`~>`), because a
  /// splat's expansion is what makes it current, not anything the enclosing cursor knew.
  fn iteration_item(&self, at: String) -> Ctx<'a> {
    Ctx {
      cursor: Cursor::Relative("~>".to_string()),
      at,
      assignment: self.assignment,
    }
  }

  fn alternative(&self, name: &str) -> Ctx<'a> {
    Ctx {
      cursor: self.cursor.clone(),
      at: path::alternative(&self.at, name),
      assignment: self.assignment,
    }
  }

  fn resolve(&self) -> Node {
    self.cursor.resolve()
  }

  fn dim_id(&self, facet: &str) -> String {
    path::dimension_id(&self.at, facet)
  }

  /// The label a container at this position prints. Deliberately the full dimensional path
  /// (`"$.response.body.items[*].sku"`) rather than the shorter form the plan-grammar spec's
  /// worked examples show relative to an ancestor label — that shortening is a rendering nicety
  /// task 3.6 owns; every label here is unambiguous and traceable on its own.
  fn label(&self) -> String {
    format!("$.{}", self.at)
  }
}

fn compile_shape(shape: &ShapeNode, ctx: &Ctx) -> Vec<Node> {
  match &shape.kind {
    ShapeKind::Component { operator, raw } => vec![compile_component(operator, raw, ctx)],
    ShapeKind::Core(core) => compile_core(core, shape.example.as_ref(), ctx),
  }
}

fn compile_core(core: &CoreShape, example: Option<&Example>, ctx: &Ctx) -> Vec<Node> {
  match core {
    CoreShape::Any => vec![Node::action("match:any", vec![ctx.resolve()])],
    CoreShape::Equality => vec![Node::action(
      "match:equality",
      vec![ctx.resolve(), Node::value(example_literal(example))],
    )],
    CoreShape::Type => vec![Node::action(
      "match:type",
      vec![ctx.resolve(), Node::value(example_literal(example))],
    )],
    CoreShape::Kind(kind) => vec![Node::action(
      format!("match:{}", kind.operator_name()),
      vec![ctx.resolve()],
    )],
    CoreShape::NotEmpty => vec![Node::action("expect:not-empty", vec![ctx.resolve()])],
    CoreShape::Regex { pattern } => vec![Node::action(
      "match:regex",
      vec![ctx.resolve(), Node::value(Literal::string(pattern))],
    )],
    CoreShape::Temporal { kind, format } => {
      let mut children = vec![ctx.resolve()];
      if let Some(format) = format {
        children.push(Node::value(Literal::string(format)));
      }
      vec![Node::action(format!("match:{}", kind.operator_name()), children)]
    }
    CoreShape::Include { substring } => vec![Node::action(
      "match:include",
      vec![ctx.resolve(), Node::value(Literal::string(substring))],
    )],
    CoreShape::ContentType { content_type } => vec![Node::action(
      "match:content-type",
      vec![ctx.resolve(), Node::value(Literal::string(content_type))],
    )],
    CoreShape::Semver => vec![Node::action("match:semver", vec![ctx.resolve()])],
    CoreShape::Object { members } => compile_object(members, ctx),
    CoreShape::Array { entries } => compile_array(entries, ctx),
    CoreShape::EachLike { items, min, max } => compile_each_like(items, *min, *max, ctx),
    CoreShape::EachEntry {
      keys,
      values,
      min,
      max,
    } => compile_each_entry(keys.as_deref(), values, *min, *max, ctx),
    CoreShape::Contains { entries } => vec![compile_contains(entries, ctx)],
    CoreShape::Optional { of } => vec![compile_optional(of, ctx)],
    CoreShape::Forbidden => vec![Node::action("expect:absent", vec![ctx.resolve()])],
    CoreShape::Nullable { of } => vec![compile_nullable(of, ctx)],
    CoreShape::AnyOf { options } => vec![compile_any_of(options, ctx)],
    CoreShape::OneOf {
      discriminator,
      alternatives,
      ..
    } => vec![compile_one_of(discriminator, alternatives, ctx)],
  }
}

/// `object` (spec §5.2): a container per named member; nothing about unnamed ones (the
/// must-ignore default, spec §4.3, is this absence — no assertion is emitted for a member nobody
/// named).
fn compile_object(members: &BTreeMap<String, ShapeNode>, ctx: &Ctx) -> Vec<Node> {
  members
    .iter()
    .map(|(name, member)| {
      let member_ctx = ctx.member(name);
      let children = compile_shape(member, &member_ctx);
      Node::container(Some(member_ctx.label()), children)
    })
    .collect()
}

/// `array` (spec §5.2): `expect:count` for the fixed length, plus a container per index — extra
/// elements are not ignored (spec §4.3's deliberate asymmetry with `object`), which is exactly
/// what asserting the count achieves.
fn compile_array(entries: &[ShapeNode], ctx: &Ctx) -> Vec<Node> {
  let mut nodes = vec![Node::action(
    "expect:count",
    vec![ctx.resolve(), Node::value(Literal::number(entries.len() as u64))],
  )];
  for (index, entry) in entries.iter().enumerate() {
    let entry_ctx = ctx.array_index(index);
    let children = compile_shape(entry, &entry_ctx);
    nodes.push(Node::container(Some(entry_ctx.label()), children));
  }
  nodes
}

/// `each-like` (spec §5.2): `expect:size` for the cardinality (narrowed to `expect:count` when
/// the cardinality dimension is pinned), plus `for-each` over a `splat` of the elements, the item
/// shape compiled once against `resolve-current`.
fn compile_each_like(items: &ShapeNode, min: u64, max: Option<u64>, ctx: &Ctx) -> Vec<Node> {
  let item_ctx = ctx.iteration_item(path::each_like_item(&ctx.at));
  let item_node = Node::container(Some(item_ctx.label()), compile_shape(items, &item_ctx));
  let for_each = Node::action("for-each", vec![Node::splat(vec![ctx.resolve()]), item_node]);
  let size_assertion = cardinality_assertion(ctx, min, max);
  vec![size_assertion, for_each]
}

/// `each-entry` (spec §5.2): the same over entries, with the key shape (if any) and the value
/// shape each checked against the current entry's `.key`/`.value` (spec §2.2's `entry` value
/// kind is what a `resolve-current` here is understood to address).
fn compile_each_entry(
  keys: Option<&ShapeNode>,
  values: &ShapeNode,
  min: u64,
  max: Option<u64>,
  ctx: &Ctx,
) -> Vec<Node> {
  let mut item_children = Vec::new();
  if let Some(keys) = keys {
    let key_ctx = ctx.iteration_item(path::each_entry_key(&ctx.at));
    let key_ctx = Ctx {
      cursor: Cursor::Relative("~>.key".to_string()),
      ..key_ctx
    };
    item_children.push(Node::container(
      Some(key_ctx.label()),
      compile_shape(keys, &key_ctx),
    ));
  }
  let value_ctx = ctx.iteration_item(path::each_entry_value(&ctx.at));
  let value_ctx = Ctx {
    cursor: Cursor::Relative("~>.value".to_string()),
    ..value_ctx
  };
  item_children.push(Node::container(
    Some(value_ctx.label()),
    compile_shape(values, &value_ctx),
  ));

  let item_node = Node::container(Some(ctx.label()), item_children);
  let for_each = Node::action("for-each", vec![Node::splat(vec![ctx.resolve()]), item_node]);
  let size_assertion = cardinality_assertion(ctx, min, max);
  vec![size_assertion, for_each]
}

fn cardinality_assertion(ctx: &Ctx, min: u64, max: Option<u64>) -> Node {
  let dim_id = ctx.dim_id("cardinality");
  match pinned_cardinality(&dim_id, min, max, ctx.assignment) {
    Some(size) => Node::action(
      "expect:count",
      vec![ctx.resolve(), Node::value(Literal::number(size))],
    ),
    None => Node::action(
      "expect:size",
      vec![
        ctx.resolve(),
        Node::value(Literal::number(min)),
        Node::value(max.map(Literal::number).unwrap_or_else(Literal::null)),
      ],
    ),
  }
}

fn pinned_cardinality(dim_id: &str, min: u64, max: Option<u64>, assignment: &Assignment) -> Option<u64> {
  let point_name = assignment.get(dim_id)?;
  cardinality_points(min, max)
    .into_iter()
    .find(|p| &p.name == point_name)
    .map(|p| p.size)
}

/// `contains` (spec §5.2): opaque — each entry shape is checked existentially against the array,
/// which this compiler represents with a relative cursor per entry rather than an addressed
/// position (there is none to give it; spec §8 already marks `contains`'s comparability opaque).
fn compile_contains(entries: &[ShapeNode], ctx: &Ctx) -> Node {
  let mut children = vec![ctx.resolve()];
  for (index, entry) in entries.iter().enumerate() {
    let entry_ctx = ctx.iteration_item(path::array_index(&ctx.at, index));
    children.push(Node::container(
      Some(entry_ctx.label()),
      compile_shape(entry, &entry_ctx),
    ));
  }
  Node::action("match:contains", children)
}

/// `optional` (spec §5.2): `if` on `check:exists`; the present branch compiles `of` (spliced
/// directly, without a wrapping container, when it is already one node — spec's worked example
/// shows exactly this: `%if(%check:exists(...), %match:datetime(...))`); pinning to `absent`
/// narrows to `expect:absent`, pinning to `present` narrows to `of` with no branch at all
/// (spec §7.1: the plan is narrower because the shape is).
fn compile_optional(of: &ShapeNode, ctx: &Ctx) -> Node {
  let dim_id = ctx.dim_id("presence");
  match ctx.assignment.get(&dim_id).map(String::as_str) {
    Some("absent") => Node::action("expect:absent", vec![ctx.resolve()]),
    Some("present") => as_single_unlabeled(compile_shape(of, ctx)),
    _ => Node::action(
      "if",
      vec![
        Node::action("check:exists", vec![ctx.resolve()]),
        as_single_unlabeled(compile_shape(of, ctx)),
      ],
    ),
  }
}

/// `nullable` (spec §5.2): `if` on `check:null`; the null branch is an explicit no-op (`and`
/// with no children — the control family's identity element, used here because the `if` grammar
/// needs a positional branch node and this shape has nothing to check once the value is null),
/// the non-null branch compiles `of`. Pinning to `null` narrows to `match:null`; pinning to
/// `non-null` narrows to `of` with no branch.
fn compile_nullable(of: &ShapeNode, ctx: &Ctx) -> Node {
  let dim_id = ctx.dim_id("nullability");
  match ctx.assignment.get(&dim_id).map(String::as_str) {
    Some("null") => Node::action("match:null", vec![ctx.resolve()]),
    Some("non-null") => as_single_unlabeled(compile_shape(of, ctx)),
    _ => Node::action(
      "if",
      vec![
        Node::action("check:null", vec![ctx.resolve()]),
        ok_node(),
        as_single_unlabeled(compile_shape(of, ctx)),
      ],
    ),
  }
}

/// `any-of` (spec §5.2): `match:any-of` over the literal options; pinned, it narrows to the
/// selected literal's `match:equality`.
fn compile_any_of(options: &[Value], ctx: &Ctx) -> Node {
  let dim_id = ctx.dim_id("value");
  let pinned = ctx
    .assignment
    .get(&dim_id)
    .and_then(|point_name| options.iter().find(|o| canonical_point_name(o) == *point_name));
  if let Some(pinned) = pinned {
    return Node::action(
      "match:equality",
      vec![ctx.resolve(), Node::value(Literal::from_json(pinned))],
    );
  }
  let mut children = vec![ctx.resolve()];
  children.extend(options.iter().map(|o| Node::value(Literal::from_json(o))));
  Node::action("match:any-of", children)
}

/// `one-of` (spec §5.2): nested `if`/`check:equals` (or `or` of them, when an alternative's
/// discriminator is an `any-of` rather than a bare literal) per alternative, ending in an `error`
/// naming the discriminator value read; pinned, it narrows straight to the selected alternative's
/// compiled object — including its own discriminator member, which is a harmless redundancy
/// (rather than stripping it) that keeps this compiler agnostic to whether an alternative is
/// wrapped in `nullable` (spec §5.4).
fn compile_one_of(discriminator: &str, alternatives: &BTreeMap<String, ShapeNode>, ctx: &Ctx) -> Node {
  let dim_id = ctx.dim_id("alternative");
  let pinned = ctx.assignment.get(&dim_id).and_then(|point_name| {
    alternatives
      .get(point_name)
      .map(|shape| (point_name.clone(), shape))
  });
  if let Some((point_name, alt_shape)) = pinned {
    let alt_ctx = ctx.alternative(&point_name);
    return Node::container(Some(point_name), compile_shape(alt_shape, &alt_ctx));
  }

  let discriminator_cursor = ctx.cursor.member(discriminator);
  let mut chain = one_of_error_branch(discriminator, alternatives.keys(), &discriminator_cursor);
  for (alt_name, alt_shape) in alternatives.iter().rev() {
    let alt_ctx = ctx.alternative(alt_name);
    let condition = discriminator_condition(alt_shape, discriminator, &discriminator_cursor);
    let branch = Node::container(Some(alt_name.clone()), compile_shape(alt_shape, &alt_ctx));
    chain = Node::action("if", vec![condition, branch, chain]);
  }
  chain
}

fn discriminator_literals(alt_shape: &ShapeNode, discriminator: &str) -> Vec<Value> {
  let members = match &alt_shape.kind {
    ShapeKind::Core(CoreShape::Object { members }) => members,
    ShapeKind::Core(CoreShape::Nullable { of }) => match &of.kind {
      ShapeKind::Core(CoreShape::Object { members }) => members,
      _ => return Vec::new(),
    },
    _ => return Vec::new(),
  };
  match members.get(discriminator).map(|m| &m.kind) {
    Some(ShapeKind::Core(CoreShape::Equality)) => members[discriminator]
      .example
      .as_ref()
      .map(|e| vec![e.value.clone()])
      .unwrap_or_default(),
    Some(ShapeKind::Core(CoreShape::AnyOf { options })) => options.clone(),
    _ => Vec::new(),
  }
}

fn discriminator_condition(
  alt_shape: &ShapeNode,
  discriminator: &str,
  discriminator_cursor: &Cursor,
) -> Node {
  let literals = discriminator_literals(alt_shape, discriminator);
  let checks: Vec<Node> = literals
    .iter()
    .map(|lit| {
      Node::action(
        "check:equals",
        vec![
          discriminator_cursor.resolve(),
          Node::value(Literal::from_json(lit)),
        ],
      )
    })
    .collect();
  match checks.len() {
    // Defensive only: well-formedness (spec §5.4) guarantees the discriminator names a literal or
    // an any-of, so this arm should be unreachable on validated input.
    0 => Node::action(
      "check:equals",
      vec![discriminator_cursor.resolve(), Node::value(Literal::null())],
    ),
    1 => checks.into_iter().next().expect("checked len == 1"),
    _ => Node::action("or", checks),
  }
}

fn one_of_error_branch<'a>(
  discriminator: &str,
  alt_names: impl Iterator<Item = &'a String>,
  discriminator_cursor: &Cursor,
) -> Node {
  let names: Vec<&str> = alt_names.map(String::as_str).collect();
  let message = format!(
    "Expected {discriminator} to be one of {} but got ",
    names.join(", ")
  );
  Node::action(
    "error",
    vec![Node::action(
      "join",
      vec![
        Node::value(Literal::string(message)),
        discriminator_cursor.resolve(),
      ],
    )],
  )
}

/// A component operator (shape spec §3.5): namespaced action, resolved value plus the operator's
/// raw configuration passed through unexamined — the kernel "treats them opaquely and MUST NOT
/// guess" what a component's members mean.
fn compile_component(operator: &str, raw: &BTreeMap<String, Value>, ctx: &Ctx) -> Node {
  let config = Value::Object(raw.iter().map(|(k, v)| (k.clone(), v.clone())).collect());
  Node::action(
    operator.to_string(),
    vec![ctx.resolve(), Node::value(Literal::from_json(&config))],
  )
}

/// Collapse to the single node when there is one; otherwise bundle into an unlabelled container.
/// Used exactly where a branch sits at the *same* dimensional path as its parent (`optional`'s
/// present branch, `nullable`'s non-null branch) and so has no label of its own to carry — unlike
/// an object member, an array index, an each-like item or a one-of alternative, which always get
/// [`Node::container`] with their own label regardless of child count.
fn as_single_unlabeled(mut nodes: Vec<Node>) -> Node {
  if nodes.len() == 1 {
    nodes.pop().expect("checked len == 1")
  } else {
    Node::container(None, nodes)
  }
}

/// The control family's identity element: an empty conjunction. Stands in for a positional `if`
/// branch that this shape's narrowing makes trivially true (`nullable`'s null case).
fn ok_node() -> Node {
  Node::action("and", vec![])
}

/// Well-formedness (shape spec §4.2) makes `example` required for `equality`/`type`; this
/// compiler assumes well-formed input (shape::parse's job) and falls back to `null` rather than
/// panicking if it is ever handed something else.
fn example_literal(example: Option<&Example>) -> Literal {
  example.map(Literal::from_example).unwrap_or_else(Literal::null)
}
