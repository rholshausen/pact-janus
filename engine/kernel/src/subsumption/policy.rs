//! Policy: warn, block and exemptions (design 2.8 §7) — the half of design 2.8 the checker
//! deliberately does not do, and plan task 7.4 does.
//!
//! The split is the one [`super`]'s module docs describe: the walk ends at a report, and a
//! *policy* turns that report into a decision. Nothing here re-walks a tree or re-derives a
//! verdict — every finding already carries the `severity` it was computed with (§4.3), so
//! dispatch is a lookup, and an exemption is a selector match against the finding's own address.
//!
//! Two things in here are semantics the specification fixes rather than choices this module
//! makes, and both are easy to get subtly wrong:
//!
//! - **Layers accumulate exemptions, and override everything else** (§7.1) — the same treatment
//!   [`crate::variant::SamplingPolicy`] gives `pin`/`exclude`, implemented the same way: a layer
//!   is an all-optional patch, not a whole policy, so a per-run override that sets `on-finding`
//!   does not silently discard the project's accepted exemptions.
//! - **An unset selector matches every value on that axis** (§7.2), which means an exemption with
//!   no selectors at all matches every finding. That is what `{ "reason": … }` asks for, and the
//!   report says how many findings each exemption actually silenced so nobody has to guess.
//!
//! Dates are compared as text. `expires` is an RFC 3339 full-date (`YYYY-MM-DD`), and lexical
//! order on that format *is* chronological order — which is why this module needs no clock and no
//! calendar library, and why the kernel stays wasm-clean (there is no wall clock behind
//! `wasm32-wasip2` worth trusting anyway). The host supplies the date to judge against; a host
//! that supplies none gets exemptions applied and every `expires` reported unevaluated, because
//! guessing today's date is the one thing worse than not knowing it.

use crate::error::Problem;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// What a severity does to a run (§7.1). Deliberately two values: `pass` is not an action a
/// policy can choose, because a policy that could turn a decided incompatibility into silence
/// would make the report's own honesty unreachable — a team that has accepted one writes an
/// exemption, with a reason, which is the record §7.2 requires.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
  Warn,
  Block,
}

impl Action {
  pub fn as_str(self) -> &'static str {
    match self {
      Action::Warn => "warn",
      Action::Block => "block",
    }
  }
}

/// An interaction selector — contract spec §4.2's identity, with `states` optional (§7.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InteractionRef {
  pub description: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub states: Option<Vec<String>>,
}

/// One accepted gap between what the provider may do and what the consumer has tested (§7.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Exemption {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub consumer: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub interaction: Option<InteractionRef>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub path: Option<String>,
  /// Required by §7.2: an exemption a team could accept silently would make "why is this
  /// exempted" archaeology instead of a one-line answer.
  pub reason: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub expires: Option<String>,
}

impl Exemption {
  /// Whether every selector this exemption *sets* matches the finding at hand (§7.2). State lists
  /// are compared as sets: an interaction's identity is its description plus which states it
  /// declares, and a policy author who wrote two state names in the other order meant the same
  /// interaction.
  pub fn matches(&self, consumer: &str, description: &str, states: &[String], path: &str) -> bool {
    if self.consumer.as_deref().is_some_and(|name| name != consumer) {
      return false;
    }
    if let Some(interaction) = &self.interaction {
      if interaction.description != description {
        return false;
      }
      if let Some(selected) = &interaction.states {
        let mut selected: Vec<&str> = selected.iter().map(String::as_str).collect();
        let mut declared: Vec<&str> = states.iter().map(String::as_str).collect();
        selected.sort_unstable();
        declared.sort_unstable();
        if selected != declared {
          return false;
        }
      }
    }
    if self.path.as_deref().is_some_and(|selected| selected != path) {
      return false;
    }
    true
  }

  /// Whether this exemption has passed its own expiry date, as of `as_of` (both `YYYY-MM-DD`).
  ///
  /// Inclusive: an exemption expiring on the 1st still applies *on* the 1st and has lapsed on the
  /// 2nd. A lapsed exemption stops silencing its finding — which is the whole point of writing a
  /// date down, and is why §7.2's "`expires` is optional" matters: a team that means "forever"
  /// says so by leaving it out rather than by choosing a date it does not believe.
  pub fn lapsed(&self, as_of: Option<&str>) -> bool {
    match (&self.expires, as_of) {
      (Some(expires), Some(as_of)) => as_of > expires.as_str(),
      _ => false,
    }
  }
}

/// One layer of policy (§7.1): `on-finding`/`on-review` override, `exemptions` accumulate — so a
/// layer parses as an all-optional patch rather than a full policy.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
struct PolicyPatch {
  on_finding: Option<Action>,
  on_review: Option<Action>,
  exemptions: Vec<Exemption>,
}

/// A resolved subsumption policy (§7.1): the specification's defaults, with every layer's
/// overrides applied in order and `exemptions` accumulated across all of them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SubsumptionPolicy {
  pub on_finding: Action,
  pub on_review: Action,
  #[serde(default)]
  pub exemptions: Vec<Exemption>,
}

impl Default for SubsumptionPolicy {
  /// ADR 0016: both default to `warn`. A checker that blocked on first contact would be adopted
  /// by nobody, and `unknown` is an honest answer rather than a lesser finding.
  fn default() -> Self {
    SubsumptionPolicy {
      on_finding: Action::Warn,
      on_review: Action::Warn,
      exemptions: Vec::new(),
    }
  }
}

impl SubsumptionPolicy {
  /// Resolve the layered policy document (§7.1): defaults, then each of `layers` in order
  /// (project configuration, the per-run override). `None` entries — a layer that supplied
  /// nothing — are ignored.
  pub fn resolve(layers: &[Option<&Value>]) -> Result<SubsumptionPolicy, Problem> {
    let mut policy = SubsumptionPolicy::default();
    for (index, layer) in layers.iter().enumerate() {
      let Some(layer) = layer else { continue };
      let patch: PolicyPatch = serde_path_to_error::deserialize(*layer).map_err(|err| Problem {
        pointer: format!("/policy/{index}{}", crate::error::json_pointer(err.path())),
        message: err.inner().to_string(),
      })?;
      if let Some(action) = patch.on_finding {
        policy.on_finding = action;
      }
      if let Some(action) = patch.on_review {
        policy.on_review = action;
      }
      policy.exemptions.extend(patch.exemptions);
    }
    Ok(policy)
  }

  /// The action a severity dispatches to, or `None` for one no policy governs — `advisory`, which
  /// §4.3 makes policy-inert by construction.
  pub fn action_for(&self, severity: super::Severity) -> Option<Action> {
    match severity {
      super::Severity::Finding => Some(self.on_finding),
      super::Severity::Review => Some(self.on_review),
      super::Severity::Advisory => None,
    }
  }
}
