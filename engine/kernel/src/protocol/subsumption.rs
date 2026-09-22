//! The subsumption operations (engine-protocol spec §8.6, plan task 7.4): `subsumption/check`
//! walks one pair, `subsumption/decide` turns what the host has collected into the RFC's
//! `can-i-deploy` answer.
//!
//! Both are session-less — pure document-in, document-out, like `upgrade/pact` — and they are two
//! operations rather than one on purpose. `check` is the reusable primitive: one consumer
//! contract against one provider shape, producing the report design 2.8 §6 defines, which is the
//! artifact a broker would *store* (task 7.5). `decide` is the question asked at deploy time,
//! over reports that already exist and verification results that came from somewhere else
//! entirely. Folding them together would make the second impossible to ask without re-walking
//! every tree, which is precisely the coupling design 2.8 §4.3 stores `severity` to avoid.
//!
//! The policy layers arrive as a *list* (§7.1's "specification default, then project config, then
//! a per-run override"), not as one merged document, because the merge rule is the part that is
//! easy to get wrong: scalars override, `exemptions` accumulate. A host that merged them itself
//! would be reimplementing ADR 0016, and two hosts would eventually disagree.

use crate::compatibility::{self, CompatibilityReport};
use crate::contract::{self, Contract, ContractError, IdentifyMode, Party};
use crate::subsumption::{CheckError, ProviderShape, SubsumptionPolicy, SubsumptionReport, check as walk};
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub struct Check {
  /// A Janus contract, or a v1–v4 pact (converted on the way in — see [`consumer_contract`]).
  pub contract: Value,
  #[serde(rename = "provider-shape")]
  pub provider_shape: Value,
}

#[derive(Debug, Deserialize)]
pub struct Decide {
  pub pairs: Vec<DecidePair>,
  /// Verification run summaries, as the terminal event of a run carries them.
  #[serde(default)]
  pub verification: Vec<Value>,
  /// Policy layers, least specific first (design 2.8 §7.1).
  #[serde(default)]
  pub policy: Vec<Value>,
  /// The date to judge exemption expiry against (`YYYY-MM-DD`). The kernel has no clock.
  #[serde(rename = "as-of", default)]
  pub as_of: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DecidePair {
  pub consumer: Party,
  pub provider: Party,
  #[serde(default)]
  pub format: Option<String>,
  /// The subsumption report for this pair, when the host has one. Its absence is not an error:
  /// a provider that published nothing gets replay-only semantics (design 2.8 §6.3).
  #[serde(default)]
  pub subsumption: Option<Value>,
}

/// Why a `subsumption/*` operation could not run.
#[derive(Debug)]
pub enum SubsumptionError {
  /// One of the two documents is not one (ADR 0011's identification), or is structurally
  /// invalid. Carries which member it was: a check reads two documents of different kinds, and an
  /// error that did not say which would send a reader to the wrong file.
  Document {
    member: &'static str,
    error: ContractError,
  },
  /// A part/slot of either document does not parse as a shape (design 2.8 §8).
  Check(CheckError),
  /// The two documents are about different providers — a pair that cannot be checked, and a
  /// mistake worth refusing rather than reporting as "nothing matched".
  MismatchedProvider { contract: String, shape: String },
  /// A policy layer, or a supplied report, is not the document it claims to be.
  Invalid { problems: Vec<crate::error::Problem> },
}

/// `subsumption/check`: one consumer contract against one provider shape (design 2.8 §3.1).
pub fn check(request: &Check) -> Result<(SubsumptionReport, String), SubsumptionError> {
  let (contract, format) = consumer_contract(&request.contract)?;
  let shape: ProviderShape = read_provider_shape(&request.provider_shape)?;
  if contract.provider.name != shape.provider.name {
    return Err(SubsumptionError::MismatchedProvider {
      contract: contract.provider.name.clone(),
      shape: shape.provider.name.clone(),
    });
  }
  let report = walk(&contract, &shape).map_err(SubsumptionError::Check)?;
  Ok((report, format))
}

/// `subsumption/decide`: the reports, the verification results and the policy, as one answer.
pub fn decide(request: &Decide) -> Result<(CompatibilityReport, String), SubsumptionError> {
  let layers: Vec<Option<&Value>> = request.policy.iter().map(Some).collect();
  let policy = SubsumptionPolicy::resolve(&layers).map_err(|problem| SubsumptionError::Invalid {
    problems: vec![problem],
  })?;

  let mut pairs = Vec::with_capacity(request.pairs.len());
  for (index, pair) in request.pairs.iter().enumerate() {
    let subsumption = match &pair.subsumption {
      None => None,
      Some(document) => {
        let report: SubsumptionReport =
          serde_path_to_error::deserialize(document).map_err(|err| SubsumptionError::Invalid {
            problems: vec![crate::error::Problem {
              pointer: format!(
                "/pairs/{index}/subsumption{}",
                crate::error::json_pointer(err.path())
              ),
              message: err.inner().to_string(),
            }],
          })?;
        Some(report)
      }
    };
    pairs.push(compatibility::Pair {
      consumer: pair.consumer.clone(),
      provider: pair.provider.clone(),
      format: pair.format.clone(),
      subsumption,
    });
  }

  let report = compatibility::decide(&pairs, &request.verification, &policy, request.as_of.as_deref());
  let text = compatibility::render(&report);
  Ok((report, text))
}

/// The consumer side of a pair, and how it identified itself.
///
/// A v1–v4 pact is converted here rather than refused, for the reason plan task 5.4 gives about
/// verification: the pact a consumer published years ago is the document a provider actually has,
/// and refusing to check it would make the loop reachable only by teams who had already migrated
/// — exactly the wrong way round. The conversion is [`crate::upgrade`]'s, findings and all; a
/// host that wants to read those calls `upgrade/pact` itself, which is why they are not smuggled
/// into this result.
fn consumer_contract(document: &Value) -> Result<(Contract, String), SubsumptionError> {
  let bytes = document.to_string();
  if contract::identify(bytes.as_bytes(), IdentifyMode::Tolerant) {
    let contract = contract::read(bytes.as_bytes(), IdentifyMode::Tolerant).map_err(|error| {
      SubsumptionError::Document {
        member: "contract",
        error,
      }
    })?;
    return Ok((contract, contract::FORMAT.to_string()));
  }
  // Identification before parsing, as everywhere else (ADR 0011): `pact_models` will read `{}` as
  // an empty pact, and "a contract with nothing in it" is exactly the misparse that would turn a
  // typo into a report full of silent passes.
  if !super::verification::looks_like_a_pact(document) {
    return Err(SubsumptionError::Document {
      member: "contract",
      error: ContractError::NotAContract,
    });
  }
  let upgraded = crate::upgrade::pact("contract", document).map_err(|error| SubsumptionError::Document {
    member: "contract",
    error,
  })?;
  let version = document
    .pointer("/metadata/pactSpecification/version")
    .or_else(|| document.pointer("/metadata/pact-specification/version"))
    .and_then(Value::as_str);
  let format = match version {
    Some(version) => format!("pact/{version}"),
    None => "pact".to_string(),
  };
  Ok((upgraded.contract, format))
}

fn read_provider_shape(document: &Value) -> Result<ProviderShape, SubsumptionError> {
  crate::subsumption::read_provider_shape(document.to_string().as_bytes()).map_err(|error| {
    SubsumptionError::Document {
      member: "provider-shape",
      error,
    }
  })
}
