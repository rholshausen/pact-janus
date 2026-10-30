//! A verification run's report (ADR 0059): the text a host prints as the run goes, written here once,
//! so `janus verify` and every SDK print the same report and none of them lays one out.
//!
//! A run asks for it with `options.report`. The reporter watches the run's stream, and follows each
//! event worth a line with a `verification/report` event: `{ text }`, and `text-segments` for a host
//! that declared the `segments` capability (spec §2.7), so it can colour what each piece is. A host
//! prints each `text` as it arrives, followed by a line break. The layout is `janus verify`'s, moved
//! here from the CLI.
//!
//! A hook that fails an exchange, or finds its state unreachable, is reported by the result that
//! follows it too, so its event is held and said under that result, once, where a reader looks for
//! why the interaction failed. A hook that aborts the run is the summary's headline.

use super::display::put;
use super::events::Observer;
use crate::display::{Kind, Marked, Segment};
use serde_json::{Value, json};

/// The kind of the events the report is made of.
pub(crate) const REPORT: &str = "verification/report";

/// Where a result's details start: under the description, past the result's tag.
const DETAIL: &str = "          ";

/// What `options.report` asked for, when it asked for a report at all.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct ReportOptions {
  /// Say every layer of an error's chain, as `janus --error-chain` does, rather than what failed
  /// and its root cause (ADR 0054).
  pub error_chain: bool,
}

impl ReportOptions {
  pub fn from_options(options: Option<&Value>) -> Option<ReportOptions> {
    let report = options?.get("report")?;
    match report {
      Value::Bool(true) => Some(ReportOptions::default()),
      Value::Object(report) => Some(ReportOptions {
        error_chain: report
          .get("error-chain")
          .and_then(Value::as_bool)
          .unwrap_or(false),
      }),
      _ => None,
    }
  }
}

/// Writes a run's report from its events.
pub(crate) struct Reporter {
  options: ReportOptions,
  segments: bool,
  held_hook: Option<Value>,
}

impl Reporter {
  pub fn new(options: ReportOptions, segments: bool) -> Reporter {
    Reporter {
      options,
      segments,
      held_hook: None,
    }
  }

  /// What the report says of one event, if anything.
  fn render(&mut self, kind: &str, payload: &Value) -> Option<Marked> {
    match kind {
      "verification/started" => Some(started(payload)),
      "verification/interaction-result" => Some(self.result(payload)),
      "verification/fetched" => Some(fetched(payload)),
      "verification/published" => Some(self.published(payload)),
      "verification/warning" => Some(warning(payload)),
      "verification/executed-plan" => Some(executed_plan(payload)),
      "verification/hook" => self.hook(payload),
      // A run the engine ended before it had anything to count has no tallies to say: the host
      // reports why it did not run, as it reports any error.
      "verification/finished" if payload["variants"].is_object() => {
        let mut text = Marked::plain("\n");
        text.append(self.summary(payload));
        Some(text)
      }
      _ => None,
    }
  }

