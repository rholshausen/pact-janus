//! The plan interpreter (plan task 3.4, plan-grammar spec §2.3-§2.4): executes a compiled plan
//! (task 3.3) against a [`Resolver`], producing a result-annotated tree ([`Executed`]) that both
//! [`outcome`] (a corpus-style verdict) and, eventually, `explain --executed`'s renderer (task
//! 3.6) can read without re-executing anything.
//!
//! **What "container/if/for-each/and/or all render as a boolean" resolves.** Spec §2.3 states a
//! container's result is the conjunction of its children's, `ok` or `error`; the worked executed
//! example (`order-payload-plan.md` §3) prints every container, `if` and `for-each` as
//! `BOOL(true/false)`, never bare `ok`/`error`, and only the *leaf* action that actually failed
//! carries a message (`ERROR(...)`). This module resolves that by giving conjunction-style nodes
//! (`container`, `if`, `or`, `and`, `for-each`) a [`RuntimeValue::Bool`] result summarising
//! whether their subtree failed, and reserving [`NodeResult::Error`] for the node that actually
//! has something to say: a failed `match:*`/`expect:*`, or `error` itself. [`outcome`] then only
//! ever needs to collect real `Error`s — nothing double-reports a descendant's failure at every
//! ancestor.
//!
//! **What is deliberately not here**: `explain`'s text rendering of [`Executed`] (task 3.6) and
//! the v1-v4 matching-rule compiler (task 3.5) — this interpreter runs whatever plan it is given,
//! from either compiler.

use super::model::{Literal, Node, NodeKind, Plan};
use super::resolve::Resolver;
use super::value::{RuntimeValue, navigate};
use serde_json::Number;

/// What executing one node produced (spec §2.3).
#[derive(Debug, Clone, PartialEq)]
pub enum NodeResult {
  Ok,
  Value(RuntimeValue),
  Error { message: String, path: Option<String> },
}

/// A plan node together with what executing it produced. `result: None` means this node was
/// **not executed** — a lazy branch a condition skipped (spec §2.4, §3.2).
#[derive(Debug, Clone, PartialEq)]
pub struct Executed {
  pub kind: ExecutedKind,
  pub result: Option<NodeResult>,
}

/// Mirrors [`NodeKind`], with children recursively [`Executed`] rather than [`Node`]. `for-each`
/// and `match:contains` legitimately have more children here than the [`Node`] they were compiled
/// from — this tree records what actually happened (every element iterated, every entry tried),
/// not the shape of the static plan.
#[derive(Debug, Clone, PartialEq)]
pub enum ExecutedKind {
  Container {
    label: Option<String>,
    children: Vec<Executed>,
  },
  Action {
    name: String,
    children: Vec<Executed>,
  },
  Value(Literal),
  Resolve {
    path: String,
  },
  ResolveCurrent {
    path: String,
  },
  Pipeline {
    children: Vec<Executed>,
  },
  Splat {
    children: Vec<Executed>,
  },
  Annotation {
    text: String,
  },
}

struct Ctx<'a> {
  resolver: &'a dyn Resolver,
  current: Vec<RuntimeValue>,
}

/// Execute a compiled plan against `resolver`.
pub fn execute(plan: &Plan, resolver: &dyn Resolver) -> Executed {
  let mut ctx = Ctx {
    resolver,
    current: Vec::new(),
  };
  run(&plan.root, &mut ctx)
}

/// The corpus-style verdict (plan-grammar spec §6.1's `ExpectedResult`): matched iff no node in
/// the tree produced an error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
  Matched,
  Mismatched,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Mismatch {
  pub path: Option<String>,
  pub message: String,
  /// The nearest enclosing action's name, when the error came from one — a `match:*`/`expect:*`
  /// failure always has one; a bare `%error` node may not.
  pub action: Option<String>,
}

/// Extract the matched/mismatched verdict and every mismatch from an executed plan.
pub fn outcome(executed: &Executed) -> (Status, Vec<Mismatch>) {
  let mut mismatches = Vec::new();
  collect_mismatches(executed, None, &mut mismatches);
  let status = if mismatches.is_empty() {
    Status::Matched
  } else {
    Status::Mismatched
  };
  (status, mismatches)
}

fn collect_mismatches(executed: &Executed, enclosing_action: Option<&str>, out: &mut Vec<Mismatch>) {
  let action_name = match &executed.kind {
    ExecutedKind::Action { name, .. } => Some(name.as_str()),
    _ => enclosing_action,
  };
  if let Some(NodeResult::Error { message, path }) = &executed.result {
    out.push(Mismatch {
      path: path.clone(),
      message: message.clone(),
      action: action_name.map(str::to_string),
    });
  }
  for child in children_of(executed) {
    collect_mismatches(child, action_name, out);
  }
}

fn children_of(executed: &Executed) -> &[Executed] {
  match &executed.kind {
    ExecutedKind::Container { children, .. }
    | ExecutedKind::Action { children, .. }
    | ExecutedKind::Pipeline { children }
    | ExecutedKind::Splat { children } => children,
    _ => &[],
  }
}

