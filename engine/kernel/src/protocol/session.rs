//! Consumer sessions (engine-protocol spec §7, §8.2): the only resource a host can hold.
//! Everything a session allocates — here, its interactions — is released when the session ends;
//! there is no per-object cleanup call (spec §7.1).

use crate::contract::Party;
use crate::interaction_spec::{self, InteractionSpec, InteractionSpecError};
use crate::plan::{self, Assignment, Plan};
use crate::variant::{Selected, VariantError, generate, select};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};

/// The variant a `serve-variant` call has most recently armed for one interaction (variant-
/// semantics spec §4.1): its id, and the concrete payload the generator produced for it, ready
/// for whatever eventually drives a transport from it (no transport is bound to a consumer
/// session yet — `start-transport` isn't wired into [`super::engine`]'s dispatch).
pub struct Armed {
  #[allow(dead_code)]
  pub variant: String,
  #[allow(dead_code)]
  pub parts: BTreeMap<String, BTreeMap<String, Value>>,
}

/// One interaction as validated and compiled by `add-interaction`. Spec §8.2: "the engine
/// validates it and compiles what it needs" — `plan::compile` is infallible once `spec` parsed
/// (matching happens at execution, not compile time), so this always succeeds once parsing does.
/// The compiled `plan` isn't consumed by anything in 4.1; it's here so 4.2 (transport) has it
/// without re-deriving session state layout.
pub(crate) struct InteractionEntry {
  pub spec: InteractionSpec,
  #[allow(dead_code)]
  pub plan: Plan,
  /// The most recent selection computed for this interaction by `consumer-session/variants`
  /// (variant-semantics spec §3.9). `None` until a host calls it — a host that never does gets
  /// the honest, minimal `not-exercised` report 4.1's finalise already gave.
  pub selection: Option<Selected>,
  /// The variant `serve-variant` most recently armed (spec §4.1: at most one at a time; re-arming
  /// replaces it).
  pub armed: Option<Armed>,
}

/// `consumer-session/variants` (variant-semantics spec §3.9): a malformed policy vs. an unknown
/// handle are different failures at the protocol boundary.
pub enum VariantsError {
  HandleNotFound,
  Variant(VariantError),
}

pub enum ServeVariantError {
  HandleNotFound,
  VariantNotFound { selection: Vec<String> },
}

/// One `consumer-session/*` session: config plus the interactions added to it so far. `order`
/// tracks submission order explicitly — `i-{n}` handles are unpadded, so string-sorting them
/// (as a `BTreeMap` would) stops matching submission order past nine interactions.
pub(crate) struct ConsumerSession {
  // Unread until contract writing (4.4) builds the Janus contract's consumer/provider (design
  // 2.5) from them.
  #[allow(dead_code)]
  pub consumer: Party,
  #[allow(dead_code)]
  pub provider: Party,
  /// The session-wide sampling policy layer (variant-semantics spec §3.8 layer 2), from
  /// `create`'s `config.policy`.
  policy: Option<Value>,
  interactions: BTreeMap<String, InteractionEntry>,
  order: Vec<String>,
  next_handle: u64,
}

impl ConsumerSession {
  fn new(consumer: Party, provider: Party, policy: Option<Value>) -> Self {
    ConsumerSession {
      consumer,
      provider,
      policy,
      interactions: BTreeMap::new(),
      order: Vec::new(),
      next_handle: 1,
    }
  }

  /// `consumer-session/add-interaction` (spec §8.2): validate and compile, or a structured
  /// `InteractionSpecError` a caller maps to `interaction-invalid`.
  pub fn add_interaction(&mut self, interaction: &Value) -> Result<String, InteractionSpecError> {
    let spec = interaction_spec::parse(interaction)?;
    let compiled = plan::compile(&spec, &Assignment::new(), None);
    let handle = format!("i-{}", self.next_handle);
    self.next_handle += 1;
    self.interactions.insert(
      handle.clone(),
      InteractionEntry {
        spec,
        plan: compiled,
        selection: None,
        armed: None,
      },
    );
    self.order.push(handle.clone());
    Ok(handle)
  }

  /// `consumer-session/variants` (variant-semantics spec §3.9): resolve the layered policy
  /// (defaults, session config, this call's override — spec §3.8) against the interaction's
  /// variant space, run the selection algorithm, cache it as this interaction's current
  /// selection — what `serve-variant` and `finalise` read next — and return the result document
  /// (spec §3.9's schema: `variants`, each with its id/label/origin/assignment, plus `report`).
  pub fn variants(&mut self, handle: &str, call_policy: Option<&Value>) -> Result<Value, VariantsError> {
    let Some(entry) = self.interactions.get(handle) else {
      return Err(VariantsError::HandleNotFound);
    };
    let space = plan::variant_space(&entry.spec);
    let layers = [self.policy.as_ref(), call_policy];
    let policy = crate::variant::SamplingPolicy::resolve(&layers)
      .map_err(|problem| VariantsError::Variant(VariantError::InvalidPolicy(vec![problem])))?;
    let selected = select(&space, &policy).map_err(VariantsError::Variant)?;

    let variants: Vec<Value> = selected.variants.iter().map(|v| v.to_json(&space)).collect();
    let result = serde_json::json!({ "variants": variants, "report": selected.report.to_json() });

    let entry = self.interactions.get_mut(handle).expect("checked above");
    entry.selection = Some(selected);
    Ok(result)
  }

