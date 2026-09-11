//! The v1–v4 matching-rule compiler (plan task 3.5, plan-grammar spec §4.4): compiles a v1–v4
//! request or response — method, path, query, headers, body, status, and the `matchingRules`
//! attached to them (`pact_models::matchingrules`) — to a plan, the same node grammar the shape
//! compiler (task 3.3) targets. **This module is the single home of v1–v4 cascading and
//! precedence semantics** (plan-grammar spec §1's table assigns that ownership here).
//!
//! **The precedence algorithm.** `pact_models::path_exp::DocPath::path_weight` scores a rule path
//! against an actual document path: each token contributes `2` for an exact field/index/root
//! match, `1` for a wildcard (`*`/`[*]`), `0` (poisoning the whole product) for a mismatch, and a
//! rule longer than the actual path never matches at all. The winning rule at a path is the one
//! maximising `weight * rule_path_length` — which is exactly what rewards both per-token
//! specificity and depth, so `$.a.b` beats `$.a.*` beats `$.*` for a path `$.a.b`. `pact_models`
//! doesn't expose header-name case-insensitivity or a deterministic tie-break, so [`best_rule`]
//! reimplements the same scoring rather than calling `path_weight` directly: [`token_weight`] is
//! `path_exp`'s `matches_token` with an added `case_insensitive` flag (headers only), and ties are
//! broken by the rule path's own text, ascending — `pact_models`' own selection
//! (`MatchingRuleCategory::max_by_path`) ties on `HashMap` iteration order, which plan-grammar
//! spec §2.5 forbids a Janus compiler from depending on.
//!
//! **Cascading, concretely.** A rule whose path is *shorter* than the node being checked
//! (`path.len() != fragments.len()`, [`best_rule`]'s second return value) reaches that node only
//! because nothing more specific was declared closer to it — recomputing the winning rule fresh at
//! every node (rather than threading a "current rule" down from the parent) is what makes a
//! deeper, more specific rule automatically override a shallower one without this compiler ever
//! having to notice the override happening. The one place cascading needs help from the
//! interpreter rather than falling out for free: `MinType`/`MaxType`/`MinMaxType` carry a
//! *collection* bound (an array or object's own size), and a cascaded rule reaches scalar
//! descendants too — [`interpret`](super::interpret)'s `match:min-type`/`match:max-type`/
//! `match:min-max-type` only enforce the bound when the resolved value is actually a collection,
//! which is what keeps a `MinType` declared on an array from wrongly re-applying its count to
//! every element several levels down.
//!
//! **Scope.** JSON bodies only (component-owned content types — XML, form, multipart — are out of
//! scope until design 2.6's components exist, matching plan task 3.3's own JSON-first scope and
//! the reuse-inventory's risk note). `Values`, `EachKey`, `EachValue`, `ArrayContains` and
//! plugin-provided rules compile to an `error` node naming the gap rather than a guess: the 803
//! specification test cases (plan task 3.7's corpus, `tests/fixtures/spec_testcases`) exercise
//! none of them, so this is a documented gap, not a silent one.

use super::model::{Literal, Node};
use crate::shape::path;
use pact_models::matchingrules::{MatchingRule, MatchingRuleCategory, MatchingRules, RuleList, RuleLogic};
use pact_models::path_exp::{DocPath, PathToken};
use serde_json::Value;
use std::collections::BTreeMap;

/// A v1–v4 request, decoded down to the parts this compiler needs. Query/header decoding from the
/// wire (a raw query string, a `Vec<String>` per repeated header) is a transport concern (design
/// 2.6, plan task 4.2) — this compiler only ever sees already-decoded maps, matching
/// [`super::resolve::CapturedValues`]'s own "never decode wire bytes" rule.
pub struct LegacyRequest {
  pub method: String,
  pub path: String,
  /// Query parameter name -> its values, in declaration order (repeated parameters keep every
  /// occurrence — v1–v4 query matching is order-sensitive within one name, spec test cases assert
  /// this directly).
  pub query: BTreeMap<String, Vec<String>>,
  /// Header name -> value. `pact_models` models a header as `Vec<String>` for repeated headers;
  /// every spec test case in scope carries single-valued headers, so this compiler takes the
  /// simpler scalar form and leaves multi-valued headers a documented gap.
  pub headers: BTreeMap<String, String>,
  /// `None` (the interaction never mentioned a body): the compiled plan asserts nothing about the
  /// actual one (`body/no body`). `Some(Value::Null)` (a body was declared, explicitly empty):
  /// the compiled plan asserts the actual body is absent too (`body/non empty body found when
  /// empty expected` is a mismatch on exactly this) — these two are not the same thing.
  pub body: Option<Value>,
  pub matching_rules: MatchingRules,
}

