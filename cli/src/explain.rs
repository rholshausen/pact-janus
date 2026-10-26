//! `janus explain` (plan tasks 3.6 and 5.5): what the engine will actually do, before it does it.
//!
//! The compiling is `verification/explain`'s — a kernel operation precisely so no SDK builds its
//! own (the RFC), and so what this prints is the plan a run would really execute rather than a
//! second, display-only rendering of the same document.
//!
//! `--executed` has two forms, and the difference is the point of plan task 5.5's hardening. The
//! one that matters is `janus verify --explain-failures`, which prints the executed plan for each
//! variant that actually failed, against the response the provider actually sent. The offline form
//! here — `--executed <values.json>` — resolves a plan against a hand-written map of captured
//! values, which is what a golden-corpus case carries (plan-grammar spec §6.1) and what you reach
//! for when there is no provider to run at all. The engine runs it too, in the same call that
//! compiles the plan (staged plan S1.12, ADR 0050), so nothing here steps outside the protocol.
//!
//! `--description` picks the interaction by what it is called rather than where it is, which in a
//! pact of hundreds is the only thing a reader knows about it. The CLI turns it into the `index` the
//! operation takes, as it checks `--index` itself: which interaction of a file was meant is the
//! command line's question, answered before the engine is asked.
//!
//! `--config` names the verifier configuration whose `components` a run would load, so a plan a
//! declared component contributes a fragment to is explained as a run compiles it.

use crate::Outcome;
use crate::args::ExplainArgs;
use crate::engine;
use crate::error::{self, Error};
use crate::highlight;
use crate::io;
use clap::error::ErrorKind;
use serde_json::{Value, json};

pub fn run(args: &ExplainArgs) -> Result<Outcome, Error> {
  let document = io::read_json(&args.document)?;
  let index = match &args.description {
    Some(description) => Some(select(&document, description)?),
    None => args.index,
  };

  // The document says what it is (ADR 0011, plan task 5.5): an interaction specification is the
  // one that cannot, so it is the one that takes a flag.
  let subject = if args.spec {
    json!({ "kind": "spec", "spec": document })
  } else {
    let mut subject = json!({ "kind": "contract-interaction", "contract": document });
    if let Some(index) = index {
      subject["index"] = json!(index);
    }
    if let Some(variant) = &args.variant {
      subject["variant"] = json!(variant);
    }
    subject
  };

  let mut request = json!({ "interaction": subject, "options": { "plan": args.plan } });
  if let Some(path) = &args.config {
    let config = pact_janus_hooks_host::loader::load(path)
      .map_err(|err| format!("reading {}: {}", path.display(), crate::engine::sentence(&err)))?;
    if let Some(components) = config.get("components") {
      request["components"] = components.clone();
    }
  }
  if let Some(path) = &args.executed {
    let values = io::read_json(path)?;
    if !values.is_object() {
      return Err(Error::Failed(format!(
        "{} must be a JSON object of path -> value",
        path.display()
      )));
    }
    request["values"] = values;
  }

  if let Some(index) = args.index {
    check_index(&document, index)?;
  }

  // `--plan` prints the plan document; every other form is text.
  let prints = if args.plan {
    engine::Prints::Documents
  } else {
    engine::Prints::Text
  };
  let mut engine = engine::start(prints)?;
  let result = engine::call(&mut engine, "verification/explain", request).map_err(|err| {
    // A document with no `$format` that is also not a pact reaches the engine as a
    // contract-interaction and is refused there, as one it cannot identify. Saying which flag reads
    // it as a specification beats the refusal alone; any other error is about a document that was
    // identified, and the hint would mislead.
    let unidentified = ["contract-invalid", "contract-version-unsupported"]
      .iter()
      .any(|code| err.starts_with(&format!("{code}:")));
    if args.spec || !unidentified {
      err
    } else {
      format!(
        "{err}\n  (if {} is an interaction specification, pass --spec)",
        args.document.display()
      )
    }
  })?;

  match &args.executed {
    Some(_) => {
      let executed = &result["executed"];
      outln!("{}", highlight::member(executed, "text"));
      Ok(match executed["status"].as_str() {
        Some("matched") => Outcome::Done,
        _ => Outcome::SubjectFailed,
      })
    }
    None if args.plan => {
      outln!("{}", crate::output::pretty(&result["plan"]));
      Ok(Outcome::Done)
    }
    None => {
      outln!("{}", highlight::member(&result, "text"));
      Ok(Outcome::Done)
    }
  }
}

