//! `upgrade/pact` (contract-file spec §8, protocol spec §8.4, plan task 5.5): a v1–v4 pact
//! converted into a Janus contract — matching rules become shapes, the single example becomes the
//! sole variant, and **every place the conversion lost something or chose between readings
//! produces a finding**.
//!
//! **Conversion is not required to be lossless, and pretending otherwise would be the failure mode
//! here** (spec §8.1). The shape language and v1–v4's matcher vocabulary are not in bijection:
//! there is no intersection operator for an `AND` of two unrelated rules, no way to name most
//! disjunctions, and v1–v4's own header and closed-request-body defaults are *comparisons* rather
//! than value sets. Each of those is reported rather than papered over, and an empty `findings`
//! list is a claim — a strong one: this conversion was exact.
//!
//! **Why this is not how a pact gets verified.** Plan task 5.4 verifies a pact where it stands,
//! through design 3.5's compiler, losing nothing. Upgrading is the *other* operation: it produces
//! a document the consumer's own suite can then grow — new variants, an `optional` where the pact
//! only ever showed one example — and it is the thing a `janus upgrade` user is asking for. A
//! verifier should never reach for it, and nothing in `protocol::verification` does.
//!
//! **Precedence is not reimplemented here.** Which declared rule governs a position, and whether
//! it cascaded, is [`plan::legacy_winning_rule`]'s answer — the same one plans get. A converter
//! that scored rules its own way could produce a contract that verifies differently from the pact
//! it came from, which is exactly what plan-grammar spec §4.4's two-path agreement rule forbids.
//!
//! Permanently v1–v4-shaped, and deliberately a sibling of [`crate::legacy_pact`] rather than a
//! module under `contract/`: this half of the kernel is the compatibility layer, and
//! `kernel-boundary-review.md`'s findings 3 and 6 asked that the module tree say so.

use crate::common::{Requirement, State, Transport};
use crate::contract::{
  self, Contract, ContractError, Interaction, Party, Problem, RecordedSelection, RecordedVariant,
  ResolvedState, ShapePart,
};
use crate::legacy_pact::{self, LegacyInteraction};
use crate::plan;
use crate::variant::{self, SamplingPolicy};
use pact_models::generators::Generators;
use pact_models::http_parts::HttpPart;
use pact_models::matchingrules::{MatchingRule, MatchingRules, RuleList, RuleLogic};
use pact_models::v4::http_parts::{HttpRequest, HttpResponse};
use serde::Serialize;
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;

/// What a conversion produced: the contract, and everything the author should know about it.
pub struct Upgraded {
  pub contract: Contract,
  pub findings: Vec<Finding>,
}

/// One spot in the source pact where the conversion lost something or made a judgement call
/// (contract-file spec §8.4, `schemas/v1/upgrade-findings.schema.json`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Finding {
  /// Open vocabulary; an unknown code is displayed, never dispatched on.
  pub code: String,
  /// `lossy`, `judgement` or `note` — three things that would otherwise be read as one severity.
  pub kind: String,
  /// RFC 6901 pointer into the **source pact**: the file the author is looking at.
  pub path: String,
  /// Prose addressed to that author — what was decided and what to check.
  pub message: String,
  /// Where the construct ended up in the converted contract, when there is such a place.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub target: Option<String>,
}

impl Finding {
  /// A finding located in the source pact only — an interaction-level decision, where there is no
  /// single place in the contract to point at.
  fn new(code: &str, kind: &str, path: String, message: String) -> Finding {
    Finding {
      code: code.to_string(),
      kind: kind.to_string(),
      path,
      message,
      target: None,
    }
  }

  /// A finding located in both documents.
  fn at(code: &str, kind: &str, at: &Where, message: String) -> Finding {
    Finding {
      code: code.to_string(),
      kind: kind.to_string(),
      path: at.source.clone(),
      message,
      target: Some(at.target.clone()),
    }
  }
}

