//! The two text forms (plan task 3.6, plan-grammar spec §3): the pretty form of a compiled
//! [`Plan`] and the executed form of an [`Executed`] tree. Both are specified surfaces — "what
//! `explain` prints" — not a debugging convenience, so this module follows spec §3's sigils and
//! layout exactly rather than approximating them, and is checked against the two worked examples
//! checked into the spec (`Documentation/specs/plan-grammar/examples/`) rather than merely against
//! its own idea of what it should produce.
//!
//! **One ambiguity the worked examples don't settle, resolved here rather than left to guess.**
//! `corpus-case.md`'s executed form shows an untaken, no-`else` `flow:if` (`optional`'s absent branch)
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
use crate::display::{Kind, Marked};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde_json::Number;

/// The pretty form of a compiled plan (spec §3.1): the root node's rendering, wrapped in one more
/// pair of un-prefixed parens — the document envelope both worked examples show around what is
/// otherwise just `plan.root`'s own `container` rendering.
pub fn pretty(plan: &Plan) -> String {
  pretty_marked(plan).text()
}

/// The executed form of an executed plan (spec §3.2): the same envelope, with every node that
/// carries a [`NodeResult`] annotated `" => <result>"` and an unexecuted (lazy-skipped) subtree
/// carrying none at all.
pub fn executed(root: &Executed) -> String {
  executed_marked(root).text()
}

/// [`pretty`], with its segments (ADR 0054): each piece marked as what spec §3.1's table says it
/// is, which is exactly how [`super::tokens`] reads the text back. The corpus runner holds the two
/// to each other.
pub fn pretty_marked(plan: &Plan) -> Marked {
  envelope(render_node(&plan.root, 1, &[]))
}

/// [`executed`], with its segments.
pub fn executed_marked(root: &Executed) -> Marked {
  envelope(render_executed_node(root, 1, &[]))
}

