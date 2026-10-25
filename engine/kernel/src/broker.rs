//! The Pact Broker client, as the kernel sees it (engine-protocol spec §8.3 and §8.7, ADR 0032).
//!
//! The broker client is the engine's, so no host speaks the broker's API, and it is a native host
//! capability, so the kernel knows no HTTP. This module is the seam between the two: a trait in the
//! broker's own terms — fetch the pacts for verification for these selectors, publish this result
//! for a pact that was fetched, publish these contracts — which a native crate implements over the
//! broker's HAL API and the native embeddings register
//! ([`crate::protocol::Engine::register_broker_client`]). An engine with none answers every
//! broker-facing request with `component-unavailable` naming `broker`.
//!
//! What a client fetched comes back with a `handle`: whatever the client needs to find that pact
//! again when the run publishes its result (its HAL links, for the native client). The kernel
//! carries it from the fetch to the publish and never reads it, so nothing about the broker's API
//! leaks into the run.

use crate::component::ComponentError;
use serde::Deserialize;
use serde_json::Value;

/// Which broker, and how to reach it (spec §8.7's descriptor). Credentials travel here and nowhere
/// else: `Debug` leaves them out, and [`BrokerDescriptor::display_url`] is how the broker is named
/// in anything the engine reports.
#[derive(Clone, Deserialize)]
pub struct BrokerDescriptor {
  pub url: String,
  #[serde(default)]
  pub auth: Option<BrokerAuth>,
  /// HTTP client options (TLS, proxies, timeouts, retries), the same document the HTTP component's
  /// client half takes (staged plan S1.5). Carried to the client as it stands.
  #[serde(default)]
  pub client: Option<Value>,
}

#[derive(Clone, Deserialize)]
pub struct BrokerAuth {
  pub kind: String,
  #[serde(default)]
  pub token: Option<String>,
  #[serde(default)]
  pub username: Option<String>,
  #[serde(default)]
  pub password: Option<String>,
}

/// The kind alone: the secrets are never shown.
impl std::fmt::Debug for BrokerAuth {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("BrokerAuth")
      .field("kind", &self.kind)
      .finish_non_exhaustive()
  }
}

impl std::fmt::Debug for BrokerDescriptor {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("BrokerDescriptor")
      .field("url", &self.display_url())
      .field("auth", &self.auth.as_ref().map(|auth| auth.kind.as_str()))
      .finish()
  }
}

impl BrokerDescriptor {
  /// The broker's URL with any user information removed: how the broker is named in an event, a
  /// summary, an error or a log (spec §8.7).
  pub fn display_url(&self) -> String {
    without_userinfo(&self.url)
  }
}

/// `scheme://user:secret@host/...` as `scheme://host/...`.
pub fn without_userinfo(url: &str) -> String {
  let Some((scheme, rest)) = url.split_once("://") else {
    return url.to_string();
  };
  let authority_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
  match rest[..authority_end].rfind('@') {
    Some(at) => format!("{scheme}://{}", &rest[at + 1..]),
    None => url.to_string(),
  }
}

/// `scheme://host[:port]` of `url`, without user information: where a pact fetched by its URL is
/// reached, and how it is named.
pub fn origin(url: &str) -> String {
  let url = without_userinfo(url);
  let Some((scheme, rest)) = url.split_once("://") else {
    return url;
  };
  let authority_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
  format!("{scheme}://{}", &rest[..authority_end])
}

/// What a `broker` source asks for (spec §8.3).
#[derive(Debug, Clone)]
pub struct FetchRequest {
  pub broker: BrokerDescriptor,
  pub provider: String,
  /// Passed to the broker as written: the vocabulary is the broker's.
  pub selectors: Vec<Value>,
  pub provider_branch: Option<String>,
  pub provider_tags: Vec<String>,
  pub include_pending: bool,
  pub include_wip_since: Option<String>,
}

/// One pact the broker selected.
#[derive(Debug, Clone)]
pub struct FetchedPact {
  /// The pact document, as the broker served it.
  pub document: Value,
  pub pending: bool,
  pub wip: bool,
  /// The broker's notices, as text.
  pub notices: Vec<String>,
  /// The client's own way back to this pact, handed back to [`BrokerClient::publish_results`].
  pub handle: Value,
  /// Whether a result can be published for it: a pact a broker selected can be, and one fetched by
  /// its URL when the document carries a broker's link to publish to (ADR 0053).
  pub publishable: bool,
}