/// A contract's interactions, a v1–v4 pact's, or a v3 message pact's messages: each where the
/// document keeps them, in the order the engine's `index` counts.
fn interactions(document: &Value) -> Option<&Vec<Value>> {
  ["interactions", "messages"]
    .iter()
    .find_map(|member| document[member].as_array())
}

/// The position of the interaction `--description` names: the one whose description is exactly
/// `text`, or else the one whose description contains it, ignoring case. An exact match wins over
/// containing ones, so a description that is a prefix of another's can still be chosen. None, or
/// more than one, is a usage error listing what there is to choose from, by position.
fn select(document: &Value, text: &str) -> Result<usize, Error> {
  let Some(all) = interactions(document) else {
    return Err(Error::Failed(format!(
      "--description '{text}': the document holds no interactions to choose from"
    )));
  };
  let described: Vec<(usize, &str)> = all
    .iter()
    .enumerate()
    .map(|(at, interaction)| (at, interaction["description"].as_str().unwrap_or_default()))
    .collect();
  let mut matches: Vec<usize> = described
    .iter()
    .filter(|(_, description)| *description == text)
    .map(|(at, _)| *at)
    .collect();
  if matches.is_empty() {
    let lower = text.to_lowercase();
    matches = described
      .iter()
      .filter(|(_, description)| description.to_lowercase().contains(&lower))
      .map(|(at, _)| *at)
      .collect();
  }
  let problem = match matches.as_slice() {
    [only] => return Ok(*only),
    [] => format!(
      "no interaction's description contains it. The document has:\n{}",
      listing(all, 0..all.len())
    ),
    several => format!(
      "{} interactions match it; choose one with --index:\n{}",
      several.len(),
      listing(all, several.iter().copied())
    ),
  };
  Err(error::usage(
    &["explain"],
    ErrorKind::ValueValidation,
    format!("invalid value '{text}' for '--description <TEXT>': {problem}"),
  ))
}

/// `  0  a request for an order (given an order exists)`, one line per interaction at `positions`:
/// the states are what tells apart two interactions a pact describes alike.
fn listing(all: &[Value], positions: impl IntoIterator<Item = usize>) -> String {
  let positions: Vec<usize> = positions.into_iter().collect();
  let width = positions.iter().max().map_or(1, |last| last.to_string().len());
  positions
    .iter()
    .map(|&at| {
      let interaction = &all[at];
      let description = interaction["description"].as_str().unwrap_or("?");
      let states = states(interaction);
      let given = if states.is_empty() {
        String::new()
      } else {
        format!(" (given {})", states.join(", "))
      };
      format!("  {at:>width$}  {description}{given}")
    })
    .collect::<Vec<_>>()
    .join("\n")
}

/// An interaction's provider states, by name: a contract's `states` and a v3 or v4 pact's
/// `providerStates` are lists of objects with a `name`, and a v1 or v2 pact's `providerState` is one
/// string (v1 also spelled it `provider_state`).
fn states(interaction: &Value) -> Vec<&str> {
  for list in ["states", "providerStates"] {
    if let Some(states) = interaction[list].as_array() {
      return states.iter().filter_map(|state| state["name"].as_str()).collect();
    }
  }
  ["providerState", "provider_state"]
    .iter()
    .find_map(|single| interaction[single].as_str())
    .into_iter()
    .collect()
}

