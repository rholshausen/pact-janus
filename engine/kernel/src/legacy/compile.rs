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
//! having to notice the override happening. The one place cascading does not fall out for free:
//! `MinType`/`MaxType`/`MinMaxType` carry the declaring array's own size bound, and a cascaded rule
//! reaches every descendant too. [`rule_list_node`] reduces a cascaded one to `Type`, so the bound
//! is enforced once, on the array that declared it, and never re-applied several levels down.
//!
//! **Scope.** A body is a document in the engine's model ([`RuntimeValue`], component-interfaces spec
//! §6.1), never JSON as such, so a content component's decoded document can reach this compiler as
//! a JSON pact's body does (staged plan S1.20). Structure is walked member by member, by name; a
//! string or bytes, which is what a text or binary body is, is one value at `$` (ADR 0036 decision
//! 3: today's `match_text` and `match_octet_stream`). A form body is compiled as a query string, the
//! way today's `match_form_urlencoded` compares one (ADR 0038). An XML body is walked element by
//! element, the way today's `match_xml` compares one (ADR 0039, [`compile_xml_body`]). A multipart
//! body is compared part by part, each by its own type, the way today's `match_mime_multipart`
//! compares one (ADR 0040, [`compile_multipart_body`]). A rule on a value that is text — a path, a header, a query or form value, an XML
//! attribute or text — reads the text as today's verifier does: `integer` is text that parses as an
//! integer (`legacy:parses-as`). `Values`, `EachKey`, `EachValue` and `ArrayContains` on a body's
//! object or array describe its entries, and compile as today's `compare_maps_with_matchingrule` and
//! `compare_lists_with_matchingrule` apply them (staged plan S1.24, ADR 0042,
//! [`compile_object_entries`], [`compile_array_rules`]); no specification test case covers them, so
//! S1.1's `matching-rules/` fixtures are their oracle. Declared on text (a path, a header, a query or
//! form value, XML), they and plugin-provided rules compile to a `flow:error` node naming the gap
//! rather than a guess: a documented gap, not a silent one.

use crate::legacy::multipart;
use crate::legacy::xml::{self, rule_name};
use crate::plan::{self as model, CapturedValues, Literal, Node, RuntimeValue, Status, member_path};
use crate::shape::path;
use indexmap::IndexMap;
use pact_models::matchingrules::expressions::MatchingRuleDefinition;
use pact_models::matchingrules::{MatchingRule, MatchingRuleCategory, MatchingRules, RuleList, RuleLogic};
use pact_models::path_exp::{DocPath, PathToken};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

/// A v1–v4 request, decoded down to the parts this compiler needs. Query/header decoding from the
/// wire (a raw query string, a `Vec<String>` per repeated header) is a transport concern (design
/// 2.6, plan task 4.2) — this compiler only ever sees already-decoded maps, matching
/// [`crate::plan::CapturedValues`]'s own "never decode wire bytes" rule.
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
  /// actual one (`body/no body`). `Some(LegacyBody::Empty)` (a body was declared, explicitly
  /// empty): the compiled plan asserts the actual body is absent too (`body/non empty body found
  /// when empty expected` is a mismatch on exactly this) — these two are not the same thing.
  pub body: Option<LegacyBody>,
  /// The body's content type as the pact gives it — its own, else the `Content-Type` header, else
  /// detected — which a received body's must agree with (`body_type_check`).
  pub content_type: Option<String>,
  pub matching_rules: MatchingRules,
}

/// A declared v1–v4 body, as the legacy layer read it by its content type (ADR 0036 decision 3).
#[derive(Debug, Clone, PartialEq)]
pub enum LegacyBody {
  /// Declared null or empty: the actual body must be absent too.
  Empty,
  /// The body as a document: a JSON document; a text body as a string; a binary body, or text that
  /// is not UTF-8, as bytes. A string and bytes are each one value at `$` to the compiler.
  Document(RuntimeValue),
  /// A form body, as the form component decodes one: each name with its values, a list of strings.
  /// Compiled as a query string is, since today's verifier compares it as one (ADR 0038).
  Form(RuntimeValue),
  /// An XML body, as the XML component decodes one ([`crate::legacy::xml`]). Compiled element by
  /// element, as today's verifier compares one (ADR 0039).
  Xml(RuntimeValue),
  /// A multipart body's parts, in body order ([`crate::legacy::multipart`]). Compiled part by part,
  /// as today's verifier compares them (ADR 0040).
  Multipart(Vec<LegacyPart>),
}

impl LegacyBody {
  /// The body as the document its content component decodes it to; absent when it is empty.
  pub fn document(&self) -> RuntimeValue {
    match self {
      LegacyBody::Empty => RuntimeValue::Absent,
      LegacyBody::Document(document) | LegacyBody::Form(document) | LegacyBody::Xml(document) => {
        document.clone()
      }
      LegacyBody::Multipart(parts) => crate::legacy::multipart::document(parts),
    }
  }
}

/// One part of a v1–v4 multipart body, read as today's verifier reads it (ADR 0040 decision 5).
#[derive(Debug, Clone, PartialEq)]
pub struct LegacyPart {
  /// Its name; empty when it has none.
  pub name: String,
  /// Its position among all the body's parts, which a rule path uses for a part with no name.
  pub index: usize,
  /// The file name its `Content-Disposition` gives.
  pub filename: Option<String>,
  /// A field (a `Content-Disposition` and no file name), which today compares as text; else a file,
  /// compared as a body of its own type.
  pub field: bool,
  /// Its headers, lower-cased, as `multer` keeps them.
  pub headers: Vec<(String, String)>,
  /// Its own `Content-Type`, when it has one.
  pub content_type: Option<String>,
  /// Its content, read by its type as a body is: never [`LegacyBody::Empty`], and a part that is
  /// itself multipart is bytes.
  pub body: LegacyBody,
}

/// A v1–v4 response. See [`LegacyRequest`] for the header/body notes, which apply identically.
pub struct LegacyResponse {
  pub status: u64,
  pub headers: BTreeMap<String, String>,
  pub body: Option<LegacyBody>,
  pub content_type: Option<String>,
  pub matching_rules: MatchingRules,
}

/// A v1–v4 message (staged plan S1.7): an asynchronous message, or the response a synchronous one
/// expects, as the legacy layer read it ([`crate::legacy::pact::legacy_message`]).
pub struct LegacyMessage {
  /// Its metadata, by key, as the pact gives it.
  pub metadata: BTreeMap<String, Value>,
  /// Its contents, read by their content type as a body is. `None` when it declares none, which
  /// asserts nothing about a produced message's, as today.
  pub body: Option<LegacyBody>,
  /// Its content type as today's verifier takes it: the contents' own, else the metadata's
  /// `contentType`, else detected. A produced message's must be equivalent to it.
  pub content_type: Option<String>,
  /// Its rules: `body` for the contents and `metadata` for the metadata. `pact_models` reads a v4
  /// message's `content` rules into `body`, the one category it has for contents, so pact-jvm's
  /// `body` and pact-js's `content` are the same rules, as they are today.
  pub matching_rules: MatchingRules,
  /// How many messages it expects back: one, or a synchronous message's response count. Today's
  /// verifier compares one, so any other count that is not zero fails, as it does today.
  pub responses: usize,
}

/// Compile a v1–v4 message to a plan rooted at `part` (staged plan S1.7), as today's verifier
/// compares one (`pact_matching::match_message`): every metadata key the pact names, by its rule,
/// else as a MIME type for `contentType`, else by equality; then, when the pact declares contents,
/// the produced message's content type, which must be equivalent to the pact's, and the contents
/// as a response body is compared, unexpected members allowed.
pub fn compile_message(message: &LegacyMessage, part: &str) -> model::Plan {
  let mut slots = Vec::new();
  if message.responses > 1 {
    slots.push(Node::action(
      "flow:error",
      vec![Node::value(Literal::string(format!(
        "Expected a message with {} responses, but the actual response had 1",
        message.responses
      )))],
    ));
  }
  if !message.metadata.is_empty() {
    let category = rules_for(&message.matching_rules, "metadata");
    let base = format!("$.{}", path::root(part, "metadata"));
    let nodes = message
      .metadata
      .iter()
      .map(|(key, example)| {
        let at = member_path(&base, key);
        let resolve = Node::resolve(at.clone());
        let fragments = vec!["$".to_string(), key.clone()];
        let text = if example.is_string() { Text::Yes } else { Text::No };
        let node = match best_rule(category.as_ref(), &fragments, false) {
          Some((list, cascaded)) => {
            rule_list_node(&list, cascaded, resolve, &Literal::from_json(example), text)
          }
          None if key.eq_ignore_ascii_case("contenttype") || key.eq_ignore_ascii_case("content-type") => {
            let expected = match example {
              Value::String(text) => text.clone(),
              other => other.to_string(),
            };
            Node::action(
              "legacy:header-value",
              vec![resolve, Node::value(Literal::string(&expected))],
            )
          }
          None => Node::action(
            "match:equality",
            vec![resolve, Node::value(Literal::from_json(example))],
          ),
        };
        Node::container(Some(at), vec![node])
      })
      .collect();
    slots.push(Node::container(Some("metadata".to_string()), nodes));
  }
  if let Some(body) = &message.body {
    let root_path = format!("$.{}", path::root(part, "body"));
    let mut nodes = Vec::new();
    if let Some(content_type) = &message.content_type {
      nodes.push(Node::action(
        "legacy:message-type",
        vec![
          Node::resolve(format!("$.{part}.content-type")),
          Node::value(Literal::string(content_type)),
        ],
      ));
    }
    nodes.extend(compile_body_content(
      body,
      rules_for(&message.matching_rules, "body").as_ref(),
      false,
      &root_path,
    ));
    slots.push(Node::container(Some("body".to_string()), nodes));
  }
  model::Plan {
    grammar: model::GRAMMAR_VERSION,
    root: Node::container(Some(part.to_string()), slots),
    variant: None,
  }
}

