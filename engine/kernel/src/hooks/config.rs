//! The resolved hook configuration (lifecycle-hooks spec §6, `hook-config.schema.json`) and
//! everything it can be wrong about (§11).
//!
//! **Resolved**, emphatically: this is the loader's output, never a file (ADR 0014). Interpolation
//! is done, script sources are inline, and no member holds a template or a path — so a run does not
//! depend on the directory it was started in, the WASM-embedded engine that can read no files runs
//! the same hooks as the native one, and the document that determined a run's behaviour is *one
//! document* that can be attached to a failure report.
//!
//! Every problem this module finds is found **before the first exchange** (§5.3): a configuration
//! that could not be understood has nothing to report into, which is exactly what separates these
//! from hook *failures*, which are outcomes.

use super::points::{Point, Policy, point};
use crate::error::Problem;
use serde::Deserialize;
use serde_json::{Map, Value};
use std::collections::BTreeMap;

/// The document the engine receives: hooks keyed by lifecycle point, in the order they run.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct HookConfig {
  #[serde(default)]
  pub version: Option<u64>,
  /// Point -> the hooks declared at it. "A point with no entry is not a point that does nothing:
  /// it is a point at which the engine calls nobody."
  #[serde(default)]
  pub hooks: BTreeMap<String, Vec<HookEntry>>,
}

/// The configuration document version this engine understands. A document that names another one
/// fails the run rather than guessing which members it understands (spec §12.1).
pub const CONFIG_VERSION: u64 = 1;

#[derive(Debug, Clone, Deserialize)]
pub struct HookEntry {
  pub name: String,
  pub run: RunSpec,
  #[serde(default)]
  pub config: Option<Value>,
  #[serde(default)]
  pub when: Option<When>,
  /// The context paths this entry may replace. Absent means it can change nothing — which is what
  /// makes an observing hook auditable as one.
  #[serde(default)]
  pub changes: Vec<String>,
  #[serde(rename = "timeout-ms", default)]
  pub timeout_ms: Option<u64>,
  #[serde(rename = "on-failure", default)]
  pub on_failure: Option<String>,
  #[serde(rename = "report-data", default)]
  pub report_data: bool,
}

/// The implementation that answers a hook. Held as its own document rather than as an enum of
/// typed variants: the kind vocabulary is open (spec §8's four are v1's), the kernel reads only
/// `kind` plus the two members it implements itself (`component`, `source`/`entry`), and everything
/// else belongs to whichever invoker answers that kind. Re-typing an `exec` hook's `env` here
/// would put a fact in the kernel that only the process spawner can act on.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct RunSpec {
  document: Value,
}

impl RunSpec {
  pub fn kind(&self) -> &str {
    self
      .document
      .get("kind")
      .and_then(Value::as_str)
      .unwrap_or_default()
  }

  pub fn get(&self, member: &str) -> Option<&Value> {
    self.document.get(member)
  }

  pub fn text(&self, member: &str) -> Option<&str> {
    self.get(member).and_then(Value::as_str)
  }

  /// The whole document, for the invoker that answers this kind.
  pub fn document(&self) -> &Value {
    &self.document
  }

  #[cfg(test)]
  pub fn from_json(document: Value) -> RunSpec {
    RunSpec { document }
  }
}

/// Which occurrences of a point a hook runs at (spec §6.3). Exact string matches, AND-ed; an
/// absent member matches everything. No globs, no regexes, no expressions — a selector that must
/// be *evaluated* to be understood cannot be reviewed, and a hook that silently matches nothing
/// looks exactly like a hook that was never needed.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct When {
  #[serde(default)]
  pub interaction: Option<String>,
  #[serde(default)]
  pub state: Option<String>,
  #[serde(default)]
  pub transport: Option<String>,
}

impl When {
  /// Whether this selector admits an occurrence described by the three facts it can name. A fact
  /// the occurrence does not have (a state name at an exchange point) does **not** match a
  /// selector that names one: matching it would run a state handler where there is no state.
  pub fn admits(&self, interaction: Option<&str>, state: Option<&str>, transport: Option<&str>) -> bool {
    matches(self.interaction.as_deref(), interaction)
      && matches(self.state.as_deref(), state)
      && matches(self.transport.as_deref(), transport)
  }
}

fn matches(wanted: Option<&str>, actual: Option<&str>) -> bool {
  match wanted {
    None => true,
    Some(wanted) => actual == Some(wanted),
  }
}

impl HookEntry {
  /// This entry's failure policy: what it declared, or the point's default (spec §5.2).
  pub fn policy(&self, point: &Point) -> Policy {
    self
      .on_failure
      .as_deref()
      .and_then(Policy::parse)
      .unwrap_or(point.default_policy)
  }

  pub fn timeout_ms(&self, point: &Point) -> u64 {
    self.timeout_ms.unwrap_or_else(|| point.default_timeout_ms())
  }

