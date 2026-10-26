//! The compatibility report (engine-protocol spec §8.6) and [`decide`], which writes one.
//!
//! Field order follows the schema, for the reason [`crate::contract::model`] gives: `serde_json`
//! serializes a struct's fields in declaration order, so a canonical member order falls out of
//! the type rather than a hand-rolled serializer.

use crate::contract::Party;
use crate::subsumption::{Action, Exemption, NOT_PUBLISHED, Severity, SubsumptionPolicy, SubsumptionReport};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// The format token this module writes.
pub const FORMAT: &str = "janus-compatibility-report/1";

/// The answer, for one pair or for the whole run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Decision {
  /// Nothing to say: deploy.
  Pass,
  /// Something a person should read, that does not stop the deploy (ADR 0016's default for both
  /// subsumption severities).
  Warn,
  /// Stop. The exit code a CI script reads is the CLI's business, not this document's.
  Block,
}

impl Decision {
  pub fn as_str(self) -> &'static str {
    match self {
      Decision::Pass => "pass",
      Decision::Warn => "warn",
      Decision::Block => "block",
    }
  }
}

/// What one reason does to the decision. `note` is the third value a [`Decision`] does not have:
/// something worth printing that changes nothing, which is most of what an honest report says.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReasonAction {
  Note,
  Warn,
  Block,
}

impl ReasonAction {
  fn decision(self) -> Decision {
    match self {
      ReasonAction::Note => Decision::Pass,
      ReasonAction::Warn => Decision::Warn,
      ReasonAction::Block => Decision::Block,
    }
  }

  fn of(action: Action) -> ReasonAction {
    match action {
      Action::Warn => ReasonAction::Warn,
      Action::Block => ReasonAction::Block,
    }
  }
}

/// Why a pair decided the way it did. One entry per fact, never a summary of several: a report
/// that said "blocked" without saying which of four possible facts blocked it would send its
/// reader back to the raw documents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reason {
  /// An open vocabulary, listed in engine-protocol spec §8.6. An unknown code is displayed,
  /// never dispatched on — `action` is what a script reads.
  pub code: String,
  pub action: ReasonAction,
  pub message: String,
}

impl Reason {
  fn new(code: &str, action: ReasonAction, message: impl Into<String>) -> Reason {
    Reason {
      code: code.to_string(),
      action,
      message: message.into(),
    }
  }
}

/// One (consumer, provider) pair the host is asking about, with whatever the checker found for it.
///
/// The host names the pairs; this module never infers them from the verification results it was
/// handed. A question nobody asked is not answered, and a pair asked about with nothing attached
/// is answered honestly rather than dropped — which is the difference between "no shape published"
/// and "not checked".
#[derive(Debug, Clone)]
pub struct Pair {
  pub consumer: Party,
  pub provider: Party,
  /// How the consumer side identified itself (ADR 0011): `janus-contract/1`, or `pact/<version>`
  /// for a v1–v4 pact converted on the way in.
  pub format: Option<String>,
  pub subsumption: Option<SubsumptionReport>,
}

/// The whole answer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompatibilityReport {
  #[serde(rename = "$format")]
  pub format: String,
  pub decision: Decision,
  pub policy: PolicyView,
  /// The date exemption expiry was judged against, when the host supplied one.
  #[serde(rename = "as-of", skip_serializing_if = "Option::is_none")]
  pub as_of: Option<String>,
  pub pairs: Vec<PairResult>,
  pub summary: Summary,
}

/// The policy this run resolved to, recorded so a report read months later says what decided it
/// (design 2.8 §7.1's layering means the answer is not in any one file).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct PolicyView {
  pub on_finding: Action,
  pub on_review: Action,
  /// How many exemptions were in force, across every layer.
  pub exemptions: usize,
}