/// Convert a v1–v4 pact document to a Janus contract.
///
/// `Err` only for a document that is not a readable pact at all — everything the conversion cannot
/// carry across is a *finding*, not an error, because a partial conversion the author can see is
/// worth more than a refusal they cannot act on.
pub fn pact(source: &str, doc: &Value) -> Result<Upgraded, ContractError> {
  let pact = legacy_pact::read(source, doc)?;
  let mut conversion = Conversion::default();
  let http = legacy_pact::http_interactions(pact.as_ref());

  // Every interaction the pact declared, HTTP or not — a message interaction cannot be converted
  // by a design that has no message support yet, and vanishing silently is exactly the outcome
  // §8.1's honesty rule forbids.
  let declared = pact.interactions().len();
  if declared > http.len() {
    conversion.finding(Finding::new(
      "interaction-dropped",
      "lossy",
      "/interactions".to_string(),
      format!(
        "{} of {declared} interactions are not HTTP request/response and were not converted; \
         message interactions have no contract form yet",
        declared - http.len()
      ),
    ));
  }

  let mut interactions = Vec::with_capacity(http.len());
  let mut seen: Vec<(String, Option<Vec<State>>)> = Vec::new();
  let mut problems = Vec::new();
  for (index, interaction) in http.iter().enumerate() {
    match conversion.interaction(index, interaction, &mut seen) {
      Ok(converted) => interactions.push(converted),
      Err(mut found) => problems.append(&mut found),
    }
  }
  // Only reachable if this module emitted a shape the engine's own reader refuses — a bug here,
  // never something the pact did. Reported as an error rather than smuggled out as a finding: a
  // contract that cannot be read back is worse than a conversion that admits it failed.
  if !problems.is_empty() {
    return Err(ContractError::Invalid { problems });
  }

  Ok(Upgraded {
    contract: Contract {
      format: contract::FORMAT.to_string(),
      schema: None,
      consumer: Party {
        name: pact.consumer().name,
      },
      provider: Party {
        name: pact.provider().name,
      },
      interactions,
      metadata: None,
    },
    findings: conversion.findings,
  })
}

#[derive(Default)]
struct Conversion {
  findings: Vec<Finding>,
}

impl Conversion {
  fn finding(&mut self, finding: Finding) {
    self.findings.push(finding);
  }

  fn interaction(
    &mut self,
    index: usize,
    interaction: &LegacyInteraction,
    seen: &mut Vec<(String, Option<Vec<State>>)>,
  ) -> Result<Interaction, Vec<Problem>> {
    let at = format!("/interactions/{index}");
    let states = self.states(interaction, &at);

    // Contract-file spec §4.2: an interaction is identified by description *and* states, and two
    // that collide would make the contract unreadable back. Disambiguated rather than dropped,
    // and reported either way.
    let mut description = interaction.description.clone();
    if seen.iter().any(|(d, s)| d == &description && s == &states) {
      let disambiguated = format!("{description} ({})", index + 1);
      self.finding(Finding::new(
        "duplicate-description",
        "lossy",
        format!("{at}/description"),
        format!(
          "another interaction already has the description '{description}' and the same states; \
           this one was renamed to '{disambiguated}' so both survive"
        ),
      ));
      description = disambiguated;
    }
    seen.push((description.clone(), states.clone()));

    let mut requires = Vec::new();
    let mut parts: BTreeMap<String, ShapePart> = BTreeMap::new();
    parts.insert(
      "request".to_string(),
      self.request_shapes(&interaction.request, &at),
    );
    parts.insert(
      "response".to_string(),
      self.response_shapes(&interaction.response, &at, &mut requires),
    );

    let mut values = legacy_pact::request_parts(&interaction.request);
    values.extend(legacy_pact::response_parts(&interaction.response));

    let selection = self.selection(&description, &states, &parts, &at)?;

    Ok(Interaction {
      description,
      transport: Some(Transport {
        kind: "http".to_string(),
        mode: Some("passive".to_string()),
      }),
      states,
      parts,
      requires: (!requires.is_empty()).then_some(requires),
      selection: RecordedSelection {
        variants: vec![RecordedVariant {
          id: "base".to_string(),
          origin: variant::Origin::Base,
          assignment: Vec::new(),
          states: self.resolved_states(interaction),
          parts: values,
        }],
        report: selection,
      },
    })
  }

  /// The interaction's states. v3's list carries parameters; v1/v2's bare string arrives here as a
  /// one-element list with none, because `as_v4_http` already normalised it.
  fn states(&mut self, interaction: &LegacyInteraction, at: &str) -> Option<Vec<State>> {
    if interaction.provider_states.is_empty() {
      return None;
    }
    let mut typed = false;
    let states = interaction
      .provider_states
      .iter()
      .map(|state| {
        typed |= !state.params.is_empty();
        State {
          name: state.name.clone(),
          params: (!state.params.is_empty())
            .then(|| state.params.iter().map(|(k, v)| (k.clone(), v.clone())).collect()),
          // A `whenVariant` binding is something an author adds *after* upgrading, once the
          // consumer's suite has variants to bind to. Inventing one here would be a guess about
          // intent, not a conversion.
          variant_params: None,
        }
      })
      .collect();
    if typed {
      self.finding(Finding::new(
        "state-params-untyped",
        "note",
        format!("{at}/providerStates"),
        "state parameters were carried across as they stand — the contract format types them, the \
         pact did not, so check that each value has the type the provider expects"
          .to_string(),
      ));
    }
    Some(states)
  }