  fn result(&mut self, payload: &Value) -> Marked {
    let line = format!(
      "{}{}",
      payload["interaction"]["description"].as_str().unwrap_or("?"),
      variant_label(payload)
    );
    let mut text = Marked::plain("  ");
    let note = match payload["status"].as_str().unwrap_or("failed") {
      "verified" => {
        text.append(tag(Kind::Pass, "ok"));
        None
      }
      "state-unavailable" => {
        text.append(tag(Kind::Warn, "STATE"));
        Some("the provider cannot reach a state this contract declares")
      }
      _ if payload["pending"] == json!(true) => {
        text.append(tag(Kind::Warn, "PENDING"));
        Some("failed, and pending: reported, and not failing the run")
      }
      _ => {
        text.append(tag(Kind::Fail, "FAILED"));
        None
      }
    };
    text.text_plain(line);
    // Its states tell two interactions with one description apart, as today's verifiers say them.
    for state in payload["interaction"]["states"].as_array().into_iter().flatten() {
      if let Some(state) = state.as_str() {
        text.text_plain(format!("\n{DETAIL}"));
        text.text_as(Kind::Quiet, format!("given {state}"));
      }
    }
    if let Some(note) = note {
      text.text_plain(format!("\n{DETAIL}"));
      text.text_as(Kind::Quiet, note);
    }
    for mismatch in payload["mismatches"].as_array().into_iter().flatten() {
      text.text_plain(format!("\n{DETAIL}"));
      text.text_as(Kind::Path, mismatch["path"].as_str().unwrap_or(""));
      text.text_plain(": ");
      text.append(marked(mismatch, "message"));
    }
    // The hook's error is the result's: said once, after which hook it was.
    if let Some(hook) = self.held_hook.take() {
      text.text_plain(format!("\n{DETAIL}{}", hook_line(&hook)));
    }
    self.error_under(&mut text, &payload["error"]);
    text
  }

  fn published(&self, payload: &Value) -> Marked {
    let consumer = payload["consumer"].as_str().unwrap_or("?");
    let version = payload["provider-version"].as_str().unwrap_or("?");
    let mut text = Marked::plain("  ");
    match payload["outcome"].as_str() {
      Some("published") => {
        text.text_as(Kind::Pass, "published");
        text.text_plain(format!(" {consumer}'s result for provider version {version}"));
      }
      Some("skipped") if payload["reason"] == json!("not-from-a-broker") => {
        text.text_as(
          Kind::Quiet,
          format!("not publishing {consumer}'s result: its pact was not fetched from a Pact Broker"),
        );
      }
      Some("skipped") => {
        text.text_as(Kind::Warn, "not publishing");
        text.text_plain(format!(
          " {consumer}'s result: the run's filter left some of its variants out"
        ));
      }
      _ => {
        text.text_as(Kind::Fail, "PUBLISH FAILED");
        text.text_plain(format!(" for {consumer}'s result"));
        self.error_under(&mut text, &payload["error"]);
      }
    }
    text
  }

  fn hook(&mut self, payload: &Value) -> Option<Marked> {
    let outcome = payload["outcome"].as_str().unwrap_or("ok");
    if outcome == "continue" || outcome == "ok" {
      return None;
    }
    let mut text = Marked::plain("  ");
    match payload["effect"].as_str() {
      Some("failed-exchange" | "state-unavailable") => {
        self.held_hook = Some(payload.clone());
        return None;
      }
      // Its error is the summary's headline, where the run's end is explained.
      Some("aborted-run") => {
        text.append(tag(Kind::Fail, "hook"));
        text.text_plain(format!(
          "{} at {}: {outcome}, and aborted the run",
          payload["hook"].as_str().unwrap_or("?"),
          payload["point"].as_str().unwrap_or("?"),
        ));
      }
      _ => {
        text.append(tag(Kind::Warn, "hook"));
        text.text_plain(hook_line(payload));
        self.error_under(&mut text, &payload["error"]);
      }
    }
    Some(text)
  }

