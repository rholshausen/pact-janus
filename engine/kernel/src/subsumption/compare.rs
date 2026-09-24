//! The subsumption walk (design 2.8 §3): `admits(P) ⊆ admits(C)`, decomposed.
//!
//! The walk is not a bespoke algorithm layered on top of the shape language (spec §3.2) — it is
//! shape spec §4.1's own `admits` definitions read as a recursive containment check, with §3.2's
//! two composition rules saying how containment factors through composition, and shape spec §8's
//! comparability classes deciding each leaf. Three things follow, and they are the invariants to
//! keep when changing this file:
//!
//! 1. **`unknown` is never a guess, in either direction.** Every arm that cannot decide returns
//!    `unknown` with a reason naming why, and no arm reaches a verdict by heuristic. Returning
//!    `unknown` where a decision was possible is a missed improvement; returning `yes` or `no`
//!    where it was not is a bug of a different order (shape spec §8).
//! 2. **Every `no` and every `unknown` carries a finding.** [`Out::no`] and [`Out::unknown`] take
//!    one, which is what makes §4.1's "a finding is recorded at the shallowest node whose verdict
//!    is not `yes`" fall out of the recursion rather than needing a second pass: a composite adds
//!    findings only for the decisions it makes *itself* (a cardinality interval, a presence set, a
//!    member its counterpart does not name) and otherwise just conjoins its children's.
//! 3. **Presence and nullability are set containment, not special rules** (shape spec §8's payoff
//!    for putting `⊥` in the domain). [`decompose`] splits any node into "admits `⊥`?", "admits
//!    `null`?" and the residual value shape, so `optional`, `forbidden`, `nullable` and a
//!    `null`-carrying `any-of` are all compared by the same three containments.

use super::phrases;
use super::report::{Finding, Side};
use crate::interaction_spec::InteractionSpec;
use crate::plan;
use crate::shape::{CoreShape, Example, ShapeKind, ShapeNode, ValueKind, path};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};

/// The walk's three-valued answer (shape spec §8). `not-published` is a report-level state, not
/// one of these (spec §6.3), and lives on [`super::InteractionResult`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Verdict {
  Yes,
  No,
  Unknown,
}

impl Verdict {
  /// Kleene conjunction (spec §3.2, Rule 1): `no` if any child is `no`; otherwise `unknown` if any
  /// child is `unknown`; otherwise `yes`.
  pub fn and(self, other: Verdict) -> Verdict {
    match (self, other) {
      (Verdict::No, _) | (_, Verdict::No) => Verdict::No,
      (Verdict::Unknown, _) | (_, Verdict::Unknown) => Verdict::Unknown,
      (Verdict::Yes, Verdict::Yes) => Verdict::Yes,
    }
  }

  pub fn as_str(self) -> &'static str {
    match self {
      Verdict::Yes => "yes",
      Verdict::No => "no",
      Verdict::Unknown => "unknown",
    }
  }
}

/// One node's contribution: its verdict, and the findings recorded at or under it.
#[derive(Debug, Clone, PartialEq)]
pub struct Out {
  pub verdict: Verdict,
  pub findings: Vec<Finding>,
}

impl Out {
  fn yes() -> Out {
    Out {
      verdict: Verdict::Yes,
      findings: Vec::new(),
    }
  }

  fn no(finding: Finding) -> Out {
    Out {
      verdict: Verdict::No,
      findings: vec![finding],
    }
  }

  fn unknown(finding: Finding) -> Out {
    Out {
      verdict: Verdict::Unknown,
      findings: vec![finding],
    }
  }

  fn and(mut self, other: Out) -> Out {
    self.verdict = self.verdict.and(other.verdict);
    self.findings.extend(other.findings);
    self
  }
}

/// One exclusion the consumer's own sampler recorded (ADR 0008), as §5's cross-reference reads it.
#[derive(Debug, Clone, PartialEq)]
pub struct RecordedExclusion {
  /// The `Exclusion` document itself (design 2.3's), reproduced verbatim into `excluded-by` and
  /// "not otherwise interpreted" (`finding.schema.json`).
  pub document: Value,
  /// The dimension *paths* (shape spec §6.2's `<path>` half of a dimension id) the exclusion's
  /// `when` set names.
  pub paths: BTreeSet<String>,
}

impl RecordedExclusion {
  /// Read the exclusions out of a contract's recorded selection report (contract spec §5.1,
  /// variant-semantics spec §4.4). A report this checker cannot read contributes nothing: the
  /// cross-reference is a `MAY` (spec §5), so a malformed one degrades to "no caveats" rather
  /// than to an error.
  pub fn from_selection_report(report: &Value) -> Vec<RecordedExclusion> {
    let Some(entries) = report.get("exclusions").and_then(Value::as_array) else {
      return Vec::new();
    };
    entries
      .iter()
      .filter_map(|entry| {
        let when = entry.get("when")?.as_array()?;
        let reason = entry.get("reason")?.as_str()?;
        let paths = when
          .iter()
          .filter_map(|point| point.get("dimension")?.as_str())
          .map(|id| dimension_path(id).to_string())
          .collect();
        Some(RecordedExclusion {
          // The recorded form carries `removed` (how many variants the exclusion took out),
          // which is the selection report's accounting, not part of design 2.3's `Exclusion`.
          document: serde_json::json!({ "when": when, "reason": reason }),
          paths,
        })
      })
      .collect()
  }
}

/// The `<path>` half of a dimension id `<path>#<facet>` (shape spec §6.2).
fn dimension_path(id: &str) -> &str {
  id.split_once('#').map(|(path, _)| path).unwrap_or(id)
}

/// Compare one published shape against one declared shape, rooted at `at` (a dimension path,
/// shape spec §6.2 — typically [`path::root`]'s `"<part>.<slot>"`).
pub fn compare(provider: &ShapeNode, consumer: &ShapeNode, at: &str) -> (Verdict, Vec<Finding>) {
  let mut walk = Walk::new(&[]);
  let out = walk.node(provider, consumer, at);
  (out.verdict, out.findings)
}

/// The walk, plus the §5 cross-reference it carries along.
pub(super) struct Walk<'a> {
  exclusions: &'a [RecordedExclusion],
}