fn envelope(root: Marked) -> Marked {
  let mut out = Marked::plain("(\n");
  out.append(root).text_plain("\n)");
  out
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
/// string'`, `42`, `true`, `NULL`). [`RuntimeValue::Absent`] is `ABSENT`: only a resolved value can
/// be one, since a literal cannot (plan task 3.4's own module docs), and a reader must be able to
/// tell a member that is not there from one that is `null` (spec §3.2, staged plan S1.30).
fn render_value(value: &RuntimeValue) -> Marked {
  match value {
    RuntimeValue::Absent => Marked::of(Kind::Literal, "ABSENT"),
    RuntimeValue::Null => Marked::of(Kind::Literal, "NULL"),
    RuntimeValue::Bool(b) => Marked::of(Kind::Literal, b.to_string()),
    RuntimeValue::Number(n) => Marked::of(Kind::Number, render_number(n)),
    RuntimeValue::String(s) => Marked::of(Kind::String, quote_string(s)),
    RuntimeValue::Bytes(bytes) => Marked::of(Kind::Literal, render_bytes(bytes)),
    RuntimeValue::Array(items) => {
      let mut out = Marked::plain("[");
      out
        .append(Marked::join(items.iter().map(render_value), ", "))
        .text_plain("]");
      out
    }
    RuntimeValue::Object(members) => {
      let mut out = Marked::plain("{");
      out
        .append(Marked::join(members.iter().map(|(k, v)| member(k, v)), ", "))
        .text_plain("}");
      out
    }
    RuntimeValue::Entry { key, value } => {
      let mut out = Marked::plain("{");
      out.append(member(key, value)).text_plain("}");
      out
    }
  }
}

/// `'key': value`, a member of an object or an entry.
fn member(key: &str, value: &RuntimeValue) -> Marked {
  let mut out = Marked::of(Kind::String, quote_string(key));
  out.text_plain(": ").append(render_value(value));
  out
}

/// How many base64 characters of a bytes value the text forms show: 36 octets, cut on a 4-character
/// boundary so the part shown decodes on its own.
const BYTES_SHOWN: usize = 48;

/// A bytes value as the text forms print it (spec §3.1): `BYTES(<length>, <base64>)`, the base64 cut
/// after [`BYTES_SHOWN`] characters and ended with `...`, which no base64 alphabet holds.
pub(crate) fn render_bytes(bytes: &[u8]) -> String {
  let encoded = BASE64.encode(bytes);
  let shown = if encoded.len() > BYTES_SHOWN {
    format!("{}...", &encoded[..BYTES_SHOWN])
  } else {
    encoded
  };
  format!("BYTES({}, {shown})", bytes.len())
}

fn render_number(n: &Number) -> String {
  n.to_string()
}

fn render_literal(literal: &Literal) -> Marked {
  render_value(&RuntimeValue::from_literal(literal))
}

/// A node's `=> <result>` suffix (spec §3.2). `Ok` and a boolean `Value` both render `BOOL(...)`
/// (module docs); every other `Value` uses [`render_value`]'s plain literal syntax; `Error` prints
/// its message verbatim — it is already a complete sentence built by the interpreter (plan task
/// 3.4), not text this renderer escapes or requotes.
fn render_result(result: &NodeResult) -> Marked {
  match result {
    NodeResult::Ok => Marked::of(Kind::Pass, "BOOL(true)"),
    NodeResult::Value(RuntimeValue::Bool(true)) => Marked::of(Kind::Pass, "BOOL(true)"),
    NodeResult::Value(RuntimeValue::Bool(false)) => Marked::of(Kind::Fail, "BOOL(false)"),
    NodeResult::Value(v) => render_value(v),
    NodeResult::Error { message, .. } => Marked::of(Kind::Fail, format!("ERROR({message})")),
  }
}

fn suffix(result: Option<&NodeResult>) -> Marked {
  match result {
    Some(r) => {
      let mut out = Marked::plain(" ");
      out
        .text_as(Kind::Arrow, "=>")
        .text_plain(" ")
        .append(render_result(r));
      out
    }
    None => Marked::new(),
  }
}

fn indent_str(depth: usize) -> String {
  "  ".repeat(depth)
}

/// Wraps already-rendered, already-indented `children` in `<prefix>(...)`, comma-separating all
/// but the last, or the single-line empty form when there are none — no worked example shows an
/// empty container or action, so this is this renderer's own, minimal choice for a case the
/// grammar allows but the corpus so far has never produced.
fn wrap(pad: &str, prefix: Marked, children: Vec<Marked>) -> Marked {
  let mut out = Marked::plain(pad);
  out.append(prefix);
  if children.is_empty() {
    out.text_plain("()");
    return out;
  }
  out.text_plain("(\n");
  let last = children.len() - 1;
  for (i, child) in children.into_iter().enumerate() {
    out.append(child);
    out.text_plain(if i == last { "\n" } else { ",\n" });
  }
  out.text_plain(format!("{pad})"));
  out
}

/// `:label ` before a container's children, or nothing for an unlabelled one.
fn container_prefix(label: Option<&String>, plain_ancestors: &[String]) -> Marked {
  match label {
    Some(l) => {
      let mut out = Marked::of(
        Kind::Label,
        format!(":{}", quote_label(&shorten_label(l, plain_ancestors))),
      );
      out.text_plain(" ");
      out
    }
    None => Marked::new(),
  }
}

/// `%name ` before an action's children.
fn action_prefix(name: &str) -> Marked {
  let mut out = Marked::of(Kind::Action, format!("%{name}"));
  out.text_plain(" ");
  out
}

/// `-> ` or `** ` before a pipeline's or splat's children.
fn operator_prefix(operator: &str) -> Marked {
  let mut out = Marked::of(Kind::Operator, operator);
  out.text_plain(" ");
  out
}

/// `#{'text'}`, an annotation, at `pad`.
fn annotation(pad: &str, text: &str) -> Marked {
  let mut out = Marked::plain(pad);
  out.text_as(Kind::Annotation, format!("#{{{}}}", quote_string(text)));
  out
}

/// A value or path leaf at `pad`, then its suffix.
fn leaf(pad: &str, body: Marked, tail: Marked) -> Marked {
  let mut out = Marked::plain(pad);
  out.append(body).append(tail);
  out
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
      "$".to_string()
    } else {
      format!("$.{joined}")
    };
    // The ancestors end where a member begins: `.name`, or `['name']` for one no plain segment can
    // hold (`$['a.b']`).
    if let Some(rest) = label
      .strip_prefix(candidate.as_str())
      .filter(|rest| rest.starts_with('.') || rest.starts_with('['))
    {
      return format!("${rest}");
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

fn render_node(node: &Node, depth: usize, plain_ancestors: &[String]) -> Marked {
  let pad = indent_str(depth);
  match &node.kind {
    NodeKind::Container { label, children } => {
      let next_ancestors = extend_ancestors(plain_ancestors, label.as_deref());
      let rendered = children
        .iter()
        .map(|c| render_node(c, depth + 1, &next_ancestors))
        .collect();
      wrap(&pad, container_prefix(label.as_ref(), plain_ancestors), rendered)
    }
    NodeKind::Action { name, children } => {
      let rendered = children
        .iter()
        .map(|c| render_node(c, depth + 1, plain_ancestors))
        .collect();
      wrap(&pad, action_prefix(name), rendered)
    }
    NodeKind::Pipeline { children } => {
      let rendered = children
        .iter()
        .map(|c| render_node(c, depth + 1, plain_ancestors))
        .collect();
      wrap(&pad, operator_prefix("->"), rendered)
    }
    NodeKind::Splat { children } => {
      let rendered = children
        .iter()
        .map(|c| render_node(c, depth + 1, plain_ancestors))
        .collect();
      wrap(&pad, operator_prefix("**"), rendered)
    }
    NodeKind::Value(literal) => leaf(&pad, render_literal(literal), Marked::new()),
    NodeKind::Resolve { path } => leaf(&pad, Marked::of(Kind::Path, path.as_str()), Marked::new()),
    NodeKind::ResolveCurrent { path } => leaf(&pad, Marked::of(Kind::Path, path.as_str()), Marked::new()),
    NodeKind::Annotation { text } => annotation(&pad, text),
  }
}

// --- the executed-form walk (Executed) ---

fn render_executed_node(executed: &Executed, depth: usize, plain_ancestors: &[String]) -> Marked {
  let pad = indent_str(depth);
  let tail = suffix(executed.result.as_ref());
  let wrapped = |prefix: Marked, rendered: Vec<Marked>| {
    let mut out = wrap(&pad, prefix, rendered);
    out.append(tail.clone());
    out
  };
  match &executed.kind {
    ExecutedKind::Container { label, children } => {
      let next_ancestors = extend_ancestors(plain_ancestors, label.as_deref());
      let rendered = children
        .iter()
        .map(|c| render_executed_node(c, depth + 1, &next_ancestors))
        .collect();
      wrapped(container_prefix(label.as_ref(), plain_ancestors), rendered)
    }
    ExecutedKind::Action { name, children } => {
      let rendered = children
        .iter()
        .map(|c| render_executed_node(c, depth + 1, plain_ancestors))
        .collect();
      wrapped(action_prefix(name), rendered)
    }
    ExecutedKind::Pipeline { children } => {
      let rendered = children
        .iter()
        .map(|c| render_executed_node(c, depth + 1, plain_ancestors))
        .collect();
      wrapped(operator_prefix("->"), rendered)
    }
    ExecutedKind::Splat { children } => {
      let rendered = children
        .iter()
        .map(|c| render_executed_node(c, depth + 1, plain_ancestors))
        .collect();
      wrapped(operator_prefix("**"), rendered)
    }
    // A static value resolves to itself, trivially — `interpret::run` still attaches a result, but
    // printing `=> 'card'` under a literal `'card'` would say nothing a reader doesn't already have
    // (order-payload-plan.md §3: `%check:equals(... => 'invoice', 'card')`, never `'card' =>
    // 'card'`). Only `resolve`/`resolve-current` results are informative enough to print.
    ExecutedKind::Value(literal) => leaf(&pad, render_literal(literal), Marked::new()),
    ExecutedKind::Resolve { path } => leaf(&pad, Marked::of(Kind::Path, path.as_str()), tail),
    ExecutedKind::ResolveCurrent { path, .. } => leaf(&pad, Marked::of(Kind::Path, path.as_str()), tail),
    // Never executed (plan-grammar spec §2.1): no `=> ` suffix, ever — `tail` is always empty here
    // because `interpret::run` never attaches a result to an annotation.
    ExecutedKind::Annotation { text } => annotation(&pad, text),
  }
}