  /// `FAILED: 0 verified, 2 failed (2 interactions)`: what the run reached, the tallies that are not
  /// zero beside the two that always matter, and how much there was. Variants are named only when
  /// there are more of them than interactions, as there are only in a contract.
  ///
  /// The status is marked by what it says, and so is each tally that is not zero: a reader's eye
  /// goes to the red `2 failed`, not to the `0 failed` of a run that passed.
  fn summary(&self, summary: &Value) -> Marked {
    let status = summary["status"].as_str().unwrap_or("failed");
    let variants = &summary["variants"];
    let tally = |name: &str| variants[name].as_u64().unwrap_or(0);
    let counted = |name: &str, label: &str, kind: Kind| match tally(name) {
      0 => Marked::plain(format!("0 {label}")),
      n => Marked::of(kind, format!("{n} {label}")),
    };
    let mut counts = vec![
      counted("verified", "verified", Kind::Pass),
      counted("failed", "failed", Kind::Fail),
    ];
    for (name, label, kind) in [
      ("state-unavailable", "state-unavailable", Kind::Warn),
      ("skipped", "skipped", Kind::Quiet),
      // `variants.total` counts what the run reached; an aborted or stopped run reached less than it
      // planned, and the rest is counted beside the tallies, never folded into them.
      ("not-run", "not run", Kind::Warn),
    ] {
      if tally(name) > 0 {
        counts.push(counted(name, label, kind));
      }
    }
    let total = tally("total") + tally("not-run");
    let interactions = summary["interactions"].as_u64().unwrap_or(0);
    let scope = if total == interactions {
      count(interactions, "interaction", "interactions")
    } else {
      format!(
        "{} across {}",
        count(total, "variant", "variants"),
        count(interactions, "interaction", "interactions")
      )
    };
    let mut text = Marked::of(status_kind(status), status.to_uppercase());
    text.text_plain(": ");
    text.append(Marked::join(counts, ", "));
    text.text_plain(format!(" ({scope})"));
    if summary["filtered"] == json!(true) {
      text.text_plain("\n");
      text.text_as(
        Kind::Warn,
        "This was a filtered run: unreplayed variants are not passing ones.",
      );
    }
    if summary["stopped"] == json!("first-failure") {
      text.text_plain("\n");
      text.text_as(
        Kind::Warn,
        "The run stopped at its first failure, as it was asked to: what it did not reach was not run.",
      );
    }
    // The abort is the headline of a run that ended this way, so it carries its cause: "the auth
    // hook failed" sends a reader to the logs for what the report already knows.
    if let Some(abort) = summary.get("aborted").filter(|abort| !abort.is_null()) {
      text.text_plain("\n");
      text.text_as(
        Kind::Fail,
        format!(
          "The run was aborted by the '{}' hook at {}",
          abort["hook"].as_str().unwrap_or("?"),
          abort["point"].as_str().unwrap_or("?")
        ),
      );
      match abort.get("error").filter(|error| !error.is_null()) {
        Some(error) => text.text_plain(format!(
          ":\n  {}",
          error_text(error, self.options.error_chain).replace('\n', "\n  ")
        )),
        None => text.text_plain("."),
      };
    }
    text
  }

  /// An error under the line it belongs to: every line of it at the details' indent.
  fn error_under(&self, text: &mut Marked, error: &Value) {
    if error.is_null() {
      return;
    }
    let said = error_text(error, self.options.error_chain).replace('\n', &format!("\n{DETAIL}"));
    text.text_plain(format!("\n{DETAIL}{said}"));
  }
}

impl Observer for Reporter {
  fn observe(&mut self, kind: &str, payload: &Value) -> Vec<(String, Value)> {
    self
      .render(kind, payload)
      .map(|text| {
        let mut report = json!({});
        put(&mut report, "text", &text, self.segments);
        vec![(REPORT.to_string(), report)]
      })
      .unwrap_or_default()
  }
}

fn started(payload: &Value) -> Marked {
  let mut text = Marked::of(
    Kind::Heading,
    format!(
      "Verifying {} in {} [{}] against {}",
      count_of(&payload["interactions"], "interaction", "interactions"),
      count_of(&payload["contracts"], "document", "documents"),
      join(&payload["formats"]),
      join(&payload["providers"]),
    ),
  );
  if payload["filtered"] == json!(true) {
    text.text_plain("\n  ");
    text.text_as(
      Kind::Warn,
      "(filtered run: not every recorded variant will be replayed)",
    );
  }
  text.text_plain("\n");
  text
}