// --- the walk ---

fn run(node: &Node, ctx: &mut Ctx) -> Executed {
  match &node.kind {
    NodeKind::Container { label, children } => {
      let executed_children: Vec<Executed> = children.iter().map(|c| run(c, ctx)).collect();
      let failed = executed_children.iter().any(child_failed);
      Executed {
        kind: ExecutedKind::Container {
          label: label.clone(),
          children: executed_children,
        },
        result: Some(NodeResult::Value(RuntimeValue::Bool(!failed))),
      }
    }
    NodeKind::Value(literal) => Executed {
      result: Some(NodeResult::Value(RuntimeValue::from_literal(literal))),
      kind: ExecutedKind::Value(literal.clone()),
    },
    NodeKind::Resolve { path } => {
      let value = ctx.resolver.resolve(path);
      Executed {
        kind: ExecutedKind::Resolve { path: path.clone() },
        result: Some(NodeResult::Value(value)),
      }
    }
    NodeKind::ResolveCurrent { path } => {
      let base = ctx.current.last().cloned().unwrap_or(RuntimeValue::Absent);
      let suffix = path.strip_prefix("~>").unwrap_or(path);
      let value = navigate(&base, suffix);
      Executed {
        kind: ExecutedKind::ResolveCurrent { path: path.clone() },
        result: Some(NodeResult::Value(value)),
      }
    }
    NodeKind::Annotation { text } => Executed {
      kind: ExecutedKind::Annotation { text: text.clone() },
      result: None,
    },
    NodeKind::Pipeline { children } => run_pipeline(children, ctx),
    NodeKind::Splat { children } => {
      let executed_children: Vec<Executed> = children.iter().map(|c| run(c, ctx)).collect();
      let result = executed_children.first().and_then(|c| c.result.clone());
      Executed {
        kind: ExecutedKind::Splat {
          children: executed_children,
        },
        result,
      }
    }
    NodeKind::Action { name, children } => run_action(name, children, ctx),
  }
}

/// Deep-clone `node` into an [`Executed`] with `result: None` throughout — a lazy branch a
/// condition did not take (spec §3.2: "a node with no result was not executed").
fn skip(node: &Node) -> Executed {
  let kind = match &node.kind {
    NodeKind::Container { label, children } => ExecutedKind::Container {
      label: label.clone(),
      children: children.iter().map(skip).collect(),
    },
    NodeKind::Action { name, children } => ExecutedKind::Action {
      name: name.clone(),
      children: children.iter().map(skip).collect(),
    },
    NodeKind::Value(literal) => ExecutedKind::Value(literal.clone()),
    NodeKind::Resolve { path } => ExecutedKind::Resolve { path: path.clone() },
    NodeKind::ResolveCurrent { path } => ExecutedKind::ResolveCurrent { path: path.clone() },
    NodeKind::Pipeline { children } => ExecutedKind::Pipeline {
      children: children.iter().map(skip).collect(),
    },
    NodeKind::Splat { children } => ExecutedKind::Splat {
      children: children.iter().map(skip).collect(),
    },
    NodeKind::Annotation { text } => ExecutedKind::Annotation { text: text.clone() },
  };
  Executed { kind, result: None }
}

fn child_failed(executed: &Executed) -> bool {
  matches!(
    &executed.result,
    Some(NodeResult::Error { .. }) | Some(NodeResult::Value(RuntimeValue::Bool(false)))
  )
}

fn truthy(result: &NodeResult) -> bool {
  matches!(result, NodeResult::Ok) || matches!(result, NodeResult::Value(RuntimeValue::Bool(true)))
}

fn value_of(executed: &Executed) -> RuntimeValue {
  match &executed.result {
    Some(NodeResult::Value(v)) => v.clone(),
    _ => RuntimeValue::Absent,
  }
}

/// What `for-each` iterates over (plan-grammar spec §5.2): an `each-like`'s array, element by
/// element, or an `each-entry`'s object, as [`RuntimeValue::Entry`] pairs in key order (a
/// `BTreeMap`'s own iteration order) so `resolve-current`'s `.key`/`.value` suffixes have
/// something to address (spec §2.2's `entry` value kind).
fn iteration_items(value: RuntimeValue) -> Vec<RuntimeValue> {
  match value {
    RuntimeValue::Array(items) => items,
    RuntimeValue::Object(members) => members
      .into_iter()
      .map(|(key, value)| RuntimeValue::Entry {
        key,
        value: Box::new(value),
      })
      .collect(),
    _ => Vec::new(),
  }
}

/// The path a mismatch should be blamed on: the first child, when it is a resolve of some kind —
/// true of every `match:*`/`expect:*`/`check:*` action this compiler ever emits (plan task 3.3
/// §5.2's table always resolves the value first).
fn locus(children: &[Executed]) -> Option<String> {
  children.first().and_then(|c| match &c.kind {
    ExecutedKind::Resolve { path } | ExecutedKind::ResolveCurrent { path } => Some(path.clone()),
    _ => None,
  })
}

