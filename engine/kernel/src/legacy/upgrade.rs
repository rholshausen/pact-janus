//! `upgrade/pact` (contract-file spec §8, protocol spec §8.4, plan task 5.5): a v1–v4 pact
//! converted into a Janus contract — matching rules become shapes, the single example becomes the
//! sole variant, and **every place the conversion lost something or chose between readings
//! produces a finding**.
//!
//! **Conversion is not required to be lossless, and pretending otherwise would be the failure mode
//! here** (spec §8.1). The shape language and v1–v4's matcher vocabulary are not in bijection:
//! there is no intersection operator for an `AND` of two unrelated rules, no way to name most
//! disjunctions, v1–v4's own header default is a *comparison* rather than a value set, and their
//! closed request query and body are closures a shape refuses to make (ADR 0007). Each of those is reported rather than papered over, and an empty `findings`
//! list is a claim — a strong one: this conversion was exact.
//!
//! **Why this is not how a pact gets verified.** Plan task 5.4 verifies a pact where it stands,
//! through design 3.5's compiler, losing nothing. Upgrading is the *other* operation: it produces
//! a document the consumer's own suite can then grow — new variants, an `optional` where the pact
//! only ever showed one example — and it is the thing a `janus upgrade` user is asking for. A
//! verifier should never reach for it, and nothing in `protocol::verification` does.
//!
//! **Precedence is not reimplemented here.** Which declared rule governs a position, and whether
//! it cascaded, is [`winning_rule`]'s answer — the same one plans get. A converter
//! that scored rules its own way could produce a contract that verifies differently from the pact
//! it came from, which is exactly what plan-grammar spec §4.4's two-path agreement rule forbids.
//!
//! Permanently v1–v4-shaped, and so inside the v1–v4 boundary ([`crate::legacy`]) rather than
//! under `contract/`: this half of the kernel is the compatibility layer, and
//! `kernel-boundary-review.md`'s findings 3 and 6 asked that the module tree say so (S1.11).

use super::compile::{
  LegacyBody, LegacyPart, OPTIONAL_PART_HEADERS, accepts_empty_text, anchored_pattern, best_rule,
  definition_rules, each_value_category, form_parameters, is_whole_equality, namespace_of, only_element,
  rerooted, rules_for, type_rule_names, type_rule_reaches,
};
use super::multipart as legacy_multipart;
use super::pact::{self as legacy_pact, BodyKind, LegacyInteraction, Unsupported};
use super::xml::{self as legacy_xml, rule_name};
use crate::common::{ContentTypes, Requirement, State, Transport};
use crate::component::PactGenerators;
use crate::component::{Parts, SlotValue};
use crate::contract::{
  self, Contract, ContractError, Interaction, Party, Problem, RecordedSelection, RecordedVariant,
  ResolvedState, ShapePart,
};
use crate::plan::{self, Literal, RuntimeValue};
use crate::variant::{self, SamplingPolicy};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use pact_models::bodies::OptionalBody;
use pact_models::generators::{Generator, Generators};
use pact_models::http_parts::HttpPart;
use pact_models::matchingrules::{MatchingRule, MatchingRuleCategory, MatchingRules, RuleList, RuleLogic};
use pact_models::path_exp::{DocPath, PathToken};
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

/// What `upgrade/pact`'s `options` may ask of a conversion (engine-protocol spec §8.4).
#[derive(Debug, Clone, Default)]
pub struct Options {
  /// Leave every pending interaction out of the contract, rather than converting it without its
  /// flag (ADR 0051). Each one left out is a `pending-excluded` finding.
  pub exclude_pending: bool,
}

/// Convert a v1–v4 pact document to a Janus contract, with the default [`Options`].
///
/// `Err` only for a document that is not a readable pact at all — everything the conversion cannot
/// carry across is a *finding*, not an error, because a partial conversion the author can see is
/// worth more than a refusal they cannot act on.
pub fn pact(source: &str, doc: &Value) -> Result<Upgraded, ContractError> {
  pact_with(source, doc, &Options::default())
}