impl<'a> Walk<'a> {
  pub(super) fn new(exclusions: &'a [RecordedExclusion]) -> Walk<'a> {
    Walk { exclusions }
  }

  /// One node pair, with §5's coverage caveat applied at this path. Structural children go
  /// through here (they sit at their own paths); the presence/nullability decomposition does not,
  /// because it stays at the same path and a caveat belongs to a path once.
  pub(super) fn node(&mut self, p: &ShapeNode, c: &ShapeNode, at: &str) -> Out {
    let out = self.value_node(p, c, at);
    self.caveat(out, p, c, at)
  }

  /// Whether shape spec §8's identity floor may short-circuit here. It may, unless §5 has a
  /// caveat for a node *inside* this one: two identical subtrees still compare `yes` node by
  /// node, and walking them is what lets [`Walk::caveat`] reach the passing node the caveat is
  /// about. An operator with no children keeps the floor whatever the exclusions say — there is
  /// nothing under it to reach, and suppressing the floor for an opaque one would turn a decided
  /// `yes` into an `unknown`.
  fn floor_stands(&self, node: &ShapeNode, at: &str) -> bool {
    !has_children(node) || !self.caveats_under(at)
  }

  fn caveats_under(&self, at: &str) -> bool {
    self.exclusions.iter().any(|exclusion| {
      exclusion
        .paths
        .iter()
        .any(|path| path.len() > at.len() && path.starts_with(at))
    })
  }

  /// §5: attach the consumer's recorded exclusions to this node when they name it, and report an
  /// otherwise-silent `yes` that sits over one as an `advisory`.
  ///
  /// "The dimensions a finding's `path` touches" is read here as *the dimensions whose own path is
  /// this node's path* — the dimensions this node contributes. That is the mechanical reading; the
  /// spec's own §3 example (a response field caveated by an exclusion between two request
  /// dimensions) is a judgement about joint coverage that no rule in §5 derives, which is why the
  /// cross-reference is a `MAY` and why this checker does not manufacture it.
  fn caveat(&mut self, mut out: Out, p: &ShapeNode, c: &ShapeNode, at: &str) -> Out {
    if self.exclusions.is_empty() {
      return out;
    }
    let applicable: Vec<&RecordedExclusion> = self
      .exclusions
      .iter()
      .filter(|exclusion| exclusion.paths.contains(at))
      .collect();
    if applicable.is_empty() {
      return out;
    }
    let documents: Vec<Value> = applicable
      .iter()
      .map(|exclusion| exclusion.document.clone())
      .collect();

    let mut attached = false;
    for finding in &mut out.findings {
      if finding.path == at {
        finding.excluded_by = documents.clone();
        attached = true;
      }
    }
    if !attached && out.verdict == Verdict::Yes {
      let mut advisory = Finding::advisory(
        at,
        Side::new(phrases::describe(p)),
        Side::new(phrases::describe(c)),
        "admitted by the consumer's shape — reported only because an exercised-coverage caveat \
         applies (spec §5)",
      );
      advisory.excluded_by = documents;
      out.findings.push(advisory);
    }
    out
  }

  /// A node pair with presence and nullability taken out of it (spec §3.2's Rule 1 for
  /// `optional`/`nullable`, and shape spec §8's presence sets), leaving a value comparison.
  fn value_node(&mut self, p: &ShapeNode, c: &ShapeNode, at: &str) -> Out {
    if same_admits(p, c) && self.floor_stands(p, at) {
      return Out::yes();
    }

    let provider = decompose(p);
    let consumer = decompose(c);
    let mut out = Out::yes();

    // `⊥` and `null` are two containments, and a node can fail both — but they are one *finding*,
    // because §4.2's table has one `weaker-presence` row and a reader looking at one path wants
    // one answer. Reporting them separately doubles the count for every field of a provider shape
    // that marks everything nullable and nothing required, which is precisely the shape of
    // document the RFC's over-broadness worry is about (spike 7.3's measurement found this).
    let widened_absence = provider.absent && !consumer.absent;
    let widened_null = provider.null && !consumer.null;
    if widened_absence || widened_null {
      let reason = match (widened_absence, widened_null) {
        (true, true) => "the provider admits absence and null; the consumer's shape admits neither",
        (true, false) => "the provider admits absence (optional); the consumer's shape does not",
        _ => "the provider admits null; the consumer's shape does not",
      };
      out = out.and(Out::no(Finding::weaker_presence(
        at,
        Side::new(phrases::describe(p)),
        Side::new(if widened_absence {
          phrases::describe_present(c)
        } else {
          phrases::describe(c)
        }),
        reason,
      )));
    }

    match (&provider.rest, &consumer.rest) {
      // The provider publishes nothing but `⊥`/`null`: the two containments above are the whole
      // comparison.
      (None, _) => out,
      // The consumer's shape admits no value at all beyond `⊥`/`null`, and the provider's does.
      (Some(p_rest), None) => out.and(Out::no(Finding::weaker_presence(
        at,
        Side::new(phrases::describe(p_rest)),
        Side::new(phrases::describe(c)),
        "the provider may produce a value where the consumer's shape admits none",
      ))),
      (Some(p_rest), Some(c_rest)) => out.and(self.operators(p_rest, c_rest, at)),
    }
  }

  /// The operator table: shape spec §8's classes, and spec §3.2's Rule 3 — nothing requires the
  /// two sides to name the same operator.
  fn operators(&mut self, p: &ShapeNode, c: &ShapeNode, at: &str) -> Out {
    if same_admits(p, c) && self.floor_stands(p, at) {
      return Out::yes();
    }

    // The opaque class (shape spec §8): `contains`, and every component operator that has not
    // declared a comparability procedure. The kernel MUST NOT guess what one admits (shape spec
    // §3.5), which makes `unknown` the only honest answer once identity has failed.
    if is_opaque(p) || is_opaque(c) {
      return Out::unknown(Finding::unreviewable(
        at,
        Side::new(phrases::describe(p)),
        Side::new(phrases::describe(c)),
        format!(
          "'{}' against '{}': opaque class, no containment procedure",
          p.operator(),
          c.operator()
        ),
      ));
    }

    // `type` is exactly a kind predicate, an unconstrained `object`, or an unconstrained list
    // (shape spec §4.2: it "constrains the kind only"). Rewriting it here removes a whole column
    // from the table below rather than special-casing it in every arm.
    let p_desugared = desugar(p);
    let c_desugared = desugar(c);
    let p = p_desugared.as_ref().unwrap_or(p);
    let c = c_desugared.as_ref().unwrap_or(c);
    if same_admits(p, c) && self.floor_stands(p, at) {
      return Out::yes();
    }

    let (ShapeKind::Core(pc), ShapeKind::Core(cc)) = (&p.kind, &c.kind) else {
      // Unreachable: the opaque check above covers every component node.
      return Out::unknown(Finding::unreviewable(
        at,
        Side::new(phrases::describe(p)),
        Side::new(phrases::describe(c)),
        "component operator",
      ));
    };

    // `any` admits every value, which decides both directions outright.
    if matches!(cc, CoreShape::Any) {
      return Out::yes();
    }
    if matches!(pc, CoreShape::Any) {
      return Out::no(Finding::broader_type(
        at,
        Side::new(phrases::describe(p)),
        Side::new(phrases::describe(c)),
        "the provider's shape admits every value; the consumer's does not",
      ));
    }

    // Kinds that cannot overlap decide `no` without looking any further, and they are what makes
    // most cross-operator pairs (an object against a list, a number against a regex) decidable
    // rather than merely uncomparable.
    if let (Some(p_classes), Some(c_classes)) = (classes(p), classes(c))
      && p_classes.is_disjoint(&c_classes)
    {
      return Out::no(Finding::broader_type(
        at,
        Side::new(phrases::describe(p)),
        Side::new(phrases::describe(c)),
        "the two shapes admit values of disjoint kinds",
      ));
    }

    // The conservative class (shape spec §8): `yes` on identity — already decided above — and
    // `yes` where the container is exactly wider. Everything else is `unknown`, deliberately:
    // two different regexes, or two different datetime formats, are not `no`.
    if is_conservative(pc) || is_conservative(cc) {
      if is_conservative(pc) && string_valued(pc) && matches!(cc, CoreShape::Kind(ValueKind::String)) {
        return Out::yes();
      }
      // A provider that admits a finite set of values against a conservative consumer is decided
      // by asking the consumer's own matcher about each value — no containment algorithm, just
      // membership, which is what the interpreter already decides for every exchange (shape spec
      // §8: `unknown` is a permission, and narrowing it is a pure improvement; phase-9 finding
      // 9). `content-type` stays out: it inspects octets, and an enumerated example is JSON.
      if string_valued(cc)
        && let Some(values) = enumerate(p)
      {
        let outside: Vec<&Value> = values.iter().filter(|value| !matched_by(c, value)).collect();
        if outside.is_empty() {
          return Out::yes();
        }
        return Out::no(Finding::wider_values(
          at,
          Side::new(phrases::describe(p)),
          Side::new(phrases::describe(c)),
          format!(
            "the consumer's {} does not admit {}",
            cc.operator_name(),
            outside
              .iter()
              .map(|value| value.to_string())
              .collect::<Vec<_>>()
              .join(", ")
          ),
        ));
      }
      return Out::unknown(Finding::unreviewable(
        at,
        Side::new(phrases::describe(p)),
        Side::new(phrases::describe(c)),
        conservative_reason(pc, cc),
      ));
    }

    self.exact(p, pc, c, cc, at)
  }

  /// The exact class (shape spec §8): "decide `yes`/`no` for every pair of operators in this
  /// class, by set containment".
  fn exact(&mut self, p: &ShapeNode, pc: &CoreShape, c: &ShapeNode, cc: &CoreShape, at: &str) -> Out {
    match (pc, cc) {
      // --- discriminated unions (spec §3.3) ---
      (CoreShape::OneOf { .. }, CoreShape::OneOf { .. }) => self.one_of_pair(p, pc, c, cc, at),
      (CoreShape::OneOf { alternatives, .. }, _) => {
        // `admits(one-of)` is the union of its alternatives, so containment in `C` is the
        // conjunction of each alternative's containment — a union on the *left* decomposes.
        let mut out = Out::yes();
        for (name, alternative) in alternatives {
          out = out.and(self.node(alternative, c, &path::alternative(at, name)));
        }
        out
      }
      (_, CoreShape::OneOf { .. }) => self.inside_alternative(p, c, cc, at),

      // --- objects (spec §3.2, Rule 2) ---
      (CoreShape::Object { members: p_members }, CoreShape::Object { members: c_members }) => {
        let mut out = Out::yes();
        for (name, c_member) in c_members {
          let member_path = path::member(at, name);
          match p_members.get(name) {
            Some(p_member) => out = out.and(self.node(p_member, c_member, &member_path)),
            // Rule 2: a name the provider's object does not bind is not silence about a narrow
            // claim, it is the widest possible claim — any value, or absent.
            None if admits_anything_or_absence(c_member) => {}
            None => {
              out = out.and(Out::no(Finding::undeclared_member(
                &member_path,
                Side::new(phrases::describe(c_member)),
              )))
            }
          }
        }
        // A member the provider names and the consumer does not is must-ignore (shape spec §4.3),
        // and reporting it "would teach a team to ignore the report" (spec §4.1).
        out
      }

      // --- positional and homogeneous collections ---
      (CoreShape::Array { entries: p_entries }, CoreShape::Array { entries: c_entries }) => {
        if p_entries.len() != c_entries.len() {
          return Out::no(Finding::wider_cardinality(
            at,
            Side::new(phrases::describe(p)),
            Side::new(phrases::describe(c)),
            format!(
              "{} positions vs {}; positions the shorter side lacks have no comparison to make",
              p_entries.len(),
              c_entries.len()
            ),
          ));
        }
        let mut out = Out::yes();
        for (index, (p_entry, c_entry)) in p_entries.iter().zip(c_entries).enumerate() {
          out = out.and(self.node(p_entry, c_entry, &path::array_index(at, index)));
        }
        out
      }
      (
        CoreShape::EachLike {
          items: p_items,
          min: p_min,
          max: p_max,
        },
        CoreShape::EachLike {
          items: c_items,
          min: c_min,
          max: c_max,
        },
      ) => {
        let mut out = self.cardinality(*p_min, *p_max, *c_min, *c_max, at, "elements");
        out = out.and(self.node(p_items, c_items, &path::each_like_item(at)));
        out
      }
      (
        CoreShape::EachLike {
          items,
          min: p_min,
          max: p_max,
        },
        CoreShape::Array { entries },
      ) => {
        let fixed = entries.len() as u64;
        let mut out = self.cardinality(*p_min, *p_max, fixed, Some(fixed), at, "elements");
        for (index, entry) in entries.iter().enumerate() {
          out = out.and(self.node(items, entry, &path::array_index(at, index)));
        }
        out
      }
      (
        CoreShape::Array { entries },
        CoreShape::EachLike {
          items,
          min: c_min,
          max: c_max,
        },
      ) => {
        let fixed = entries.len() as u64;
        let mut out = self.cardinality(fixed, Some(fixed), *c_min, *c_max, at, "elements");
        for (index, entry) in entries.iter().enumerate() {
          out = out.and(self.node(entry, items, &path::array_index(at, index)));
        }
        out
      }
      (
        CoreShape::EachEntry {
          keys: p_keys,
          values: p_values,
          min: p_min,
          max: p_max,
        },
        CoreShape::EachEntry {
          keys: c_keys,
          values: c_values,
          min: c_min,
          max: c_max,
        },
      ) => {
        let mut out = self.cardinality(*p_min, *p_max, *c_min, *c_max, at, "entries");
        let any = any_node();
        let p_keys = p_keys.as_deref().unwrap_or(&any);
        let c_keys = c_keys.as_deref().unwrap_or(&any);
        out = out.and(self.node(p_keys, c_keys, &path::each_entry_key(at)));
        out.and(self.node(p_values, c_values, &path::each_entry_value(at)))
      }

      // --- emptiness ---
      (_, CoreShape::NotEmpty) => match admits_only_non_empty(p) {
        Some(true) => Out::yes(),
        Some(false) => Out::no(Finding::wider_cardinality(
          at,
          Side::new(phrases::describe(p)),
          Side::new(phrases::describe(c)),
          "the provider may produce an empty value; the consumer tested only non-empty ones",
        )),
        None => Out::unknown(Finding::unreviewable(
          at,
          Side::new(phrases::describe(p)),
          Side::new(phrases::describe(c)),
          format!(
            "'{}' against 'not-empty': emptiness not decidable here",
            p.operator()
          ),
        )),
      },
      (CoreShape::NotEmpty, _) => Out::no(Finding::broader_type(
        at,
        Side::new(phrases::describe(p)),
        Side::new(phrases::describe(c)),
        "'not-empty' admits a non-empty value of every kind; the consumer's shape admits one kind",
      )),

      // --- literal sets ---
      (CoreShape::Equality, _) => match p.example.as_ref().map(|example| admits_example(c, example)) {
        Some(Some(true)) => Out::yes(),
        Some(Some(false)) => Out::no(Finding::wider_values(
          at,
          Side::new(phrases::describe(p)),
          Side::new(phrases::describe(c)),
          "the provider's value is not admitted by the consumer's shape",
        )),
        _ => Out::unknown(Finding::unreviewable(
          at,
          Side::new(phrases::describe(p)),
          Side::new(phrases::describe(c)),
          format!(
            "a literal against '{}': membership not decidable without matching",
            c.operator()
          ),
        )),
      },
      (CoreShape::AnyOf { options }, _) => {
        let mut decided = Vec::new();
        for option in options {
          decided.push(admits_value(c, option));
        }
        if decided.contains(&Some(false)) {
          Out::no(Finding::wider_values(
            at,
            Side::new(phrases::describe(p)),
            Side::new(phrases::describe(c)),
            match cc {
              CoreShape::AnyOf { options: c_options } => format!(
                "{} options vs {}; finite sets, exact",
                options.len(),
                c_options.len()
              ),
              _ => "an option the consumer's shape does not admit".to_string(),
            },
          ))
        } else if decided.iter().any(Option::is_none) {
          Out::unknown(Finding::unreviewable(
            at,
            Side::new(phrases::describe(p)),
            Side::new(phrases::describe(c)),
            format!(
              "a literal set against '{}': membership not decidable without matching",
              c.operator()
            ),
          ))
        } else {
          Out::yes()
        }
      }
      // A consumer `equality` is a one-element literal set, and is decided the same way a
      // consumer `any-of` is: by whether the provider's own admitted set is finite and inside it.
      (_, CoreShape::Equality) => {
        let options: Vec<Value> = c
          .example
          .as_ref()
          .filter(|example| example.encoded.is_none())
          .map(|example| vec![example.value.clone()])
          .unwrap_or_default();
        if options.is_empty() {
          return Out::unknown(Finding::unreviewable(
            at,
            Side::new(phrases::describe(p)),
            Side::new(phrases::describe(c)),
            "the consumer's `equality` carries no comparable example",
          ));
        }
        self.literal_set(p, c, &options, at)
      }
      (_, CoreShape::AnyOf { options }) => {
        let options = options.clone();
        self.literal_set(p, c, &options, at)
      }

      // --- kinds ---
      (CoreShape::Kind(p_kind), CoreShape::Kind(c_kind)) => {
        if kind_subset(*p_kind, *c_kind) {
          Out::yes()
        } else {
          Out::no(Finding::broader_type(
            at,
            Side::new(phrases::describe(p)),
            Side::new(phrases::describe(c)),
            format!(
              "kind lattice: {} subset of {}, exact",
              c_kind.operator_name(),
              p_kind.operator_name()
            ),
          ))
        }
      }
      (CoreShape::Semver, CoreShape::Kind(ValueKind::String)) => Out::yes(),
      (CoreShape::Kind(ValueKind::String), CoreShape::Semver) => Out::no(Finding::broader_type(
        at,
        Side::new(phrases::describe(p)),
        Side::new(phrases::describe(c)),
        "the provider may produce any string; the consumer tested only semantic versions",
      )),

      // Everything left is a structural pair this checker does not decide — an object against a
      // map, a list against a scalar shape of the same kind. Saying so is the point (shape spec
      // §8): the alternative is a guess.
      _ => Out::unknown(Finding::unreviewable(
        at,
        Side::new(phrases::describe(p)),
        Side::new(phrases::describe(c)),
        format!(
          "'{}' against '{}': this checker decides no containment for the pair",
          p.operator(),
          c.operator()
        ),
      )),
    }
  }

  /// A consumer shape that is a finite set of literals (`any-of`, or `equality`'s set of one).
  /// Containment holds only if the provider's own admitted set is finite too and inside it — an
  /// operator admitting unboundedly many values is never inside a finite set, which is the "wider
  /// enum" finding the RFC lists, read in the one direction that decides it.
  fn literal_set(&mut self, p: &ShapeNode, c: &ShapeNode, options: &[Value], at: &str) -> Out {
    match enumerate(p) {
      Some(values) if values.iter().all(|v| options.iter().any(|o| value_eq(o, v))) => Out::yes(),
      Some(_) => Out::no(Finding::wider_values(
        at,
        Side::new(phrases::describe(p)),
        Side::new(phrases::describe(c)),
        "the provider may produce a value outside the consumer's option set",
      )),
      None => Out::no(Finding::wider_values(
        at,
        Side::new(phrases::describe(p)),
        Side::new(phrases::describe(c)),
        "the provider's shape admits unboundedly many values; the consumer declared a finite set",
      )),
    }
  }

  /// The cardinality child of `each-like`/`each-entry` (spec §3.2, Rule 1): plain interval
  /// containment, a leaf of the recursion rather than a further composite.
  fn cardinality(
    &mut self,
    p_min: u64,
    p_max: Option<u64>,
    c_min: u64,
    c_max: Option<u64>,
    at: &str,
    unit: &str,
  ) -> Out {
    let within = p_min >= c_min
      && match (p_max, c_max) {
        (_, None) => true,
        (None, Some(_)) => false,
        (Some(p_max), Some(c_max)) => p_max <= c_max,
      };
    if within {
      return Out::yes();
    }
    let phrase = |min: u64, max: Option<u64>| match unit {
      "entries" => phrases::cardinality_entries(min, max),
      _ => phrases::cardinality(min, max),
    };
    let reason = if p_min < c_min && p_min == 0 {
      "the empty array is admitted by the provider, not by the consumer".to_string()
    } else if p_min < c_min {
      format!("the provider admits as few as {p_min} {unit}; the consumer declared at least {c_min}")
    } else {
      format!(
        "the provider's upper bound is outside the consumer's: {} vs {}",
        p_max
          .map(|m| m.to_string())
          .unwrap_or_else(|| "unbounded".to_string()),
        c_max
          .map(|m| m.to_string())
          .unwrap_or_else(|| "unbounded".to_string())
      )
    };
    Out::no(Finding::wider_cardinality(
      at,
      Side::new(phrase(p_min, p_max)),
      Side::new(phrase(c_min, c_max)),
      reason,
    ))
  }

  /// Two discriminated unions (spec §3.3). Alternatives are keyed by their *discriminator
  /// literals*, not by their names: the name is the author's label and the literal is the
  /// identity, so two documents that spell the same union differently still compare.
  fn one_of_pair(&mut self, p: &ShapeNode, pc: &CoreShape, c: &ShapeNode, cc: &CoreShape, at: &str) -> Out {
    let (
      CoreShape::OneOf {
        discriminator: p_discriminator,
        alternatives: p_alternatives,
        ..
      },
      CoreShape::OneOf {
        discriminator: c_discriminator,
        alternatives: c_alternatives,
        ..
      },
    ) = (pc, cc)
    else {
      unreachable!("one_of_pair is only called with two one-of nodes");
    };

    if p_discriminator != c_discriminator {
      return Out::unknown(Finding::unreviewable(
        at,
        Side::new(phrases::describe(p)),
        Side::new(phrases::describe(c)),
        format!(
          "the two unions are discriminated on different members ('{p_discriminator}' and \
           '{c_discriminator}')"
        ),
      ));
    }

    let consumer_by_literal = by_discriminator(c_alternatives, c_discriminator);
    let mut out = Out::yes();
    for (p_name, p_alternative) in p_alternatives {
      let literals = discriminator_literals(p_alternative, p_discriminator);
      let matched: BTreeSet<&str> = literals
        .iter()
        .filter_map(|literal| {
          consumer_by_literal
            .get(&literal_key(literal))
            .map(|(name, _)| *name)
        })
        .collect();
      match (matched.len(), literals.is_empty()) {
        (1, false) => {
          let name = matched.iter().next().copied().expect("one match");
          let (_, c_alternative) = consumer_by_literal
            .values()
            .find(|(candidate, _)| *candidate == name)
            .expect("the matched alternative");
          out = out.and(self.node(p_alternative, c_alternative, &path::alternative(at, name)));
        }
        _ => {
          // The provider binds a discriminator value the consumer's contract names no
          // alternative for — "which is not narrower than anything the consumer declared".
          out = out.and(Out::no(Finding::wider_values(
            &path::alternative(at, p_name),
            Side::new(phrases::describe(p_alternative)),
            Side::new(phrases::describe(c)),
            format!(
              "the provider may produce the '{p_discriminator}' value(s) {} , which the \
               consumer's contract names no alternative for",
              phrases::literals(&literals)
            ),
          )));
        }
      }
    }
    out
  }

  /// Spec §3.2's Rule 3 for a `one-of` on the consumer's side: a union on the *right* does not
  /// decompose, so the walk resolves it by reading the provider's own discriminator binding and
  /// testing containment in the alternative it picks out.
  fn inside_alternative(&mut self, p: &ShapeNode, c: &ShapeNode, cc: &CoreShape, at: &str) -> Out {
    let CoreShape::OneOf {
      discriminator,
      alternatives,
      ..
    } = cc
    else {
      unreachable!("inside_alternative is only called with a one-of consumer node");
    };
    let literals = discriminator_literals(p, discriminator);
    let by_literal = by_discriminator(alternatives, discriminator);
    let matched: BTreeSet<&str> = literals
      .iter()
      .filter_map(|literal| by_literal.get(&literal_key(literal)).map(|(name, _)| *name))
      .collect();
    if literals.is_empty() || matched.len() != 1 {
      return Out::unknown(Finding::unreviewable(
        at,
        Side::new(phrases::describe(p)),
        Side::new(phrases::describe(c)),
        format!(
          "the provider's shape does not bind '{discriminator}' to one alternative's literal, so \
           which alternative it must be inside is not decidable here"
        ),
      ));
    }
    let name = matched.iter().next().copied().expect("one match");
    let (_, alternative) = by_literal
      .values()
      .find(|(candidate, _)| *candidate == name)
      .expect("the matched alternative");
    self.node(p, alternative, &path::alternative(at, name))
  }
}

// -----------------------------------------------------------------------------------------------
// Presence, nullability, and the residual value shape
// -----------------------------------------------------------------------------------------------

/// A node split into the three containments shape spec §8 decides by set containment.
struct Presence<'a> {
  /// `⊥ ∈ admits(node)`.
  absent: bool,
  /// `null ∈ admits(node)`.
  null: bool,
  /// What is left once `⊥` and `null` are taken out — `None` when the node admits nothing else.
  rest: Option<Cow<'a, ShapeNode>>,
}