fn run_pipeline(children: &[Node], ctx: &mut Ctx) -> Executed {
  let mut executed = Vec::with_capacity(children.len());
  let mut pushed = 0usize;
  let mut last_result = NodeResult::Ok;
  for child in children {
    let e = run(child, ctx);
    last_result = e.result.clone().unwrap_or(NodeResult::Ok);
    if let NodeResult::Value(v) = &last_result {
      ctx.current.push(v.clone());
      pushed += 1;
    }
    executed.push(e);
  }
  for _ in 0..pushed {
    ctx.current.pop();
  }
  Executed {
    kind: ExecutedKind::Pipeline { children: executed },
    result: Some(last_result),
  }
}

fn run_action(name: &str, children: &[Node], ctx: &mut Ctx) -> Executed {
  match name {
    "if" => run_if(children, ctx),
    "or" => run_or(children, ctx),
    "for-each" => run_for_each(children, ctx),
    "match:contains" => run_contains(children, ctx),
    _ => {
      let executed_children: Vec<Executed> = children.iter().map(|c| run(c, ctx)).collect();
      let result = dispatch(name, &executed_children);
      Executed {
        kind: ExecutedKind::Action {
          name: name.to_string(),
          children: executed_children,
        },
        result: Some(result),
      }
    }
  }
}

/// `if` (spec §2.4): the condition always executes; exactly one branch does. Two children means
/// "no explicit else" — an untaken condition is `ok` (plan task 3.3's compiler relies on this for
/// `optional`'s absent case).
fn run_if(children: &[Node], ctx: &mut Ctx) -> Executed {
  let cond = run(&children[0], ctx);
  let cond_truthy = cond.result.as_ref().is_some_and(truthy);
  let mut executed = vec![cond];
  let result = if cond_truthy {
    let branch = run(&children[1], ctx);
    let failed = child_failed(&branch);
    executed.push(branch);
    if let Some(else_branch) = children.get(2) {
      executed.push(skip(else_branch));
    }
    NodeResult::Value(RuntimeValue::Bool(!failed))
  } else if let Some(else_branch) = children.get(2) {
    executed.push(skip(&children[1]));
    let branch = run(else_branch, ctx);
    let failed = child_failed(&branch);
    executed.push(branch);
    NodeResult::Value(RuntimeValue::Bool(!failed))
  } else {
    executed.push(skip(&children[1]));
    NodeResult::Ok
  };
  Executed {
    kind: ExecutedKind::Action {
      name: "if".to_string(),
      children: executed,
    },
    result: Some(result),
  }
}

/// `or` (spec §2.4): lazy, stops at the first success.
fn run_or(children: &[Node], ctx: &mut Ctx) -> Executed {
  let mut executed = Vec::with_capacity(children.len());
  let mut success = false;
  for child in children {
    if success {
      executed.push(skip(child));
      continue;
    }
    let e = run(child, ctx);
    if e.result.as_ref().is_some_and(truthy) {
      success = true;
    }
    executed.push(e);
  }
  Executed {
    kind: ExecutedKind::Action {
      name: "or".to_string(),
      children: executed,
    },
    result: Some(NodeResult::Value(RuntimeValue::Bool(success))),
  }
}

/// `for-each` (spec §5.2): the source `splat` is executed once for its array; the item template
/// is executed once per element, each with that element pushed as the current item
/// (`resolve-current`'s addressee). An empty array is a vacuous success (spec's own note: the
/// cardinality assertion beside it, not this loop, carries the "0 items" verdict).
fn run_for_each(children: &[Node], ctx: &mut Ctx) -> Executed {
  let (splat_executed, items) = match &children[0].kind {
    NodeKind::Splat {
      children: splat_children,
    } => {
      let inner = run(&splat_children[0], ctx);
      let items = iteration_items(value_of(&inner));
      let splat = Executed {
        kind: ExecutedKind::Splat {
          children: vec![inner],
        },
        result: Some(NodeResult::Value(RuntimeValue::Array(items.clone()))),
      };
      (splat, items)
    }
    _ => {
      let e = run(&children[0], ctx);
      let items = iteration_items(value_of(&e));
      (e, items)
    }
  };

  let template = &children[1];
  let mut any_failed = false;
  let mut item_executions = Vec::with_capacity(items.len());
  for item in items {
    ctx.current.push(item);
    let executed_item = run(template, ctx);
    ctx.current.pop();
    if child_failed(&executed_item) {
      any_failed = true;
    }
    item_executions.push(executed_item);
  }

  let mut all_children = vec![splat_executed];
  all_children.extend(item_executions);
  Executed {
    kind: ExecutedKind::Action {
      name: "for-each".to_string(),
      children: all_children,
    },
    result: Some(NodeResult::Value(RuntimeValue::Bool(!any_failed))),
  }
}

