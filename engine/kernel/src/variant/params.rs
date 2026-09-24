//! Variant-bound provider state — the RFC's `whenVariant`, designed in variant-semantics spec §6
//! and settled by [ADR 0009](../../../../Documentation/decisions/0009-variant-bound-provider-state-parameters.md)
//! (plan task 5.2).
//!
//! A state parameter whose value depends on which variant is running, so the verifier can put the
//! provider into the state each variant needs: `shipped: true` for the variant where `shippedAt`
//! is present, `false` where it is absent. Three things make it work, and each is a decision
//! rather than a mechanism:
//!
//! - **The binding sits beside the parameters, never inside a value** (§6.2). A parameter value is
//!   user data, so an in-band marker would be ambiguous against a user whose data happens to look
//!   like one.
//! - **A `dimension` is a reference, resolved at `add-interaction`** (§6.3) — while the author is
//!   looking at the DSL that produced it — and what gets *recorded* is the resolved id. A verifier
//!   reading a contract resolves nothing and therefore cannot resolve it differently.
//! - **Resolution is a total function of the assignment and the binding** (§6.4), so the consumer
//!   resolving at record time and the verifier resolving at replay time agree structurally rather
//!   than by trust — and because the resolved values are recorded too, a disagreement is visible
//!   immediately instead of becoming a mystery in someone's CI.

use super::{RefResolution, resolve_dimension_ref};
use crate::common::State;
use crate::contract::ResolvedState;
use crate::error::Problem;
use crate::plan::Assignment;
use crate::shape::variant_space::VariantSpace;
use serde_json::Value;
use std::collections::BTreeMap;