fn decompose(node: &ShapeNode) -> Presence<'_> {
  let ShapeKind::Core(core) = &node.kind else {
    return Presence {
      absent: false,
      null: false,
      rest: Some(Cow::Borrowed(node)),
    };
  };
  match core {
    CoreShape::Optional { of } => Presence {
      absent: true,
      ..decompose(of)
    },
    CoreShape::Forbidden => Presence {
      absent: true,
      null: false,
      rest: None,
    },
    // `nullable` never wraps `optional` (shape spec §5.2), so recursing cannot re-introduce `⊥`.
    CoreShape::Nullable { of } => Presence {
      null: true,
      ..decompose(of)
    },
    CoreShape::Kind(ValueKind::Null) => Presence {
      absent: false,
      null: true,
      rest: None,
    },
    CoreShape::Equality if node.example.as_ref().is_some_and(is_null_example) => Presence {
      absent: false,
      null: true,
      rest: None,
    },
    CoreShape::AnyOf { options } if options.iter().any(Value::is_null) => {
      let rest: Vec<Value> = options.iter().filter(|o| !o.is_null()).cloned().collect();
      Presence {
        absent: false,
        null: true,
        rest: (!rest.is_empty()).then(|| {
          Cow::Owned(ShapeNode {
            example: node.example.clone().filter(|e| !is_null_example(e)),
            generator: None,
            kind: ShapeKind::Core(CoreShape::AnyOf { options: rest }),
          })
        }),
      }
    }
    // `any` admits `null` among everything else; its residue is still itself, which costs nothing
    // because a consumer `any` decides `yes` outright.
    CoreShape::Any => Presence {
      absent: false,
      null: true,
      rest: Some(Cow::Borrowed(node)),
    },
    _ => Presence {
      absent: false,
      null: false,
      rest: Some(Cow::Borrowed(node)),
    },
  }
}