  fn resolved_states(&self, interaction: &LegacyInteraction) -> Option<Vec<ResolvedState>> {
    if interaction.provider_states.is_empty() {
      return None;
    }
    Some(
      interaction
        .provider_states
        .iter()
        .map(|state| ResolvedState {
          name: state.name.clone(),
          params: (!state.params.is_empty())
            .then(|| state.params.iter().map(|(k, v)| (k.clone(), v.clone())).collect()),
        })
        .collect(),
    )
  }

  /// The selection report a converted interaction carries (contract-file spec §8.3): computed from
  /// the shapes just produced under a `base-only` policy, never hand-built. The invariant that
  /// holds for every conversion is `selected: 1`; `space.size` is whatever the shapes actually
  /// say, and a converted `min` on an array makes it larger than one. A contract that is honestly
  /// under-covered says so in its own report.
  fn selection(
    &mut self,
    description: &str,
    states: &Option<Vec<State>>,
    parts: &BTreeMap<String, ShapePart>,
    at: &str,
  ) -> Result<BTreeMap<String, Value>, Vec<Problem>> {
    let mut document = json!({ "description": description, "parts": parts });
    if let Some(states) = states {
      document["states"] = json!(states);
    }
    // Parsing the converted shapes back is the conversion's own proof that it produced a
    // readable document — and it is where the variant space comes from, so the check is free.
    let space = match crate::interaction_spec::parse(&document) {
      Ok(spec) => plan::variant_space(&spec),
      Err(err) => {
        return Err(
          err
            .problems
            .into_iter()
            .map(|problem| Problem {
              pointer: format!("{at}{}", problem.pointer),
              message: problem.message,
            })
            .collect(),
        );
      }
    };
    let policy = SamplingPolicy {
      strategy: "base-only".to_string(),
      boundaries: false,
      ..SamplingPolicy::default()
    };
    let report = match variant::select(&space, &policy) {
      Ok(selected) => selected.report.to_json(),
      // `base-only` has no budget to exceed and no policy to be invalid, so this is unreachable;
      // an empty report is still a readable contract rather than a panic if it ever is not.
      Err(_) => Value::Object(Map::new()),
    };
    Ok(match report {
      Value::Object(map) => map.into_iter().collect(),
      _ => BTreeMap::new(),
    })
  }

  // -------------------------------------------------------------------------------------------
  // Slots
  // -------------------------------------------------------------------------------------------

  fn request_shapes(&mut self, request: &HttpRequest, at: &str) -> ShapePart {
    let rules = &request.matching_rules;
    let mut part = ShapePart::new();
    part.insert(
      "method".to_string(),
      self.scalar_slot(
        rules,
        "method",
        &Value::String(request.method.clone()),
        &Where::slot(at, "request", "method"),
      ),
    );
    part.insert(
      "path".to_string(),
      self.scalar_slot(
        rules,
        "path",
        &Value::String(request.path.clone()),
        &Where::slot(at, "request", "path"),
      ),
    );
    if let Some(query) = &request.query
      && !query.is_empty()
    {
      let query: BTreeMap<String, Vec<String>> = query
        .iter()
        .map(|(name, values)| {
          (
            name.clone(),
            values.iter().map(|v| v.clone().unwrap_or_default()).collect(),
          )
        })
        .collect();
      part.insert(
        "query".to_string(),
        self.multi_map_slot(
          rules,
          "query",
          &query,
          false,
          &Where::slot(at, "request", "query"),
        ),
      );
    }
    if let Some(headers) = &request.headers
      && !headers.is_empty()
    {
      part.insert(
        "headers".to_string(),
        self.headers_slot(rules, headers, &Where::slot(at, "request", "headers")),
      );
    }
    let declared = request.lookup_content_type();
    if let Some(body) = self.body_slot(
      rules,
      &request.body,
      "request",
      declared,
      &Where::slot(at, "request", "body"),
    ) {
      part.insert("body".to_string(), body);
    }
    self.generators(&request.generators, &format!("{at}/request/generators"));
    part
  }

