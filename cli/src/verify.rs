//! `janus verify` (plan task 5.5): source documents in, a provider under test, a verdict out.
//!
//! **Everything this command knows about a run, it learns from events.** It starts the run with
//! one `verification/verify` frame and then polls the stream until the terminal event (spec §9.2:
//! termination is structural, so this loop reads `last` and never a kind vocabulary). There is no
//! second channel and no Rust callback — which is what makes the rendering below something an SDK
//! could reproduce exactly, and why the RFC insists hook activity and per-variant results are
//! events rather than log lines.
//!
//! Sources are read as *documents*, never sniffed: a Janus contract identifies itself (ADR 0011)
//! and a v1–v4 pact is read as one (plan task 5.4). Both are handed to the engine untouched, which
//! is why `janus verify` needs no flag saying which kind of file it was given.

use crate::Outcome;
use crate::args::{Output, VerifyArgs};
use crate::engine;
use crate::error::Error;
use crate::io;
use serde_json::{Value, json};

/// How long one `events/poll` may wait for something to happen. An empty answer only means
/// nothing did within it: the run is over when its terminal event arrives, and only then (protocol
/// spec §9.6 — a run that cannot complete still sends one).
const POLL_WAIT_MS: u64 = 10_000;

pub fn run(args: &VerifyArgs) -> Result<Outcome, Error> {
  let documents = io::read_all_documents(&args.sources)?;

  let hooks = match &args.config {
    None => None,
    Some(path) => Some(
      pact_janus_hooks_host::loader::load(path)
        .map_err(|err| format!("reading {}: {err}", path.display()))?,
    ),
  };

  let mut options = json!({});
  if !args.variants.is_empty() {
    options["variants"] = json!(args.variants);
  }
  if args.explain_failures {
    options["executed-plan"] = json!("on-failure");
  }

  let mut target = json!({
    "transports": [{
      "transport": args.transport,
      "options": { "base-url": args.provider_url },
    }]
  });
  if let Some(mut hooks) = hooks {
    // One file, two members, two owners (lifecycle-hooks spec §6.1): the components are design
    // 2.6's and ride beside the hooks, not inside them.
    if let Some(components) = hooks
      .as_object_mut()
      .and_then(|config| config.remove("components"))
    {
      target["components"] = components;
    }
    target["hooks"] = hooks;
  }

  let mut engine = engine::start()?;
  let started = engine::call(
    &mut engine,
    "verification/verify",
    json!({
      "source": { "kind": "inline", "contracts": documents.into_iter().map(|(_, doc)| doc).collect::<Vec<_>>() },
      "target": target,
      "options": options,
    }),
  )?;
  let stream = started["stream"].as_str().unwrap_or_default().to_string();

  let text = args.output == Output::Text;
  let summary = loop {
    let polled = engine::call(
      &mut engine,
      "events/poll",
      json!({ "streams": [stream], "wait-ms": POLL_WAIT_MS }),
    )?;
    let events = polled["events"].as_array().cloned().unwrap_or_default();
    if events.is_empty() {
      tracing::debug!(stream, "no events within {POLL_WAIT_MS}ms; still waiting");
      continue;
    }
    let mut last = None;
    for event in events {
      if text {
        report(&event);
      }
      if event["last"] == json!(true) {
        last = Some(event["payload"].clone());
      }
    }
    if let Some(summary) = last {
      break summary;
    }
  };

  if text {
    outln!();
    outln!("{}", render_summary(&summary));
  } else {
    outln!("{}", crate::output::pretty(&summary));
  }
  Ok(match summary["status"].as_str() {
    Some("verified") => Outcome::Done,
    _ => Outcome::SubjectFailed,
  })
}