/// One pair's answer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PairResult {
  pub consumer: Party,
  pub provider: Party,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub format: Option<String>,
  pub decision: Decision,
  pub verification: VerificationView,
  pub subsumption: SubsumptionView,
  pub reasons: Vec<Reason>,
  #[serde(skip_serializing_if = "Vec::is_empty")]
  pub findings: Vec<FindingEntry>,
  #[serde(skip_serializing_if = "Vec::is_empty")]
  pub exemptions: Vec<ExemptionResult>,
}

/// What the supplied verification results say about this pair.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VerificationView {
  /// `verified`, `failed`, `incomplete` (a run a hook aborted), or `unknown` — no supplied result
  /// covers this pair.
  pub status: String,
  /// How many supplied results covered it.
  pub runs: usize,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub filtered: Option<bool>,
  /// The variants the covering run replayed for this pair: its `pairs` entries' counts, added up
  /// when it verified more than one document between these two (ADR 0033). Present only when one
  /// run covered the pair — two runs' counts are two answers, not one — and, for a summary from
  /// before `pairs`, only when that run covered this pair alone.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub variants: Option<Value>,
}

/// What the subsumption report says about this pair, after policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SubsumptionView {
  /// The *walk's* aggregate verdict, unchanged by policy: `yes`, `no`, `unknown`, or
  /// `not-checked` when no report was attached. An exemption changes the decision, never this.
  pub verdict: String,
  pub interactions: usize,
  pub matched: usize,
  pub not_published: usize,
  /// `finding`-severity results no exemption silenced.
  pub findings: usize,
  /// `review`-severity results no exemption silenced.
  pub reviews: usize,
  /// Results of either severity an exemption silenced.
  pub exempt: usize,
  /// `advisory` entries (design 2.8 §5), which no policy governs and no exemption silences.
  pub advisories: usize,
}

/// Where one finding ended up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Disposition {
  Live,
  Exempt,
}

/// One finding, with the interaction it belongs to and what policy did with it. The finding
/// itself is design 2.8's document, carried through untouched.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FindingEntry {
  pub interaction: InteractionId,
  pub disposition: Disposition,
  /// The exemption that silenced it, when one did — the reason a reader is looking for.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub exemption: Option<ExemptionRef>,
  pub finding: crate::subsumption::Finding,
}

/// An interaction by contract spec §4.2's identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InteractionId {
  pub description: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub states: Option<Vec<String>>,
}

/// The silencing exemption, quoted where the finding is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExemptionRef {
  pub reason: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub expires: Option<String>,
}

/// What became of one exemption.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExemptionStatus {
  /// It silenced at least one finding.
  Applied,
  /// It matched nothing — either fixed, or never right in the first place.
  Unused,
  /// Its `expires` date has passed, so it silenced nothing (design 2.8 §7.2).
  Lapsed,
}

/// One exemption's outcome, echoing the exemption itself so a reader of the report does not need
/// the policy file open beside it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExemptionResult {
  pub status: ExemptionStatus,
  pub matched: usize,
  #[serde(flatten)]
  pub exemption: Exemption,
}

/// Counts across the whole report, so a reader sees the shape of the answer without counting the
/// list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Summary {
  pub pairs: usize,
  pub blocked: usize,
  pub warned: usize,
  pub passed: usize,
  pub findings: usize,
  pub reviews: usize,
  pub exempt: usize,
}

