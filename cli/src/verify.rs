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

use crate::args::{Args, Spec};
use crate::engine;
use crate::io;
use serde_json::{Value, json};
use std::process::ExitCode;

pub const SPEC: Spec = Spec {
  values: &["provider-url", "config", "variant", "transport"],
  flags: &["explain-failures", "json"],
};

pub const USAGE: &str = "\
usage: janus verify <contract-or-pact>... --provider-url <url> [options]

  <contract-or-pact>...      Janus contract or v1-v4 pact files, or directories of them
  --provider-url <url>       the provider under test
  --config <file>            a verifier.janus.yaml (hooks: states, auth, anything else)
  --variant <id>             replay only this variant; repeatable. A filtered run is
                             reported as filtered — a partial run is not a pass
  --transport <kind>         transport to bind (default: http)
  --explain-failures         print the executed plan for every variant that failed
  --json                     print the run summary as JSON instead of prose";

pub fn run(args: &Args) -> ExitCode {
  let paths = args.positionals();
  if paths.is_empty() {
    return io::usage("verify", "missing <contract-or-pact>", USAGE);
  }
  let Some(provider_url) = args.value("provider-url") else {
    return io::usage("verify", "missing --provider-url", USAGE);
  };

  let mut documents = Vec::new();
  for path in paths {
    match io::read_documents(path) {
      Ok(found) => documents.extend(found),
      Err(err) => return io::fail(&format!("janus verify: {err}")),
    }
  }
  if documents.is_empty() {
    return io::fail(&format!(
      "janus verify: no contract or pact documents found in {}",
      paths.join(", ")
    ));
  }

  let hooks = match args.value("config") {
    None => None,
    Some(path) => match pact_janus_hooks_host::loader::load(path) {
      Ok(document) => Some(document),
      Err(err) => return io::fail(&format!("janus verify: reading {path}: {err}")),
    },
  };

  let mut options = json!({});
  let variants = args.values("variant");
  if !variants.is_empty() {
    options["variants"] = json!(variants);
  }
  if args.flag("explain-failures") {
    options["executed-plan"] = json!("on-failure");
  }

  let mut target = json!({
    "transports": [{
      "transport": args.value("transport").unwrap_or("http"),
      "options": { "base-url": provider_url },
    }]
  });
  if let Some(hooks) = hooks {
    target["hooks"] = hooks;
  }

  let mut engine = match engine::start() {
    Ok(engine) => engine,
    Err(err) => return io::fail(&format!("janus verify: {err}")),
  };
  let started = engine::call(
    &mut engine,
    "verification/verify",
    json!({
      "source": { "kind": "inline", "contracts": documents.iter().map(|(_, doc)| doc.clone()).collect::<Vec<_>>() },
      "target": target,
      "options": options,
    }),
  );
  let started = match started {
    Ok(result) => result,
    Err(err) => return io::fail(&format!("janus verify: {err}")),
  };
  let stream = started["stream"].as_str().unwrap_or_default().to_string();

  let mut summary = Value::Null;
  let json_output = args.flag("json");
  loop {
    let polled = match engine::call(
      &mut engine,
      "events/poll",
      json!({ "streams": [stream], "wait-ms": 10_000 }),
    ) {
      Ok(polled) => polled,
      Err(err) => return io::fail(&format!("janus verify: {err}")),
    };
    let events = polled["events"].as_array().cloned().unwrap_or_default();
    for event in &events {
      if !json_output {
        report(event);
      }
      if event["last"] == json!(true) {
        summary = event["payload"].clone();
      }
    }
    if events.iter().any(|event| event["last"] == json!(true)) {
      break;
    }
    if events.is_empty() {
      return io::fail("janus verify: the run stopped producing events");
    }
  }

  if json_output {
    println!("{}", serde_json::to_string_pretty(&summary).unwrap_or_default());
  } else {
    println!();
    println!("{}", render_summary(&summary));
  }
  match summary["status"].as_str() {
    Some("verified") => ExitCode::SUCCESS,
    _ => ExitCode::from(1),
  }
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
      println!(
        "Verifying {} interaction(s) in {} document(s) [{formats}] against {providers}",
        payload["interactions"], payload["contracts"]
      );
      if payload["filtered"] == json!(true) {
        println!("  (filtered run: not every recorded variant will be replayed)");
      }
      println!();
    }
    "verification/interaction-result" => {
      let description = payload["interaction"]["description"].as_str().unwrap_or("?");
      let variant = payload["variant"].as_str().unwrap_or("?");
      match payload["status"].as_str().unwrap_or("failed") {
        "verified" => println!("  ok      {description} [{variant}]"),
        "state-unavailable" => {
          println!("  STATE   {description} [{variant}]");
          println!("          the provider cannot reach a state this contract declares");
          print_error(&payload["error"]);
        }
        _ => {
          println!("  FAILED  {description} [{variant}]");
          for mismatch in payload["mismatches"].as_array().into_iter().flatten() {
            println!(
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
      println!("  warn    {}", payload["code"].as_str().unwrap_or("warning"));
    }
    // `--explain-failures` is this event arriving: the executed plan for the variant that just
    // failed, printed where the failure is, rather than left for a second command to re-derive.
    "verification/executed-plan" => {
      println!();
      println!(
        "  --- executed plan: {} [{}] ---",
        payload["interaction"]["description"].as_str().unwrap_or("?"),
        payload["variant"].as_str().unwrap_or("?")
      );
      for line in payload["text"].as_str().unwrap_or("").lines() {
        println!("  {line}");
      }
      println!();
    }
    "verification/hook" => {
      let outcome = payload["outcome"].as_str().unwrap_or("ok");
      if outcome != "continue" && outcome != "ok" {
        println!(
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
    other => println!("  event   {other}: {payload}"),
  }
}

fn print_error(error: &Value) {
  if error.is_null() {
    return;
  }
  println!("          {}", engine::render_error(error).replace('\n', "\n  "));
}

fn render_summary(summary: &Value) -> String {
  let variants = &summary["variants"];
  // `variants.total` counts what the run reached. An aborted run reached less than it planned, and
  // the rest is in the hook report as not run (lifecycle-hooks spec §10.2) — counted beside the
  // tallies, never folded into them, so "of 0 variant(s)" does not read as a run with nothing in it.
  let not_run = summary["hooks"]["aborted"]["exchanges-not-run"]
    .as_u64()
    .unwrap_or(0);
  let mut text = format!(
    "{}: {} verified, {} failed, {} state-unavailable, {} skipped{} (of {} variant(s) across {} interaction(s))",
    summary["status"].as_str().unwrap_or("failed").to_uppercase(),
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
  );
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
