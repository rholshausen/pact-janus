//! `janus derive` (staged plan S4.2, ADR 0088): a type document in, a provider shape out, and every
//! place the mapping lost something.
//!
//! **Like `component`, this command does not speak the protocol**, because deriving is not an engine
//! operation: it reads one document and writes another, and no engine state takes part (spike 7.3 §5).
//! The shape it writes is read by `janus check` and the engine like any other, by its `$format`.
//!
//! **The losses are half the output.** A derived shape that silently admits more than its document
//! says is how a derived shape misleads, so each widening, each dropped constraint and each claim the
//! document did not make is printed by default, and "no losses" is said when there are none.

mod openapi;

use crate::Outcome;
use crate::args::{DeriveArgs, Output};
use crate::error::{self, Error};
use crate::io;
use crate::style::{self, paint};
use clap::error::ErrorKind;
use serde_json::{Value, json};

/// What the mapping needs beside the document.
pub struct Options {
  /// `--base-path`: the prefix every operation's path is under, whatever `servers` says.
  pub base_path: Option<String>,
  /// `--assume-non-empty`: an array with no `minItems` is never empty.
  pub assume_non_empty: bool,
}

/// One place the shape admits more than the document says, or claims what the document did not.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Loss {
  /// The JSON pointer, into the document, to what was lost: first, so the losses sort by where.
  pub pointer: String,
  pub code: &'static str,
  /// The entry it is in, as the shape names it: `GET /orders/{id} 200`.
  pub entry: Option<String>,
  pub message: String,
}

impl Loss {
  fn to_json(&self) -> Value {
    let mut loss = json!({ "code": self.code, "pointer": self.pointer, "message": self.message });
    if let Some(entry) = &self.entry {
      loss["entry"] = Value::from(entry.as_str());
    }
    loss
  }
}

pub fn run(args: &DeriveArgs) -> Result<Outcome, Error> {
  if args.output == Output::Json {
    for (given, flag) in [(args.write.is_some(), "--write <FILE>"), (args.quiet, "--quiet")] {
      if given {
        return Err(error::usage(
          &["derive"],
          ErrorKind::ArgumentConflict,
          format!("the argument '--output json' cannot be used with '{flag}'"),
        ));
      }
    }
  }

  let document = io::read_config(&args.document)?;
  let named = args.document.display();
  let kind = kind_of(&document).map_err(|message| format!("{named}: {message}"))?;
  let provider = match (
    &args.provider,
    document.pointer("/info/title").and_then(Value::as_str),
  ) {
    (Some(provider), _) => provider.clone(),
    (None, Some(title)) if !title.trim().is_empty() => title.trim().to_string(),
    (None, _) => {
      return Err(error::usage(
        &["derive"],
        ErrorKind::MissingRequiredArgument,
        format!("{named} has no info.title to name the provider; name it with '--provider <NAME>'"),
      ));
    }
  };
  let options = Options {
    base_path: args.base_path.clone(),
    assume_non_empty: args.assume_non_empty,
  };

  let mut losses = Vec::new();
  let interactions = match kind {
    Kind::OpenApi => openapi::interactions(&document, &options, &mut losses),
  };
  if interactions.is_empty() {
    return Err(Error::Failed(format!(
      "{named} documents no operation with a response, so there is no shape to write"
    )));
  }
  losses.sort();
  losses.dedup();
  let interactions: Vec<Value> = interactions.iter().map(canonical).collect();
  let shape = json!({
    "$format": pact_janus_kernel::persisted::PROVIDER_SHAPE_FORMAT.writes(),
    "provider": { "name": provider },
    "provenance": "derived",
    "interactions": interactions
  });

  if args.output == Output::Json {
    let losses: Vec<Value> = losses.iter().map(Loss::to_json).collect();
    outln!(
      "{}",
      crate::output::pretty(&json!({ "provider-shape": shape, "losses": losses }))
    );
    return Ok(Outcome::Done);
  }

  let mut text = crate::output::pretty(&shape);
  text.push('\n');
  match &args.write {
    Some(path) => {
      std::fs::write(path, text.as_bytes())
        .map_err(|err| format!("writing {}: {}", path.display(), io::io_error(&err)))?;
      errln!("janus derive: wrote {}", path.display());
    }
    None => crate::output::out_bytes(text.as_bytes()),
  }
  if !args.quiet {
    report(&losses);
  }
  Ok(Outcome::Done)
}

/// An entry with each slot's shape written as subsumption-check spec §2.4 says, which is how the
/// recorder writes one: a shape's own members by name, an object's members in declared order.
fn canonical(entry: &Value) -> Value {
  let mut entry = entry.clone();
  for member in ["selector", "parts"] {
    if let Some(parts) = entry.get_mut(member).and_then(Value::as_object_mut) {
      for slots in parts.values_mut().filter_map(Value::as_object_mut) {
        for shape in slots.values_mut() {
          *shape = pact_janus_kernel::shape::canonical(shape);
        }
      }
    }
  }
  entry
}

enum Kind {
  OpenApi,
}

/// Which type document this is, by its own version member, refusing what is not read.
fn kind_of(document: &Value) -> Result<Kind, String> {
  if let Some(version) = document.get("openapi").and_then(Value::as_str) {
    return if version.starts_with("3.0.") || version.starts_with("3.1.") {
      Ok(Kind::OpenApi)
    } else {
      Err(format!(
        "OpenAPI {version} is not read; janus derive reads OpenAPI 3.0 and 3.1"
      ))
    };
  }
  if let Some(version) = document.get("swagger") {
    return Err(format!(
      "Swagger {} is not read; convert it to OpenAPI 3 first",
      version
        .as_str()
        .map(str::to_string)
        .unwrap_or_else(|| version.to_string())
    ));
  }
  if document.get("asyncapi").is_some() {
    return Err("an AsyncAPI document is not read yet; janus derive reads OpenAPI 3.0 and 3.1".to_string());
  }
  Err("not an OpenAPI document: it has no 'openapi' member naming its version".to_string())
}

/// The losses, by entry, on standard error.
fn report(losses: &[Loss]) {
  errln!();
  if losses.is_empty() {
    errln!(
      "{}",
      paint(style::PASS, "No losses: the shape says what the document says.")
    );
    return;
  }
  errln!(
    "{} — the shape admits more than the document says, or claims what it did not",
    paint(style::WARN.bold(), "LOSSY")
  );
  for loss in losses {
    errln!(
      "  {} at {}",
      paint(style::HEADING, loss.code),
      paint(style::PATH, &loss.pointer)
    );
    errln!("      {}", loss.message);
    if let Some(entry) = &loss.entry {
      errln!("      {}", paint(style::DIM, format!("-> in {entry}")));
    }
  }
}