/// `match:contains` (spec §4.3, §5.2): each entry must claim a *distinct* element (spec §4.3),
/// found here by a greedy first-fit search — not a full bipartite matcher, which is more rigour
/// than an operator spec §8 already calls opaque needs from a prototype interpreter.
fn run_contains(children: &[Node], ctx: &mut Ctx) -> Executed {
  let resolve_executed = run(&children[0], ctx);
  let array = match value_of(&resolve_executed) {
    RuntimeValue::Array(items) => items,
    _ => Vec::new(),
  };
  let mut used = vec![false; array.len()];
  let mut executed_entries = Vec::with_capacity(children.len().saturating_sub(1));
  let mut any_failed = false;

  for entry_node in &children[1..] {
    let mut matched = None;
    for (index, item) in array.iter().enumerate() {
      if used[index] {
        continue;
      }
      ctx.current.push(item.clone());
      let attempt = run(entry_node, ctx);
      ctx.current.pop();
      let ok = !child_failed(&attempt);
      matched = Some((index, attempt));
      if ok {
        break;
      }
    }
    match matched {
      Some((index, attempt)) if !child_failed(&attempt) => {
        used[index] = true;
        executed_entries.push(attempt);
      }
      Some((_, attempt)) => {
        any_failed = true;
        executed_entries.push(attempt);
      }
      None => {
        any_failed = true;
        executed_entries.push(skip(entry_node));
      }
    }
  }

  let mut all_children = vec![resolve_executed];
  all_children.extend(executed_entries);
  let result = if any_failed {
    NodeResult::Error {
      message: "not every entry found a distinct matching element".to_string(),
      path: None,
    }
  } else {
    NodeResult::Value(RuntimeValue::Bool(true))
  };
  Executed {
    kind: ExecutedKind::Action {
      name: "match:contains".to_string(),
      children: all_children,
    },
    result: Some(result),
  }
}