/// One parameter bound to a dimension (`variant-params.schema.json`'s `Binding`).
#[derive(Debug, Clone, PartialEq)]
pub struct Binding {
  pub name: String,
  /// The reference exactly as the author wrote it — kept for messages, never for lookups.
  pub reference: String,
  /// The dimension id the reference resolved to: what §6.3 says is recorded.
  pub dimension: String,
  pub cases: Vec<Case>,
  /// `None` means the binding has no `default` — which is not the same as a default of `null`
  /// (`Some(Value::Null)`), and the difference is exactly "absent" versus "null" in §6.4.
  pub default: Option<Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Case {
  pub point: String,
  pub value: Value,
}

/// Bind and validate every state's `variant-params` against the interaction's variant space
/// (§6.5). Index-aligned with `states`: entry `i` is state `i`'s bindings, empty when it has none.
///
/// `pointer_base` is the JSON pointer of the state array in the document being validated
/// (`"/states"` in an interaction specification), so every problem locates itself in the document
/// the author actually wrote.
pub fn bind(
  states: &[State],
  space: &VariantSpace,
  pointer_base: &str,
) -> Result<Vec<Vec<Binding>>, Vec<Problem>> {
  let mut problems = Vec::new();
  let mut bound = Vec::with_capacity(states.len());

  for (i, state) in states.iter().enumerate() {
    let mut bindings = Vec::new();
    let raw = match &state.variant_params {
      Some(raw) => raw.as_slice(),
      None => &[],
    };
    for (j, entry) in raw.iter().enumerate() {
      let at = format!("{pointer_base}/{i}/variant-params/{j}");
      match bind_one(entry, state, space, &at, &mut problems) {
        Some(binding) => bindings.push(binding),
        None => continue,
      }
    }
    bound.push(bindings);
  }

  if problems.is_empty() {
    Ok(bound)
  } else {
    Err(problems)
  }
}

/// `states` with every binding's `dimension` replaced by the id it resolved to (§6.3: "what is
/// recorded is the resolved id"). `bound` is [`bind`]'s successful result for the same states, so
/// binding `j` of state `i` is entry `j` of its `variant-params`; everything else in an entry —
/// `name`, `cases`, `default`, members this version does not know — is kept as written.
pub fn with_resolved_dimensions(states: &[State], bound: &[Vec<Binding>]) -> Vec<State> {
  states
    .iter()
    .zip(bound)
    .map(|(state, bindings)| {
      let mut state = state.clone();
      if let Some(entries) = state.variant_params.as_mut() {
        for (entry, binding) in entries.iter_mut().zip(bindings) {
          if let Some(object) = entry.as_object_mut() {
            object.insert("dimension".to_string(), Value::String(binding.dimension.clone()));
          }
        }
      }
      state
    })
    .collect()
}

fn bind_one(
  entry: &Value,
  state: &State,
  space: &VariantSpace,
  at: &str,
  problems: &mut Vec<Problem>,
) -> Option<Binding> {
  let Some(object) = entry.as_object() else {
    problems.push(problem(at, "a variant-params entry must be an object"));
    return None;
  };

  let name = match object.get("name").and_then(Value::as_str) {
    Some(name) if !name.is_empty() => name.to_string(),
    _ => {
      problems.push(problem(
        &format!("{at}/name"),
        "'name' is required and must be a non-empty string",
      ));
      return None;
    }
  };
  let reference = match object.get("dimension").and_then(Value::as_str) {
    Some(reference) if !reference.is_empty() => reference.to_string(),
    _ => {
      problems.push(problem(
        &format!("{at}/dimension"),
        "'dimension' is required and must be a non-empty string",
      ));
      return None;
    }
  };

  // §6.5, last row: a value that is sometimes literal and sometimes computed is a debugging
  // problem nobody needs.
  if state
    .params
    .as_ref()
    .is_some_and(|params| params.contains_key(&name))
  {
    problems.push(problem(
      &format!("{at}/name"),
      format!("parameter '{name}' is both a literal parameter and a variant-bound one"),
    ));
  }

  let dimension = match resolve_dimension_ref(space, &reference) {
    RefResolution::Found(dimension) => dimension,
    RefResolution::NotFound => {
      let known: Vec<&str> = space.dimensions.iter().map(|d| d.id.as_str()).collect();
      problems.push(problem(
        &format!("{at}/dimension"),
        format!(
          "'{reference}' matches no dimension of this interaction; it has {}",
          if known.is_empty() {
            "none".to_string()
          } else {
            known.join(", ")
          }
        ),
      ));
      return None;
    }
    RefResolution::Ambiguous(candidates) => {
      problems.push(problem(
        &format!("{at}/dimension"),
        format!(
          "'{reference}' matches more than one dimension: {} — name one of them",
          candidates.join(", ")
        ),
      ));
      return None;
    }
  };

  let mut cases = Vec::new();
  match object.get("cases") {
    Some(Value::Array(raw_cases)) if !raw_cases.is_empty() => {
      for (k, raw_case) in raw_cases.iter().enumerate() {
        let case_at = format!("{at}/cases/{k}");
        let point = raw_case.get("point").and_then(Value::as_str);
        let value = raw_case.get("value");
        match (point, value) {
          (Some(point), Some(value)) if !point.is_empty() => {
            // §6.5, third row: `whenVariant('shippedAt', 'set')` would silently never fire.
            if !dimension.points.iter().any(|p| p.name == point) {
              let points: Vec<&str> = dimension.points.iter().map(|p| p.name.as_str()).collect();
              problems.push(problem(
                &format!("{case_at}/point"),
                format!(
                  "dimension '{}' has no point '{point}'; its points are {}",
                  dimension.id,
                  points.join(", ")
                ),
              ));
              continue;
            }
            cases.push(Case {
              point: point.to_string(),
              value: value.clone(),
            });
          }
          _ => problems.push(problem(
            &case_at,
            "a case must have a non-empty 'point' and a 'value'",
          )),
        }
      }
    }
    _ => problems.push(problem(
      &format!("{at}/cases"),
      "'cases' is required and must be a non-empty array",
    )),
  }

  // `contains_key` rather than a parsed `Option`, because `"default": null` is a real default of
  // null and serde would collapse it into "no default" (§6.4's absent-is-not-null).
  let default = object.get("default").cloned();

  // §6.5, fourth row: a gated dimension is inactive in some variants by construction, so the
  // fallback is not hypothetical — without a default the parameter would vanish for part of the
  // run, and nobody would see it happen.
  if !dimension.gated_by.is_empty() && default.is_none() {
    let gates: Vec<String> = dimension
      .gated_by
      .iter()
      .map(|gate| format!("{}={}", gate.dimension, gate.point))
      .collect();
    problems.push(problem(
      at,
      format!(
        "dimension '{}' is gated ({}), so this binding needs a 'default' for the variants where it is inactive",
        dimension.id,
        gates.join(", ")
      ),
    ));
  }

  Some(Binding {
    name,
    reference,
    dimension: dimension.id.clone(),
    cases,
    default,
  })
}

fn problem(pointer: &str, message: impl Into<String>) -> Problem {
  Problem {
    pointer: pointer.to_string(),
    message: message.into(),
  }
}

/// Resolve one binding for one variant (§6.4). `None` is **absent** — not null, not empty: a state
/// handler that treats a missing parameter as a default gets to keep doing so.
pub fn resolve_binding(binding: &Binding, assignment: &Assignment) -> Option<Value> {
  if let Some(point) = assignment.get(&binding.dimension)
    && let Some(case) = binding.cases.iter().find(|case| &case.point == point)
  {
    return Some(case.value.clone());
  }
  binding.default.clone()
}

/// Every state of an interaction, resolved for one variant (§6.4, contract-file spec §6): literal
/// parameters as they stand, bound parameters resolved, and nothing else — a `ResolvedState`
/// carries values only, never bindings, because a verifier must not have to re-derive them.
///
/// Bindings that do not validate are skipped with a warning rather than failing here: validation
/// happens at `add-interaction` ([`bind`]), where the author can act on it, and a contract that
/// arrived from elsewhere carrying a binding this engine cannot read is better verified with the
/// literal parameters than not verified at all.
pub fn resolve_states(
  states: Option<&Vec<State>>,
  space: &VariantSpace,
  assignment: &Assignment,
) -> Option<Vec<ResolvedState>> {
  let states = states?;
  let bound = bind(states, space, "/states").unwrap_or_else(|problems| {
    tracing::warn!(
      ?problems,
      "variant-param bindings did not validate; resolving literals only"
    );
    vec![Vec::new(); states.len()]
  });

  Some(
    states
      .iter()
      .zip(bound.iter())
      .map(|(state, bindings)| {
        let mut params: BTreeMap<String, Value> = state.params.clone().unwrap_or_default();
        for binding in bindings {
          match resolve_binding(binding, assignment) {
            Some(value) => {
              params.insert(binding.name.clone(), value);
            }
            None => {
              params.remove(&binding.name);
            }
          }
        }
        ResolvedState {
          name: state.name.clone(),
          params: (!params.is_empty()).then_some(params),
        }
      })
      .collect(),
  )
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::interaction_spec;
  use crate::plan;
  use serde_json::json;

  /// An interaction whose response body has one `optional` member — so exactly one dimension,
  /// `response.body.shippedAt#presence`, with points `present`/`absent`.
  fn space_with_an_optional_member() -> VariantSpace {
    let spec = interaction_spec::parse(&json!({
      "description": "a request for an order",
      "parts": {
        "response": { "body": { "shape": "object", "members": {
          "shippedAt": { "shape": "optional", "of": { "shape": "string", "example": "2026-07-30" } } } } }
      }
    }))
    .expect("the fixture parses");
    plan::variant_space(&spec)
  }

  fn state(params: Option<Value>, variant_params: Option<Value>) -> State {
    let mut document = json!({ "name": "an order exists" });
    let object = document.as_object_mut().unwrap();
    if let Some(params) = params {
      object.insert("params".to_string(), params);
    }
    if let Some(variant_params) = variant_params {
      object.insert("variant-params".to_string(), variant_params);
    }
    serde_json::from_value(document).expect("a State fixture")
  }

  fn shipped_binding(dimension: &str) -> Value {
    json!([{ "name": "shipped", "dimension": dimension,
             "cases": [ { "point": "present", "value": true },
                        { "point": "absent", "value": false } ] }])
  }

  fn assignment(entries: &[(&str, &str)]) -> Assignment {
    entries
      .iter()
      .map(|(dim, point)| (dim.to_string(), point.to_string()))
      .collect()
  }

  #[test]
  fn a_reference_resolves_by_id_by_path_or_by_trailing_segments() {
    let space = space_with_an_optional_member();
    for reference in [
      "response.body.shippedAt#presence",
      "response.body.shippedAt",
      "body.shippedAt",
      "shippedAt",
    ] {
      let states = [state(None, Some(shipped_binding(reference)))];
      let bound = bind(&states, &space, "/states").unwrap_or_else(|problems| {
        panic!("'{reference}' should resolve, got {problems:?}");
      });
      assert_eq!(
        bound[0][0].dimension, "response.body.shippedAt#presence",
        "what is recorded is the resolved id, never the reference (§6.3)"
      );
    }
  }

  #[test]
  fn a_reference_matching_nothing_is_rejected_with_the_dimensions_that_exist() {
    let space = space_with_an_optional_member();
    let states = [state(None, Some(shipped_binding("shipedAt")))];
    let problems = bind(&states, &space, "/states").expect_err("a typo is caught here");
    assert_eq!(problems[0].pointer, "/states/0/variant-params/0/dimension");
    assert!(
      problems[0].message.contains("response.body.shippedAt#presence"),
      "the message names what it could have meant: {}",
      problems[0].message
    );
  }

  #[test]
  fn an_ambiguous_reference_is_rejected_with_its_candidates() {
    // The same member name in two parts: `shippedAt` alone names both, and which one the author
    // meant is a genuine question only they can answer (§6.3).
    let spec = interaction_spec::parse(&json!({
      "description": "an order round trip",
      "parts": {
        "request": { "body": { "shape": "object", "members": {
          "shippedAt": { "shape": "optional", "of": { "shape": "string", "example": "2026-07-30" } } } } },
        "response": { "body": { "shape": "object", "members": {
          "shippedAt": { "shape": "optional", "of": { "shape": "string", "example": "2026-07-30" } } } } }
      }
    }))
    .expect("the fixture parses");
    let space = plan::variant_space(&spec);

    let states = [state(None, Some(shipped_binding("shippedAt")))];
    let problems = bind(&states, &space, "/states").expect_err("'shippedAt' names two dimensions");
    assert_eq!(problems[0].pointer, "/states/0/variant-params/0/dimension");
    assert!(
      problems[0].message.contains("request.body.shippedAt#presence")
        && problems[0].message.contains("response.body.shippedAt#presence"),
      "both candidates are listed: {}",
      problems[0].message
    );

    // Qualifying it settles it, with no change to the shape.
    let states = [state(None, Some(shipped_binding("response.body.shippedAt")))];
    bind(&states, &space, "/states").expect("a qualified reference resolves");
  }

  #[test]
  fn a_point_the_dimension_does_not_have_is_rejected() {
    let space = space_with_an_optional_member();
    let states = [state(
      None,
      Some(json!([{ "name": "shipped", "dimension": "shippedAt",
                    "cases": [ { "point": "set", "value": true } ] }])),
    )];
    let problems = bind(&states, &space, "/states").expect_err("'set' is not a presence point");
    assert_eq!(problems[0].pointer, "/states/0/variant-params/0/cases/0/point");
    assert!(
      problems[0].message.contains("present"),
      "the message lists the points it does have: {}",
      problems[0].message
    );
  }

  #[test]
  fn a_parameter_cannot_be_both_literal_and_bound() {
    let space = space_with_an_optional_member();
    let states = [state(
      Some(json!({ "shipped": true })),
      Some(shipped_binding("shippedAt")),
    )];
    let problems = bind(&states, &space, "/states").expect_err("§6.2 forbids it");
    assert_eq!(problems[0].pointer, "/states/0/variant-params/0/name");
  }

  #[test]
  fn a_binding_on_a_gated_dimension_needs_a_default() {
    // A `one-of` gates the dimensions inside each alternative: `dueDate#presence` is inactive in
    // every `card` variant, so a binding on it without a default would silently vanish for half
    // the run (§6.5's fourth row).
    let spec = interaction_spec::parse(&json!({
      "description": "a payment",
      "parts": { "request": { "body": {
        "shape": "one-of", "discriminator": "method",
        "alternatives": {
          "card": { "shape": "object", "members": {
            "method": { "shape": "equality", "example": "card" } } },
          "invoice": { "shape": "object", "members": {
            "method": { "shape": "equality", "example": "invoice" },
            "dueDate": { "shape": "optional", "of": { "shape": "string", "example": "2026-01-01" } } } }
        } } } }
    }))
    .expect("the fixture parses");
    let space = plan::variant_space(&spec);
    let gated = space
      .dimensions
      .iter()
      .find(|d| d.id.contains("dueDate"))
      .expect("the alternative's optional member contributes a dimension");
    assert!(!gated.gated_by.is_empty(), "and it is gated: {gated:?}");

    let binding = json!([{ "name": "due", "dimension": gated.id,
                           "cases": [ { "point": "present", "value": true } ] }]);
    let states = [state(None, Some(binding.clone()))];
    let problems = bind(&states, &space, "/states").expect_err("a gated binding needs a default");
    assert_eq!(problems[0].pointer, "/states/0/variant-params/0");

    let mut with_default = binding;
    with_default[0]
      .as_object_mut()
      .unwrap()
      .insert("default".to_string(), json!(false));
    let states = [state(None, Some(with_default))];
    bind(&states, &space, "/states").expect("with a default it is fine");
  }

  #[test]
  fn resolution_takes_the_case_the_variants_point_names() {
    let space = space_with_an_optional_member();
    let states = vec![state(None, Some(shipped_binding("shippedAt")))];

    let present = resolve_states(
      Some(&states),
      &space,
      &assignment(&[("response.body.shippedAt#presence", "present")]),
    )
    .expect("states resolve");
    assert_eq!(present[0].params.as_ref().unwrap()["shipped"], json!(true));

    let absent = resolve_states(
      Some(&states),
      &space,
      &assignment(&[("response.body.shippedAt#presence", "absent")]),
    )
    .expect("states resolve");
    assert_eq!(
      absent[0].params.as_ref().unwrap()["shipped"],
      json!(false),
      "the same binding, the other variant, the other value — that is the whole feature"
    );
  }

  #[test]
  fn a_parameter_with_no_matching_case_and_no_default_is_absent_not_null() {
    let space = space_with_an_optional_member();
    let states = vec![state(
      Some(json!({ "id": "66" })),
      Some(json!([{ "name": "shipped", "dimension": "shippedAt",
                    "cases": [ { "point": "present", "value": true } ] }])),
    )];
    let resolved = resolve_states(
      Some(&states),
      &space,
      &assignment(&[("response.body.shippedAt#presence", "absent")]),
    )
    .expect("states resolve");
    let params = resolved[0].params.as_ref().expect("the literal param survives");
    assert_eq!(params["id"], json!("66"));
    assert!(
      !params.contains_key("shipped"),
      "absent, so a handler that defaults a missing parameter keeps working: {params:?}"
    );
  }

  #[test]
  fn a_default_of_null_is_a_value_not_an_absence() {
    let space = space_with_an_optional_member();
    let states = vec![state(
      None,
      Some(
        json!([{ "name": "shipped", "dimension": "shippedAt", "default": null,
                    "cases": [ { "point": "present", "value": true } ] }]),
      ),
    )];
    let resolved = resolve_states(
      Some(&states),
      &space,
      &assignment(&[("response.body.shippedAt#presence", "absent")]),
    )
    .expect("states resolve");
    assert_eq!(resolved[0].params.as_ref().unwrap()["shipped"], json!(null));
  }

  #[test]
  fn an_inactive_dimension_falls_back_to_the_default() {
    let space = space_with_an_optional_member();
    let states = vec![state(
      None,
      Some(
        json!([{ "name": "shipped", "dimension": "shippedAt", "default": "unknown",
                    "cases": [ { "point": "present", "value": true } ] }]),
      ),
    )];
    // An assignment mentioning no dimension at all: nothing is active, so the default applies.
    let resolved = resolve_states(Some(&states), &space, &Assignment::new()).expect("states resolve");
    assert_eq!(resolved[0].params.as_ref().unwrap()["shipped"], json!("unknown"));
  }

  #[test]
  fn a_state_with_no_parameters_at_all_resolves_to_none() {
    let space = space_with_an_optional_member();
    let states = vec![state(None, None)];
    let resolved = resolve_states(Some(&states), &space, &Assignment::new()).expect("states resolve");
    assert_eq!(resolved[0].name, "an order exists");
    assert_eq!(resolved[0].params, None, "params is omitted, not an empty object");
  }
}