  fn response_shapes(
    &mut self,
    response: &HttpResponse,
    at: &str,
    requires: &mut Vec<Requirement>,
  ) -> ShapePart {
    let rules = &response.matching_rules;
    let mut part = ShapePart::new();
    part.insert("status".to_string(), self.status_slot(response, at, requires));
    if let Some(headers) = &response.headers
      && !headers.is_empty()
    {
      part.insert(
        "headers".to_string(),
        self.headers_slot(rules, headers, &Where::slot(at, "response", "headers")),
      );
    }
    let declared = response.lookup_content_type();
    if let Some(body) = self.body_slot(
      rules,
      &response.body,
      "response",
      declared,
      &Where::slot(at, "response", "body"),
    ) {
      part.insert("body".to_string(), body);
    }
    self.generators(&response.generators, &format!("{at}/response/generators"));
    part
  }

  /// `status`, with the one component operator contract-file spec §8.2's table names: a v4
  /// `statusCode` matcher becomes `http:status-class`, which the kernel does not understand and
  /// must not pretend to (shape spec §3.5). The interaction records the requirement, so a verifier
  /// without that component says `component-unavailable` naming it rather than quietly matching
  /// less.
  fn status_slot(&mut self, response: &HttpResponse, at: &str, requires: &mut Vec<Requirement>) -> Value {
    let at = Where::slot(at, "response", "status");
    let example = Value::from(response.status);
    let Some((list, _)) =
      plan::legacy_winning_rule(&response.matching_rules, "status", &["$".to_string()], false)
    else {
      self.frozen(&at, 1);
      return json!({ "shape": "equality", "example": example });
    };
    if let Some(MatchingRule::StatusCode(status)) = list.rules.first() {
      requires.push(Requirement {
        component: "http".to_string(),
        min_version: None,
      });
      self.finding(Finding::at(
        "rule-narrowed",
        "note",
        &at,
        "the status-class matcher became the component operator 'http:status-class'; the \
         interaction now requires the 'http' component, which a verifier reports by name if it is \
         not loaded"
          .to_string(),
      ));
      return json!({
        "shape": "http:status-class",
        "class": status_class(status),
        "example": example,
      });
    }
    self.shape_for(Some(list), &example, &at)
  }

  fn scalar_slot(&mut self, rules: &MatchingRules, category: &str, example: &Value, at: &Where) -> Value {
    let list = plan::legacy_winning_rule(rules, category, &["$".to_string()], false);
    self.shape_for(list.map(|(list, _)| list), example, at)
  }

  /// Headers, in the document form a transport actually produces: `{name: [value, …]}` — so an
  /// `object` of `array`s, never a flat string map (component-interfaces spec §4).
  ///
  /// A header with no rule is where this conversion is least exact, and it says so. v1–v4 compare
  /// a header value with their own defaulted comparison — MIME parameters as a set, whitespace
  /// around commas ignored (plan-grammar spec §4.4's `match:header-value`) — and the shape
  /// language has no operator for that: `header:parse` is its eventual answer and no component
  /// contributes it yet. `equality` is strictly narrower, which is a judgement worth a finding
  /// rather than a silent tightening.
  fn headers_slot(
    &mut self,
    rules: &MatchingRules,
    headers: &std::collections::HashMap<String, Vec<String>>,
    at: &Where,
  ) -> Value {
    let headers: BTreeMap<String, Vec<String>> = headers
      .iter()
      .map(|(name, values)| (name.to_ascii_lowercase(), values.clone()))
      .collect();
    let before = self.findings.len();
    let slot = self.multi_map_slot(rules, "header", &headers, true, at);
    if self.findings[before..]
      .iter()
      .any(|f| f.code == "example-frozen-as-equality")
    {
      self.finding(Finding::at(
        "rule-narrowed",
        "judgement",
        at,
        "header values with no matching rule became exact `equality` constraints; v1–v4 compared \
         them more loosely (MIME parameters as a set, whitespace around commas ignored), and the \
         shape language has no operator for that comparison yet"
          .to_string(),
      ));
    }
    slot
  }

