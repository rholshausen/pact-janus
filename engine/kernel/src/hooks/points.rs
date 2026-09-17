//! The point vocabulary (lifecycle-hooks spec §3): what each point is called, how often it runs,
//! what it may change and what its failure means.
//!
//! The table is data rather than a match arm per point because every one of its columns is
//! normative and each is read from somewhere different — the runner reads the scope, the
//! configuration validator reads the mutable set, the failure handler reads the default policy —
//! and a fact spread across three matches drifts.
//!
//! The vocabulary is **open** (protocol §2.2 rule 1): later versions add points. That cuts one
//! way only. An engine that grows a point does not break a hook that does not know it, but a
//! *configuration* naming a point this engine does not have is `hook-config-invalid` naming it and
//! never a silently ignored block — an ignored hook is indistinguishable from a hook that ran, and
//! the ignored one is how an unsigned request reaches a provider (spec §12.2).

/// How often a point runs, and therefore what its context can hold (spec §2.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
  /// Once per run.
  Run,
  /// Once per interaction × variant attempt — the unit a verification result reports on.
  Exchange,
  /// Once per provider state, per exchange.
  State,
}

/// What a failed invocation does to the run (spec §5.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Policy {
  /// The run stops; the summary says which hook stopped it and how much never ran.
  AbortRun,
  /// This interaction-and-variant attempt fails with the hook's error as its cause; the run goes
  /// on to the next exchange, and teardown still runs.
  FailExchange,
  /// Recorded, reported, and the run continues — including the remaining hooks at that point.
  Warn,
}

impl Policy {
  pub fn parse(text: &str) -> Option<Policy> {
    match text {
      "abort-run" => Some(Policy::AbortRun),
      "fail-exchange" => Some(Policy::FailExchange),
      "warn" => Some(Policy::Warn),
      _ => None,
    }
  }

  pub fn as_str(self) -> &'static str {
    match self {
      Policy::AbortRun => "abort-run",
      Policy::FailExchange => "fail-exchange",
      Policy::Warn => "warn",
    }
  }
}

/// One point's specification (spec §3's table).
#[derive(Debug, Clone, Copy)]
pub struct Point {
  pub name: &'static str,
  pub scope: Scope,
  /// The default when the entry names no `on-failure`. The rule behind every one of them: **the
  /// default is never the quiet one where quietness would mislead.** A `before-request` hook that
  /// failed to sign a request would produce a 401 the report would blame on the provider.
  pub default_policy: Policy,
  /// Whether this point can change anything at all, and what. `Mutable::None` is not a limitation
  /// to work around: at `after-response` a hook that could rewrite the response would be editing
  /// the evidence, and the header it normalised away is the one whose absence the consumer would
  /// have noticed (spec §3.6).
  pub mutable: Mutable,
}

/// What a point permits a hook to replace (spec §3, §4.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mutable {
  /// Nothing. Most points.
  None,
  /// The slots of the outbound parts: `parts.<part>.<slot>`, as the transport names them.
  OutboundSlots,
  /// The whole parts document — `produce-message`, where producing it is the hook's job.
  Parts,
}

/// Every point v1 defines (spec §3), in the order a run reaches them.
pub const POINTS: &[Point] = &[
  Point {
    name: "before-verification",
    scope: Scope::Run,
    default_policy: Policy::AbortRun,
    mutable: Mutable::None,
  },
  Point {
    name: "state-setup",
    scope: Scope::State,
    default_policy: Policy::FailExchange,
    mutable: Mutable::None,
  },
  Point {
    name: "before-request",
    scope: Scope::Exchange,
    default_policy: Policy::FailExchange,
    mutable: Mutable::OutboundSlots,
  },
  Point {
    name: "produce-message",
    scope: Scope::Exchange,
    default_policy: Policy::FailExchange,
    mutable: Mutable::Parts,
  },
  Point {
    name: "consume-message",
    scope: Scope::Exchange,
    default_policy: Policy::FailExchange,
    mutable: Mutable::None,
  },
  Point {
    name: "after-response",
    scope: Scope::Exchange,
    default_policy: Policy::FailExchange,
    mutable: Mutable::None,
  },
  Point {
    name: "state-teardown",
    scope: Scope::State,
    default_policy: Policy::Warn,
    mutable: Mutable::None,
  },
  Point {
    name: "after-verification",
    scope: Scope::Run,
    default_policy: Policy::Warn,
    mutable: Mutable::None,
  },
];