/// A v1–v4 response. See [`LegacyRequest`] for the header/body notes, which apply identically.
pub struct LegacyResponse {
  pub status: u64,
  pub headers: BTreeMap<String, String>,
  pub body: Option<Value>,
  pub matching_rules: MatchingRules,
}

/// Compile a v1–v4 request to a plan.
pub fn compile_request(req: &LegacyRequest) -> super::model::Plan {
  let mut slots = vec![
    Node::container(
      Some("method".to_string()),
      vec![compile_scalar_slot(
        "method",
        &req.matching_rules,
        "request",
        "method",
        &Value::String(req.method.clone()),
      )],
    ),
    Node::container(
      Some("path".to_string()),
      vec![compile_scalar_slot(
        "path",
        &req.matching_rules,
        "request",
        "path",
        &Value::String(req.path.clone()),
      )],
    ),
    Node::container(
      Some("query".to_string()),
      compile_query(&req.query, &req.matching_rules),
    ),
    Node::container(
      Some("headers".to_string()),
      compile_headers(&req.headers, &req.matching_rules, "request"),
    ),
  ];
  if let Some(node) = compile_body_slot(req.body.as_ref(), &req.matching_rules, "request") {
    slots.push(node);
  }
  super::model::Plan {
    grammar: super::model::GRAMMAR_VERSION,
    root: Node::container(Some("request".to_string()), slots),
    variant: None,
  }
}

/// Compile a v1–v4 response to a plan.
pub fn compile_response(res: &LegacyResponse) -> super::model::Plan {
  let mut slots = Vec::new();
  slots.push(Node::container(
    Some("status".to_string()),
    vec![compile_scalar_slot(
      "status",
      &res.matching_rules,
      "response",
      "status",
      &Value::from(res.status),
    )],
  ));
  slots.push(Node::container(
    Some("headers".to_string()),
    compile_headers(&res.headers, &res.matching_rules, "response"),
  ));
  if let Some(node) = compile_body_slot(res.body.as_ref(), &res.matching_rules, "response") {
    slots.push(node);
  }
  super::model::Plan {
    grammar: super::model::GRAMMAR_VERSION,
    root: Node::container(Some("response".to_string()), slots),
    variant: None,
  }
}

// --- precedence: the weighted, deterministic rule lookup every category below calls ---

/// `path_exp`'s `matches_token` (§module docs), plus header-name case-insensitivity.
fn token_weight(token: &PathToken, fragment: &str, case_insensitive: bool) -> usize {
  let field_eq = |name: &str| {
    if case_insensitive {
      name.eq_ignore_ascii_case(fragment)
    } else {
      name == fragment
    }
  };
  match token {
    PathToken::Root => (fragment == "$") as usize * 2,
    PathToken::Field(name) => field_eq(name) as usize * 2,
    PathToken::Index(index) => (fragment.parse::<usize>() == Ok(*index)) as usize * 2,
    PathToken::StarIndex => (fragment == "[*]" || fragment.parse::<usize>().is_ok()) as usize,
    PathToken::Star => 1,
  }
}

/// The winning rule at `fragments` (spec §module docs' precedence algorithm), and whether it
/// cascaded down from a shorter (ancestor or wildcard-shallower) path. `None` when the category is
/// absent or no declared rule reaches this path at all.
fn best_rule(
  category: Option<&MatchingRuleCategory>,
  fragments: &[String],
  case_insensitive: bool,
) -> Option<(RuleList, bool)> {
  let category = category?;
  let mut best: Option<(usize, &DocPath, &RuleList)> = None;
  for (candidate_path, list) in &category.rules {
    let tokens = candidate_path.tokens();
    if fragments.len() < tokens.len() {
      continue;
    }
    let weight = tokens
      .iter()
      .zip(fragments.iter())
      .fold(1usize, |acc, (token, fragment)| {
        acc * token_weight(token, fragment, case_insensitive)
      });
    if weight == 0 {
      continue;
    }
    let score = weight * tokens.len();
    let replace = match &best {
      None => true,
      Some((best_score, best_path, _)) => {
        score > *best_score || (score == *best_score && candidate_path.to_string() < best_path.to_string())
      }
    };
    if replace {
      best = Some((score, candidate_path, list));
    }
  }
  best.map(|(_, candidate_path, list)| (list.clone(), candidate_path.len() != fragments.len()))
}