  /// A `{name: [value, …]}` slot — headers and query parameters have the same document shape and
  /// the same per-value rule lookup, differing only in whether names are compared case-insensitively.
  fn multi_map_slot(
    &mut self,
    rules: &MatchingRules,
    category: &str,
    entries: &BTreeMap<String, Vec<String>>,
    case_insensitive: bool,
    at: &Where,
  ) -> Value {
    let mut members = Map::new();
    for (name, values) in entries {
      let escaped = escape_pointer(name);
      let member = at.push(&format!("/{escaped}"), &format!("/members/{escaped}"));
      let mut shapes = Vec::with_capacity(values.len());
      for (index, value) in values.iter().enumerate() {
        let fragments = vec!["$".to_string(), name.clone(), index.to_string()];
        let list = plan::legacy_winning_rule(rules, category, &fragments, case_insensitive);
        let example = Value::String(value.clone());
        let entry = member.push(&format!("/{index}"), &format!("/entries/{index}"));
        shapes.push(self.shape_for(list.map(|(list, _)| list), &example, &entry));
      }
      members.insert(name.clone(), json!({ "shape": "array", "entries": shapes }));
    }
    json!({ "shape": "object", "members": members })
  }

  /// The body, walked the way design 3.5 walks it — same rule fragments, same cascading — with a
  /// shape at each position instead of a plan node.
  fn body_slot(
    &mut self,
    rules: &MatchingRules,
    body: &pact_models::bodies::OptionalBody,
    part: &str,
    declared: Option<String>,
    at: &Where,
  ) -> Option<Value> {
    use pact_models::bodies::OptionalBody;
    let json = match body {
      // The interaction never mentioned a body: there is nothing to say about it, and an invented
      // shape would assert something the pact did not.
      OptionalBody::Missing => return None,
      // Declared and empty: v1–v4 require the actual body be absent too, which is `forbidden`.
      OptionalBody::Null | OptionalBody::Empty => {
        return Some(json!({ "shape": "forbidden" }));
      }
      OptionalBody::Present(bytes, content_type, _) => {
        let effective = content_type
          .as_ref()
          .map(ToString::to_string)
          .or_else(|| declared.clone());
        if let Some(content_type) = effective.as_deref().filter(|ct| !ct.contains("json")) {
          self.finding(Finding::at(
            "body-not-parsed",
            "lossy",
            at,
            format!(
              "the body is '{content_type}', which this engine has no content component for: the \
               example is carried across as bytes, but nothing is asserted about it"
            ),
          ));
          return None;
        }
        // §8.4's `content-type-inferred` is about a pact that declared *nothing* — a body slot
        // with no type and no `Content-Type` header. A pact that put the type in its headers, as
        // most do, declared it perfectly well and gets no finding.
        if content_type.is_none() && declared.is_none() {
          self.finding(Finding::at(
            "content-type-inferred",
            "judgement",
            at,
            "the pact declared no content type for this body, in the body or in a header, and it \
             was read as JSON"
              .to_string(),
          ));
        }
        match serde_json::from_slice::<Value>(bytes) {
          Ok(json) => json,
          Err(err) => {
            self.finding(Finding::at(
              "body-not-parsed",
              "lossy",
              at,
              format!("the body did not parse as JSON ({err}); nothing is asserted about it"),
            ));
            return None;
          }
        }
      }
    };

    // v1–v4 request bodies are closed to unnamed members and the shape language refuses closed
    // objects outright (ADR 0007 commitment 4, plan-grammar spec §5.3). The contract therefore
    // admits more than the pact did, at exactly one position, and says so.
    if part == "request" && json.is_object() {
      self.finding(Finding::at(
        "request-body-opened",
        "lossy",
        at,
        "v1–v4 request bodies reject members the pact did not name; shapes are must-ignore by \
         design (ADR 0007), so the converted contract admits extra members. Add `forbidden` \
         members if any of them must stay out"
          .to_string(),
      ));
    }

    let ctx = BodyCtx {
      fragments: vec!["$".to_string()],
      at: at.clone(),
    };
    Some(self.body_shape(rules, &json, &ctx))
  }

  fn body_shape(&mut self, rules: &MatchingRules, value: &Value, ctx: &BodyCtx) -> Value {
    let list = plan::legacy_winning_rule(rules, "body", &ctx.fragments, false);
    match value {
      Value::Object(members) => self.object_shape(rules, members, list, ctx),
      Value::Array(items) => self.array_shape(rules, items, list, ctx),
      scalar => self.shape_for(list.map(|(list, _)| list), scalar, &ctx.at),
    }
  }