fn is_null_example(example: &Example) -> bool {
  example.encoded.is_none() && example.value.is_null()
}

// -----------------------------------------------------------------------------------------------
// Comparability classes and the small decidable questions the table asks
// -----------------------------------------------------------------------------------------------

/// Whether a node has sub-shapes the walk would recurse into. `contains` does not count: its
/// entries are sub-shapes, but its comparability is opaque (shape spec §8), so the walk never
/// reaches them.
fn has_children(node: &ShapeNode) -> bool {
  matches!(
    &node.kind,
    ShapeKind::Core(
      CoreShape::Object { .. }
        | CoreShape::Array { .. }
        | CoreShape::EachLike { .. }
        | CoreShape::EachEntry { .. }
        | CoreShape::OneOf { .. }
        | CoreShape::Optional { .. }
        | CoreShape::Nullable { .. }
    )
  )
}

fn is_opaque(node: &ShapeNode) -> bool {
  match &node.kind {
    ShapeKind::Component { .. } => true,
    ShapeKind::Core(core) => matches!(core, CoreShape::Contains { .. }),
  }
}

fn is_conservative(core: &CoreShape) -> bool {
  matches!(
    core,
    CoreShape::Regex { .. }
      | CoreShape::Temporal { .. }
      | CoreShape::Include { .. }
      | CoreShape::ContentType { .. }
  )
}