/// Every action that is not given special-cased tree-shape handling above: execute all children
/// eagerly (spec §2.4's default), then compute this action's own result from them.
///
/// Unknown/namespaced names fall through to a plain error result rather than the engine
/// protocol's `interaction-invalid`/`component-unavailable`/`component-failed` taxonomy
/// (plan-grammar spec §8) — that taxonomy is a protocol-boundary concern (design 2.1) this
/// in-process interpreter does not yet sit behind.
fn dispatch(name: &str, children: &[Executed]) -> NodeResult {
  match name {
    "and" => {
      if children.is_empty() {
        // The empty conjunction: plan task 3.3's compiler emits this as the explicit no-op branch
        // where the grammar needs a positional node but the shape is trivially satisfied.
        return NodeResult::Ok;
      }
      if children.iter().any(child_failed) {
        NodeResult::Value(RuntimeValue::Bool(false))
      } else {
        NodeResult::Ok
      }
    }
    "error" => NodeResult::Error {
      message: plain_text(&value_of(&children[0])),
      path: None,
    },
    "apply" | "tee" => {
      // Neither has a worked example anywhere in the specs; this is a documented placeholder
      // pending a real use (a component fragment, or design 3.5's legacy compiler) rather than a
      // guess dressed up as a decision.
      children
        .last()
        .and_then(|c| c.result.clone())
        .unwrap_or(NodeResult::Ok)
    }
    "join" => NodeResult::Value(RuntimeValue::String(
      children.iter().map(|c| plain_text(&value_of(c))).collect(),
    )),
    "join-with" => {
      let separator = plain_text(&value_of(&children[0]));
      let joined = children[1..]
        .iter()
        .map(|c| plain_text(&value_of(c)))
        .collect::<Vec<_>>()
        .join(&separator);
      NodeResult::Value(RuntimeValue::String(joined))
    }
    "length" => NodeResult::Value(RuntimeValue::Number(Number::from(length_of(&value_of(
      &children[0],
    ))))),
    "lower-case" => NodeResult::Value(RuntimeValue::String(
      plain_text(&value_of(&children[0])).to_lowercase(),
    )),
    "upper-case" => NodeResult::Value(RuntimeValue::String(
      plain_text(&value_of(&children[0])).to_uppercase(),
    )),
    "to-string" => NodeResult::Value(RuntimeValue::String(plain_text(&value_of(&children[0])))),

    "check:exists" => NodeResult::Value(RuntimeValue::Bool(!matches!(
      value_of(&children[0]),
      RuntimeValue::Absent
    ))),
    "check:null" => NodeResult::Value(RuntimeValue::Bool(matches!(
      value_of(&children[0]),
      RuntimeValue::Null
    ))),
    "check:equals" => NodeResult::Value(RuntimeValue::Bool(runtime_eq(
      &value_of(&children[0]),
      &value_of(&children[1]),
    ))),

    "expect:absent" => {
      let value = value_of(&children[0]);
      if matches!(value, RuntimeValue::Absent) {
        NodeResult::Ok
      } else {
        NodeResult::Error {
          message: format!("Expected no value but got {}", display(&value)),
          path: locus(children),
        }
      }
    }
    "expect:not-empty" => {
      let value = value_of(&children[0]);
      if is_empty(&value) {
        NodeResult::Error {
          message: format!("Expected a non-empty value but got {}", display(&value)),
          path: locus(children),
        }
      } else {
        NodeResult::Ok
      }
    }
    "expect:empty" => {
      let value = value_of(&children[0]);
      if is_empty(&value) {
        NodeResult::Ok
      } else {
        NodeResult::Error {
          message: format!("Expected an empty value but got {}", display(&value)),
          path: locus(children),
        }
      }
    }
    "expect:count" | "expect:entries" => {
      let value = value_of(&children[0]);
      let expected = as_u64(&value_of(&children[1]));
      match (length_of_opt(&value), expected) {
        (Some(len), Some(n)) if len == n => NodeResult::Ok,
        (Some(len), _) => NodeResult::Error {
          message: format!("Expected exactly {} item(s) but got {len}", expected.unwrap_or(0)),
          path: locus(children),
        },
        (None, _) => NodeResult::Error {
          message: format!("Expected a countable value but got {}", display(&value)),
          path: locus(children),
        },
      }
    }
    "expect:size" => {
      let value = value_of(&children[0]);
      let min = as_u64(&value_of(&children[1])).unwrap_or(0);
      let max = as_u64(&value_of(&children[2]));
      match length_of_opt(&value) {
        Some(len) if len >= min && max.map(|m| len <= m).unwrap_or(true) => NodeResult::Ok,
        Some(len) => NodeResult::Error {
          message: format!(
            "Expected at least {min} item(s){} but got {len}",
            max.map(|m| format!(" and at most {m}")).unwrap_or_default()
          ),
          path: locus(children),
        },
        None => NodeResult::Error {
          message: format!("Expected a countable value but got {}", display(&value)),
          path: locus(children),
        },
      }
    }
    "expect:only-entries" => {
      let value = value_of(&children[0]);
      let allowed: Vec<String> = children[1..].iter().map(|c| plain_text(&value_of(c))).collect();
      match value {
        RuntimeValue::Object(members) => {
          let extra: Vec<&str> = members
            .keys()
            .filter(|k| !allowed.iter().any(|a| a == *k))
            .map(String::as_str)
            .collect();
          if extra.is_empty() {
            NodeResult::Ok
          } else {
            NodeResult::Error {
              message: format!("Unexpected member(s): {}", extra.join(", ")),
              path: locus(children),
            }
          }
        }
        other => NodeResult::Error {
          message: format!("Expected an object but got {}", display(&other)),
          path: locus(children),
        },
      }
    }

    "match:any" => match value_of(&children[0]) {
      RuntimeValue::Absent => NodeResult::Error {
        message: "Expected a value but it was absent".to_string(),
        path: locus(children),
      },
      _ => NodeResult::Value(RuntimeValue::Bool(true)),
    },
    "match:equality" => {
      let (actual, expected) = (value_of(&children[0]), value_of(&children[1]));
      if runtime_eq(&actual, &expected) {
        NodeResult::Value(RuntimeValue::Bool(true))
      } else {
        NodeResult::Error {
          message: format!("Expected {} to equal {}", display(&actual), display(&expected)),
          path: locus(children),
        }
      }
    }
    "match:type" => {
      let (actual, expected) = (value_of(&children[0]), value_of(&children[1]));
      if !matches!(actual, RuntimeValue::Absent) && kind_of(&actual) == kind_of(&expected) {
        NodeResult::Value(RuntimeValue::Bool(true))
      } else {
        NodeResult::Error {
          message: format!("Expected a {} but got {}", kind_of(&expected), display(&actual)),
          path: locus(children),
        }
      }
    }
    "match:string" => match_kind(children, |v| matches!(v, RuntimeValue::String(_)), "string"),
    "match:boolean" => match_kind(children, |v| matches!(v, RuntimeValue::Bool(_)), "boolean"),
    "match:null" => match_kind(children, |v| matches!(v, RuntimeValue::Null), "null"),
    "match:number" => match_kind(children, |v| matches!(v, RuntimeValue::Number(_)), "number"),
    "match:integer" => match_kind(
      children,
      |v| matches!(v, RuntimeValue::Number(n) if is_integer(n)),
      "integer",
    ),
    "match:decimal" => match_kind(
      children,
      |v| matches!(v, RuntimeValue::Number(n) if !is_integer(n)),
      "decimal",
    ),
    "match:regex" => {
      let value = value_of(&children[0]);
      let pattern = plain_text(&value_of(&children[1]));
      match as_matchable_text(&value) {
        None => NodeResult::Error {
          message: format!("Expected a string but got {}", display(&value)),
          path: locus(children),
        },
        Some(text) => match regex::Regex::new(&pattern) {
          Err(err) => NodeResult::Error {
            message: format!("Invalid regex '{pattern}': {err}"),
            path: locus(children),
          },
          Ok(re) if re.is_match(&text) => NodeResult::Value(RuntimeValue::Bool(true)),
          Ok(_) => NodeResult::Error {
            message: format!("Expected '{text}' to match '{pattern}'"),
            path: locus(children),
          },
        },
      }
    }
    "match:datetime" => match_temporal(children, "yyyy-MM-dd'T'HH:mm:ssXXX", "datetime"),
    "match:date" => match_temporal(children, "yyyy-MM-dd", "date"),
    "match:time" => match_temporal(children, "HH:mm:ss", "time"),
    "match:include" => {
      let value = value_of(&children[0]);
      let substring = plain_text(&value_of(&children[1]));
      match as_text(&value) {
        Some(text) if text.contains(&substring) => NodeResult::Value(RuntimeValue::Bool(true)),
        Some(text) => NodeResult::Error {
          message: format!("Expected '{text}' to include '{substring}'"),
          path: locus(children),
        },
        None => NodeResult::Error {
          message: format!("Expected a string but got {}", display(&value)),
          path: locus(children),
        },
      }
    }
    "match:content-type" => {
      let value = value_of(&children[0]);
      let expected = plain_text(&value_of(&children[1]));
      match detect_content_type(&value) {
        Some(detected) if detected == expected => NodeResult::Value(RuntimeValue::Bool(true)),
        Some(detected) => NodeResult::Error {
          message: format!("Expected content type '{expected}' but detected '{detected}'"),
          path: locus(children),
        },
        None => NodeResult::Error {
          message: format!("Could not detect a content type for {}", display(&value)),
          path: locus(children),
        },
      }
    }
    "match:semver" => {
      let value = value_of(&children[0]);
      match as_text(&value) {
        Some(text) if semver::Version::parse(&text).is_ok() => NodeResult::Value(RuntimeValue::Bool(true)),
        Some(text) => NodeResult::Error {
          message: format!("Expected '{text}' to be a semantic version"),
          path: locus(children),
        },
        None => NodeResult::Error {
          message: format!("Expected a string but got {}", display(&value)),
          path: locus(children),
        },
      }
    }
    "match:any-of" => {
      let value = value_of(&children[0]);
      let options: Vec<RuntimeValue> = children[1..].iter().map(value_of).collect();
      if options.iter().any(|o| runtime_eq(o, &value)) {
        NodeResult::Value(RuntimeValue::Bool(true))
      } else {
        let names = options.iter().map(display).collect::<Vec<_>>().join(", ");
        NodeResult::Error {
          message: format!("Expected {} to be one of {names}", display(&value)),
          path: locus(children),
        }
      }
    }
    "match:min-type" => {
      match_type_with_bounds(children, Some(as_u64(&value_of(&children[2])).unwrap_or(0)), None)
    }
    "match:max-type" => {
      match_type_with_bounds(children, None, Some(as_u64(&value_of(&children[2])).unwrap_or(0)))
    }
    "match:min-max-type" => match_type_with_bounds(
      children,
      Some(as_u64(&value_of(&children[2])).unwrap_or(0)),
      Some(as_u64(&value_of(&children[3])).unwrap_or(0)),
    ),

    "match:array-contains" => {
      // Legacy-only (spec §4.4): reachable once design 3.5's compiler emits it, not yet.
      let value = value_of(&children[0]);
      let target = value_of(&children[1]);
      match &value {
        RuntimeValue::Array(items) if items.iter().any(|i| runtime_eq(i, &target)) => {
          NodeResult::Value(RuntimeValue::Bool(true))
        }
        RuntimeValue::Array(_) => NodeResult::Error {
          message: format!("Expected the array to contain {}", display(&target)),
          path: locus(children),
        },
        _ => NodeResult::Error {
          message: format!("Expected an array but got {}", display(&value)),
          path: locus(children),
        },
      }
    }

    "match:header-value" => {
      let value = value_of(&children[0]);
      let expected = plain_text(&value_of(&children[1]));
      match as_text(&value) {
        None => NodeResult::Error {
          message: format!("Expected a string but got {}", display(&value)),
          path: locus(children),
        },
        Some(actual) if header_values_match(&expected, &actual) => {
          NodeResult::Value(RuntimeValue::Bool(true))
        }
        Some(actual) => NodeResult::Error {
          message: format!("Expected '{actual}' to equal '{expected}'"),
          path: locus(children),
        },
      }
    }

    _ => NodeResult::Error {
      message: format!("unknown or unavailable action '{name}'"),
      path: None,
    },
  }
}