  /// An object: the map-entry rules (`values`, `eachKey`, `eachValue`) describe the map itself and
  /// become `each-entry`; anything else declared on an object's own path does not replace the
  /// structural walk, exactly as design 3.5 found against the specification corpus.
  fn object_shape(
    &mut self,
    rules: &MatchingRules,
    members: &Map<String, Value>,
    list: Option<(RuleList, bool)>,
    ctx: &BodyCtx,
  ) -> Value {
    if let Some((list, false)) = &list
      && let Some(rule) = list.rules.iter().find(|rule| is_map_entry_rule(rule))
    {
      let sample = members.values().next().cloned().unwrap_or(Value::Null);
      let values = self.body_shape(rules, &sample, &ctx.entry_value());
      let mut shape = json!({ "shape": "each-entry", "values": values, "min": 0 });
      if let MatchingRule::EachKey(_) = rule {
        shape["keys"] = json!({ "shape": "string", "example": members.keys().next() });
      }
      return shape;
    }
    let mut shapes = Map::new();
    for (name, value) in members {
      let member = ctx.member(name);
      shapes.insert(name.clone(), self.body_shape(rules, value, &member));
    }
    json!({ "shape": "object", "members": shapes })
  }

  /// An array: a rule reaching this path (exactly or cascaded) makes it an `each-like` over a
  /// template compiled from the first element — the same switch design 3.5 makes between
  /// `expect:count` and `for-each`. Without one, the pact asserted an exact list, which is `array`.
  fn array_shape(
    &mut self,
    rules: &MatchingRules,
    items: &[Value],
    list: Option<(RuleList, bool)>,
    ctx: &BodyCtx,
  ) -> Value {
    let Some((list, _)) = list else {
      let entries: Vec<Value> = items
        .iter()
        .enumerate()
        .map(|(index, item)| self.body_shape(rules, item, &ctx.index(index)))
        .collect();
      return json!({ "shape": "array", "entries": entries });
    };
    if let Some(MatchingRule::ArrayContains(_)) = list.rules.first() {
      let entries: Vec<Value> = items
        .iter()
        .enumerate()
        .map(|(index, item)| self.body_shape(rules, item, &ctx.index(index)))
        .collect();
      return json!({ "shape": "contains", "entries": entries });
    }
    let (min, max) = cardinality(&list);
    let template = items.first().cloned().unwrap_or(Value::Null);
    let items_shape = self.body_shape(rules, &template, &ctx.template_item());
    let mut shape = json!({ "shape": "each-like", "items": items_shape, "min": min });
    if let Some(max) = max {
      shape["max"] = json!(max);
    }
    shape
  }

  // -------------------------------------------------------------------------------------------
  // One position's rule -> one shape
  // -------------------------------------------------------------------------------------------

  /// The heart of §8.2's table. `None` is not a gap: "no rule at this position" is what v1–v4
  /// already mean by an example, and the conversion writes that down as `equality` rather than
  /// changing it — with a `note`, because an author reading the contract should know which
  /// constraints they chose and which the format chose for them.
  fn shape_for(&mut self, list: Option<RuleList>, example: &Value, at: &Where) -> Value {
    let Some(list) = list else {
      self.frozen(at, 1);
      return json!({ "shape": "equality", "example": example });
    };
    let rule = match list.rules.len() {
      0 => {
        self.frozen(at, 1);
        return json!({ "shape": "equality", "example": example });
      }
      1 => &list.rules[0],
      _ => {
        let first = &list.rules[0];
        if list.rules.iter().all(|rule| rule == first) {
          self.finding(Finding::at(
            "rule-narrowed",
            "note",
            at,
            "several identical matching rules were declared here; one survived".to_string(),
          ));
        } else if list.rule_logic == RuleLogic::Or {
          // A disjunction denotes the union of what each alternative admits. The shape language
          // expresses that only where the alternatives are value sets it can name, and a general
          // one is not.
          self.finding(Finding::at(
            "rule-combination-or",
            "lossy",
            at,
            format!(
              "an OR of {} matching rules has no shape form; the first ('{}') survived and the \
               others were dropped",
              list.rules.len(),
              first.name()
            ),
          ));
        } else {
          // The shape language has no intersection operator (spec §8.2).
          self.finding(Finding::at(
            "rule-unmapped",
            "lossy",
            at,
            format!(
              "an AND of {} matching rules has no shape form unless one subsumes the others; the \
               first ('{}') survived",
              list.rules.len(),
              first.name()
            ),
          ));
        }
        first
      }
    };
    match map_rule(rule, example) {
      Some(shape) => shape,
      None => {
        self.finding(Finding::at(
          "rule-unmapped",
          "lossy",
          at,
          format!(
            "the matching rule '{}' has no shape equivalent; the example survived as an exact \
             value instead",
            rule.name()
          ),
        ));
        json!({ "shape": "equality", "example": example })
      }
    }
  }