fn fetched(payload: &Value) -> Marked {
  let pacts = payload["pacts"].as_array().cloned().unwrap_or_default();
  let mut text = Marked::of(
    Kind::Heading,
    format!(
      "Fetched {} from {}",
      count(pacts.len() as u64, "pact", "pacts"),
      payload["broker"].as_str().unwrap_or("the broker")
    ),
  );
  for pact in &pacts {
    text.text_plain(format!("\n  {}", pact["consumer"].as_str().unwrap_or("?")));
    match (pact["wip"] == json!(true), pact["pending"] == json!(true)) {
      (true, _) => text.text_as(Kind::Warn, " (work in progress, pending)"),
      (false, true) => text.text_as(Kind::Warn, " (pending)"),
      _ => &mut text,
    };
    for notice in pact["notices"].as_array().into_iter().flatten() {
      text.text_plain("\n    ");
      text.text_as(Kind::Quiet, notice.as_str().unwrap_or_default());
    }
  }
  text.text_plain("\n");
  text
}

fn warning(payload: &Value) -> Marked {
  let code = payload["code"].as_str().unwrap_or("warning");
  let mut text = Marked::plain("  ");
  text.append(tag(Kind::Warn, "warn"));
  text.text_plain(code);
  if let Some(target) = payload["target"].as_str() {
    text.text_plain(" at ");
    text.text_as(Kind::Path, target);
  }
  if let Some(message) = payload["message"].as_str() {
    text.text_plain(format!(": {message}"));
  }
  text
}

/// `--explain-failures`' plan for the variant that just failed, printed where the failure is.
fn executed_plan(payload: &Value) -> Marked {
  let mut text = Marked::plain("\n  ");
  text.text_as(
    Kind::Quiet,
    format!(
      "--- executed plan: {}{} ---",
      payload["interaction"]["description"].as_str().unwrap_or("?"),
      variant_label(payload)
    ),
  );
  text.text_plain("\n  ");
  text.append(indented(marked(payload, "text"), "\n  "));
  text.text_plain("\n");
  text
}

/// `word` as `kind`, padded to the result column: the tag every line of a run's report opens with.
fn tag(kind: Kind, word: &str) -> Marked {
  let mut text = Marked::of(kind, word);
  text.text_plain(" ".repeat(8usize.saturating_sub(word.len())));
  text
}

/// `auth at before-request: failed`.
fn hook_line(payload: &Value) -> String {
  format!(
    "{} at {}: {}",
    payload["hook"].as_str().unwrap_or("?"),
    payload["point"].as_str().unwrap_or("?"),
    payload["outcome"].as_str().unwrap_or("?")
  )
}

/// ` [shipped]`, the variant a result is about, or nothing for a v1–v4 pact's: one example is one
/// variant, always `base`, and naming it on every line says nothing.
fn variant_label(payload: &Value) -> String {
  let pact = payload["interaction"]["format"]
    .as_str()
    .is_some_and(|format| format.starts_with("pact/"));
  match payload["variant"].as_str() {
    Some(_) if pact => String::new(),
    Some(variant) => format!(" [{variant}]"),
    None => String::new(),
  }
}

/// `member` of `document` with its segments, when they were written and join to its text.
fn marked(document: &Value, member: &str) -> Marked {
  let text = document[member].as_str().unwrap_or_default();
  let segments: Option<Vec<Segment>> =
    serde_json::from_value(document[format!("{member}-segments")].clone()).ok();
  match segments {
    Some(segments)
      if segments
        .iter()
        .map(|segment| segment.text.as_str())
        .collect::<String>()
        == text =>
    {
      let mut marked = Marked::new();
      for segment in segments {
        marked.push(segment.kind, segment.text);
      }
      marked
    }
    _ => Marked::plain(text),
  }
}

/// `text` with every line break followed by `indent`, its segments kept.
fn indented(text: Marked, indent: &str) -> Marked {
  let mut out = Marked::new();
  for segment in text.segments() {
    for (at, piece) in segment.text.split('\n').enumerate() {
      if at > 0 {
        out.push(segment.kind, "\n");
        out.text_plain(indent.trim_start_matches('\n'));
      }
      out.push(segment.kind, piece);
    }
  }
  out
}