fn rules_for(rules: &MatchingRules, category: &str) -> Option<MatchingRuleCategory> {
  rules.rules_for_category(category).filter(|c| c.is_not_empty())
}

// --- a single rule -> a single match node ---

/// One [`MatchingRule`] to one plan node. `example` is the expected value at this path, carried
/// for the same reason the shape compiler carries one (plan-grammar spec §5.2): a mismatch message
/// worth reading names what was expected, not just that something failed.
///
/// `Values`, `EachKey`, `EachValue`, `ArrayContains` and plugin-provided rules are a documented gap
/// (module docs' "Scope"): none of the specification test cases in `tests/fixtures/spec_testcases`
/// exercise them.
fn rule_node(rule: &MatchingRule, resolve: Node, example: &Value) -> Node {
  match rule {
    MatchingRule::Equality => Node::action(
      "match:equality",
      vec![resolve, Node::value(Literal::from_json(example))],
    ),
    MatchingRule::Regex(pattern) => Node::action(
      "match:regex",
      vec![resolve, Node::value(Literal::string(pattern))],
    ),
    MatchingRule::Type => Node::action(
      "match:type",
      vec![resolve, Node::value(Literal::from_json(example))],
    ),
    MatchingRule::MinType(min) => Node::action(
      "match:min-type",
      vec![
        resolve,
        Node::value(Literal::from_json(example)),
        Node::value(Literal::number(*min as u64)),
      ],
    ),
    MatchingRule::MaxType(max) => Node::action(
      "match:max-type",
      vec![
        resolve,
        Node::value(Literal::from_json(example)),
        Node::value(Literal::number(*max as u64)),
      ],
    ),
    MatchingRule::MinMaxType(min, max) => Node::action(
      "match:min-max-type",
      vec![
        resolve,
        Node::value(Literal::from_json(example)),
        Node::value(Literal::number(*min as u64)),
        Node::value(Literal::number(*max as u64)),
      ],
    ),
    MatchingRule::Timestamp(format) => Node::action(
      "match:datetime",
      vec![resolve, Node::value(Literal::string(format))],
    ),
    MatchingRule::Time(format) => {
      Node::action("match:time", vec![resolve, Node::value(Literal::string(format))])
    }
    MatchingRule::Date(format) => {
      Node::action("match:date", vec![resolve, Node::value(Literal::string(format))])
    }
    MatchingRule::Include(substring) => Node::action(
      "match:include",
      vec![resolve, Node::value(Literal::string(substring))],
    ),
    MatchingRule::Number => Node::action("match:number", vec![resolve]),
    MatchingRule::Integer => Node::action("match:integer", vec![resolve]),
    MatchingRule::Decimal => Node::action("match:decimal", vec![resolve]),
    MatchingRule::Null => Node::action("match:null", vec![resolve]),
    MatchingRule::ContentType(content_type) => Node::action(
      "match:content-type",
      vec![resolve, Node::value(Literal::string(content_type))],
    ),
    MatchingRule::Boolean => Node::action("match:boolean", vec![resolve]),
    MatchingRule::NotEmpty => Node::action("expect:not-empty", vec![resolve]),
    MatchingRule::Semver => Node::action("match:semver", vec![resolve]),
    other => Node::action(
      "error",
      vec![Node::value(Literal::string(format!(
        "legacy matching rule '{}' is not yet supported by the compiled plan",
        other.name()
      )))],
    ),
  }
}

/// A [`RuleList`] to one node: the single rule's node, or every rule's node combined by
/// `RuleLogic::And`/`Or` (plan-grammar spec §4.2's `and`/`or` control actions — the interpreter
/// already short-circuits `or` and conjoins `and`, so the combination needs no special handling
/// here beyond picking the right wrapper).
fn rule_list_node(list: &RuleList, resolve: Node, example: &Value) -> Node {
  let mut nodes: Vec<Node> = list
    .rules
    .iter()
    .map(|rule| rule_node(rule, resolve.clone(), example))
    .collect();
  match nodes.len() {
    1 => nodes.pop().expect("checked len == 1"),
    _ => {
      let family = match list.rule_logic {
        RuleLogic::And => "and",
        RuleLogic::Or => "or",
      };
      Node::action(family, nodes)
    }
  }
}

// --- method, path, status: one scalar, one rule lookup, no cascading ---