/// Of the conservative operators, the ones whose values are strings — so "the container is exactly
/// wider" (`regex ⊆ string`) applies. `content-type` inspects octets by design (shape spec §4.2).
fn string_valued(core: &CoreShape) -> bool {
  matches!(
    core,
    CoreShape::Regex { .. } | CoreShape::Temporal { .. } | CoreShape::Include { .. }
  )
}

fn conservative_reason(pc: &CoreShape, cc: &CoreShape) -> String {
  match (pc, cc) {
    (CoreShape::Regex { .. }, CoreShape::Regex { .. }) => {
      "two different regexes: conservative class, no guessing".to_string()
    }
    (CoreShape::Temporal { kind: p_kind, .. }, CoreShape::Temporal { kind: c_kind, .. })
      if p_kind == c_kind =>
    {
      format!(
        "two different {} formats: conservative class, no guessing",
        p_kind.operator_name()
      )
    }
    _ => format!(
      "'{}' against '{}': conservative class, no guessing",
      pc.operator_name(),
      cc.operator_name()
    ),
  }
}

/// The JSON kinds a shape's values can take, when they are known exactly (shape spec §4.2's
/// kinds). `None` means "not pinned down by this operator", which makes the disjointness check
/// abstain rather than guess.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Class {
  Null,
  Boolean,
  Number,
  String,
  Array,
  Object,
  Bytes,
}

