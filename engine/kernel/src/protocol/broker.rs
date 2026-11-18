//! The `broker/*` operations (engine-protocol spec §8.7), each in a broker session:
//! `broker/publish-contracts` (ADR 0032 decision 4), a consumer's contracts published to a Pact
//! Broker for one of its versions; `broker/publish-provider-shape` (ADR 0093), a provider's shape
//! published for one of its versions; and `broker/check-contracts` (ADR 0093), contracts checked
//! against the shapes their providers published, each report published back when asked.
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
//! so the same document is the same bytes however it arrived, its text or the document itself, and
//! `$format`, which sorts first, still opens it (contract-file spec §2.3). A provider shape is
//! written the same way, and so is the contract a subsumption report is published with, so the
//! broker gives the report's contract the SHA it gives the contract's publication (ADR 0093
//! decision 4).
//!
//! `check-contracts` checks; it does not decide. Its reports go back to the host, which decides
//! over them with `subsumption/decide` beside the verification summaries and policy layers it holds
//! (ADR 0093 decision 2).

use super::background::{Background, SpawnError};
use super::events::Stream;
use super::frame::EngineError;
use super::verification::{self, Run, Source, VerifyError};
use crate::broker::{
  BrokerClient, BrokerDescriptor, ContractToPublish, ContractsToPublish, FetchedShape, ReportToPublish,
  ShapeAt, ShapeRequest, ShapeToPublish,
};
use crate::component::{ComponentError, ContentRegistry, InTreeMatcher};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::HashMap;
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
      let (source, content) = read_contract(index, document)?;
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
      contracts.push(ContractToPublish {
        provider: source.provider().to_string(),
        janus: matches!(source, Source::Janus(_)),
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

/// `contracts/<index>`, read as `verify` reads an inline document, and the bytes a publication of
/// it carries.
fn read_contract(index: usize, document: &Value) -> Result<(Source, Vec<u8>), EngineError> {
  let source = verification::read_document(index, document)
    .map_err(|err| super::engine::verify_error(repointed(err)))?;
  let content = publication_bytes(document, &format!("/contracts/{index}"))?;
  Ok((source, content))
}

/// A document as a publication carries it: compact JSON, members in name order, ending in a line
/// break, whether the host sent the document or its text (ADR 0063 decision 3).
fn publication_bytes(document: &Value, pointer: &str) -> Result<Vec<u8>, EngineError> {
  let document = crate::contract::text::document(document, pointer)
    .map_err(|problem| EngineError::contract_invalid(&[problem]))?;
  let mut content = serde_json::to_vec(document.as_ref())
    .map_err(|err| EngineError::internal(format!("writing {pointer}: {err}")))?;
  content.push(b'\n');
  Ok(content)
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

/// Start the publication in the background (ADR 0062), as a verification run is started: the
/// operation answers at once, and the outcome arrives on `stream`.
pub(crate) fn start(
  client: Arc<dyn BrokerClient>,
  publication: Publication,
  stream: Arc<Stream>,
  background: &dyn Background,
) -> Result<Run, SpawnError> {
  let thread_stream = Arc::clone(&stream);
  let task = background.spawn(
    "janus publication",
    Box::new(move || {
      let stream = thread_stream;
      let shown = publication.broker.display_url();
      let outcome = shielded(&stream, "the publication", || {
        client.publish_contracts(&ContractsToPublish {
          broker: &publication.broker,
          consumer: &publication.consumer,
          consumer_version: &publication.consumer_version,
          branch: publication.branch.as_deref(),
          tags: &publication.tags,
          build_url: publication.build_url.as_deref(),
          contracts: &publication.contracts,
        })
      });
      let (notices, error) = match outcome {
        Ok(published) => (published.notices, None),
        Err(error) => (Vec::new(), Some(error)),
      };

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
    }),
  )?;
  Ok(Run { stream, task })
}

/// `work`, with its error as the protocol's (`component-failed` naming `broker`, carrying the
/// client's own error) and a panic caught as `internal`, so nothing a client does ends a broker
/// session without its terminal event. `what` says what stopped, in the log and the error.
fn shielded<T>(
  stream: &Stream,
  what: &str,
  work: impl FnOnce() -> Result<T, ComponentError>,
) -> Result<T, Value> {
  let error = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(work)) {
    Ok(Ok(value)) => return Ok(value),
    Ok(Err(error)) => EngineError::component_failed("broker", &error),
    Err(panic) => {
      let detail = panic
        .downcast_ref::<&str>()
        .map(|text| text.to_string())
        .or_else(|| panic.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "no detail".to_string());
      tracing::error!(stream = %stream.id(), %detail, "{what} panicked");
      EngineError::internal(format!("{what} stopped unexpectedly: {detail}"))
    }
  };
  Err(serde_json::to_value(error).unwrap_or(Value::Null))
}

// ---------------------------------------------------------------------------------------------
// broker/publish-provider-shape
// ---------------------------------------------------------------------------------------------

/// `broker/publish-provider-shape`'s request body (`broker.schema.json`'s `PublishProviderShape`).
#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct PublishProviderShape {
  broker: BrokerDescriptor,
  provider_version: String,
  #[serde(default)]
  branch: Option<String>,
  #[serde(default)]
  tags: Vec<String>,
  #[serde(default)]
  build_url: Option<String>,
  provider_shape: Value,
}

/// A shape publication that has been checked: the shape read, and its bytes.
pub(crate) struct ShapePublication {
  broker: BrokerDescriptor,
  provider: String,
  provider_version: String,
  branch: Option<String>,
  tags: Vec<String>,
  build_url: Option<String>,
  content: Vec<u8>,
}

impl PublishProviderShape {
  /// Read the shape as `subsumption/check` reads one, so only a shape a check could read is
  /// published, and name the provider it is about.
  pub(crate) fn prepare(self) -> Result<ShapePublication, EngineError> {
    if self.provider_version.trim().is_empty() {
      return Err(EngineError::malformed_frame(
        "provider-version: the provider version this shape belongs to",
      ));
    }
    let content = publication_bytes(&self.provider_shape, "/provider-shape")?;
    let shape = crate::subsumption::read_provider_shape(&content)
      .map_err(|error| EngineError::document_error("provider-shape", &error))?;
    Ok(ShapePublication {
      broker: self.broker,
      provider: shape.provider.name,
      provider_version: self.provider_version,
      branch: self.branch,
      tags: self.tags,
      build_url: self.build_url,
      content,
    })
  }
}

/// Start the shape's publication in the background, as a contracts publication is started.
pub(crate) fn start_shape(
  client: Arc<dyn BrokerClient>,
  publication: ShapePublication,
  stream: Arc<Stream>,
  background: &dyn Background,
) -> Result<Run, SpawnError> {
  let thread_stream = Arc::clone(&stream);
  let task = background.spawn(
    "janus shape publication",
    Box::new(move || {
      let stream = thread_stream;
      let outcome = shielded(&stream, "the publication", || {
        client.publish_provider_shape(&ShapeToPublish {
          broker: &publication.broker,
          provider: &publication.provider,
          provider_version: &publication.provider_version,
          branch: publication.branch.as_deref(),
          tags: &publication.tags,
          build_url: publication.build_url.as_deref(),
          content: &publication.content,
        })
      });
      let mut finished = json!({
        "status": if outcome.is_ok() { "published" } else { "failed" },
        "broker": publication.broker.display_url(),
        "provider": publication.provider,
        "provider-version": publication.provider_version,
      });
      match outcome {
        Ok(published) => finished["notices"] = json!(published.notices),
        Err(error) => {
          finished["notices"] = json!([]);
          finished["error"] = error;
        }
      }
      stream.finish("broker/finished", finished);
    }),
  )?;
  Ok(Run { stream, task })
}

// ---------------------------------------------------------------------------------------------
// broker/check-contracts
// ---------------------------------------------------------------------------------------------

/// `broker/check-contracts`'s request body (`broker.schema.json`'s `CheckContracts`).
#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct CheckContracts {
  broker: BrokerDescriptor,
  contracts: Vec<Value>,
  #[serde(default)]
  provider_version: Option<String>,
  #[serde(default)]
  provider_branch: Option<String>,
  #[serde(default)]
  publish_reports: bool,
}