  /// One `example-frozen-as-equality` note per *position group* rather than per position: a
  /// typical pact freezes dozens, and a finding list nobody can read is a finding list nobody
  /// reads. Consecutive freezes under one slot collapse into one entry that counts them.
  fn frozen(&mut self, at: &Where, count: usize) {
    // The slot, not the position: `/interactions/0/response/body/items/0/sku` collapses onto
    // `/interactions/0/response/body`.
    let slot = slot_of(&at.source);
    if let Some(existing) = self
      .findings
      .iter_mut()
      .find(|f| f.code == "example-frozen-as-equality" && f.path == slot)
    {
      let n = existing
        .message
        .split_whitespace()
        .next()
        .and_then(|n| n.parse::<usize>().ok())
        .unwrap_or(1)
        + count;
      existing.message = frozen_message(n);
      return;
    }
    self.finding(Finding {
      code: "example-frozen-as-equality".to_string(),
      kind: "note".to_string(),
      path: slot,
      message: frozen_message(count),
      target: Some(slot_of(&at.target)),
    });
  }

  /// v1–v4 generators produce values; the shape language's `generator` member names a *component*
  /// that does, and no component in this prototype implements the v1–v4 generator vocabulary.
  /// Dropping one is lossy but never dangerous: it weakens what a contract can *produce*, never
  /// what it *accepts*.
  fn generators(&mut self, generators: &Generators, pointer: &str) {
    if generators.is_empty() {
      return;
    }
    let count: usize = generators.categories.values().map(|c| c.len()).sum();
    self.finding(Finding::new(
      "generator-dropped",
      "lossy",
      pointer.to_string(),
      format!(
        "{count} generator(s) were dropped: the contract format names a generator *component*, and \
         none implements the v1–v4 generator vocabulary yet. What the contract accepts is unchanged"
      ),
    ));
  }
}

/// A position's pointer cut back to the slot that contains it: five segments
/// (`/interactions/{i}/{part}/{slot}`) in the pact, six (`/interactions/{i}/parts/{part}/{slot}`)
/// in the contract — both are "the first four or five non-empty segments", so one rule serves.
fn slot_of(pointer: &str) -> String {
  let segments: Vec<&str> = pointer.split('/').skip(1).collect();
  let keep = if segments.get(2) == Some(&"parts") { 5 } else { 4 };
  format!(
    "/{}",
    segments.into_iter().take(keep).collect::<Vec<_>>().join("/")
  )
}

/// The status class as the shape language names it: lower-case words, never a Rust enum's
/// `Debug` rendering.
fn status_class(status: &pact_models::HttpStatus) -> String {
  use pact_models::HttpStatus;
  match status {
    HttpStatus::Information => "information".to_string(),
    HttpStatus::Success => "success".to_string(),
    HttpStatus::Redirect => "redirect".to_string(),
    HttpStatus::ClientError => "client-error".to_string(),
    HttpStatus::ServerError => "server-error".to_string(),
    HttpStatus::NonError => "non-error".to_string(),
    HttpStatus::Error => "error".to_string(),
    HttpStatus::StatusCodes(codes) => codes.iter().map(u16::to_string).collect::<Vec<_>>().join(","),
  }
}

fn frozen_message(count: usize) -> String {
  format!(
    "{count} position(s) here had no matching rule, so their examples became `equality` \
     constraints — which is what v1–v4 already meant by them. Loosen any that were only ever \
     illustrative"
  )
}

/// One position, in both documents a finding names: where it is in the **source pact** (the file
/// the author is looking at) and where it ended up in the **converted contract**. Carrying both
/// costs two string pushes per position and is what makes a finding navigable instead of merely
/// true (contract-file spec §8.4's `path` and `target`).
#[derive(Clone)]
struct Where {
  source: String,
  target: String,
}

impl Where {
  fn slot(at: &str, part: &str, slot: &str) -> Where {
    Where {
      source: format!("{at}/{part}/{slot}"),
      target: format!("{at}/parts/{part}/{slot}"),
    }
  }

  fn push(&self, source: &str, target: &str) -> Where {
    Where {
      source: format!("{}{source}", self.source),
      target: format!("{}{target}", self.target),
    }
  }
}

/// Where a body walk is, in the three coordinate systems it needs: the rule-lookup fragments
/// design 3.5 scores against, and the two pointers a finding reports.
struct BodyCtx {
  fragments: Vec<String>,
  at: Where,
}

