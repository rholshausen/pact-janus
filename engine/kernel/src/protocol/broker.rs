//! `broker/publish-contracts` (engine-protocol spec §8.7, ADR 0032 decision 4): a consumer's
//! contracts published to a Pact Broker for one of its versions, in a broker session.
//!
//! The split is the one `verification/verify` draws (spec §10.2). Everything wrong with the
//! *request* — a document that is neither a Janus contract nor a v1–v4 pact, contracts from two
//! consumers — is the operation's error, before anything reaches the network. Everything after
//! that is events: one `broker/published` per contract, then `broker/finished`, which ends the
//! session. A broker that refuses the publication, or cannot be reached, is an outcome of that
//! kind, carrying the client's own error.
//!
//! What is published is the document as the host sent it, not the engine's re-writing of it: the
//! contract model keeps only the members this engine knows, and a newer writer's additions would
//! be lost on the way to the broker. It is written as compact JSON with its members in name order,
//! so the same document is the same bytes however it arrived, and `$format`, which sorts first,
//! still opens it (contract-file spec §2.3).

use super::events::Stream;
use super::frame::EngineError;
use super::verification::{self, Run, VerifyError};
use crate::broker::{BrokerClient, BrokerDescriptor, ContractToPublish, ContractsToPublish};
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::Arc;

/// `broker/publish-contracts`'s request body (`broker.schema.json`'s `PublishContracts`).
#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct PublishContracts {
  broker: BrokerDescriptor,
  consumer_version: String,
  #[serde(default)]
  branch: Option<String>,
  #[serde(default)]
  tags: Vec<String>,
  #[serde(default)]
  build_url: Option<String>,
  contracts: Vec<Value>,
}

/// A request that has been checked: every document read, all of them one consumer's.
pub(crate) struct Publication {
  broker: BrokerDescriptor,
  consumer: String,
  consumer_version: String,
  branch: Option<String>,
  tags: Vec<String>,
  build_url: Option<String>,
  contracts: Vec<ContractToPublish>,
}

impl PublishContracts {
  /// Read every document as `verification/verify` reads an inline one, so a publication can only
  /// carry what a verifier could verify, and check that one consumer owns them all.
  pub(crate) fn prepare(self) -> Result<Publication, EngineError> {
    if self.consumer_version.trim().is_empty() {
      return Err(EngineError::malformed_frame(
        "consumer-version: the consumer version these contracts belong to",
      ));
    }
    if self.contracts.is_empty() {
      return Err(EngineError::malformed_frame(
        "contracts: at least one contract to publish",
      ));
    }
    let mut consumer: Option<(usize, String)> = None;
    let mut contracts = Vec::with_capacity(self.contracts.len());
    for (index, document) in self.contracts.iter().enumerate() {
      let source = verification::read_document(index, document)
        .map_err(|err| super::engine::verify_error(repointed(err)))?;
      match &consumer {
        None => consumer = Some((index, source.consumer().to_string())),
        Some((first, name)) if name != source.consumer() => {
          return Err(EngineError::malformed_frame(format!(
            "contracts/{index} is for the consumer '{}' and contracts/{first} for '{name}': one publication is one consumer's",
            source.consumer()
          )));
        }
        Some(_) => {}
      }
      let mut content = serde_json::to_vec(document)
        .map_err(|err| EngineError::internal(format!("writing contracts/{index}: {err}")))?;
      content.push(b'\n');
      contracts.push(ContractToPublish {
        provider: source.provider().to_string(),
        content,
      });
    }
    Ok(Publication {
      broker: self.broker,
      consumer: consumer.map(|(_, name)| name).unwrap_or_default(),
      consumer_version: self.consumer_version,
      branch: self.branch,
      tags: self.tags,
      build_url: self.build_url,
      contracts,
    })
  }
}

/// A document's problems point into this request, whose contracts are at `/contracts`, rather than
/// into a verification's `/source/contracts`.
fn repointed(err: VerifyError) -> VerifyError {
  match err {
    VerifyError::ContractInvalid { problems } => VerifyError::ContractInvalid {
      problems: problems
        .into_iter()
        .map(|mut problem| {
          if let Some(rest) = problem.pointer.strip_prefix("/source") {
            problem.pointer = rest.to_string();
          }
          problem
        })
        .collect(),
    },
    other => other,
  }
}

/// Start the publication on its own thread, as a verification run is started: the operation
/// answers at once, and the outcome arrives on `stream`.
pub(crate) fn start(client: Arc<dyn BrokerClient>, publication: Publication, stream: Arc<Stream>) -> Run {
  let thread_stream = Arc::clone(&stream);
  let thread = std::thread::spawn(move || {
    let stream = thread_stream;
    let shown = publication.broker.display_url();
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
      client.publish_contracts(&ContractsToPublish {
        broker: &publication.broker,
        consumer: &publication.consumer,
        consumer_version: &publication.consumer_version,
        branch: publication.branch.as_deref(),
        tags: &publication.tags,
        build_url: publication.build_url.as_deref(),
        contracts: &publication.contracts,
      })
    }));
    let (notices, error) = match outcome {
      Ok(Ok(published)) => (published.notices, None),
      Ok(Err(error)) => (Vec::new(), Some(EngineError::component_failed("broker", &error))),
      Err(panic) => {
        let detail = panic
          .downcast_ref::<&str>()
          .map(|text| text.to_string())
          .or_else(|| panic.downcast_ref::<String>().cloned())
          .unwrap_or_else(|| "no detail".to_string());
        tracing::error!(stream = %stream.id(), %detail, "the publication panicked");
        (
          Vec::new(),
          Some(EngineError::internal(format!(
            "the publication stopped unexpectedly: {detail}"
          ))),
        )
      }
    };
    let error = error.map(|error| serde_json::to_value(error).unwrap_or(Value::Null));

    // The broker accepts or refuses a publication whole, so every contract shares its outcome.
    let mut results = Vec::with_capacity(publication.contracts.len());
    for contract in &publication.contracts {
      let mut result = json!({
        "consumer": publication.consumer,
        "provider": contract.provider,
        "outcome": if error.is_none() { "published" } else { "failed" },
      });
      if let Some(error) = &error {
        result["error"] = error.clone();
      }
      stream.emit("broker/published", result.clone());
      results.push(result);
    }
    let mut finished = json!({
      "status": if error.is_none() { "published" } else { "failed" },
      "broker": shown,
      "consumer": publication.consumer,
      "consumer-version": publication.consumer_version,
      "contracts": results,
      "notices": notices,
    });
    if let Some(error) = error {
      finished["error"] = error;
    }
    stream.finish("broker/finished", finished);
  });
  Run { stream, thread }
}