/// Compile a v1–v4 request to a plan.
pub fn compile_request(req: &LegacyRequest) -> model::Plan {
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
      compile_parameters(
        &req.query,
        rules_for(&req.matching_rules, "query").as_ref(),
        &format!("$.{}", path::root("request", "query")),
      ),
    ),
  ];
  slots.extend(compile_headers_slot(&req.headers, &req.matching_rules, "request"));
  let check = body_type_check(
    req.body.as_ref(),
    &req.headers,
    req.content_type.as_deref(),
    "request",
  );
  if let Some(node) = compile_body_slot(req.body.as_ref(), &req.matching_rules, "request", check) {
    slots.push(node);
  }
  model::Plan {
    grammar: model::GRAMMAR_VERSION,
    root: Node::container(Some("request".to_string()), slots),
    variant: None,
  }
}

/// Compile a v1–v4 response to a plan.
pub fn compile_response(res: &LegacyResponse) -> model::Plan {
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
  slots.extend(compile_headers_slot(
    &res.headers,
    &res.matching_rules,
    "response",
  ));
  let check = body_type_check(
    res.body.as_ref(),
    &res.headers,
    res.content_type.as_deref(),
    "response",
  );
  if let Some(node) = compile_body_slot(res.body.as_ref(), &res.matching_rules, "response", check) {
    slots.push(node);
  }
  model::Plan {
    grammar: model::GRAMMAR_VERSION,
    root: Node::container(Some("response".to_string()), slots),
    variant: None,
  }
}

/// A whole interaction — request and response together — as one plan, described the way
/// [`crate::plan::compile`] labels a shape-compiled interaction's root: mirrors
/// [`compile_request`]/[`compile_response`]'s own root containers as its two children, so a v1–v4
/// interaction's overall verdict (both sides, not just one) is one plan a golden-corpus case (plan
/// task 3.7) or `explain` can render and execute in a single pass.
pub fn compile_interaction(
  description: &str,
  request: &LegacyRequest,
  response: &LegacyResponse,
) -> model::Plan {
  let request_plan = compile_request(request);
  let response_plan = compile_response(response);
  model::Plan {
    grammar: model::GRAMMAR_VERSION,
    root: Node::container(
      Some(description.to_string()),
      vec![request_plan.root, response_plan.root],
    ),
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

/// The winning rule at one position, for a caller that is not compiling a plan — the v1–v4 → shape
/// converter (contract-file spec §8.2, plan task 5.5), which has to resolve exactly the same
/// question this module answers for plans: *which* declared rule governs this path, and did it
/// cascade down from a shallower one.
///
/// Exported rather than reimplemented because plan-grammar spec §1 assigns v1–v4 cascading and
/// precedence to **this module alone**. A converter that scored rules its own way could upgrade a
/// pact into a contract that verifies differently from the pact — which is precisely the failure
/// §4.4's two-path agreement rule exists to forbid.
pub fn winning_rule(
  rules: &MatchingRules,
  category_name: &str,
  fragments: &[String],
  case_insensitive: bool,
) -> Option<(RuleList, bool)> {
  best_rule(
    rules_for(rules, category_name).as_ref(),
    fragments,
    case_insensitive,
  )
}

// --- a single rule -> a single match node ---

/// A v1–v4 regex, as a pattern `match:regex` reads the way today's verifiers do: matching the whole
/// value (staged plan S1.18). pact-reference matches with Oniguruma's `is_match`, which requires the
/// match to span the string, and pact-jvm with Kotlin's `String.matches`; `match:regex` matches
/// anywhere (shape-language spec §4.2), so the pattern is anchored here, where v1–v4 semantics live.
///
/// Always wrapped, never only when it looks unanchored: `^a|b$` starts with `^` and ends with `$`,
/// yet `b$` alone matches `xb`. The group keeps an alternation inside the anchors.
pub fn anchored_pattern(pattern: &str) -> String {
  format!("^(?:{pattern})$")
}

/// One [`MatchingRule`] to one plan node. `example` is the expected value at this path, carried
/// for the same reason the shape compiler carries one (plan-grammar spec §5.2): a mismatch message
/// worth reading names what was expected, not just that something failed.
///
/// On an object or an array, `Values`, `EachKey`, `EachValue` and `ArrayContains` describe its
/// entries, and the body walk compiles them ([`compile_object_entries`], [`compile_array_rules`]).
/// What reaches here is the value they land on otherwise: `Values` compares it by equality, and the
/// other three accept any value in a document, as today's JSON matcher does. A text value one of
/// them cascaded down to is accepted too, since today's string matcher passes a rule that cannot
/// cascade (`can_cascade`) when it did. Declared on text itself (a path, a header, a query or form
/// value, XML), they are still a documented gap, as are plugin-provided rules (module docs' "Scope").
fn rule_node(rule: &MatchingRule, resolve: Node, example: &Literal, text: Text, cascaded: bool) -> Node {
  match rule {
    // Text is read as today's verifier reads it against a type rule (module docs).
    MatchingRule::Number | MatchingRule::Integer | MatchingRule::Decimal | MatchingRule::Boolean
      if text == Text::Yes =>
    {
      let kind = match rule {
        MatchingRule::Number => "number",
        MatchingRule::Integer => "integer",
        MatchingRule::Decimal => "decimal",
        _ => "boolean",
      };
      Node::action(
        "legacy:parses-as",
        vec![resolve, Node::value(Literal::string(kind))],
      )
    }
    MatchingRule::Equality | MatchingRule::Values => {
      Node::action("match:equality", vec![resolve, Node::value(example.clone())])
    }
    MatchingRule::EachKey(_) | MatchingRule::EachValue(_) if text == Text::No || cascaded => {
      Node::action("match:any", vec![resolve])
    }
    MatchingRule::ArrayContains(_) if text == Text::No => Node::action("match:any", vec![resolve]),
    MatchingRule::Regex(pattern) => Node::action(
      "match:regex",
      vec![resolve, Node::value(Literal::string(anchored_pattern(pattern)))],
    ),
    MatchingRule::Type => Node::action("match:type", vec![resolve, Node::value(example.clone())]),
    MatchingRule::MinType(min) => Node::action(
      "legacy:min-type",
      vec![
        resolve,
        Node::value(example.clone()),
        Node::value(Literal::number(*min as u64)),
      ],
    ),
    MatchingRule::MaxType(max) => Node::action(
      "legacy:max-type",
      vec![
        resolve,
        Node::value(example.clone()),
        Node::value(Literal::number(*max as u64)),
      ],
    ),
    MatchingRule::MinMaxType(min, max) => Node::action(
      "legacy:min-max-type",
      vec![
        resolve,
        Node::value(example.clone()),
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
      "flow:error",
      vec![Node::value(Literal::string(format!(
        "legacy matching rule '{}' is not yet supported by the compiled plan",
        other.name()
      )))],
    ),
  }
}

/// A [`RuleList`] to one node: the single rule's node, or every rule's node combined by
/// `RuleLogic::And`/`Or` (plan-grammar spec §4.2's `flow:and`/`flow:or` control actions — the interpreter
/// already short-circuits `flow:or` and conjoins `flow:and`, so the combination needs no special handling
/// here beyond picking the right wrapper).
/// One position's winning [`RuleList`] to its node. A `cascaded` list reached this position from a
/// shallower path, and a cascaded `MinType`/`MaxType`/`MinMaxType` is reduced to `Type`: the bound
/// is the declaring array's own size, and v1–v4 never re-apply it below that array — not to a
/// scalar, and not to a nested array either (`pact_matching`'s JSON matcher checks the bound only
/// when `!cascaded`). Emitting the bounded action would both misread in `explain` and misbehave.
fn rule_list_node(list: &RuleList, cascaded: bool, resolve: Node, example: &Literal, text: Text) -> Node {
  let nodes: Vec<Node> = list
    .rules
    .iter()
    .map(|rule| match rule {
      MatchingRule::MinType(_) | MatchingRule::MaxType(_) | MatchingRule::MinMaxType(_, _) if cascaded => {
        rule_node(&MatchingRule::Type, resolve.clone(), example, text, cascaded)
      }
      _ => rule_node(rule, resolve.clone(), example, text, cascaded),
    })
    .collect();
  combine(list, nodes)
}

/// Whether a value is text, which a type rule reads by parsing it (module docs), or a document value,
/// which it reads by its kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Text {
  Yes,
  No,
}

/// One node per rule, combined as the list says: the one node alone, or every one under `flow:and`
/// or `flow:or`.
fn combine(list: &RuleList, mut nodes: Vec<Node>) -> Node {
  match nodes.len() {
    1 => nodes.pop().expect("checked len == 1"),
    _ => {
      let family = match list.rule_logic {
        RuleLogic::And => "flow:and",
        RuleLogic::Or => "flow:or",
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
  let text = if example.is_string() { Text::Yes } else { Text::No };
  match best_rule(category.as_ref(), &fragments, false) {
    Some((list, cascaded)) => rule_list_node(&list, cascaded, resolve, &Literal::from_json(example), text),
    // Method is the one scalar slot v1-v4 compares case-insensitively by default (spec test case
    // `method/method is different case`) — expressed with the existing `value:upper-case` core action
    // rather than a new one, since `match:equality`'s own children may be any value-producing node.
    // Upper, not lower: that is the method's canonical form, so a mismatch reads `'POST'` against
    // `'GET'`. Only the actual needs it — `pact_models` upper-cases the example when it loads.
    None if category_name == "method" => Node::action(
      "match:equality",
      vec![
        Node::action("value:upper-case", vec![resolve]),
        Node::value(Literal::from_json(example)),
      ],
    ),
    None => Node::action(
      "match:equality",
      vec![resolve, Node::value(Literal::from_json(example))],
    ),
  }
}

// --- headers: scalar values, case-insensitive names, no cascading ---

/// The `headers` slot, or nothing when the pact names no headers. v1–v4 headers are open — an
/// unexpected actual header is never a mismatch — so an empty container would assert nothing and
/// only add a line to every plan `explain` prints.
fn compile_headers_slot(
  headers: &BTreeMap<String, String>,
  rules: &MatchingRules,
  part: &str,
) -> Option<Node> {
  (!headers.is_empty())
    .then(|| Node::container(Some("headers".to_string()), compile_headers(headers, rules, part)))
}

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
        Some((list, cascaded)) => {
          rule_list_node(&list, cascaded, resolve, &Literal::from_json(&example), Text::Yes)
        }
        // The v1-v4 default header comparison (module docs): not plain string equality — a
        // multi-valued (comma-separated) header tolerates whitespace around the commas, and a
        // MIME-shaped one (`type/subtype; param=value`) compares type and parameters as a set,
        // extra actual parameters allowed. Legacy-only: no shape operator needs this.
        None => Node::action(
          "legacy:header-value",
          vec![resolve, Node::value(Literal::from_json(&example))],
        ),
      };
      Node::container(
        Some(format!("$.{}.{name}", path::root(part, "headers"))),
        vec![node],
      )
    })
    .collect()
}

// --- query: value lists, order-sensitive by default, per-value rule lookup (cascades) ---

/// v1–v4's query default is closed, unlike headers' (spec test case `query/unexpected param`):
/// every actual query parameter must be named in `expected`.
///
/// A form body is compared by the same rules (`pact_matching::match_form_urlencoded` hands both bodies
/// to the query matcher), so this compiles one too, with the `body` category's rules at `base_path`
/// (ADR 0038). Names are compiled by name.
fn compile_parameters(
  query: &BTreeMap<String, Vec<String>>,
  category: Option<&MatchingRuleCategory>,
  base_path: &str,
) -> Vec<Node> {
  let base_path = base_path.to_string();
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
        let node = match best_rule(category, &fragments, false) {
          Some((list, cascaded)) => {
            rule_list_node(&list, cascaded, resolve, &Literal::from_json(&example), Text::Yes)
          }
          None => Node::action(
            "match:equality",
            vec![resolve, Node::value(Literal::from_json(&example))],
          ),
        };
        children.push(node);
      }
      Node::container(Some(format!("{base_path}.{name}")), children)
    })
    .collect();
  let names: Vec<Node> = query.keys().map(|k| Node::value(Literal::string(k))).collect();
  let mut only_entries = vec![Node::resolve(base_path)];
  only_entries.extend(names);
  nodes.push(Node::action("legacy:only-entries", only_entries));
  nodes
}

