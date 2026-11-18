//! `janus publish` (staged plan S1.3, broker notes §11; S4.3b, ADR 0093): Janus contracts or v1–v4
//! pacts, published to a Pact Broker for a version of their consumer, and provider shapes, published
//! for a version of their provider.
//!
//! Through the protocol, as every command is: the engine's broker client publishes them
//! (`broker/publish-contracts` and `broker/publish-provider-shape`, engine-protocol spec §8.7,
//! ADR 0032), and this command learns each outcome from its broker session's events. Each document
//! is identified by what it says it is (ADR 0011): a provider shape by its `$format`, and anything
//! else is a contract or a pact, which the engine reads before it publishes any, so a file that is
//! neither is refused by name before the broker is asked. One publication of contracts is one
//! consumer's, so they are grouped by the consumer they name and published a consumer at a time,
//! each for the same version, as today's publisher does. Each shape is a publication of its own.

use crate::Outcome;
use crate::args::{Output, PublishArgs};
use crate::engine;
use crate::error::{self, Error};
use crate::io;
use crate::style::{self, paint};
use clap::error::ErrorKind;
use pact_janus_kernel::protocol::Engine;
use serde_json::{Value, json};

/// How long one `events/poll` may wait. As in `verify`, an empty answer only means nothing happened
/// within it: the publication is over at its terminal event.
const POLL_WAIT_MS: u64 = 10_000;

pub fn run(args: &PublishArgs) -> Result<Outcome, Error> {
  let documents = io::read_all_publishable(&args.sources)?;
  let mut consumers: Vec<(String, Vec<Value>)> = Vec::new();
  let mut shapes: Vec<(String, Value)> = Vec::new();
  for (path, document) in documents {
    if io::is_provider_shape(&document) {
      shapes.push((path, document));
      continue;
    }
    let consumer = document["consumer"]["name"]
      .as_str()
      .unwrap_or_default()
      .to_string();
    match consumers.iter_mut().find(|(name, _)| *name == consumer) {
      Some((_, contracts)) => contracts.push(document),
      None => consumers.push((consumer, vec![document])),
    }
  }
  let consumer_version = match (&args.consumer_app_version, consumers.is_empty()) {
    (Some(version), false) => Some(version.as_str()),
    (None, false) => {
      return Err(error::usage(
        &["publish"],
        ErrorKind::MissingRequiredArgument,
        "--consumer-app-version <VERSION> is required to publish contracts",
      ));
    }
    (_, true) => None,
  };
  let provider_version = match (&args.provider_app_version, shapes.is_empty()) {
    (Some(version), false) => Some(version.as_str()),
    (None, false) => {
      return Err(error::usage(
        &["publish"],
        ErrorKind::MissingRequiredArgument,
        "--provider-app-version <VERSION> is required to publish provider shapes",
      ));
    }
    (_, true) => None,
  };

  // The engine reads no credential from its environment (spec §8.7): the CLI does, by today's
  // names, and passes them in the descriptor.
  let broker = descriptor(
    &args.broker_base_url,
    args.broker_token.as_deref(),
    args.broker_username.as_deref(),
    args.broker_password.as_deref(),
  );

  let text = args.output == Output::Text;
  let mut engine = engine::start(match args.output {
    Output::Text => engine::Prints::Text,
    Output::Json => engine::Prints::Documents,
  })?;
  let mut publications = Vec::with_capacity(consumers.len() + shapes.len());
  let mut failed = Vec::new();
  for (consumer, contracts) in consumers {
    let version = consumer_version.unwrap_or_default();
    let mut body = json!({ "broker": broker, "consumer-version": version, "contracts": contracts });
    versioned(&mut body, args);
    if text {
      outln!(
        "{}",
        paint(
          style::HEADING,
          format!(
            "Publishing {} for {consumer} version {version}",
            match contracts.len() {
              1 => "1 contract".to_string(),
              n => format!("{n} contracts"),
            },
          )
        )
      );
    }
    let finished = session(&mut engine, "broker/publish-contracts", body, text)?;
    if finished["status"] != json!("published") {
      failed.push(format!("the contracts for {consumer} were not published"));
    }
    publications.push(finished);
  }
  for (path, shape) in shapes {
    let version = provider_version.unwrap_or_default();
    let provider = shape["provider"]["name"].as_str().unwrap_or("?").to_string();
    let mut body = json!({ "broker": broker, "provider-version": version, "provider-shape": shape });
    versioned(&mut body, args);
    if text {
      outln!(
        "{}",
        paint(
          style::HEADING,
          format!("Publishing the provider shape for {provider} version {version}")
        )
      );
    }
    let finished =
      session(&mut engine, "broker/publish-provider-shape", body, text).map_err(|err| match err {
        Error::Failed(message) => Error::Failed(format!("{path}: {message}")),
        other => other,
      })?;
    if finished["status"] != json!("published") {
      failed.push(format!("the provider shape for {provider} was not published"));
    }
    publications.push(finished);
  }

  if !text {
    outln!(
      "{}",
      crate::output::pretty(&json!({
        "status": if failed.is_empty() { "published" } else { "failed" },
        "publications": publications,
      }))
    );
  }
  if failed.is_empty() {
    return Ok(Outcome::Done);
  }
  // Not "the broker refused them": a broker nobody could reach refused nothing. Why each was not
  // published is in what was printed above it, or in the document.
  Err(Error::Failed(failed.join("; ")))
}