/// One contract to check: its document, the bytes a report about it is published with, and who it
/// is between.
struct ContractToCheck {
  document: Value,
  content: Vec<u8>,
  consumer: String,
  provider: String,
}

/// A check that has been prepared: every contract read, and which shape each is checked against.
pub(crate) struct Check {
  broker: BrokerDescriptor,
  at: ShapeAt,
  publish_reports: bool,
  contracts: Vec<ContractToCheck>,
}

impl CheckContracts {
  pub(crate) fn prepare(self) -> Result<Check, EngineError> {
    let at = match (self.provider_version, self.provider_branch) {
      (Some(version), None) => ShapeAt::Version(version),
      (None, Some(branch)) => ShapeAt::Branch(branch),
      _ => {
        return Err(EngineError::malformed_frame(
          "provider-version or provider-branch: exactly one, saying which of each provider's shapes to check against",
        ));
      }
    };
    if self.contracts.is_empty() {
      return Err(EngineError::malformed_frame(
        "contracts: at least one contract to check",
      ));
    }
    let mut contracts = Vec::with_capacity(self.contracts.len());
    for (index, document) in self.contracts.into_iter().enumerate() {
      let (source, content) = read_contract(index, &document)?;
      contracts.push(ContractToCheck {
        consumer: source.consumer().to_string(),
        provider: source.provider().to_string(),
        document,
        content,
      });
    }
    if let ShapeAt::Version(version) = &at
      && let Some(other) = contracts
        .iter()
        .find(|contract| contract.provider != contracts[0].provider)
    {
      return Err(EngineError::malformed_frame(format!(
        "provider-version '{version}' names a version of one provider, and the contracts are with '{}' and '{}': check them against a provider-branch, or one provider at a time",
        contracts[0].provider, other.provider
      )));
    }
    Ok(Check {
      broker: self.broker,
      at,
      publish_reports: self.publish_reports,
      contracts,
    })
  }
}