/// One event, rendered. The vocabulary is open (spec §9.6), so an unrecognised kind is *printed*
/// rather than dropped: a host that silently ignored an event kind it did not know would hide
/// exactly the information a newer engine added to tell it something.
fn report(event: &Value) {
  let payload = &event["payload"];
  match event["kind"].as_str().unwrap_or_default() {
    "verification/started" => {
      let providers = join(&payload["providers"]);
      let formats = join(&payload["formats"]);
      outln!(
        "Verifying {} interaction(s) in {} document(s) [{formats}] against {providers}",
        payload["interactions"],
        payload["contracts"]
      );
      if payload["filtered"] == json!(true) {
        outln!("  (filtered run: not every recorded variant will be replayed)");
      }
      outln!();
    }
    "verification/interaction-result" => {
      let description = payload["interaction"]["description"].as_str().unwrap_or("?");
      let variant = payload["variant"].as_str().unwrap_or("?");
      match payload["status"].as_str().unwrap_or("failed") {
        "verified" => outln!("  ok      {description} [{variant}]"),
        "state-unavailable" => {
          outln!("  STATE   {description} [{variant}]");
          outln!("          the provider cannot reach a state this contract declares");
          print_error(&payload["error"]);
        }
        _ => {
          outln!("  FAILED  {description} [{variant}]");
          for mismatch in payload["mismatches"].as_array().into_iter().flatten() {
            outln!(
              "          {}: {}",
              mismatch["path"].as_str().unwrap_or(""),
              mismatch["message"].as_str().unwrap_or("")
            );
          }
          print_error(&payload["error"]);
        }
      }
    }
    "verification/warning" => {
      outln!("  warn    {}", payload["code"].as_str().unwrap_or("warning"));
    }
    // `--explain-failures` is this event arriving: the executed plan for the variant that just
    // failed, printed where the failure is, rather than left for a second command to re-derive.
    "verification/executed-plan" => {
      outln!();
      outln!(
        "  --- executed plan: {} [{}] ---",
        payload["interaction"]["description"].as_str().unwrap_or("?"),
        payload["variant"].as_str().unwrap_or("?")
      );
      for line in payload["text"].as_str().unwrap_or("").lines() {
        outln!("  {line}");
      }
      outln!();
    }
    "verification/hook" => {
      let outcome = payload["outcome"].as_str().unwrap_or("ok");
      if outcome != "continue" && outcome != "ok" {
        outln!(
          "  hook    {} at {}: {outcome}",
          payload["hook"].as_str().unwrap_or("?"),
          payload["point"].as_str().unwrap_or("?")
        );
        print_error(&payload["error"]);
      }
    }
    // The result line that follows says everything these two carry; printing them as well would
    // double every run's output for no reader's benefit.
    "verification/interaction-started" | "verification/finished" => {}
    other => outln!("  event   {other}: {payload}"),
  }
}

fn print_error(error: &Value) {
  if error.is_null() {
    return;
  }
  outln!("          {}", engine::render_error(error).replace('\n', "\n  "));
}

fn render_summary(summary: &Value) -> String {
  let status = summary["status"].as_str().unwrap_or("failed").to_uppercase();
  let variants = &summary["variants"];
  let mut text = if variants.is_object() {
    // `variants.total` counts what the run reached. An aborted run reached less than it planned,
    // and the rest is in the hook report as not run (lifecycle-hooks spec §10.2) — counted beside
    // the tallies, never folded into them, so "of 0 variant(s)" does not read as a run with nothing
    // in it.
    let not_run = summary["hooks"]["aborted"]["exchanges-not-run"]
      .as_u64()
      .unwrap_or(0);
    format!(
      "{status}: {} verified, {} failed, {} state-unavailable, {} skipped{} (of {} variant(s) across {} interaction(s))",
      variants["verified"],
      variants["failed"],
      variants["state-unavailable"],
      variants["skipped"],
      if not_run > 0 {
        format!(", {not_run} not run")
      } else {
        String::new()
      },
      variants["total"].as_u64().unwrap_or(0) + not_run,
      summary["interactions"],
    )
  } else {
    // A run the engine could not complete has no tallies to report (protocol spec §9.6).
    format!(
      "{status}: the run did not complete ({} interaction(s) in {} document(s))",
      summary["interactions"], summary["contracts"]
    )
  };
  if summary["filtered"] == json!(true) {
    text.push_str("\nThis was a filtered run: unreplayed variants are not passing ones.");
  }
  // The abort is the headline of a run that ended this way, so it carries its cause: "the auth
  // hook failed" sends a reader to the logs for what the report already knows.
  if let Some(abort) = summary.get("aborted").filter(|a| !a.is_null()) {
    text.push_str(&format!(
      "\nThe run was aborted by the '{}' hook at {}",
      abort["hook"].as_str().unwrap_or("?"),
      abort["point"].as_str().unwrap_or("?")
    ));
    match abort.get("error").filter(|e| !e.is_null()) {
      Some(error) => text.push_str(&format!(
        ":\n  {}",
        engine::render_error(error).replace('\n', "\n  ")
      )),
      None => text.push('.'),
    }
  }
  if let Some(error) = summary.get("error").filter(|e| !e.is_null()) {
    text.push_str(&format!(
      "\nThe engine stopped the run:\n  {}",
      engine::render_error(error).replace('\n', "\n  ")
    ));
  }
  text
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
  use super::render_summary;
  use pretty_assertions::assert_eq;
  use serde_json::json;

  /// A run the engine could not complete ends with a summary that has no tallies, and an error
  /// (protocol spec §9.6). It reads as a failure with its cause, not as a run of nulls.
  #[test]
  fn a_run_that_did_not_complete_says_so_and_why() {
    let summary = json!({
      "status": "failed",
      "contracts": 1,
      "interactions": 2,
      "error": { "code": "internal", "message": "the verification run stopped unexpectedly: boom" },
    });
    assert_eq!(
      render_summary(&summary),
      "FAILED: the run did not complete (2 interaction(s) in 1 document(s))\n\
       The engine stopped the run:\n  \
       internal: the verification run stopped unexpectedly: boom"
    );
  }
}