  /// The paths this invocation may replace: the intersection of what the point permits and what
  /// the entry declared (spec §4.3). Carried in the context as `mutable`, so a hook can be written
  /// against what it was given rather than against a table.
  pub fn mutable(&self, point: &Point) -> Vec<String> {
    self
      .changes
      .iter()
      .filter(|path| point.permits(path))
      .cloned()
      .collect()
  }
}

/// Parse and validate a resolved configuration document. Every problem is collected, because a
/// configuration with three mistakes should take one run to find, not three.
pub fn parse(document: &Value) -> Result<HookConfig, Vec<Problem>> {
  let config: HookConfig = match serde_path_to_error::deserialize(document) {
    Ok(config) => config,
    Err(err) => {
      return Err(vec![Problem {
        pointer: crate::error::json_pointer(err.path()),
        message: err.inner().to_string(),
      }]);
    }
  };
  validate(&config)?;
  Ok(config)
}

/// Spec §11's `hook-config-invalid`, in full: an unknown version, a point that does not exist, a
/// duplicate name within a point, an entry whose `run` names no kind, and a `changes` path the
/// point does not permit.
///
/// Problems carry the engine's standard `{pointer, message}` shape (protocol §10.2). The hooks
/// specification's §11 table writes `{path, message}` for this one error's details; using a second
/// problem shape for the same idea, inside one error taxonomy, would cost every host a special
/// case for one code, so the pointer form is used and this note is the record of the choice.
pub fn validate(config: &HookConfig) -> Result<(), Vec<Problem>> {
  let mut problems = Vec::new();

  if let Some(version) = config.version
    && version != CONFIG_VERSION
  {
    problems.push(Problem {
      pointer: "/version".to_string(),
      message: format!(
        "hook configuration version {version} is not supported (this engine reads {CONFIG_VERSION})"
      ),
    });
  }

  for (point_name, entries) in &config.hooks {
    let at = format!("/hooks/{point_name}");
    let Some(point) = point(point_name) else {
      problems.push(Problem {
        pointer: at,
        message: format!(
          "'{point_name}' is not a lifecycle point; this engine has {}",
          super::points::POINTS
            .iter()
            .map(|p| p.name)
            .collect::<Vec<_>>()
            .join(", ")
        ),
      });
      continue;
    };

    let mut seen: Vec<&str> = Vec::new();
    for (i, entry) in entries.iter().enumerate() {
      let entry_at = format!("{at}/{i}");
      if !is_valid_name(&entry.name) {
        problems.push(Problem {
          pointer: format!("{entry_at}/name"),
          message: format!("'{}' is not a hook name ([a-z][a-z0-9-]*)", entry.name),
        });
      }
      if seen.contains(&entry.name.as_str()) {
        problems.push(Problem {
          pointer: format!("{entry_at}/name"),
          message: format!(
            "'{}' is declared twice at '{point_name}'; a hook that cannot be named in a failure message is a hook nobody can debug",
            entry.name
          ),
        });
      }
      seen.push(&entry.name);

      if entry.run.kind().is_empty() {
        problems.push(Problem {
          pointer: format!("{entry_at}/run/kind"),
          message: "'run' must name an implementation kind".to_string(),
        });
      }
      if let Some(on_failure) = &entry.on_failure
        && Policy::parse(on_failure).is_none()
      {
        problems.push(Problem {
          pointer: format!("{entry_at}/on-failure"),
          message: format!("'{on_failure}' is not a failure policy (abort-run, fail-exchange, warn)"),
        });
      }
      for (j, path) in entry.changes.iter().enumerate() {
        if !point.permits(path) {
          problems.push(Problem {
            pointer: format!("{entry_at}/changes/{j}"),
            message: format!(
              "'{point_name}' does not permit changing '{path}'; declaring a change the point refuses would be a hook that fails the first time it tries"
            ),
          });
        }
      }
    }
  }

  if problems.is_empty() {
    Ok(())
  } else {
    Err(problems)
  }
}

fn is_valid_name(name: &str) -> bool {
  let mut chars = name.chars();
  matches!(chars.next(), Some(c) if c.is_ascii_lowercase())
    && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// The entries at one point, in declaration order (spec §4.2: a list is already an order).
pub fn entries_at<'a>(config: &'a HookConfig, point: &str) -> &'a [HookEntry] {
  config.hooks.get(point).map(Vec::as_slice).unwrap_or_default()
}

/// Which implementation kinds a configuration actually needs — what an embedding is checked
/// against before the run starts (spec §8.5).
pub fn kinds_used(config: &HookConfig) -> Vec<(&str, &HookEntry)> {
  config
    .hooks
    .values()
    .flatten()
    .map(|entry| (entry.run.kind(), entry))
    .collect()
}