/// `match:min-type`/`match:max-type`/`match:min-max-type` (legacy only, plan task 3.5's
/// `MinType`/`MaxType`/`MinMaxType`): a `match:type` check, plus a collection-size bound enforced
/// only when the resolved value actually is a collection. That guard is what lets a cascaded
/// `MinType` (plan task 3.5's compiler reaches every descendant of the path it was declared on)
/// pass through scalar descendants as a plain type check instead of wrongly re-applying an
/// ancestor's own cardinality bound to them.
fn match_type_with_bounds(children: &[Executed], min: Option<u64>, max: Option<u64>) -> NodeResult {
  let (actual, expected) = (value_of(&children[0]), value_of(&children[1]));
  if matches!(actual, RuntimeValue::Absent) || kind_of(&actual) != kind_of(&expected) {
    return NodeResult::Error {
      message: format!("Expected a {} but got {}", kind_of(&expected), display(&actual)),
      path: locus(children),
    };
  }
  match length_of_opt(&actual) {
    Some(len) if min.is_some_and(|min| len < min) => NodeResult::Error {
      message: format!("Expected at least {} item(s) but got {len}", min.unwrap_or(0)),
      path: locus(children),
    },
    Some(len) if max.is_some_and(|max| len > max) => NodeResult::Error {
      message: format!("Expected at most {} item(s) but got {len}", max.unwrap_or(0)),
      path: locus(children),
    },
    _ => NodeResult::Value(RuntimeValue::Bool(true)),
  }
}