/// Decide `pairs`, against the verification summaries in `verifications` and one resolved policy.
///
/// `as_of` is an RFC 3339 full-date (`YYYY-MM-DD`) the host supplies — this engine has no clock
/// (see [`crate::subsumption::Exemption::lapsed`]). Passing `None` applies every exemption and
/// reports each `expires` as unevaluated, which is the honest answer to "expired relative to
/// when?" when nobody said.
pub fn decide(
  pairs: &[Pair],
  verifications: &[Value],
  policy: &SubsumptionPolicy,
  as_of: Option<&str>,
) -> CompatibilityReport {
  let unattributable = verifications
    .iter()
    .filter(|summary| summary.get("consumers").and_then(Value::as_array).is_none())
    .count();

  let results: Vec<PairResult> = pairs
    .iter()
    .map(|pair| decide_pair(pair, verifications, unattributable, policy, as_of))
    .collect();

  let mut summary = Summary {
    pairs: results.len(),
    blocked: 0,
    warned: 0,
    passed: 0,
    findings: 0,
    reviews: 0,
    exempt: 0,
  };
  let mut decision = Decision::Pass;
  for result in &results {
    match result.decision {
      Decision::Block => summary.blocked += 1,
      Decision::Warn => summary.warned += 1,
      Decision::Pass => summary.passed += 1,
    }
    summary.findings += result.subsumption.findings;
    summary.reviews += result.subsumption.reviews;
    summary.exempt += result.subsumption.exempt;
    decision = decision.max(result.decision);
  }

  CompatibilityReport {
    format: FORMAT.to_string(),
    decision,
    policy: PolicyView {
      on_finding: policy.on_finding,
      on_review: policy.on_review,
      exemptions: policy.exemptions.len(),
    },
    as_of: as_of.map(str::to_string),
    pairs: results,
    summary,
  }
}

fn decide_pair(
  pair: &Pair,
  verifications: &[Value],
  unattributable: usize,
  policy: &SubsumptionPolicy,
  as_of: Option<&str>,
) -> PairResult {
  let consumer = pair.consumer.name.as_str();
  let provider = pair.provider.name.as_str();
  let mut reasons = Vec::new();

  let verification = verification_for(verifications, consumer, provider);
  match verification.status.as_str() {
    "failed" => reasons.push(Reason::new(
      "verification-failed",
      ReasonAction::Block,
      "the contract was replayed at the provider and found mismatches; no policy and no \
       exemption makes that a pass",
    )),
    "incomplete" => reasons.push(Reason::new(
      "verification-incomplete",
      ReasonAction::Block,
      "the verification run was aborted before it finished, so what it did not reach is unknown, \
       not passing",
    )),
    "unknown" => reasons.push(Reason::new(
      "verification-missing",
      ReasonAction::Warn,
      if unattributable > 0 {
        match unattributable {
          1 => "no verification result names this pair (1 supplied result names no consumers at \
                all, and cannot be attributed)"
            .to_string(),
          n => format!(
            "no verification result names this pair ({n} supplied results name no consumers at \
             all, and cannot be attributed)"
          ),
        }
      } else {
        "no verification result was supplied for this pair; a contract nobody replayed is not a \
         passing one"
          .to_string()
      },
    )),
    _ => {}
  }
  if verification.filtered == Some(true) {
    reasons.push(Reason::new(
      "verification-filtered",
      ReasonAction::Warn,
      "the verification run was filtered: unreplayed variants are not passing ones",
    ));
  }

  // --- the subsumption half: dispose of every finding, then dispatch on what is left ----------

  let (subsumption, findings, exemptions, exemption_reasons) = match &pair.subsumption {
    None => (
      SubsumptionView {
        verdict: "not-checked".to_string(),
        interactions: 0,
        matched: 0,
        not_published: 0,
        findings: 0,
        reviews: 0,
        exempt: 0,
        advisories: 0,
      },
      Vec::new(),
      Vec::new(),
      Vec::new(),
    ),
    Some(report) => dispose(report, consumer, policy, as_of),
  };

  if pair.subsumption.is_none() {
    reasons.push(Reason::new(
      "provider-shape-missing",
      ReasonAction::Note,
      "the provider published no shape, so this pair gets replay-only semantics — today's \
       behaviour, which is what makes the check adoptable per provider",
    ));
  } else {
    if subsumption.not_published > 0 {
      reasons.push(Reason::new(
        "interactions-not-published",
        ReasonAction::Note,
        match subsumption.not_published {
          1 => format!(
            "1 of {} has no published provider shape and was not checked",
            super::count(subsumption.interactions as u64, "interaction", "interactions")
          ),
          n => format!(
            "{n} of {} have no published provider shape and were not checked",
            super::count(subsumption.interactions as u64, "interaction", "interactions")
          ),
        },
      ));
    }
    if subsumption.findings > 0 {
      reasons.push(Reason::new(
        "subsumption-findings",
        ReasonAction::of(policy.on_finding),
        format!(
          "{}: the provider may produce responses this consumer has not tested",
          super::count(subsumption.findings as u64, "decided finding", "decided findings")
        ),
      ));
    }
    if subsumption.reviews > 0 {
      reasons.push(Reason::new(
        "subsumption-reviews",
        ReasonAction::of(policy.on_review),
        format!(
          "{} the checker cannot decide either way; a person has to look",
          super::count(subsumption.reviews as u64, "comparison", "comparisons")
        ),
      ));
    }
    if subsumption.exempt > 0 {
      reasons.push(Reason::new(
        "subsumption-exempt",
        ReasonAction::Note,
        format!(
          "{} silenced by an exemption this policy carries",
          super::count(subsumption.exempt as u64, "finding", "findings")
        ),
      ));
    }
    if subsumption.advisories > 0 {
      reasons.push(Reason::new(
        "subsumption-advisories",
        ReasonAction::Note,
        match subsumption.advisories {
          1 => "1 passing field carries an exercised-coverage caveat (design 2.8 §5)".to_string(),
          n => format!("{n} passing fields carry an exercised-coverage caveat (design 2.8 §5)"),
        },
      ));
    }
  }
  reasons.extend(exemption_reasons);

  let decision = reasons
    .iter()
    .map(|reason| reason.action.decision())
    .max()
    .unwrap_or(Decision::Pass);

  PairResult {
    consumer: pair.consumer.clone(),
    provider: pair.provider.clone(),
    format: pair.format.clone(),
    decision,
    verification,
    subsumption,
    reasons,
    findings,
    exemptions,
  }
}