fn status_kind(status: &str) -> Kind {
  match status {
    "verified" => Kind::Pass,
    "failed" => Kind::Fail,
    _ => Kind::Warn,
  }
}

/// An engine error as a report says it: what failed, with its code, and its root cause; or every
/// layer of its chain (ADR 0054); then the problems it lists, each where it is.
pub(crate) fn error_text(error: &Value, full_chain: bool) -> String {
  let chain: Vec<(Option<&str>, &str)> = match error["chain"].as_array().filter(|chain| !chain.is_empty()) {
    Some(chain) => chain
      .iter()
      .map(|layer| {
        (
          layer["code"].as_str(),
          layer["message"].as_str().unwrap_or_default(),
        )
      })
      .collect(),
    None => vec![(
      error["code"].as_str(),
      error["message"].as_str().unwrap_or_default(),
    )],
  };
  let said = |(code, message): (Option<&str>, &str)| match (code, message) {
    (Some(code), "") => code.to_string(),
    (Some(code), message) => format!("{code}: {message}"),
    (None, message) => message.to_string(),
  };
  let mut text = if full_chain {
    let mut layers = chain.iter().copied().map(said);
    let mut text = layers.next().unwrap_or_default();
    for layer in layers {
      text.push_str(&format!("\n  caused by: {layer}"));
    }
    text
  } else {
    let lead = chain.iter().rposition(|(code, _)| code.is_some()).unwrap_or(0);
    let (code, message) = chain[lead];
    let code = code.or(error["code"].as_str()).unwrap_or("internal");
    let mut text = said((Some(code), message));
    if lead + 1 < chain.len() {
      text.push_str(&format!("\n  caused by: {}", chain[chain.len() - 1].1));
    }
    text
  };
  for problem in error["details"]["problems"].as_array().into_iter().flatten() {
    let message = problem["message"].as_str().unwrap_or("");
    // An empty pointer is the whole document, which needs no position.
    match problem["pointer"].as_str().filter(|pointer| !pointer.is_empty()) {
      Some(pointer) => text.push_str(&format!("\n  {pointer}: {message}")),
      None => text.push_str(&format!("\n  {message}")),
    }
  }
  text
}

/// `1 interaction`, `2 interactions`.
fn count(n: u64, one: &str, many: &str) -> String {
  match n {
    1 => format!("1 {one}"),
    n => format!("{n} {many}"),
  }
}

fn count_of(n: &Value, one: &str, many: &str) -> String {
  match n.as_u64() {
    Some(n) => count(n, one, many),
    None => format!("? {many}"),
  }
}

fn join(value: &Value) -> String {
  value
    .as_array()
    .map(|items| {
      items
        .iter()
        .map(|item| item.as_str().unwrap_or_default().to_string())
        .collect::<Vec<_>>()
        .join(", ")
    })
    .unwrap_or_default()
}

#[cfg(test)]
mod tests {
  use super::*;
  use pretty_assertions::assert_eq;

  fn text(reporter: &mut Reporter, kind: &str, payload: Value) -> String {
    reporter
      .render(kind, &payload)
      .map(|text| text.text())
      .unwrap_or_default()
  }

  fn reporter() -> Reporter {
    Reporter::new(ReportOptions::default(), true)
  }

  /// The status is marked by what it says, and so is each tally that is not zero.
  #[test]
  fn a_summary_marks_its_status_and_the_tallies_that_are_not_zero() {
    let summary = json!({
      "status": "failed", "interactions": 2,
      "variants": { "total": 2, "verified": 0, "failed": 2, "state-unavailable": 0, "skipped": 0 },
    });
    let marked = reporter().summary(&summary);
    assert_eq!(marked.text(), "FAILED: 0 verified, 2 failed (2 interactions)");
    let kinds: Vec<(Option<Kind>, &str)> = marked
      .segments()
      .iter()
      .map(|segment| (segment.kind, segment.text.as_str()))
      .collect();
    assert_eq!(
      kinds,
      vec![
        (Some(Kind::Fail), "FAILED"),
        (None, ": 0 verified, "),
        (Some(Kind::Fail), "2 failed"),
        (None, " (2 interactions)"),
      ]
    );
  }

