//! The provider-shape session (plan task 7.2, engine-protocol spec §8.5): the recorder
//! ([`crate::subsumption::Recorder`]) as an engine mode, so that a provider's own test suite can
//! record a shape in whatever language it is written in.
//!
//! It is a third session kind rather than an extension of an existing one, which is what spec §7.3
//! requires ("new session kinds arrive as new operations plus capabilities, not as changes to
//! existing ones") and what the shape of the work wants anyway: a consumer session accumulates
//! *evidence for a contract* and a verification session *replays* one, while this accumulates
//! evidence about a provider and belongs to no consumer at all (design 2.8 §2.1).
//!
//! The engine does no HTTP here and holds no opinion about how the provider was called. A host
//! observes *decoded values*, exactly as a contract records them (contract spec §5.3's `content`),
//! which keeps the architecture rule intact: the kernel does not know what a response is, only
//! what a value is.

use crate::contract::Party;
use crate::subsumption::{ProviderShape, Recorder, RecordingPolicy};
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Deserialize)]
pub struct Create {
  pub provider: Party,
  /// Overrides the recorder's own judgements (design 2.8 §2.3's provenance is fixed at
  /// `recorded` — this is not that).
  #[serde(default)]
  pub policy: Option<Policy>,
}

/// The two recording judgements, as a host may set them (`Recorder`'s module docs). Both optional:
/// a host that passes neither gets the defaults, which is what a provider's test suite normally
/// wants.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Policy {
  #[serde(default)]
  pub max_options: Option<usize>,
  #[serde(default)]
  pub min_evidence: Option<u64>,
}

impl Policy {
  pub fn resolve(&self) -> RecordingPolicy {
    let defaults = RecordingPolicy::default();
    RecordingPolicy {
      max_options: self.max_options.unwrap_or(defaults.max_options),
      min_evidence: self.min_evidence.unwrap_or(defaults.min_evidence),
    }
  }
}

#[derive(Debug, Deserialize)]
pub struct Observe {
  pub session: String,
  pub description: String,
  /// State *names* only (design 2.8 §2.2): a provider knows its own state vocabulary, never a
  /// given consumer's parameter values.
  #[serde(default)]
  pub states: Vec<String>,
  /// Part name -> slot name -> the value that slot carried, wrapped exactly as a contract wraps
  /// one (contract spec §5.3) so a host that already builds those has nothing new to learn.
  pub parts: BTreeMap<String, BTreeMap<String, SlotValue>>,
}

/// One slot's observed value, wrapped as contract spec §5.3 wraps one.
///
/// `content` may be any JSON value, `null` included — a slot that carried nothing is left out of
/// the map rather than given a special value, which is that section's own rule. A wrapper's other
/// members (`encoded`, `content-type`) are accepted and ignored under the protocol's must-ignore
/// rule (spec §2.2): a byte payload arrives as the base64 *string* its tag describes (spec §2.5),
/// and a string is what the recorder records — an octet sequence is one value, not a structure to
/// walk into.
#[derive(Debug, Deserialize)]
pub struct SlotValue {
  pub content: Value,
}

#[derive(Debug, Deserialize)]
pub struct Finalise {
  pub session: String,
}

/// One recording session: a [`Recorder`] and the counter a host sees back from `observe`.
#[derive(Debug)]
pub struct Session {
  recorder: Recorder,
  observations: u64,
}

impl Session {
  pub fn new(provider: Party, policy: RecordingPolicy) -> Session {
    Session {
      recorder: Recorder::with_policy(provider.name, policy),
      observations: 0,
    }
  }

  pub fn observe(&mut self, request: &Observe) {
    let parts = request
      .parts
      .iter()
      .map(|(part, slots)| {
        let values = slots
          .iter()
          .map(|(slot, value)| (slot.clone(), value.content.clone()))
          .collect();
        (part.clone(), values)
      })
      .collect();
    self
      .recorder
      .observe(&request.description, &request.states, &parts);
    self.observations += 1;
  }

  /// What this session has accumulated so far, for the `observe` result: enough for a host to
  /// report "recorded 24 responses across 6 operations" without holding its own counters.
  pub fn progress(&self) -> (u64, usize) {
    (self.observations, self.recorder.interaction_count())
  }

  pub fn finish(self) -> ProviderShape {
    self.recorder.finish()
  }
}