/// Every finding in one report, disposed of against the policy's exemptions — and the exemptions,
/// with what each one actually did.
fn dispose(
  report: &SubsumptionReport,
  consumer: &str,
  policy: &SubsumptionPolicy,
  as_of: Option<&str>,
) -> (
  SubsumptionView,
  Vec<FindingEntry>,
  Vec<ExemptionResult>,
  Vec<Reason>,
) {
  let mut view = SubsumptionView {
    // `not-checked` when nothing matched, whether or not a report was attached: a shape that
    // covers none of this contract's interactions has decided exactly as much as no shape at all,
    // and folding an empty conjunction to `yes` would report coverage nobody has.
    verdict: if report.summary.matched == 0 {
      "not-checked".to_string()
    } else {
      report
        .interactions
        .iter()
        .filter(|interaction| interaction.matched)
        .map(|interaction| interaction.verdict.as_str())
        .fold("yes", worst_verdict)
        .to_string()
    },
    interactions: report.interactions.len(),
    matched: report.summary.matched,
    not_published: report
      .interactions
      .iter()
      .filter(|interaction| interaction.verdict == NOT_PUBLISHED)
      .count(),
    findings: 0,
    reviews: 0,
    exempt: 0,
    advisories: 0,
  };
  let mut entries = Vec::new();
  let mut matched = vec![0usize; policy.exemptions.len()];

  for interaction in &report.interactions {
    let states: Vec<String> = interaction
      .states
      .iter()
      .flatten()
      .map(|state| state.name.clone())
      .collect();
    let id = InteractionId {
      description: interaction.description.clone(),
      states: (!states.is_empty()).then(|| states.clone()),
    };
    for finding in &interaction.findings {
      // An `advisory` is policy-inert by construction (design 2.8 §4.3), so no exemption is
      // consulted for one: silencing a caveat nobody dispatches on would only inflate the
      // exempt count and hide the caveat.
      let exempting = policy.action_for(finding.severity).and_then(|_| {
        policy
          .exemptions
          .iter()
          .enumerate()
          .filter(|(_, exemption)| !exemption.lapsed(as_of))
          .find(|(_, exemption)| {
            exemption.matches(consumer, &interaction.description, &states, &finding.path)
          })
      });
      match exempting {
        Some((index, exemption)) => {
          matched[index] += 1;
          view.exempt += 1;
          entries.push(FindingEntry {
            interaction: id.clone(),
            disposition: Disposition::Exempt,
            exemption: Some(ExemptionRef {
              reason: exemption.reason.clone(),
              expires: exemption.expires.clone(),
            }),
            finding: finding.clone(),
          });
        }
        None => {
          match finding.severity {
            Severity::Finding => view.findings += 1,
            Severity::Review => view.reviews += 1,
            Severity::Advisory => view.advisories += 1,
          }
          entries.push(FindingEntry {
            interaction: id.clone(),
            disposition: Disposition::Live,
            exemption: None,
            finding: finding.clone(),
          });
        }
      }
    }
  }

  let mut lapsed = 0;
  let mut unused = 0;
  let mut no_expiry = 0;
  let mut unevaluated = 0;
  let exemptions: Vec<ExemptionResult> = policy
    .exemptions
    .iter()
    .zip(&matched)
    .map(|(exemption, count)| {
      let status = if exemption.lapsed(as_of) {
        lapsed += 1;
        ExemptionStatus::Lapsed
      } else if *count == 0 {
        unused += 1;
        ExemptionStatus::Unused
      } else {
        if exemption.expires.is_none() {
          no_expiry += 1;
        } else if as_of.is_none() {
          unevaluated += 1;
        }
        ExemptionStatus::Applied
      };
      ExemptionResult {
        status,
        matched: *count,
        exemption: exemption.clone(),
      }
    })
    .collect();

  let mut reasons = Vec::new();
  if lapsed > 0 {
    reasons.push(Reason::new(
      "exemption-lapsed",
      ReasonAction::Warn,
      match lapsed {
        1 => "1 exemption is past its `expires` date and silenced nothing; whatever it covered is \
              decided again"
          .to_string(),
        n => format!(
          "{n} exemptions are past their `expires` date and silenced nothing; whatever they \
           covered is decided again"
        ),
      },
    ));
  }
  if no_expiry > 0 {
    // Design 2.8 §7.2 asks task 7.4 to surface this rather than the schema forbidding it: some
    // gaps are permanent by design, and a mandatory date would just be a date nobody believes.
    reasons.push(Reason::new(
      "exemption-no-expiry",
      ReasonAction::Note,
      match no_expiry {
        1 => {
          "1 applied exemption carries no `expires` date and will never be revisited on its own".to_string()
        }
        n => {
          format!("{n} applied exemptions carry no `expires` date and will never be revisited on their own")
        }
      },
    ));
  }
  if unused > 0 {
    reasons.push(Reason::new(
      "exemption-unused",
      ReasonAction::Note,
      format!(
        "{} matched nothing — either the gap closed, or the selectors never fitted it",
        super::count(unused as u64, "exemption", "exemptions")
      ),
    ));
  }
  if unevaluated > 0 {
    reasons.push(Reason::new(
      "expiry-not-evaluated",
      ReasonAction::Note,
      match unevaluated {
        1 => "1 applied exemption carries an `expires` date that was not evaluated: no date to judge \
              against was supplied"
          .to_string(),
        n => format!(
          "{n} applied exemptions carry an `expires` date that was not evaluated: no date to judge \
           against was supplied"
        ),
      },
    ));
  }

  (view, entries, exemptions, reasons)
}