/// What the walk compares component operators with: the embedding's in-tree matchers and content
/// components, as `subsumption/check` has them (ADR 0087).
pub(crate) struct Comparers {
  pub matchers: Vec<InTreeMatcher>,
  pub content: ContentRegistry,
}

/// Start the check in the background: fetch each provider's shape once, walk each pair, publish
/// each report when asked, and report each contract as it is decided.
pub(crate) fn start_check(
  client: Arc<dyn BrokerClient>,
  check: Check,
  comparers: Comparers,
  stream: Arc<Stream>,
  background: &dyn Background,
) -> Result<Run, SpawnError> {
  let thread_stream = Arc::clone(&stream);
  let task = background.spawn(
    "janus broker check",
    Box::new(move || {
      let stream = thread_stream;
      let components = super::contributions::check_components(&comparers.matchers, &comparers.content);
      let mut shapes: HashMap<String, Result<Option<FetchedShape>, Value>> = HashMap::new();
      let mut results = Vec::with_capacity(check.contracts.len());
      for contract in &check.contracts {
        let shape = shapes.entry(contract.provider.clone()).or_insert_with(|| {
          shielded(&stream, "fetching the provider shape", || {
            client.fetch_provider_shape(&ShapeRequest {
              broker: &check.broker,
              provider: &contract.provider,
              at: &check.at,
            })
          })
        });
        let mut result = json!({ "consumer": contract.consumer, "provider": contract.provider });
        match shape {
          Err(error) => {
            result["outcome"] = json!("failed");
            result["error"] = error.clone();
          }
          Ok(None) => result["outcome"] = json!("not-published"),
          Ok(Some(fetched)) => {
            if let Some(version) = &fetched.provider_version {
              result["provider-version"] = json!(version);
            }
            let walked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
              super::subsumption::check(
                &super::subsumption::Check {
                  contract: contract.document.clone(),
                  provider_shape: fetched.document.clone(),
                },
                &components,
              )
            }));
            match walked {
              Ok(Ok((report, format))) => {
                let report = serde_json::to_value(&report).unwrap_or(Value::Null);
                if check.publish_reports {
                  result["publication"] =
                    publish_report(&*client, &check.broker, fetched, contract, &report, &stream);
                }
                result["outcome"] = json!("checked");
                result["format"] = json!(format);
                result["report"] = report;
              }
              Ok(Err(error)) => {
                result["outcome"] = json!("failed");
                result["error"] =
                  serde_json::to_value(super::engine::subsumption_error(error)).unwrap_or(Value::Null);
              }
              Err(_) => {
                tracing::error!(stream = %stream.id(), "the subsumption check panicked");
                result["outcome"] = json!("failed");
                result["error"] = serde_json::to_value(EngineError::internal(
                  "the subsumption check stopped unexpectedly",
                ))
                .unwrap_or(Value::Null);
              }
            }
          }
        }
        stream.emit("broker/checked", result.clone());
        results.push(result);
      }
      let failed = results.iter().any(|result| result["outcome"] == json!("failed"));
      stream.finish(
        "broker/finished",
        json!({
          "status": if failed { "failed" } else { "checked" },
          "broker": check.broker.display_url(),
          "results": results,
        }),
      );
    }),
  )?;
  Ok(Run { stream, task })
}

/// One report's publication, as the `publication` member of its contract's result.
fn publish_report(
  client: &dyn BrokerClient,
  broker: &BrokerDescriptor,
  shape: &FetchedShape,
  contract: &ContractToCheck,
  report: &Value,
  stream: &Stream,
) -> Value {
  if !shape.reportable {
    return json!({ "outcome": "skipped", "reason": "not-supported" });
  }
  match shielded(stream, "publishing the report", || {
    client.publish_report(&ReportToPublish {
      broker,
      handle: &shape.handle,
      contract: &contract.content,
      report,
    })
  }) {
    Ok(()) => json!({ "outcome": "published" }),
    Err(error) => json!({ "outcome": "failed", "error": error }),
  }
}