impl BodyCtx {
  fn member(&self, name: &str) -> BodyCtx {
    let mut fragments = self.fragments.clone();
    fragments.push(name.to_string());
    let escaped = escape_pointer(name);
    BodyCtx {
      fragments,
      at: self
        .at
        .push(&format!("/{escaped}"), &format!("/members/{escaped}")),
    }
  }

  fn index(&self, index: usize) -> BodyCtx {
    let mut fragments = self.fragments.clone();
    fragments.push(index.to_string());
    BodyCtx {
      fragments,
      at: self.at.push(&format!("/{index}"), &format!("/entries/{index}")),
    }
  }

  /// The template element inside a rule-driven array: the synthetic `"0"` fragment design 3.5 uses
  /// for the same purpose — it scores identically to any real index against a `[*]` rule token.
  /// In the contract it is the `each-like`'s single `items` shape, not an entry.
  fn template_item(&self) -> BodyCtx {
    let mut fragments = self.fragments.clone();
    fragments.push("0".to_string());
    BodyCtx {
      fragments,
      at: self.at.push("/0", "/items"),
    }
  }

  fn entry_value(&self) -> BodyCtx {
    let mut fragments = self.fragments.clone();
    fragments.push("*".to_string());
    BodyCtx {
      fragments,
      at: self.at.push("/*", "/values"),
    }
  }
}

fn escape_pointer(segment: &str) -> String {
  segment.replace('~', "~0").replace('/', "~1")
}

fn is_map_entry_rule(rule: &MatchingRule) -> bool {
  matches!(
    rule,
    MatchingRule::Values | MatchingRule::EachKey(_) | MatchingRule::EachValue(_)
  )
}

/// `min`/`max`/`minmax` on a collection become `each-like`'s own cardinality (spec §8.2). A bare
/// `type` on an array says "every element is like the template" with no bound, which is
/// `each-like`'s default of one — the same default `eachLike` has always had.
fn cardinality(list: &RuleList) -> (u64, Option<u64>) {
  for rule in &list.rules {
    match rule {
      MatchingRule::MinType(min) => return (*min as u64, None),
      MatchingRule::MaxType(max) => return (0, Some(*max as u64)),
      MatchingRule::MinMaxType(min, max) => return (*min as u64, Some(*max as u64)),
      _ => {}
    }
  }
  (1, None)
}

/// Contract-file spec §8.2's table, row by row. `None` means no shape equivalent — the caller
/// turns that into a `rule-unmapped` finding, never a silent guess.
fn map_rule(rule: &MatchingRule, example: &Value) -> Option<Value> {
  let shape = match rule {
    MatchingRule::Equality => json!({ "shape": "equality", "example": example }),
    MatchingRule::Type
    | MatchingRule::MinType(_)
    | MatchingRule::MaxType(_)
    | MatchingRule::MinMaxType(_, _) => {
      json!({ "shape": "type", "example": example })
    }
    MatchingRule::Regex(pattern) => json!({ "shape": "regex", "pattern": pattern, "example": example }),
    MatchingRule::Include(substring) => {
      json!({ "shape": "include", "substring": substring, "example": example })
    }
    MatchingRule::Number => json!({ "shape": "number", "example": example }),
    MatchingRule::Integer => json!({ "shape": "integer", "example": example }),
    MatchingRule::Decimal => json!({ "shape": "decimal", "example": example }),
    MatchingRule::Boolean => json!({ "shape": "boolean", "example": example }),
    MatchingRule::Null => json!({ "shape": "null", "example": example }),
    MatchingRule::NotEmpty => json!({ "shape": "not-empty", "example": example }),
    MatchingRule::Semver => json!({ "shape": "semver", "example": example }),
    MatchingRule::ContentType(content_type) => {
      json!({ "shape": "content-type", "content-type": content_type, "example": example })
    }
    // Format strings carry across character for character: the shape language's pattern
    // vocabulary is the same `DateTimeFormatter` subset v3/v4 already use (shape spec §4.2).
    MatchingRule::Timestamp(format) => temporal("datetime", format, example),
    MatchingRule::Date(format) => temporal("date", format, example),
    MatchingRule::Time(format) => temporal("time", format, example),
    _ => return None,
  };
  Some(shape)
}

fn temporal(operator: &str, format: &str, example: &Value) -> Value {
  let mut shape = json!({ "shape": operator, "example": example });
  if !format.is_empty() {
    shape["format"] = json!(format);
  }
  shape
}
