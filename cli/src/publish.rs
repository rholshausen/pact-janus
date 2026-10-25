//! `janus publish` (staged plan S1.3, broker notes §11): Janus contracts or v1–v4 pacts, published
//! to a Pact Broker for a version of their consumer.
//!
//! Through the protocol, as every command is: the engine's broker client publishes them
//! (`broker/publish-contracts`, engine-protocol spec §8.7, ADR 0032), and this command learns the
//! outcome from the broker session's events. One publication is one consumer's, so the documents
//! are grouped by the consumer they name and published a consumer at a time, each for the same
//! version, as today's publisher does. The engine reads every document before it publishes any,
//! so a file that is neither a contract nor a pact is refused by name before the broker is asked.

use crate::Outcome;
use crate::args::{Output, PublishArgs};
use crate::engine;
use crate::error::Error;
use crate::io;
use serde_json::{Value, json};

/// How long one `events/poll` may wait. As in `verify`, an empty answer only means nothing happened
/// within it: the publication is over at its terminal event.
const POLL_WAIT_MS: u64 = 10_000;

pub fn run(args: &PublishArgs) -> Result<Outcome, Error> {
  let documents = io::read_all_documents(&args.sources)?;
  let mut consumers: Vec<(String, Vec<Value>)> = Vec::new();
  for (_, document) in documents {
    let consumer = document["consumer"]["name"]
      .as_str()
      .unwrap_or_default()
      .to_string();
    match consumers.iter_mut().find(|(name, _)| *name == consumer) {
      Some((_, contracts)) => contracts.push(document),
      None => consumers.push((consumer, vec![document])),
    }
  }

  // The engine reads no credential from its environment (spec §8.7): the CLI does, by today's
  // names, and passes them in the descriptor.
  let mut broker = json!({ "url": args.broker_base_url });
  if let Some(token) = &args.broker_token {
    broker["auth"] = json!({ "kind": "bearer", "token": token });
  } else if let Some(user) = &args.broker_username {
    broker["auth"] = json!({
      "kind": "basic",
      "username": user,
      "password": args.broker_password.clone().unwrap_or_default(),
    });
  }

  let text = args.output == Output::Text;
  let mut engine = engine::start()?;
  let mut publications = Vec::with_capacity(consumers.len());
  for (consumer, contracts) in consumers {
    let mut body = json!({
      "broker": broker,
      "consumer-version": args.consumer_app_version,
      "contracts": contracts,
    });
    if let Some(branch) = &args.branch {
      body["branch"] = json!(branch);
    }
    if !args.tags.is_empty() {
      body["tags"] = json!(args.tags);
    }
    if let Some(build_url) = &args.build_url {
      body["build-url"] = json!(build_url);
    }
    if text {
      outln!(
        "Publishing {} for {consumer} version {}",
        match contracts.len() {
          1 => "1 contract".to_string(),
          n => format!("{n} contracts"),
        },
        args.consumer_app_version
      );
    }
    let started = engine::call(&mut engine, "broker/publish-contracts", body)?;
    let stream = started["stream"].as_str().unwrap_or_default().to_string();
    let finished = loop {
      let polled = engine::call(
        &mut engine,
        "events/poll",
        json!({ "streams": [stream], "wait-ms": POLL_WAIT_MS }),
      )?;
      let mut last = None;
      for event in polled["events"].as_array().into_iter().flatten() {
        if text {
          report(event);
        }
        if event["last"] == json!(true) {
          last = Some(event["payload"].clone());
        }
      }
      if let Some(finished) = last {
        break finished;
      }
    };
    publications.push(finished);
  }

  let published = publications
    .iter()
    .all(|publication| publication["status"] == json!("published"));
  if !text {
    outln!(
      "{}",
      crate::output::pretty(&json!({
        "status": if published { "published" } else { "failed" },
        "publications": publications,
      }))
    );
  }
  if published {
    return Ok(Outcome::Done);
  }
  // Not "the broker refused them": a broker nobody could reach refused nothing. Why each was not
  // published is in what was printed above it, or in the document.
  let failed: Vec<String> = publications
    .iter()
    .filter(|publication| publication["status"] != json!("published"))
    .filter_map(|publication| publication["consumer"].as_str())
    .map(str::to_string)
    .collect();
  Err(Error::Failed(format!(
    "the contracts for {} were not published",
    failed.join(", ")
  )))
}

/// One event, rendered. As in `verify`, a kind this command does not know is printed, not dropped.
fn report(event: &Value) {
  let payload = &event["payload"];
  match event["kind"].as_str().unwrap_or_default() {
    "broker/published" => {
      let consumer = payload["consumer"].as_str().unwrap_or("?");
      let provider = payload["provider"].as_str().unwrap_or("?");
      match payload["outcome"].as_str() {
        Some("published") => outln!("  published {consumer} -> {provider}"),
        _ => outln!("  FAILED    {consumer} -> {provider}"),
      }
    }
    "broker/finished" => {
      let broker = payload["broker"].as_str().unwrap_or("the broker");
      match payload["status"].as_str() {
        Some("published") => outln!("Published to {broker}"),
        _ => outln!("Not published to {broker}"),
      }
      for notice in payload["notices"].as_array().into_iter().flatten() {
        outln!("  {}", notice.as_str().unwrap_or_default());
      }
      if let Some(error) = payload.get("error").filter(|error| !error.is_null()) {
        outln!("  {}", engine::render_error(error).replace('\n', "\n  "));
      }
      outln!();
    }
    other => outln!("  event   {other}: {payload}"),
  }
}