/// Kleene conjunction over the report's own verdict strings (design 2.8 §6.2, one level up
/// again): `no` dominates `unknown` dominates `yes`. `not-published` is not one of the three and
/// is counted separately rather than folded in — an interaction nobody checked must not make a
/// pair's verdict worse *or* better.
fn worst_verdict(left: &'static str, right: &str) -> &'static str {
  match (left, right) {
    ("no", _) | (_, "no") => "no",
    ("unknown", _) | (_, "unknown") => "unknown",
    _ => left,
  }
}

/// What the supplied verification summaries say about one pair.
///
/// A summary names the pairs it covered in `pairs` (ADR 0033): one entry per source document, with
/// its own status and variant counts. A summary from before `pairs` names them in
/// `consumers`/`providers`, positionally parallel, and its `failures` the pair each failing variant
/// belonged to — enough for the status, and for the counts only when the run covered this pair
/// alone.
fn verification_for(verifications: &[Value], consumer: &str, provider: &str) -> VerificationView {
  let mut runs = 0;
  let mut status = "unknown";
  let mut filtered = None;
  let mut variants = None;

  for summary in verifications {
    let Some(consumers) = summary.get("consumers").and_then(Value::as_array) else {
      continue;
    };
    let providers = summary
      .get("providers")
      .and_then(Value::as_array)
      .map(|providers| providers.as_slice())
      .unwrap_or_default();
    let pairs: Vec<(&str, &str)> = consumers
      .iter()
      .zip(providers)
      .filter_map(|(consumer, provider)| Some((consumer.as_str()?, provider.as_str()?)))
      .collect();
    if !pairs.contains(&(consumer, provider)) {
      continue;
    }
    runs += 1;

    if let Some(entries) = summary.get("pairs").and_then(Value::as_array) {
      let mine: Vec<&Value> = entries
        .iter()
        .filter(|entry| entry["consumer"] == json!(consumer) && entry["provider"] == json!(provider))
        .collect();
      // Filtered for this pair, not merely in the same run (ADR 0041 decision 6): a run that
      // narrowed another pair's document verified this one whole.
      let narrowed = mine.iter().any(|entry| entry["status"] == json!("filtered"));
      filtered = Some(filtered.unwrap_or(false) || narrowed);
      variants = (runs == 1).then(|| added_up(&mine));
      let this = mine.iter().filter_map(|entry| entry["status"].as_str()).fold(
        "verified",
        |worst, status| match status {
          "failed" => worst_status(worst, "failed"),
          "incomplete" => worst_status(worst, "incomplete"),
          _ => worst,
        },
      );
      status = worst_status(status, this);
      continue;
    }

    // A summary from before `pairs` says only whether the run was filtered.
    filtered = Some(filtered.unwrap_or(false) || summary["filtered"] == Value::Bool(true));
    if pairs.len() == 1 && runs == 1 {
      variants = summary.get("variants").cloned();
    } else {
      variants = None;
    }

    let failed = summary["failures"]
      .as_array()
      .map(|failures| {
        failures.iter().any(|failure| {
          let contract = &failure["interaction"]["contract"];
          contract["consumer"] == Value::String(consumer.to_string())
            && contract["provider"] == Value::String(provider.to_string())
        })
      })
      .unwrap_or(false);
    let this = if failed {
      "failed"
    } else if summary.get("aborted").is_some_and(|abort| !abort.is_null()) {
      "incomplete"
    } else {
      "verified"
    };
    status = worst_status(status, this);
  }

  VerificationView {
    status: status.to_string(),
    runs,
    filtered,
    variants,
  }
}

/// Several `pairs` entries' variant counts as one set: a run that verified two documents between
/// the same two parties replayed both documents' variants for that pair.
fn added_up(entries: &[&Value]) -> Value {
  let mut total = serde_json::Map::new();
  for entry in entries {
    for (name, count) in entry["variants"].as_object().into_iter().flatten() {
      let sum = total.get(name).and_then(Value::as_u64).unwrap_or(0) + count.as_u64().unwrap_or(0);
      total.insert(name.clone(), json!(sum));
    }
  }
  Value::Object(total)
}

fn worst_status(left: &'static str, right: &'static str) -> &'static str {
  for candidate in ["failed", "incomplete", "verified"] {
    if left == candidate || right == candidate {
      return candidate;
    }
  }
  left
}