fn classes(node: &ShapeNode) -> Option<BTreeSet<Class>> {
  let ShapeKind::Core(core) = &node.kind else {
    return None;
  };
  let one = |class: Class| Some(BTreeSet::from([class]));
  match core {
    CoreShape::Any | CoreShape::NotEmpty => None,
    CoreShape::Equality | CoreShape::Type => node
      .example
      .as_ref()
      .map(class_of_example)
      .map(|class| BTreeSet::from([class])),
    CoreShape::Kind(kind) => one(match kind {
      ValueKind::String => Class::String,
      ValueKind::Number | ValueKind::Integer | ValueKind::Decimal => Class::Number,
      ValueKind::Boolean => Class::Boolean,
      ValueKind::Null => Class::Null,
    }),
    CoreShape::Regex { .. } | CoreShape::Temporal { .. } | CoreShape::Include { .. } | CoreShape::Semver => {
      one(Class::String)
    }
    CoreShape::ContentType { .. } => one(Class::Bytes),
    CoreShape::Object { .. } | CoreShape::EachEntry { .. } => one(Class::Object),
    CoreShape::Array { .. } | CoreShape::EachLike { .. } | CoreShape::Contains { .. } => one(Class::Array),
    CoreShape::AnyOf { options } => Some(options.iter().map(class_of_value).collect()),
    CoreShape::OneOf { alternatives, .. } => {
      let mut classes = BTreeSet::from([Class::Object]);
      // An alternative may be wrapped in `nullable` (shape spec §5.4), which widens the union to
      // admit `null` — and a class set that missed it could decide a wrong `no`.
      if alternatives
        .values()
        .any(|alternative| decompose(alternative).null)
      {
        classes.insert(Class::Null);
      }
      Some(classes)
    }
    // Presence operators are taken out before the table runs.
    CoreShape::Optional { .. } | CoreShape::Nullable { .. } | CoreShape::Forbidden => None,
  }
}

fn class_of_value(value: &Value) -> Class {
  match value {
    Value::Null => Class::Null,
    Value::Bool(_) => Class::Boolean,
    Value::Number(_) => Class::Number,
    Value::String(_) => Class::String,
    Value::Array(_) => Class::Array,
    Value::Object(_) => Class::Object,
  }
}

fn class_of_example(example: &Example) -> Class {
  if example.encoded.is_some() {
    return Class::Bytes;
  }
  class_of_value(&example.value)
}

/// The kind lattice (shape spec §8): `integer ⊂ number`, `decimal ⊂ number`, and nothing else
/// nests. `integer` and `decimal` are disjoint — the interpreter's `match:decimal` admits a number
/// *with* a fractional part.
fn kind_subset(provider: ValueKind, consumer: ValueKind) -> bool {
  provider == consumer
    || matches!(
      (provider, consumer),
      (ValueKind::Integer | ValueKind::Decimal, ValueKind::Number)
    )
}

/// `type` rewritten as the operator it is equivalent to (shape spec §4.2).
fn desugar(node: &ShapeNode) -> Option<ShapeNode> {
  let ShapeKind::Core(CoreShape::Type) = &node.kind else {
    return None;
  };
  let example = node.example.as_ref()?;
  if example.encoded.is_some() {
    return None;
  }
  let kind = match &example.value {
    Value::Null => CoreShape::Kind(ValueKind::Null),
    Value::Bool(_) => CoreShape::Kind(ValueKind::Boolean),
    Value::Number(_) => CoreShape::Kind(ValueKind::Number),
    Value::String(_) => CoreShape::Kind(ValueKind::String),
    // "An `object` example does *not* make `type` recurse into members" — an object of any
    // shape is exactly an `object` naming none, and a list of any length is an unbounded
    // `each-like` of `any`.
    Value::Object(_) => CoreShape::Object {
      members: Default::default(),
    },
    Value::Array(_) => CoreShape::EachLike {
      items: Box::new(any_node()),
      min: 0,
      max: None,
    },
  };
  Some(ShapeNode {
    example: None,
    generator: None,
    kind: ShapeKind::Core(kind),
  })
}

