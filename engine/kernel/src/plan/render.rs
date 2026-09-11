//! The two text forms (plan task 3.6, plan-grammar spec §3): the pretty form of a compiled
//! [`Plan`] and the executed form of an [`Executed`] tree. Both are specified surfaces — "what
//! `explain` prints" — not a debugging convenience, so this module follows spec §3's sigils and
//! layout exactly rather than approximating them, and is checked against the two worked examples
//! checked into the spec (`Documentation/specs/plan-grammar/examples/`) rather than merely against
//! its own idea of what it should produce.
//!
//! **One ambiguity the worked examples don't settle, resolved here rather than left to guess.**
//! `corpus-case.md`'s executed form shows an untaken, no-`else` `if` (`optional`'s absent branch)
//! rendered as `=> BOOL(true)` — but [`interpret::run_if`](super::interpret) deliberately returns
//! bare [`NodeResult::Ok`] in exactly that case (its own doc comment explains why, and plan task
//! 3.3's compiler relies on `ok`/`Value(Bool(true))` being interchangeable for `truthy`). Rather
//! than reopen that already-tested decision, this renderer maps **every** `NodeResult::Ok` to
//! `BOOL(true)`: `ok` per spec §2.3 means "succeeded, produced no value", and every case either
//! worked example shows a successful conjunction- or control-family node landing on `BOOL(true)`
//! — never a bare, distinct "OK" token — so treating "succeeded with nothing to report" as `true`
//! in the pass/fail sense that already pervades every other rendered result is the reading the
//! evidence actually supports, made entirely in this module rather than by changing what the
//! interpreter returns.
//!
//! A second, smaller simplification: a resolved boolean *value* (real data, not a pass/fail
//! signal) and a synthetic conjunction result both render as `BOOL(...)` — neither worked example
//! resolves an actual boolean field, so there is no evidence either way, and one shared rule beats
//! guessing at a distinction nothing exercises.

use super::interpret::{Executed, ExecutedKind, NodeResult};
use super::model::{Literal, Node, NodeKind, Plan};
use super::value::RuntimeValue;
use serde_json::Number;

/// The pretty form of a compiled plan (spec §3.1): the root node's rendering, wrapped in one more
/// pair of un-prefixed parens — the document envelope both worked examples show around what is
/// otherwise just `plan.root`'s own `container` rendering.
pub fn pretty(plan: &Plan) -> String {
  format!("(\n{}\n)", render_node(&plan.root, 1, &[]))
}

/// The executed form of an executed plan (spec §3.2): the same envelope, with every node that
/// carries a [`NodeResult`] annotated `" => <result>"` and an unexecuted (lazy-skipped) subtree
/// carrying none at all.
pub fn executed(root: &Executed) -> String {
  format!("(\n{}\n)", render_executed_node(root, 1, &[]))
}

// --- shared leaf rendering: labels, quoted strings, literals ---

/// A bare (unquoted) label is a simple identifier: a letter, then letters/digits/`-`/`_`. Anything
/// else — whitespace, a dimensional path's `.`/`[`/`$`, a name that starts with a digit — is
/// double-quoted. `:response`, `:card` and `:query-test` are bare; `:"$.id"` and
/// `:"query parameters"` are not, and neither is quoted merely because it has whitespace (spec
/// §3.1's own description undersells the rule: `:"$.id"` has none).
fn is_bare_label(s: &str) -> bool {
  let mut chars = s.chars();
  match chars.next() {
    Some(c) if c.is_ascii_alphabetic() => chars.all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
    _ => false,
  }
}

fn quote_label(s: &str) -> String {
  if is_bare_label(s) {
    s.to_string()
  } else {
    let mut out = String::from("\"");
    for c in s.chars() {
      match c {
        '\\' => out.push_str("\\\\"),
        '"' => out.push_str("\\\""),
        _ => out.push(c),
      }
    }
    out.push('"');
    out
  }
}

/// A string value, single-quoted with `\` and `'` escaped (spec §3.1's table; the escaping is
/// exactly what the worked example's `'yyyy-MM-dd\'T\'HH:mm:ssX'` datetime format shows — a
/// pattern that itself contains single quotes).
fn quote_string(s: &str) -> String {
  let mut out = String::from("'");
  for c in s.chars() {
    match c {
      '\\' => out.push_str("\\\\"),
      '\'' => out.push_str("\\'"),
      _ => out.push(c),
    }
  }
  out.push('\'');
  out
}