  /// The tallies that are zero are left out, but for the two that always matter; variants are named
  /// only when there are more of them than interactions.
  #[test]
  fn a_summary_says_what_the_run_reached_and_how_much_there_was() {
    let contract_run = json!({
      "status": "failed", "interactions": 1, "stopped": "first-failure",
      "variants": { "total": 2, "verified": 1, "failed": 1, "state-unavailable": 1, "skipped": 0, "not-run": 3 },
    });
    assert_eq!(
      reporter().summary(&contract_run).text(),
      "FAILED: 1 verified, 1 failed, 1 state-unavailable, 3 not run (5 variants across 1 interaction)\n\
       The run stopped at its first failure, as it was asked to: what it did not reach was not run."
    );
  }

  #[test]
  fn a_result_says_its_states_its_mismatches_and_the_hook_that_failed_it() {
    let mut reporter = reporter();
    let hook =
      json!({ "hook": "auth", "point": "before-request", "outcome": "failed", "effect": "failed-exchange" });
    assert_eq!(text(&mut reporter, "verification/hook", hook), "");
    let result = json!({
      "interaction": { "description": "get a product", "format": "pact/4.0", "states": ["a product exists"] },
      "variant": "base", "status": "failed",
      "mismatches": [ { "path": "$.response.status", "message": "Expected 200 to equal 404" } ],
    });
    assert_eq!(
      text(&mut reporter, "verification/interaction-result", result),
      "  FAILED  get a product\n\
       \x20         given a product exists\n\
       \x20         $.response.status: Expected 200 to equal 404\n\
       \x20         auth at before-request: failed"
    );
  }

  #[test]
  fn a_pacts_one_variant_goes_unnamed() {
    let pact = json!({ "interaction": { "format": "pact/3.0.0" }, "variant": "base" });
    let contract = json!({ "interaction": { "format": "janus-contract/1" }, "variant": "shipped" });
    assert_eq!(variant_label(&pact), "");
    assert_eq!(variant_label(&contract), " [shipped]");
  }

  #[test]
  fn a_run_that_counted_nothing_has_no_summary_to_report() {
    let mut reporter = reporter();
    assert_eq!(
      reporter.observe(
        "verification/finished",
        &json!({ "status": "failed", "error": { "code": "pacts-not-found" } })
      ),
      vec![]
    );
  }

  #[test]
  fn each_report_carries_its_segments_when_the_host_declared_them() {
    let mut plain = Reporter::new(ReportOptions::default(), false);
    let summary = json!({ "status": "verified", "interactions": 1, "variants": { "total": 1, "verified": 1, "failed": 0 } });
    let reported = plain.observe("verification/finished", &summary);
    assert_eq!(
      reported,
      vec![(
        REPORT.to_string(),
        json!({ "text": "\nVERIFIED: 1 verified, 0 failed (1 interaction)" })
      )]
    );
    let marked = reporter().observe("verification/finished", &summary);
    assert!(marked[0].1["text-segments"].is_array());
  }

  #[test]
  fn report_options_are_an_object_or_true() {
    assert_eq!(ReportOptions::from_options(Some(&json!({}))), None);
    assert_eq!(
      ReportOptions::from_options(Some(&json!({ "report": true }))),
      Some(ReportOptions::default())
    );
    assert_eq!(
      ReportOptions::from_options(Some(&json!({ "report": { "error-chain": true } }))),
      Some(ReportOptions { error_chain: true })
    );
  }
}