// --- body: the recursive, cascading walk ---

/// The check today's verifier makes before it compares a body (`pact_matching::match_body`): the
/// received body's content type agrees with the pact's (`legacy:body-type`, ADR 0036). Only when the
/// pact names no `Content-Type` header, whose own check already covers it, and only for a body with
/// content, of a type that is not unknown.
fn body_type_check(
  body: Option<&LegacyBody>,
  headers: &BTreeMap<String, String>,
  content_type: Option<&str>,
  part: &str,
) -> Option<Node> {
  let has_content = matches!(
    body,
    Some(LegacyBody::Document(value)) if !matches!(value, RuntimeValue::Null | RuntimeValue::Absent)
  ) || matches!(
    body,
    Some(LegacyBody::Form(_) | LegacyBody::Xml(_) | LegacyBody::Multipart(_))
  );
  let named = headers
    .keys()
    .any(|name| name.eq_ignore_ascii_case("content-type"));
  let content_type = content_type.filter(|content_type| !content_type.contains('*'))?;
  (has_content && !named).then(|| {
    Node::action(
      "legacy:body-type",
      vec![
        Node::resolve(format!("$.{}.content-type", path::root(part, "headers"))),
        Node::value(Literal::string(content_type)),
      ],
    )
  })
}

fn compile_body_slot(
  body: Option<&LegacyBody>,
  rules: &MatchingRules,
  part: &str,
  check: Option<Node>,
) -> Option<Node> {
  let root_path = format!("$.{}", path::root(part, "body"));
  let container =
    |nodes: Vec<Node>| Node::container(Some("body".to_string()), check.into_iter().chain(nodes).collect());
  match body {
    // No body was part of this interaction at all: nothing to assert (spec test case `body/no
    // body`).
    None => None,
    // Explicitly null: v1-v4 requires the actual body be absent too (`body/non empty body found
    // when empty expected` is a mismatch on exactly this) — an assertion, not a skip.
    Some(LegacyBody::Empty) | Some(LegacyBody::Document(RuntimeValue::Null | RuntimeValue::Absent)) => {
      Some(container(vec![Node::action(
        "expect:absent",
        vec![Node::resolve(root_path)],
      )]))
    }
    Some(body) => Some(container(compile_body_content(
      body,
      rules_for(rules, "body").as_ref(),
      part == "request",
      &root_path,
    ))),
  }
}

/// A body with content, at `root_path`, under `category`'s rules (rooted at the body's `$`), closed
/// to unnamed members when `closed` (a request's): the whole body, or one multipart part's.
fn compile_body_content(
  body: &LegacyBody,
  category: Option<&MatchingRuleCategory>,
  closed: bool,
  root_path: &str,
) -> Vec<Node> {
  match body {
    // A part is never empty: an empty body is the slot's to assert (`compile_body_slot`).
    LegacyBody::Empty => Vec::new(),
    // Names and their values, every name required whichever part this is, as a query string's.
    LegacyBody::Form(form) => compile_parameters(&form_parameters(form), category, root_path),
    // Elements, attributes and text, as today's `match_xml` walks them.
    LegacyBody::Xml(document) => compile_xml_body(document, category, closed, root_path),
    // Parts by name, each by its own type, as today's `match_mime_multipart` compares them.
    LegacyBody::Multipart(parts) => compile_multipart_body(parts, category, closed, root_path),
    // Structure, or one value at `$`: a string, or octets compared by the rules there or byte for
    // byte, as today's `match_octet_stream` compares them (ADR 0036 decision 3).
    LegacyBody::Document(body) => {
      let ctx = BodyCtx {
        fragments: vec!["$".to_string()],
        cursor: Cursor::Absolute(root_path.to_string()),
        label_path: root_path.to_string(),
        category,
        closed_objects: closed,
      };
      compile_body_value(body, &ctx)
    }
  }
}