fn compile_scalar_slot(
  category_name: &str,
  rules: &MatchingRules,
  part: &str,
  slot: &str,
  example: &Value,
) -> Node {
  let category = rules_for(rules, category_name);
  let fragments = vec!["$".to_string()];
  let resolve = Node::resolve(format!("$.{}", path::root(part, slot)));
  match best_rule(category.as_ref(), &fragments, false) {
    Some((list, _cascaded)) => rule_list_node(&list, resolve, example),
    // Method is the one scalar slot v1-v4 compares case-insensitively by default (spec test case
    // `method/method is different case`) — expressed with the existing `lower-case` core action
    // rather than a new one, since `match:equality`'s own children may be any value-producing node.
    None if category_name == "method" => Node::action(
      "match:equality",
      vec![
        Node::action("lower-case", vec![resolve]),
        Node::action("lower-case", vec![Node::value(Literal::from_json(example))]),
      ],
    ),
    None => Node::action(
      "match:equality",
      vec![resolve, Node::value(Literal::from_json(example))],
    ),
  }
}

// --- headers: scalar values, case-insensitive names, no cascading ---

fn compile_headers(headers: &BTreeMap<String, String>, rules: &MatchingRules, part: &str) -> Vec<Node> {
  let category = rules_for(rules, "header");
  headers
    .iter()
    .map(|(name, value)| {
      let lower_name = name.to_ascii_lowercase();
      let fragments = vec!["$".to_string(), lower_name.clone()];
      let resolve = Node::resolve(format!("$.{}.{lower_name}", path::root(part, "headers")));
      let example = Value::String(value.clone());
      let node = match best_rule(category.as_ref(), &fragments, true) {
        Some((list, _cascaded)) => rule_list_node(&list, resolve, &example),
        // The v1-v4 default header comparison (module docs): not plain string equality — a
        // multi-valued (comma-separated) header tolerates whitespace around the commas, and a
        // MIME-shaped one (`type/subtype; param=value`) compares type and parameters as a set,
        // extra actual parameters allowed. Legacy-only: no shape operator needs this.
        None => Node::action(
          "match:header-value",
          vec![resolve, Node::value(Literal::from_json(&example))],
        ),
      };
      Node::container(Some(format!("$.headers.{name}")), vec![node])
    })
    .collect()
}

// --- query: value lists, order-sensitive by default, per-value rule lookup (cascades) ---

/// v1–v4's query default is closed, unlike headers' (spec test case `query/unexpected param`):
/// every actual query parameter must be named in `expected`.
fn compile_query(query: &BTreeMap<String, Vec<String>>, rules: &MatchingRules) -> Vec<Node> {
  let category = rules_for(rules, "query");
  let base_path = "$.request.query".to_string();
  let mut nodes: Vec<Node> = query
    .iter()
    .map(|(name, values)| {
      let base = format!("{base_path}.{name}");
      let mut children = vec![Node::action(
        "expect:count",
        vec![
          Node::resolve(base.clone()),
          Node::value(Literal::number(values.len() as u64)),
        ],
      )];
      for (index, value) in values.iter().enumerate() {
        let fragments = vec!["$".to_string(), name.clone(), index.to_string()];
        let resolve = Node::resolve(format!("{base}[{index}]"));
        let example = Value::String(value.clone());
        let node = match best_rule(category.as_ref(), &fragments, false) {
          Some((list, _cascaded)) => rule_list_node(&list, resolve, &example),
          None => Node::action(
            "match:equality",
            vec![resolve, Node::value(Literal::from_json(&example))],
          ),
        };
        children.push(node);
      }
      Node::container(Some(format!("$.query.{name}")), children)
    })
    .collect();
  let names: Vec<Node> = query.keys().map(|k| Node::value(Literal::string(k))).collect();
  let mut only_entries = vec![Node::resolve(base_path)];
  only_entries.extend(names);
  nodes.push(Node::action("expect:only-entries", only_entries));
  nodes
}

// --- body: the recursive, cascading walk ---

fn compile_body_slot(body: Option<&Value>, rules: &MatchingRules, part: &str) -> Option<Node> {
  let root_path = format!("$.{}", path::root(part, "body"));
  let body = match body {
    // No body was part of this interaction at all: nothing to assert (spec test case `body/no
    // body`).
    None => return None,
    // Explicitly null: v1-v4 requires the actual body be absent too (`body/non empty body found
    // when empty expected` is a mismatch on exactly this) — an assertion, not a skip.
    Some(Value::Null) => {
      return Some(Node::container(
        Some("body".to_string()),
        vec![Node::action("expect:absent", vec![Node::resolve(root_path)])],
      ));
    }
    Some(body) => body,
  };
  let category = rules_for(rules, "body");
  let ctx = BodyCtx {
    fragments: vec!["$".to_string()],
    cursor: Cursor::Absolute(root_path.clone()),
    label_path: root_path,
    category: category.as_ref(),
    closed_objects: part == "request",
  };
  Some(Node::container(
    Some("body".to_string()),
    compile_body_value(body, &ctx),
  ))
}