fn any_node() -> ShapeNode {
  ShapeNode {
    example: None,
    generator: None,
    kind: ShapeKind::Core(CoreShape::Any),
  }
}

/// Rule 2's escape hatch: the one consumer shape an unnamed provider member is still inside — "an
/// `optional` wrapping `any`, in practice never written".
fn admits_anything_or_absence(node: &ShapeNode) -> bool {
  let presence = decompose(node);
  presence.absent
    && match &presence.rest {
      // A residue of nothing means the shape admits `⊥` (and perhaps `null`) and no value at
      // all, which is the narrowest claim there is, not the widest.
      None => false,
      Some(rest) => matches!(&rest.kind, ShapeKind::Core(CoreShape::Any)),
    }
}

/// Whether every value a shape admits is non-empty, for the `not-empty` column. `None` is "not
/// decided here".
fn admits_only_non_empty(node: &ShapeNode) -> Option<bool> {
  let ShapeKind::Core(core) = &node.kind else {
    return None;
  };
  match core {
    // `is_empty` treats a number or a boolean as non-empty; `null` and `⊥` are empty.
    CoreShape::Kind(ValueKind::Number | ValueKind::Integer | ValueKind::Decimal | ValueKind::Boolean) => {
      Some(true)
    }
    CoreShape::Kind(ValueKind::String) | CoreShape::Kind(ValueKind::Null) => Some(false),
    CoreShape::Semver => Some(true),
    CoreShape::Equality => node.example.as_ref().map(|example| !is_empty_example(example)),
    CoreShape::AnyOf { options } => Some(options.iter().all(|option| !is_empty_value(option))),
    CoreShape::Array { entries } => Some(!entries.is_empty()),
    CoreShape::EachLike { min, .. } | CoreShape::EachEntry { min, .. } => Some(*min >= 1),
    CoreShape::Object { members } => Some(members.values().any(|member| !decompose(member).absent)),
    _ => None,
  }
}

fn is_empty_value(value: &Value) -> bool {
  match value {
    Value::Null => true,
    Value::String(text) => text.is_empty(),
    Value::Array(items) => items.is_empty(),
    Value::Object(members) => members.is_empty(),
    _ => false,
  }
}

fn is_empty_example(example: &Example) -> bool {
  match &example.encoded {
    Some(_) => example.value.as_str().is_some_and(str::is_empty),
    None => is_empty_value(&example.value),
  }
}

/// Every value a shape admits, when there are finitely many and they are cheap to list — which is
/// what makes `admits(P) ⊆ any-of(...)` decidable rather than merely "infinite, so no".
fn enumerate(node: &ShapeNode) -> Option<Vec<Value>> {
  let ShapeKind::Core(core) = &node.kind else {
    return None;
  };
  match core {
    CoreShape::Kind(ValueKind::Null) => Some(vec![Value::Null]),
    CoreShape::Kind(ValueKind::Boolean) => Some(vec![Value::Bool(true), Value::Bool(false)]),
    CoreShape::Equality => node
      .example
      .as_ref()
      .filter(|example| example.encoded.is_none())
      .map(|example| vec![example.value.clone()]),
    CoreShape::AnyOf { options } => Some(options.clone()),
    CoreShape::Array { entries } if entries.is_empty() => Some(vec![Value::Array(Vec::new())]),
    _ => None,
  }
}

/// `value ∈ admits(node)`, for the operators where membership is a decision rather than a
/// matching run. `None` is "not decided here" — the conservative class never reaches this.
fn admits_value(node: &ShapeNode, value: &Value) -> Option<bool> {
  let ShapeKind::Core(core) = &node.kind else {
    return None;
  };
  match core {
    CoreShape::Any => Some(true),
    CoreShape::Kind(kind) => Some(match kind {
      ValueKind::String => value.is_string(),
      ValueKind::Number => value.is_number(),
      ValueKind::Integer => value.as_f64().is_some_and(|n| n.fract() == 0.0),
      ValueKind::Decimal => value.as_f64().is_some_and(|n| n.fract() != 0.0),
      ValueKind::Boolean => value.is_boolean(),
      ValueKind::Null => value.is_null(),
    }),
    CoreShape::Equality => node
      .example
      .as_ref()
      .filter(|example| example.encoded.is_none())
      .map(|example| value_eq(&example.value, value)),
    CoreShape::AnyOf { options } => Some(options.iter().any(|option| value_eq(option, value))),
    CoreShape::NotEmpty => Some(!is_empty_value(value)),
    CoreShape::Semver => Some(
      value
        .as_str()
        .is_some_and(|text| semver::Version::parse(text).is_ok()),
    ),
    CoreShape::Optional { of } | CoreShape::Nullable { of } => {
      if value.is_null() && matches!(core, CoreShape::Nullable { .. }) {
        Some(true)
      } else {
        admits_value(of, value)
      }
    }
    CoreShape::Forbidden => Some(false),
    _ => None,
  }
}

/// `value ∈ admits(node)` decided by the engine's own matcher: the node compiled as the only slot
/// of a one-slot interaction and run against the value. Shape spec §7.4 makes the compiled plan
/// the definition of `admits`, so this cannot disagree with what a verification would decide.
fn matched_by(node: &ShapeNode, value: &Value) -> bool {
  let spec = InteractionSpec {
    description: "membership".to_string(),
    transport: None,
    states: None,
    parts: BTreeMap::from([(
      "subsumption".to_string(),
      BTreeMap::from([("value".to_string(), node.clone())]),
    )]),
    content_types: None,
    requires: None,
  };
  let plan = plan::compile(&spec, &plan::Assignment::new(), None);
  let resolver =
    plan::CapturedValues::new().capture("$.subsumption.value", plan::RuntimeValue::from_json(value));
  plan::outcome(&plan::execute(&plan, &resolver)).0 == plan::Status::Matched
}

fn admits_example(node: &ShapeNode, example: &Example) -> Option<bool> {
  if example.encoded.is_some() {
    return None;
  }
  admits_value(node, &example.value)
}

/// Structural equality with the shape language's numeric rule (shape spec §4.2): `1` and `1.0` are
/// the same value.
fn value_eq(a: &Value, b: &Value) -> bool {
  match (a, b) {
    (Value::Number(x), Value::Number(y)) => match (x.as_f64(), y.as_f64()) {
      (Some(x), Some(y)) => x == y,
      _ => x == y,
    },
    (Value::Array(x), Value::Array(y)) => x.len() == y.len() && x.iter().zip(y).all(|(x, y)| value_eq(x, y)),
    (Value::Object(x), Value::Object(y)) => {
      x.len() == y.len()
        && x
          .iter()
          .all(|(key, x)| y.get(key).is_some_and(|y| value_eq(x, y)))
    }
    _ => a == b,
  }
}