  /// `consumer-session/serve-variant` (spec §4.1, §8.2): arm the named variant of a previously
  /// selected interaction, generating its concrete payload now (plan task 4.3's generator). No
  /// transport is bound to a session yet, so arming stops at recording the variant and its
  /// payload — a future task wires this into an actual passive/emissive exchange.
  pub fn serve_variant(&mut self, handle: &str, variant_id: &str) -> Result<(), ServeVariantError> {
    let Some(entry) = self.interactions.get_mut(handle) else {
      return Err(ServeVariantError::HandleNotFound);
    };
    let assignment = {
      let Some(selection) = &entry.selection else {
        return Err(ServeVariantError::VariantNotFound {
          selection: Vec::new(),
        });
      };
      match selection.variants.iter().find(|v| v.id == variant_id) {
        Some(variant) => variant.assignment.clone(),
        None => {
          let selection = selection.variants.iter().map(|v| v.id.clone()).collect();
          return Err(ServeVariantError::VariantNotFound { selection });
        }
      }
    };
    let parts = generate::interaction(&entry.spec, &assignment);
    entry.armed = Some(Armed {
      variant: variant_id.to_string(),
      parts,
    });
    Ok(())
  }

  /// `consumer-session/finalise`'s `results` (spec §8.2): one entry per interaction, in
  /// submission order, with a `variants` breakdown once `variants` has been called for it
  /// (variant-semantics spec §4.2: every selected variant is required, and honestly
  /// `not-exercised` until something exercises it — no transport is wired up yet to do that).
  pub fn results(&self) -> Vec<Value> {
    self
      .order
      .iter()
      .map(|handle| {
        let entry = &self.interactions[handle];
        match &entry.selection {
          None => serde_json::json!({ "handle": handle, "status": "not-exercised" }),
          Some(selected) => {
            let variants: Vec<Value> = selected
              .variants
              .iter()
              .map(|v| serde_json::json!({ "variant": v.id, "status": "not-exercised" }))
              .collect();
            serde_json::json!({ "handle": handle, "status": "not-exercised", "variants": variants })
          }
        }
      })
      .collect()
  }
}

/// In-memory store of live consumer sessions, scoped to one `Engine` (one pipe, spec §7.1:
/// "session ids are engine-assigned opaque strings, unique within the life of the pipe").
#[derive(Default)]
pub(crate) struct SessionStore {
  sessions: HashMap<String, ConsumerSession>,
  next_session: u64,
}

impl SessionStore {
  /// `consumer-session/create` (spec §8.2). Returns the new session id.
  pub fn create(&mut self, consumer: Party, provider: Party, policy: Option<Value>) -> String {
    self.next_session += 1;
    let id = format!("cs-{}", self.next_session);
    self
      .sessions
      .insert(id.clone(), ConsumerSession::new(consumer, provider, policy));
    id
  }

  pub fn get_mut(&mut self, session: &str) -> Option<&mut ConsumerSession> {
    self.sessions.get_mut(session)
  }

  /// `consumer-session/finalise` (spec §8.2, §7.1): ends the session unconditionally. `None`
  /// means the id was already unknown or already ended — a caller maps that to
  /// `session-not-found`.
  pub fn end(&mut self, session: &str) -> Option<ConsumerSession> {
    self.sessions.remove(session)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn party(name: &str) -> Party {
    Party {
      name: name.to_string(),
    }
  }

  #[test]
  fn session_ids_are_distinct_and_stable() {
    let mut store = SessionStore::default();
    let a = store.create(party("web-app"), party("order-api"), None);
    let b = store.create(party("web-app"), party("order-api"), None);
    assert_ne!(a, b);
    assert!(store.get_mut(&a).is_some());
    assert!(store.get_mut(&b).is_some());
  }

  #[test]
  fn handles_are_allocated_in_submission_order() {
    let mut session = ConsumerSession::new(party("web-app"), party("order-api"), None);
    let spec = serde_json::json!({
      "description": "a request for an order",
      "parts": { "response": { "status": { "shape": "equality", "example": 200 } } }
    });
    let first = session.add_interaction(&spec).unwrap();
    let second = session.add_interaction(&spec).unwrap();
    assert_eq!(first, "i-1");
    assert_eq!(second, "i-2");
    assert_eq!(
      session.results(),
      vec![
        serde_json::json!({ "handle": "i-1", "status": "not-exercised" }),
        serde_json::json!({ "handle": "i-2", "status": "not-exercised" }),
      ]
    );
  }

  #[test]
  fn end_removes_the_session() {
    let mut store = SessionStore::default();
    let id = store.create(party("web-app"), party("order-api"), None);
    assert!(store.end(&id).is_some());
    assert!(store.get_mut(&id).is_none());
    assert!(store.end(&id).is_none());
  }
}