/// One pact's verification result, to publish (spec §8.3's `options.publish`).
#[derive(Debug, Clone)]
pub struct PublishRequest<'a> {
  pub broker: &'a BrokerDescriptor,
  /// The [`FetchedPact::handle`] of the pact this result is for.
  pub handle: &'a Value,
  pub provider_version: &'a str,
  pub branch: Option<&'a str>,
  pub tags: &'a [String],
  pub build_url: Option<&'a str>,
  /// Every interaction verified, and every one failed counts against it, pending or not: the
  /// broker decides what a pending pact's failure means, from the provider version's branch.
  pub success: bool,
  pub results: Vec<InteractionResult>,
}

/// One interaction's part in a published result.
#[derive(Debug, Clone)]
pub struct InteractionResult {
  /// The broker's own id for the interaction (`_id` in the document it served), when it gave one.
  pub id: Option<String>,
  pub description: String,
  pub success: bool,
  /// The engine's mismatch documents (`{ path, message }`), for the client to put in the broker's
  /// terms.
  pub mismatches: Vec<Value>,
  /// The error a variant failed with before any matching, when it did.
  pub error: Option<Value>,
}

/// A consumer's contracts, to publish for one of its versions (spec §8.7's `broker/publish-contracts`).
#[derive(Debug, Clone)]
pub struct ContractsToPublish<'a> {
  pub broker: &'a BrokerDescriptor,
  /// The consumer every contract belongs to.
  pub consumer: &'a str,
  pub consumer_version: &'a str,
  pub branch: Option<&'a str>,
  pub tags: &'a [String],
  pub build_url: Option<&'a str>,
  pub contracts: &'a [ContractToPublish],
}

/// One contract: its provider, and the bytes to publish, which the broker stores as they are.
#[derive(Debug, Clone)]
pub struct ContractToPublish {
  pub provider: String,
  /// The document as UTF-8 JSON. A Janus contract and a v1–v4 pact alike: the client publishes
  /// both as the broker's `pact` specification (ADR 0011 decision 6).
  pub content: Vec<u8>,
}

/// What the broker said about a publication it accepted.
#[derive(Debug, Clone, Default)]
pub struct ContractsPublished {
  /// The broker's notices, as text: what it created, and what to do next.
  pub notices: Vec<String>,
}

/// The broker client a native embedding registers. Blocking: it is called from a run's own thread,
/// which already blocks on every replayed request.
pub trait BrokerClient: Send + Sync {
  /// The pacts for verification for `request.provider`: what the broker's selectors chose, each
  /// with its pending and work-in-progress status and notices. An error is the client's own
  /// document (`broker-unreachable`, `broker-refused`, `broker-response-invalid`; spec §10.2), which
  /// the run reports as `component-failed` naming `broker`.
  fn fetch_for_verification(&self, request: &FetchRequest) -> Result<Vec<FetchedPact>, ComponentError>;

  /// One pact, by its URL (spec §8.3's `urls` source, ADR 0053): today's `--url` and
  /// `--webhook-callback-url`. `broker` carries the credentials and client options it is fetched
  /// with, and its `url` is the pact URL's origin. It is never pending.
  fn fetch_pact(&self, broker: &BrokerDescriptor, url: &str) -> Result<FetchedPact, ComponentError>;

  /// Publish one pact's result to that pact's own link.
  fn publish_results(&self, request: &PublishRequest<'_>) -> Result<(), ComponentError>;

  /// Publish a consumer's contracts for one of its versions, all together: the broker accepts or
  /// refuses them as one publication.
  fn publish_contracts(&self, request: &ContractsToPublish<'_>)
  -> Result<ContractsPublished, ComponentError>;
}

#[cfg(test)]
mod tests {
  use super::*;
  use rstest::rstest;

  #[rstest]
  #[case(
    "https://user:secret@broker.example.com/base",
    "https://broker.example.com/base"
  )]
  #[case("https://broker.example.com/a@b", "https://broker.example.com/a@b")]
  #[case("http://127.0.0.1:9292", "http://127.0.0.1:9292")]
  #[case("not a url", "not a url")]
  fn a_broker_is_named_without_its_credentials(#[case] url: &str, #[case] shown: &str) {
    assert_eq!(without_userinfo(url), shown);
  }

  #[test]
  fn debug_output_names_the_auth_kind_and_never_the_secret() {
    let descriptor: BrokerDescriptor = serde_json::from_value(serde_json::json!({
      "url": "https://u:p@broker.example.com",
      "auth": { "kind": "bearer", "token": "s3cret" }
    }))
    .unwrap();
    let shown = format!("{descriptor:?}");
    assert!(!shown.contains("s3cret") && !shown.contains("u:p"), "{shown}");
    assert!(shown.contains("bearer"), "{shown}");
  }
}