/// Convert a v1–v4 pact document to a Janus contract, as `options` ask.
pub fn pact_with(source: &str, doc: &Value, options: &Options) -> Result<Upgraded, ContractError> {
  let pact = legacy_pact::read(source, doc)?;
  let mut conversion = Conversion::default();
  // Each HTTP interaction beside its position in the pact, so a finding points at the interaction
  // the author wrote, whatever was left out before it.
  let mut http = Vec::new();
  let mut needing_plugins = Vec::new();
  let mut excluded = 0;
  for (index, interaction) in legacy_pact::interactions(pact.as_ref()).into_iter().enumerate() {
    // ADR 0051: a contract has no pending interactions. Asked to, the conversion leaves them out,
    // so a contract upgraded from a pact whose pending interactions are known to fail verifies as
    // the pact does; otherwise each converts without its flag, as `pending-dropped` says.
    if options.exclude_pending && interaction.pending() {
      excluded += 1;
      conversion.finding(Finding::new(
        "pending-excluded",
        "lossy",
        format!("/interactions/{index}"),
        format!(
          "'{}' is pending and was left out, as asked: the contract says nothing about it. Pending \
           is the broker's to decide for a contract, from the provider's results (ADR 0051)",
          interaction.description()
        ),
      ));
      continue;
    }
    match interaction {
      legacy_pact::PactInteraction::Http(interaction) => http.push((index, *interaction)),
      // Verified since staged plan S1.7, but not converted: a contract has no message form until the
      // consumer side has one (S6.2). Counted below, with whatever else is not HTTP.
      legacy_pact::PactInteraction::Message(_) => {}
      legacy_pact::PactInteraction::NeedsPlugins(needs) => needing_plugins.push(needs),
    }
  }

  // Every interaction the pact declared, HTTP or not — a message interaction cannot be converted
  // by a design that has no message support yet, and vanishing silently is exactly the outcome
  // §8.1's honesty rule forbids.
  let declared = pact.interactions().len();
  let messages = declared - excluded - http.len() - needing_plugins.len();
  if messages > 0 {
    conversion.finding(Finding::new(
      "interaction-dropped",
      "lossy",
      "/interactions".to_string(),
      format!(
        "{messages} of {declared} interactions are not HTTP request/response and were not converted; \
         message interactions have no contract form yet"
      ),
    ));
  }
  // One that needs a pact plugin (staged plan S1.8): its body or rules are the plugin's, and a
  // contract converted without it would assert less than the pact does.
  for needs in &needing_plugins {
    let named = needs
      .plugins
      .iter()
      .map(|plugin| format!("'{plugin}'"))
      .collect::<Vec<_>>()
      .join(", ");
    conversion.finding(Finding::new(
      "interaction-dropped",
      "lossy",
      "/interactions".to_string(),
      format!(
        "'{}' needs the pact plugin(s) {named} and was not converted; pact plugins have no contract \
         form yet",
        needs.description
      ),
    ));
  }

  let mut interactions = Vec::with_capacity(http.len());
  let mut seen: Vec<(String, Option<Vec<State>>)> = Vec::new();
  let mut problems = Vec::new();
  for (index, interaction) in &http {
    match conversion.interaction(*index, interaction, &mut seen) {
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
    if interaction.pending {
      self.finding(Finding::new(
        "pending-dropped",
        "lossy",
        format!("{at}/pending"),
        "the interaction is pending, so v1–v4 report its failure without failing the run; a contract \
         has no pending interactions, so a failure of this one fails the run. Pending is the broker's \
         to decide for a contract, from the provider's results; to leave the interaction out instead, \
         ask the conversion to exclude pending interactions (ADR 0051)"
          .to_string(),
      ));
    }

    let mut declarations = Declarations::default();
    let mut parts: BTreeMap<String, ShapePart> = BTreeMap::new();
    parts.insert(
      "request".to_string(),
      self.request_shapes(&interaction.request, &at, &mut declarations),
    );
    parts.insert(
      "response".to_string(),
      self.response_shapes(&interaction.response, &at, &mut declarations),
    );

    let mut values = legacy_pact::request_parts(&interaction.request);
    values.extend(legacy_pact::response_parts(&interaction.response));
    declarations.record_evidence(&mut values);
    let Declarations {
      content_types,
      requires,
      ..
    } = declarations;

    let selection = self.selection(&description, &states, &parts, &content_types, &at)?;

    Ok(Interaction {
      description,
      transport: Some(Transport {
        kind: "http".to_string(),
        mode: Some("passive".to_string()),
      }),
      states,
      parts,
      // A body a built-in component reads (text, form, XML, multipart) declares its type, so it is
      // decoded as the pact's verification decodes it, whatever the reply labels it (§5.5). JSON and
      // bytes are what an undeclared slot already means (staged plan S1.25).
      content_types: (!content_types.is_empty()).then_some(content_types),
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
    content_types: &ContentTypes,
    at: &str,
  ) -> Result<BTreeMap<String, Value>, Vec<Problem>> {
    let mut document = json!({ "description": description, "parts": parts });
    if let Some(states) = states {
      document["states"] = json!(states);
    }
    if !content_types.is_empty() {
      document["content-types"] = json!(content_types);
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

  fn request_shapes(&mut self, request: &HttpRequest, at: &str, decl: &mut Declarations) -> ShapePart {
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
    // v1–v4 request queries are closed — every actual parameter must be one the pact named (spec
    // test case `query/unexpected param`) — and that holds for a request with no query, which
    // admits none. A shape can forbid a member it names but never the ones it does not (ADR 0007),
    // so every converted request admits more than its pact did, and says so.
    let query_where = Where::slot(at, "request", "query");
    self.finding(match request.query.as_ref().filter(|query| !query.is_empty()) {
      Some(_) => Finding::at(
        "request-query-opened",
        "lossy",
        &query_where,
        "v1–v4 request queries reject parameters the pact did not name; shapes are must-ignore \
         by design (ADR 0007), so the converted contract admits extra parameters. Add `forbidden` \
         members if any of them must stay out"
          .to_string(),
      ),
      None => Finding::new(
        "request-query-opened",
        "lossy",
        format!("{at}/request"),
        "v1–v4 treat a request with no query as one that sends none; shapes are must-ignore by \
         design (ADR 0007), so the converted contract admits a request with any query parameters \
         at all. Add a `query` with `forbidden` members if any of them must stay out"
          .to_string(),
      ),
    });
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
      let category = rules_for(rules, "query");
      part.insert(
        "query".to_string(),
        self.multi_map_slot(category.as_ref(), &query, false, &query_where),
      );
    }
    let body = self.headers_and_body(
      &mut part,
      request,
      request.headers.as_ref(),
      rules,
      "request",
      at,
      decl,
    );
    self.place_generators(&mut part, request, &request.generators, body, "request", at, decl);
    part
  }

  fn response_shapes(&mut self, response: &HttpResponse, at: &str, decl: &mut Declarations) -> ShapePart {
    let rules = &response.matching_rules;
    let mut part = ShapePart::new();
    part.insert("status".to_string(), self.status_slot(response, at, decl));
    let body = self.headers_and_body(
      &mut part,
      response,
      response.headers.as_ref(),
      rules,
      "response",
      at,
      decl,
    );
    self.place_generators(
      &mut part,
      response,
      &response.generators,
      body,
      "response",
      at,
      decl,
    );
    part
  }

  /// A part's `headers` and `body` slots, which are converted together: a pact that names no
  /// `Content-Type` header still checks the reply's type before its body (`legacy:body-type`, ADR
  /// 0036 decision 4), and the contract checks it on the header ([`body_type_header`]).
  #[allow(clippy::too_many_arguments)]
  fn headers_and_body(
    &mut self,
    part: &mut ShapePart,
    http: &dyn HttpPart,
    headers: Option<&std::collections::HashMap<String, Vec<String>>>,
    rules: &MatchingRules,
    part_name: &str,
    at: &str,
    decl: &mut Declarations,
  ) -> Option<BodyKindShaped> {
    let headers_at = Where::slot(at, part_name, "headers");
    let named = headers.is_some_and(|headers| {
      headers
        .keys()
        .any(|name| name.eq_ignore_ascii_case("content-type"))
    });
    let mut slot = headers
      .filter(|headers| !headers.is_empty())
      .map(|headers| self.headers_slot(rules, headers, &headers_at));
    let body = self.body_slot(http, rules, part_name, &Where::slot(at, part_name, "body"));
    if let Some(body) = &body
      && !named
      && let Some(checked) = &body.checked_type
      && let Some(header) = self.body_type_header(checked, &headers_at)
    {
      let slot = slot.get_or_insert_with(|| json!({ "shape": "object", "members": {} }));
      slot["members"]["content-type"] = header;
    }
    if let Some(slot) = slot {
      part.insert("headers".to_string(), slot);
    }
    let body = body?;
    decl.body(part_name, &body);
    part.insert("body".to_string(), body.shape);
    Some(body.kind)
  }

  /// `status`, with the one component operator contract-file spec §8.2's table names: a v4
  /// `statusCode` matcher becomes `http:status-class`, which the kernel does not understand and
  /// must not pretend to (shape spec §3.5). The interaction records the requirement, so a verifier
  /// without that component says `component-unavailable` naming it rather than quietly matching
  /// less.
  fn status_slot(&mut self, response: &HttpResponse, at: &str, decl: &mut Declarations) -> Value {
    let at = Where::slot(at, "response", "status");
    let example = Value::from(response.status);
    let category = rules_for(&response.matching_rules, "status");
    let Some((list, _)) = best_rule(category.as_ref(), &["$".to_string()], false) else {
      self.frozen(&at, 1);
      return json!({ "shape": "equality", "example": example });
    };
    if let Some(MatchingRule::StatusCode(status)) = list.rules.first() {
      decl.require("http");
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
    self.shape_for(Some(list), &example, &at, Text::No)
  }

  /// `method` and `path`: one value each, which is text when the pact's is (`legacy:parses-as`).
  fn scalar_slot(&mut self, rules: &MatchingRules, category: &str, example: &Value, at: &Where) -> Value {
    let category = rules_for(rules, category);
    let list = best_rule(category.as_ref(), &["$".to_string()], false);
    self.shape_for(list.map(|(list, _)| list), example, at, Text::of(example))
  }

  /// Headers, in the document form a transport actually produces: `{name: [value, …]}` — so an
  /// `object` of `array`s, never a flat string map (component-interfaces spec §4).
  ///
  /// A header with no rule is where this conversion is least exact, and it says so. v1–v4 compare
  /// a header value with their own defaulted comparison — MIME parameters as a set, whitespace
  /// around commas ignored (plan-grammar spec §4.4's `legacy:header-value`) — and the shape
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
    let category = rules_for(rules, "header");
    let slot = self.multi_map_slot(category.as_ref(), &headers, true, at);
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

  /// A `{name: [value, …]}` slot — headers, query parameters and a form body have the same document
  /// shape and the same per-value rule lookup, differing only in whether names are compared
  /// case-insensitively. Every value is text.
  fn multi_map_slot(
    &mut self,
    category: Option<&MatchingRuleCategory>,
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
        let list = best_rule(category, &fragments, case_insensitive);
        let example = Value::String(value.clone());
        let entry = member.push(&format!("/{index}"), &format!("/entries/{index}"));
        shapes.push(self.shape_for(list.map(|(list, _)| list), &example, &entry, Text::Yes));
      }
      members.insert(name.clone(), json!({ "shape": "array", "entries": shapes }));
    }
    json!({ "shape": "object", "members": members })
  }

  /// The check a v1–v4 pact makes on the reply's type when it names no `Content-Type` header
  /// (`legacy:body-type`, ADR 0036 decision 4), as a shape on the reply's `content-type` header: an
  /// `optional`, since a reply with no type passes it, of a pattern that admits what the check
  /// admits ([`media_type_pattern`]). A label that is not a type, or has a wildcard, also passes the
  /// check and not the pattern, which is the judgement this reports.
  fn body_type_header(&mut self, checked: &str, headers_at: &Where) -> Option<Value> {
    let pattern = media_type_pattern(checked)?;
    let at = headers_at.push("", "/members/content-type");
    self.finding(Finding::at(
      "body-type-on-header",
      "judgement",
      &at,
      format!(
        "the pact names no Content-Type header, and v1–v4 check that the body's type agrees with \
         '{checked}' before they compare it; the contract checks the content-type header with a \
         pattern that admits what that check admits, except a type that is not one or has a \
         wildcard, which v1–v4 also admit"
      ),
    ));
    Some(json!({
      "shape": "optional",
      "of": { "shape": "array", "entries": [
        { "shape": "regex", "pattern": pattern, "example": checked }
      ] },
    }))
  }

  /// The body, read as the pact's verification reads it ([`legacy_pact::legacy_body`]: by its
  /// content type, ADR 0036 decision 3), and shaped as the legacy compiler compares it, kind by kind
  /// (staged plan S1.25): JSON walked the way design 3.5 walks it, text and bytes one value at `$`,
  /// a form as names of values, XML element by element, multipart part by part.
  fn body_slot(
    &mut self,
    http: &dyn HttpPart,
    rules: &MatchingRules,
    part: &str,
    at: &Where,
  ) -> Option<ConvertedBody> {
    let content_type = legacy_pact::part_content_type(http);
    let checked_type = Some(content_type.to_string()).filter(|ct| !ct.contains('*'));
    let body = match legacy_pact::legacy_body(http) {
      // The interaction never mentioned a body: there is nothing to say about it, and an invented
      // shape would assert something the pact did not.
      Ok(None) => return None,
      // Declared and empty: v1–v4 require the actual body be absent too, which is `forbidden`.
      Ok(Some(LegacyBody::Empty)) => {
        return Some(ConvertedBody::of(
          json!({ "shape": "forbidden" }),
          BodyKindShaped::Empty,
        ));
      }
      Ok(Some(body)) => body,
      Err(Unsupported::Malformed(message)) => {
        self.finding(Finding::at(
          "body-not-parsed",
          "lossy",
          at,
          format!(
            "the {message}; the example is carried across as it stands, and nothing is asserted \
             about it"
          ),
        ));
        return None;
      }
    };
    // §8.4's `content-type-inferred` is about a pact that declared *nothing* — a body slot with
    // no type and no `Content-Type` header. A pact that put the type in its headers, as most do,
    // declared it perfectly well and gets no finding.
    let own_type = matches!(http.body(), OptionalBody::Present(_, Some(_), _));
    if !own_type && http.lookup_content_type().is_none() {
      self.finding(Finding::at(
        "content-type-inferred",
        "judgement",
        at,
        format!(
          "the pact declared no content type for this body, in the body or in a header, and it \
           was read as '{content_type}'"
        ),
      ));
    }
    let has_content = !matches!(
      &body,
      LegacyBody::Document(RuntimeValue::Null | RuntimeValue::Absent)
    );
    let category = rules_for(rules, "body");
    let closed = part == "request";
    let mut converted = match (BodyKind::of(&content_type), &body) {
      (BodyKind::Json, LegacyBody::Document(document)) => {
        // v1–v4 request bodies are closed to unnamed members and the shape language refuses closed
        // objects outright (ADR 0007 commitment 4, plan-grammar spec §5.3). The contract therefore
        // admits more than the pact did, at exactly one position, and says so.
        if closed && matches!(document, RuntimeValue::Object(_)) {
          self.opened(at, "request-body-opened", OPENED_MEMBERS);
        }
        let ctx = BodyCtx::root(at);
        // Declared too, so a reply is decoded as JSON whatever it is labelled, as the pact's
        // verification decodes it (ADR 0036 decision 4, ADR 0048 decision 10).
        ConvertedBody::declared(
          self.body_shape(category.as_ref(), &document.to_json(), &ctx),
          &content_type.to_string(),
          "json",
          BodyKindShaped::Json,
        )
        .requiring_nothing()
      }
      // Text that is UTF-8 is a string, and the text component reads it (ADR 0036 decision 1).
      (BodyKind::Text, LegacyBody::Document(RuntimeValue::String(text))) => {
        let list = best_rule(category.as_ref(), &["$".to_string()], false);
        let shape = self.shape_for(
          list.map(|(list, _)| list),
          &Value::String(text.clone()),
          at,
          Text::No,
        );
        ConvertedBody::declared(shape, &content_type.to_string(), "text", BodyKindShaped::Text)
      }
      // Octets: a binary type, or text that is not UTF-8. No component claims them, so the slot is
      // left undeclared, and an undeclared slot of a type nobody claims is bytes (ADR 0036 decision 2).
      (_, LegacyBody::Document(RuntimeValue::Bytes(bytes))) => {
        if BodyKind::of(&content_type) == BodyKind::Text {
          self.finding(Finding::at(
            "body-not-parsed",
            "judgement",
            at,
            format!(
              "the body is '{content_type}' and is not UTF-8, so it is compared as octets, as the \
               pact's verification compares it; a reply labelled as text is decoded as text, which \
               octets that are not UTF-8 are not"
            ),
          ));
        }
        let list = best_rule(category.as_ref(), &["$".to_string()], false);
        ConvertedBody::of(
          self.bytes_shape(list.map(|(list, _)| list), bytes, at),
          BodyKindShaped::Bytes,
        )
      }
      (_, LegacyBody::Form(form)) => {
        self.opened(
          at,
          if closed {
            "request-body-opened"
          } else {
            "response-body-opened"
          },
          OPENED_NAMES,
        );
        let shape = self.multi_map_slot(category.as_ref(), &form_parameters(form), false, at);
        ConvertedBody::declared(shape, &content_type.to_string(), "form", BodyKindShaped::Form)
      }
      (_, LegacyBody::Xml(document)) => {
        if closed {
          self.opened(at, "request-body-opened", OPENED_XML);
        }
        let shape = self.xml_body(document, category.as_ref(), closed, at);
        ConvertedBody::declared(shape, &content_type.to_string(), "xml", BodyKindShaped::Xml)
      }
      (_, LegacyBody::Multipart(parts)) => {
        let shape = self.multipart_body(parts, category.as_ref(), closed, at);
        // The boundary is the octets' own, so a reply's is read from its label (ADR 0047).
        ConvertedBody::declared(
          shape,
          &without_boundary(&content_type.to_string()),
          "multipart",
          BodyKindShaped::Multipart,
        )
      }
      // A JSON document of another kind (a string or a number, say), read as JSON.
      (_, LegacyBody::Document(document)) => {
        let ctx = BodyCtx::root(at);
        ConvertedBody::of(
          self.body_shape(category.as_ref(), &document.to_json(), &ctx),
          BodyKindShaped::Json,
        )
      }
      (_, LegacyBody::Empty) => ConvertedBody::of(json!({ "shape": "forbidden" }), BodyKindShaped::Empty),
    };
    converted.checked_type = checked_type.filter(|_| has_content);
    if converted.declared.is_some()
      && let OptionalBody::Present(bytes, _, _) = http.body()
    {
      converted.evidence = Some(octets(bytes, &content_type.to_string()));
    }
    Some(converted)
  }

  /// One value at `$` that is octets: the rules there or equality, with a base64 example (shape
  /// spec §2.2).
  fn bytes_shape(&mut self, list: Option<RuleList>, bytes: &[u8], at: &Where) -> Value {
    let mut shape = self.shape_for(list, &Value::String(BASE64.encode(bytes)), at, Text::No);
    if shape.get("example").is_some() {
      shape["encoded"] = json!("base64");
    }
    shape
  }

  /// A body the shape language must-ignores where v1–v4 close it: reported once per body.
  fn opened(&mut self, at: &Where, code: &str, what: &str) {
    if self
      .findings
      .iter()
      .any(|f| f.code == code && f.path == at.source)
    {
      return;
    }
    let part = if code == "request-body-opened" {
      "request"
    } else {
      "response"
    };
    self.finding(Finding::at(
      code,
      "lossy",
      at,
      format!(
        "v1–v4 {part} bodies reject {what} the pact did not name; shapes are must-ignore by design \
         (ADR 0007), so the converted contract admits them. Add `forbidden` members if any of them \
         must stay out"
      ),
    ));
  }

  fn body_shape(&mut self, rules: Option<&MatchingRuleCategory>, value: &Value, ctx: &BodyCtx) -> Value {
    let list = best_rule(rules, &ctx.fragments, false);
    match value {
      Value::Object(members) => self.object_shape(rules, members, list, ctx),
      Value::Array(items) => self.array_shape(rules, items, list, ctx),
      scalar => self.shape_for(list.map(|(list, _)| list), scalar, &ctx.at, Text::No),
    }
  }

  /// An object: the map-entry rules (`values`, `eachKey`, `eachValue`) describe the map itself and
  /// become `each-entry`; anything else declared on an object's own path does not replace the
  /// structural walk, exactly as design 3.5 found against the specification corpus.
  fn object_shape(
    &mut self,
    rules: Option<&MatchingRuleCategory>,
    members: &Map<String, Value>,
    list: Option<(RuleList, bool)>,
    ctx: &BodyCtx,
  ) -> Value {
    if let Some((list, false)) = &list
      && let Some(rule) = list.rules.iter().find(|rule| is_map_entry_rule(rule))
    {
      // The member first by name, not first as written: the sample decides every value's shape,
      // and nothing is decided by member order (ADR 0037).
      let first = members.iter().min_by(|(a, _), (b, _)| a.cmp(b));
      let sample = first.map(|(_, value)| value.clone()).unwrap_or(Value::Null);
      // `eachValue` matches the values by its own rules alone, at `*` below the map, as the legacy
      // compiler does (ADR 0042); the other two by the body's.
      let own = match rule {
        MatchingRule::EachValue(definition) => {
          let (own, unresolved) = definition_rules(definition);
          if !unresolved.is_empty() {
            self.finding(Finding::at(
              "rule-unmapped",
              "lossy",
              &ctx.at,
              "an eachValue rule refers to a rule it does not define, which v1–v4 fail on; the \
               reference was dropped"
                .to_string(),
            ));
          }
          Some(each_value_category(&ctx.fragments, own))
        }
        _ => None,
      };
      // `eachKey` asks something of every key, by its own rules, and nothing of the values but the
      // pact's own members, which v1–v4 compare only when the reply has them. `each-entry` cannot
      // say "this member, when it is there", so the values are left open, and that is reported.
      if let MatchingRule::EachKey(definition) = rule {
        let (own, _) = definition_rules(definition);
        let example = Value::String(first.map(|(name, _)| name.clone()).unwrap_or_default());
        let list = RuleList {
          rules: own,
          rule_logic: RuleLogic::And,
          cascaded: false,
        };
        let keys = if list.rules.is_empty() {
          json!({ "shape": "string", "example": example })
        } else {
          self.shape_for(Some(list), &example, &ctx.at, Text::Yes)
        };
        self.finding(Finding::at(
          "rule-unmapped",
          "lossy",
          &ctx.at,
          "an eachKey rule became each-entry's keys; v1–v4 also compare each member the pact names when \
           the reply has it, which each-entry cannot say, so the values are left open, or held to a rule \
           that reaches them, which v1–v4 apply to the pact's members alone"
            .to_string(),
        ));
        // A rule that reaches the values (`$.prices.*`) is applied to every one: stricter than v1–v4,
        // which apply it to the pact's own members, so a false failure rather than a false pass.
        let entry = ctx.entry_value();
        let values = if matches!(best_rule(rules, &entry.fragments, false), Some((_, false))) {
          self.body_shape(rules, &sample, &entry)
        } else {
          json!({ "shape": "any" })
        };
        return json!({ "shape": "each-entry", "keys": keys, "values": values, "min": 0 });
      }
      let values = self.body_shape(own.as_ref().or(rules), &sample, &ctx.entry_value());
      return json!({ "shape": "each-entry", "values": values, "min": 0 });
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
    rules: Option<&MatchingRuleCategory>,
    items: &[Value],
    list: Option<(RuleList, bool)>,
    ctx: &BodyCtx,
  ) -> Value {
    let Some((list, cascaded)) = list else {
      let entries: Vec<Value> = items
        .iter()
        .enumerate()
        .map(|(index, item)| self.body_shape(rules, item, &ctx.index(index)))
        .collect();
      return json!({ "shape": "array", "entries": entries });
    };
    if let Some(MatchingRule::ArrayContains(variants)) = list.rules.first() {
      return self.contains_shape(items, variants, ctx);
    }
    // `eachValue` on a list matches every element by its own rules alone, and any length, as the
    // legacy compiler applies it (ADR 0042).
    if !cascaded && let Some(MatchingRule::EachValue(definition)) = list.rules.first() {
      let (own, _) = definition_rules(definition);
      let own = each_value_category(&ctx.fragments, own);
      let template = items.first().cloned().unwrap_or(Value::Null);
      let items_shape = self.body_shape(Some(&own), &template, &ctx.template_item());
      return json!({ "shape": "each-like", "items": items_shape, "min": 0 });
    }
    // A rule that cascaded here from a shallower path carries no bound for this array: `min`/`max`
    // size the array that declared them, and v1–v4 never re-apply them below it (`pact_matching`
    // checks the bound only when `!cascaded`) — nor does design 3.5's plan, which reduces a
    // cascaded one to `match:type`, admitting any length. `min: 0` is what agrees with both.
    let (min, max) = if cascaded { (0, None) } else { cardinality(&list) };
    // A bare `type` names no bound, and v1–v4 then admit `[]` where `each-like`'s default `min: 1`
    // does not (phase-9 finding 2). Stricter, so false failures rather than false passes — but the
    // two paths now disagree about the empty list, and plan-grammar spec §4.4 wants that said.
    if !cascaded && list.rules.iter().all(|rule| matches!(rule, MatchingRule::Type)) {
      self.finding(Finding::at(
        "rule-narrowed",
        "judgement",
        &ctx.at,
        "a type rule on an array names no length, and v1–v4 admit an empty one; the contract's \
         each-like keeps its default min: 1 — write min: 0 if an empty list is a case the consumer \
         handles"
          .to_string(),
      ));
    }
    let template = items.first().cloned().unwrap_or(Value::Null);
    let items_shape = self.body_shape(rules, &template, &ctx.template_item());
    let mut shape = json!({ "shape": "each-like", "items": items_shape, "min": min });
    if let Some(max) = max {
      shape["max"] = json!(max);
    }
    shape
  }

  /// `arrayContains` (ADR 0042): each variant's element of the pact, by the variant's own rules rooted
  /// at the element, or equal to it when the variant has none or they ask for equality. The shape
  /// language's `contains` claims a distinct element for each entry, where v1–v4 let one element
  /// satisfy two variants, which is the judgement reported.
  fn contains_shape(
    &mut self,
    items: &[Value],
    variants: &[(
      usize,
      MatchingRuleCategory,
      std::collections::HashMap<pact_models::path_exp::DocPath, pact_models::generators::Generator>,
    )],
    ctx: &BodyCtx,
  ) -> Value {
    let variants: Vec<(usize, Option<&MatchingRuleCategory>)> = if variants.is_empty() {
      (0..items.len()).map(|index| (index, None)).collect()
    } else {
      variants
        .iter()
        .map(|(index, rules, _)| (*index, Some(rules)))
        .collect()
    };
    let mut entries = Vec::new();
    for (index, rules) in variants {
      let element = ctx.index(index);
      let Some(item) = items.get(index) else {
        self.finding(Finding::at(
          "rule-unmapped",
          "lossy",
          &ctx.at,
          format!("an arrayContains variant names element {index}, which the pact's list does not have; it was dropped"),
        ));
        continue;
      };
      let element = BodyCtx {
        fragments: vec!["$".to_string()],
        ..element
      };
      entries.push(match rules.filter(|rules| !is_whole_equality(rules)) {
        Some(rules) => self.body_shape(Some(rules), item, &element),
        None => {
          self.frozen(&element.at, 1);
          json!({ "shape": "equality", "example": item })
        }
      });
    }
    if entries.len() > 1 {
      self.finding(Finding::at(
        "rule-narrowed",
        "judgement",
        &ctx.at,
        "each arrayContains variant became an entry of `contains`, which needs a distinct element for \
         each; v1–v4 let one element satisfy more than one variant"
          .to_string(),
      ));
    }
    json!({ "shape": "contains", "entries": entries })
  }

  // -------------------------------------------------------------------------------------------
  // XML (ADR 0039's document) and multipart (ADR 0040's), as the legacy compiler compares them
  // -------------------------------------------------------------------------------------------

  /// An XML body: an object holding its root element ([`Self::xml_element`]). Positions inside it
  /// have no pointer into the pact, whose body is one string, so a finding names the body.
  fn xml_body(
    &mut self,
    document: &RuntimeValue,
    rules: Option<&MatchingRuleCategory>,
    closed: bool,
    at: &Where,
  ) -> Value {
    let RuntimeValue::Object(root) = document else {
      return json!({ "shape": "any" });
    };
    let mut members = Map::new();
    let mut narrowed = 0;
    for (name, element) in root {
      let element = only_element(element);
      let fragments = vec!["$".to_string(), rule_name(name, namespace_of(element))];
      let at = at.push("", &format!("/members/{}", escape_pointer(name)));
      let ctx = XmlCtx { rules, closed };
      members.insert(
        name.clone(),
        self.xml_element(element, name, &fragments, &ctx, &at, &mut narrowed),
      );
    }
    if narrowed > 0 {
      self.finding(Finding::at(
        "rule-narrowed",
        "judgement",
        at,
        format!(
          "{narrowed} group(s) of child elements with no type rule became fixed arrays: v1–v4 \
           compare the pact's children with the reply's first ones and allow more of the same name \
           in a response, and the contract admits exactly as many"
        ),
      ));
    }
    json!({ "shape": "object", "members": members })
  }

  /// One element, as `compile_xml_element` compares it: its rule, namespace, attributes, text and
  /// children, and what it may have that the pact does not name.
  fn xml_element(
    &mut self,
    element: &RuntimeValue,
    name: &str,
    fragments: &[String],
    ctx: &XmlCtx,
    at: &Where,
    narrowed: &mut usize,
  ) -> Value {
    let RuntimeValue::Object(element_members) = element else {
      return json!({ "shape": "any" });
    };
    let mut members = Map::new();
    if let Some((list, cascaded)) = best_rule(ctx.rules, fragments, false) {
      self.xml_element_rule(&list, cascaded, name, at);
    }
    if let Some(namespace) = namespace_of(element) {
      members.insert(
        legacy_xml::NAMESPACE.to_string(),
        json!({ "shape": "equality", "example": namespace }),
      );
    }

    // Attributes, by name; each is text.
    let mut attributes: Vec<(&String, &RuntimeValue)> = element_members
      .iter()
      .filter(|(key, _)| key.starts_with('@'))
      .collect();
    attributes.sort_by_key(|(key, _)| *key);
    for (key, value) in attributes {
      let mut attribute = fragments.to_vec();
      attribute.push(format!("@{}", rule_name(&key[1..], None)));
      let list = best_rule(ctx.rules, &attribute, false).map(|(list, _)| list);
      let member = at.push("", &format!("/members/{}", escape_pointer(key)));
      members.insert(
        key.clone(),
        self.shape_for(list, &value.to_json(), &member, Text::Yes),
      );
    }

    // Text, compared whether or not the pact has any.
    let expected = match element_members.get(legacy_xml::TEXT) {
      Some(RuntimeValue::String(text)) => text.clone(),
      _ => String::new(),
    };
    let mut text_fragments = fragments.to_vec();
    text_fragments.push(legacy_xml::TEXT.to_string());
    let text_at = at.push("", &format!("/members/{}", legacy_xml::TEXT));
    let text = match best_rule(ctx.rules, &text_fragments, false) {
      Some((list, cascaded)) => {
        let example = Value::String(expected.clone());
        // An element with no text is the empty string to a rule. A type rule admits it, and every
        // text the component writes is a string, so it says nothing; another rule that admits it is
        // optional, and one that does not requires text.
        if accepts_empty_text(&list, cascaded, &Literal::string(&expected)) {
          if list.rules.iter().all(is_type_rule) {
            None
          } else {
            let of = self.shape_for(Some(list), &example, &text_at, Text::Yes);
            Some(json!({ "shape": "optional", "of": of }))
          }
        } else {
          Some(self.shape_for(Some(list), &example, &text_at, Text::Yes))
        }
      }
      // The component writes no `#text` that would be empty, so the pact's empty text is none.
      None if expected.is_empty() => Some(json!({ "shape": "forbidden" })),
      None => {
        self.frozen(&text_at, 1);
        Some(json!({ "shape": "equality", "example": expected }))
      }
    };
    if let Some(text) = text {
      members.insert(legacy_xml::TEXT.to_string(), text);
    }

    // Children, a group per name, by name.
    let mut children: Vec<(&String, Vec<&RuntimeValue>)> = element_members
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
      let group = at.push("", &format!("/members/{}", escape_pointer(child)));
      let shape = if type_rule_reaches(ctx.rules, &child_fragments) {
        let item = self.xml_element(
          first,
          child,
          &child_fragments,
          ctx,
          &group.push("", "/items"),
          narrowed,
        );
        json!({ "shape": "each-like", "items": item, "min": 1 })
      } else {
        if !ctx.closed {
          *narrowed += 1;
        }
        let entries: Vec<Value> = items
          .iter()
          .enumerate()
          .map(|(index, item)| {
            let entry = group.push("", &format!("/entries/{index}"));
            self.xml_element(item, child, &child_fragments, ctx, &entry, narrowed)
          })
          .collect();
        json!({ "shape": "array", "entries": entries })
      };
      members.insert((*child).clone(), shape);
    }

    // What the element may have that the pact does not name. A request's is closed, which
    // `request-body-opened` reports for the whole body.
    if !ctx.closed {
      let mut unnamed = fragments.to_vec();
      unnamed.push(String::new());
      if type_rule_reaches(ctx.rules, &unnamed) {
        self.opened(at, "response-body-opened", OPENED_CHILDREN);
      } else {
        for forbidden in type_rule_names(ctx.rules, fragments) {
          if !children.iter().any(|(name, _)| **name == forbidden) {
            members.insert(forbidden, json!({ "shape": "forbidden" }));
          }
        }
      }
    }
    json!({ "shape": "object", "members": members })
  }

  /// A rule on an element itself (`xml_element_rule`): a type rule asks that it be an element, which
  /// `object` does. A name rule compares the pact's own name, so it is decided here. A size rule
  /// counts the element's children of every name, and `notEmpty` asks for a child or text, which no
  /// shape says.
  fn xml_element_rule(&mut self, list: &RuleList, cascaded: bool, name: &str, at: &Where) {
    for rule in &list.rules {
      let unmapped = match rule {
        MatchingRule::Type | MatchingRule::Equality => false,
        MatchingRule::MinType(_) | MatchingRule::MaxType(_) | MatchingRule::MinMaxType(_, _) => !cascaded,
        MatchingRule::Regex(pattern) => regex::Regex::new(&anchored_pattern(pattern))
          .map(|regex| !regex.is_match(name))
          .unwrap_or(true),
        _ => true,
      };
      if unmapped {
        self.finding(Finding::at(
          "rule-unmapped",
          "lossy",
          at,
          format!(
            "the matching rule '{}' on the element '{name}' has no shape equivalent: a size rule \
             counts its children of every name, and notEmpty asks for a child or text; it was \
             dropped",
            rule.name()
          ),
        ));
      }
    }
  }

  /// A multipart body, as `compile_multipart_body` compares one: each name's parts, the reply's first
  /// of that name against the pact's, or the unnamed ones by position. A part is its headers, its
  /// kind (a field has no file name) and its body. The contract admits exactly the pact's number of
  /// parts of each name, where v1–v4 allow more, which it reports.
  fn multipart_body(
    &mut self,
    parts: &[LegacyPart],
    rules: Option<&MatchingRuleCategory>,
    closed: bool,
    at: &Where,
  ) -> Value {
    let mut groups: indexmap::IndexMap<&str, Vec<&LegacyPart>> = indexmap::IndexMap::new();
    for part in parts {
      groups.entry(part.name.as_str()).or_default().push(part);
    }
    let mut members = Map::new();
    for (name, group) in &groups {
      let group_at = at.push("", &format!("/members/{}", escape_pointer(name)));
      // Every part the pact has of a name is compared with the reply's first of it.
      let compared: Vec<&LegacyPart> = if name.is_empty() {
        group.clone()
      } else {
        if group.len() > 1 {
          self.finding(Finding::at(
            "rule-unmapped",
            "lossy",
            &group_at,
            format!(
              "{} parts are named '{name}', and v1–v4 compare the reply's first part of that name \
               with each of them; the contract keeps the first",
              group.len()
            ),
          ));
        }
        vec![group[0]]
      };
      let entries: Vec<Value> = compared
        .iter()
        .enumerate()
        .map(|(index, part)| {
          let entry = group_at.push("", &format!("/entries/{index}"));
          self.part_shape(part, rules, closed, &entry)
        })
        .collect();
      members.insert(name.to_string(), json!({ "shape": "array", "entries": entries }));
    }
    if !groups.is_empty() {
      self.finding(Finding::at(
        "rule-narrowed",
        "judgement",
        at,
        "each name's parts became a fixed array: v1–v4 compare the reply's first part of a name and \
         allow more of it, and the contract admits exactly as many as the pact has"
          .to_string(),
      ));
    }
    json!({ "shape": "object", "members": members })
  }

  /// One part (`compile_part_headers`, `compile_part_body`).
  fn part_shape(
    &mut self,
    part: &LegacyPart,
    rules: Option<&MatchingRuleCategory>,
    closed: bool,
    at: &Where,
  ) -> Value {
    let key = if part.name.is_empty() {
      part.index.to_string()
    } else {
      part.name.clone()
    };
    let headers_at = at.push("", &format!("/members/{}", legacy_multipart::HEADERS));
    let mut headers = Map::new();
    let mut sorted = part.headers.clone();
    sorted.sort();
    for (name, value) in &sorted {
      let fragments = vec!["$".to_string(), key.clone(), name.clone()];
      let header_at = headers_at.push("", &format!("/members/{}", escape_pointer(name)));
      let example = Value::String(value.clone());
      let shape = match best_rule(rules, &fragments, true) {
        Some((list, false)) => self.shape_for(Some(list), &example, &header_at, Text::Yes),
        // A part's `Content-Disposition` is compared only by a rule, as today.
        _ if name == "content-disposition" => continue,
        _ => {
          self.frozen(&header_at, 1);
          json!({ "shape": "equality", "example": example })
        }
      };
      let shape = if OPTIONAL_PART_HEADERS.contains(&name.as_str()) {
        json!({ "shape": "optional", "of": shape })
      } else {
        shape
      };
      headers.insert(name.clone(), shape);
    }
    let mut members = Map::new();

    // Its kind: a field has a `Content-Disposition` and no file name. A named reply part has one,
    // since its name is in it, so the file name decides.
    match (&part.filename, part.field) {
      (Some(filename), _) => {
        members.insert(
          legacy_multipart::FILENAME.to_string(),
          json!({ "shape": "type", "example": filename }),
        );
      }
      (None, true) => {
        members.insert(
          legacy_multipart::FILENAME.to_string(),
          json!({ "shape": "forbidden" }),
        );
        if part.name.is_empty() && !headers.contains_key("content-disposition") {
          let disposition = part
            .headers
            .iter()
            .find(|(name, _)| name == "content-disposition")
            .map(|(_, value)| value.clone())
            .unwrap_or_default();
          headers.insert(
            "content-disposition".to_string(),
            json!({ "shape": "type", "example": disposition }),
          );
        }
      }
      // An unnamed part with no `Content-Disposition`: a reply's part at its position is a file
      // when it has a file name or none, which no shape here says.
      (None, false) => self.finding(Finding::at(
        "rule-unmapped",
        "lossy",
        at,
        "v1–v4 require the reply's part here to be a file, not a field; no shape the conversion \
         writes says so, and the contract admits either"
          .to_string(),
      )),
    }
    members.insert(
      legacy_multipart::HEADERS.to_string(),
      json!({ "shape": "object", "members": headers }),
    );

    // Its body. A field of text is compared as text, by the rules at `$.<part>`; anything else as a
    // body of its type, by the rules under `$.<part>` moved to its root.
    let body_at = at.push("", &format!("/members/{}", legacy_multipart::BODY));
    let text = part
      .content_type
      .as_deref()
      .is_none_or(|content_type| content_type.trim().to_ascii_lowercase().starts_with("text/"));
    let body = if part.field && text {
      let list = best_rule(rules, &["$".to_string(), key.clone()], false).map(|(list, _)| list);
      match part.body.document() {
        RuntimeValue::Bytes(bytes) => self.bytes_shape(list, &bytes, &body_at),
        document => self.shape_for(list, &document.to_json(), &body_at, Text::Yes),
      }
    } else {
      let rules = rerooted(rules, &key);
      self.part_body(&part.body, rules.as_ref(), closed, &body_at)
    };
    members.insert(legacy_multipart::BODY.to_string(), body);
    json!({ "shape": "object", "members": members })
  }

  /// A part's body of its own type (`compile_body_content`), closed when the body is a request's.
  fn part_body(
    &mut self,
    body: &LegacyBody,
    rules: Option<&MatchingRuleCategory>,
    closed: bool,
    at: &Where,
  ) -> Value {
    match body {
      LegacyBody::Document(RuntimeValue::Bytes(bytes)) => {
        let list = best_rule(rules, &["$".to_string()], false).map(|(list, _)| list);
        self.bytes_shape(list, bytes, at)
      }
      LegacyBody::Document(document) => {
        if closed && matches!(document, RuntimeValue::Object(_)) {
          self.opened(at, "request-body-opened", OPENED_MEMBERS);
        }
        self.body_shape(rules, &document.to_json(), &BodyCtx::nested(at))
      }
      LegacyBody::Form(form) => {
        self.opened(
          at,
          if closed {
            "request-body-opened"
          } else {
            "response-body-opened"
          },
          OPENED_NAMES,
        );
        self.multi_map_slot(rules, &form_parameters(form), false, at)
      }
      LegacyBody::Xml(document) => {
        if closed {
          self.opened(at, "request-body-opened", OPENED_XML);
        }
        self.xml_body(document, rules, closed, at)
      }
      // A part is never empty, and one that is itself multipart is bytes.
      LegacyBody::Empty | LegacyBody::Multipart(_) => json!({ "shape": "any" }),
    }
  }

  // -------------------------------------------------------------------------------------------
  // One position's rule -> one shape
  // -------------------------------------------------------------------------------------------

  /// The heart of §8.2's table. `None` is not a gap: "no rule at this position" is what v1–v4
  /// already mean by an example, and the conversion writes that down as `equality` rather than
  /// changing it — with a `note`, because an author reading the contract should know which
  /// constraints they chose and which the format chose for them.
  ///
  /// `text` says whether the value is text, which v1–v4 read against a number or boolean rule by
  /// parsing it (`legacy:parses-as`): such a rule becomes the pattern that reading accepts.
  fn shape_for(&mut self, list: Option<RuleList>, example: &Value, at: &Where, text: Text) -> Value {
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
    if text == Text::Yes
      && let Some(pattern) = text_pattern(rule)
    {
      self.patterned(at);
      return json!({ "shape": "regex", "pattern": pattern, "example": example });
    }
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
      let n = leading_count(&existing.message) + count;
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

  /// One `rule-as-pattern` note per slot, counting the rules on text it wrote as patterns.
  fn patterned(&mut self, at: &Where) {
    let slot = slot_of(&at.source);
    if let Some(existing) = self
      .findings
      .iter_mut()
      .find(|f| f.code == "rule-as-pattern" && f.path == slot)
    {
      let n = leading_count(&existing.message) + 1;
      existing.message = patterned_message(n);
      return;
    }
    self.finding(Finding {
      code: "rule-as-pattern".to_string(),
      kind: "note".to_string(),
      path: slot,
      message: patterned_message(1),
      target: Some(slot_of(&at.target)),
    });
  }

  /// A v1–v4 generator onto the shape at its target (shape-language spec §3.4, ADR 0048), naming the
  /// built-in `pact` component's generator and its configuration: a verifier fills a request's when it
  /// sends it, as it generates a pact's (ADR 0034), and a mock a response's when it serves it. What
  /// the contract accepts is unchanged. A generator no component provides, or one at a target today's
  /// verifier does not generate, is dropped and reported: lossy, never dangerous, since it weakens what
  /// a contract can *produce*, never what it *accepts*.
  #[allow(clippy::too_many_arguments)]
  fn place_generators(
    &mut self,
    part: &mut ShapePart,
    http: &dyn HttpPart,
    generators: &Generators,
    body: Option<BodyKindShaped>,
    part_name: &str,
    at: &str,
    decl: &mut Declarations,
  ) {
    let mut placed_in_body = false;
    let mut every: Vec<(String, String, &DocPath, &Generator)> = generators
      .categories
      .iter()
      .flat_map(|(category, generators)| {
        let category = category_name(category).to_string();
        generators
          .iter()
          .map(move |(path, generator)| (category.clone(), path.to_string(), path, generator))
      })
      .collect();
    every.sort_by(|a, b| (&a.0, &a.1).cmp(&(&b.0, &b.1)));
    for (category, path, key, generator) in every {
      // Where the pact holds it: a body generator under its path, a header or query one under its
      // name, and a path or status one as the category itself.
      let pointer = match category.as_str() {
        "body" => format!("{at}/{part_name}/generators/body/{}", escape_pointer(&path)),
        "header" | "query" => format!(
          "{at}/{part_name}/generators/{category}/{}",
          escape_pointer(key.first_field().unwrap_or(&path))
        ),
        _ => format!("{at}/{part_name}/generators/{category}"),
      };
      let name = generator.name();
      if !PactGenerators::GENERATORS.contains(&name.as_str()) {
        let why = if matches!(generator, Generator::MockServerURL(..)) {
          "a MockServerURL generator writes the consumer's mock server URL, which no generator component \
           provides yet; the recorded value is kept"
            .to_string()
        } else {
          format!("no generator component provides '{name}'; the recorded value is kept")
        };
        self.finding(Finding::new("generator-dropped", "lossy", pointer, why));
        continue;
      }
      let (slot, targets) = match category.as_str() {
        "path" => ("path", vec![String::new()]),
        "status" => ("status", vec![String::new()]),
        "header" => match key.first_field() {
          Some(header) => (
            "headers",
            vec![format!(
              "/members/{}/entries/0",
              escape_pointer(&header.to_ascii_lowercase())
            )],
          ),
          None => ("headers", Vec::new()),
        },
        "query" => match key.first_field() {
          Some(name) => {
            let count = part
              .get("query")
              .and_then(|query| query.pointer(&format!("/members/{}/entries", escape_pointer(name))))
              .and_then(Value::as_array)
              .map_or(0, Vec::len);
            (
              "query",
              (0..count)
                .map(|index| format!("/members/{}/entries/{index}", escape_pointer(name)))
                .collect(),
            )
          }
          None => ("query", Vec::new()),
        },
        "body" => {
          let tokens = key.tokens();
          let shape = part.get("body").cloned().unwrap_or(Value::Null);
          let targets = match body {
            Some(BodyKindShaped::Json) => {
              let mut out = Vec::new();
              json_targets(
                &shape,
                tokens.get(1..).unwrap_or_default(),
                String::new(),
                &mut out,
              );
              out
            }
            Some(BodyKindShaped::Form) => form_target(&shape, tokens).into_iter().collect(),
            Some(BodyKindShaped::Xml) => xml_targets(&shape, tokens),
            _ => {
              self.finding(Finding::new(
                "generator-dropped",
                "lossy",
                pointer,
                "a body generator applies to a JSON, form or XML body, as today's verifier applies \
                 one, and this body is not one; it was dropped, as today's verifier ignores it"
                  .to_string(),
              ));
              continue;
            }
          };
          ("body", targets)
        }
        other => {
          self.finding(Finding::new(
            "generator-dropped",
            "lossy",
            pointer,
            format!("a generator for the '{other}' category has no slot here; it was dropped"),
          ));
          continue;
        }
      };
      let node = generator_node(generator);
      let mut placed = 0;
      if let Some(slot_shape) = part.get_mut(slot) {
        for target in &targets {
          if let Some(Value::Object(shape)) = slot_shape.pointer_mut(target)
            && shape.get("shape").and_then(Value::as_str) != Some("forbidden")
          {
            shape.insert(crate::shape::fresh::GENERATOR.to_string(), node.clone());
            placed += 1;
          }
        }
      }
      if placed == 0 {
        self.finding(Finding::new(
          "generator-dropped",
          "lossy",
          pointer,
          format!(
            "the generator's path names no value the pact's example has, or one the contract shapes as \
             a list rule's template (`{path}`); it was dropped"
          ),
        ));
      } else if slot == "body" {
        placed_in_body = true;
      }
    }
    // A request body is filled as its document, and a JSON one is read by the type the pact gives it,
    // so its evidence carries that type.
    if placed_in_body
      && part_name == "request"
      && body == Some(BodyKindShaped::Json)
      && !decl.evidence.contains_key(part_name)
      && let OptionalBody::Present(bytes, _, _) = http.body()
    {
      let content_type = legacy_pact::part_content_type(http).to_string();
      decl
        .evidence
        .insert(part_name.to_string(), octets(bytes, &content_type));
    }
  }
}

/// A generator category as a pact's `generators` member names it.
fn category_name(category: &pact_models::generators::GeneratorCategory) -> &'static str {
  use pact_models::generators::GeneratorCategory;
  match category {
    GeneratorCategory::METHOD => "method",
    GeneratorCategory::PATH => "path",
    GeneratorCategory::HEADER => "header",
    GeneratorCategory::QUERY => "query",
    GeneratorCategory::BODY => "body",
    GeneratorCategory::STATUS => "status",
    GeneratorCategory::METADATA => "metadata",
  }
}

/// A generator as a shape's `generator` member: the `pact` component's name for it and its
/// configuration, the members the pact gives it but its `type`.
fn generator_node(generator: &Generator) -> Value {
  let mut config = generator
    .to_json()
    .and_then(|json| json.as_object().cloned())
    .unwrap_or_default();
  config.remove("type");
  let mut node = json!({ "name": PactGenerators::qualified(&generator.name()) });
  if !config.is_empty() {
    node["config"] = Value::Object(config);
  }
  node
}

/// Every node of a JSON body's shape a generator's path names, as pointers into the shape: a member
/// by name, or every member for `*`; an entry by index, or every entry or an `each-like`'s template
/// for `[*]`; `optional` and `nullable` looked through. An index into an `each-like` names one element
/// where the template is every element, so it names nothing.
fn json_targets(shape: &Value, tokens: &[PathToken], at: String, out: &mut Vec<String>) {
  let operator = shape.get("shape").and_then(Value::as_str).unwrap_or_default();
  if matches!(operator, "optional" | "nullable") {
    if let Some(of) = shape.get("of") {
      json_targets(of, tokens, format!("{at}/of"), out);
    }
    return;
  }
  let Some((token, rest)) = tokens.split_first() else {
    out.push(at);
    return;
  };
  let mut each = |key: &str, child: &Value| json_targets(child, rest, format!("{at}/{key}"), out);
  match (token, operator) {
    (PathToken::Field(name), "object") => {
      if let Some(member) = shape.pointer(&format!("/members/{}", escape_pointer(name))) {
        each(&format!("members/{}", escape_pointer(name)), member);
      }
    }
    (PathToken::Star, "object") => {
      for (name, member) in shape
        .get("members")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
      {
        each(&format!("members/{}", escape_pointer(name)), member);
      }
    }
    (PathToken::Field(_) | PathToken::Star, "each-entry") => {
      if let Some(values) = shape.get("values") {
        each("values", values);
      }
    }
    (PathToken::Index(index), "array" | "contains") => {
      if let Some(entry) = shape.pointer(&format!("/entries/{index}")) {
        each(&format!("entries/{index}"), entry);
      }
    }
    (PathToken::StarIndex | PathToken::Star, "array") => {
      for (index, entry) in shape
        .get("entries")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .enumerate()
      {
        each(&format!("entries/{index}"), entry);
      }
    }
    (PathToken::StarIndex | PathToken::Star, "each-like") => {
      if let Some(items) = shape.get("items") {
        each("items", items);
      }
    }
    _ => {}
  }
}

/// The value a form body's generator names (`FormUrlEncodedHandler`): `$.<name>` is the name's first
/// value, `$.<name>[i]` its `i`th.
fn form_target(shape: &Value, tokens: &[PathToken]) -> Option<String> {
  let (name, index) = match tokens {
    [PathToken::Root, PathToken::Field(name)] => (name, 0),
    [PathToken::Root, PathToken::Field(name), PathToken::Index(index)] => (name, *index),
    _ => return None,
  };
  let target = format!("/members/{}/entries/{index}", escape_pointer(name));
  shape.pointer(&target).map(|_| target)
}

/// The values an XML body's generator names (`XmlHandler`): its path goes element by element from the
/// root, every child of a name, and ends at an attribute (`@name`) or the text (`#text`). Names are
/// matched by local name, since ADR 0039's document keeps no prefixes.
fn xml_targets(shape: &Value, tokens: &[PathToken]) -> Vec<String> {
  let local = |name: &str| name.rsplit(':').next().unwrap_or(name).to_string();
  let fields: Vec<&str> = tokens
    .iter()
    .filter_map(|token| match token {
      PathToken::Field(name) => Some(name.as_str()),
      _ => None,
    })
    .collect();
  let Some((root, rest)) = fields.split_first() else {
    return Vec::new();
  };
  let mut current = vec![format!("/members/{}", escape_pointer(&local(root)))];
  for (position, field) in rest.iter().enumerate() {
    let last = position + 1 == rest.len();
    let mut next = Vec::new();
    for at in &current {
      let Some(element) = shape.pointer(at) else {
        continue;
      };
      let members = element.get("members").and_then(Value::as_object);
      if let Some(attribute) = field.strip_prefix('@').filter(|_| last) {
        let wanted = local(attribute);
        for key in members.into_iter().flatten().map(|(key, _)| key) {
          let name = key.strip_prefix('@').unwrap_or_default();
          let name = name.rsplit('}').next().unwrap_or(name);
          if key.starts_with('@') && name == wanted {
            next.push(format!("{at}/members/{}", escape_pointer(key)));
          }
        }
      } else if *field == legacy_xml::TEXT && last {
        next.push(format!("{at}/members/{}", legacy_xml::TEXT));
      } else {
        let group = format!("{at}/members/{}", escape_pointer(&local(field)));
        match shape
          .pointer(&group)
          .and_then(|group| group.get("shape"))
          .and_then(Value::as_str)
        {
          Some("each-like") => next.push(format!("{group}/items")),
          Some("array") => {
            let count = shape
              .pointer(&format!("{group}/entries"))
              .and_then(Value::as_array)
              .map_or(0, Vec::len);
            next.extend((0..count).map(|index| format!("{group}/entries/{index}")));
          }
          _ => {}
        }
      }
    }
    current = next;
  }
  // A path that stops at an element names no value.
  if rest
    .last()
    .is_none_or(|field| !field.starts_with('@') && *field != legacy_xml::TEXT)
  {
    return Vec::new();
  }
  current
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

/// The count a collapsed finding's message starts with.
fn leading_count(message: &str) -> usize {
  message
    .split_whitespace()
    .next()
    .and_then(|n| n.parse::<usize>().ok())
    .unwrap_or(1)
}

fn patterned_message(count: usize) -> String {
  format!(
    "{count} number or boolean rule(s) here are on text, which v1–v4 read by parsing it, and became \
     the pattern that reading accepts. An integer's pattern also admits digits too large for a \
     64-bit integer, which v1–v4 refuse"
  )
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
/// design 3.5 scores against, and the two pointers a finding reports. A walk inside a body the pact
/// holds as one string (a multipart part's) has no pointer into the pact below the body, so
/// `source` stays there.
struct BodyCtx {
  fragments: Vec<String>,
  at: Where,
  in_pact: bool,
}

impl BodyCtx {
  /// A JSON body, whose positions are positions in the pact too.
  fn root(at: &Where) -> BodyCtx {
    BodyCtx {
      fragments: vec!["$".to_string()],
      at: at.clone(),
      in_pact: true,
    }
  }

  /// A JSON document inside a body the pact holds as one string.
  fn nested(at: &Where) -> BodyCtx {
    BodyCtx {
      in_pact: false,
      ..BodyCtx::root(at)
    }
  }

  fn push(&self, fragment: String, source: &str, target: &str) -> BodyCtx {
    let mut fragments = self.fragments.clone();
    fragments.push(fragment);
    BodyCtx {
      fragments,
      at: self.at.push(if self.in_pact { source } else { "" }, target),
      in_pact: self.in_pact,
    }
  }

  fn member(&self, name: &str) -> BodyCtx {
    let escaped = escape_pointer(name);
    self.push(
      name.to_string(),
      &format!("/{escaped}"),
      &format!("/members/{escaped}"),
    )
  }

  fn index(&self, index: usize) -> BodyCtx {
    self.push(
      index.to_string(),
      &format!("/{index}"),
      &format!("/entries/{index}"),
    )
  }

  /// The template element inside a rule-driven array: the synthetic `"0"` fragment design 3.5 uses
  /// for the same purpose — it scores identically to any real index against a `[*]` rule token.
  /// In the contract it is the `each-like`'s single `items` shape, not an entry.
  fn template_item(&self) -> BodyCtx {
    self.push("0".to_string(), "/0", "/items")
  }

  fn entry_value(&self) -> BodyCtx {
    self.push("*".to_string(), "/*", "/values")
  }
}

/// An XML walk's rules, and whether its elements are closed (a request's).
struct XmlCtx<'a> {
  rules: Option<&'a MatchingRuleCategory>,
  closed: bool,
}

/// Whether a value is text, which v1–v4 read against a number or boolean rule by parsing it
/// (`legacy:parses-as`, ADR 0039 decision 5): a path, a header, a query or form value, an XML
/// attribute or text, a multipart field. A body's own value is not, even when it is a string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Text {
  Yes,
  No,
}

impl Text {
  fn of(example: &Value) -> Text {
    if example.is_string() { Text::Yes } else { Text::No }
  }
}

/// A body converted (staged plan S1.25): its shape, and what the interaction must say for it to be
/// read as the pact's verification reads it.
struct ConvertedBody {
  shape: Value,
  /// Which document the shape is over, which says where a body generator's path lands.
  kind: BodyKindShaped,
  /// Its type, for `content-types` (contract-file spec §5.5): a body a built-in component reads.
  declared: Option<String>,
  /// That component, for `requires` (§7).
  component: Option<&'static str>,
  /// The pact's octets, as the variant's evidence, for a declared body.
  evidence: Option<SlotValue>,
  /// The type a reply's must agree with, for a body with content: the pact's, unless it has a
  /// wildcard (`legacy:body-type`).
  checked_type: Option<String>,
}

impl ConvertedBody {
  fn of(shape: Value, kind: BodyKindShaped) -> ConvertedBody {
    ConvertedBody {
      shape,
      kind,
      declared: None,
      component: None,
      evidence: None,
      checked_type: None,
    }
  }

  /// JSON is the content component every engine is built with, which an undeclared slot already
  /// assumes, so a JSON body's declaration needs no requirement.
  fn requiring_nothing(self) -> ConvertedBody {
    ConvertedBody {
      component: None,
      ..self
    }
  }

  fn declared(
    shape: Value,
    content_type: &str,
    component: &'static str,
    kind: BodyKindShaped,
  ) -> ConvertedBody {
    ConvertedBody {
      declared: Some(content_type.to_string()),
      component: Some(component),
      ..ConvertedBody::of(shape, kind)
    }
  }
}

/// Which document a converted body's shape is over.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BodyKindShaped {
  Json,
  Text,
  Bytes,
  Form,
  Xml,
  Multipart,
  Empty,
}

/// What an interaction declares beside its parts: its slots' content types and the components it
/// needs, and the recorded body values that replace the generic ones.
#[derive(Default)]
struct Declarations {
  content_types: ContentTypes,
  requires: Vec<Requirement>,
  evidence: BTreeMap<String, SlotValue>,
}

impl Declarations {
  fn require(&mut self, component: &str) {
    if !self.requires.iter().any(|r| r.component == component) {
      self.requires.push(Requirement {
        component: component.to_string(),
        min_version: None,
      });
    }
  }

  fn body(&mut self, part: &str, body: &ConvertedBody) {
    if let Some(declared) = &body.declared {
      self
        .content_types
        .entry(part.to_string())
        .or_default()
        .insert("body".to_string(), declared.clone());
    }
    if let Some(component) = body.component {
      self.require(&format!("content/{component}"));
    }
    if let Some(evidence) = &body.evidence {
      self.evidence.insert(part.to_string(), evidence.clone());
    }
  }

  fn record_evidence(&self, values: &mut Parts) {
    for (part, evidence) in &self.evidence {
      if let Some(slots) = values.get_mut(part) {
        slots.insert("body".to_string(), evidence.clone());
      }
    }
  }
}

/// A body's octets as recorded evidence (contract-file spec §5.3): text when they are UTF-8, so a
/// reader sees the pact's example, base64 otherwise, with the pact's type. A verifier sends either
/// as it stands, so the provider gets the bytes the pact's verification sends.
fn octets(bytes: &[u8], content_type: &str) -> SlotValue {
  let (content, encoded) = match std::str::from_utf8(bytes) {
    Ok(text) => (Value::String(text.to_string()), "text"),
    Err(_) => (Value::String(BASE64.encode(bytes)), "base64"),
  };
  SlotValue {
    content,
    encoded: Some(encoded.to_string()),
    content_type: Some(content_type.to_string()),
  }
}

/// A multipart type without its `boundary`: the boundary is the octets' own, and a reply's is read
/// from its own label (ADR 0047).
fn without_boundary(content_type: &str) -> String {
  content_type
    .split(';')
    .map(str::trim)
    .filter(|segment| {
      !segment
        .split_once('=')
        .is_some_and(|(name, _)| name.trim().eq_ignore_ascii_case("boundary"))
    })
    .collect::<Vec<_>>()
    .join("; ")
}

const OPENED_MEMBERS: &str = "members";
const OPENED_NAMES: &str = "names";
const OPENED_XML: &str = "attributes and child elements";
const OPENED_CHILDREN: &str = "child elements";

/// Whether a rule only asks for a value of the example's kind: on text, any text.
fn is_type_rule(rule: &MatchingRule) -> bool {
  matches!(
    rule,
    MatchingRule::Type | MatchingRule::MinType(_) | MatchingRule::MaxType(_) | MatchingRule::MinMaxType(_, _)
  )
}

/// Text an unsigned 64-bit integer parses from, as `legacy:parses-as` reads `integer`: digits, with an
/// optional `+`. Larger values are admitted too.
const INTEGER_TEXT: &str = r"^\+?[0-9]+$";

/// Text a 64-bit float parses from, as `legacy:parses-as` reads `number` and `decimal`: Rust's
/// `f64::from_str` grammar.
const NUMBER_TEXT: &str =
  r"^[+-]?(?:(?i:inf|infinity|nan)|(?:[0-9]+\.?[0-9]*|\.[0-9]+)(?:[eE][+-]?[0-9]+)?)$";

const BOOLEAN_TEXT: &str = "^(?:true|false)$";

/// The pattern a number or boolean rule on text becomes (`legacy:parses-as`).
fn text_pattern(rule: &MatchingRule) -> Option<&'static str> {
  match rule {
    MatchingRule::Integer => Some(INTEGER_TEXT),
    MatchingRule::Number | MatchingRule::Decimal => Some(NUMBER_TEXT),
    MatchingRule::Boolean => Some(BOOLEAN_TEXT),
    _ => None,
  }
}

/// A pattern for a `Content-Type` that `legacy:body-type` finds agrees with `expected`: the same type
/// and subtype, in any case, or with no parameters in `expected`, a suffix type whose suffix is its
/// subtype (`application/hal+json` for `application/json`), and any parameters; with parameters in
/// `expected`, exactly those, in any order, the names in any case. A label with no `/`, or a wildcard,
/// also agrees, and the pattern does not admit it. `None` when every label agrees.
fn media_type_pattern(expected: &str) -> Option<String> {
  let mut segments = expected.split(';').map(str::trim);
  let base = segments.next()?.to_ascii_lowercase();
  let (main, sub) = base.split_once('/')?;
  if main == "*" || sub == "*" {
    return None;
  }
  let mut params: Vec<(String, String)> = segments
    .filter_map(|segment| segment.split_once('='))
    .map(|(name, value)| {
      (
        name.trim().to_ascii_lowercase(),
        value.trim().trim_matches('"').to_string(),
      )
    })
    .collect();
  let start = format!(r"^\s*(?i:{})/", regex::escape(main));
  let sub_pattern = format!("(?i:{})", regex::escape(sub));
  if params.is_empty() {
    let suffix = if sub.contains('+') { "" } else { r"(?:[^;]*\+)?" };
    return Some(format!(r"{start}{suffix}{sub_pattern}\s*(?:;.*)?$"));
  }
  params.sort();
  // A segment with no `=` is no parameter, and is ignored.
  let other = r"(?:;[^;=]*)*";
  let orders = if params.len() <= 4 {
    permutations(&params)
  } else {
    vec![params.clone()]
  };
  let mut alternatives: Vec<String> = orders
    .iter()
    .map(|order| {
      order
        .iter()
        .map(|(name, value)| {
          format!(
            r#";\s*(?i:{})\s*=\s*"*{}"*\s*{other}"#,
            regex::escape(name),
            regex::escape(value)
          )
        })
        .collect::<String>()
    })
    .collect();
  alternatives.dedup();
  let alternatives = if alternatives.len() == 1 {
    alternatives.remove(0)
  } else {
    format!("(?:{})", alternatives.join("|"))
  };
  Some(format!(r"{start}{sub_pattern}\s*{other}{alternatives}$"))
}

/// Every order of `items`, in order of the first item, then the second.
fn permutations(items: &[(String, String)]) -> Vec<Vec<(String, String)>> {
  if items.len() <= 1 {
    return vec![items.to_vec()];
  }
  let mut orders = Vec::new();
  for (index, item) in items.iter().enumerate() {
    let mut rest = items.to_vec();
    rest.remove(index);
    for mut order in permutations(&rest) {
      order.insert(0, item.clone());
      if !orders.contains(&order) {
        orders.push(order);
      }
    }
  }
  orders
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
    // Anchored, so the contract keeps the pact's verdicts: a pact's regex matches the whole value,
    // a shape's matches anywhere (S1.18).
    MatchingRule::Regex(pattern) => json!({
      "shape": "regex",
      "pattern": anchored_pattern(pattern),
      "example": example
    }),
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

#[cfg(test)]
mod tests {
  use super::*;
  use crate::plan::{CapturedValues, Node, Plan, Status};

  fn passes(node: Node) -> bool {
    let plan = Plan {
      grammar: plan::GRAMMAR_VERSION,
      root: Node::container(None, vec![node]),
      variant: None,
    };
    plan::outcome(&plan::execute(&plan, &CapturedValues::new())).0 == Status::Matched
  }

  fn matches(pattern: &str, text: &str) -> bool {
    regex::Regex::new(pattern)
      .expect("a valid pattern")
      .is_match(text)
  }

  /// The text patterns accept what `legacy:parses-as` does, an integer too large for 64 bits aside.
  #[test]
  fn a_text_pattern_accepts_what_the_text_parses_as() {
    for text in [
      "0",
      "17",
      "+17",
      "-17",
      "007",
      "1.5",
      "1.",
      ".5",
      ".",
      "-.5",
      "1e5",
      "1E-5",
      "1e",
      "e5",
      "inf",
      "-Infinity",
      "NaN",
      "nan1",
      "1_000",
      " 1",
      "1 ",
      "",
      "0x1f",
      "1,5",
      "+",
      "true",
      "True",
      "false",
    ] {
      let parses = |kind: &str| {
        passes(Node::action(
          "legacy:parses-as",
          vec![
            Node::value(Literal::string(text)),
            Node::value(Literal::string(kind)),
          ],
        ))
      };
      assert_eq!(
        matches(INTEGER_TEXT, text),
        parses("integer"),
        "integer: {text:?}"
      );
      assert_eq!(matches(NUMBER_TEXT, text), parses("number"), "number: {text:?}");
      assert_eq!(
        matches(BOOLEAN_TEXT, text),
        parses("boolean"),
        "boolean: {text:?}"
      );
    }
    assert!(
      matches(INTEGER_TEXT, "99999999999999999999"),
      "the one difference"
    );
  }

  /// The `Content-Type` pattern admits what `legacy:body-type` does for every label that is a type
  /// with no wildcard; the rest it refuses, and `legacy:body-type` admits.
  #[test]
  fn a_body_type_pattern_admits_what_the_body_type_check_does() {
    let mains = ["text", "TEXT", "application", "*", ""];
    let subs = [
      "plain",
      "Plain",
      "vnd.x+plain",
      "plain+x",
      "hal+json",
      "json",
      "*",
      "plainer",
    ];
    let params = [
      "",
      "; charset=utf-8",
      ";charset=\"utf-8\"",
      "; CHARSET=utf-8 ",
      "; charset=UTF-8",
      "; a=1; charset=utf-8",
      "; charset=utf-8; a=1",
      "; junk",
      "; charset = utf-8",
      "; charset=utf-8; charset=utf-8",
      ";",
    ];
    let expected = [
      "text/plain",
      "text/plain;charset=utf-8",
      "application/hal+json",
      "application/json",
      "text/plain;charset=utf-8;a=1",
    ];
    for expected in expected {
      let pattern = media_type_pattern(expected).expect("a type with no wildcard has a pattern");
      for main in mains {
        for sub in subs {
          for param in params {
            for (lead, trail) in [("", ""), ("  ", " ")] {
              let label = if main.is_empty() {
                format!("{lead}{sub}{trail}{param}")
              } else {
                format!("{lead}{main}/{sub}{trail}{param}")
              };
              let agrees = passes(Node::action(
                "legacy:body-type",
                vec![
                  Node::value(Literal::string(&label)),
                  Node::value(Literal::string(expected)),
                ],
              ));
              let base = label.split(';').next().unwrap_or_default().trim();
              let typed = base.split_once('/').is_some_and(|(m, s)| m != "*" && s != "*");
              if typed {
                assert_eq!(matches(&pattern, &label), agrees, "{expected} against {label:?}");
              } else {
                assert!(
                  agrees && !matches(&pattern, &label),
                  "{expected} against {label:?}"
                );
              }
            }
          }
        }
      }
    }
    assert_eq!(media_type_pattern("*/*"), None);
    assert_eq!(media_type_pattern("text/*"), None);
  }
}