// -----------------------------------------------------------------------------------------------
// Discriminated unions
// -----------------------------------------------------------------------------------------------

/// The literal values an alternative binds its discriminator to (shape spec §5.4: `equality` or
/// `any-of`, "never an open matcher"), canonicalised so two documents agree on the key.
fn discriminator_literals(alternative: &ShapeNode, discriminator: &str) -> Vec<Value> {
  let node = decompose(alternative);
  let Some(rest) = node.rest else {
    return Vec::new();
  };
  let ShapeKind::Core(CoreShape::Object { members }) = &rest.kind else {
    return Vec::new();
  };
  let Some(tag) = members.get(discriminator) else {
    return Vec::new();
  };
  match &tag.kind {
    ShapeKind::Core(CoreShape::Equality) => tag
      .example
      .as_ref()
      .filter(|example| example.encoded.is_none())
      .map(|example| vec![example.value.clone()])
      .unwrap_or_default(),
    ShapeKind::Core(CoreShape::AnyOf { options }) => options.clone(),
    _ => Vec::new(),
  }
}

/// Literal -> (alternative name, alternative), keyed by the literal's canonical JSON text so that
/// `1` and `1.0` (shape spec §4.2's numeric rule) do not become two keys.
fn by_discriminator<'a>(
  alternatives: &'a std::collections::BTreeMap<String, ShapeNode>,
  discriminator: &str,
) -> std::collections::BTreeMap<String, (&'a str, &'a ShapeNode)> {
  let mut by_literal = std::collections::BTreeMap::new();
  for (name, alternative) in alternatives {
    for literal in discriminator_literals(alternative, discriminator) {
      by_literal.insert(literal_key(&literal), (name.as_str(), alternative));
    }
  }
  by_literal
}

fn literal_key(value: &Value) -> String {
  match value {
    Value::Number(number) => number
      .as_f64()
      .map(|n| format!("n:{n}"))
      .unwrap_or_else(|| format!("n:{number}")),
    other => other.to_string(),
  }
}

// -----------------------------------------------------------------------------------------------
// The identity floor
// -----------------------------------------------------------------------------------------------

/// Shape spec §8's identity floor: "if `P` and `C` are structurally identical nodes,
/// `admits(P) ⊆ admits(C)` is **yes**, whatever the operator, including a component operator the
/// kernel knows nothing about".
///
/// "Structurally identical" is read here as *identical in everything `admits` depends on*:
/// `generator` is a production concern, and `example` is decoration except on `equality` and
/// `type`, where shape spec §4.2 makes it the operator's parameter. Reading it more strictly would
/// lower the floor — two `datetime` nodes with the same format and different examples admit exactly
/// the same set, and calling that `unknown` would be a worse answer, not a safer one.
fn same_admits(p: &ShapeNode, c: &ShapeNode) -> bool {
  match (&p.kind, &c.kind) {
    (
      ShapeKind::Component {
        operator: p_operator,
        raw: p_raw,
      },
      ShapeKind::Component {
        operator: c_operator,
        raw: c_raw,
      },
    ) => p_operator == c_operator && p_raw == c_raw,
    (ShapeKind::Core(p_core), ShapeKind::Core(c_core)) => match (p_core, c_core) {
      (CoreShape::Equality, CoreShape::Equality) => match (&p.example, &c.example) {
        (Some(p_example), Some(c_example)) => {
          p_example.encoded == c_example.encoded && value_eq(&p_example.value, &c_example.value)
        }
        _ => false,
      },
      (CoreShape::Type, CoreShape::Type) => match (&p.example, &c.example) {
        (Some(p_example), Some(c_example)) => class_of_example(p_example) == class_of_example(c_example),
        _ => false,
      },
      (CoreShape::Object { members: p_members }, CoreShape::Object { members: c_members }) => {
        p_members.len() == c_members.len()
          && p_members
            .iter()
            .zip(c_members)
            .all(|((p_name, p_member), (c_name, c_member))| {
              p_name == c_name && same_admits(p_member, c_member)
            })
      }
      (CoreShape::Array { entries: p_entries }, CoreShape::Array { entries: c_entries })
      | (CoreShape::Contains { entries: p_entries }, CoreShape::Contains { entries: c_entries }) => {
        p_entries.len() == c_entries.len()
          && p_entries
            .iter()
            .zip(c_entries)
            .all(|(p_entry, c_entry)| same_admits(p_entry, c_entry))
      }
      (
        CoreShape::EachLike {
          items: p_items,
          min: p_min,
          max: p_max,
        },
        CoreShape::EachLike {
          items: c_items,
          min: c_min,
          max: c_max,
        },
      ) => p_min == c_min && p_max == c_max && same_admits(p_items, c_items),
      (
        CoreShape::EachEntry {
          keys: p_keys,
          values: p_values,
          min: p_min,
          max: p_max,
        },
        CoreShape::EachEntry {
          keys: c_keys,
          values: c_values,
          min: c_min,
          max: c_max,
        },
      ) => {
        p_min == c_min
          && p_max == c_max
          && match (p_keys, c_keys) {
            (None, None) => true,
            (Some(p_keys), Some(c_keys)) => same_admits(p_keys, c_keys),
            _ => false,
          }
          && same_admits(p_values, c_values)
      }
      (CoreShape::Optional { of: p_of }, CoreShape::Optional { of: c_of })
      | (CoreShape::Nullable { of: p_of }, CoreShape::Nullable { of: c_of }) => same_admits(p_of, c_of),
      (CoreShape::AnyOf { options: p_options }, CoreShape::AnyOf { options: c_options }) => {
        p_options.len() == c_options.len()
          && p_options
            .iter()
            .all(|option| c_options.iter().any(|candidate| value_eq(option, candidate)))
      }
      (
        CoreShape::OneOf {
          discriminator: p_discriminator,
          alternatives: p_alternatives,
          ..
        },
        CoreShape::OneOf {
          discriminator: c_discriminator,
          alternatives: c_alternatives,
          ..
        },
      ) => {
        p_discriminator == c_discriminator
          && p_alternatives.len() == c_alternatives.len()
          && p_alternatives.iter().zip(c_alternatives).all(
            |((p_name, p_alternative), (c_name, c_alternative))| {
              p_name == c_name && same_admits(p_alternative, c_alternative)
            },
          )
      }
      // The scalar leaves carry their whole parameterisation in the enum.
      (p_core, c_core) => p_core == c_core,
    },
    _ => false,
  }
}