/// Where a compiled node resolves its value from — [`super::compile`]'s own `Cursor`, duplicated
/// rather than shared: this compiler's `BodyCtx` and that one's `Ctx` diverge in every other field
/// (rule fragments vs. dimension ids), so the two structs are unrelated even though this one enum
/// happens to coincide.
#[derive(Clone)]
enum Cursor {
  Absolute(String),
  Relative(String),
}

impl Cursor {
  fn member(&self, name: &str) -> Cursor {
    match self {
      Cursor::Absolute(p) => Cursor::Absolute(path::member(p, name)),
      Cursor::Relative(p) => Cursor::Relative(path::member(p, name)),
    }
  }

  fn array_index(&self, index: usize) -> Cursor {
    match self {
      Cursor::Absolute(p) => Cursor::Absolute(path::array_index(p, index)),
      Cursor::Relative(p) => Cursor::Relative(path::array_index(p, index)),
    }
  }

  fn resolve(&self) -> Node {
    match self {
      Cursor::Absolute(p) => Node::resolve(p.clone()),
      Cursor::Relative(p) => Node::resolve_current(p.clone()),
    }
  }
}

struct BodyCtx<'a> {
  /// Rule-lookup fragments, rooted at `$` and relative to the body — matching how
  /// `pact_models::matchingrules` stores body-category paths (`$.animals`, not `$.body.animals`).
  fragments: Vec<String>,
  /// Where a `resolve`/`resolve-current` node emitted here actually addresses (absolute, or
  /// relative to a `for-each`'s current item once one has been entered).
  cursor: Cursor,
  /// The dotted display path (`$.request.body.animals[*]`) a container labels itself with —
  /// always absolute-looking and always growing, independent of `cursor` (plan task 3.3's
  /// `Ctx::at`/`Ctx::label` play the identical role for the shape compiler).
  label_path: String,
  category: Option<&'a MatchingRuleCategory>,
  /// Whether an object encountered under this context is closed to unnamed members
  /// (`expect:only-entries`) or must-ignores them. v1–v4's default is asymmetric by part — request
  /// bodies are closed, response bodies are not (spec test case `body/unexpected key with null
  /// value`, which mismatches on the request side and matches on the response side of the same
  /// shape) — so this is set once, from `part`, when the walk starts, and threaded down unchanged.
  closed_objects: bool,
}

impl<'a> BodyCtx<'a> {
  fn member(&self, name: &str) -> BodyCtx<'a> {
    let mut fragments = self.fragments.clone();
    fragments.push(name.to_string());
    BodyCtx {
      fragments,
      cursor: self.cursor.member(name),
      label_path: path::member(&self.label_path, name),
      category: self.category,
      closed_objects: self.closed_objects,
    }
  }

  /// A real array index — used for the strict, no-rule default (plan-grammar spec §5.2's `array`).
  fn index(&self, index: usize) -> BodyCtx<'a> {
    let mut fragments = self.fragments.clone();
    fragments.push(index.to_string());
    BodyCtx {
      fragments,
      cursor: self.cursor.array_index(index),
      label_path: path::array_index(&self.label_path, index),
      category: self.category,
      closed_objects: self.closed_objects,
    }
  }

  /// The item context inside a rule-driven array's `for-each` (module docs): a fresh relative
  /// cursor, because the splat's expansion is what makes the element current, not anything the
  /// enclosing cursor knew (plan task 3.3's `Ctx::iteration_item` is the same move). The fragment
  /// pushed is a synthetic `"0"`: it scores identically to any real index against a
  /// `StarIndex`/`Star` rule token, which is all a template ever needs to look up its own cascaded
  /// rule.
  fn template_item(&self) -> BodyCtx<'a> {
    let mut fragments = self.fragments.clone();
    fragments.push("0".to_string());
    BodyCtx {
      fragments,
      cursor: Cursor::Relative("~>".to_string()),
      label_path: path::each_like_item(&self.label_path),
      category: self.category,
      closed_objects: self.closed_objects,
    }
  }

  fn best(&self) -> Option<(RuleList, bool)> {
    best_rule(self.category, &self.fragments, false)
  }

  fn resolve(&self) -> Node {
    self.cursor.resolve()
  }

  fn label(&self) -> String {
    self.label_path.clone()
  }
}