/// A literal or a resolved value's own text — shared by the `value` node's static rendering and
/// (for every kind but boolean) a `resolve`/`resolve-current` node's `=> ` result, since both are
/// "print this value" with no pass/fail framing attached (spec §2.2, §3.1's kind table: `'a
/// string'`, `42`, `true`, `NULL`). [`RuntimeValue::Absent`] renders the same as `Null` — the plan
/// document model has no term for it (plan task 3.4's own module docs), and neither does the text
/// form.
fn render_value(value: &RuntimeValue) -> String {
  match value {
    RuntimeValue::Absent | RuntimeValue::Null => "NULL".to_string(),
    RuntimeValue::Bool(b) => b.to_string(),
    RuntimeValue::Number(n) => render_number(n),
    RuntimeValue::String(s) => quote_string(s),
    RuntimeValue::Bytes(bytes) => format!("<{} byte(s)>", bytes.len()),
    RuntimeValue::Array(items) => format!(
      "[{}]",
      items.iter().map(render_value).collect::<Vec<_>>().join(", ")
    ),
    RuntimeValue::Object(members) => format!(
      "{{{}}}",
      members
        .iter()
        .map(|(k, v)| format!("{}: {}", quote_string(k), render_value(v)))
        .collect::<Vec<_>>()
        .join(", ")
    ),
    RuntimeValue::Entry { key, value } => format!("{{{}: {}}}", quote_string(key), render_value(value)),
  }
}

fn render_number(n: &Number) -> String {
  n.to_string()
}

fn render_literal(literal: &Literal) -> String {
  render_value(&RuntimeValue::from_literal(literal))
}

/// A node's `=> <result>` suffix (spec §3.2). `Ok` and a boolean `Value` both render `BOOL(...)`
/// (module docs); every other `Value` uses [`render_value`]'s plain literal syntax; `Error` prints
/// its message verbatim — it is already a complete sentence built by the interpreter (plan task
/// 3.4), not text this renderer escapes or requotes.
fn render_result(result: &NodeResult) -> String {
  match result {
    NodeResult::Ok => "BOOL(true)".to_string(),
    NodeResult::Value(RuntimeValue::Bool(b)) => format!("BOOL({b})"),
    NodeResult::Value(v) => render_value(v),
    NodeResult::Error { message, .. } => format!("ERROR({message})"),
  }
}

fn suffix(result: Option<&NodeResult>) -> String {
  match result {
    Some(r) => format!(" => {}", render_result(r)),
    None => String::new(),
  }
}

fn indent_str(depth: usize) -> String {
  "  ".repeat(depth)
}

/// Wraps already-rendered, already-indented `children` in `<prefix>(...)`, comma-separating all
/// but the last, or the single-line empty form when there are none — no worked example shows an
/// empty container or action, so this is this renderer's own, minimal choice for a case the
/// grammar allows but the corpus so far has never produced.
fn wrap(pad: &str, prefix: &str, children: &[String]) -> String {
  if children.is_empty() {
    return format!("{pad}{prefix}()");
  }
  let last = children.len() - 1;
  let body = children
    .iter()
    .enumerate()
    .map(|(i, c)| if i == last { c.clone() } else { format!("{c},") })
    .collect::<Vec<_>>()
    .join("\n");
  format!("{pad}{prefix}(\n{body}\n{pad})")
}

// --- container labels: full dimensional paths, shortened for display ---

/// Plan task 3.3's compiler deliberately labels every structural container with its full
/// dimensional path (`compile.rs`'s own `Ctx::label` doc comment: "the shorter form the
/// plan-grammar spec's worked examples show... is a rendering nicety task 3.6 owns"). This is that
/// nicety: `:"$.id"`, not `:"$.response.body.id"` (`corpus-case.md` §2; `order-payload-plan.md` §1
/// shortens every member the same way, however deep — `:"$.items[*].sku"`, not
/// `:"$.response.body.items[*].sku"`).
///
/// `plain_ancestors` is every plain (non-`$`-prefixed) container label seen from the plan's root
/// down to this one — the description, the part name, the slot name, in that order for the shape
/// compiler; just the part and slot names for the legacy compiler (design 3.5's plans have no
/// description wrapper). The two compilers disagree on how many plain labels sit above the first
/// dimensional one, so rather than hard-code a depth, this tries stripping longer prefixes first
/// (the full ancestor chain, then every shorter suffix of it) and keeps the first one that's
/// actually a prefix of the label — which is exactly "part.slot." for both compilers, found without
/// either one having to agree on where the interaction description belongs.
fn shorten_label(label: &str, plain_ancestors: &[String]) -> String {
  if !label.starts_with('$') {
    return label.to_string();
  }
  for start in 0..=plain_ancestors.len() {
    let joined = plain_ancestors[start..].join(".");
    let candidate = if joined.is_empty() {
      "$.".to_string()
    } else {
      format!("$.{joined}.")
    };
    if let Some(rest) = label.strip_prefix(candidate.as_str()) {
      return format!("$.{rest}");
    }
  }
  label.to_string()
}