/// The default deadline at run points (spec §5.4), where starting a container is the normal case.
pub const RUN_TIMEOUT_MS: u64 = 30_000;
/// The default deadline at exchange and state points.
pub const EXCHANGE_TIMEOUT_MS: u64 = 5_000;

pub fn point(name: &str) -> Option<&'static Point> {
  POINTS.iter().find(|point| point.name == name)
}

pub fn points_of_scope(scope: Scope) -> impl Iterator<Item = &'static Point> {
  POINTS.iter().filter(move |point| point.scope == scope)
}

impl Point {
  /// This point's default deadline (spec §5.4).
  pub fn default_timeout_ms(&self) -> u64 {
    match self.scope {
      Scope::Run => RUN_TIMEOUT_MS,
      Scope::Exchange | Scope::State => EXCHANGE_TIMEOUT_MS,
    }
  }

  /// Whether `path` is within this point's mutable set (spec §4.3, test 1). A dotted path into the
  /// context: `parts.request.headers` at `before-request`, `parts` at `produce-message`.
  pub fn permits(&self, path: &str) -> bool {
    match self.mutable {
      Mutable::None => false,
      Mutable::Parts => path == "parts",
      // `parts.<part>.<slot>` — three segments, no deeper: a hook replaces a slot, and reaching
      // inside one would be editing a document the content component owns.
      Mutable::OutboundSlots => {
        let segments: Vec<&str> = path.split('.').collect();
        segments.len() == 3 && segments[0] == "parts" && !segments[1].is_empty() && !segments[2].is_empty()
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn every_point_of_the_specs_table_is_present_with_its_own_defaults() {
    assert_eq!(POINTS.len(), 8);
    assert_eq!(
      point("before-verification").unwrap().default_policy,
      Policy::AbortRun
    );
    assert_eq!(point("state-setup").unwrap().default_policy, Policy::FailExchange);
    assert_eq!(point("state-teardown").unwrap().default_policy, Policy::Warn);
    assert_eq!(point("after-verification").unwrap().default_policy, Policy::Warn);
    assert!(point("before-lunch").is_none());
  }

  #[test]
  fn run_points_get_the_longer_deadline() {
    assert_eq!(point("before-verification").unwrap().default_timeout_ms(), 30_000);
    assert_eq!(point("before-request").unwrap().default_timeout_ms(), 5_000);
    assert_eq!(point("state-setup").unwrap().default_timeout_ms(), 5_000);
  }

  #[test]
  fn before_request_permits_a_slot_and_nothing_around_it() {
    let before_request = point("before-request").unwrap();
    assert!(before_request.permits("parts.request.headers"));
    assert!(before_request.permits("parts.request.body"));
    assert!(!before_request.permits("parts"), "a slot, not the whole document");
    assert!(
      !before_request.permits("parts.request"),
      "a slot, not a whole part"
    );
    assert!(
      !before_request.permits("parts.request.body.total"),
      "inside a slot is the content component's document, not a hook's"
    );
    assert!(!before_request.permits("interaction.description"));
  }

  #[test]
  fn the_observing_points_permit_nothing() {
    for name in [
      "after-response",
      "state-setup",
      "state-teardown",
      "after-verification",
    ] {
      let point = point(name).unwrap();
      assert!(!point.permits("parts"), "{name}");
      assert!(!point.permits("parts.request.headers"), "{name}");
    }
  }

  #[test]
  fn produce_message_owns_the_whole_parts_document() {
    let produce = point("produce-message").unwrap();
    assert!(produce.permits("parts"));
    assert!(
      !produce.permits("parts.message.body"),
      "the hook produces all of it or none"
    );
  }
}