/// A broker descriptor (spec §8.7) from what the command line and the environment said: a token,
/// else a username and password.
pub fn descriptor(url: &str, token: Option<&str>, username: Option<&str>, password: Option<&str>) -> Value {
  let mut broker = json!({ "url": url });
  if let Some(token) = token {
    broker["auth"] = json!({ "kind": "bearer", "token": token });
  } else if let Some(user) = username {
    broker["auth"] = json!({ "kind": "basic", "username": user, "password": password.unwrap_or_default() });
  }
  broker
}

/// The version's branch, tags and build URL, which a contract publication and a shape publication
/// both take.
fn versioned(body: &mut Value, args: &PublishArgs) {
  if let Some(branch) = &args.branch {
    body["branch"] = json!(branch);
  }
  if !args.tags.is_empty() {
    body["tags"] = json!(args.tags);
  }
  if let Some(build_url) = &args.build_url {
    body["build-url"] = json!(build_url);
  }
}

/// Start a broker session with `op` and drain its stream, printing each event when `text` says so:
/// the terminal event's payload.
pub fn session(engine: &mut Engine, op: &str, body: Value, text: bool) -> Result<Value, Error> {
  let started = engine::call(engine, op, body)?;
  let stream = started["stream"].as_str().unwrap_or_default().to_string();
  loop {
    let polled = engine::call(
      engine,
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
      return Ok(finished);
    }
  }
}

/// One event, rendered. As in `verify`, a kind this command does not know is printed, not dropped.
fn report(event: &Value) {
  let payload = &event["payload"];
  match event["kind"].as_str().unwrap_or_default() {
    "broker/published" => {
      let consumer = payload["consumer"].as_str().unwrap_or("?");
      let provider = payload["provider"].as_str().unwrap_or("?");
      match payload["outcome"].as_str() {
        Some("published") => outln!("  {} {consumer} -> {provider}", paint(style::PASS, "published")),
        _ => outln!("  {}    {consumer} -> {provider}", paint(style::FAIL, "FAILED")),
      }
    }
    "broker/finished" => {
      let broker = payload["broker"].as_str().unwrap_or("the broker");
      match payload["status"].as_str() {
        Some("published") => outln!("{} to {broker}", paint(style::PASS.bold(), "Published")),
        _ => outln!("{} to {broker}", paint(style::FAIL, "Not published")),
      }
      for notice in payload["notices"].as_array().into_iter().flatten() {
        outln!("  {}", paint(style::DIM, notice.as_str().unwrap_or_default()));
      }
      if let Some(error) = payload.get("error").filter(|error| !error.is_null()) {
        outln!("  {}", engine::render_error(error).replace('\n', "\n  "));
      }
      outln!();
    }
    other => outln!("  {}   {other}: {payload}", paint(style::DIM, "event")),
  }
}