/// The `plain_ancestors` to hand a container's children: itself appended, if its own label is
/// plain — a dimensional label freezes the chain (every deeper label shares its prefix, so nothing
/// further needs adding), and no label (an unlabelled bundling container) passes it through
/// unchanged.
fn extend_ancestors(plain_ancestors: &[String], label: Option<&str>) -> Vec<String> {
  match label {
    Some(l) if !l.starts_with('$') => {
      let mut next = plain_ancestors.to_vec();
      next.push(l.to_string());
      next
    }
    _ => plain_ancestors.to_vec(),
  }
}

// --- the pretty-form walk (Node) ---

fn render_node(node: &Node, depth: usize, plain_ancestors: &[String]) -> String {
  let pad = indent_str(depth);
  match &node.kind {
    NodeKind::Container { label, children } => {
      let prefix = match label {
        Some(l) => format!(":{} ", quote_label(&shorten_label(l, plain_ancestors))),
        None => String::new(),
      };
      let next_ancestors = extend_ancestors(plain_ancestors, label.as_deref());
      let rendered: Vec<String> = children
        .iter()
        .map(|c| render_node(c, depth + 1, &next_ancestors))
        .collect();
      wrap(&pad, &prefix, &rendered)
    }
    NodeKind::Action { name, children } => {
      let rendered: Vec<String> = children
        .iter()
        .map(|c| render_node(c, depth + 1, plain_ancestors))
        .collect();
      wrap(&pad, &format!("%{name} "), &rendered)
    }
    NodeKind::Pipeline { children } => {
      let rendered: Vec<String> = children
        .iter()
        .map(|c| render_node(c, depth + 1, plain_ancestors))
        .collect();
      wrap(&pad, "-> ", &rendered)
    }
    NodeKind::Splat { children } => {
      let rendered: Vec<String> = children
        .iter()
        .map(|c| render_node(c, depth + 1, plain_ancestors))
        .collect();
      wrap(&pad, "** ", &rendered)
    }
    NodeKind::Value(literal) => format!("{pad}{}", render_literal(literal)),
    NodeKind::Resolve { path } => format!("{pad}{path}"),
    NodeKind::ResolveCurrent { path } => format!("{pad}{path}"),
    NodeKind::Annotation { text } => format!("{pad}#{{{}}}", quote_string(text)),
  }
}

// --- the executed-form walk (Executed) ---

fn render_executed_node(executed: &Executed, depth: usize, plain_ancestors: &[String]) -> String {
  let pad = indent_str(depth);
  let tail = suffix(executed.result.as_ref());
  match &executed.kind {
    ExecutedKind::Container { label, children } => {
      let prefix = match label {
        Some(l) => format!(":{} ", quote_label(&shorten_label(l, plain_ancestors))),
        None => String::new(),
      };
      let next_ancestors = extend_ancestors(plain_ancestors, label.as_deref());
      let rendered: Vec<String> = children
        .iter()
        .map(|c| render_executed_node(c, depth + 1, &next_ancestors))
        .collect();
      format!("{}{tail}", wrap(&pad, &prefix, &rendered))
    }
    ExecutedKind::Action { name, children } => {
      let rendered: Vec<String> = children
        .iter()
        .map(|c| render_executed_node(c, depth + 1, plain_ancestors))
        .collect();
      format!("{}{tail}", wrap(&pad, &format!("%{name} "), &rendered))
    }
    ExecutedKind::Pipeline { children } => {
      let rendered: Vec<String> = children
        .iter()
        .map(|c| render_executed_node(c, depth + 1, plain_ancestors))
        .collect();
      format!("{}{tail}", wrap(&pad, "-> ", &rendered))
    }
    ExecutedKind::Splat { children } => {
      let rendered: Vec<String> = children
        .iter()
        .map(|c| render_executed_node(c, depth + 1, plain_ancestors))
        .collect();
      format!("{}{tail}", wrap(&pad, "** ", &rendered))
    }
    // A static value resolves to itself, trivially — `interpret::run` still attaches a result, but
    // printing `=> 'card'` under a literal `'card'` would say nothing a reader doesn't already have
    // (order-payload-plan.md §3: `%check:equals(... => 'invoice', 'card')`, never `'card' =>
    // 'card'`). Only `resolve`/`resolve-current` results are informative enough to print.
    ExecutedKind::Value(literal) => format!("{pad}{}", render_literal(literal)),
    ExecutedKind::Resolve { path } => format!("{pad}{path}{tail}"),
    ExecutedKind::ResolveCurrent { path } => format!("{pad}{path}{tail}"),
    // Never executed (plan-grammar spec §2.1): no `=> ` suffix, ever — `tail` is always empty here
    // because `interpret::run` never attaches a result to an annotation.
    ExecutedKind::Annotation { text } => format!("{pad}#{{{}}}", quote_string(text)),
  }
}