fn match_kind(
  children: &[Executed],
  predicate: impl Fn(&RuntimeValue) -> bool,
  kind_name: &str,
) -> NodeResult {
  let value = value_of(&children[0]);
  if predicate(&value) {
    NodeResult::Value(RuntimeValue::Bool(true))
  } else {
    NodeResult::Error {
      message: format!("Expected a {kind_name} but got {}", display(&value)),
      path: locus(children),
    }
  }
}

/// `format` absent means "any ISO-8601 string" (shape spec §4.2); a single representative pattern
/// stands in for the family of equivalent ISO-8601 spellings, which is a simplification a
/// prototype can afford and a golden corpus (task 3.7) is the place to prove or disprove.
fn match_temporal(children: &[Executed], default_format: &str, kind_name: &str) -> NodeResult {
  let value = value_of(&children[0]);
  let format = children
    .get(1)
    .map(|c| plain_text(&value_of(c)))
    .unwrap_or_else(|| default_format.to_string());
  match as_text(&value) {
    None => NodeResult::Error {
      message: format!("Expected a string but got {}", display(&value)),
      path: locus(children),
    },
    Some(text) => match pact_models::time_utils::validate_datetime(&text, &format) {
      Ok(()) => NodeResult::Value(RuntimeValue::Bool(true)),
      Err(err) => NodeResult::Error {
        message: format!("'{text}' is not a {kind_name} in format '{format}': {err}"),
        path: locus(children),
      },
    },
  }
}

/// A minimal, in-kernel content-type sniffer, standing in for the real content component this
/// operator's semantics belong to (design 2.6, plan task 4.2). Recognising it as a stand-in
/// rather than a finished implementation is exactly the finding task 3.8's kernel-boundary review
/// exists to make.
fn detect_content_type(value: &RuntimeValue) -> Option<String> {
  let text = match value {
    RuntimeValue::Bytes(bytes) => std::str::from_utf8(bytes).ok().map(str::trim),
    RuntimeValue::String(s) => Some(s.trim()),
    _ => None,
  }?;
  if text.starts_with('{') || text.starts_with('[') {
    Some("application/json".to_string())
  } else if text.starts_with('<') {
    Some("application/xml".to_string())
  } else {
    None
  }
}

/// `match:header-value` (legacy only, plan task 3.5): v1-v4's default header comparison is not
/// plain string equality. A MIME-shaped value (`type/subtype; param=value; ...`) compares the base
/// type case-insensitively and requires every parameter named in `expected` to be present in
/// `actual` with a case-insensitively equal value — extra parameters on the actual side, or a
/// different parameter order, don't fail it (spec test cases `matches content type with charset`,
/// `... with parameters in different order`, `content type parameters do not match`). Otherwise, a
/// comma-separated value tolerates whitespace around the commas (`whitespace after comma
/// different`); anything else is exact, case-sensitive equality (`header value is different case`).
fn header_values_match(expected: &str, actual: &str) -> bool {
  if expected.contains(';') || actual.contains(';') {
    let (expected_type, expected_params) = mime_parts(expected);
    let (actual_type, actual_params) = mime_parts(actual);
    return expected_type.eq_ignore_ascii_case(&actual_type)
      && expected_params.iter().all(|(key, value)| {
        actual_params
          .iter()
          .any(|(k, v)| k.eq_ignore_ascii_case(key) && v.eq_ignore_ascii_case(value))
      });
  }
  if expected.contains(',') || actual.contains(',') {
    fn split(s: &str) -> Vec<&str> {
      s.split(',').map(str::trim).collect()
    }
    return split(expected) == split(actual);
  }
  expected == actual
}

fn mime_parts(value: &str) -> (String, Vec<(String, String)>) {
  let mut parts = value.split(';').map(str::trim);
  let base = parts.next().unwrap_or_default().to_string();
  let params = parts
    .filter_map(|p| p.split_once('='))
    .map(|(k, v)| (k.trim().to_string(), v.trim().trim_matches('"').to_string()))
    .collect();
  (base, params)
}

