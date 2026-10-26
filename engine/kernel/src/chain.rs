//! An error's causes, as layers (ADR 0054, engine-protocol spec §10.1, component-interfaces spec §11.1).
//!
//! An error may carry `chain`: its layers outermost first, each in its own words. The first is the
//! error itself and the last is its root cause, and the error's `message` is the layers joined, so a
//! host that reads only `message` loses nothing. A host that reads the chain can say the headline and
//! the root cause apart without parsing a sentence.
//!
//! Built with the standard library: a foreign error's causes are its `source()` chain, read by
//! [`layers_of`]. Errors are values here, so there is no `anyhow`. [`Context`] adds an outer layer to
//! a failed `Result`, as `anyhow`'s does, but the layer can carry a code and the error is still a
//! value that serializes.

use crate::component::ComponentError;
use serde::{Deserialize, Serialize};

/// One layer of an error: what it says, in its own words, and its code when it has one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Layer {
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub code: Option<String>,
  pub message: String,
}

impl Layer {
  /// A layer with no code: a cause in a library's words, or context a caller added.
  pub fn new(message: impl Into<String>) -> Self {
    Layer {
      code: None,
      message: message.into(),
    }
  }

  /// A layer with a code a host can look up.
  pub fn coded(code: impl Into<String>, message: impl Into<String>) -> Self {
    Layer {
      code: Some(code.into()),
      message: message.into(),
    }
  }
}

/// The layers' messages as one sentence: what an error's `message` says when it has a chain.
pub fn joined(layers: &[Layer]) -> String {
  layers
    .iter()
    .map(|layer| layer.message.as_str())
    .filter(|message| !message.is_empty())
    .collect::<Vec<_>>()
    .join(": ")
}

/// `err` and every cause beneath it, outermost first, each in its own words.
///
/// A library that writes its cause into its own message as well (`could not connect: refused`, then
/// `refused`) has that suffix taken off, so each cause is said once, in its own layer. A layer an
/// outer one already says in some other way is left out. `shown` is the producer's say over each
/// layer's text: `None` leaves out a layer that only names the library in the way, and the text it
/// returns is the text kept, so a producer can redact a secret from every layer.
pub fn layers_of(err: &dyn std::error::Error, shown: impl Fn(String) -> Option<String>) -> Vec<Layer> {
  let mut texts: Vec<String> = Vec::new();
  let mut cause: Option<&dyn std::error::Error> = Some(err);
  while let Some(error) = cause {
    texts.push(error.to_string());
    cause = error.source();
  }
  // An outer layer that ends with everything its cause says loses those words. Compared with the
  // cause's whole text, which may itself end with its own cause's.
  let whole = texts.clone();
  let mut repeated = vec![false; texts.len()];
  for at in 1..texts.len() {
    if let Some(own) = whole[at - 1].strip_suffix(&format!(": {}", whole[at])) {
      texts[at - 1] = own.to_string();
      repeated[at] = true;
    }
  }
  // A cause an outer layer already says in some other way is left out.
  let mut kept: Vec<String> = Vec::new();
  for (at, text) in texts.into_iter().enumerate() {
    let said = !repeated[at] && kept.iter().any(|outer| outer.contains(&text));
    if !text.is_empty() && !said {
      kept.push(text);
    }
  }
  kept.into_iter().filter_map(shown).map(Layer::new).collect()
}

/// `Connection refused`, without the `(os error 111)` an `io::Error` adds: the platform's number for
/// it, which says nothing a reader can act on. A producer passes it to [`layers_of`] as `shown`.
pub fn without_os_error(text: String) -> Option<String> {
  Some(match text.rfind(" (os error ") {
    Some(at) if text.ends_with(')') => text[..at].to_string(),
    _ => text,
  })
}

/// An error document as the protocol shapes one (engine-protocol spec §10.1), for an error that
/// travels as JSON, as a hook's does: `own` is its layer, `causes` the layers beneath, and its
/// `message` says them all. With no causes it has no `chain`.
pub fn document(code: &str, own: impl Into<String>, causes: Vec<Layer>) -> serde_json::Value {
  let own = own.into();
  if causes.is_empty() {
    return serde_json::json!({ "code": code, "message": own });
  }
  let mut chain = vec![Layer::coded(code, own)];
  chain.extend(causes);
  serde_json::json!({ "code": code, "message": joined(&chain), "chain": chain })
}

/// `err` and its causes as one sentence, for a log line: [`layers_of`] joined.
pub fn sentence(err: &dyn std::error::Error) -> String {
  joined(&layers_of(err, Some))
}

/// A failed `Result` given an outer layer, as `anyhow`'s `context` gives one, without leaving errors
/// as values: the result is a [`ComponentError`] whose chain is the new layer, then the old ones.
pub trait Context<T> {
  /// An outer layer with no code. The error keeps its code and category.
  fn context(self, message: impl Into<String>) -> Result<T, ComponentError>;
  /// An outer layer with a code, which becomes the error's code.
  fn layer(self, code: &str, message: impl Into<String>) -> Result<T, ComponentError>;
}