/// A form document as the query compiler takes one: each name with its values as text. The form
/// component writes only strings; anything else is taken as its JSON text rather than dropped.
fn form_parameters(form: &RuntimeValue) -> BTreeMap<String, Vec<String>> {
  let text = |value: &RuntimeValue| match value {
    RuntimeValue::String(text) => text.clone(),
    other => other.to_json().to_string(),
  };
  let RuntimeValue::Object(members) = form else {
    return BTreeMap::new();
  };
  members
    .iter()
    .map(|(name, values)| {
      let values = match values {
        RuntimeValue::Array(values) => values.iter().map(text).collect(),
        value => vec![text(value)],
      };
      (name.clone(), values)
    })
    .collect()
}

/// Where a compiled node resolves its value from — [`crate::plan::compile`]'s own `Cursor`, duplicated
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

  /// A member by a name that may need quoting: an XML name's `@`, `#`, `{` (ADR 0039).
  fn member_quoted(&self, name: &str) -> Cursor {
    match self {
      Cursor::Absolute(p) => Cursor::Absolute(member_path(p, name)),
      Cursor::Relative(p) => Cursor::Relative(member_path(p, name)),
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
  /// relative to a `flow:for-each`'s current item once one has been entered).
  cursor: Cursor,
  /// The dotted display path (`$.request.body.animals[*]`) a container labels itself with —
  /// always absolute-looking and always growing, independent of `cursor` (plan task 3.3's
  /// `Ctx::at`/`Ctx::label` play the identical role for the shape compiler).
  label_path: String,
  category: Option<&'a MatchingRuleCategory>,
  /// Whether an object encountered under this context is closed to unnamed members
  /// (`legacy:only-entries`) or must-ignores them. v1–v4's default is asymmetric by part — request
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

  /// The item context inside a rule-driven array's `flow:for-each` (module docs): a fresh relative
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

fn compile_body_value(value: &RuntimeValue, ctx: &BodyCtx) -> Vec<Node> {
  match value {
    RuntimeValue::Object(members) => compile_body_object(members, ctx),
    RuntimeValue::Array(items) => compile_body_array(items, ctx),
    _ => vec![compile_body_scalar(value, ctx)],
  }
}

fn compile_body_scalar(value: &RuntimeValue, ctx: &BodyCtx) -> Node {
  match ctx.best() {
    Some((list, cascaded)) => rule_list_node(
      &list,
      cascaded,
      ctx.resolve(),
      &Literal::from_document(value),
      Text::No,
    ),
    None => Node::action(
      "match:equality",
      vec![ctx.resolve(), Node::value(Literal::from_document(value))],
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
/// one of the map-entry rules ([`is_map_entry_rule`]), which [`compile_object_entries`] compiles:
/// that mirrors the oracle's own behaviour (a bare `Type`/`Regex`/`Equality` declared on an object
/// path does not replace the structural walk), confirmed against `tests/fixtures/spec_testcases`
/// rather than assumed.
///
/// Members are compiled by name, not in the order the pact wrote them (ADR 0037): a v1–v4 JSON
/// object's members are a set, and a pact's plan must not change when a broker or a formatter
/// writes it out again.
fn compile_body_object(members: &IndexMap<String, RuntimeValue>, ctx: &BodyCtx) -> Vec<Node> {
  if let Some((list, false)) = ctx.best()
    && list.rules.iter().any(is_map_entry_rule)
  {
    return compile_object_entries(members, &list, ctx);
  }
  let mut by_name: Vec<(&String, &RuntimeValue)> = members.iter().collect();
  by_name.sort_by_key(|(name, _)| *name);
  let mut nodes: Vec<Node> = by_name
    .iter()
    .map(|(name, value)| {
      let member_ctx = ctx.member(name);
      Node::container(Some(member_ctx.label()), compile_body_value(value, &member_ctx))
    })
    .collect();
  if ctx.closed_objects {
    let names: Vec<Node> = by_name
      .iter()
      .map(|(k, _)| Node::value(Literal::string(*k)))
      .collect();
    let mut only_entries = vec![ctx.resolve()];
    only_entries.extend(names);
    nodes.push(Node::action("legacy:only-entries", only_entries));
  }
  nodes
}

/// The rules that, declared on an object's own path, describe its entries rather than leave its
/// members to be walked by name: `pact_models`' values matchers (`is_values_matcher`: `Values`,
/// `EachValue` and `ArrayContains`) and `EachKey`. Cascaded down to an object, none of them does
/// anything to it, as today (`compare_maps_with_matchingrule`).
fn is_map_entry_rule(rule: &MatchingRule) -> bool {
  matches!(
    rule,
    MatchingRule::Values
      | MatchingRule::EachKey(_)
      | MatchingRule::EachValue(_)
      | MatchingRule::ArrayContains(_)
  )
}

/// `array` (module docs): a rule reaching this path (exactly or cascaded — `matcher_is_defined`'s
/// own behaviour, not just an exact hit) switches the array from the strict, no-rule default
/// (`expect:count` plus one compiled container per literal index) to the rule-driven form: the
/// rule's own node, plus (when there is a first element to serve as one) a `flow:for-each` compiling
/// that one element once as a template and applying it to every actual element at run time —
/// exactly [`compile.rs`](crate::plan::compile)'s `each-like`, with the item shape replaced by whatever
/// rule cascades into the template's own path.
fn compile_body_array(items: &[RuntimeValue], ctx: &BodyCtx) -> Vec<Node> {
  match ctx.best() {
    Some((list, cascaded)) if list.rules.iter().any(is_map_entry_rule) => {
      compile_array_rules(items, &list, cascaded, ctx)
    }
    Some((list, cascaded)) => {
      let mut nodes = vec![rule_list_node(
        &list,
        cascaded,
        ctx.resolve(),
        &Literal::from_document(&RuntimeValue::Array(items.to_vec())),
        Text::No,
      )];
      if let Some(template) = items.first() {
        let item_ctx = ctx.template_item();
        let item_node = Node::container(Some(item_ctx.label()), compile_body_value(template, &item_ctx));
        nodes.push(Node::action(
          "flow:for-each",
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

// --- the rules that describe a collection's entries: Values, EachKey, EachValue, ArrayContains ---
//
// Staged plan S1.24, ADR 0042. Today's verifier reads these four on an object or an array as being
// about its entries, not its members by name (`compare_maps_with_matchingrule`,
// `compare_lists_with_matchingrule`). What follows compiles each the way those functions apply it.

/// The fragment an entry template is compiled under when the actual key is one the pact does not
/// name: no rule names it, it is not an index, so only a `*` rule reaches it, as only a `*` rule
/// reaches a key nobody wrote down.
const ANY_KEY: &str = "\u{0}*";

impl<'a> BodyCtx<'a> {
  /// One entry of the object under this context, inside a `flow:for-each` over its entries: the
  /// value is `~>.value`, and rules are looked up at the entry's key, or at [`ANY_KEY`] for a key
  /// the template does not name. It is labelled as the member it is (`.red`), or as any entry's
  /// value (`{*}`, shape-language spec §6.2) for a key the template does not name.
  fn entry(&self, key: Option<&str>) -> BodyCtx<'a> {
    let mut fragments = self.fragments.clone();
    fragments.push(key.unwrap_or(ANY_KEY).to_string());
    BodyCtx {
      fragments,
      cursor: Cursor::Relative("~>.value".to_string()),
      label_path: match key {
        Some(key) => path::member(&self.label_path, key),
        None => path::each_entry_value(&self.label_path),
      },
      category: self.category,
      closed_objects: self.closed_objects,
    }
  }

  /// The same position, with other rules: what `EachValue` and an `ArrayContains` variant do to
  /// today's matching context (`clone_with`), which replaces every rule, not only the ones here.
  fn with_category<'b>(&self, category: Option<&'b MatchingRuleCategory>) -> BodyCtx<'b> {
    BodyCtx {
      fragments: self.fragments.clone(),
      cursor: self.cursor.clone(),
      label_path: self.label_path.clone(),
      category,
      closed_objects: self.closed_objects,
    }
  }

  /// The names a rule gives to members of the object under this context: a rule path that reaches
  /// here and goes on through a named member.
  fn names_ruled_below(&self) -> BTreeSet<String> {
    let Some(category) = self.category else {
      return BTreeSet::new();
    };
    let depth = self.fragments.len();
    category
      .rules
      .keys()
      .filter_map(|rule_path| {
        let tokens = rule_path.tokens();
        let reaches = tokens.len() > depth
          && tokens
            .iter()
            .zip(&self.fragments)
            .all(|(token, fragment)| token_weight(token, fragment, false) > 0);
        match tokens.get(depth) {
          Some(PathToken::Field(name)) if reaches => Some(name.clone()),
          _ => None,
        }
      })
      .collect()
  }
}

/// An object whose own path carries one of [`is_map_entry_rule`]'s rules, as today's
/// `compare_maps` applies each rule of the list in turn, all of them required whatever the list's
/// logic says:
///
/// - `Values` and `ArrayContains` compare every actual entry ([`compile_entry_values`]);
/// - `EachValue` does the same with only its own rules, at `<path>.*`;
/// - any other rule, `EachKey` among them, checks the object as a whole and walks it by name
///   ([`compile_object_walk`]).
fn compile_object_entries(
  members: &IndexMap<String, RuntimeValue>,
  list: &RuleList,
  ctx: &BodyCtx,
) -> Vec<Node> {
  let mut nodes = vec![Node::action("expect:object", vec![ctx.resolve()])];
  // Today's first check, before any rule: a request's empty object has no entries either.
  if members.is_empty() && ctx.closed_objects {
    nodes.push(Node::action("legacy:only-entries", vec![ctx.resolve()]));
  }
  let mut walked = false;
  for rule in &list.rules {
    match rule {
      MatchingRule::Values | MatchingRule::ArrayContains(_) => {
        nodes.extend(compile_entry_values(members, ctx))
      }
      MatchingRule::EachValue(definition) => {
        let (rules, unresolved) = definition_rules(definition);
        nodes.extend(unresolved);
        let category = each_value_category(&ctx.fragments, rules);
        nodes.extend(compile_entry_values(members, &ctx.with_category(Some(&category))));
      }
      other => {
        nodes.extend(object_rule_check(other, members, ctx));
        if !walked {
          walked = true;
          nodes.extend(compile_object_walk(members, list, ctx));
        }
      }
    }
  }
  nodes
}

/// Every actual entry against the pact's: the value the pact gives its key, or the pact's first
/// value (by key) for a key it does not give. A key the pact gives and the reply lacks is not a
/// mismatch, nor is a key the reply adds. What today's values matcher does.
fn compile_entry_values(members: &IndexMap<String, RuntimeValue>, ctx: &BodyCtx) -> Vec<Node> {
  let mut by_name: Vec<(&String, &RuntimeValue)> = members.iter().collect();
  by_name.sort_by_key(|(name, _)| *name);
  let Some((_, first)) = by_name.first() else {
    return Vec::new();
  };
  let compiled = |value: &RuntimeValue, key: Option<&str>| {
    let entry = ctx.entry(key);
    Node::container(Some(entry.label()), compile_body_value(value, &entry))
  };
  // A key a rule names but the pact does not is compared against the first value with that rule,
  // as today looks rules up at the actual key.
  let ruled: Vec<String> = ctx
    .names_ruled_below()
    .into_iter()
    .filter(|name| !members.contains_key(name))
    .collect();
  let mut template = compiled(first, None);
  let branches = by_name
    .iter()
    .map(|(name, value)| (name.as_str(), *value))
    .chain(ruled.iter().map(|name| (name.as_str(), *first)));
  let branches: Vec<(&str, &RuntimeValue)> = branches.collect();
  for (name, value) in branches.into_iter().rev() {
    template = Node::action(
      "flow:if",
      vec![
        Node::action(
          "check:equals",
          vec![
            Node::resolve_current("~>.key".to_string()),
            Node::value(Literal::string(name)),
          ],
        ),
        compiled(value, Some(name)),
        template,
      ],
    );
  }
  vec![Node::action(
    "flow:for-each",
    vec![Node::splat(vec![ctx.resolve()]), template],
  )]
}

/// The object walked by name under a rule that is not a values matcher, as today's `match_keys`
/// and member comparison do: its keys must be the pact's (a request's exactly, a response's at
/// least) unless the list has `Values`, `EachValue` or `EachKey`; every actual key meets each
/// `EachKey` rule; and each member the pact gives is compared when the reply has it.
fn compile_object_walk(
  members: &IndexMap<String, RuntimeValue>,
  list: &RuleList,
  ctx: &BodyCtx,
) -> Vec<Node> {
  let mut by_name: Vec<(&String, &RuntimeValue)> = members.iter().collect();
  by_name.sort_by_key(|(name, _)| *name);
  let mut nodes = Vec::new();
  let any_keys = list.rules.iter().any(|rule| {
    matches!(
      rule,
      MatchingRule::Values | MatchingRule::EachValue(_) | MatchingRule::EachKey(_)
    )
  });
  if !any_keys {
    if ctx.closed_objects {
      let mut only = vec![ctx.resolve()];
      only.extend(
        by_name
          .iter()
          .map(|(name, _)| Node::value(Literal::string(*name))),
      );
      nodes.push(Node::action("legacy:only-entries", only));
    }
    nodes.extend(
      by_name
        .iter()
        .map(|(name, _)| Node::action("match:any", vec![ctx.member(name).resolve()])),
    );
  }
  for rule in &list.rules {
    if let MatchingRule::EachKey(definition) = rule {
      let (rules, unresolved) = definition_rules(definition);
      nodes.extend(unresolved);
      let key = Node::resolve_current("~>.key".to_string());
      let checks: Vec<Node> = rules
        .iter()
        .map(|rule| rule_node(rule, key.clone(), &Literal::string(""), Text::Yes, false))
        .collect();
      if !checks.is_empty() {
        let label = path::each_entry_key(&ctx.label_path);
        nodes.push(Node::action(
          "flow:for-each",
          vec![
            Node::splat(vec![ctx.resolve()]),
            Node::container(Some(label), checks),
          ],
        ));
      }
    }
  }
  for (name, value) in by_name {
    let member = ctx.member(name);
    nodes.push(Node::action(
      "flow:if",
      vec![
        Node::action("check:exists", vec![member.resolve()]),
        Node::container(Some(member.label()), compile_body_value(value, &member)),
      ],
    ));
  }
  nodes
}

/// What a rule other than a values matcher says about an object as a whole, as today's map matcher
/// reads it: equality is the whole object's, a size bound counts its entries, and a rule about a kind
/// of value cannot match an object at all.
fn object_rule_check(
  rule: &MatchingRule,
  members: &IndexMap<String, RuntimeValue>,
  ctx: &BodyCtx,
) -> Vec<Node> {
  let bounds = |min: usize, max: Option<usize>| {
    vec![Node::action(
      "expect:size",
      vec![
        ctx.resolve(),
        Node::value(Literal::number(min as u64)),
        Node::value(
          max
            .map(|max| Literal::number(max as u64))
            .unwrap_or_else(Literal::null),
        ),
      ],
    )]
  };
  match rule {
    MatchingRule::Equality => vec![Node::action(
      "match:equality",
      vec![
        ctx.resolve(),
        Node::value(Literal::from_document(&RuntimeValue::Object(members.clone()))),
      ],
    )],
    MatchingRule::MinType(min) => bounds(*min, None),
    MatchingRule::MaxType(max) => bounds(0, Some(*max)),
    MatchingRule::MinMaxType(min, max) => bounds(*min, Some(*max)),
    MatchingRule::NotEmpty => vec![Node::action("expect:not-empty", vec![ctx.resolve()])],
    MatchingRule::Regex(_)
    | MatchingRule::Type
    | MatchingRule::Values
    | MatchingRule::EachKey(_)
    | MatchingRule::EachValue(_)
    | MatchingRule::ArrayContains(_) => Vec::new(),
    other => vec![Node::action(
      "flow:error",
      vec![Node::value(Literal::string(format!(
        "Unable to match an object using '{}'",
        other.name()
      )))],
    )],
  }
}

/// An array whose rules include one of [`is_map_entry_rule`]'s, as today's
/// `compare_lists_with_matchingrules` applies them: each rule on its own, combined as the list says.
/// A pact's empty array is checked against no rule at all, as today.
fn compile_array_rules(items: &[RuntimeValue], list: &RuleList, cascaded: bool, ctx: &BodyCtx) -> Vec<Node> {
  let mut nodes = vec![Node::action("expect:array", vec![ctx.resolve()])];
  if items.is_empty() {
    return nodes;
  }
  // Every element against the pact's first, under `ctx`'s rules: what today's `match_list_contents`
  // does after every rule but `ArrayContains` and `EachValue`.
  let contents = |ctx: &BodyCtx| -> Vec<Node> {
    let item = ctx.template_item();
    vec![Node::action(
      "flow:for-each",
      vec![
        Node::splat(vec![ctx.resolve()]),
        Node::container(Some(item.label()), compile_body_value(&items[0], &item)),
      ],
    )]
  };
  let example = Literal::from_document(&RuntimeValue::Array(items.to_vec()));
  let per_rule: Vec<(Vec<Node>, bool)> = list
    .rules
    .iter()
    .map(|rule| match rule {
      MatchingRule::ArrayContains(variants) => (compile_array_contains(items, variants, ctx), false),
      // Cascaded, today's `EachValue` checks nothing on a list.
      MatchingRule::EachValue(_) if cascaded => (Vec::new(), false),
      MatchingRule::EachValue(definition) => {
        let (rules, mut unresolved) = definition_rules(definition);
        let category = each_value_category(&ctx.fragments, rules);
        unresolved.extend(contents(&ctx.with_category(Some(&category))));
        (unresolved, false)
      }
      MatchingRule::Values | MatchingRule::EachKey(_) => (Vec::new(), true),
      other => {
        let single = RuleList {
          rules: vec![other.clone()],
          rule_logic: RuleLogic::And,
          cascaded,
        };
        (
          vec![rule_list_node(
            &single,
            cascaded,
            ctx.resolve(),
            &example,
            Text::No,
          )],
          true,
        )
      }
    })
    .collect();
  match list.rule_logic {
    RuleLogic::And => {
      let walks = per_rule.iter().any(|(_, walks)| *walks);
      for (rule_nodes, _) in per_rule {
        nodes.extend(rule_nodes);
      }
      if walks {
        nodes.extend(contents(ctx));
      }
    }
    RuleLogic::Or => {
      let alternatives: Vec<Node> = per_rule
        .into_iter()
        .map(|(mut rule_nodes, walks)| {
          if walks {
            rule_nodes.extend(contents(ctx));
          }
          Node::container(Some(ctx.label()), rule_nodes)
        })
        .collect();
      nodes.push(Node::action("flow:or", alternatives));
    }
  }
  nodes
}

/// `ArrayContains`: each variant, the pact's element at its index under the variant's own rules,
/// must match some element of the reply, in any order and not necessarily a different one. A
/// variant with no rules, or none at all, is that element compared by equality.
fn compile_array_contains(
  items: &[RuntimeValue],
  variants: &[(usize, MatchingRuleCategory, ArrayContainsGenerators)],
  ctx: &BodyCtx,
) -> Vec<Node> {
  let variants: Vec<(usize, Option<&MatchingRuleCategory>)> = if variants.is_empty() {
    (0..items.len()).map(|index| (index, None)).collect()
  } else {
    variants
      .iter()
      .map(|(index, rules, _)| (*index, Some(rules)))
      .collect()
  };
  variants
    .into_iter()
    .map(|(index, rules)| {
      let Some(item) = items.get(index) else {
        return Node::action(
          "flow:error",
          vec![Node::value(Literal::string(format!(
            "ArrayContains: variant {index} is missing from the expected list, which has {} items",
            items.len()
          )))],
        );
      };
      let element = BodyCtx {
        fragments: vec!["$".to_string()],
        cursor: Cursor::Relative("~>".to_string()),
        label_path: some_element(&ctx.label_path, index),
        category: rules.filter(|rules| !is_whole_equality(rules)),
        closed_objects: ctx.closed_objects,
      };
      let template = match rules {
        Some(rules) if !is_whole_equality(rules) => compile_body_value(item, &element),
        _ => vec![Node::action(
          "match:equality",
          vec![element.resolve(), Node::value(Literal::from_document(item))],
        )],
      };
      Node::action(
        "legacy:array-contains",
        vec![
          ctx.resolve(),
          Node::container(Some(element.label()), template),
          Node::value(Literal::number(index as u64)),
          Node::value(Literal::from_document(item)),
        ],
      )
    })
    .collect()
}

/// The element an `ArrayContains` variant is matched against (plan-grammar spec §4.4): `[?i]`,
/// whichever element of the array satisfies variant `i`. Neither `[i]`, the reply's `i`th element,
/// nor `[*]`, every element, says that, and the shape language has no segment for it, since only
/// the legacy compiler emits `legacy:array-contains`.
fn some_element(base: &str, index: usize) -> String {
  format!("{base}[?{index}]")
}

/// An `ArrayContains` variant's generators, which verification does not apply to a reply.
type ArrayContainsGenerators = std::collections::HashMap<DocPath, pact_models::generators::Generator>;

/// A variant `pact_models` read with no `rules`: equality on the empty path, which today applies to
/// the element as a whole, an object's extra keys included.
fn is_whole_equality(rules: &MatchingRuleCategory) -> bool {
  rules.rules.len() == 1
    && rules.rules.iter().all(|(rule_path, list)| {
      rule_path
        .tokens()
        .iter()
        .all(|token| matches!(token, PathToken::Root))
        && list.rules == [MatchingRule::Equality]
    })
}

/// An `EachKey` or `EachValue` definition's rules, and a failing node for each reference it names,
/// which `pact_models` leaves unresolved and today reports as a mismatch.
fn definition_rules(definition: &MatchingRuleDefinition) -> (Vec<MatchingRule>, Vec<Node>) {
  let mut rules = Vec::new();
  let mut unresolved = Vec::new();
  for rule in &definition.rules {
    if let Some(rule) = rule.as_ref().left() {
      rules.push(rule.clone());
    } else if let Some(reference) = rule.as_ref().right() {
      unresolved.push(Node::action(
        "flow:error",
        vec![Node::value(Literal::string(format!(
          "Found an un-resolved reference {}",
          reference.name
        )))],
      ));
    }
  }
  (rules, unresolved)
}

/// The rules `EachValue` matches a collection's values with, and nothing else: its own, at `*` below
/// the collection, as today's `EachValue` builds a new matching context.
fn each_value_category(fragments: &[String], rules: Vec<MatchingRule>) -> MatchingRuleCategory {
  let mut at = DocPath::root();
  for fragment in fragments.iter().skip(1) {
    at = at.join(fragment.as_str());
  }
  let mut category = MatchingRuleCategory::empty("body");
  category.rules.insert(
    at.join("*"),
    RuleList {
      rules,
      rule_logic: RuleLogic::And,
      cascaded: false,
    },
  );
  category
}

// --- XML: elements, attributes and text (ADR 0039) ---

/// What every element of one XML body is compiled under: the body's rules, and whether a request's
/// closed comparison applies (`DiffConfig::NoUnexpectedKeys`) or a response's open one.
struct XmlCtx<'a> {
  category: Option<&'a MatchingRuleCategory>,
  closed: bool,
}

/// An XML body, as today's `match_xml` compares one: the root element by its name, then each element
/// by its rule or its name, its attributes, its text and its children, recursively. The document is
/// the XML component's ([`crate::legacy::xml`]): an element's attributes are `@` members, its text
/// `#text`, its namespace `#namespace`, and its children an array per local name.
///
/// Rules are looked up by today's paths, not the document's: a child is its name with no index
/// (`$.animals.alligator`), a namespaced name is `<uri>:<local>`, an attribute `@<name>`, text
/// `#text`. Where today's verifier answers from a parsed element rather than a value, the plan asks
/// the same question of the document:
///
/// - a rule on an element compares its *name* (`Type`, `Equality`, `Regex`) or counts its *children*
///   (`MinType`, `MaxType`, `NotEmpty`); any other rule cannot match an element;
/// - children are grouped by name. A group a type rule reaches is compared item by item against the
///   pact's first; any other is compared index by index, and a request must have exactly as many;
/// - a request's element has only the attributes and children the pact names. A response's may have
///   others, except a child a type rule reaches (today's `type_matcher_defined`);
/// - text is compared even when the pact has none, and a rule on text sees an element with none as
///   the empty string.
///
/// An element the pact puts in a namespace must be in it; one in no namespace matches its local name
/// in any (ADR 0039 decision 4, pact-specification's `unexpected xml namespace`).
fn compile_xml_body(
  document: &RuntimeValue,
  category: Option<&MatchingRuleCategory>,
  closed: bool,
  root_path: &str,
) -> Vec<Node> {
  let root = match document {
    RuntimeValue::Object(members) if members.len() == 1 => members.first(),
    _ => None,
  };
  let Some((name, element)) = root else {
    return vec![Node::action(
      "flow:error",
      vec![Node::value(Literal::string(
        "an XML body is an object holding its one root element",
      ))],
    )];
  };
  let element = only_element(element);
  let ctx = XmlCtx { category, closed };
  let at = Cursor::Absolute(member_path(root_path, name));
  let fragments = vec!["$".to_string(), rule_name(name, namespace_of(element))];
  vec![
    Node::action("expect:object", vec![at.resolve()]),
    compile_xml_element(
      element,
      name,
      &fragments,
      &at,
      &member_path(root_path, name),
      &ctx,
    ),
  ]
}

/// An element the document holds as a one-element array, as a child list is, or alone.
fn only_element(value: &RuntimeValue) -> &RuntimeValue {
  match value {
    RuntimeValue::Array(items) if items.len() == 1 => &items[0],
    other => other,
  }
}

fn namespace_of(element: &RuntimeValue) -> Option<&str> {
  match element {
    RuntimeValue::Object(members) => match members.get(xml::NAMESPACE) {
      Some(RuntimeValue::String(namespace)) => Some(namespace.as_str()),
      _ => None,
    },
    _ => None,
  }
}

/// One element (see [`compile_xml_body`]): `fragments` is its rule path, `at` where its actual
/// counterpart resolves, and `label` its container's.
fn compile_xml_element(
  element: &RuntimeValue,
  name: &str,
  fragments: &[String],
  at: &Cursor,
  label: &str,
  ctx: &XmlCtx,
) -> Node {
  let RuntimeValue::Object(members) = element else {
    return Node::container(
      Some(label.to_string()),
      vec![Node::action(
        "flow:error",
        vec![Node::value(Literal::string(format!(
          "the XML element '{name}' is not an object"
        )))],
      )],
    );
  };
  let mut nodes = Vec::new();

  if let Some((list, cascaded)) = best_rule(ctx.category, fragments, false) {
    nodes.push(xml_element_rule(&list, cascaded, name, at));
  }
  if let Some(namespace) = namespace_of(element) {
    nodes.push(Node::action(
      "match:equality",
      vec![
        at.member_quoted(xml::NAMESPACE).resolve(),
        Node::value(Literal::string(namespace)),
      ],
    ));
  }

  // Attributes, by name.
  let mut attributes: Vec<(&String, &RuntimeValue)> =
    members.iter().filter(|(key, _)| key.starts_with('@')).collect();
  attributes.sort_by_key(|(key, _)| *key);
  for (key, value) in &attributes {
    let mut attribute_fragments = fragments.to_vec();
    attribute_fragments.push(format!("@{}", rule_name(&key[1..], None)));
    let attribute = at.member_quoted(key);
    let example = Literal::from_document(value);
    let node = match best_rule(ctx.category, &attribute_fragments, false) {
      Some((list, cascaded)) => rule_list_node(&list, cascaded, attribute.resolve(), &example, Text::Yes),
      None => Node::action("match:equality", vec![attribute.resolve(), Node::value(example)]),
    };
    nodes.push(Node::container(Some(member_path(label, key)), vec![node]));
  }

  // Text, compared whether or not the pact has any.
  let expected_text = match members.get(xml::TEXT) {
    Some(RuntimeValue::String(text)) => text.clone(),
    _ => String::new(),
  };
  let mut text_fragments = fragments.to_vec();
  text_fragments.push(xml::TEXT.to_string());
  let text = at.member_quoted(xml::TEXT);
  let text_node = match best_rule(ctx.category, &text_fragments, false) {
    Some((list, cascaded)) => {
      let example = Literal::string(&expected_text);
      let rule = rule_list_node(&list, cascaded, text.resolve(), &example, Text::Yes);
      // An element with no text is the empty string to a rule: where the rule accepts that, no text
      // is a pass; where it does not, an absent value fails it as the empty string would.
      if accepts_empty_text(&list, cascaded, &example) {
        Node::action(
          "flow:if",
          vec![Node::action("check:exists", vec![text.resolve()]), rule],
        )
      } else {
        rule
      }
    }
    // The XML component leaves out a `#text` that would be empty, so text that is there is not
    // empty, and is compared with the empty string the pact has.
    None if expected_text.is_empty() => Node::action(
      "flow:if",
      vec![
        Node::action("check:exists", vec![text.resolve()]),
        Node::action(
          "match:equality",
          vec![text.resolve(), Node::value(Literal::string(""))],
        ),
      ],
    ),
    None => Node::action(
      "match:equality",
      vec![text.resolve(), Node::value(Literal::string(&expected_text))],
    ),
  };
  nodes.push(Node::container(
    Some(member_path(label, xml::TEXT)),
    vec![text_node],
  ));

  // Children, a group per name, by name.
  let mut children: Vec<(&String, Vec<&RuntimeValue>)> = members
    .iter()
    .filter(|(key, _)| !key.starts_with('@') && !key.starts_with('#'))
    .map(|(key, value)| {
      let items = match value {
        RuntimeValue::Array(items) => items.iter().collect(),
        other => vec![other],
      };
      (key, items)
    })
    .collect();
  children.sort_by_key(|(key, _)| *key);
  for (child, items) in &children {
    let Some(first) = items.first() else { continue };
    let mut child_fragments = fragments.to_vec();
    child_fragments.push(rule_name(child, namespace_of(first)));
    let group = at.member_quoted(child);
    let group_label = member_path(label, child);
    if type_rule_reaches(ctx.category, &child_fragments) {
      nodes.push(Node::action("expect:not-empty", vec![group.resolve()]));
      let item = compile_xml_element(
        first,
        child,
        &child_fragments,
        &Cursor::Relative("~>".to_string()),
        &path::each_like_item(&group_label),
        ctx,
      );
      nodes.push(Node::action(
        "flow:for-each",
        vec![Node::splat(vec![group.resolve()]), item],
      ));
    } else {
      let count = Node::value(Literal::number(items.len() as u64));
      nodes.push(if ctx.closed {
        Node::action("expect:count", vec![group.resolve(), count])
      } else {
        Node::action(
          "expect:size",
          vec![group.resolve(), count, Node::value(Literal::null())],
        )
      });
      for (index, item) in items.iter().enumerate() {
        nodes.push(compile_xml_element(
          item,
          child,
          &child_fragments,
          &group.array_index(index),
          &path::array_index(&group_label, index),
          ctx,
        ));
      }
    }
  }

  // What the element may have that the pact does not name.
  let names: Vec<&String> = children.iter().map(|(name, _)| *name).collect();
  if ctx.closed {
    let mut only = vec![at.resolve()];
    only.extend(
      members
        .keys()
        .filter(|key| !key.starts_with('#'))
        .chain([&xml::TEXT.to_string(), &xml::NAMESPACE.to_string()])
        .map(|key| Node::value(Literal::string(key))),
    );
    nodes.push(Node::action("legacy:only-entries", only));
  } else {
    // A child no rule names and a type rule reaches: any name, when the rule reaches every child.
    let mut unnamed = fragments.to_vec();
    unnamed.push(String::new());
    if type_rule_reaches(ctx.category, &unnamed) {
      let mut only = vec![at.resolve()];
      only.extend(names.iter().map(|name| Node::value(Literal::string(*name))));
      nodes.push(Node::action("legacy:xml-only-children", only));
    } else {
      for forbidden in type_rule_names(ctx.category, fragments) {
        if !names.iter().any(|name| **name == forbidden) {
          nodes.push(Node::action(
            "expect:empty",
            vec![at.member_quoted(&forbidden).resolve()],
          ));
        }
      }
    }
  }

  Node::container(Some(label.to_string()), nodes)
}

/// A rule on an element (`pact_matching`'s `DoMatch<&Element>`). A name rule compares the element's
/// name, which is the pact's: the plan found the element by it. A size rule counts the element's
/// child elements, and `NotEmpty` wants a child or text. Anything else cannot match an element.
fn xml_element_rule(list: &RuleList, cascaded: bool, name: &str, at: &Cursor) -> Node {
  let children = || Node::action("legacy:xml-children", vec![at.resolve()]);
  let size = |min: Option<usize>, max: Option<usize>| {
    let bound = |n: Option<usize>| match n {
      Some(n) => Literal::number(n as u64),
      None => Literal::null(),
    };
    Node::action(
      "expect:size",
      vec![
        children(),
        Node::value(bound(Some(min.unwrap_or(0)))),
        Node::value(bound(max)),
      ],
    )
  };
  let nodes = list
    .rules
    .iter()
    .map(|rule| match rule {
      MatchingRule::Regex(pattern) => Node::action(
        "match:regex",
        vec![
          Node::value(Literal::string(name)),
          Node::value(Literal::string(anchored_pattern(pattern))),
        ],
      ),
      MatchingRule::MinType(min) if !cascaded => size(Some(*min), None),
      MatchingRule::MaxType(max) if !cascaded => size(None, Some(*max)),
      MatchingRule::MinMaxType(min, max) if !cascaded => size(Some(*min), Some(*max)),
      MatchingRule::Type
      | MatchingRule::Equality
      | MatchingRule::MinType(_)
      | MatchingRule::MaxType(_)
      | MatchingRule::MinMaxType(_, _) => Node::action("expect:object", vec![at.resolve()]),
      MatchingRule::NotEmpty => Node::action(
        "flow:or",
        vec![
          Node::action("expect:not-empty", vec![children()]),
          Node::action("expect:not-empty", vec![at.member_quoted(xml::TEXT).resolve()]),
        ],
      ),
      other => Node::action(
        "flow:error",
        vec![Node::value(Literal::string(format!(
          "Unable to match the element '{name}' using {}",
          other.name()
        )))],
      ),
    })
    .collect();
  combine(list, nodes)
}

/// Whether a type rule reaches `fragments`: today's `type_matcher_defined`, which asks every rule whose
/// path matches, the shallower ones included, not only the winner. An empty last fragment stands for a
/// name the pact does not have, which only a wildcard or a shallower rule reaches.
fn type_rule_reaches(category: Option<&MatchingRuleCategory>, fragments: &[String]) -> bool {
  let Some(category) = category else {
    return false;
  };
  category.rules.iter().any(|(rule_path, list)| {
    let tokens = rule_path.tokens();
    tokens.len() <= fragments.len()
      && tokens
        .iter()
        .zip(fragments)
        .all(|(token, fragment)| token_weight(token, fragment, false) > 0)
      && list.type_matcher_defined()
  })
}

/// The names of children a type rule names directly below `fragments`, as document keys (their local
/// names): a response may not have one the pact lacks.
fn type_rule_names(category: Option<&MatchingRuleCategory>, fragments: &[String]) -> Vec<String> {
  let Some(category) = category else {
    return Vec::new();
  };
  let mut names: Vec<String> = category
    .rules
    .iter()
    .filter(|(_, list)| list.type_matcher_defined())
    .filter_map(|(rule_path, _)| {
      let tokens = rule_path.tokens();
      let (last, prefix) = tokens.split_last()?;
      let PathToken::Field(name) = last else {
        return None;
      };
      (prefix.len() == fragments.len()
        && !name.starts_with('@')
        && !name.starts_with('#')
        && prefix
          .iter()
          .zip(fragments)
          .all(|(token, fragment)| token_weight(token, fragment, false) > 0))
      .then(|| name.rsplit(':').next().unwrap_or(name).to_string())
    })
    .collect();
  names.sort();
  names.dedup();
  names
}

/// Whether `list` accepts the empty string, which is what today's verifier compares a rule on text
/// against when an element has none. Answered by running the rule on it.
fn accepts_empty_text(list: &RuleList, cascaded: bool, example: &Literal) -> bool {
  let node = rule_list_node(
    list,
    cascaded,
    Node::value(Literal::string("")),
    example,
    Text::Yes,
  );
  let plan = model::Plan {
    grammar: model::GRAMMAR_VERSION,
    root: Node::container(None, vec![node]),
    variant: None,
  };
  let executed = model::execute(&plan, &CapturedValues::new());
  model::outcome(&executed).0 == Status::Matched
}

// --- multipart: parts by name, each by its own type (ADR 0040) ---

/// The part headers today's verifier does not require of the reply's part when the pact's has them.
const OPTIONAL_PART_HEADERS: [&str; 3] = ["content-type", "content-disposition", "content-transfer-encoding"];

/// A multipart body, as today's `match_mime_multipart` compares one: each of the pact's parts, in
/// order, against the reply's first part of its name, or the unnamed part at its position among the
/// unnamed parts. `legacy:mime-part` fails a missing part or one of the wrong kind; then its headers
/// and its body are compared. A part the pact does not have is allowed, in a request too.
fn compile_multipart_body(
  parts: &[LegacyPart],
  category: Option<&MatchingRuleCategory>,
  closed: bool,
  root_path: &str,
) -> Vec<Node> {
  let mut unnamed = 0;
  parts
    .iter()
    .map(|part| {
      let position = if part.name.is_empty() {
        unnamed += 1;
        unnamed - 1
      } else {
        0
      };
      let part_path = path::array_index(&member_path(root_path, &part.name), position);
      // A rule path names a part by its name, or by its position among all the parts.
      let key = if part.name.is_empty() {
        part.index.to_string()
      } else {
        part.name.clone()
      };
      let mut checks = compile_part_headers(part, category, &key, &part_path);
      checks.extend(compile_part_body(part, category, closed, &key, &part_path));
      let kind = if part.field { "field" } else { "file" };
      Node::container(
        Some(part_path.clone()),
        vec![Node::action(
          "flow:if",
          vec![
            Node::action(
              "legacy:mime-part",
              vec![
                Node::resolve(part_path.clone()),
                Node::value(Literal::string(kind)),
                Node::value(Literal::string(&part.name)),
              ],
            ),
            Node::action("flow:and", checks),
          ],
        )],
      )
    })
    .collect()
}

/// Each header the pact's part has, as today's `match_headers` compares it: by a rule at exactly
/// `$.<part>.<header>`, or by equality. `Content-Disposition` is compared only by a rule, and the reply's
/// part may lack it, `Content-Type` and `Content-Transfer-Encoding`.
fn compile_part_headers(
  part: &LegacyPart,
  category: Option<&MatchingRuleCategory>,
  key: &str,
  part_path: &str,
) -> Vec<Node> {
  let mut headers = part.headers.clone();
  headers.sort();
  headers
    .iter()
    .filter_map(|(name, value)| {
      let at = member_path(&member_path(part_path, multipart::HEADERS), name);
      let fragments = vec!["$".to_string(), key.to_string(), name.clone()];
      let example = Literal::string(value);
      let node = match best_rule(category, &fragments, true) {
        Some((list, false)) => rule_list_node(&list, false, Node::resolve(at.clone()), &example, Text::Yes),
        _ if name == "content-disposition" => return None,
        _ => Node::action(
          "match:equality",
          vec![Node::resolve(at.clone()), Node::value(example)],
        ),
      };
      Some(if OPTIONAL_PART_HEADERS.contains(&name.as_str()) {
        Node::action(
          "flow:if",
          vec![Node::action("check:exists", vec![Node::resolve(at)]), node],
        )
      } else {
        node
      })
    })
    .collect()
}

/// A part's body. A field of text is compared as text, by the rules at `$.<part>`, as today's
/// `match_field` compares one. A file, or a field with a type of its own, is compared as a body of
/// its type is, by the rules under `$.<part>` moved to its root, as today's `match_file_part`
/// compares one; a file's type is checked first, with the part's `Content-Type`, and a file of another
/// type is not compared further.
fn compile_part_body(
  part: &LegacyPart,
  category: Option<&MatchingRuleCategory>,
  closed: bool,
  key: &str,
  part_path: &str,
) -> Vec<Node> {
  let at = member_path(part_path, multipart::BODY);
  let text = part
    .content_type
    .as_deref()
    .is_none_or(|content_type| content_type.trim().to_ascii_lowercase().starts_with("text/"));
  if part.field && text {
    let fragments = vec!["$".to_string(), key.to_string()];
    let example = Literal::from_document(&part.body.document());
    return vec![match best_rule(category, &fragments, false) {
      Some((list, cascaded)) => rule_list_node(&list, cascaded, Node::resolve(at), &example, Text::Yes),
      None => Node::action("match:equality", vec![Node::resolve(at), Node::value(example)]),
    }];
  }
  let rules = rerooted(category, key);
  let nodes = compile_body_content(&part.body, rules.as_ref(), closed, &at);
  // A file of another type is not compared, as today: the type mismatch is the finding.
  match part
    .content_type
    .as_deref()
    .filter(|ct| !part.field && !ct.contains('*'))
  {
    Some(content_type) => vec![Node::action(
      "flow:if",
      vec![
        Node::action(
          "legacy:body-type",
          vec![
            Node::resolve(member_path(
              &member_path(part_path, multipart::HEADERS),
              "content-type",
            )),
            Node::value(Literal::string(content_type)),
          ],
        ),
        Node::action("flow:and", nodes),
      ],
    )],
    None => nodes,
  }
}

/// The rules under `$.<key>`, rooted at `$`: what today's `match_file_part` compares a part's body by.
/// Only a rule naming the part itself counts; a wildcard in its place does not, as today.
fn rerooted(category: Option<&MatchingRuleCategory>, key: &str) -> Option<MatchingRuleCategory> {
  let category = category?;
  let rules = category
    .rules
    .iter()
    .filter_map(|(rule_path, list)| {
      let tokens = rule_path.tokens();
      let names_part = matches!(tokens.first(), Some(PathToken::Root))
        && match tokens.get(1) {
          Some(PathToken::Field(name)) => name == key,
          Some(PathToken::Index(index)) => index.to_string() == key,
          _ => false,
        };
      names_part.then(|| {
        let mut path = DocPath::root();
        for token in &tokens[2..] {
          path.push(token.clone());
        }
        (path, list.clone())
      })
    })
    .collect();
  Some(MatchingRuleCategory {
    name: category.name.clone(),
    rules,
  })
}
