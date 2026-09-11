//! Consumer sessions (engine-protocol spec §7, §8.2): the only resource a host can hold.
//! Everything a session allocates — here, its interactions — is released when the session ends;
//! there is no per-object cleanup call (spec §7.1).

use crate::contract::Party;
use crate::interaction_spec::{self, InteractionSpec, InteractionSpecError};
use crate::plan::{self, Assignment, Plan};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};

/// One interaction as validated and compiled by `add-interaction`. Spec §8.2: "the engine
/// validates it and compiles what it needs" — `plan::compile` is infallible once `spec` parsed
/// (matching happens at execution, not compile time), so this always succeeds once parsing does.
/// The compiled `plan` isn't consumed by anything in 4.1; it's here so 4.2/4.3 (transport,
/// variants) have it without re-deriving session state layout.
pub(crate) struct InteractionEntry {
  #[allow(dead_code)]
  pub spec: InteractionSpec,
  #[allow(dead_code)]
  pub plan: Plan,
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
  interactions: BTreeMap<String, InteractionEntry>,
  order: Vec<String>,
  next_handle: u64,
}

impl ConsumerSession {
  fn new(consumer: Party, provider: Party) -> Self {
    ConsumerSession {
      consumer,
      provider,
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
    self
      .interactions
      .insert(handle.clone(), InteractionEntry { spec, plan: compiled });
    self.order.push(handle.clone());
    Ok(handle)
  }

  /// `consumer-session/finalise`'s `results` (spec §8.2): one entry per interaction, in
  /// submission order. Every 4.1-era interaction is `not-exercised` — no `serve-variant` exists
  /// yet to have exercised any of them (4.3). Honest, not a placeholder: this is exactly what
  /// the spec says an unexercised interaction reports.
  pub fn results(&self) -> Vec<Value> {
    self
      .order
      .iter()
      .map(|handle| serde_json::json!({ "handle": handle, "status": "not-exercised" }))
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
  pub fn create(&mut self, consumer: Party, provider: Party) -> String {
    self.next_session += 1;
    let id = format!("cs-{}", self.next_session);
    self
      .sessions
      .insert(id.clone(), ConsumerSession::new(consumer, provider));
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
    let a = store.create(party("web-app"), party("order-api"));
    let b = store.create(party("web-app"), party("order-api"));
    assert_ne!(a, b);
    assert!(store.get_mut(&a).is_some());
    assert!(store.get_mut(&b).is_some());
  }

  #[test]
  fn handles_are_allocated_in_submission_order() {
    let mut session = ConsumerSession::new(party("web-app"), party("order-api"));
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
    let id = store.create(party("web-app"), party("order-api"));
    assert!(store.end(&id).is_some());
    assert!(store.get_mut(&id).is_none());
    assert!(store.end(&id).is_none());
  }
}