fn compile_body_value(value: &Value, ctx: &BodyCtx) -> Vec<Node> {
  match value {
    Value::Object(members) => compile_body_object(members, ctx),
    Value::Array(items) => compile_body_array(items, ctx),
    _ => vec![compile_body_scalar(value, ctx)],
  }
}

fn compile_body_scalar(value: &Value, ctx: &BodyCtx) -> Node {
  match ctx.best() {
    Some((list, _cascaded)) => rule_list_node(&list, ctx.resolve(), value),
    None => Node::action(
      "match:equality",
      vec![ctx.resolve(), Node::value(Literal::from_json(value))],
    ),
  }
}

/// `object` (module docs): v1–v4's default is closed for a request body and must-ignore (the
/// shape language's own default, ADR 0007 commitment 4) for a response one — an asymmetry the
/// corpus states directly (`body/unexpected key with null value` is the same body shape run twice,
/// mismatching under `request/` and matching under `response/`), not a guess this compiler makes:
/// `ctx.closed_objects` carries the part-level decision down unchanged (`BodyCtx` docs).
///
/// A rule found directly on the object's own path is deliberately not applied here unless it is
/// one of the map-entry rules (`Values`/`EachKey`/`EachValue`, module docs' gap): that mirrors the
/// oracle's own behaviour (a bare `Type`/`Regex`/`Equality` declared on an object path does not
/// replace the structural walk), confirmed against `tests/fixtures/spec_testcases` rather than
/// assumed.
fn compile_body_object(members: &serde_json::Map<String, Value>, ctx: &BodyCtx) -> Vec<Node> {
  if let Some((list, false)) = ctx.best()
    && list.rules.iter().any(is_map_entry_rule)
  {
    return vec![rule_list_node(
      &list,
      ctx.resolve(),
      &Value::Object(members.clone()),
    )];
  }
  let mut nodes: Vec<Node> = members
    .iter()
    .map(|(name, value)| {
      let member_ctx = ctx.member(name);
      Node::container(Some(member_ctx.label()), compile_body_value(value, &member_ctx))
    })
    .collect();
  if ctx.closed_objects {
    let names: Vec<Node> = members.keys().map(|k| Node::value(Literal::string(k))).collect();
    let mut only_entries = vec![ctx.resolve()];
    only_entries.extend(names);
    nodes.push(Node::action("expect:only-entries", only_entries));
  }
  nodes
}

fn is_map_entry_rule(rule: &MatchingRule) -> bool {
  matches!(
    rule,
    MatchingRule::Values | MatchingRule::EachKey(_) | MatchingRule::EachValue(_)
  )
}

/// `array` (module docs): a rule reaching this path (exactly or cascaded — `matcher_is_defined`'s
/// own behaviour, not just an exact hit) switches the array from the strict, no-rule default
/// (`expect:count` plus one compiled container per literal index) to the rule-driven form: the
/// rule's own node, plus (when there is a first element to serve as one) a `for-each` compiling
/// that one element once as a template and applying it to every actual element at run time —
/// exactly [`compile.rs`](super::compile)'s `each-like`, with the item shape replaced by whatever
/// rule cascades into the template's own path.
fn compile_body_array(items: &[Value], ctx: &BodyCtx) -> Vec<Node> {
  match ctx.best() {
    Some((list, _cascaded)) => {
      let mut nodes = vec![rule_list_node(
        &list,
        ctx.resolve(),
        &Value::Array(items.to_vec()),
      )];
      if let Some(template) = items.first() {
        let item_ctx = ctx.template_item();
        let item_node = Node::container(Some(item_ctx.label()), compile_body_value(template, &item_ctx));
        nodes.push(Node::action(
          "for-each",
          vec![Node::splat(vec![ctx.resolve()]), item_node],
        ));
      }
      nodes
    }
    None => {
      let mut nodes = vec![Node::action(
        "expect:count",
        vec![ctx.resolve(), Node::value(Literal::number(items.len() as u64))],
      )];
      for (index, item) in items.iter().enumerate() {
        let item_ctx = ctx.index(index);
        nodes.push(Node::container(
          Some(item_ctx.label()),
          compile_body_value(item, &item_ctx),
        ));
      }
      nodes
    }
  }
}