fn is_empty(value: &RuntimeValue) -> bool {
  match value {
    RuntimeValue::String(s) => s.is_empty(),
    RuntimeValue::Array(items) => items.is_empty(),
    RuntimeValue::Object(members) => members.is_empty(),
    RuntimeValue::Bytes(bytes) => bytes.is_empty(),
    RuntimeValue::Absent | RuntimeValue::Null => true,
    _ => false,
  }
}

fn length_of_opt(value: &RuntimeValue) -> Option<u64> {
  match value {
    RuntimeValue::String(s) => Some(s.chars().count() as u64),
    RuntimeValue::Array(items) => Some(items.len() as u64),
    RuntimeValue::Object(members) => Some(members.len() as u64),
    RuntimeValue::Bytes(bytes) => Some(bytes.len() as u64),
    _ => None,
  }
}

fn length_of(value: &RuntimeValue) -> u64 {
  length_of_opt(value).unwrap_or(0)
}

fn as_u64(value: &RuntimeValue) -> Option<u64> {
  match value {
    RuntimeValue::Number(n) => n.as_u64().or_else(|| n.as_f64().map(|f| f as u64)),
    _ => None,
  }
}

fn as_text(value: &RuntimeValue) -> Option<String> {
  match value {
    RuntimeValue::String(s) => Some(s.clone()),
    RuntimeValue::Bytes(bytes) => std::str::from_utf8(bytes).ok().map(str::to_string),
    _ => None,
  }
}

/// [`as_text`], widened to numbers and booleans (v1-v4's `regex` matcher applies to a JSON number
/// or boolean body value too — spec test case `body/matches with integers` regexes a bare `4`).
fn as_matchable_text(value: &RuntimeValue) -> Option<String> {
  match value {
    RuntimeValue::Number(_) | RuntimeValue::Bool(_) => Some(plain_text(value)),
    other => as_text(other),
  }
}

fn kind_of(value: &RuntimeValue) -> &'static str {
  match value {
    RuntimeValue::Absent => "absent",
    RuntimeValue::Null => "null",
    RuntimeValue::Bool(_) => "boolean",
    RuntimeValue::Number(_) => "number",
    RuntimeValue::String(_) => "string",
    RuntimeValue::Array(_) => "array",
    RuntimeValue::Object(_) => "object",
    RuntimeValue::Bytes(_) => "bytes",
    RuntimeValue::Entry { .. } => "entry",
  }
}

fn is_integer(n: &Number) -> bool {
  n.is_i64() || n.is_u64() || n.as_f64().map(|f| f.fract() == 0.0).unwrap_or(false)
}

/// Structural equality with the shape language's numeric rule (shape spec §4.2): `1` and `1.0`
/// are the same value.
fn runtime_eq(a: &RuntimeValue, b: &RuntimeValue) -> bool {
  match (a, b) {
    (RuntimeValue::Absent, RuntimeValue::Absent) | (RuntimeValue::Null, RuntimeValue::Null) => true,
    (RuntimeValue::Bool(x), RuntimeValue::Bool(y)) => x == y,
    (RuntimeValue::Number(x), RuntimeValue::Number(y)) => x.as_f64() == y.as_f64(),
    (RuntimeValue::String(x), RuntimeValue::String(y)) => x == y,
    (RuntimeValue::Bytes(x), RuntimeValue::Bytes(y)) => x == y,
    (RuntimeValue::Array(x), RuntimeValue::Array(y)) => {
      x.len() == y.len() && x.iter().zip(y).all(|(a, b)| runtime_eq(a, b))
    }
    (RuntimeValue::Object(x), RuntimeValue::Object(y)) => {
      x.len() == y.len()
        && x
          .iter()
          .all(|(k, v)| y.get(k).is_some_and(|v2| runtime_eq(v, v2)))
    }
    _ => false,
  }
}

/// A value's text, unquoted — the raw content of a `join`/`join-with`/`to-string`/`lower-case`/
/// `upper-case` result and of a parameter meant to be used as text (a regex pattern, a
/// `content-type`, a datetime format, `expect:only-entries`' allowed key names), as opposed to
/// [`display`]'s job of describing a *compared* value inside a mismatch message. Conflating the
/// two quotes text that must not be quoted — a regex pattern, an error message built by `join` —
/// which is exactly the bug this split exists to make impossible to reintroduce by accident.
fn plain_text(value: &RuntimeValue) -> String {
  match value {
    RuntimeValue::String(s) => s.clone(),
    other => display(other),
  }
}

fn display(value: &RuntimeValue) -> String {
  match value {
    RuntimeValue::Absent => "<absent>".to_string(),
    RuntimeValue::Null => "null".to_string(),
    RuntimeValue::Bool(b) => b.to_string(),
    RuntimeValue::Number(n) => n.to_string(),
    RuntimeValue::String(s) => format!("'{s}'"),
    RuntimeValue::Bytes(bytes) => format!("<{} byte(s)>", bytes.len()),
    RuntimeValue::Entry { key, .. } => format!("entry '{key}'"),
    RuntimeValue::Array(_) | RuntimeValue::Object(_) => {
      serde_json::to_string(&value.to_json()).unwrap_or_default()
    }
  }
}
