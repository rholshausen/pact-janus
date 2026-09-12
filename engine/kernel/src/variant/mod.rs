//! Variant machinery (plan task 4.3, variant-semantics spec — design 2.3): turning a compiled
//! interaction's variant space (shape spec §6, [`crate::shape::variant_space`]) into a selected,
//! deterministic sample — the `janus-ipog-v1` algorithm of spec §3.4 — plus the concrete payload
//! each selected variant produces ([`generate`]).
//!
//! What is deliberately **not** here: variant-bound provider state (spec §6, `whenVariant`) is
//! plan task 5.2's; actually driving a transport from an armed variant (spec §4.1) is wherever
//! `start-transport` gets wired into [`crate::protocol`] (a gap this module does not close, since
//! no transport is bound to a consumer session yet).

pub mod generate;

use crate::error::Problem;
use crate::plan::Assignment;
use crate::shape::variant_space::{Dimension, VariantSpace};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet};

// ---------------------------------------------------------------------------------------------
// Dimension-reference resolution and labels (spec §2.2, §6.3): shared path-segment machinery.
// ---------------------------------------------------------------------------------------------

/// Split a dimension path into tokens, each carrying its own leading delimiter (shape spec
/// §6.2's path syntax) except the very first: `"response.body.items[*]"` ->
/// `["response", ".body", ".items", "[*]"]`. The single source of truth for both a trailing-run
/// reference match (spec §6.3) and a shortened display label (spec §2.2).
fn path_tokens(path: &str) -> Vec<String> {
  let mut tokens = Vec::new();
  let mut current = String::new();
  for ch in path.chars() {
    if matches!(ch, '.' | '[' | '{' | '@') && !current.is_empty() {
      tokens.push(std::mem::take(&mut current));
    }
    current.push(ch);
  }
  if !current.is_empty() {
    tokens.push(current);
  }
  tokens
}

/// The text of the last `k` tokens (or all of them, if fewer), joined back together.
fn suffix_text(tokens: &[String], k: usize) -> String {
  let start = tokens.len().saturating_sub(k);
  tokens[start..].concat()
}

/// A `.` joiner is an artefact of concatenation, not part of what a reader typed; `[`, `{`, `@`
/// read fine standing alone.
fn strip_leading_dot(s: &str) -> String {
  s.strip_prefix('.')
    .map(str::to_string)
    .unwrap_or_else(|| s.to_string())
}

pub(crate) enum RefResolution<'a> {
  NotFound,
  Ambiguous(Vec<String>),
  Found(&'a Dimension),
}

/// Resolve a dimension reference (spec §6.3): the dimension's full id, its full path, or a
/// trailing run of whole path segments of it.
pub(crate) fn resolve_dimension_ref<'a>(space: &'a VariantSpace, reference: &str) -> RefResolution<'a> {
  let mut found: Vec<&Dimension> = Vec::new();
  for dim in &space.dimensions {
    if dim.id == reference || dim.path == reference {
      found.push(dim);
      continue;
    }
    let tokens = path_tokens(&dim.path);
    let hit = (1..=tokens.len()).any(|k| strip_leading_dot(&suffix_text(&tokens, k)) == reference);
    if hit {
      found.push(dim);
    }
  }
  match found.len() {
    0 => RefResolution::NotFound,
    1 => RefResolution::Found(found[0]),
    _ => RefResolution::Ambiguous(found.into_iter().map(|d| d.id.clone()).collect()),
  }
}

/// The shortest trailing path segments unique within `space`, plus the facet where the path
/// alone is ambiguous (spec §2.2). Display only — never recorded, never accepted as input.
fn dimension_label(dim: &Dimension, space: &VariantSpace) -> String {
  let tokens = path_tokens(&dim.path);
  let max_k = tokens.len().max(1);
  for k in 1..=max_k {
    let candidate = strip_leading_dot(&suffix_text(&tokens, k));
    let collides = space.dimensions.iter().any(|other| {
      if other.id == dim.id {
        return false;
      }
      let theirs = path_tokens(&other.path);
      strip_leading_dot(&suffix_text(&theirs, k.min(theirs.len().max(1)))) == candidate
    });
    if !collides {
      return candidate;
    }
    if k == max_k {
      return format!("{candidate}#{}", dim.facet);
    }
  }
  dim.id.clone()
}

fn compute_labels(space: &VariantSpace) -> HashMap<String, String> {
  space
    .dimensions
    .iter()
    .map(|d| (d.id.clone(), dimension_label(d, space)))
    .collect()
}

