//! The `can-i-deploy` page (engine-protocol spec §8.6): design 2.8 §6.4's finding block, under a
//! decision, with the verification line the RFC asks for beside it.
//!
//! The block itself is not re-worded here. [`crate::subsumption::render`]'s own pieces produce
//! every finding line, which is what keeps the RFC's sketch reproducible verbatim — the header
//! line and the two `provider may produce` / `consumer has only tested` lines are the
//! specification's, and this module only decides which findings appear where.
//!
//! One marker is this page's own: `~` for a finding an exemption silenced. Design 2.8 §6.5 fixes
//! three (none, `?`, `!`) for the three severities, and an exempted finding is not a fourth
//! severity — it is a finding with a decision over it, which is exactly the distinction the
//! marker has to carry.

use super::decide::{CompatibilityReport, Decision, Disposition, ExemptionStatus, PairResult, ReasonAction};
use crate::subsumption::{Severity, finding_lines, header};

/// Render a compatibility report as the text a person reads.
pub fn render(report: &CompatibilityReport) -> String {
  let mut lines = Vec::new();
  for pair in &report.pairs {
    lines.extend(render_pair(pair));
    lines.push(String::new());
  }
  lines.push(verdict_line(report));
  lines.join("\n")
}

fn render_pair(pair: &PairResult) -> Vec<String> {
  let live: Vec<_> = pair
    .findings
    .iter()
    .filter(|entry| entry.disposition == Disposition::Live)
    .collect();
  let has_findings = live
    .iter()
    .any(|entry| entry.finding.severity == Severity::Finding);
  let has_reviews = live
    .iter()
    .any(|entry| entry.finding.severity == Severity::Review);

  let mut lines = vec![header(
    &pair.consumer.name,
    &pair.provider.name,
    has_findings,
    has_reviews,
    pair.subsumption.matched > 0,
  )];

  for entry in &live {
    lines.extend(finding_lines(
      &entry.interaction.description,
      &entry.finding,
      None,
    ));
  }
  for entry in pair
    .findings
    .iter()
    .filter(|entry| entry.disposition == Disposition::Exempt)
  {
    lines.extend(finding_lines(
      &entry.interaction.description,
      &entry.finding,
      Some("~ "),
    ));
    if let Some(exemption) = &entry.exemption {
      lines.push(match &exemption.expires {
        Some(expires) => format!("    exempted until {expires}: {}", exemption.reason),
        None => format!("    exempted: {}", exemption.reason),
      });
    }
  }
  lines.push(format!("  verification: {}", verification_phrase(pair)));
  for reason in &pair.reasons {
    let marker = match reason.action {
      ReasonAction::Block => "✗",
      ReasonAction::Warn => "!",
      ReasonAction::Note => "-",
    };
    lines.push(format!("  {marker} {}", reason.message));
  }
  for exemption in &pair.exemptions {
    if exemption.status == ExemptionStatus::Lapsed {
      lines.push(format!(
        "  ! lapsed exemption ({}): {}",
        exemption.exemption.expires.as_deref().unwrap_or("no expiry"),
        exemption.exemption.reason
      ));
    }
  }
  lines.push(format!(
    "  => {}: {} -> {}",
    pair.decision.as_str().to_uppercase(),
    pair.consumer.name,
    pair.provider.name
  ));
  lines
}

/// The verification half of the page, in one line — the RFC's `can-i-deploy` combines the two,
/// and a page that printed subsumption findings without saying whether the contract was ever
/// replayed would be answering half the question.
fn verification_phrase(pair: &PairResult) -> String {
  let status = pair.verification.status.as_str();
  let mut text = match status {
    "unknown" => "no result supplied".to_string(),
    other => other.to_string(),
  };
  if let Some(variants) = &pair.verification.variants {
    let total = variants["total"].as_u64().unwrap_or(0);
    let verified = variants["verified"].as_u64().unwrap_or(0);
    text.push_str(&format!(" ({verified} of {total} variant(s))"));
  } else if pair.verification.runs > 1 {
    text.push_str(&format!(" (across {} runs)", pair.verification.runs));
  }
  if pair.verification.filtered == Some(true) {
    text.push_str(", filtered");
  }
  text
}

fn verdict_line(report: &CompatibilityReport) -> String {
  let summary = &report.summary;
  let mut text = format!(
    "{}: {} pair(s) blocked, {} warned, {} passed",
    report.decision.as_str().to_uppercase(),
    summary.blocked,
    summary.warned,
    summary.passed
  );
  text.push_str(&format!(
    " — policy on-finding {}, on-review {}",
    report.policy.on_finding.as_str(),
    report.policy.on_review.as_str()
  ));
  if let Some(as_of) = &report.as_of {
    text.push_str(&format!(", exemptions as of {as_of}"));
  }
  if report.decision == Decision::Pass && summary.pairs == 0 {
    return "PASS: nothing to decide — no pairs were given".to_string();
  }
  text
}