impl<T> Context<T> for Result<T, ComponentError> {
  fn context(self, message: impl Into<String>) -> Result<T, ComponentError> {
    self.map_err(|error| error.within(Layer::new(message)))
  }

  fn layer(self, code: &str, message: impl Into<String>) -> Result<T, ComponentError> {
    self.map_err(|error| {
      let mut error = error.within(Layer::coded(code, message));
      error.code = code.to_string();
      error
    })
  }
}

/// Any other error, read by [`layers_of`]. With no code to give it, `context` makes it `internal`:
/// a failure nobody classified is what that category is for (engine-protocol spec §10.2).
impl<T, E: std::error::Error> Context<T> for Result<T, E> {
  fn context(self, message: impl Into<String>) -> Result<T, ComponentError> {
    self.map_err(|err| ComponentError::internal(message).caused_by(layers_of(&err, Some)))
  }

  fn layer(self, code: &str, message: impl Into<String>) -> Result<T, ComponentError> {
    self.map_err(|err| {
      ComponentError {
        code: code.to_string(),
        category: "component".to_string(),
        message: message.into(),
        source: None,
        details: None,
        chain: None,
      }
      .caused_by(layers_of(&err, Some))
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use pretty_assertions::assert_eq;

  /// An error whose message may repeat its cause's, as some libraries' do.
  #[derive(Debug)]
  struct Wrapped(&'static str, Option<Box<Wrapped>>);

  impl std::fmt::Display for Wrapped {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      f.write_str(self.0)
    }
  }

  impl std::error::Error for Wrapped {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
      self.1.as_deref().map(|cause| cause as &dyn std::error::Error)
    }
  }

  fn wrapped(texts: &[&'static str]) -> Wrapped {
    let mut error: Option<Box<Wrapped>> = None;
    for text in texts.iter().rev() {
      error = Some(Box::new(Wrapped(text, error)));
    }
    *error.expect("at least one layer")
  }

  fn messages(layers: &[Layer]) -> Vec<&str> {
    layers.iter().map(|layer| layer.message.as_str()).collect()
  }

  #[test]
  fn each_cause_is_said_once_in_its_own_layer() {
    let err = wrapped(&[
      "error sending request",
      "client error (Connect): tcp connect error: Connection refused",
      "tcp connect error: Connection refused",
      "Connection refused",
    ]);
    assert_eq!(
      messages(&layers_of(&err, Some)),
      vec![
        "error sending request",
        "client error (Connect)",
        "tcp connect error",
        "Connection refused"
      ]
    );
  }

  #[test]
  fn a_cause_an_outer_layer_already_says_is_left_out() {
    let err = wrapped(&["could not read (Connection refused)", "Connection refused"]);
    assert_eq!(
      messages(&layers_of(&err, Some)),
      vec!["could not read (Connection refused)"]
    );
  }

  #[test]
  fn the_producer_leaves_out_and_rewrites_layers() {
    let err = wrapped(&["error sending request", "secret-token refused"]);
    let shown = |text: String| (text != "error sending request").then(|| text.replace("secret-token", "***"));
    assert_eq!(messages(&layers_of(&err, shown)), vec!["*** refused"]);
  }

  #[test]
  fn context_puts_a_layer_outside_and_keeps_the_code() {
    let failed: Result<(), ComponentError> =
      Err(ComponentError::transport_failed("GET http://x/").caused_by(vec![Layer::new("refused")]));
    let error = failed.context("fetching the pacts").unwrap_err();
    assert_eq!(error.code, "transport-failed");
    assert_eq!(error.message, "fetching the pacts: GET http://x/: refused");
    assert_eq!(
      error.chain,
      Some(vec![
        Layer::new("fetching the pacts"),
        Layer::coded("transport-failed", "GET http://x/"),
        Layer::new("refused"),
      ])
    );
  }

  #[test]
  fn layer_gives_a_library_error_a_code() {
    let failed: Result<(), Wrapped> = Err(wrapped(&["could not connect", "Connection refused"]));
    let error = failed
      .layer("broker-unreachable", "GET http://broker/")
      .unwrap_err();
    assert_eq!(error.code, "broker-unreachable");
    assert_eq!(error.category, "component");
    assert_eq!(
      error.message,
      "GET http://broker/: could not connect: Connection refused"
    );
    assert_eq!(
      serde_json::to_value(&error.chain).unwrap(),
      serde_json::json!([
        { "code": "broker-unreachable", "message": "GET http://broker/" },
        { "message": "could not connect" },
        { "message": "Connection refused" },
      ])
    );
  }

  #[test]
  fn an_unclassified_failure_is_internal() {
    let failed: Result<(), Wrapped> = Err(wrapped(&["bad byte"]));
    let error = failed.context("reading the archive").unwrap_err();
    assert_eq!(
      (error.code.as_str(), error.message.as_str()),
      ("internal", "reading the archive: bad byte")
    );
  }
}