/// Render an assignment the way spec §2.2 names a variant: the dimensions whose point differs
/// from their default, in variant-space order, `<name>=<point>` joined with `;`; `"base"` when
/// none do. `names` supplies either dimension ids (for the recorded `id`) or labels (for
/// display).
fn render_assignment(
  space: &VariantSpace,
  assignment: &Assignment,
  names: &HashMap<String, String>,
) -> String {
  let mut parts = Vec::new();
  for dim in &space.dimensions {
    if let Some(point) = assignment.get(&dim.id)
      && point != &dim.default
    {
      parts.push(format!("{}={}", names[&dim.id], point));
    }
  }
  if parts.is_empty() {
    "base".to_string()
  } else {
    parts.join(";")
  }
}

// ---------------------------------------------------------------------------------------------
// The sampling policy document (spec §3.8, `schemas/v1/sampling-policy.schema.json`).
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct PointRef {
  pub dimension: String,
  pub point: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Pin {
  pub assignment: Vec<PointRef>,
  #[serde(default)]
  pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Exclusion {
  pub when: Vec<PointRef>,
  pub reason: String,
}

/// One layer of policy (spec §3.8): every member overrides except `pin`/`exclude`, which
/// accumulate — so a layer is parsed as an all-optional patch rather than a full policy.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
struct PolicyPatch {
  strategy: Option<String>,
  strength: Option<u32>,
  exhaustive_threshold: Option<u64>,
  max_variants: Option<u64>,
  boundaries: Option<bool>,
  algorithm: Option<String>,
  pin: Vec<Pin>,
  exclude: Vec<Exclusion>,
}

/// A resolved sampling policy (spec §3.8): the defaults, with every layer's overrides applied in
/// order and `pin`/`exclude` accumulated across all of them.
#[derive(Debug, Clone, PartialEq)]
pub struct SamplingPolicy {
  pub strategy: String,
  pub strength: u32,
  pub exhaustive_threshold: u64,
  pub max_variants: u64,
  pub boundaries: bool,
  pub algorithm: String,
  pub pin: Vec<Pin>,
  pub exclude: Vec<Exclusion>,
}

impl Default for SamplingPolicy {
  /// Spec §3.2/§3.6's stated defaults: `auto`, pairwise (strength 2), an exhaustive threshold of
  /// 8, a `max-variants` budget of 50, boundaries seeded, `janus-ipog-v1`.
  fn default() -> Self {
    SamplingPolicy {
      strategy: "auto".to_string(),
      strength: 2,
      exhaustive_threshold: 8,
      max_variants: 50,
      boundaries: true,
      algorithm: "janus-ipog-v1".to_string(),
      pin: Vec::new(),
      exclude: Vec::new(),
    }
  }
}

impl SamplingPolicy {
  fn apply(&mut self, patch: PolicyPatch) {
    if let Some(v) = patch.strategy {
      self.strategy = v;
    }
    if let Some(v) = patch.strength {
      self.strength = v;
    }
    if let Some(v) = patch.exhaustive_threshold {
      self.exhaustive_threshold = v;
    }
    if let Some(v) = patch.max_variants {
      self.max_variants = v;
    }
    if let Some(v) = patch.boundaries {
      self.boundaries = v;
    }
    if let Some(v) = patch.algorithm {
      self.algorithm = v;
    }
    self.pin.extend(patch.pin);
    self.exclude.extend(patch.exclude);
  }

  /// Resolve the layered policy document (spec §3.8): defaults, then each of `layers` in order
  /// (session config, the per-call override, ...) — the interaction specification's own policy
  /// (layer 3) has no document model yet (designs 2.5/3.2's business) and is skipped. `None`
  /// entries (a layer that supplied nothing) are ignored.
  pub fn resolve(layers: &[Option<&Value>]) -> Result<SamplingPolicy, Problem> {
    let mut policy = SamplingPolicy::default();
    for layer in layers.iter().flatten() {
      let patch: PolicyPatch = serde_path_to_error::deserialize(*layer).map_err(|err| Problem {
        pointer: "/policy".to_string(),
        message: err.to_string(),
      })?;
      policy.apply(patch);
    }
    Ok(policy)
  }
}

// ---------------------------------------------------------------------------------------------
// Selection and its report (spec §3.9, `schemas/v1/variant-selection.schema.json`).
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Origin {
  Base,
  Boundary,
  Pinned,
  Covering,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Variant {
  pub id: String,
  pub label: String,
  pub origin: Origin,
  pub assignment: Assignment,
}

impl Variant {
  pub fn to_json(&self, space: &VariantSpace) -> Value {
    let mut assignment = Vec::new();
    for dim in &space.dimensions {
      if let Some(point) = self.assignment.get(&dim.id) {
        assignment.push(serde_json::json!({ "dimension": dim.id, "point": point }));
      }
    }
    serde_json::json!({
      "id": self.id,
      "label": self.label,
      "origin": self.origin,
      "assignment": assignment,
    })
  }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SpaceSummary {
  pub size: u64,
  pub exact: bool,
  pub dimensions: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Coverage {
  pub targets: usize,
  pub covered: usize,
  pub removed: usize,
  pub dropped: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Budgets {
  pub exhaustive_threshold: u64,
  pub max_variants: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AppliedExclusion {
  pub when: Vec<Value>,
  pub reason: String,
  pub removed: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SelectionReport {
  pub space: SpaceSummary,
  pub strategy: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub strength: Option<u32>,
  pub algorithm: String,
  pub selected: usize,
  pub coverage: Coverage,
  pub budgets: Budgets,
  pub boundaries: bool,
  #[serde(skip_serializing_if = "Vec::is_empty")]
  pub exclusions: Vec<AppliedExclusion>,
}

impl SelectionReport {
  pub fn to_json(&self) -> Value {
    serde_json::to_value(self).expect("SelectionReport always serializes")
  }
}

#[derive(Debug)]
pub struct Selected {
  pub variants: Vec<Variant>,
  pub report: SelectionReport,
}

#[derive(Debug, Clone, PartialEq)]
pub enum VariantError {
  /// `interaction-invalid` (spec §8): a malformed policy, or an unresolvable/ambiguous/mistyped
  /// dimension reference in a pin or exclusion.
  InvalidPolicy(Vec<Problem>),
  /// `variant-budget-exceeded` (spec §3.6, §8).
  BudgetExceeded {
    space: u64,
    selected: usize,
    budget: u64,
    dimensions: Vec<String>,
  },
}

// ---------------------------------------------------------------------------------------------
// §2.1 — assignments, activation, `complete`.
// ---------------------------------------------------------------------------------------------

fn is_active(assignment: &Assignment, dim: &Dimension) -> bool {
  dim
    .gated_by
    .iter()
    .all(|g| assignment.get(&g.dimension).is_some_and(|p| p == &g.point))
}

/// `complete(α)` (spec §2.1): walk the dimensions in variant-space order; skip the inactive
/// ones; keep the point `partial` gives an active one, or its default.
pub fn complete(space: &VariantSpace, partial: &Assignment) -> Assignment {
  let mut result = Assignment::new();
  for dim in &space.dimensions {
    if !is_active(&result, dim) {
      continue;
    }
    let point = partial
      .get(&dim.id)
      .cloned()
      .unwrap_or_else(|| dim.default.clone());
    result.insert(dim.id.clone(), point);
  }
  result
}

// ---------------------------------------------------------------------------------------------
// §3.5 — exclusions, resolved against the space.
// ---------------------------------------------------------------------------------------------

struct ResolvedExclusion {
  when: Vec<(String, String)>,
  reason: String,
  removed: usize,
}

fn assignment_matches(assignment: &Assignment, pairs: &[(String, String)]) -> bool {
  pairs
    .iter()
    .all(|(d, p)| assignment.get(d).is_some_and(|ap| ap == p))
}

fn matching_exclusion<'a>(
  assignment: &Assignment,
  exclusions: &'a [ResolvedExclusion],
) -> Option<&'a ResolvedExclusion> {
  exclusions
    .iter()
    .find(|e| assignment_matches(assignment, &e.when))
}

/// Resolve every pin/exclusion's dimension references against `space` (spec §6.3), and validate
/// the points named exist (spec §6.5's table, minus the variant-params-specific rows which don't
/// apply here).
fn resolve_refs(
  space: &VariantSpace,
  policy: &SamplingPolicy,
) -> Result<(Vec<Assignment>, Vec<ResolvedExclusion>), Vec<Problem>> {
  let mut problems = Vec::new();
  let mut pins = Vec::new();
  for pin in &policy.pin {
    let mut partial = Assignment::new();
    for point_ref in &pin.assignment {
      match resolve_point_ref(space, point_ref) {
        Ok((id, point)) => {
          partial.insert(id, point);
        }
        Err(problem) => problems.push(problem),
      }
    }
    pins.push(partial);
  }
  let mut exclusions = Vec::new();
  for excl in &policy.exclude {
    let mut when = Vec::new();
    for point_ref in &excl.when {
      match resolve_point_ref(space, point_ref) {
        Ok(pair) => when.push(pair),
        Err(problem) => problems.push(problem),
      }
    }
    exclusions.push(ResolvedExclusion {
      when,
      reason: excl.reason.clone(),
      removed: 0,
    });
  }
  if problems.is_empty() {
    Ok((pins, exclusions))
  } else {
    Err(problems)
  }
}

fn resolve_point_ref(space: &VariantSpace, point_ref: &PointRef) -> Result<(String, String), Problem> {
  match resolve_dimension_ref(space, &point_ref.dimension) {
    RefResolution::NotFound => Err(Problem {
      pointer: "/policy".to_string(),
      message: format!(
        "'{}' does not name any dimension of this interaction",
        point_ref.dimension
      ),
    }),
    RefResolution::Ambiguous(candidates) => Err(Problem {
      pointer: "/policy".to_string(),
      message: format!(
        "'{}' names more than one dimension ({})",
        point_ref.dimension,
        candidates.join(", ")
      ),
    }),
    RefResolution::Found(dim) => {
      if dim.points.iter().any(|p| p.name == point_ref.point) {
        Ok((dim.id.clone(), point_ref.point.clone()))
      } else {
        Err(Problem {
          pointer: "/policy".to_string(),
          message: format!("dimension '{}' has no point '{}'", dim.id, point_ref.point),
        })
      }
    }
  }
}

// ---------------------------------------------------------------------------------------------
// §2.3 — counting the space.
// ---------------------------------------------------------------------------------------------

fn last_gate(dim: &Dimension) -> Option<(&str, &str)> {
  dim
    .gated_by
    .last()
    .map(|g| (g.dimension.as_str(), g.point.as_str()))
}

fn children_by_gate(space: &VariantSpace) -> HashMap<(&str, &str), Vec<&Dimension>> {
  let mut map: HashMap<(&str, &str), Vec<&Dimension>> = HashMap::new();
  for dim in &space.dimensions {
    if let Some(gate) = last_gate(dim) {
      map.entry(gate).or_default().push(dim);
    }
  }
  map
}

/// `weight(d)`/`size` (spec §2.3), saturating at `u64::MAX` (a fact about the shape that nobody
/// needs exact past that point — the algorithm never depends on the precise saturated value).
fn space_size(space: &VariantSpace) -> (u64, bool) {
  let children = children_by_gate(space);
  let by_id: HashMap<&str, &Dimension> = space.dimensions.iter().map(|d| (d.id.as_str(), d)).collect();

  fn weight(
    dim_id: &str,
    by_id: &HashMap<&str, &Dimension>,
    children: &HashMap<(&str, &str), Vec<&Dimension>>,
  ) -> (u64, bool) {
    let dim = by_id[dim_id];
    let mut total = 0u64;
    let mut exact = true;
    for point in &dim.points {
      let mut product = 1u64;
      if let Some(kids) = children.get(&(dim_id, point.name.as_str())) {
        for kid in kids {
          let (w, kid_exact) = weight(&kid.id, by_id, children);
          exact &= kid_exact;
          let (p, overflowed) = product.overflowing_mul(w);
          product = if overflowed { u64::MAX } else { p };
          exact &= !overflowed;
        }
      }
      let (t, overflowed) = total.overflowing_add(product);
      total = if overflowed { u64::MAX } else { t };
      exact &= !overflowed;
    }
    (total, exact)
  }

  let mut size = 1u64;
  let mut exact = true;
  for dim in &space.dimensions {
    if dim.gated_by.is_empty() {
      let (w, dim_exact) = weight(&dim.id, &by_id, &children);
      exact &= dim_exact;
      let (s, overflowed) = size.overflowing_mul(w);
      size = if overflowed { u64::MAX } else { s };
      exact &= !overflowed;
    }
  }
  (size, exact)
}

// ---------------------------------------------------------------------------------------------
// §3.1 — the boundary variants.
// ---------------------------------------------------------------------------------------------

fn descendant_count(dim_id: &str, point: &str, children: &HashMap<(&str, &str), Vec<&Dimension>>) -> usize {
  match children.get(&(dim_id, point)) {
    None => 0,
    Some(kids) => kids
      .iter()
      .map(|kid| {
        1 + kid
          .points
          .iter()
          .map(|p| descendant_count(&kid.id, &p.name, children))
          .sum::<usize>()
      })
      .sum(),
  }
}

/// The extreme (by `key`) of `items`, ties broken by point order — i.e. the *first* item reaching
/// that extreme, for both the minimal and the maximal search (spec §3.1: "ties broken by point
/// order," not "last for maximal"). `Iterator::max_by_key` breaks ties toward the *last* element,
/// which is the wrong direction here, so this is written out rather than reused.
fn first_extreme<'a>(items: impl Iterator<Item = (usize, &'a str)>, minimal: bool) -> Option<&'a str> {
  let mut best: Option<(usize, &str)> = None;
  for (key, name) in items {
    best = match best {
      None => Some((key, name)),
      Some((bk, _)) if (minimal && key < bk) || (!minimal && key > bk) => Some((key, name)),
      other => other,
    };
  }
  best.map(|(_, name)| name)
}

/// The minimal or maximal boundary partial assignment (spec §3.1's table): each dimension takes
/// its extreme point and `complete` settles the rest, built incrementally so a dimension's own
/// activity (and, for `alternative`, its descendant count) is decided from what came before it.
fn boundary_partial(space: &VariantSpace, minimal: bool) -> Assignment {
  let children = children_by_gate(space);
  let mut result = Assignment::new();
  for dim in &space.dimensions {
    if !is_active(&result, dim) {
      continue;
    }
    let point = match dim.facet {
      "presence" => {
        if minimal {
          "absent"
        } else {
          "present"
        }
      }
      "nullability" => {
        if minimal {
          "null"
        } else {
          "non-null"
        }
      }
      "cardinality" => {
        let extreme = first_extreme(
          dim
            .points
            .iter()
            .map(|p| (p.size.unwrap_or(0) as usize, p.name.as_str())),
          minimal,
        );
        extreme.unwrap_or(dim.default.as_str())
      }
      "alternative" => {
        let extreme = first_extreme(
          dim
            .points
            .iter()
            .map(|p| (descendant_count(&dim.id, &p.name, &children), p.name.as_str())),
          minimal,
        );
        extreme.unwrap_or(dim.default.as_str())
      }
      // "value" (any-of) and any other facet: no option is smaller than another (spec §3.1);
      // take the default, same as `complete({})`.
      _ => dim.default.as_str(),
    };
    result.insert(dim.id.clone(), point.to_string());
  }
  result
}

// ---------------------------------------------------------------------------------------------
// §3.4 — the selection algorithm (`janus-ipog-v1`).
// ---------------------------------------------------------------------------------------------

type Target = BTreeMap<String, String>;

/// The requirement set a target implies once its dimensions' gates are pulled in transitively
/// (spec §3.3): the target's own pairs plus every gate of every dimension it names. `None` means
/// inconsistent — some dimension required at two different points — so the target is not
/// reachable.
fn target_requirements(space: &VariantSpace, target: &Target) -> Option<BTreeMap<String, String>> {
  let by_id: HashMap<&str, &Dimension> = space.dimensions.iter().map(|d| (d.id.as_str(), d)).collect();
  let mut reqs: BTreeMap<String, String> = BTreeMap::new();
  fn insert(reqs: &mut BTreeMap<String, String>, dim: &str, point: &str) -> bool {
    match reqs.get(dim) {
      Some(existing) if existing != point => false,
      _ => {
        reqs.insert(dim.to_string(), point.to_string());
        true
      }
    }
  }
  for (dim_id, point) in target {
    if !insert(&mut reqs, dim_id, point) {
      return None;
    }
    let dim = by_id.get(dim_id.as_str())?;
    for gate in &dim.gated_by {
      if !insert(&mut reqs, &gate.dimension, &gate.point) {
        return None;
      }
    }
  }
  Some(reqs)
}

/// Every reachable target at strength `t` (spec §3.3), split into those excluded (counted
/// against `coverage.removed`) and those genuinely in play.
fn enumerate_targets(space: &VariantSpace, t: usize, exclusions: &mut [ResolvedExclusion]) -> Vec<Target> {
  let dims = &space.dimensions;
  let mut targets: HashSet<Target> = HashSet::new();
  let mut combo = vec![0usize; t];
  choose_combinations(dims.len(), t, &mut combo, 0, 0, &mut |indices| {
    cartesian_points(dims, indices, &mut |target| {
      let Some(reqs) = target_requirements(space, target) else {
        return;
      };
      // A target's own dimensions must actually be active under its own requirement closure —
      // true by construction of `target_requirements`, which only accepts consistent chains.
      let _ = reqs;
      if let Some(excl) = exclusions
        .iter_mut()
        .find(|e| assignment_matches(target, &e.when))
      {
        excl.removed += 1;
        return;
      }
      targets.insert(target.clone());
    });
  });
  targets.into_iter().collect()
}

/// Every `t`-combination of dimension indices, in increasing order.
fn choose_combinations(
  n: usize,
  t: usize,
  combo: &mut [usize],
  start: usize,
  depth: usize,
  out: &mut impl FnMut(&[usize]),
) {
  if depth == t {
    out(combo);
    return;
  }
  for i in start..n {
    combo[depth] = i;
    choose_combinations(n, t, combo, i + 1, depth + 1, out);
  }
}

/// Every combination of points across the dimensions named by `indices`, as a `Target`.
fn cartesian_points(dims: &[Dimension], indices: &[usize], out: &mut impl FnMut(&Target)) {
  fn go(
    dims: &[Dimension],
    indices: &[usize],
    pos: usize,
    current: &mut Target,
    out: &mut impl FnMut(&Target),
  ) {
    if pos == indices.len() {
      out(current);
      return;
    }
    let dim = &dims[indices[pos]];
    for point in &dim.points {
      current.insert(dim.id.clone(), point.name.clone());
      go(dims, indices, pos + 1, current, out);
    }
    current.remove(&dim.id);
  }
  let mut current = Target::new();
  go(dims, indices, 0, &mut current, out);
}

/// Step 2 — dimension processing order: descending point count, ties broken by variant-space
/// order, subject to a dimension never preceding one that gates it.
fn processing_order(space: &VariantSpace) -> Vec<usize> {
  let n = space.dimensions.len();
  let id_to_idx: HashMap<&str, usize> = space
    .dimensions
    .iter()
    .enumerate()
    .map(|(i, d)| (d.id.as_str(), i))
    .collect();
  let mut placed = vec![false; n];
  let mut order = Vec::with_capacity(n);
  while order.len() < n {
    let mut best: Option<usize> = None;
    for i in 0..n {
      if placed[i] {
        continue;
      }
      let ready = space.dimensions[i]
        .gated_by
        .iter()
        .all(|g| placed[id_to_idx[g.dimension.as_str()]]);
      if !ready {
        continue;
      }
      let better = match best {
        None => true,
        Some(b) => {
          let pc_i = space.dimensions[i].points.len();
          let pc_b = space.dimensions[b].points.len();
          pc_i > pc_b
        }
      };
      if better {
        best = Some(i);
      }
    }
    let chosen = best.expect("gates form a forest, so some ready dimension always remains");
    placed[chosen] = true;
    order.push(chosen);
  }
  order
}

fn is_covered(target: &Target, assignment: &Assignment) -> bool {
  target
    .iter()
    .all(|(d, p)| assignment.get(d).is_some_and(|ap| ap == p))
}

/// Runs the `janus-ipog-v1` covering algorithm (spec §3.4) and returns the covering variants'
/// partial assignments, in the order produced, plus the count of targets it had to drop.
fn ipog(
  space: &VariantSpace,
  t: usize,
  seeds: &[Assignment],
  mut targets: HashSet<Target>,
  exclusions: &[ResolvedExclusion],
) -> (Vec<Assignment>, usize) {
  // Targets already covered by a seed cost nothing.
  targets.retain(|target| !seeds.iter().any(|s| is_covered(target, s)));
  if targets.is_empty() || space.dimensions.is_empty() {
    return (Vec::new(), 0);
  }

  let order = processing_order(space);
  let t = t.min(order.len()).max(1);

  // Step 3 — initial block over the first t dimensions.
  let mut ts: Vec<Assignment> = Vec::new();
  build_initial_block(space, &order[..t], &mut Assignment::new(), 0, exclusions, &mut ts);
  ts.retain(|tau| !seeds.iter().any(|s| covers_partial(tau, s)));

  // Dimensions decided by the initial block are already "processed" for the purposes of the
  // growth loop below.
  let mut processed: HashSet<&str> = order[..t]
    .iter()
    .map(|&i| space.dimensions[i].id.as_str())
    .collect();

  // Step 4 — growth over the remaining dimensions.
  for &dim_idx in &order[t..] {
    let dim = &space.dimensions[dim_idx];
    processed.insert(dim.id.as_str());

    // Horizontal.
    for tau in ts.iter_mut() {
      if !is_active(tau, dim) {
        continue;
      }
      let mut best_point: Option<&str> = None;
      let mut best_count = 0usize;
      for point in &dim.points {
        let mut candidate = tau.clone();
        candidate.insert(dim.id.clone(), point.name.clone());
        if matching_exclusion(&candidate, exclusions).is_some() {
          continue;
        }
        let count = targets
          .iter()
          .filter(|target| target.contains_key(&dim.id) && is_covered(target, &candidate))
          .count();
        if best_point.is_none() || count > best_count {
          best_point = Some(&point.name);
          best_count = count;
        }
      }
      if let Some(point) = best_point {
        tau.insert(dim.id.clone(), point.to_string());
        targets.retain(|target| !is_covered(target, tau));
      }
    }

    // Vertical. Only a target whose every dimension has already been processed (this one
    // included) is in play here — one naming a dimension later in `order` waits for that
    // dimension's own turn, so a pair is never resolved before both its dimensions are "current".
    let remaining: Vec<Target> = targets
      .iter()
      .filter(|target| target.contains_key(&dim.id) && target.keys().all(|k| processed.contains(k.as_str())))
      .cloned()
      .collect();
    for target in remaining {
      if !targets.contains(&target) {
        continue; // already covered by an earlier extension this pass
      }
      let compatible_idx = ts.iter().position(|tau| is_compatible(tau, &target));
      match compatible_idx {
        Some(idx) => {
          if let Some(reqs) = target_requirements(space, &target) {
            for (d, p) in &reqs {
              ts[idx].entry(d.clone()).or_insert_with(|| p.clone());
            }
          }
        }
        None => {
          if let Some(reqs) = target_requirements(space, &target) {
            let assignment: Assignment = reqs.into_iter().collect();
            ts.push(assignment);
          }
        }
      }
      targets.retain(|t2| {
        !ts.last().is_some_and(|tau| is_covered(t2, tau)) && !ts.iter().any(|tau| is_covered(t2, tau))
      });
    }
  }

  // Step 5 — completion.
  let mut result = Vec::new();
  let mut dropped = 0usize;
  for tau in ts {
    match complete_avoiding_exclusions(space, &tau, exclusions) {
      Some(full) => result.push(full),
      None => dropped += 1,
    }
  }
  (result, dropped)
}

fn covers_partial(partial: &Assignment, full: &Assignment) -> bool {
  partial.iter().all(|(d, p)| full.get(d).is_some_and(|fp| fp == p))
}

fn is_compatible(tau: &Assignment, target: &Target) -> bool {
  target.iter().all(|(d, p)| tau.get(d).is_none_or(|tp| tp == p))
}

fn build_initial_block(
  space: &VariantSpace,
  dims: &[usize],
  current: &mut Assignment,
  pos: usize,
  exclusions: &[ResolvedExclusion],
  out: &mut Vec<Assignment>,
) {
  if pos == dims.len() {
    if matching_exclusion(current, exclusions).is_none() {
      out.push(current.clone());
    }
    return;
  }
  let dim = &space.dimensions[dims[pos]];
  if !is_active(current, dim) {
    build_initial_block(space, dims, current, pos + 1, exclusions, out);
    return;
  }
  for point in &dim.points {
    current.insert(dim.id.clone(), point.name.clone());
    build_initial_block(space, dims, current, pos + 1, exclusions, out);
  }
  current.remove(&dim.id);
}

/// `complete`, but where the result would match an exclusion, try substituting a free
/// dimension's other points (spec §3.4 step 5) until it doesn't; drop it (return `None`,
/// reported as dropped) if no substitution clears every exclusion.
fn complete_avoiding_exclusions(
  space: &VariantSpace,
  partial: &Assignment,
  exclusions: &[ResolvedExclusion],
) -> Option<Assignment> {
  let full = complete(space, partial);
  if matching_exclusion(&full, exclusions).is_none() {
    return Some(full);
  }
  for dim in &space.dimensions {
    if partial.contains_key(&dim.id) {
      continue; // not free: this dimension's point was explicitly required.
    }
    let Some(current_point) = full.get(&dim.id) else {
      continue; // inactive in `full`; nothing to substitute.
    };
    for point in &dim.points {
      if &point.name == current_point {
        continue;
      }
      let mut candidate_partial = partial.clone();
      candidate_partial.insert(dim.id.clone(), point.name.clone());
      let candidate = complete(space, &candidate_partial);
      if matching_exclusion(&candidate, exclusions).is_none() {
        return Some(candidate);
      }
    }
  }
  None
}

/// Compute a variant space's selection (spec §3): the heart of plan task 4.3.
pub fn select(space: &VariantSpace, policy: &SamplingPolicy) -> Result<Selected, VariantError> {
  let (pins, mut exclusions) = resolve_refs(space, policy).map_err(VariantError::InvalidPolicy)?;
  let labels = compute_labels(space);
  let (size, exact) = space_size(space);

  let strategy = match policy.strategy.as_str() {
    "auto" => {
      if exact && size <= policy.exhaustive_threshold {
        "exhaustive"
      } else {
        "t-wise"
      }
    }
    other @ ("exhaustive" | "t-wise" | "base-only") => other,
    other => {
      return Err(VariantError::InvalidPolicy(vec![Problem {
        pointer: "/policy/strategy".to_string(),
        message: format!("unknown sampling strategy '{other}'"),
      }]));
    }
  };

  let mut seen: HashSet<Assignment> = HashSet::new();
  let mut variants: Vec<Variant> = Vec::new();
  let push =
    |assignment: Assignment, origin: Origin, variants: &mut Vec<Variant>, seen: &mut HashSet<Assignment>| {
      if seen.insert(assignment.clone()) {
        variants.push(Variant {
          id: render_assignment(
            space,
            &assignment,
            &space
              .dimensions
              .iter()
              .map(|d| (d.id.clone(), d.id.clone()))
              .collect(),
          ),
          label: render_assignment(space, &assignment, &labels),
          origin,
          assignment,
        });
      }
    };

  // Step 0 — seeds.
  push(
    complete(space, &Assignment::new()),
    Origin::Base,
    &mut variants,
    &mut seen,
  );
  if policy.boundaries && strategy != "base-only" {
    push(
      complete(space, &boundary_partial(space, true)),
      Origin::Boundary,
      &mut variants,
      &mut seen,
    );
    push(
      complete(space, &boundary_partial(space, false)),
      Origin::Boundary,
      &mut variants,
      &mut seen,
    );
  }
  for pin in &pins {
    push(complete(space, pin), Origin::Pinned, &mut variants, &mut seen);
  }

  let mut dropped = 0usize;
  let mut covering_count = 0usize;
  let mut total_targets = 0usize;

  match strategy {
    "base-only" => {}
    "exhaustive" => {
      let seeds: Vec<Assignment> = variants.iter().map(|v| v.assignment.clone()).collect();
      let mut all = Vec::new();
      enumerate_all(space, &mut Assignment::new(), 0, &exclusions, &mut all);
      for assignment in all {
        if !seeds.iter().any(|s| s == &assignment) {
          push(assignment, Origin::Covering, &mut variants, &mut seen);
          covering_count += 1;
        }
      }
    }
    _ => {
      let t = policy.strength.max(1) as usize;
      let targets: HashSet<Target> = enumerate_targets(space, t, &mut exclusions).into_iter().collect();
      total_targets = targets.len();
      let seeds: Vec<Assignment> = variants.iter().map(|v| v.assignment.clone()).collect();
      let (covering, drop_count) = ipog(space, t, &seeds, targets, &exclusions);
      dropped = drop_count;
      for assignment in covering {
        push(assignment, Origin::Covering, &mut variants, &mut seen);
        covering_count += 1;
      }
    }
  }
  let _ = covering_count;

  if variants.len() as u64 > policy.max_variants {
    let mut by_points: Vec<&Dimension> = space.dimensions.iter().collect();
    by_points.sort_by_key(|d| std::cmp::Reverse(d.points.len()));
    let dimensions = by_points.into_iter().take(3).map(|d| d.id.clone()).collect();
    return Err(VariantError::BudgetExceeded {
      space: size,
      selected: variants.len(),
      budget: policy.max_variants,
      dimensions,
    });
  }

  let covered = if strategy == "t-wise" {
    total_targets.saturating_sub(dropped)
  } else {
    0
  };
  let removed: usize = exclusions.iter().map(|e| e.removed).sum();

  let applied_exclusions: Vec<AppliedExclusion> = exclusions
    .iter()
    .filter(|e| e.removed > 0)
    .map(|e| AppliedExclusion {
      when: e
        .when
        .iter()
        .map(|(d, p)| serde_json::json!({ "dimension": d, "point": p }))
        .collect(),
      reason: e.reason.clone(),
      removed: e.removed,
    })
    .collect();

  let report = SelectionReport {
    space: SpaceSummary {
      size,
      exact,
      dimensions: space.dimensions.len(),
    },
    strategy: strategy.to_string(),
    strength: (strategy == "t-wise").then_some(policy.strength),
    algorithm: policy.algorithm.clone(),
    selected: variants.len(),
    coverage: Coverage {
      targets: total_targets,
      covered,
      removed,
      dropped,
    },
    budgets: Budgets {
      exhaustive_threshold: policy.exhaustive_threshold,
      max_variants: policy.max_variants,
    },
    boundaries: policy.boundaries,
    exclusions: applied_exclusions,
  };

  Ok(Selected { variants, report })
}

fn enumerate_all(
  space: &VariantSpace,
  current: &mut Assignment,
  pos: usize,
  exclusions: &[ResolvedExclusion],
  out: &mut Vec<Assignment>,
) {
  if pos == space.dimensions.len() {
    if matching_exclusion(current, exclusions).is_none() {
      out.push(current.clone());
    }
    return;
  }
  let dim = &space.dimensions[pos];
  if !is_active(current, dim) {
    enumerate_all(space, current, pos + 1, exclusions, out);
    return;
  }
  for point in &dim.points {
    current.insert(dim.id.clone(), point.name.clone());
    enumerate_all(space, current, pos + 1, exclusions, out);
  }
  current.remove(&dim.id);
}