/// `--index` past the document's last interaction is the command line asking for something that is
/// not there: a usage error naming the flag, before the engine is asked. A document with no
/// interactions where [`interactions`] looks is left for the engine to refuse by what it is.
fn check_index(document: &Value, index: usize) -> Result<(), Error> {
  let Some(count) = interactions(document).map(Vec::len) else {
    return Ok(());
  };
  if index < count {
    return Ok(());
  }
  let has = match count {
    0 => "the document has no interactions".to_string(),
    1 => "the document has one interaction, at 0".to_string(),
    _ => format!("the document has {count} interactions, at 0 to {}", count - 1),
  };
  Err(error::usage(
    &["explain"],
    ErrorKind::ValueValidation,
    format!("invalid value '{index}' for '--index <N>': {has}"),
  ))
}

#[cfg(test)]
mod tests {
  use super::*;
  use pretty_assertions::assert_eq;

  /// What `select` refused with, as clap would print it, or the position it chose.
  fn chosen(document: &Value, text: &str) -> Result<usize, String> {
    select(document, text).map_err(|err| match err {
      Error::Usage(err) => err.to_string(),
      Error::Failed(message) | Error::Refused(message) => message,
    })
  }

  #[test]
  fn an_exact_description_wins_over_ones_that_contain_it() {
    let pact = json!({ "interactions": [
      { "description": "get an order" },
      { "description": "get an order that does not exist" },
    ]});
    assert_eq!(chosen(&pact, "get an order"), Ok(0));
    assert_eq!(chosen(&pact, "DOES NOT"), Ok(1));
  }

  /// A pact describes interactions alike when only their states differ, so the list says the states
  /// and the positions `--index` takes.
  #[test]
  fn several_matches_are_listed_with_their_states_and_positions() {
    let pact = json!({ "interactions": [
      { "description": "get an order", "providerStates": [{ "name": "an order exists" }, { "name": "it shipped" }] },
      { "description": "list orders" },
      { "description": "get an order", "providerStates": [{ "name": "no orders exist" }] },
    ]});
    let refused = chosen(&pact, "get an order").unwrap_err();
    assert!(
      refused.contains(
        "2 interactions match it; choose one with --index:\n  \
         0  get an order (given an order exists, it shipped)\n  \
         2  get an order (given no orders exist)\n"
      ),
      "{refused}"
    );
  }

  #[test]
  fn no_match_lists_every_interaction_aligned_by_position() {
    let interactions: Vec<Value> = (0..11)
      .map(|at| json!({ "description": format!("call {at}"), "providerState": "ready" }))
      .collect();
    let refused = chosen(&json!({ "interactions": interactions }), "payment").unwrap_err();
    assert!(
      refused.contains("no interaction's description contains it. The document has:\n"),
      "{refused}"
    );
    assert!(refused.contains("\n   0  call 0 (given ready)\n"), "{refused}");
    assert!(refused.contains("\n  10  call 10 (given ready)\n"), "{refused}");
  }

  /// A v3 message pact keeps its messages under `messages`; a contract its states under `states`.
  #[test]
  fn messages_and_contracts_are_chosen_from_where_they_keep_them() {
    let messages =
      json!({ "messages": [{ "description": "an order event" }, { "description": "a refund event" }] });
    assert_eq!(chosen(&messages, "refund"), Ok(1));
    let contract = json!({ "interactions": [
      { "description": "get", "states": [{ "name": "one" }] },
      { "description": "get", "states": [{ "name": "two" }] },
    ]});
    assert!(
      chosen(&contract, "get")
        .unwrap_err()
        .contains("  1  get (given two)")
    );
  }

  #[test]
  fn a_document_with_no_interactions_cannot_be_chosen_from() {
    assert_eq!(
      chosen(&json!({ "shape": {} }), "get"),
      Err("--description 'get': the document holds no interactions to choose from".to_string())
    );
  }
}