/// A hook entry as it may be *rendered* — name, point and implementation kind, never its `config`,
/// `env` or `headers` (spec §7.3: the engine never sends a project's secrets back out).
pub fn describe(entry: &HookEntry, point: &str) -> Map<String, Value> {
  let mut described = Map::new();
  described.insert("hook".to_string(), Value::String(entry.name.clone()));
  described.insert("point".to_string(), Value::String(point.to_string()));
  described.insert(
    "implementation".to_string(),
    Value::String(entry.run.kind().to_string()),
  );
  described
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json::json;

  fn config_of(document: Value) -> Result<HookConfig, Vec<Problem>> {
    parse(&document)
  }

  #[test]
  fn a_configuration_the_engine_understands_parses_with_its_defaults() {
    let config = config_of(json!({
      "version": 1,
      "hooks": {
        "before-request": [
          { "name": "sign-requests", "run": { "kind": "script", "source": "function hook(){}" },
            "changes": ["parts.request.headers"] }
        ]
      }
    }))
    .expect("valid");
    let entries = entries_at(&config, "before-request");
    assert_eq!(entries.len(), 1);
    let point = point("before-request").unwrap();
    assert_eq!(
      entries[0].policy(point),
      Policy::FailExchange,
      "the point's default"
    );
    assert_eq!(entries[0].timeout_ms(point), 5_000);
    assert_eq!(
      entries[0].mutable(point),
      vec!["parts.request.headers".to_string()]
    );
    assert!(
      !entries[0].report_data,
      "a hook's scratch is not reported by default"
    );
  }

  #[test]
  fn an_unknown_point_is_named_never_ignored() {
    let problems = config_of(json!({ "hooks": { "before-lunch": [] } })).expect_err("no such point");
    assert_eq!(problems[0].pointer, "/hooks/before-lunch");
    assert!(
      problems[0].message.contains("before-request"),
      "the message lists the points there are: {}",
      problems[0].message
    );
  }

  #[test]
  fn two_hooks_with_one_name_at_one_point_are_rejected() {
    let entry = json!({ "name": "sign", "run": { "kind": "script", "source": "" } });
    let problems = config_of(json!({ "hooks": { "before-request": [entry.clone(), entry] } }))
      .expect_err("a name must identify one hook");
    assert_eq!(problems[0].pointer, "/hooks/before-request/1/name");
  }

  #[test]
  fn a_change_the_point_refuses_is_rejected_at_load_rather_than_at_the_first_invocation() {
    let problems = config_of(json!({
      "hooks": { "after-response": [
        { "name": "tidy", "run": { "kind": "script", "source": "" },
          "changes": ["parts.response.headers"] } ] }
    }))
    .expect_err("after-response mutates nothing");
    assert_eq!(problems[0].pointer, "/hooks/after-response/0/changes/0");
  }

  #[test]
  fn an_entry_with_no_implementation_kind_is_rejected() {
    let problems = config_of(json!({
      "hooks": { "before-request": [ { "name": "sign", "run": { "path": "./sign.ts" } } ] }
    }))
    .expect_err("a path is not a kind");
    assert_eq!(problems[0].pointer, "/hooks/before-request/0/run/kind");
  }

  #[test]
  fn an_unreadable_version_is_refused_rather_than_guessed_at() {
    let problems = config_of(json!({ "version": 2, "hooks": {} })).expect_err("version 2 is not v1");
    assert_eq!(problems[0].pointer, "/version");
  }

  #[test]
  fn a_bad_policy_name_is_rejected_with_the_ones_that_exist() {
    let problems = config_of(json!({
      "hooks": { "before-request": [
        { "name": "sign", "run": { "kind": "script", "source": "" }, "on-failure": "ignore" } ] }
    }))
    .expect_err("'ignore' is not a policy");
    assert!(problems[0].message.contains("abort-run"));
  }

  #[test]
  fn a_selector_matches_on_every_member_it_names_and_ignores_the_ones_it_does_not() {
    let when: When = serde_json::from_value(json!({ "state": "an order exists" })).unwrap();
    assert!(when.admits(
      Some("a request for an order"),
      Some("an order exists"),
      Some("http")
    ));
    assert!(!when.admits(
      Some("a request for an order"),
      Some("no orders exist"),
      Some("http")
    ));
    assert!(
      !when.admits(Some("a request for an order"), None, Some("http")),
      "a state selector does not match an occurrence that has no state"
    );

    let empty = When::default();
    assert!(
      empty.admits(None, None, None),
      "an absent member matches everything"
    );
  }

  #[test]
  fn a_rendering_of_a_hook_carries_no_configuration() {
    let config = config_of(json!({
      "hooks": { "before-request": [
        { "name": "sign", "run": { "kind": "exec", "command": "./sign", "env": { "SECRET": "hunter2" } },
          "config": { "secret": "hunter2" } } ] }
    }))
    .expect("valid");
    let described = describe(&entries_at(&config, "before-request")[0], "before-request");
    let rendered = serde_json::to_string(&described).unwrap();
    assert!(
      !rendered.contains("hunter2"),
      "secrets never come back out: {rendered}"
    );
    assert_eq!(described["implementation"], json!("exec"));
  }
}
