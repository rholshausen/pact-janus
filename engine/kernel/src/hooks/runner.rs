//! Running the hooks at a point (lifecycle-hooks spec §4, §5): select the entries whose `when`
//! admits this occurrence, invoke them **one at a time, in declaration order**, each seeing the
//! changes the ones before it made, and turn what comes back into an effect on the run.
//!
//! Order is declaration order and nothing else (§4.2). There is no priority number and no
//! dependency graph, because ordering metadata makes the order a function of every entry in the
//! file rather than of the sequence a reader can see. Concurrency is refused for the reason it is
//! refused for variants: two hooks mutating the same parts concurrently make the result depend on
//! scheduling, and there is nothing to win at a handful of calls per exchange.

use super::config::{HookConfig, HookEntry, entries_at};
use super::invoke::{HookFailure, HookInvoker, InvokeResult, Outcome, apply_changes, parts_json};
use super::points::{Point, Policy, Scope, point};
use super::report::{Abort, Effect, HookReport, Invocation};
use crate::component::{HookComponent, Invoke, Parts};
use crate::error::Problem;
use serde_json::{Map, Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

/// Why a hook configuration could not be run at all (spec §11). Every one of these is detectable
/// **before the first exchange**, which is what separates them from hook failures: a configuration
/// that could not be understood has no run to report into.
#[derive(Debug)]
pub enum ConfigError {
  /// `hook-config-invalid`: the document is wrong about itself.
  Invalid(Vec<Problem>),
  /// `hook-unavailable`: an implementation kind this embedding cannot run, or a `component` hook
  /// naming a component that was never registered.
  Unavailable {
    kind: String,
    hook: String,
    available: Vec<String>,
  },
}

/// What one point's hooks did to the run.
#[derive(Debug, Clone, PartialEq)]
pub enum PointOutcome {
  /// Every hook was `ok`, `skipped`, or failed under a `warn` policy.
  Continue,
  /// A hook failed under `fail-exchange`: this interaction-and-variant attempt is failed with the
  /// hook's own error as its cause. Teardown still runs.
  FailExchange { hook: String, error: Value },
  /// A `state-setup` hook answered `unsupported`: the provider cannot reach the state
  /// (variant-semantics spec §6.7). Its own status, deliberately not a synonym for failure.
  StateUnavailable { hook: String, error: Value },
  /// A hook failed under `abort-run`.
  Abort { hook: String, error: Value },
}

/// What an occurrence of a point is *about*: the facts a `when` selector can name and the context
/// members the point carries. Assembled by the caller (the verification run), because only it knows
/// where it is.
#[derive(Debug, Default, Clone)]
pub struct Occurrence {
  pub interaction: Option<Value>,
  pub variant: Option<Value>,
  pub state: Option<Value>,
  pub exchange: Option<Value>,
  pub endpoint: Option<Value>,
  pub summary: Option<Value>,
  pub transport: Option<String>,
}

impl Occurrence {
  fn interaction_description(&self) -> Option<&str> {
    self.interaction.as_ref()?.get("description")?.as_str()
  }

  fn state_name(&self) -> Option<&str> {
    self.state.as_ref()?.get("name")?.as_str()
  }

  fn exchange_id(&self) -> Option<String> {
    Some(self.exchange.as_ref()?.get("id")?.as_str()?.to_string())
  }

  fn variant_id(&self) -> Option<String> {
    Some(self.variant.as_ref()?.get("id")?.as_str()?.to_string())
  }
}

/// The hooks of one run: the configuration, the implementations that can answer it, the run-scope
/// `data` hooks have accumulated, and the report being built.
pub struct HookRunner {
  config: HookConfig,
  invokers: HashMap<String, Arc<dyn HookInvoker>>,
  components: HashMap<String, Arc<dyn HookComponent>>,
  run: Value,
  run_data: Map<String, Value>,
  pub report: HookReport,
}

impl HookRunner {
  /// Build a runner for `document`, checking everything §11 requires to be checked before the
  /// first exchange: that the configuration is valid, and that every implementation it names is one
  /// this embedding can actually run.
  pub fn new(
    document: &Value,
    invokers: HashMap<String, Arc<dyn HookInvoker>>,
    components: HashMap<String, Arc<dyn HookComponent>>,
    run: Value,
  ) -> Result<HookRunner, ConfigError> {
    let config = super::config::parse(document).map_err(ConfigError::Invalid)?;

    let mut available: Vec<String> = invokers.keys().cloned().collect();
    if !components.is_empty() {
      available.push("component".to_string());
    }
    available.sort();

    for (point_name, entries) in &config.hooks {
      for entry in entries {
        let kind = entry.run.kind();
        let runnable = match kind {
          "component" => entry
            .run
            .text("component")
            .is_some_and(|name| components.contains_key(name)),
          other => invokers.contains_key(other),
        };
        if !runnable {
          tracing::warn!(point = %point_name, hook = %entry.name, kind, "no implementation for this hook");
          return Err(ConfigError::Unavailable {
            kind: kind.to_string(),
            hook: entry.name.clone(),
            available: available.clone(),
          });
        }
      }
    }

    Ok(HookRunner {
      config,
      invokers,
      components,
      run,
      run_data: Map::new(),
      report: HookReport::default(),
    })
  }

  /// Whether any hook is configured at `point` — so a run can skip assembling a context nobody
  /// will read.
  pub fn has(&self, point: &str) -> bool {
    !entries_at(&self.config, point).is_empty()
  }

  /// Invoke every hook at `point` that this occurrence admits. `parts` is the parts in flight,
  /// mutated in place by whatever changes are declared, permitted and applied; `exchange_data`
  /// accumulates exchange-scope `data`. Every invocation is handed to `emit` as a
  /// `verification/hook` event payload before the next one runs, so a host watching the stream
  /// sees hook activity as it happens rather than at the end.
  pub fn run_point(
    &mut self,
    point_name: &str,
    occurrence: &Occurrence,
    parts: Option<&mut Parts>,
    exchange_data: &mut Map<String, Value>,
    emit: &mut dyn FnMut(Value),
  ) -> PointOutcome {
    let Some(point) = point(point_name) else {
      // Unreachable: the configuration was validated against this same table.
      return PointOutcome::Continue;
    };
    let entries = entries_at(&self.config, point_name).to_vec();
    if entries.is_empty() {
      return PointOutcome::Continue;
    }

    let mut scratch_parts = Parts::new();
    let parts = parts.unwrap_or(&mut scratch_parts);

    for entry in &entries {
      let admitted = entry.when.as_ref().is_none_or(|when| {
        when.admits(
          occurrence.interaction_description(),
          occurrence.state_name(),
          occurrence.transport.as_deref(),
        )
      });
      if !admitted {
        // Not recorded: a hook that did not run is absent from the report, and that difference from
        // "ran and changed nothing" is exactly how a wrong selector is spotted (spec §10.2).
        tracing::debug!(point = point_name, hook = %entry.name, "selector did not admit this occurrence");
        continue;
      }

      let outcome = self.invoke_entry(point, entry, occurrence, parts, exchange_data, emit);
      match outcome {
        PointOutcome::Continue => continue,
        // "remaining hooks at the point do not run" (spec §5.2)
        terminal => return terminal,
      }
    }
    PointOutcome::Continue
  }

  #[allow(clippy::too_many_arguments)]
  fn invoke_entry(
    &mut self,
    point: &'static Point,
    entry: &HookEntry,
    occurrence: &Occurrence,
    parts: &mut Parts,
    exchange_data: &mut Map<String, Value>,
    emit: &mut dyn FnMut(Value),
  ) -> PointOutcome {
    let deadline_ms = entry.timeout_ms(point);
    let context = self.context(point, entry, occurrence, parts, exchange_data, deadline_ms);
    let started = Instant::now();

    let answered = match entry.run.kind() {
      "component" => self.invoke_component(entry, point, &context, deadline_ms),
      kind => match self.invokers.get(kind) {
        Some(invoker) => invoker.invoke(entry.run.document(), &context, deadline_ms),
        // Unreachable: `new` refused an unrunnable configuration.
        None => Err(HookFailure::errored(format!("no invoker for '{kind}'"))),
      },
    };

    let mut invocation = Invocation {
      point: point.name.to_string(),
      hook: entry.name.clone(),
      implementation: entry.run.kind().to_string(),
      outcome: Outcome::Ok,
      effect: Effect::None,
      exchange: occurrence.exchange_id(),
      interaction: occurrence.interaction_description().map(str::to_string),
      variant: occurrence.variant_id(),
      state: occurrence.state_name().map(str::to_string),
      changed: Vec::new(),
      duration_ms: 0,
      data: None,
      error: None,
    };

    let mut outcome = match answered {
      Ok(result) => {
        invocation.outcome = result.outcome();
        self.absorb(entry, &result, occurrence, exchange_data, &mut invocation);
        match result.outcome() {
          Outcome::Ok | Outcome::Skipped => {
            match apply_changes(&result, entry, point, parts) {
              Ok(changed) => {
                invocation.changed = changed;
                PointOutcome::Continue
              }
              Err(refusal) => {
                // A refused change is a failed invocation (spec §4.3), so the point's policy
                // applies to it exactly as it would to a hook that said `failed` itself.
                invocation.outcome = Outcome::Failed;
                invocation.error = Some(refusal.clone());
                self.effect_of(entry, point, refusal, &mut invocation)
              }
            }
          }
          Outcome::Unsupported => self.unsupported(entry, point, &result, &mut invocation),
          _ => {
            let error = result.error.clone().unwrap_or_else(
              || json!({ "code": "hook-failed", "message": format!("hook '{}' failed", entry.name) }),
            );
            invocation.error = Some(error.clone());
            self.effect_of(entry, point, error, &mut invocation)
          }
        }
      }
      Err(failure) => {
        invocation.outcome = failure.outcome;
        invocation.error = Some(failure.error.clone());
        self.effect_of(entry, point, failure.error, &mut invocation)
      }
    };

    invocation.duration_ms = started.elapsed().as_millis() as u64;
    if let PointOutcome::Abort { .. } = &outcome {
      // The abort's own record: which point, which hook, which error (spec §10.2). The count of
      // exchanges never run is the run's to fill in, since only it knows how many were left.
      self.report.aborted = Some(Abort {
        point: point.name.to_string(),
        hook: entry.name.clone(),
        error: invocation.error.clone(),
        exchanges_not_run: 0,
      });
    }
    emit(invocation.to_json());
    self.report.record(invocation);

    // A `warn` policy is not a terminal outcome: the remaining hooks at the point still run.
    if matches!(&outcome, PointOutcome::Continue) {
      outcome = PointOutcome::Continue;
    }
    outcome
  }

  fn invoke_component(
    &self,
    entry: &HookEntry,
    point: &Point,
    context: &Value,
    deadline_ms: u64,
  ) -> Result<InvokeResult, HookFailure> {
    let Some(name) = entry.run.text("component") else {
      return Err(HookFailure::errored("a component hook must name its component"));
    };
    let Some(component) = self.components.get(name) else {
      return Err(HookFailure::errored(format!(
        "no component '{name}' is registered"
      )));
    };
    let answered = component.invoke(Invoke {
      point: point.name.to_string(),
      context: context.clone(),
      config: entry.config.clone(),
      deadline_ms,
    });
    match answered {
      // The component's own error document, passed through verbatim: the kernel does not translate
      // component error interiors (protocol §10.2).
      Err(error) => Err(HookFailure {
        outcome: Outcome::Errored,
        error: serde_json::to_value(&error).unwrap_or_else(|_| json!({ "code": "component-failed" })),
      }),
      Ok(document) => InvokeResult::parse(&document).map_err(|message| HookFailure {
        outcome: Outcome::Errored,
        error: json!({ "code": "hook-result-invalid", "message": message }),
      }),
    }
  }

  /// Take a result's `data` where its scope says it goes (spec §4.4), and report it only when the
  /// entry asked for that (§7.3).
  fn absorb(
    &mut self,
    entry: &HookEntry,
    result: &InvokeResult,
    occurrence: &Occurrence,
    exchange_data: &mut Map<String, Value>,
    invocation: &mut Invocation,
  ) {
    let Some(data) = result.data.clone() else {
      return;
    };
    if entry.report_data {
      invocation.data = Some(data.clone());
    }
    let scope = point(&invocation.point)
      .map(|p| p.scope)
      .unwrap_or(Scope::Exchange);
    match scope {
      Scope::Run => {
        self.run_data.insert(entry.name.clone(), data);
      }
      _ => {
        let _ = occurrence;
        exchange_data.insert(entry.name.clone(), data);
      }
    }
  }

  /// `unsupported` at a state point is variant-semantics §6.7's status; anywhere else it is a hook
  /// answering something its point has no meaning for, which is a failure rather than a silent
  /// pass — the outcome vocabulary is open, and guessing what an out-of-place value meant is what
  /// the open-discriminator rule forbids.
  fn unsupported(
    &mut self,
    entry: &HookEntry,
    point: &Point,
    result: &InvokeResult,
    invocation: &mut Invocation,
  ) -> PointOutcome {
    let error = result.error.clone().unwrap_or_else(
      || json!({ "code": "state-unsupported", "message": "the provider cannot reach this state" }),
    );
    invocation.error = Some(error.clone());
    if point.scope == Scope::State {
      invocation.effect = Effect::StateUnavailable;
      PointOutcome::StateUnavailable {
        hook: entry.name.clone(),
        error,
      }
    } else {
      invocation.outcome = Outcome::Failed;
      self.effect_of(entry, point, error, invocation)
    }
  }

  /// Apply the point's failure policy (spec §5.2).
  fn effect_of(
    &mut self,
    entry: &HookEntry,
    point: &Point,
    error: Value,
    invocation: &mut Invocation,
  ) -> PointOutcome {
    match entry.policy(point) {
      Policy::AbortRun => {
        invocation.effect = Effect::AbortedRun;
        PointOutcome::Abort {
          hook: entry.name.clone(),
          error,
        }
      }
      Policy::FailExchange => {
        invocation.effect = Effect::FailedExchange;
        PointOutcome::FailExchange {
          hook: entry.name.clone(),
          error,
        }
      }
      Policy::Warn => {
        invocation.effect = Effect::Warned;
        PointOutcome::Continue
      }
    }
  }

  /// The `HookContext` for one invocation (spec §2.3, `hook-context.schema.json`). Built per
  /// invocation rather than per point, because `mutable` and `deadline-ms` are the entry's and
  /// `run.data` may have grown since the last one.
  fn context(
    &self,
    point: &Point,
    entry: &HookEntry,
    occurrence: &Occurrence,
    parts: &Parts,
    exchange_data: &Map<String, Value>,
    deadline_ms: u64,
  ) -> Value {
    let mut context = Map::new();
    context.insert("point".to_string(), json!(point.name));
    context.insert("role".to_string(), json!("provider"));

    let mut run = self.run.as_object().cloned().unwrap_or_default();
    run.insert("data".to_string(), Value::Object(self.run_data.clone()));
    context.insert("run".to_string(), Value::Object(run));

    for (key, value) in [
      ("interaction", &occurrence.interaction),
      ("variant", &occurrence.variant),
      ("state", &occurrence.state),
      ("endpoint", &occurrence.endpoint),
      ("summary", &occurrence.summary),
    ] {
      if let Some(value) = value {
        context.insert(key.to_string(), value.clone());
      }
    }

    if let Some(exchange) = &occurrence.exchange {
      let mut exchange = exchange.as_object().cloned().unwrap_or_default();
      exchange.insert("data".to_string(), Value::Object(exchange_data.clone()));
      context.insert("exchange".to_string(), Value::Object(exchange));
    }

    if point.scope != Scope::Run {
      context.insert("parts".to_string(), parts_json(parts));
    }
    if let Some(config) = &entry.config {
      context.insert("config".to_string(), config.clone());
    }
    context.insert("mutable".to_string(), json!(entry.mutable(point)));
    context.insert("deadline-ms".to_string(), json!(deadline_ms));
    Value::Object(context)
  }

  /// Fill in how many exchanges never ran, once the run knows (spec §10.2).
  pub fn note_exchanges_not_run(&mut self, count: usize) {
    if let Some(abort) = &mut self.report.aborted {
      abort.exchanges_not_run = count;
    }
  }
}
